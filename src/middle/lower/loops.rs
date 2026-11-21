//! 循环族（`AstToIrGenerator` 的循环生成）
//!
//! while/for/range/in/spawn-for 与迭代器协议循环，含 break/continue 跳转目标上下文。
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.2），语义未改。

use super::AstToIrGenerator;
use super::LoopTargets;
use crate::ast;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

impl AstToIrGenerator {
    /// 进入循环体生成：压入跳转目标上下文（#311）
    fn enter_loop_targets(
        &mut self,
        loop_start_idx: usize,
    ) {
        // #393：循环态临时地板 = 进入时的 next_temp——for 的 iterator/iterable
        // 等临时在进入前分配、体内每轮读取，语句级回收不得回滚到它们之下。
        // while 的 cond 在进入后分配、读点都在体内语句之前，地板盖不住它
        // 也无害（每轮循环开头重写）。
        let saved_temp_floor = self.temp_floor;
        self.temp_floor = self.next_temp;
        self.loop_stack.push(LoopTargets {
            continue_target: loop_start_idx,
            break_fixups: Vec::new(),
            saved_temp_floor,
        });
    }

    /// 离开循环体生成：把循环体内 break 的占位 Jmp 回填到循环出口 `end_idx`（#311）
    fn exit_loop_targets(
        &mut self,
        end_idx: usize,
        instructions: &mut [Instruction],
    ) {
        if let Some(targets) = self.loop_stack.pop() {
            // #393：恢复外层地板（嵌套循环各保各的）
            self.temp_floor = targets.saved_temp_floor;
            for fixup in targets.break_fixups {
                if let Instruction::Jmp {
                    ref mut target,
                    span: _,
                } = instructions[fixup]
                {
                    *target = end_idx;
                }
            }
        }
    }

    /// Generate While expression IR
    pub(in crate::middle) fn generate_while_expr_ir(
        &mut self,
        condition: &ast::Expr,
        body: &ast::Block,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // #409（while/for 同款）：先给值寄存器写 Void（等价于 block_value_ty 的「空块 ⇒ Void」
        // 分支），此后凡体尾产出值即覆写它。该初值同时覆盖两种情形：
        //   1. 条件首次即为假（零迭代）；
        //   2. 每次完整迭代的体尾都不产出值（如体尾是赋值语句）——continue
        //      直接回跳条件重判点、不经过体尾，故保留上一轮的值。
        //
        // 零迭代取 Void 的依据（有意为之，非遗留）：循环体一次都没求值 ⇒ 没有
        // 「末次迭代的体值」（`for` 同理：源集合为空时没有体值）⇒ 「未产出值」只能用 Void 表达。要给它一个按值类型
        // 定制的默认值，需要在 IR 层知道该值寄存器的**静态类型**；但 ir_gen 拿不到
        // 这个信息——LocalSlot.ty 由 register_local 置为 Int(64)（本文件 619 行），
        // 注解类型不写回槽位，TypeCheckResult 也没有按节点索引的类型表。故按类型
        // 定制默认值不是本层能做的事（ir_gen 内对 resolve_type 的调用数为 0）。
        // 后果：零迭代时值寄存器是 Void，把它当非 Void 类型用会在运行期以
        // 类型不匹配显式失败——是「响亮失败」而非静默错值。类型侧
        // block_value_ty 给的是体尾表达式的类型（静态近似），它对「循环是否至少
        // 执行一次」无判定能力（需要数据流/可证性分析），故本层不擅自改动它。
        instructions.push(Instruction::Load {
            dst: result_reg.clone(),
            src: Operand::Const(ConstValue::Void),
            span: self.cur_span,
        });

        // Label: condition_check
        let loop_start_idx = instructions.len();
        // #311：压入循环上下文——continue 跳回条件重判点，break 占位待出口回填
        self.enter_loop_targets(loop_start_idx);

        // Evaluate condition
        let cond_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, cond_reg.clone(), instructions, constants)?;

