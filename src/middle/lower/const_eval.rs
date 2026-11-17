//! 常量求值族（`AstToIrGenerator::eval_const_expr`）
//!
//! 编译期常量折叠与字符串实参提取。方法体逐字搬自原 `middle/ir/ir_gen.rs`
//! （WBS 7.0.2），语义未改。

use super::AstToIrGenerator;
use crate::ast;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::ConstValue;

impl AstToIrGenerator {
    /// 尝试将表达式求值为编译时常量
    #[allow(clippy::only_used_in_recursion)]
    pub(in crate::middle) fn eval_const_expr(
        &self,
        expr: &ast::Expr,
    ) -> Option<ConstValue> {
        match expr {
            ast::Expr::Lit(literal, _) => match literal {
                ast::Literal::Int(n) => Some(ConstValue::Int(*n)),
                ast::Literal::Float(f) => Some(ConstValue::Float(*f)),
                ast::Literal::Bool(b) => Some(ConstValue::Bool(*b)),
                ast::Literal::String(s) => Some(ConstValue::String(s.clone())),
                ast::Literal::Char(c) => Some(ConstValue::Char(*c)),
                ast::Literal::Void => None,
            },
            // RFC-027: 常量表达式折叠（#261）。
            // 仅折叠纯数值/字符串/布尔的算术与比较，避开除法（除 0）与 Range/Assign。
            // 任一子表达式不能折时整条不折，与 RFC-027 §362「失败回退 Unproven」一致。
            ast::Expr::BinOp {
                op, left, right, ..
            } => {
                use ast::BinOp as B;
                let l = self.eval_const_expr(left)?;
                let r = self.eval_const_expr(right)?;
                // ponytail: 二元折叠的穷举不含 Range/Assign，后者语义不允许初始化于常量
                match (l, r) {
                    (ConstValue::Int(a), ConstValue::Int(b)) => match op {
                        B::Add => Some(ConstValue::Int(a.wrapping_add(b))),
                        B::Sub => Some(ConstValue::Int(a.wrapping_sub(b))),
                        B::Mul => Some(ConstValue::Int(a.wrapping_mul(b))),
                        // RFC-011b：`%` floor 取模（共享实现，见 util::arith）。
                        // 溢出（MIN % -1）折不出 → 返回 None 交给运行时报错。
                        B::Mod => (b != 0)
                            .then(|| crate::util::arith::floor_mod_signed(a, b))
                            .flatten()
                            .map(ConstValue::Int),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        B::Lt => Some(ConstValue::Bool(a < b)),
                        B::Le => Some(ConstValue::Bool(a <= b)),
                        B::Gt => Some(ConstValue::Bool(a > b)),
                        B::Ge => Some(ConstValue::Bool(a >= b)),
                        B::And => Some(ConstValue::Bool(a != 0 && b != 0)),
                        B::Or => Some(ConstValue::Bool(a != 0 || b != 0)),
                        // #285: 位运算/移位常量折叠（与 const_eval 一致）
                        B::BitAnd => Some(ConstValue::Int(a & b)),
                        B::BitOr => Some(ConstValue::Int(a | b)),
                        B::BitXor => Some(ConstValue::Int(a ^ b)),
                        B::Shl => (0..=63)
                            .contains(&b)
                            .then(|| ConstValue::Int(a.wrapping_shl(b as u32))),
                        B::Shr => (0..=63)
                            .contains(&b)
                            .then(|| ConstValue::Int(a.wrapping_shr(b as u32))),
                        B::Div | B::Range | B::Assign => None,
                    },
                    (ConstValue::Float(a), ConstValue::Float(b)) => match op {
                        B::Add => Some(ConstValue::Float(a + b)),
                        B::Sub => Some(ConstValue::Float(a - b)),
                        B::Mul => Some(ConstValue::Float(a * b)),
                        // RFC-011b：`%` floor 取模（符号随除数）
                        B::Mod => Some(ConstValue::Float(crate::util::arith::floor_mod_f64(a, b))),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        B::Lt => Some(ConstValue::Bool(a < b)),
                        B::Le => Some(ConstValue::Bool(a <= b)),
                        B::Gt => Some(ConstValue::Bool(a > b)),
                        B::Ge => Some(ConstValue::Bool(a >= b)),
                        // 位运算/移位仅限 Int：Float 折叠为 None（运行时同样受限）
                        B::BitAnd | B::BitOr | B::BitXor | B::Shl | B::Shr => None,
                        B::Div | B::Range | B::Assign => None,
                        B::And | B::Or => None,
                    },
                    // RFC-011b：Int~Float 混合折叠——Int 侧提升为 f64，
                    // 与运行时混合臂语义一致（Div 沿用不折纪律）
                    (ConstValue::Int(a), ConstValue::Float(b)) => match op {
                        B::Add => Some(ConstValue::Float(a as f64 + b)),
                        B::Sub => Some(ConstValue::Float(a as f64 - b)),
                        B::Mul => Some(ConstValue::Float(a as f64 * b)),
                        B::Mod => Some(ConstValue::Float(crate::util::arith::floor_mod_f64(
                            a as f64, b,
                        ))),
                        // 比较：混合 Int/Float **不折**，与运行时比较臂（无混合臂）
                        // 及 typecheck 的「比较两侧同型」纪律保持一致（B9）。
                        // 折了会让同一表达式在顶层能编、在函数体内报 E6007。
                        B::Eq | B::Neq | B::Lt | B::Le | B::Gt | B::Ge => None,
                        _ => None,
                    },
                    (ConstValue::Float(a), ConstValue::Int(b)) => match op {
                        B::Add => Some(ConstValue::Float(a + b as f64)),
                        B::Sub => Some(ConstValue::Float(a - b as f64)),
                        B::Mul => Some(ConstValue::Float(a * b as f64)),
                        B::Mod => Some(ConstValue::Float(crate::util::arith::floor_mod_f64(
                            a, b as f64,
                        ))),
                        // 比较：同上，不折
                        B::Eq | B::Neq | B::Lt | B::Le | B::Gt | B::Ge => None,
                        _ => None,
                    },
                    (ConstValue::String(a), ConstValue::String(b)) => match op {
                        B::Add => Some(ConstValue::String(format!("{a}{b}"))),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        _ => None,
                    },
                    (ConstValue::Bool(a), ConstValue::Bool(b)) => match op {
                        B::And => Some(ConstValue::Bool(a && b)),
                        B::Or => Some(ConstValue::Bool(a || b)),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            ast::Expr::UnOp { op, expr, .. } => {
                use ast::UnOp as U;
                let v = self.eval_const_expr(expr)?;
                match (op, v) {
                    (U::Neg, ConstValue::Int(n)) => Some(ConstValue::Int(n.wrapping_neg())),
                    (U::Neg, ConstValue::Float(f)) => Some(ConstValue::Float(-f)),
                    (U::Not, ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
                    _ => None,
                }
            }
            // RFC-012: F-string 常量求值
            ast::Expr::FString { segments, .. } => {
                let mut result = String::new();
                for seg in segments {
                    match seg {
                        ast::FStringSegment::Text(s) => result.push_str(s),
                        ast::FStringSegment::Interpolation { expr, format_spec } => {
                            let val = self.eval_const_expr(expr)?;
                            let val_str = match &val {
                                ConstValue::Int(n) => n.to_string(),
                                ConstValue::Float(f) => f.to_string(),
                                ConstValue::Bool(b) => b.to_string(),
                                ConstValue::String(s) => s.clone(),
                                ConstValue::Char(c) => c.to_string(),
                                ConstValue::Void => String::new(),
                                ConstValue::Bytes(b) => format!("{:?}", b),
                                // #492：原生函数值不是可格式化常量，退回运行时求值
                                ConstValue::NativeFn(_) => return None,
                                ConstValue::LibraryRef { .. } | ConstValue::ExternRef { .. } => {
                                    // FFI types cannot be formatted at compile time
                                    return None;
                                }
                            };
                            // 格式化说明符在常量求值中不处理，遇到则退回运行时
                            if format_spec.is_some() {
                                return None;
                            }
                            result.push_str(&val_str);
                        }
                    }
                }
                Some(ConstValue::String(result))
            }
            // 编译期 FFI 求值：Native.c("lib") → ConstValue::LibraryRef
            // lib("sym") → ConstValue::ExternRef
            ast::Expr::Call { func: _, args, .. } => {
                // 通过 type_result 检查调用表达式的推断类型
                if let Some(expr_type) = self.get_expr_mono_type(expr) {
                    match expr_type {
                        MonoType::LibraryRef { mechanism, .. } => {
                            // Native.c("lib") — 需要提取 lib 名字符串
                            if args.len() == 1 {
                                if let Some(lib_str) = Self::extract_string_arg(args) {
                                    return Some(ConstValue::LibraryRef {
                                        mechanism,
                                        lib: lib_str,
                                    });
                                }
                            }
                        }
                        MonoType::ExternRef { mechanism, lib, .. }
                            // lib("sym") — 需要提取 symbol 名字符串
                            if args.len() == 1 => {
                                if let Some(sym_str) = Self::extract_string_arg(args) {
                                    return Some(ConstValue::ExternRef {
                                        mechanism,
                                        lib,
                                        symbol: sym_str,
                                    });
                                }
                            }
                        _ => {}
                    }
                }
                None
            }
            // TODO: 支持更复杂的常量表达式
            _ => None,
        }
    }

    /// 从函数调用的参数列表中提取第一个字符串字面量
    fn extract_string_arg(args: &[ast::Expr]) -> Option<String> {
        args.first().and_then(|arg| match arg {
            ast::Expr::Lit(ast::Literal::String(s), _) => Some(s.clone()),
            _ => None,
        })
    }
}
