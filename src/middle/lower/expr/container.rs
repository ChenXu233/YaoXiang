//! 容器与聚合表达式（tuple / index / dict / list / 内建容器 / 列表推导）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

impl AstToIrGenerator {
    pub(super) fn generate_tuple_expr_ir(
        &mut self,
        items: &Vec<Expr>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // SPEC §3.6 元组字面量：逐个求值元素，用 NewTuple 一次性构造
        let mut item_regs = Vec::with_capacity(items.len());
        for item_expr in items {
            let item_reg = self.next_temp_reg();
            self.generate_expr_ir(item_expr, item_reg, instructions, constants)?;
            item_regs.push(Operand::Local(item_reg));
        }
        instructions.push(Instruction::NewTuple {
            dst: Operand::Local(result_reg),
            items: item_regs,
            span: self.cur_span,
        });
        Ok(())
    }

    pub(super) fn generate_index_expr_ir(
        &mut self,
        expr: &Expr,
        index: &Expr,
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let src_reg = self.next_temp_reg();
        self.generate_expr_ir(expr, src_reg, instructions, constants)?;

        let index_reg = self.next_temp_reg();
        self.generate_expr_ir(index, index_reg, instructions, constants)?;

        // RFC-011b：实现 Index 接口的用户容器——原生 LoadIndex 替换为
        // `Call "Type.index"`（接收者居首，与 method_bindings 形态一致）
        if let Some((type_name, method, _negate)) = self.operator_dispatches.get(span).cloned() {
            let short_name = format!("{}.{}", type_name, method);
            let func_name =
                if let Some(qualified) = self.registry.short_to_qualified_map().get(&short_name) {
                    qualified.clone()
                } else {
                    short_name
                };
            // WBS 4.9.8 修复轮 R1 / issue #465：算子接口派发与普通方法调用同一判据——
            // 接收者是导入类型时把裸类型前缀换成它的定义单元限定名（`Temp.index`
            // → `b.Temp.index`），否则同名类型的算子实现互相落错单元（静默错值）。
            let dispatch_name = self.method_dispatch_name(expr, &func_name);
            instructions.push(Instruction::Call {
                dst: Some(Operand::Local(result_reg)),
                func: Operand::Const(ConstValue::String(dispatch_name)),
                args: vec![Operand::Local(src_reg), Operand::Local(index_reg)],
                span: *span,
                def: None,
            });
            return Ok(());
        }

        // #385 定案 D3：String 基底下标脱糖为 StringGetChar——标量值下标、
        // 产出单字符 String（与 chars()/substring 同表示），越界运行时报
        // E6003。其余容器走 LoadIndex。
        let base_is_string = self.get_expr_mono_type(expr).is_some_and(|t| {
            let mut r = t;
            while let MonoType::Ref { inner, .. } = r {
                r = *inner;
            }
            r.is_string()
        });
        if base_is_string {
            instructions.push(Instruction::StringGetChar {
                dst: Operand::Local(result_reg),
                src: Operand::Local(src_reg),
                index: Operand::Local(index_reg),
                span: *span,
            });
            return Ok(());
        }

        instructions.push(Instruction::LoadIndex {
            dst: Operand::Local(result_reg),
            src: Operand::Local(src_reg),
            index: Operand::Local(index_reg),
            span: *span,
        });
        Ok(())
    }

    pub(super) fn generate_dict_expr_ir(
        &mut self,
        pairs: &Vec<(Expr, Expr)>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 字典字面量：使用 NewDict 指令一次性创建
        let mut keys = Vec::new();
        let mut values = Vec::new();
        for (key_expr, val_expr) in pairs {
            let key_reg = self.next_temp_reg();
            self.generate_expr_ir(key_expr, key_reg, instructions, constants)?;
            keys.push(Operand::Local(key_reg));
            let val_reg = self.next_temp_reg();
            self.generate_expr_ir(val_expr, val_reg, instructions, constants)?;
            values.push(Operand::Local(val_reg));
        }
        instructions.push(Instruction::NewDict {
            dst: Operand::Local(result_reg),
            keys,
            values,
            span: self.cur_span,
        });
        Ok(())
    }

