//! RFC-011 高级类型层测试
//!
//! 测试条件类型、依赖类型、类型级运算和Const泛型

#[cfg(test)]
mod conditional_types_tests {
    use crate::frontend::type_level::conditional_types::{
        If, MatchType, TypeCondition, conditions, MatchArm,
    };
    use crate::frontend::core::type_system::{MonoType, TypeVar};

    #[test]
    fn test_if_true_branch() {
        let if_type = If::new(
            conditions::bool(true),
            MonoType::Int,
            MonoType::String,
        );

        let result = if_type.eval();
        assert!(result.is_normalized());
        assert_eq!(result.ok().unwrap(), MonoType::Int);
    }

    #[test]
    fn test_if_false_branch() {
        let if_type = If::new(
            conditions::bool(false),
            MonoType::Int,
            MonoType::String,
        );

        let result = if_type.eval();
        assert!(result.is_normalized());
        assert_eq!(result.ok().unwrap(), MonoType::String);
    }

    #[test]
    fn test_if_pending() {
        let if_type = If::new(
            conditions::eq(MonoType::TypeVar(TypeVar::new("T")), MonoType::Int),
            MonoType::Int,
            MonoType::String,
        );

        let result = if_type.eval();
        assert!(!result.is_normalized());
    }

    #[test]
    fn test_match_type() {
        let match_type = MatchType::new(
            MonoType::Int,
            vec![
                MatchArm {
                    pattern: MonoType::Int,
                    result: MonoType::String,
                },
                MatchArm {
                    pattern: MonoType::Wildcard,
                    result: MonoType::Bool,
                },
            ],
        );

        let result = match_type.eval();
        assert!(result.is_normalized());
        assert_eq!(result.ok().unwrap(), MonoType::String);
    }

    #[test]
    fn test_match_wildcard() {
        let match_type = MatchType::with_wildcard(
            MonoType::Float,
            MonoType::String,
        );

        let result = match_type.eval();
        assert!(result.is_normalized());
        assert_eq!(result.ok().unwrap(), MonoType::String);
    }
}

#[cfg(test)]
mod type_level_operations_tests {
    use crate::frontend::type_level::operations::{
        TypeArithmetic, TypeComparison, TypeLogic, ArithOp, CmpOp, LogicOp,
        TypeLevelValue, constants,
    };

    #[test]
    fn test_arithmetic_add() {
        let arithmetic = TypeArithmetic::new();

        let result = arithmetic.add(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3));
        assert_eq!(result, Some(TypeLevelValue::Int(8)));
    }

    #[test]
    fn test_arithmetic_mul() {
        let arithmetic = TypeArithmetic::new();

        let result = arithmetic.mul(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3));
        assert_eq!(result, Some(TypeLevelValue::Int(15)));
    }

    #[test]
    fn test_arithmetic_div() {
        let arithmetic = TypeArithmetic::new();

        let result = arithmetic.div(&TypeLevelValue::Int(10), &TypeLevelValue::Int(3));
        assert_eq!(result, Some(TypeLevelValue::Int(3)));
    }

    #[test]
    fn test_comparison_eq() {
        let comparison = TypeComparison::new();

        let result = comparison.eq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(5));
        assert_eq!(result, Some(TypeLevelValue::Bool(true)));

        let result = comparison.eq(&TypeLevelValue::Int(5), &TypeLevelValue::Int(3));
        assert_eq!(result, Some(TypeLevelValue::Bool(false)));
    }

    #[test]
    fn test_comparison_lt() {
        let comparison = TypeComparison::new();

        let result = comparison.lt(&TypeLevelValue::Int(3), &TypeLevelValue::Int(5));
        assert_eq!(result, Some(TypeLevelValue::Bool(true)));
    }

    #[test]
    fn test_logic_and() {
        let logic = TypeLogic::new();

        let result = logic.and(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(true));
        assert_eq!(result, Some(TypeLevelValue::Bool(true)));

        let result = logic.and(&TypeLevelValue::Bool(true), &TypeLevelValue::Bool(false));
        assert_eq!(result, Some(TypeLevelValue::Bool(false)));
    }

    #[test]
    fn test_logic_or() {
        let logic = TypeLogic::new();

        let result = logic.or(&TypeLevelValue::Bool(false), &TypeLevelValue::Bool(true));
        assert_eq!(result, Some(TypeLevelValue::Bool(true)));
    }

    #[test]
    fn test_logic_not() {
        let logic = TypeLogic::new();

        let result = logic.not(&TypeLevelValue::Bool(true));
        assert_eq!(result, Some(TypeLevelValue::Bool(false)));
    }

    #[test]
    fn test_short_circuit() {
        let logic = TypeLogic::with_short_circuit(true);

        // 短路求值：左边为 false 时右边不会被求值
        let result = logic.and(&TypeLevelValue::Bool(false), &constants::TRUE);
        assert_eq!(result, Some(TypeLevelValue::Bool(false)));
    }
}

