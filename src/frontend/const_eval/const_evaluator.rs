//! Const求值引擎（RFC-011 Phase 4）
//!
//! 实现编译期常量表达式求值引擎，支持：
//! - 基本算术运算（+、-、*、/、%）
//! - 比较运算（==、!=、<、<=、>、>=）
//! - 逻辑运算（&&、||）
//! - 位运算（&、|、^、<<、>>）
//! - 一元运算（-、!、~）
//! - 条件表达式（if-then-else）
//! - 递归深度限制

use crate::frontend::typecheck::types::{
    BinOp, ConstEvalError, ConstExpr, ConstValue, UnOp,
};
use crate::util::span::Span;
use std::collections::HashMap;

/// Const求值环境
#[derive(Debug, Clone)]
pub struct ConstEvalEnv {
    /// 变量绑定（常量值）
    variables: HashMap<String, ConstValue>,
    /// 递归深度
    recursion_depth: usize,
    /// 最大递归深度
    max_depth: usize,
}

impl ConstEvalEnv新的求值环境
    pub fn new() {
    /// 创建 -> Self {
        ConstEvalEnv {
            variables: HashMap::new(),
            recursion_depth: 0,
            max_depth: 1000,
        }
    }

    /// 设置最大递归深度
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// 添加变量绑定
    pub fn bind(&mut self, name: String, value: ConstValue) {
        self.variables.insert(name, value);
    }

    /// 获取变量值
    pub fn get(&self, name: &str) -> Option<&ConstValue> {
        self.variables.get(name)
    }

    /// 增加递归深度
    fn inc_depth(&mut self) -> Result<(), ConstEvalError> {
        self.recursion_depth += 1;
        if self.recursion_depth > self.max_depth {
            Err(ConstEvalError::RecursionTooDeep {
                depth: self.recursion_depth,
                max_depth: self.max_depth,
                span: Span::default(),
            })
        } else {
            Ok(())
        }
    }

    /// 减少递归深度
    fn dec_depth(&mut self) {
        self.recursion_depth = self.recursion_depth.saturating_sub(1);
    }
}

impl Default for ConstEvalEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Const求值器
pub struct ConstEvaluator {
    /// 求值环境
    env: ConstEvalEnv,
}

impl ConstEvaluator {
    /// 创建新的Const求值器
    pub fn new() -> Self {
        ConstEvaluator {
            env: ConstEvalEnv::new(),
        }
    }

    /// 创建新的Const求值器（带环境）
    pub fn with_env(env: ConstEvalEnv) -> Self {
        ConstEvaluator { env }
    }

    /// 求值Const表达式
    pub fn evaluate(
        &mut self,
        expr: &ConstExpr,
    ) -> Result<ConstValue, ConstEvalError> {
        self.evaluate_with_depth(expr, 0)
    }