        // Jump to end if false
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: cond_reg.clone(),
            target: 0,
            span: self.cur_span,
        }); // Placeholder

        // Body：尾表达式直接写入值寄存器（RFC-010a 规则① / spec §2.9：所有块的
        // 值由尾表达式给出）。#409 之前此处传 None 丢掉体值，并在出口无条件覆写
        // 成 Void，与类型侧 block_value_ty 的判据分叉。
        self.generate_block_ir(body, Some(result_reg), instructions, constants)?;

        // Jump back to start
        instructions.push(Instruction::Jmp {
            target: loop_start_idx,
            span: self.cur_span,
        });

        // Fix JmpIfNot target
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_end_idx]
        {
            *target = end_idx;
        }
        // #311：回填循环体内 break 的占位跳转到出口
        self.exit_loop_targets(end_idx, instructions);

        Ok(())
    }

    /// Generate For loop IR
    #[allow(clippy::too_many_arguments)]
    pub(in crate::middle) fn generate_for_loop_ir(
        &mut self,
        var_name: &str,
        #[allow(unused_variables)] var_mut: bool,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: Option<Operand>,
        for_span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // #302：Range 特判已删——`for i in 0..10` / `for i in r` 统一走迭代器协议
        // （std.range.iter/has_next/next，由下方按静态类型派发）
        if let Some(_iter_ty) = self.get_expr_mono_type(iterable).filter(|t| {
            // #299 去特殊化：List/Tuple/Dict 为 Generic，Range 仍为原生变体
            t.is_list() || t.is_tuple() || t.is_dict() || t.is_range()
        }) {
            // 使用迭代器协议的 For 循环
            self.generate_iterator_for_loop_ir(
                var_name,
                iterable,
                body,
                result_reg,
                for_span,
                instructions,
                constants,
            )
        } else if let Some(_iter_ty) = self.get_expr_mono_type(iterable) {
            // 不支持的迭代器类型，返回错误（使用实际类型名称）
            let iter_type = self.get_expr_type_name(iterable);
            let span = Self::get_expr_span(iterable);
            Err(ErrorCodeDefinition::ir_unsupported_iterator(&iter_type)
                .at(span)
                .build())
        } else {
            // 不支持的迭代器类型，返回错误（使用实际类型名称）
            let iter_type = self.get_expr_type_name(iterable);
            let span = Self::get_expr_span(iterable);
            Err(ErrorCodeDefinition::ir_unsupported_iterator(&iter_type)
                .at(span)
                .build())
        }
    }

    /// #300 I 项：Range 值构造——三槽 [start, end, step]
    ///
    /// `(a..b)..c` 是 step 形态（左操作数已是 Range 构造）；否则 step 默认 1。
    /// #302：动态 step=0 不在构造点拦截——消费点（std.range.iter/contains）显式报错；
    /// step=0 的偏性未来升格为 Result(Range(Int), Error)，见 #301 错误系统。
    #[allow(clippy::too_many_arguments)]
    pub(in crate::middle) fn generate_range_construction_ir(
        &mut self,
        left: &ast::Expr,
        right: &ast::Expr,
        result_reg: Operand,
        _span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 解构 step 形态：左操作数本身是 Range 构造
        let (start_expr, end_expr, step_expr) = if let ast::Expr::BinOp {
            op: ast::BinOp::Range,
            left: inner_start,
            right: inner_end,
            ..
        } = left
        {
            (inner_start.as_ref(), inner_end.as_ref(), Some(right))
        } else {
            (left, right, None)
        };

        let start_reg = self.next_temp_reg();
        let end_reg = self.next_temp_reg();
        let step_reg = self.next_temp_reg();
        self.generate_expr_ir(start_expr, start_reg.clone(), instructions, constants)?;
        self.generate_expr_ir(end_expr, end_reg.clone(), instructions, constants)?;
        if let Some(step) = step_expr {
            self.generate_expr_ir(step, step_reg.clone(), instructions, constants)?;
        } else {
            instructions.push(Instruction::Load {
                dst: step_reg.clone(),
                src: Operand::Const(ConstValue::Int(1)),
                span: self.cur_span,
            });
        }

        // #302：动态 step=0 不在构造点拦截——消费点（std.range.iter/contains）
        // 显式运行时报错，构造出的 Range 值本身无观察者时无害；
        // 字面量零已被 typecheck 拒绝；Result 化挂 #301

        // #302：正式运行时身份——不再借 Tuple 外壳
        instructions.push(Instruction::NewRange {
            dst: result_reg.clone(),
            start: start_reg.clone(),
            end: end_reg.clone(),
            step: step_reg.clone(),
            span: self.cur_span,
        });
        Ok(())
    }

    /// `elem in container` 的 IR 生成
    ///
    /// #302：Range 容器走 std.range.contains 调用；其余容器走 Contains 指令
    #[allow(clippy::too_many_arguments)]
    pub(in crate::middle) fn generate_in_ir(
        &mut self,
        elem: &ast::Expr,
        container: &ast::Expr,
        result_reg: Operand,
        span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let _ = span; // contains 调用的 span 经 Instruction::Call 携带
                      // #302：Range 臂迁移——`x in r` 运行时路径走 std.range.contains
                      // （界检查符号派发 + 步长对齐；动态 step=0 显式运行时错误）。
                      // AST 形态保持 Expr::In 一等谓词不变——精化/SMT 管道直接识别区间命题，
                      // 区间保持区间，不物化（#299 §3 同款定位）。
        if self
            .get_expr_mono_type(container)
            .map(|t| t.is_range())
            .unwrap_or(false)
        {
            let elem_reg = self.next_temp_reg();
            let container_reg = self.next_temp_reg();
            self.generate_expr_ir(elem, elem_reg.clone(), instructions, constants)?;
            self.generate_expr_ir(container, container_reg.clone(), instructions, constants)?;
            instructions.push(Instruction::Call {
                dst: Some(result_reg.clone()),
                func: Operand::Const(ConstValue::String("std.range.contains".to_string())),
                args: vec![container_reg.clone(), elem_reg.clone()],
                span,
                def: None,
            });
            // #316：contains 已 Result(Bool, Error) 化——`x in r` 糖降级统一解包，
            // Err（动态 step=0）分支显式失败（Bool 谓词无可传播的 Result）
            self.emit_result_unwrap_or_abort(
                result_reg.clone(),
                result_reg.clone(),
                "std.range.abort_invalid_step",
                container_reg.clone(),
                span,
                instructions,
            );
            return Ok(());
        }

        // 其余容器：Contains 指令（List/Array/Tuple/Dict/String）
        {
            let elem_reg = self.next_temp_reg();
            let container_reg = self.next_temp_reg();
            self.generate_expr_ir(elem, elem_reg.clone(), instructions, constants)?;
            self.generate_expr_ir(container, container_reg.clone(), instructions, constants)?;
            instructions.push(Instruction::Contains {
                dst: result_reg.clone(),
                elem: elem_reg.clone(),
                container: container_reg.clone(),
                span,
            });
        }
        Ok(())
    }

    /// #316：Result 解包，Err 分支转显式失败——`for`/`in` 糖消费 Result API 的统一降级
    ///
    /// IR 序列同 #301 `?`（VariantTag/Eq/JmpIfNot/VariantPayload），差别在 Err
    /// 分支：糖形态不产 Result 值（for 循环无返回、`x in r` 是 Bool 谓词），
    /// 隐式 `?` 对非 Result 函数是类型违约——改为调用 abort native 显式失败
    /// （失败点在用户消费处，消息明确，绝不静默续跑/死循环）。
    fn emit_result_unwrap_or_abort(
        &mut self,
        result_reg: Operand,
        dst_reg: Operand,
        abort_fn: &str,
        abort_arg: Operand,
        span: Span,
        instructions: &mut Vec<Instruction>,
    ) {
        let tag_reg = self.next_temp_reg();
        instructions.push(Instruction::VariantTag {
            dst: tag_reg.clone(),
            obj: result_reg.clone(),
            group: "Result".to_string(),
            span,
        });
        let ok_const = self.next_temp_reg();
        instructions.push(Instruction::Load {
            dst: ok_const.clone(),
            src: Operand::Const(ConstValue::Int(0)),
            span: self.cur_span,
        });
        let eq_reg = self.next_temp_reg();
        instructions.push(Instruction::Eq {
            dst: eq_reg.clone(),
            lhs: tag_reg.clone(),
            rhs: ok_const.clone(),
            span: self.cur_span,
        });
        // variant != 0（Err）→ 跳到显式失败
        let is_err_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: eq_reg.clone(),
            target: 0,
            span: self.cur_span,
        });
        // Ok 路径：payload 原地解包为消费值
        instructions.push(Instruction::VariantPayload {
            dst: dst_reg.clone(),
            obj: result_reg.clone(),
            group: "Result".to_string(),
            span,
        });
        let end_idx = instructions.len();
        instructions.push(Instruction::Jmp {
            target: 0,
            span: self.cur_span,
        });
        // Err 路径：abort native 必返 ExecutorError，后续指令不可达
        let err_target = instructions.len();
        instructions[is_err_idx] = Instruction::JmpIfNot {
            cond: eq_reg.clone(),
            target: err_target,
            span: self.cur_span,
        };
        instructions.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String(abort_fn.to_string())),
            args: vec![abort_arg],
            span,
            def: None,
        });
        let end_target = instructions.len();
        instructions[end_idx] = Instruction::Jmp {
            target: end_target,
            span: self.cur_span,
        };
    }

    /// 生成基于迭代器协议的 For 循环 IR
    /// 这是新的迭代器协议实现，调用 iter()/next()/has_next() 方法
    #[allow(clippy::too_many_arguments)]
    fn generate_iterator_for_loop_ir(
        &mut self,
        var_name: &str,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: Option<Operand>,
        for_span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        self.enter_scope();

        // #409：零迭代兜底——先给值寄存器写 Void（等价于 block_value_ty 的
        // 「空块 ⇒ Void」分支），随后每轮由体尾表达式覆写。与 while 同款语义
        //（零迭代取 Void 的依据见 generate_while_expr_ir 的注释）。
        if let Some(reg) = result_reg.clone() {
            instructions.push(Instruction::Load {
                dst: reg.clone(),
                src: Operand::Const(ConstValue::Void),
                span: self.cur_span,
            });
        }

        // 1. 计算可迭代对象
        let iterable_reg = self.next_temp_reg();
        self.generate_expr_ir(iterable, iterable_reg.clone(), instructions, constants)?;

        // 1.5 #302：按静态类型派发协议函数名（Range → std.range.*，其余 → std.list.*）
        let is_range = self
            .get_expr_mono_type(iterable)
            .map(|t| t.is_range())
            .unwrap_or(false);
        // 迭代协议函数名：range 仍是 native（限定名直用）；
        // list 已迁到纯 yx（src/std/list.yx），其函数以裸名注册，
        // 交给 `resolve_namespace` 决定最终名（命中限定名用限定名，否则退裸名）。
        let (iter_fn, has_next_fn, next_fn) = if is_range {
            (
                "std.range.iter".to_string(),
                "std.range.has_next".to_string(),
                "std.range.next".to_string(),
            )
        } else {
            (
                self.resolver().resolve_namespace("list", &["iter"]),
                self.resolver().resolve_namespace("list", &["has_next"]),
                self.resolver().resolve_namespace("list", &["next"]),
            )
        };

        // 2. 创建迭代器: iterator = iter(iterable)
        let iterator_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(iterator_reg.clone()),
            func: Operand::Const(ConstValue::String(iter_fn.clone())),
            args: vec![iterable_reg.clone()],
            span: for_span,
            def: None,
        });
        // #316：Range 协议 iter 已 Result(Iterator, Error) 化——糖降级统一解包，
        // Err（动态 step=0）分支显式失败（for 循环无可传播的 Result，且绝不死循环）
        if is_range {
            self.emit_result_unwrap_or_abort(
                iterator_reg.clone(),
                iterator_reg.clone(),
                "std.range.abort_invalid_step",
                iterable_reg.clone(),
                for_span,
                instructions,
            );
        }

        // 3. 注册循环变量（具名变量：Local 槽，不产生 SSA 值）
        let var_reg = self.next_local_slot();
        self.register_local(var_name, var_reg);

        // 4. 循环开始
        let loop_start_idx = instructions.len();
        // #311：压入循环上下文——continue 跳回 has_next 检查点，break 占位待出口回填
        self.enter_loop_targets(loop_start_idx);

        // 5. 检查是否有更多元素: has_more = has_next(iterator)
        let has_more_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(has_more_reg.clone()),
            func: Operand::Const(ConstValue::String(has_next_fn.to_string())),
            args: vec![iterator_reg.clone()],
            span: for_span,
            def: None,
        });

        // 6. 如果没有更多元素，跳转到结束
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: has_more_reg.clone(),
            target: 0,
            span: self.cur_span,
        });

        // 7. 获取下一个元素: var = next(iterator)
        let element_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(element_reg.clone()),
            func: Operand::Const(ConstValue::String(next_fn.to_string())),
            args: vec![iterator_reg.clone()],
            span: for_span,
            def: None,
        });
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: element_reg.clone(),
            span: for_span,
        });

        // 8. 执行循环体
        // #409：尾表达式直接写入值寄存器（RFC-010a 规则①；与 while 同款）。
        // 此前此处传 None 并在出口把值覆写成 Void，故 for 作值在运行期恒 void。
        self.generate_block_ir(body, result_reg, instructions, constants)?;

        // 9. 跳转回循环开始
        instructions.push(Instruction::Jmp {
            target: loop_start_idx,
            span: self.cur_span,
        });

        // 10. 修复跳转
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_end_idx]
        {
            *target = end_idx;
        }
        // #311：回填循环体内 break 的占位跳转到出口
        self.exit_loop_targets(end_idx, instructions);

        self.exit_scope();

        Ok(())
    }

    /// 生成 spawn for 数据并行循环的 IR
    ///
    /// 将 `spawn for item in items { process(item) }` 展开为：
    /// ```text
    /// closures = []          // AllocArray
    /// iterator = iter(items) // Call
    /// while has_next(iterator) {
    ///     item = next(iterator)
    ///     closure = () => { body }  // Lambda
    ///     push(closures, closure)   // Call std.list.push
    /// }
    /// Spawn { closures, plan, result }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub(in crate::middle) fn generate_spawn_for_ir(
        &mut self,
        var_name: &str,
        // ponytail: for-mut 的可变性由 typecheck 校验，ir_gen 无需追踪
        _var_mut: bool,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: Operand,
        span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        use crate::frontend::core::spawn::analysis::analyze_spawn_for;

        // 1. 分析循环体读写集
        let (trait_table, local_var_types) = if let Some(ref type_result) = self.type_result {
            (&type_result.trait_table, &type_result.local_var_types)
        } else {
            static EMPTY_TRAIT_TABLE: std::sync::LazyLock<
                crate::frontend::core::types::TraitTable,
            > = std::sync::LazyLock::new(crate::frontend::core::types::TraitTable::default);
            static EMPTY_VAR_TYPES: std::sync::LazyLock<
                std::collections::HashMap<String, crate::frontend::core::types::MonoType>,
            > = std::sync::LazyLock::new(std::collections::HashMap::new);
            (&*EMPTY_TRAIT_TABLE, &*EMPTY_VAR_TYPES)
        };

        let spawn_for_analysis =
            analyze_spawn_for(var_name, iterable, body, trait_table, local_var_types);

        // 2. 进入 spawn 作用域
        self.enter_scope();

        // 3. 创建空列表寄存器，用于收集闭包
        let closures_list_reg = self.next_temp_reg();
        instructions.push(Instruction::AllocArray {
            dst: closures_list_reg.clone(),
            size: Operand::Const(ConstValue::Int(0)),
            elem_size: Operand::Const(ConstValue::Int(1)),
            span: self.cur_span,
        });

        // 4. 计算可迭代对象
        let iterable_reg = self.next_temp_reg();
        self.generate_expr_ir(iterable, iterable_reg.clone(), instructions, constants)?;

        // 5. 创建迭代器: iterator = iter(iterable)
        let iterator_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(iterator_reg.clone()),
            func: Operand::Const(ConstValue::String(
                self.resolver().resolve_namespace("list", &["iter"]),
            )),
            args: vec![iterable_reg.clone()],
            span,
            def: None,
        });

        // 6. 注册循环变量（具名变量：Local 槽，不产生 SSA 值）
        let var_reg = self.next_local_slot();
        self.register_local(var_name, var_reg);

        // 7. 循环开始
        let loop_start_idx = instructions.len();

        // 8. has_next?
        let has_more_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(has_more_reg.clone()),
            func: Operand::Const(ConstValue::String("std.list.has_next".to_string())),
            args: vec![iterator_reg.clone()],
            span,
            def: None,
        });

        // 9. 如果没有更多元素，跳转到结束
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: has_more_reg.clone(),
            target: 0,
            span: self.cur_span,
        });

        // 10. 获取下一个元素: var = next(iterator)
        let element_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(element_reg.clone()),
            func: Operand::Const(ConstValue::String("std.list.next".to_string())),
            args: vec![iterator_reg.clone()],
            span,
            def: None,
        });
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: element_reg.clone(),
            span,
        });

        // 11. 在循环体内创建闭包：(item) => { body }
        //     迭代变量作为参数传入，当前值通过 env 捕获
        let closure_reg = self.next_temp_reg();
        // 捕获迭代变量的当前值（element_reg 在 next() 返回后被设置）
        self.pending_env_vars = vec![element_reg.clone()];
        let lambda = ast::Expr::Lambda {
            params: vec![ast::Param {
                name: var_name.to_string(),
                ty: None,
                is_mut: false,
                quantity: None,
                span,
            }],
            body: Box::new(body.clone()),
            span,
        };
        self.generate_expr_ir(&lambda, closure_reg.clone(), instructions, constants)?;

        // 12. 将闭包推入列表: push(closures, closure)
        instructions.push(Instruction::Call {
            dst: Some(closures_list_reg.clone()),
            func: Operand::Const(ConstValue::String("std.list.push".to_string())),
            args: vec![closures_list_reg.clone(), closure_reg.clone()],
            span,
            def: None,
        });

        // 13. 跳转回循环开始
        instructions.push(Instruction::Jmp {
            target: loop_start_idx,
            span: self.cur_span,
        });

        // 14. 修复跳出循环的跳转目标
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_end_idx]
        {
            *target = end_idx;
        }

        // 15. 构建执行计划
        //     所有迭代共享同一个读写集（编译期分析的循环体特征）
        let read_write_sets = vec![(
            spawn_for_analysis.reads.clone(),
            spawn_for_analysis.writes.clone(),
        )];
        let resource_var_sets = vec![spawn_for_analysis.resource_vars.clone()];
        let plan = crate::frontend::core::spawn::analysis::build_execution_plan(
            &read_write_sets,
            &resource_var_sets,
        );

        // 16. 生成 SpawnFromList 指令
        //     closures_list 参数是闭包列表寄存器（运行时动态收集）
        instructions.push(Instruction::SpawnFromList {
            closures_list: closures_list_reg.clone(),
            plan,
            result: result_reg.clone(),
            span: self.cur_span,
        });

        // 17. 退出 spawn 作用域
        self.exit_scope();

        Ok(())
    }
}