    pub(super) fn generate_list_expr_ir(
        &mut self,
        elements: &[Expr],
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 列表字面量：先创建空列表，再按索引写入元素
        instructions.push(Instruction::AllocArray {
            dst: Operand::Local(result_reg),
            size: Operand::Const(ConstValue::Int(elements.len() as i128)),
            elem_size: Operand::Const(ConstValue::Int(1)),
            span: self.cur_span,
        });

        for (idx, element) in elements.iter().enumerate() {
            let element_reg = self.next_temp_reg();
            self.generate_expr_ir(element, element_reg, instructions, constants)?;

            let index_reg = self.next_temp_reg();
            instructions.push(Instruction::Load {
                dst: Operand::Local(index_reg),
                src: Operand::Const(ConstValue::Int(idx as i128)),
                span: self.cur_span,
            });

            instructions.push(Instruction::StoreIndex {
                dst: Operand::Local(result_reg),
                index: Operand::Local(index_reg),
                src: Operand::Local(element_reg),
                span: *span,
            });
        }
        Ok(())
    }

    /// 生成内置容器构造（RFC-011 §「Vec(T) 的构造形式」）。
    ///
    /// 三条形态：
    /// - `Vec(Int)()` → 空容器（长度 0，元素事后追加）
    /// - `Vec(Int)(1, 2, 3)` → 元素构造（长度 = 元素个数）
    /// - `Vec(Int)(len = 64)` → 槽位分配（64 个零值槽位）
    ///
    /// 前两者与容器字面量同形，直接复用 `generate_list_expr_ir`；
    /// 第三者用 `AllocFixedArray` 分配 n 个 Void 占位（与 Array 字面量落点同款）。
    pub(super) fn generate_builtin_container_ir(
        &mut self,
        args: &[Expr],
        named_args: &[(String, Expr)],
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // `len = n`：分配 n 个零值槽位。
        // 长度是**运行期**值（RFC-011：Vec 是运行时长度的地基），
        // 故用 AllocArray 而非编译期定长的 AllocFixedArray。
        if let Some((_, len_expr)) = named_args.iter().find(|(n, _)| n == "len") {
            // 空缓冲 + 长度写入。
            //
            // 不走 `AllocArray.size`：字节码 `NewListWithCap` 的容量是**字面量**
            // （`capacity: u16`），而 RFC-011 说 `Vec(T)` 的长度是运行期值
            // （`len=self.data.length * 2`）——无法用现有字段表达动态长度。
            // 改用已测试的「长度写入」路径（`v.length = n` 同一条
            // SetField → resize(n, Void)），语义一致且无需扯动字节码格式。
            let len_reg = self.next_temp_reg();
            self.generate_expr_ir(len_expr, len_reg, instructions, constants)?;
            instructions.push(Instruction::AllocArray {
                dst: Operand::Local(result_reg),
                size: Operand::Const(ConstValue::Int(0)),
                elem_size: Operand::Const(ConstValue::Int(1)),
                span: *span,
            });
            instructions.push(Instruction::StoreField {
                dst: Operand::Local(result_reg),
                field: 0,
                src: Operand::Local(len_reg),
                type_name: None,
                field_name: Some("length".to_string()),
                span: *span,
            });
            return Ok(());
        }

        // 元素构造 / 空构造：与 `[a, b, c]` / `[]` 同形
        self.generate_list_expr_ir(args, span, result_reg, instructions, constants)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_list_comp_expr_ir(
        &mut self,
        element: &Expr,
        generators: &[ast::ListCompGenerator],
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 列表推导式 IR 生成（#401：多生成器嵌套循环，每层可带 if 过滤）
        // [e for x in a (if c)? for y in b (if d)?] 等价于:
        //   1. 创建空结果列表
        //   2. 逐层嵌套 for 循环：绑定期迭代变量 → has_next/next 取元素 →
        //      过滤条件(可选)不满足则 continue → 最内层计算 element 并 push
        //   3. 返回结果列表

        // 1. 创建空结果列表
        instructions.push(Instruction::AllocArray {
            dst: Operand::Local(result_reg),
            size: Operand::Const(ConstValue::Int(0)),
            elem_size: Operand::Const(ConstValue::Int(1)),
            span: self.cur_span,
        });

        self.generate_comp_generators_ir(
            element,
            generators,
            0,
            span,
            result_reg,
            instructions,
            constants,
        )
    }

    /// 递归展开第 `idx` 层生成器子句：非最内层时循环体是下一层子句，
    /// 最内层循环体是元素计算 + push。过滤条件失败直接跳回本层循环头
    /// （has_next 检查），即 continue 语义。
    #[allow(clippy::too_many_arguments)]
    fn generate_comp_generators_ir(
        &mut self,
        element: &Expr,
        generators: &[ast::ListCompGenerator],
        idx: usize,
        span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let gen = &generators[idx];

        // 计算可迭代对象
        let iterable_reg = self.next_temp_reg();
        self.generate_expr_ir(&gen.iterable, iterable_reg, instructions, constants)?;

        // 创建迭代器
        let iterator_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(iterator_reg)),
            func: Operand::Const(ConstValue::String(
                self.resolver().resolve_namespace("list", &["iter"]),
            )),
            args: vec![Operand::Local(iterable_reg)],
            span: *span,
            def: None,
        });

        // 注册循环变量
        let var_reg = self.next_temp_reg();
        self.register_local(&gen.var, var_reg);

        // 循环开始
        let loop_start_idx = instructions.len();

        // has_next?
        let has_next_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(has_next_reg)),
            func: Operand::Const(ConstValue::String("std.list.has_next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
            span: *span,
            def: None,
        });

        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: Operand::Local(has_next_reg),
            target: 0, // 占位符
            span: self.cur_span,
        });

        // next element
        let element_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(element_reg)),
            func: Operand::Const(ConstValue::String("std.list.next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
            span: *span,
            def: None,
        });

        // 存储到循环变量
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: Operand::Local(element_reg),
            span: *span,
        });

        // 过滤条件（可选）：不满足则跳回循环头
        let mut cond_jump_idx = None;
        if let Some(cond_expr) = &gen.condition {
            let cond_reg = self.next_temp_reg();
            self.generate_expr_ir(cond_expr, cond_reg, instructions, constants)?;

            let j = instructions.len();
            instructions.push(Instruction::JmpIfNot {
                cond: Operand::Local(cond_reg),
                target: 0, // 占位符
                span: self.cur_span,
            });
            cond_jump_idx = Some(j);
        }

        // 循环体：内层生成器循环，或最内层的元素计算 + push
        if idx + 1 < generators.len() {
            self.generate_comp_generators_ir(
                element,
                generators,
                idx + 1,
                span,
                result_reg,
                instructions,
                constants,
            )?;
        } else {
            let comp_reg = self.next_temp_reg();
            self.generate_expr_ir(element, comp_reg, instructions, constants)?;

            instructions.push(Instruction::Call {
                dst: Some(Operand::Local(result_reg)),
                func: Operand::Const(ConstValue::String("std.list.push".to_string())),
                args: vec![Operand::Local(result_reg), Operand::Local(comp_reg)],
                span: *span,
                def: None,
            });
        }

        // 修复条件跳转：失败 → 跳回本层循环头
        if let Some(j) = cond_jump_idx {
            if let Instruction::JmpIfNot {
                cond: _,
                ref mut target,
                span: _,
            } = instructions[j]
            {
                *target = loop_start_idx;
            }
        }

        // 跳回循环开始
        instructions.push(Instruction::Jmp {
            target: loop_start_idx,
            span: self.cur_span,
        });

        // 修复跳出循环的跳转目标
        let end_pos = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_end_idx]
        {
            *target = end_pos;
        }
        Ok(())
    }
}
