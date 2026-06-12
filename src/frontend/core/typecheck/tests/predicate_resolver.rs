//! PredicateResolver 模块测试
//!
//! 测试谓词解析器功能，包括：
//! - 字面量解析
//! - 变量解析
//! - 未知谓词处理

use crate::frontend::core::typecheck::predicate_resolver::PredicateResolver;
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, PredicateDef};
use crate::frontend::core::types::mono::MonoType;

fn make_positive_def() -> PredicateDef {
    PredicateDef {
        param_name: "x".into(),
        param_type: MonoType::Int(64),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    }
}

#[test]
fn test_resolve_positive_with_literal() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    let result = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::Literal {
            name: "5".into(),
            base_type: Box::new(MonoType::Int(64)),
            value: ConstValue::Int(5),
        }],
    );

    assert!(result.is_some());
    match result.unwrap() {
        MonoType::Refined { base, constraint } => {
            assert_eq!(*base, MonoType::Int(64));
            // 约束应是 5 > 0
            match constraint {
                ConstExpr::BinOp { op, left, right } => {
                    assert_eq!(op, BinOp::Gt);
                    assert_eq!(*left, ConstExpr::Lit(ConstValue::Int(5)));
                    assert_eq!(*right, ConstExpr::Lit(ConstValue::Int(0)));
                }
                _ => panic!("Expected BinOp"),
            }
        }
        _ => panic!("Expected Refined"),
    }
}

#[test]
fn test_resolve_positive_with_variable() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    let result =
        PredicateResolver::try_resolve(&env, "Positive", &[MonoType::TypeRef("b".into())]);

    assert!(result.is_some());
    match result.unwrap() {
        MonoType::Refined { base, constraint } => {
            assert_eq!(*base, MonoType::Int(64));
            match constraint {
                ConstExpr::BinOp { op, left, .. } => {
                    assert_eq!(op, BinOp::Gt);
                    assert_eq!(*left, ConstExpr::NamedVar("b".into()));
                }
                _ => panic!("Expected BinOp"),
            }
        }
        _ => panic!("Expected Refined"),
    }
}

#[test]
fn test_resolve_unknown_predicate_returns_none() {
    let env = TypeEnvironment::new();
    let result = PredicateResolver::try_resolve(&env, "UnknownPredicate", &[MonoType::Int(64)]);
    assert!(result.is_none());
}
