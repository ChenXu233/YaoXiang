//! 运算符表达式（一元 / 二元 / `?`）
//!
//! 方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.3），语义未改。

use crate::ast;
use crate::ast::Expr;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::span::Span;

impl AstToIrGenerator {
    pub(super) fn generate_un_op_expr_ir(
        &mut self,
        op: &ast::UnOp,
        expr: &Expr,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 一元运算符
        match op {
            ast::UnOp::Deref => {
                // 解引用：*ptr
                // 生成指针表达式的 IR
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg.clone(), instructions, constants)?;

                // 生成 PtrDeref 指令
                instructions.push(Instruction::PtrDeref {
                    dst: result_reg.clone(),
                    src: src_reg.clone(),
                    span: self.cur_span,
                });
            }
            ast::UnOp::Neg => {
                // 负号：-x
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg.clone(), instructions, constants)?;
                instructions.push(Instruction::Neg {
                    dst: result_reg.clone(),
                    src: src_reg.clone(),
                    span: self.cur_span,
                });
            }
            ast::UnOp::Pos => {
                // 正号：+x（无操作）
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            ast::UnOp::Not => {
                // 逻辑非：!x
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg.clone(), instructions, constants)?;
                instructions.push(Instruction::Not {
                    dst: result_reg.clone(),
                    src: src_reg.clone(),
                    span: self.cur_span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn generate_try_expr_ir(
        &mut self,
        expr: &Expr,
        span: &Span,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // `?` 的接口驱动 lowering（RFC-011b 阶段 2 定案）：不再手写
        // VariantTag/Eq/Payload 序列，统一生成 Try 四方法调用链——
        //   is_failure(t) 为真 → Ret from_error(residual(t))
        //   为假           → 表达式值为 success(t)
        // Result / Option（std yx 实现）与用户自定义 Try 类型同一路径；
        // 方法名经 std 方法绑定限定名映射改写（用户类型用 `Type.method`
        // 原名）。typecheck 侧已拦截：未实现 Try（E1082）、外层返回类型
        // 未实现 Try（E1081）、错误类型不匹配（E1083）。
        //
        // 特化说明：方法调用不产生单态化请求，`std.` 前缀函数按 mono
        // 保留规则以未特化原版存活（解释器类型擦除执行），与显式方法
        // 调用同一执行模型。
        self.generate_expr_ir(expr, result_reg.clone(), instructions, constants)?;
        let span = *span;

        // 接收者类型名 → 四个方法的目标名。
        // 优先按 span 查 typecheck 记录的实现类型名（接收者是任意表达式时
        // 这是唯一来处）；退回表达式类型推导（局部变量形态）。
        let base_name = self.try_expr_impls.get(&span).cloned().or_else(|| {
            self.get_expr_mono_type(expr).and_then(|ty| {
                let mut ty = ty;
                while let crate::frontend::core::types::MonoType::Ref { inner, .. } = ty {
                    ty = *inner;
                }
                match ty {
                    crate::frontend::core::types::MonoType::Generic { name, .. } => Some(name),
                    crate::frontend::core::types::MonoType::Struct(s) => Some(s.name.clone()),
                    crate::frontend::core::types::MonoType::TypeRef(n) => Some(n.clone()),
                    _ => None,
                }
            })
        });
        let method_names = base_name.map(|base| {
            use crate::frontend::core::sema::operator_interfaces::TRY_METHODS;
            let qualified = self.registry.std_method_binding_qualified_map();
            // 名字集合的唯一真相源是 `TRY_METHODS`（typecheck 侧注册同表）；
            // 此处不再手抄一遍，否则改名时两边静默漂移。
            let names: Vec<String> = TRY_METHODS
                .iter()
                .map(|m| {
                    let key = format!("{}.{}", base, m);
                    let std_qualified = qualified.get(&key).cloned().unwrap_or(key);
                    // WBS 3.18.3 / issue #506：派发名同经 method_dispatch_name 单一
                    // 判据——base 是定义侧裸类型名（#465 同族），接收者类型经导入面
                    // （含别名）引入时按定义单元限定；std 已限定名与本单元类型
                    // 原样返回。可达路径上该改写与 qualify_names 的终点一致
                    // （已限定名经 resolve_call_target 原样通过），零漂移。
                    self.method_dispatch_name(expr, &std_qualified)
                })
                .collect();
            // TRY_METHODS 恒为四元（见 operator_interfaces.rs 定义）
            match <[String; 4]>::try_from(names) {
                Ok(arr) => arr,
                Err(_) => unreachable!("TRY_METHODS 必须是四个方法名"),
            }
        });
        let Some([is_failure_name, success_name, residual_name, from_error_name]) = method_names
        else {
            // typecheck 已保证接收者实现 Try；无类型名属于防御性分支
            return Err(ErrorCodeDefinition::ir_unsupported_pattern("try receiver")
                .at(span)
                .build());
        };
        let call_method = |name: &str, dst: Operand, arg: Operand| Instruction::Call {
            dst: Some(dst),
            func: Operand::Const(ConstValue::String(name.to_string())),
            args: vec![arg],
            span,
            def: None,
        };

        // is_failure(t) 为真 → 失败路径
        let f_reg = self.next_temp_reg();
        instructions.push(call_method(
            &is_failure_name,
            f_reg.clone(),
            result_reg.clone(),
        ));
        let fail_jmp_idx = instructions.len();
        instructions.push(Instruction::JmpIf {
            cond: f_reg.clone(),
            target: 0,
            span: self.cur_span,
        });
        // 成功路径：表达式值为 success(t)
        let s_reg = self.next_temp_reg();
        instructions.push(call_method(
            &success_name,
            s_reg.clone(),
            result_reg.clone(),
        ));
        instructions.push(Instruction::Load {
            dst: result_reg.clone(),
            src: s_reg.clone(),
            span: self.cur_span,
        });
        let end_jmp_idx = instructions.len();
        instructions.push(Instruction::Jmp {
            target: 0,
            span: self.cur_span,
        });
        // 失败路径：Ret from_error(residual(t))
        let fail_target = instructions.len();
        instructions[fail_jmp_idx] = Instruction::JmpIf {
            cond: f_reg.clone(),
            target: fail_target,
            span: self.cur_span,
        };
        let e_reg = self.next_temp_reg();
        instructions.push(call_method(&residual_name, e_reg.clone(), result_reg));
        let err_reg = self.next_temp_reg();
        instructions.push(call_method(&from_error_name, err_reg.clone(), e_reg));
        instructions.push(Instruction::Ret {
            value: Some(err_reg.clone()),
            span: self.cur_span,
        });
        let end_target = instructions.len();
        instructions[end_jmp_idx] = Instruction::Jmp {
            target: end_target,
            span: self.cur_span,
        };
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_bin_op_expr_ir(
        &mut self,
        op: &ast::BinOp,
        left: &Expr,
        right: &Expr,
        span: &Span,
        result_reg: Operand,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        tlog!(debug, MSG::DebugGeneratingIRBinOp, &format!("{:?}", op));
        // 二元运算
        let instr = match op {
            ast::BinOp::Assign => {
                if let Expr::Var(var_name, _) = left {
                    let local_idx = if let Some(idx) = self.lookup_local(var_name) {
                        idx
                    } else {
                        let idx = self.next_local_slot();
                        self.register_local(var_name, idx);
                        idx
                    };
                    let val_reg = self.next_temp_reg();
                    self.generate_expr_ir(right, val_reg.clone(), instructions, constants)?;

                    // 更新变量的类型信息
                    // 优先使用 typecheck 结果推导类型名，AST 推断仅作为兜底
                    let inferred = self.get_expr_type_name(right);
                    if inferred != "<unknown>" {
                        self.local_var_types.insert(var_name.clone(), inferred);
                    }
                    // WBS 4.9.8 / issue #465：赋值的**写出**类型名（构造器形态）
                    if let Some(written) = self.constructor_written_type_name(right) {
                        self.note_local_written_type(var_name, &written);
                    }

                    // 统一走 Store — 消除 Var→Var 走 Move 的特殊情况
                    instructions.push(Instruction::Store {
                        dst: Operand::Local(local_idx),
                        src: val_reg.clone(),
                        span: *span,
                    });
                    instructions.push(Instruction::Load {
                        dst: result_reg.clone(),
                        src: Operand::Local(local_idx),
                        span: self.cur_span,
                    });
                }
                return Ok(());
            }
            // #300 I 项：Range 是一等值——三槽 [start, end, step]，复用 Tuple 载体
            // （静态类型驱动一切消费，载体不透明；#302 落地 std.Range 后换正式身份）
            ast::BinOp::Range => {
                self.generate_range_construction_ir(
                    left,
                    right,
                    result_reg,
                    *span,
                    instructions,
                    constants,
                )?;
                return Ok(());
            }
            ast::BinOp::And | ast::BinOp::Or => {
                // SPEC §2.2 / RFC-010 权威语义：and/or 短路求值
                // a and b ≡ if a { b } else { false }；a or b ≡ if a { true } else { b }
                let lhs_reg = self.next_temp_reg();
                self.generate_expr_ir(left, lhs_reg.clone(), instructions, constants)?;
                let is_and = matches!(op, ast::BinOp::And);
                let short_idx = instructions.len();
                if is_and {
                    instructions.push(Instruction::JmpIfNot {
                        cond: lhs_reg.clone(),
                        target: 0,
                        span: self.cur_span,
                    });
                } else {
                    instructions.push(Instruction::JmpIf {
                        cond: lhs_reg.clone(),
                        target: 0,
                        span: self.cur_span,
                    });
                }
                self.generate_expr_ir(right, result_reg.clone(), instructions, constants)?;
                let end_idx = instructions.len();
                instructions.push(Instruction::Jmp {
                    target: 0,
                    span: self.cur_span,
                });
                // 短路值：and → false，or → true
                let sc_target = instructions.len();
                if let Instruction::JmpIf {
                    cond: _,
                    target: ref mut t,
                    span: _,
                } = instructions[short_idx]
                {
                    *t = sc_target;
                }
                if let Instruction::JmpIfNot {
                    cond: _,
                    target: ref mut t,
                    span: _,
                } = instructions[short_idx]
                {
                    *t = sc_target;
                }
                instructions.push(Instruction::Load {
                    dst: result_reg.clone(),
                    src: Operand::Const(if is_and {
                        ConstValue::Bool(false)
                    } else {
                        ConstValue::Bool(true)
                    }),
                    span: self.cur_span,
                });
                let end_target = instructions.len();
                if let Instruction::Jmp {
                    target: ref mut t,
                    span: _,
                } = instructions[end_idx]
                {
                    *t = end_target;
                }
                return Ok(());
            }
            _ => {
                let left_reg = self.next_temp_reg();
                let right_reg = self.next_temp_reg();
                self.generate_expr_ir(left, left_reg.clone(), instructions, constants)?;
                self.generate_expr_ir(right, right_reg.clone(), instructions, constants)?;

                // RFC-011b: 显式接口派发——`==`/`!=` 命中 Equal 实例化时
                // 生成方法调用（接收者居首），`!=` 对结果取反
                if let Some((type_name, method, negate)) =
                    self.operator_dispatches.get(span).cloned()
                {
                    let short_name = format!("{}.{}", type_name, method);
                    let func_name = if let Some(qualified) =
                        self.registry.short_to_qualified_map().get(&short_name)
                    {
                        qualified.clone()
                    } else {
                        short_name
                    };
                    // WBS 4.9.8 修复轮 R1 / issue #465：同 Index 臂——接收者（左操作数）
                    // 是导入类型时按定义单元限定派发名，本地类型行为不变。
                    let dispatch_name = self.method_dispatch_name(left, &func_name);
                    instructions.push(Instruction::Call {
                        dst: Some(result_reg.clone()),
                        func: Operand::Const(ConstValue::String(dispatch_name)),
                        args: vec![left_reg.clone(), right_reg.clone()],
                        span: self.cur_span,
                        def: None,
                    });
                    if negate {
                        instructions.push(Instruction::Not {
                            dst: result_reg.clone(),
                            src: result_reg.clone(),
                            span: self.cur_span,
                        });
                    }
                    return Ok(());
                }

                match op {
                    ast::BinOp::Add => Instruction::Add {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Sub => Instruction::Sub {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Mul => Instruction::Mul {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Div => Instruction::Div {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: *span,
                    },
                    ast::BinOp::Mod => Instruction::Mod {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: *span,
                    },
                    // #285: 位运算/移位（SPEC §2.2 级 7/8）
                    ast::BinOp::BitAnd => Instruction::And {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::BitOr => Instruction::Or {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::BitXor => Instruction::Xor {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Shl => Instruction::Shl {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Shr => Instruction::Shr {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Eq => Instruction::Eq {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Neq => Instruction::Ne {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Lt => Instruction::Lt {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Le => Instruction::Le {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Gt => Instruction::Gt {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    ast::BinOp::Ge => Instruction::Ge {
                        dst: result_reg.clone(),
                        lhs: left_reg.clone(),
                        rhs: right_reg.clone(),
                        span: self.cur_span,
                    },
                    // Assign 在上方分支处理；And/Or 走短路求值；Range 仅限 for/切片上下文。
                    // 剩余运算符到达此处即内部错误——禁止静默兜底（教训：&&/|| 曾静默编译为常量 0，#251）
                    _ => {
                        return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                            "unhandled binary operator: {:?}",
                            op
                        ))
                        .build());
                    }
                }
            }
        };
        instructions.push(instr);
        Ok(())
    }
}