#[cfg(test)]
mod type_evaluation_tests {
    use crate::frontend::type_level::evaluation::{
        TypeNormalizer, TypeReducer, TypeUnifier, NormalForm,
    };
    use crate::frontend::core::type_system::MonoType;

    #[test]
    fn test_normalize_basic_types() {
        let mut normalizer = TypeNormalizer::new();

        assert_eq!(normalizer.normalize(&MonoType::Int), NormalForm::Normalized);
        assert_eq!(normalizer.normalize(&MonoType::Bool), NormalForm::Normalized);
        assert_eq!(normalizer.normalize(&MonoType::Void), NormalForm::Normalized);
    }

    #[test]
    fn test_reduce_basic() {
        let mut reducer = TypeReducer::new();

        // 基本类型无法归约
        let result = reducer.reduce(&MonoType::Int);
        assert!(matches!(result, super::super::ReductionResult::Stuck));
    }

    #[test]
    fn test_unify_same_type() {
        let mut unifier = TypeUnifier::new();

        let result = unifier.unify(&MonoType::Int, &MonoType::Int);
        assert!(matches!(result, super::super::UnificationResult::Success(_)));
    }

    #[test]
    fn test_unify_type_var() {
        let mut unifier = TypeUnifier::new();

        let tv = MonoType::TypeVar(TypeVar::new("T"));
        let result = unifier.unify(&tv, &MonoType::Int);
        assert!(matches!(result, super::super::UnificationResult::Success(_)));

        let sub = result.ok().unwrap();
        assert_eq!(sub.get("T"), Some(&MonoType::Int));
    }
}

#[cfg(test)]
mod const_generics_tests {
    use crate::frontend::type_level::const_generics::{
        eval::{ConstGenericEval, ConstExpr, ConstBinOp},
        generic_size::GenericSize,
    };
    use crate::frontend::core::type_system::ConstValue;

    #[test]
    fn test_const_eval_int() {
        let eval = ConstGenericEval::new();

        let expr = ConstExpr::Int(42);
        let result = eval.eval(&expr);
        assert_eq!(result, Ok(ConstValue::Int(42)));
    }

    #[test]
    fn test_const_eval_binop() {
        let mut eval = ConstGenericEval::new();

        let expr = ConstExpr::BinOp {
            op: ConstBinOp::Add,
            lhs: Box::new(ConstExpr::Int(10)),
            rhs: Box::new(ConstExpr::Int(32)),
        };

        let result = eval.eval(&expr);
        assert_eq!(result, Ok(ConstValue::Int(42)));
    }

    #[test]
    fn test_const_eval_function() {
        let mut eval = ConstGenericEval::new();

        // 注册 factorial 函数
        eval.register_function(
            "factorial".to_string(),
            eval::predefined::factorial(),
        );

        let expr = ConstExpr::Call {
            name: "factorial".to_string(),
            args: vec![ConstExpr::Int(5)],
        };

        let result = eval.eval(&expr);
        assert_eq!(result, Ok(ConstValue::Int(120)));
    }

    #[test]
    fn test_size_of_basic_types() {
        let size = GenericSize::new();

        assert_eq!(size.size_of(&MonoType::Int).unwrap(), 8);
        assert_eq!(size.size_of(&MonoType::Bool).unwrap(), 1);
        assert_eq!(size.size_of(&MonoType::Void).unwrap(), 0);
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::frontend::type_level::evaluation::compute::TypeComputer;
    use crate::frontend::core::type_system::MonoType;

    #[test]
    fn test_type_computer_basic() {
        let mut computer = TypeComputer::new();

        // 基本类型应该直接返回
        let result = computer.compute(&MonoType::Int);
        assert!(matches!(result, super::compute::ComputeResult::Done(_)));
    }
}
