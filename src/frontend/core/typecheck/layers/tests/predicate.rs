//! Predicate 模块测试
//!
//! 测试谓词检查功能，包括：
//! - Refined 类型谓词验证
//! - 谓词为真/假的判定
//! - 非 Refined 类型直接通过

use std::collections::HashMap;

use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::{ProofContext, ProofResult};
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

#[test]
fn test_check_predicate_true() {
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("b".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(5));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(result.is_proved());
}

#[test]
fn test_check_predicate_false() {
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("b".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(!result.is_proved());
    match result {
        ProofResult::Disproved(model) => {
            assert!(model.assignments.iter().any(|(k, _)| k == "b"));
        }
        _ => panic!("Expected Disproved"),
    }
}

#[test]
fn test_check_predicate_non_refined_passes() {
    let non_refined = MonoType::Int(64);
    let bindings = HashMap::new();
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &non_refined, &bindings);
    assert!(result.is_proved()); // 非 Refined 类型直接通过
}
