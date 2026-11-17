//! 控制流表达式（if / match / 模式编译 / return / break / continue / for）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast::Expr;
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;
use std::collections::HashMap;

impl AstToIrGenerator {
    /// 生成 if 表达式的 IR
    #[allow(clippy::too_many_arguments)]
    pub(in crate::middle) fn generate_if_expr_ir(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        else_if_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 进入新的作用域
        self.enter_scope();

        // RFC-010a 规则③：`if` 无 `else` → 值为 `Void`。
        // 此时分支的值不写入 result_reg（改写 scratch），最后统一写 Void；
        // 否则 `x = if true { 19 }` 会拿到 19，与 typecheck 判定的 `x : Void` 矛盾（#346）。
        let value_reg = if else_branch.is_some() {
            result_reg
        } else {
            self.next_temp_reg()
        };

        // 1. 评估条件
        let condition_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, condition_reg, instructions, constants)?;

        // 2. 跳转到下一个分支的占位符 (JmpIfNot to next)
        let jump_to_next_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot {
            cond: Operand::Local(condition_reg),
            target: 0,
            span: self.cur_span,
        }); // 占位符

        // 3. then 分支
        let then_result_reg = self.next_temp_reg();
        self.generate_block_ir(then_branch, Some(then_result_reg), instructions, constants)?;
        instructions.push(Instruction::Move {
            dst: Operand::Local(value_reg),
            src: Operand::Local(then_result_reg),
            span: self.cur_span,
        });

        // 4. 跳转到结束 (Jmp to end)
        let mut jumps_to_end = Vec::new();
        let jmp_idx = instructions.len();
        instructions.push(Instruction::Jmp {
            target: 0,
            span: self.cur_span,
        }); // 占位符
        jumps_to_end.push(jmp_idx);

        // 5. 修复条件跳转
        let len = instructions.len();
        if let Instruction::JmpIfNot {
            cond: _,
            ref mut target,
            span: _,
        } = instructions[jump_to_next_idx]
        {
            *target = len;
        }

        // 6. else if 分支
        for (else_if_condition, else_if_body) in else_if_branches.iter() {
            let else_if_cond_reg = self.next_temp_reg();
            self.generate_expr_ir(else_if_condition, else_if_cond_reg, instructions, constants)?;

            let jump_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot {
                cond: Operand::Local(else_if_cond_reg),
                target: 0,
                span: self.cur_span,
            });

            let else_if_res = self.next_temp_reg();
            self.generate_block_ir(else_if_body, Some(else_if_res), instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(value_reg),
                src: Operand::Local(else_if_res),
                span: self.cur_span,
            });

            let jmp_end_idx = instructions.len();
            instructions.push(Instruction::Jmp {
                target: 0,
                span: self.cur_span,
            });
            jumps_to_end.push(jmp_end_idx);
            let len = instructions.len();
            if let Instruction::JmpIfNot {
                cond: _,
                ref mut target,
                span: _,
            } = instructions[jump_idx]
            {
                *target = len;
            }
        }

        // 7. Else 分支
        if let Some(else_body) = else_branch {
            let else_res = self.next_temp_reg();
            self.generate_block_ir(else_body, Some(else_res), instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(value_reg),
                src: Operand::Local(else_res),
                span: self.cur_span,
            });
        }

        // 8. 修复所有跳转到结束的指令
        let end_len = instructions.len();
        for idx in jumps_to_end {
            if let Instruction::Jmp {
                ref mut target,
                span: _,
            } = instructions[idx]
            {
                *target = end_len;
            }
        }

        // 9. 无 else：值为 Void（RFC-010a 规则③）
        if else_branch.is_none() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::Void),
                span: self.cur_span,
            });
        }

        self.exit_scope();
        Ok(())
    }

    pub(super) fn generate_return_expr_ir(
        &mut self,
        expr: &Option<Box<Expr>>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 生成返回指令
        if let Some(e) = expr {
            self.generate_expr_ir(e, result_reg, instructions, constants)?;
            instructions.push(Instruction::Ret {
                value: Some(Operand::Local(result_reg)),
                span: self.cur_span,
            });
        } else {
            instructions.push(Instruction::Ret {
                value: None,
                span: self.cur_span,
            });
        }
        Ok(())
    }

    pub(super) fn generate_break_expr_ir(
        &mut self,
        span: &Span,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), Diagnostic> {
        // #311: break → 跳出最近一层循环；出口生成期未知，占位后由
        // exit_loop_targets 回填。#314：标签语法已移除（Python 风格定案），
        // break 恒指最近一层循环。
        match self.loop_stack.last_mut() {
            Some(top) => {
                let fixup = instructions.len();
                instructions.push(Instruction::Jmp {
                    target: 0,
                    span: self.cur_span,
                });
                top.break_fixups.push(fixup);
            }
            None => {
                // 不变式：typecheck 已用 E1102 拦截循环外 break，走到这里说明层间失联
                return Err(ErrorCodeDefinition::ir_internal_error(
                    "break reached IR generation without enclosing loop",
                )
                .at(*span)
                .build());
            }
        }
        Ok(())
    }

    pub(super) fn generate_continue_expr_ir(
        &mut self,
        span: &Span,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), Diagnostic> {
        // #311: continue → 跳回最近一层循环的条件重判点（生成时即知）
        match self.loop_stack.last() {
            Some(top) => {
                let target = top.continue_target;
                instructions.push(Instruction::Jmp {
                    target,
                    span: self.cur_span,
                });
            }
            None => {
                return Err(ErrorCodeDefinition::ir_internal_error(
                    "continue reached IR generation without enclosing loop",
                )
                .at(*span)
                .build());
            }
        }
        Ok(())
    }

    pub(super) fn generate_match_expr_ir(
        &mut self,
        match_expr: &Expr,
        arms: &Vec<ast::MatchArm>,
        match_span: crate::util::span::Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // Match 表达式 IR 生成（RFC-010b：递归模式编译器）
        //
        // 每臂统一结构：测试链（compile_pattern 递归发射，失败跳下一臂）→
        // 守卫（可选，失败同样跳下一臂）→ 体 → Move 结果 → Jmp end。
        // 绑定进臂专属 scope（遮蔽外层，出臂恢复外层槽位映射——否则后续
        // match 以该名为 scrutinee 会读到从未写入的载荷槽 → 运行时 Void）。
        //
        // WBS 2.1.2 实测缺陷修复：结果槽先显式预置 Void——末臂守卫失败的
        // 容错落点（fail_jumps 全跳到 end）此前依赖帧 Void 预初始化（隐式，
        // frames.rs:55），汇合点读在 CFG 上存在无定义路径（verify_loose 支配性
        // 判定无法覆盖隐式语义）。显式预置使定义显式化。行为等价：容错路径
        // 读到的值同为 Void，非容错路径被臂内 Move 覆盖。
        // （Load 而非 Move：translate_move 的 src 走寄存器解析，Const 会触发
        // E3017；translate_load 的 Const 臂走 LOAD_CONST 常量表）
        instructions.push(Instruction::Load {
            dst: Operand::Local(result_reg),
            src: Operand::Const(ConstValue::Void),
            span: match_span,
        });
        //
        // 1. 评估 scrutinee（类型信息驱动嵌套模式的载荷/字段/元组类型解析）
        let scrutinee_reg = self.next_temp_reg();
        self.generate_expr_ir(match_expr, scrutinee_reg, instructions, constants)?;
        // #389：scrutinee 类型优先回查 checker 落的 span 表（推断权威），
        // miss（无 type_result 的路径）回退 AST 猜测器。猜测器对调用/内联
        // 构造/字段访问形态返回 None，Union 模式会被 #330 安全网误拦。
        let scrutinee_ty = self
            .type_result
            .as_ref()
            .and_then(|tr| tr.match_scrutinee_types.get(&match_span))
            .cloned()
            .or_else(|| self.get_expr_mono_type(match_expr));

        let mut jumps_to_end: Vec<usize> = Vec::new();

        for arm in arms {
            // RFC-010b：守卫解包——`pat if cond => body`
            let (pat, guard): (&ast::Pattern, Option<&Expr>) = match &arm.pattern {
                ast::Pattern::Guard { pattern, condition } => (pattern, Some(condition)),
                p => (p, None),
            };

            let mut fail_jumps: Vec<usize> = Vec::new();
            let mut binds: Vec<(String, usize)> = Vec::new();
            // 预分配绑定寄存器：或模式各备选对同名绑定的载荷提取落到
            // 同一寄存器（体读到的必然是命中备选写入的值）
            let bind_names = Self::collect_binding_names(pat);
            let mut pre: HashMap<String, usize> = HashMap::new();
            for bn in &bind_names {
                pre.insert(bn.clone(), self.next_temp_reg());
            }
            let needs_scope = !bind_names.is_empty();
            if needs_scope {
                self.enter_scope();
            }
            self.compile_pattern(
                scrutinee_reg,
                pat,
                scrutinee_ty.as_ref(),
                &pre,
                &mut fail_jumps,
                &mut binds,
                instructions,
                constants,
            )?;
            for (bn, reg) in &binds {
                self.register_local(bn, *reg);
            }

            // 守卫：绑定已注册（对守卫可见），失败同样落到下一臂
            if let Some(cond) = guard {
                let g_reg = self.next_temp_reg();
                self.generate_expr_ir(cond, g_reg, instructions, constants)?;
                let j = instructions.len();
                instructions.push(Instruction::JmpIfNot {
                    cond: Operand::Local(g_reg),
                    target: 0,
                    span: self.cur_span,
                });
                fail_jumps.push(j);
            }

            // 生成 arm body，结果放入 result_reg
            let arm_result_reg = self.next_temp_reg();
            self.generate_block_ir(&arm.body, Some(arm_result_reg), instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(result_reg),
                src: Operand::Local(arm_result_reg),
                span: self.cur_span,
            });

            // 跳转到 match 结束
            let jmp_end_idx = instructions.len();
            instructions.push(Instruction::Jmp {
                target: 0,
                span: self.cur_span,
            });
            jumps_to_end.push(jmp_end_idx);

            // 绑定作用域结束（恢复外层槽位映射）
            if needs_scope {
                self.exit_scope();
            }

            // 修复本臂全部失败跳转 → 下一臂起点
            let next_arm_pos = instructions.len();
            for j in fail_jumps {
                if let Instruction::JmpIfNot {
                    cond: _,
                    ref mut target,
                    span: _,
                } = instructions[j]
                {
                    *target = next_arm_pos;
                }
            }
        }

        // 修复所有跳转到结束的指令
        let end_pos = instructions.len();
        for idx in jumps_to_end {
            if let Instruction::Jmp {
                ref mut target,
                span: _,
            } = instructions[idx]
            {
                *target = end_pos;
            }
        }
        Ok(())
    }

    /// RFC-010b：递归模式编译——按 `pattern` 匹配 `reg` 中的值。
    ///
    /// 失败路径：每处比较发射 JmpIfNot 占位，调用方回填 target = 下一臂起点；
    /// 绑定：`(名字, 寄存器)` 收集进 `binds`，由调用方统一 register_local
    ///（守卫与体都在注册后生成，绑定对两者可见）。
    /// `ty`：被匹配值的静态类型——Union 载荷 / 元组元素 / 结构体字段的
    /// 嵌套模式类型由此解析；None 时仅 Wildcard/Identifier/Literal 可用。
    #[allow(clippy::too_many_arguments)]
    fn compile_pattern(
        &mut self,
        reg: usize,
        pattern: &ast::Pattern,
        ty: Option<&MonoType>,
        pre: &HashMap<String, usize>,
        fail: &mut Vec<usize>,
        binds: &mut Vec<(String, usize)>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        let cur_span = self.cur_span;
        let unsupported = |label: &str| {
            ErrorCodeDefinition::ir_unsupported_pattern(label)
                .at(cur_span)
                .build()
        };
        match pattern {
            ast::Pattern::Wildcard => Ok(()),
            ast::Pattern::Identifier(bn) => {
                if let Some(r) = pre.get(bn) {
                    // 或模式共享寄存器：把值拷进臂级预分配槽（仅命中路径执行）
                    instructions.push(Instruction::Move {
                        dst: Operand::Local(*r),
                        src: Operand::Local(reg),
                        span: self.cur_span,
                    });
                    binds.push((bn.clone(), *r));
                } else {
                    binds.push((bn.clone(), reg));
                }
                Ok(())
            }
            ast::Pattern::Literal(lit) => {
                let const_val = match lit {
                    ast::Literal::Int(n) => ConstValue::Int(*n),
                    ast::Literal::Float(f) => ConstValue::Float(*f),
                    ast::Literal::Bool(b) => ConstValue::Bool(*b),
                    ast::Literal::String(s) => ConstValue::String(s.clone()),
                    ast::Literal::Char(c) => ConstValue::Char(*c),
                    ast::Literal::Void => ConstValue::Void,
                };
                constants.push(const_val.clone());
                let cmp_reg = self.next_temp_reg();
                instructions.push(Instruction::Load {
                    dst: Operand::Local(cmp_reg),
                    src: Operand::Const(const_val),
                    span: self.cur_span,
                });
                let eq_reg = self.next_temp_reg();
                instructions.push(Instruction::Eq {
                    dst: Operand::Local(eq_reg),
                    lhs: Operand::Local(reg),
                    rhs: Operand::Local(cmp_reg),
                    span: self.cur_span,
                });
                let j = instructions.len();
                instructions.push(Instruction::JmpIfNot {
                    cond: Operand::Local(eq_reg),
                    target: 0,
                    span: self.cur_span,
                });
                fail.push(j);
                Ok(())
            }
            ast::Pattern::Tuple(ps) => {
                let Some(MonoType::Generic { name, args }) = ty else {
                    return Err(unsupported("tuple"));
                };
                if name != "Tuple" {
                    return Err(unsupported("tuple"));
                }
                if args.len() != ps.len() {
                    return Err(unsupported("tuple arity"));
                }
                for (i, p) in ps.iter().enumerate() {
                    let elem_reg = self.next_temp_reg();
                    instructions.push(Instruction::LoadField {
                        dst: Operand::Local(elem_reg),
                        src: Operand::Local(reg),
                        field: i,
                        span: self.cur_span,
                    });
                    self.compile_pattern(
                        elem_reg,
                        p,
                        Some(&args[i]),
                        pre,
                        fail,
                        binds,
                        instructions,
                        constants,
                    )?;
                }
                Ok(())
            }
            ast::Pattern::Struct { name, fields } => {
                // 非泛型结构体（struct_definitions 只收非泛型；泛型结构体
                // 模式由 typecheck 前置拒绝——此处防御性报 unsupported）
                let Some(decl_fields) = self.struct_definitions.get(name).cloned() else {
                    return Err(unsupported(name));
                };
                for (fname, _fmut, fpat) in fields {
                    let Some(pos) = decl_fields.iter().position(|f| &f.name == fname) else {
                        return Err(unsupported(fname));
                    };
                    let field_ty = MonoType::from(decl_fields[pos].ty.clone());
                    let elem_reg = self.next_temp_reg();
                    instructions.push(Instruction::LoadField {
                        dst: Operand::Local(elem_reg),
                        src: Operand::Local(reg),
                        field: pos,
                        span: self.cur_span,
                    });
                    self.compile_pattern(
                        elem_reg,
                        fpat,
                        Some(&field_ty),
                        pre,
                        fail,
                        binds,
                        instructions,
                        constants,
                    )?;
                }
                Ok(())
            }
            ast::Pattern::Union {
                variant,
                pattern: payload,
                ..
            } => {
                // 和类型上下文从被匹配类型解析（载荷位可能是另一和类型）
                let mut stripped = ty.cloned();
                while let Some(MonoType::Ref { inner, .. }) = stripped {
                    stripped = Some(*inner);
                }
                let (sum_name, args): (String, Vec<MonoType>) = match &stripped {
                    Some(MonoType::Generic { name, args })
                        if self.sum_type_variants.contains_key(name) =>
                    {
                        (name.clone(), args.clone())
                    }
                    Some(MonoType::Struct(s)) if self.sum_type_variants.contains_key(&s.name) => {
                        (s.name.clone(), Vec::new())
                    }
                    Some(MonoType::TypeRef(n))
                        if self.sum_type_variants.contains_key(n.as_str()) =>
                    {
                        (n.clone(), Vec::new())
                    }
                    _ => return Err(unsupported(variant)),
                };
                let Some(variants) = self.sum_type_variants.get(&sum_name).cloned() else {
                    return Err(unsupported(variant));
                };
                let Some((variant_index, params)) = variants
                    .iter()
                    .enumerate()
                    .find(|(_, (n, _))| n == variant)
                    .map(|(i, (_, params))| (i, params.clone()))
                else {
                    return Err(unsupported(variant));
                };
                // tag 比较（VariantTag 取声明序 tag）
                let tag_reg = self.next_temp_reg();
                instructions.push(Instruction::VariantTag {
                    dst: Operand::Local(tag_reg),
                    obj: Operand::Local(reg),
                    group: sum_name.clone(),
                    span: self.cur_span,
                });
                let idx_reg = self.next_temp_reg();
                instructions.push(Instruction::Load {
                    dst: Operand::Local(idx_reg),
                    src: Operand::Const(ConstValue::Int(variant_index as i128)),
                    span: self.cur_span,
                });
                let eq_reg = self.next_temp_reg();
                instructions.push(Instruction::Eq {
                    dst: Operand::Local(eq_reg),
                    lhs: Operand::Local(tag_reg),
                    rhs: Operand::Local(idx_reg),
                    span: self.cur_span,
                });
                let j = instructions.len();
                instructions.push(Instruction::JmpIfNot {
                    cond: Operand::Local(eq_reg),
                    target: 0,
                    span: self.cur_span,
                });
                fail.push(j);
                // 载荷
                match payload.as_deref() {
                    None => {
                        if !params.is_empty() {
                            return Err(unsupported("variant payload arity"));
                        }
                        Ok(())
                    }
                    Some(p) => {
                        if params.is_empty() {
                            return Err(unsupported("variant payload arity"));
                        }
                        let payload_reg = self.next_temp_reg();
                        instructions.push(Instruction::VariantPayload {
                            dst: Operand::Local(payload_reg),
                            obj: Operand::Local(reg),
                            group: sum_name.clone(),
                            span: self.cur_span,
                        });
                        // 载荷类型：变体参数按 scrutinee 实参（声明序）替换
                        let param_names = self
                            .sum_type_param_names
                            .get(&sum_name)
                            .cloned()
                            .unwrap_or_default();
                        let mut subst: HashMap<String, MonoType> = HashMap::new();
                        for (i, pn) in param_names.iter().enumerate() {
                            if let Some(a) = args.get(i) {
                                subst.insert(pn.clone(), a.clone());
                            }
                        }
                        let substituted: Vec<MonoType> =
                            params.iter().map(|pty| pty.substitute(&subst)).collect();
                        let payload_ty = match substituted.len() {
                            1 => substituted.into_iter().next().unwrap(),
                            _ => MonoType::make_tuple(substituted),
                        };
                        self.compile_pattern(
                            payload_reg,
                            p,
                            Some(&payload_ty),
                            pre,
                            fail,
                            binds,
                            instructions,
                            constants,
                        )
                    }
                }
            }
            ast::Pattern::Or(alts) => {
                // 各备选顺序发射：备选 i 的测试链失败 → 备选 i+1 起点
                //（继续尝试）；命中则提取载荷后直接跳臂体（跳过后续备选）。
                // 末位备选失败 → 下一臂（fail 交给调用方），成功自然落穿。
                let last = alts.len().saturating_sub(1);
                let mut success_jumps: Vec<usize> = Vec::new();
                for (i, alt) in alts.iter().enumerate() {
                    let mut alt_fail: Vec<usize> = Vec::new();
                    self.compile_pattern(
                        reg,
                        alt,
                        ty,
                        pre,
                        &mut alt_fail,
                        binds,
                        instructions,
                        constants,
                    )?;
                    if i < last {
                        let sj = instructions.len();
                        instructions.push(Instruction::Jmp {
                            target: 0,
                            span: self.cur_span,
                        });
                        success_jumps.push(sj);
                        let next_alt_pos = instructions.len();
                        for j in alt_fail {
                            if let Instruction::JmpIfNot {
                                cond: _,
                                ref mut target,
                                span: _,
                            } = instructions[j]
                            {
                                *target = next_alt_pos;
                            }
                        }
                    } else {
                        fail.append(&mut alt_fail);
                    }
                }
                // 命中跳转回填到臂体起点（Or 全部发射完成处）
                let body_pos = instructions.len();
                for sj in success_jumps {
                    if let Instruction::Jmp {
                        ref mut target,
                        span: _,
                    } = instructions[sj]
                    {
                        *target = body_pos;
                    }
                }
                Ok(())
            }
            ast::Pattern::Guard { .. } => Err(unsupported("guard")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_for_expr_ir(
        &mut self,
        var: &str,
        var_mut: &bool,
        iterable: &Expr,
        body: &ast::Block,
        for_span: &Span,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        self.generate_for_loop_ir(
            var,
            *var_mut,
            iterable,
            body,
            Some(result_reg),
            *for_span,
            instructions,
            constants,
        )?;
        Ok(())
    }
}