    /// 带深度的求值（内部使用）
    fn evaluate_with_depth(
        &mut self,
        expr: &ConstExpr,
        depth: usize,
    ) -> Result<ConstValue, ConstEvalError> {
        // 检查递归深度
        if depth > self.env.max_depth {
            return Err(ConstEvalError::RecursionTooDeep {
                depth,
                max_depth: self.env.max_depth,
                span: Span::default(),
            });
        }

        match expr {
            ConstExpr::Lit(value) => Ok(value.clone()),
            ConstExpr::Var(var) => {
                // 简化实现：直接使用变量索引作为值
                // 实际实现需要从环境查找
                Ok(ConstValue::Int(var.index() as i128))
            }
            ConstExpr::BinOp { op, left, right } => {
                let left_val = self.evaluate_with_depth(left, depth + 1)?;
                let right_val = self.evaluate_with_depth(right, depth + 1)?;
                self.evaluate_binop(*op, &left_val, &right_val)
            }
            ConstExpr::UnOp { op, expr } => {
                let val = self.evaluate_with_depth(expr, depth + 1)?;
                self.evaluate_unop(*op, &val)
            }
            ConstExpr::Call { func, args } => {
                // 简化实现：检查是否是内建函数
                self.evaluate_call(func, args, depth + 1)
            }
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.evaluate_with_depth(condition, depth + 1)?;
                match cond_val {
                    ConstValue::Bool(true) => {
                        self.evaluate_with_depth(then_branch, depth + 1)
                    }
                    ConstValue::Bool(false) => {
                        self.evaluate_with_depth(else_branch, depth + 1)
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "Bool".to_string(),
                        found: cond_val.kind().type_name().to_string(),
                        span: Span::default(),
                    }),
                }
            }
            ConstExpr::Range { start, end } => {
                let start_val = self.evaluate_with_depth(start, depth + 1)?;
                let end_val = self.evaluate_with_depth(end, depth + 1)?;
                Ok(ConstValue::Int(end_val.to_string().parse::<i128>().unwrap_or(0)
                    - start_val.to_string().parse::<i128>().unwrap_or(0)))
            }
        }
    }

    /// 求值二元运算
    fn evaluate_binop(
        &self,
        op: BinOp,
        left: &ConstValue,
        right: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        match op {
            // 算术运算
            BinOp::Add => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    Ok(ConstValue::Int(l.checked_add(*r).ok_or_else(|| {
                        ConstEvalError::Overflow {
                            value: format!("{} + {}", l, r),
                            ty: "Int".to_string(),
                            span: Span::default(),
                        }
                    })?))
                }
                (ConstValue::Float(l), ConstValue::Float(r)) => {
                    Ok(ConstValue::Float(l + r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Sub => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    Ok(ConstValue::Int(l.checked_sub(*r).ok_or_else(|| {
                        ConstEvalError::Overflow {
                            value: format!("{} - {}", l, r),
                            ty: "Int".to_string(),
                            span: Span::default(),
                        }
                    })?))
                }
                (ConstValue::Float(l), ConstValue::Float(r)) => {
                    Ok(ConstValue::Float(l - r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Mul => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    Ok(ConstValue::Int(l.checked_mul(*r).ok_or_else(|| {
                        ConstEvalError::Overflow {
                            value: format!("{} * {}", l, r),
                            ty: "Int".to_string(),
                            span: Span::default(),
                        }
                    })?))
                }
                (ConstValue::Float(l), ConstValue::Float(r)) => {
                    Ok(ConstValue::Float(l * r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Div => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    if *r == 0 {
                        return Err(ConstEvalError::DivisionByZero {
                            span: Span::default(),
                        });
                    }
                    Ok(ConstValue::Int(l / r))
                }
                (ConstValue::Float(l), ConstValue::Float(r)) => {
                    if *r == 0.0 {
                        return Err(ConstEvalError::DivisionByZero {
                            span: Span::default(),
                        });
                    }
                    Ok(ConstValue::Float(l / r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Mod => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    if *r == 0 {
                        return Err(ConstEvalError::DivisionByZero {
                            span: Span::default(),
                        });
                    }
                    Ok(ConstValue::Int(l % r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            // 比较运算
            BinOp::Eq => Ok(ConstValue::Bool(left == right)),
            BinOp::Ne => Ok(ConstValue::Bool(left != right)),
            BinOp::Lt => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Bool(l < r)),
                (ConstValue::Float(l), ConstValue::Float(r)) => Ok(ConstValue::Bool(l < r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Le => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Bool(l <= r)),
                (ConstValue::Float(l), ConstValue::Float(r)) => Ok(ConstValue::Bool(l <= r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Gt => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Bool(l > r)),
                (ConstValue::Float(l), ConstValue::Float(r)) => Ok(ConstValue::Bool(l > r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Ge => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Bool(l >= r)),
                (ConstValue::Float(l), ConstValue::Float(r)) => Ok(ConstValue::Bool(l >= r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            // 逻辑运算
            BinOp::And => match (left, right) {
                (ConstValue::Bool(l), ConstValue::Bool(r)) => Ok(ConstValue::Bool(*l && *r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Bool".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Or => match (left, right) {
                (ConstValue::Bool(l), ConstValue::Bool(r)) => Ok(ConstValue::Bool(*l || *r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Bool".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            // 位运算
            BinOp::BitAnd => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Int(l & r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::BitOr => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Int(l | r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::BitXor => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => Ok(ConstValue::Int(l ^ r)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Shl => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    if *r < 0 || *r >= 128 {
                        return Err(ConstEvalError::Overflow {
                            value: format!("{} << {}", l, r),
                            ty: "Int".to_string(),
                            span: Span::default(),
                        });
                    }
                    Ok(ConstValue::Int(l << r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
            BinOp::Shr => match (left, right) {
                (ConstValue::Int(l), ConstValue::Int(r)) => {
                    if *r < 0 || *r >= 128 {
                        return Err(ConstEvalError::Overflow {
                            value: format!("{} >> {}", l, r),
                            ty: "Int".to_string(),
                            span: Span::default(),
                        });
                    }
                    Ok(ConstValue::Int(l >> r))
                }
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                    span: Span::default(),
                }),
            },
        }
    }

    /// 求值一元运算
    fn evaluate_unop(
        &self,
        op: UnOp,
        val: &ConstValue,
    ) -> Result<ConstValue, ConstEvalError> {
        match op {
            UnOp::Neg => match val {
                ConstValue::Int(n) => Ok(ConstValue::Int(-*n)),
                ConstValue::Float(f) => Ok(ConstValue::Float(-*f)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "numeric".to_string(),
                    found: val.kind().type_name().to_string(),
                    span: Span::default(),
                }),
            },
            UnOp::Not => match val {
                ConstValue::Bool(b) => Ok(ConstValue::Bool(!*b)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Bool".to_string(),
                    found: val.kind().type_name().to_string(),
                    span: Span::default(),
                }),
            },
            UnOp::BitNot => match val {
                ConstValue::Int(n) => Ok(ConstValue::Int(!*n)),
                _ => Err(ConstEvalError::TypeMismatch {
                    expected: "Int".to_string(),
                    found: val.kind().type_name().to_string(),
                    span: Span::default(),
                }),
            },
        }
    }

    /// 求值函数调用（简化实现）
    fn evaluate_call(
        &mut self,
        func: &str,
        args: &[ConstExpr],
        depth: usize,
    ) -> Result<ConstValue, ConstEvalError> {
        // 简化实现：支持内建函数
        match func {
            "abs" => {
                if args.len() != 1 {
                    return Err(ConstEvalError::ArgCountMismatch {
                        expected: 1,
                        found: args.len(),
                        span: Span::default(),
                    });
                }
                let val = self.evaluate_with_depth(&args[0], depth)?;
                match val {
                    ConstValue::Int(n) => Ok(ConstValue::Int(if n < 0 { -n } else { n })),
                    ConstValue::Float(f) => Ok(ConstValue::Float(if f < 0.0 { -f } else { f })),
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "numeric".to_string(),
                        found: val.kind().type_name().to_string(),
                        span: Span::default(),
                    }),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err(ConstEvalError::ArgCountMismatch {
                        expected: 2,
                        found: args.len(),
                        span: Span::default(),
                    });
                }
                let left = self.evaluate_with_depth(&args[0], depth)?;
                let right = self.evaluate_with_depth(&args[1], depth)?;
                match (&left, &right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Int(if l < r { *l } else { *r }))
                    }
                    (ConstValue::Float(l), ConstValue::Float(r)) => {
                        Ok(ConstValue::Float(if l < r { *l } else { *r }))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "numeric".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Err(ConstEvalError::ArgCountMismatch {
                        expected: 2,
                        found: args.len(),
                        span: Span::default(),
                    });
                }
                let left = self.evaluate_with_depth(&args[0], depth)?;
                let right = self.evaluate_with_depth(&args[1], depth)?;
                match (&left, &right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Int(if l > r { *l } else { *r }))
                    }
                    (ConstValue::Float(l), ConstValue::Float(r)) => {
                        Ok(ConstValue::Float(if l > r { *l } else { *r }))
                    }
                    _ => Err(ConstEvalError::TypeMismatch {
                        expected: "numeric".to_string(),
                        found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                        span: Span::default(),
                    }),
                }
            }
            _ => Err(ConstEvalError::NonConstFunctionCall {
                func: func.to_string(),
                span: Span::default(),
            }),
        }
    }

    /// 获取环境（只读）
    pub fn env(&self) -> &ConstEvalEnv {
        &self.env
    }

    /// 获取环境（可变）
    pub fn env_mut(&mut self) -> &mut ConstEvalEnv {
        &mut self.env
    }
}

impl Default for ConstEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_literal() {
        let mut evaluator = ConstEvaluator::new();

        let int_lit = ConstExpr::Lit(ConstValue::Int(42));
        assert_eq!(evaluator.evaluate(&int_lit).unwrap(), ConstValue::Int(42));

        let bool_lit = ConstExpr::Lit(ConstValue::Bool(true));
        assert_eq!(evaluator.evaluate(&bool_lit).unwrap(), ConstValue::Bool(true));
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let mut evaluator = ConstEvaluator::new();

        // 测试加法
        let expr = ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(20))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(30));

        // 测试减法
        let expr = ConstExpr::BinOp {
            op: BinOp::Sub,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(100))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(50))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(50));

        // 测试乘法
        let expr = ConstExpr::BinOp {
            op: BinOp::Mul,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(6))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(7))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(42));
    }

    #[test]
    fn test_evaluate_comparison() {
        let mut evaluator = ConstEvaluator::new();

        // 测试相等
        let expr = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(true));

        // 测试小于
        let expr = ConstExpr::BinOp {
            op: BinOp::Lt,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(20))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(true));
    }

    #[test]
    fn test_evaluate_logical() {
        let mut evaluator = ConstEvaluator::new();

        // 测试逻辑与
        let expr = ConstExpr::BinOp {
            op: BinOp::And,
            left: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
            right: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(true));

        // 测试逻辑或
        let expr = ConstExpr::BinOp {
            op: BinOp::Or,
            left: Box::new(ConstExpr::Lit(ConstValue::Bool(false))),
            right: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(true));
    }

    #[test]
    fn test_evaluate_unary() {
        let mut evaluator = ConstEvaluator::new();

        // 测试负号
        let expr = ConstExpr::UnOp {
            op: UnOp::Neg,
            expr: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(-42));

        // 测试逻辑非
        let expr = ConstExpr::UnOp {
            op: UnOp::Not,
            expr: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(false));
    }

    #[test]
    fn test_evaluate_conditional() {
        let mut evaluator = ConstEvaluator::new();

        // 测试条件表达式（真）
        let expr = ConstExpr::If {
            condition: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
            then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
            else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(1));

        // 测试条件表达式（假）
        let expr = ConstExpr::If {
            condition: Box::new(ConstExpr::Lit(ConstValue::Bool(false))),
            then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
            else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(0));
    }

    #[test]
    fn test_recursion_depth_limit() {
        let mut evaluator = ConstEvaluator::new();
        let mut expr = ConstExpr::Lit(ConstValue::Int(0));

        // 创建深度嵌套的表达式
        for _ in 0..1001 {
            expr = ConstExpr::If {
                condition: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
                then_branch: Box::new(expr),
                else_branch: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            };
        }

        match evaluator.evaluate(&expr) {
            Err(ConstEvalError::RecursionTooDeep { depth, max_depth, .. }) => {
                assert!(depth > 0);
                assert_eq!(max_depth, 1000);
            }
            _ => panic!("Expected RecursionTooDeep error"),
        }
    }
}
