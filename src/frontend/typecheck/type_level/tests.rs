//! 类型级计算模块测试 (RFC-011 Phase 5)

#[cfg(test)]
mod type_level_tests {
    use super::super::*;
    use crate::frontend::typecheck::types::MonoType;

    // =========================================================================
    // 类型级计算引擎测试
    // =========================================================================

    #[test]
    fn test_type_level_computer_basic() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::add(3, 4);
        let result = computer.compute(&family).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(7)));
    }

    #[test]
    fn test_type_level_computer_mult() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::mult(5, 6);
        let result = computer.compute(&family).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(30)));
    }

    #[test]
    fn test_type_level_computer_fn_type() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::fn_type(vec![MonoType::Int(64), MonoType::String], MonoType::Bool);
        let result = computer.compute(&family).unwrap();
        match result {
            TypeLevelValue::Type(MonoType::Fn {
                params,
                return_type,
                ..
            }) => {
                assert_eq!(params.len(), 2);
                assert_eq!(*return_type, MonoType::Bool);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_type_level_computer_tuple_type() {
        let mut computer = TypeLevelComputer::new();
        let family = TypeFamily::tuple(vec![MonoType::Int(64), MonoType::String, MonoType::Bool]);
        let result = computer.compute(&family).unwrap();
        match result {
            TypeLevelValue::Type(MonoType::Tuple(elements)) => {
                assert_eq!(elements.len(), 3);
            }
            _ => panic!("Expected tuple type"),
        }
    }

    #[test]
    fn test_nat_add_associativity() {
        let nat1 = Nat::from_usize(1);
        let nat2 = Nat::from_usize(2);
        let nat3 = Nat::from_usize(3);

        // (1 + 2) + 3 = 6
        let result1 = nat1.add(&nat2).add(&nat3);
        // 1 + (2 + 3) = 6
        let result2 = nat1.add(&nat2.add(&nat3));

        assert_eq!(result1.to_usize(), Some(6));
        assert_eq!(result2.to_usize(), Some(6));
    }

    #[test]
    fn test_nat_mult_distributivity() {
        let nat1 = Nat::from_usize(2);
        let nat2 = Nat::from_usize(3);
        let nat3 = Nat::from_usize(4);

        // 2 * (3 + 4) = 14
        let left = nat1.mul(&nat2.add(&nat3));
        // (2 * 3) + (2 * 4) = 14
        let right = nat1.mul(&nat2).add(&nat1.mul(&nat3));

        assert_eq!(left.to_usize(), Some(14));
        assert_eq!(right.to_usize(), Some(14));
    }

    #[test]
    fn test_type_level_computer_recursion_depth() {
        let mut computer = TypeLevelComputer::new();
        computer.set_max_depth(100);

        computer.bind_var(0, TypeLevelValue::Nat(Nat::from_usize(1000)));

        let family = TypeFamily::Var {
            name: "x".to_string(),
            index: 0,
        };
        let result = computer.compute(&family).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(1000)));
    }

    // =========================================================================
    // 条件类型测试
    // =========================================================================

    #[test]
    fn test_conditional_type_if_true() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Bool(true));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::Int(64));
    }

    #[test]
    fn test_conditional_type_if_false() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Bool(false));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::String);
    }

    #[test]
    fn test_conditional_type_if_nat_zero() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Nat(Nat::Zero));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::String);
    }

    #[test]
    fn test_conditional_type_if_nat_nonzero() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Nat(Nat::from_usize(5)));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::Int(64));
    }

    #[test]
    fn test_conditional_type_match() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Type(MonoType::Int(64)));

        let arms = vec![
            TypeMatchArm::new(
                TypeMatch::ty(MonoType::Int(64)),
                ConditionalType::concrete(MonoType::Bool),
                false,
            ),
            TypeMatchArm::new(
                TypeMatch::wildcard(),
                ConditionalType::concrete(MonoType::Void),
                true,
            ),
        ];

        let cond_type = ConditionalType::match_type(ConditionalType::var("x", 0), arms);

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::Bool);
    }

    #[test]
    fn test_conditional_type_union() {
        let mut checker = ConditionalTypeChecker::new();
        let union_type = ConditionalType::union(vec![
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::Int(64)),
        ]);

        let result = checker.check(&union_type).unwrap();
        assert_eq!(result, MonoType::Int(64));
    }

    #[test]
    fn test_type_match_wildcard() {
        let pattern = TypeMatch::wildcard();
        let result = EvalResult::Type(MonoType::String);
        let mut computer = TypeLevelComputer::new();

        assert!(pattern.matches(&result, &mut computer).unwrap());
    }

    #[test]
    fn test_type_match_constructor() {
        let pattern = TypeMatch::constructor("Option", vec![]);
        let result = EvalResult::Type(MonoType::TypeRef("Option".to_string()));
        let mut computer = TypeLevelComputer::new();

        assert!(pattern.matches(&result, &mut computer).unwrap());
    }

    #[test]
    fn test_or_pattern_match() {
        let pattern = TypeMatch::or(vec![
            TypeMatch::ty(MonoType::Int(32)),
            TypeMatch::ty(MonoType::Int(64)),
        ]);

        let result1 = EvalResult::Type(MonoType::Int(32));
        let result2 = EvalResult::Type(MonoType::Int(64));
        let result3 = EvalResult::Type(MonoType::String);

        let mut computer = TypeLevelComputer::new();

        assert!(pattern.matches(&result1, &mut computer).unwrap());
        assert!(pattern.matches(&result2, &mut computer).unwrap());
        assert!(!pattern.matches(&result3, &mut computer).unwrap());
    }

    // =========================================================================
    // 类型族测试
    // =========================================================================

    #[test]
    fn test_type_level_arithmetic_add() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::add(10, 20);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(30)));
    }

    #[test]
    fn test_type_level_arithmetic_sub() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::sub(20, 5);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(15)));
    }

    #[test]
    fn test_type_level_arithmetic_mult() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::mult(7, 8);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(56)));
    }

    #[test]
    fn test_type_level_arithmetic_div() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::div(17, 5);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(3)));
    }

    #[test]
    fn test_type_level_arithmetic_mod() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::mod_(17, 5);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(2)));
    }

    #[test]
    fn test_type_level_arithmetic_pow() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelArithmetic::pow(2, 8);
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(256)));
    }

    #[test]
    fn test_type_level_comparison_eq() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::eq(5, 5);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_type_level_comparison_ne() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::neq(5, 3);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_type_level_comparison_lt() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::lt(3, 5);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_type_level_comparison_le() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::le(5, 5);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_type_level_comparison_gt() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::gt(7, 3);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_type_level_comparison_ge() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        let expr = TypeLevelComparison::ge(7, 7);
        let result = processor.compute_cmp(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Bool(true));
    }

    #[test]
    fn test_complex_arithmetic_expression() {
        let mut processor = TypeLevelArithmeticProcessor::new();
        // (10 + 20) * 3 - 15 = 75
        let expr = TypeLevelArithmetic::sub(
            TypeLevelArithmetic::mult(TypeLevelArithmetic::add(10, 20), 3),
            15,
        );
        let result = processor.compute_arith(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Nat(Nat::from_usize(75)));
    }

    // =========================================================================
    // 依赖类型测试
    // =========================================================================

    #[test]
    fn test_dependent_vector_type() {
        let mut checker = DependentTypeChecker::new();
        let vector_type = DependentType::vector(MonoType::Int(64), Nat::from_usize(5));
        let mono_type = checker.check(&vector_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_dependent_list_type() {
        let mut checker = DependentTypeChecker::new();
        let list_type = DependentType::list(MonoType::String);
        let mono_type = checker.check(&list_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_dependent_option_type() {
        let mut checker = DependentTypeChecker::new();
        let option_type = DependentType::option(MonoType::Int(32));
        let mono_type = checker.check(&option_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_vector_nil() {
        let nil_vec = DependentType::nil();
        assert!(nil_vec.is_nil());
    }

    #[test]
    fn test_vector_cons() {
        let cons_vec =
            DependentType::cons(DependentType::base(MonoType::Int(64)), DependentType::nil());
        assert!(cons_vec.is_cons());
        assert!(cons_vec.head().is_some());
        assert!(cons_vec.tail().is_some());
    }

    #[test]
    fn test_vector_concat() {
        let v1 = DependentType::vector(MonoType::Int(64), Nat::from_usize(3));
        let v2 = DependentType::vector(MonoType::Int(64), Nat::from_usize(2));
        let v_concat = VectorOps::concat(&v1, &v2).unwrap();
        assert_eq!(v_concat.length(), Some(Nat::from_usize(5)));
    }

    #[test]
    fn test_vector_len() {
        let v = DependentType::vector(MonoType::String, Nat::from_usize(10));
        assert_eq!(VectorOps::len(&v), Some(Nat::from_usize(10)));
    }

    #[test]
    fn test_vector_empty() {
        let empty = VectorOps::empty::<MonoType>();
        assert_eq!(empty.length(), Some(Nat::Zero));
    }

    #[test]
    fn test_vector_singleton() {
        let singleton = VectorOps::singleton(DependentType::base(MonoType::Int(64)));
        assert_eq!(singleton.length(), Some(Nat::from_usize(1)));
    }

    #[test]
    fn test_dependent_type_display() {
        let vec_type = DependentType::vector(MonoType::Int(64), Nat::from_usize(5));
        let s = vec_type.to_string();
        assert!(s.contains("Vector"));
        assert!(s.contains("5"));
    }

    #[test]
    fn test_dependent_type_builder_vector() {
        let mut builder = DependentTypeBuilder::new();
        let vector_type = builder
            .build_vector(MonoType::Int(64), Nat::from_usize(5))
            .unwrap();
        assert!(matches!(vector_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_dependent_type_builder_list() {
        let mut builder = DependentTypeBuilder::new();
        let list_type = builder.build_list(MonoType::String).unwrap();
        assert!(matches!(list_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_dependent_type_builder_option() {
        let mut builder = DependentTypeBuilder::new();
        let option_type = builder.build_option(MonoType::Int(32)).unwrap();
        assert!(matches!(option_type, MonoType::TypeRef(_)));
    }

    #[test]
    fn test_dependent_type_checker_bind() {
        let mut checker = DependentTypeChecker::new();
        checker.bind(0, TypeLevelValue::Nat(Nat::from_usize(42)));

        let vector_type = DependentType::vector(DependentType::var("N", 0), Nat::from_usize(10));
        let mono_type = checker.check(&vector_type).unwrap();
        assert!(matches!(mono_type, MonoType::TypeRef(_)));
    }

    // =========================================================================
    // 综合测试
    // =========================================================================

    #[test]
    fn test_vector_length_computation() {
        let mut processor = TypeLevelArithmeticProcessor::new();

        // (3 + 2) * 2 = 10
        let len = TypeLevelArithmetic::mult(TypeLevelArithmetic::add(3, 2), 2);
        let result = processor.compute_arith(&len).unwrap();

        match result {
            TypeLevelValue::Nat(n) => assert_eq!(n.to_usize(), Some(10)),
            _ => panic!("Expected Nat"),
        }
    }

    #[test]
    fn test_type_family_with_dependent_types() {
        let mut computer = TypeLevelComputer::new();

        // Vector[Int, 5] 构造器
        let vector_type = TypeFamily::list(TypeFamily::concrete(MonoType::Int(64)));
        let result = computer.compute(&vector_type).unwrap();

        match result {
            TypeLevelValue::Type(ty) => {
                assert!(matches!(ty, MonoType::TypeRef(_)));
            }
            _ => panic!("Expected Type"),
        }
    }

    #[test]
    fn test_nat_arithmetic_properties() {
        // 测试自然数算术的性质
        assert_eq!(Nat::Zero.add(&Nat::Zero).to_usize(), Some(0));
        assert_eq!(Nat::from_usize(5).is_zero(), false);
        assert_eq!(Nat::Zero.is_zero(), true);

        // 交换律：a + b = b + a
        let a = Nat::from_usize(7);
        let b = Nat::from_usize(11);
        assert_eq!(a.add(&b), b.add(&a));

        // 结合律：(a + b) + c = a + (b + c)
        let c = Nat::from_usize(3);
        assert_eq!(a.add(&b).add(&c), a.add(&b.add(&c)));
    }

    #[test]
    fn test_type_level_error_handling() {
        let mut processor = TypeLevelArithmeticProcessor::new();

        // 除零错误
        let expr = TypeLevelArithmetic::div(10, 0);
        let result = processor.compute_arith(&expr);
        assert!(result.is_err());
    }
}
