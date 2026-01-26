//! 静态断言实现（RFC-011 Phase 4）
//!
//! 支持编译期断言检查：
//! - `static_assert(condition)` - 编译期布尔表达式验证
//!
//! 示例：
//! static_assert(Array[Int, 10].length == 10)
//! static_assert(factorial(5) == 120)

use crate::frontend::parser::ast;
use crate::frontend::typecheck::types::{
    ConstEvalError, ConstExpr, ConstValue,
};
use crate::util::span::Span;

/// 静态断言结构
#[derive(Debug, Clone)]
pub struct StaticAssert {
    /// 断言条件（编译期可求值的布尔表达式）
    pub condition: ConstExpr,
    /// 可选的错误消息
    pub message: Option<String>,
    /// 位置信息
    pub span: Span,
}

impl StaticAssert {
    /// 创建新的静态断言
    pub fn new(
        condition: ConstExpr,
        message: Option<String>,
        span: Span,
    ) -> Self {
        StaticAssert {
            condition,
            message,
            span,
        }
    }

    /// 创建新的静态断言（无自定义消息）
    pub fn simple(condition: ConstExpr, span: Span) -> Self {
        StaticAssert::new(condition, None, span)
    }
}

/// 静态断言检查器
pub struct StaticAssertChecker {
    /// Const求值器
    evaluator: crate::frontend::const_eval::const_evaluator::ConstEvaluator,
    /// 函数求值器（用于Const函数调用）
    fn_evaluator: crate::frontend::const_eval::const_fn::ConstFnEvaluator,
}

impl StaticAssertChecker {
    /// 创建新的静态断言检查器
    pub fn new() -> Self {
        StaticAssertChecker {
            evaluator: crate::frontend::const_eval::const_evaluator::ConstEvaluator::new(),
            fn_evaluator: crate::frontend::const_eval::const_fn::ConstFnEvaluator::new(),
        }
    }

    /// 创建新的静态断言检查器（带求值器）
    pub fn with_evaluators(
        evaluator: crate::frontend::const_eval::const_evaluator::ConstEvaluator,
        fn_evaluator: crate::frontend::const_eval::const_fn::ConstFnEvaluator,
    ) -> Self {
        StaticAssertChecker {
            evaluator,
            fn_evaluator,
        }
    }

    /// 检查静态断言
    pub fn check(&mut self, assert: &StaticAssert) -> Result<(), StaticAssertError> {
        // 求值条件表达式
        let result = self.evaluate_condition(&assert.condition)?;

        // 检查结果是否为布尔值
        let is_true = match result {
            ConstValue::Bool(b) => b,
            _ => {
                return Err(StaticAssertError::NonBoolAssert {
                    found: result,
                    span: assert.span,
                });
            }
        };

        // 如果条件不满足，返回错误
        if !is_true {
            Err(StaticAssertError::AssertionFailed {
                condition: self.expr_to_string(&assert.condition),
                message: assert.message.clone(),
                span: assert.span,
            })
        } else {
            Ok(())
        }
    }

