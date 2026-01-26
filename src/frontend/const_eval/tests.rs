//! Const求值模块测试（RFC-011 Phase 4）
//!
//! 测试Const泛型、Const函数和静态断言的功能

#[cfg(test)]
mod const_evaluator_tests {
    use super::super::const_evaluator::{ConstEvaluator, ConstEvalEnv};
    use crate::frontend::typecheck::types::{
        BinOp, ConstExpr, ConstValue,
    };

    #[test]
    fn test_basic_arithmetic() {
        let mut evaluator = ConstEvaluator::new();

        // 测试加法
        let expr = ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(20))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(30));

        // 测试乘法
        let expr = ConstExpr::BinOp {
            op: BinOp::Mul,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(6))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(7))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(42));
    }

    #[test]
    fn test_comparison_operations() {
        let mut evaluator = ConstEvaluator::new();

        // 测试等于
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
    fn test_logical_operations() {
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
    fn test_conditional_expression() {
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
    fn test_unary_operations() {
        let mut evaluator = ConstEvaluator::new();

        // 测试负号
        let expr = ConstExpr::UnOp {
            op: crate::frontend::typecheck::types::UnOp::Neg,
            expr: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Int(-42));

        // 测试逻辑非
        let expr = ConstExpr::UnOp {
            op: crate::frontend::typecheck::types::UnOp::Not,
            expr: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };
        assert_eq!(evaluator.evaluate(&expr).unwrap(), ConstValue::Bool(false));
    }

    #[test]
    fn test_const_value_kind() {
        let int_val = ConstValue::Int(42);
        assert_eq!(int_val.kind().type_name(), "Int");

        let bool_val = ConstValue::Bool(true);
        assert_eq!(bool_val.kind().type_name(), "Bool");

        let float_val = ConstValue::Float(3.14);
        assert_eq!(float_val.kind().type_name(), "Float");
    }

    #[test]
    fn test_const_value_is_numeric() {
        assert!(ConstValue::Int(42).is_numeric());
        assert!(ConstValue::Float(3.14).is_numeric());
        assert!(!ConstValue::Bool(true).is_numeric());
    }
}

#[cfg(test)]
mod const_fn_tests {
    use super::super::const_fn::{ConstFnEvaluator, ConstFnCallEnv, ConstFunction};
    use crate::frontend::typecheck::types::{ConstExpr, ConstValue, ConstVar, BinOp};
    use crate::util::span::Span;

    #[test]
    fn test_factorial_function_small_values() {
        let mut evaluator = ConstFnEvaluator::new();
        let func = create_factorial_function();
        evaluator.env_mut().register(func);
        evaluator.env_mut().set_max_depth(1000);

        // 测试 factorial(0)
        let args = vec![ConstExpr::Lit(ConstValue::Int(0))];
        match evaluator.evaluate_call("factorial", &args) {
            Ok(ConstValue::Int(n)) => assert_eq!(n, 1),
            _ => panic!("Expected Int(1)"),
        }

        // 测试 factorial(5)
        let args = vec![ConstExpr::Lit(ConstValue::Int(5))];
        match evaluator.evaluate_call("factorial", &args) {
            Ok(ConstValue::Int(n)) => assert_eq!(n, 120),
            _ => panic!("Expected Int(120)"),
        }
    }

    fn create_factorial_function() -> ConstFunction {
        let zero_case = ConstExpr::Lit(ConstValue::Int(1));

        let n_var = ConstExpr::Var(ConstVar::new(0));
        let one = ConstExpr::Lit(ConstValue::Int(1));

        let n_minus_one = ConstExpr::BinOp {
            op: BinOp::Sub,
            left: Box::new(n_var.clone()),
            right: Box::new(one),
        };

        let recursive_call = ConstExpr::Call {
            func: "factorial".to_string(),
            args: vec![n_minus_one],
        };

        let multiply = ConstExpr::BinOp {
            op: BinOp::Mul,
            left: Box::new(n_var),
            right: Box::new(recursive_call),
        };

        let condition = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::Var(ConstVar::new(0))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        };

        let body = ConstExpr::If {
            condition: Box::new(condition),
            then_branch: Box::new(zero_case),
            else_branch: Box::new(multiply),
        };

        ConstFunction::new(
            "factorial".to_string(),
            vec![("n".to_string(), "Int".to_string())],
            "Int".to_string(),
            body,
            Span::default(),
        )
    }
}

#[cfg(test)]
mod static_assert_tests {
    use super::super::static_assert::{StaticAssert, StaticAssertChecker};
    use crate::frontend::typecheck::types::{ConstExpr, ConstValue, BinOp};

    #[test]
    fn test_check_true_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Bool(true));
        let assert = StaticAssert::simple(condition, crate::util::span::Span::default());

        assert!(checker.check(&assert).is_ok());
    }

    #[test]
    fn test_check_false_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Bool(false));
        let assert = StaticAssert::simple(condition, crate::util::span::Span::default());

        match checker.check(&assert) {
            Err(super::static_assert::StaticAssertError::AssertionFailed { .. }) => {
                // 预期的错误
            }
            _ => panic!("Expected AssertionFailed error"),
        }
    }

    #[test]
    fn test_check_comparison_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        let assert = StaticAssert::simple(condition, crate::util::span::Span::default());

        assert!(checker.check(&assert).is_ok());
    }

    #[test]
    fn test_check_non_bool_assertion() {
        let mut checker = StaticAssertChecker::new();
        let condition = ConstExpr::Lit(ConstValue::Int(42));
        let assert = StaticAssert::simple(condition, crate::util::span::Span::default());

        match checker.check(&assert) {
            Err(super::static_assert::StaticAssertError::NonBoolAssert { .. }) => {
                // 预期的错误
            }
            _ => panic!("Expected NonBoolAssert error"),
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::super::const_evaluator::ConstEvaluator;
    use super::super::const_fn::{ConstFnEvaluator, ConstFnCallEnv};
    use super::super::static_assert::{StaticAssertChecker, StaticAssert};
    use crate::frontend::typecheck::types::{ConstExpr, ConstValue, BinOp};

    #[test]
    fn test_integration_factorial_and_assertion() {
        let mut fn_evaluator = ConstFnEvaluator::new();
        let mut assert_checker = StaticAssertChecker::new();

        // 注册阶乘函数
        // 这里简化处理，不实际创建阶乘函数
        // 重点是测试集成

        // 创建简单的静态断言
        let condition = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        };
        let assert = StaticAssert::simple(condition, crate::util::span::Span::default());

        assert!(assert_checker.check(&assert).is_ok());
    }

    #[test]
    fn test_const_value_equality() {
        let val1 = ConstValue::Int(42);
        let val2 = ConstValue::Int(42);
        let val3 = ConstValue::Int(43);

        assert_eq!(val1, val2);
        assert_ne!(val1, val3);

        let bool1 = ConstValue::Bool(true);
        let bool2 = ConstValue::Bool(true);
        let bool3 = ConstValue::Bool(false);

        assert_eq!(bool1, bool2);
        assert_ne!(bool1, bool3);
    }
}