    /// 求值条件表达式
    fn evaluate_condition(&mut self, expr: &ConstExpr) -> Result<ConstValue, StaticAssertError> {
        match expr {
            ConstExpr::Lit(value) => Ok(value.clone()),
            ConstExpr::BinOp { op, left, right } => {
                let left_val = self.evaluate_condition(left)?;
                let right_val = self.evaluate_condition(right)?;
                self.evaluate_binop(*op, &left_val, &right_val)
            }
            ConstExpr::UnOp { op, expr } => {
                let val = self.evaluate_condition(expr)?;
                self.evaluate_unop(*op, &val)
            }
            ConstExpr::Call { func, args } => {
                // 尝试作为Const函数调用求值
                let mut arg_exprs = Vec::new();
                for arg in args {
                    arg_exprs.push(arg.clone());
                }
                match self.fn_evaluator.evaluate_call(func, &arg_exprs) {
                    Ok(val) => Ok(val),
                    Err(e) => Err(StaticAssertError::ConstEvalFailed {
                        error: e,
                        span: Span::default(),
                    }),
                }
            }
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.evaluate_condition(condition)?;
                match cond_val {
                    ConstValue::Bool(true) => self.evaluate_condition(then_branch),
                    ConstValue::Bool(false) => self.evaluate_condition(else_branch),
                    _ => Err(StaticAssertError::NonBoolAssert {
                        found: cond_val,
                        span: Span::default(),
                    }),
                }
            }
            _ => Err(StaticAssertError::NonBoolAssert {
                found: ConstValue::Bool(false),
                span: Span::default(),
            }),
        }
    }

    /// 求值二元运算
    fn evaluate_binop(
        &self,
        op: crate::frontend::typecheck::types::BinOp,
        left: &ConstValue,
        right: &ConstValue,
    ) -> Result<ConstValue, StaticAssertError> {
        match op {
            crate::frontend::typecheck::types::BinOp::Eq => {
                Ok(ConstValue::Bool(left == right))
            }
            crate::frontend::typecheck::types::BinOp::Ne => {
                Ok(ConstValue::Bool(left != right))
            }
            crate::frontend::typecheck::types::BinOp::Lt => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Bool(l < r))
                    }
                    _ => Err(StaticAssertError::ConstEvalFailed {
                        error: ConstEvalError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Le => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Bool(l <= r))
                    }
                    _ => Err(StaticAssertError::ConstEvalFailed {
                        error: ConstEvalError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Gt => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Bool(l > r))
                    }
                    _ => Err(StaticAssertError::ConstEvalFailed {
                        error: ConstEvalError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                }
            }
            crate::frontend::typecheck::types::BinOp::Ge => {
                match (left, right) {
                    (ConstValue::Int(l), ConstValue::Int(r)) => {
                        Ok(ConstValue::Bool(l >= r))
                    }
                    _ => Err(StaticAssertError::ConstEvalFailed {
                        error: ConstEvalError::TypeMismatch {
                            expected: "Int".to_string(),
                            found: format!("{} and {}", left.kind().type_name(), right.kind().type_name()),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                }
            }
            _ => Err(StaticAssertError::ConstEvalFailed {
                error: ConstEvalError::CannotEvaluate {
                    reason: format!("unsupported binary operator: {:?}", op),
                    span: Span::default(),
                },
                span: Span::default(),
            }),
        }
    }

    /// 求值一元运算
    fn evaluate_unop(
        &self,
        op: crate::frontend::typecheck::types::UnOp,
        val: &ConstValue,
    ) -> Result<ConstValue, StaticAssertError> {
        match op {
            crate::frontend::typecheck::types::UnOp::Not => {
                match val {
                    ConstValue::Bool(b) => Ok(ConstValue::Bool(!*b)),
                    _ => Err(StaticAssertError::ConstEvalFailed {
                        error: ConstEvalError::TypeMismatch {
                            expected: "Bool".to_string(),
                            found: val.kind().type_name().to_string(),
                            span: Span::default(),
                        },
                        span: Span::default(),
                    }),
                }
            }
            _ => Err(StaticAssertError::ConstEvalFailed {
                error: ConstEvalError::CannotEvaluate {
                    reason: format!("unsupported unary operator: {:?}", op),
                    span: Span::default(),
                },
                span: Span::default(),
            }),
        }
    }

    /// 将表达式转换为字符串（用于错误消息）
    fn expr_to_string(&self, expr: &ConstExpr) -> String {
        match expr {
            ConstExpr::Lit(value) => value.to_string(),
            ConstExpr::Var(var) => format!("{}", var),
            ConstExpr::BinOp { op, left, right } => {
                format!(
                    "{} {} {}",
                    self.expr_to_string(left),
                    op,
                    self.expr_to_string(right)
                )
            }
            ConstExpr::UnOp { op, expr } => {
                format!("{}{}", op, self.expr_to_string(expr))
            }
            ConstExpr::Call { func, args } => {
                format!(
                    "{}({})",
                    func,
                    args.iter()
                        .map(|a| self.expr_to_string(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                format!(
                    "if {} then {} else {}",
                    self.expr_to_string(condition),
                    self.expr_to_string(then_branch),
                    self.expr_to_string(else_branch)
                )
            }
            ConstExpr::Range { start, end } => {
                format!(
                    "{}..{}",
                    self.expr_to_string(start),
                    self.expr_to_string(end)
                )
            }
        }
    }

    /// 获取求值器（用于调试）
    pub fn evaluator(&self) -> &crate::frontend::const_eval::const_evaluator::ConstEvaluator {
        &self.evaluator
    }

    /// 获取函数求值器（用于调试）
    pub fn fn_evaluator(&self) -> &crate::frontend::const_eval::const_fn::ConstFnEvaluator {
        &self.fn_evaluator
    }
}

impl Default for StaticAssertChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 静态断言错误
#[derive(Debug, Clone)]
pub enum StaticAssertError {
    /// 断言失败
    AssertionFailed {
        condition: String,
        message: Option<String>,
        span: Span,
    },
    /// 非布尔断言（条件表达式不是布尔类型）
    NonBoolAssert {
        found: ConstValue,
        span: Span,
    },
    /// Const求值失败
    ConstEvalFailed {
        error: ConstEvalError,
        span: Span,
    },
}

impl fmt::Display for StaticAssertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StaticAssertError::AssertionFailed {
                condition,
                message,
                ..
            } => {
                write!(f, "static assertion failed: {}", condition)?;
                if let Some(msg) = message {
                    write!(f, " - {}", msg)?;
                }
                Ok(())
            }
            StaticAssertError::NonBoolAssert { found, .. } => {
                write!(
                    f,
                    "static assertion condition must be Bool, found {}",
                    found.kind().type_name()
                )
            }
            StaticAssertError::ConstEvalFailed { error, .. } => {
                write!(f, "constant evaluation failed: {}", error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_assert_creation() {
        let condition = ConstExpr::Lit(ConstValue::Bool(true));
        let assert = StaticAssert::simple(condition, Span::default());
        assert!(assert.message.is_none());
    }

    #[test]
    fn test_static_assert_creation_with_message() {
        let condition = ConstExpr::Lit(ConstValue::Bool(true));
        let message = Some("test message".to_string());
        let assert = StaticAssert::new(condition, message, Span::default());
        assert!(assert.message.is_some());
        assert_eq!(assert.message.unwrap(), "test message");
    }

    #[test]
    fn test_check_true_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Bool(true));
        let assert = StaticAssert::simple(condition, Span::default());

        assert!(checker.check(&assert).is_ok());
    }

    #[test]
    fn test_check_false_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Bool(false));
        let assert = StaticAssert::simple(condition, Span::default());

        match checker.check(&assert) {
            Err(StaticAssertError::AssertionFailed { condition, .. }) => {
                assert_eq!(condition, "false");
            }
            _ => panic!("Expected AssertionFailed error"),
        }
    }

    #[test]
    fn test_check_comparison_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::BinOp {
            op: crate::frontend::typecheck::types::BinOp::Eq,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        let assert = StaticAssert::simple(condition, Span::default());

        assert!(checker.check(&assert).is_ok());
    }

    #[test]
    fn test_check_non_bool_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Int(42));
        let assert = StaticAssert::simple(condition, Span::default());

        match checker.check(&assert) {
            Err(StaticAssertError::NonBoolAssert { found, .. }) => {
                assert!(matches!(found, ConstValue::Int(_)));
            }
            _ => panic!("Expected NonBoolAssert error"),
        }
    }

    #[test]
    fn test_check_assertion_with_custom_message() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Bool(false));
        let message = Some("Expected factorial(5) == 120".to_string());
        let assert = StaticAssert::new(condition, message, Span::default());

        match checker.check(&assert) {
            Err(StaticAssertError::AssertionFailed { message, .. }) => {
                assert!(message.is_some());
                assert_eq!(message.unwrap(), "Expected factorial(5) == 120");
            }
            _ => panic!("Expected AssertionFailed error"),
        }
    }

    #[test]
    fn test_expr_to_string() {
        let checker = StaticAssertChecker::new();

        // 测试字面量
        let expr = ConstExpr::Lit(ConstValue::Int(42));
        assert_eq!(checker.expr_to_string(&expr), "42");

        // 测试二元运算
        let expr = ConstExpr::BinOp {
            op: crate::frontend::typecheck::types::BinOp::Add,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(20))),
        };
        assert_eq!(checker.expr_to_string(&expr), "10 + 20");

        // 测试比较运算
        let expr = ConstExpr::BinOp {
            op: crate::frontend::typecheck::types::BinOp::Eq,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        assert_eq!(checker.expr_to_string(&expr), "42 == 42");
    }
}
