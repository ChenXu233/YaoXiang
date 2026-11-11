//! RFC-027 阶段 1 端到端集成测试
//!
//! 验证从类型标注到证明结果的完整数据流：
//! PredicateResolver → Refined → check_predicate → ProofResult

use crate::frontend::core::sema::test_util::{binop};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::sema::environment::TypeEnvironment;
use crate::frontend::core::sema::predicate_resolver::{PredicateDef, PredicateResolver};
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, ProofResultExt};
use crate::frontend::core::sema::layers::equivalence::check_type_equivalence;
use crate::frontend::core::sema::layers::predicate::check_predicate;
use std::collections::HashMap;

// 辅助函数

/// 创建一个简单的 Positive 谓词定义
/// Positive(x): x > 0
fn make_positive_def() -> PredicateDef {
    PredicateDef {
        param_name: "x".into(),
        param_type: MonoType::Int(64),
        constraint: binop(
            BinOp::Gt,
            ConstExpr::NamedVar("x".into()),
            ConstExpr::Lit(ConstValue::Int(0)),
        ),
    }
}

// 数据流测试：PredicateResolver → ProofResult

/// 端到端：Positive(5) → Refined → eval_expr → Proved
///
/// 验证完整数据流：
/// 1. PredicateResolver 将 Positive(5) 正格化为 Refined { base: Int, constraint: 5 > 0 }
/// 2. check_predicate 求值 5 > 0 并返回 Proved
#[test]
fn test_e2e_positive_literal_proved() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    // 模拟：用户写 b: Positive(5)
    let resolved = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::Literal {
            name: "5".into(),
            base_type: Box::new(MonoType::Int(64)),
            value: ConstValue::Int(5),
        }],
    )
    .expect("Positive(5) 应被识别为已注册谓词（#263 三值语义）")
    .expect("Positive(5) 合法实参应解析成功（#263 三值语义）");

    // 检查返回的是 Refined
    assert!(matches!(resolved, MonoType::Refined { .. }));

    // 谓词验证
    let ctx = ProofContext::new(&env);
    let bindings = HashMap::new();
    // 此时约束已被正格化为 Gt(Lit(Int(5)), Lit(Int(0)))，不需要额外 binding
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(result.is_proved(), "5 > 0 应该被证明");
}

/// 端到端：Positive(0) → Refined → eval_expr → Disproved
///
/// 验证证伪路径：
/// 1. PredicateResolver 正格化为 Refined { base: Int, constraint: 0 > 0 }
/// 2. check_predicate 求值 0 > 0 为 false，返回 Disproved
#[test]
fn test_e2e_positive_zero_disproved() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    let resolved = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::Literal {
            name: "0".into(),
            base_type: Box::new(MonoType::Int(64)),
            value: ConstValue::Int(0),
        }],
    )
    .expect("Positive(0) 应被识别为已注册谓词（#263 三值语义）")
    .expect("Positive(0) 合法实参应解析成功（#263 三值语义）");

    let ctx = ProofContext::new(&env);
    let bindings = HashMap::new();
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(!result.is_proved(), "0 > 0 应该被证伪");
    match result {
        Err(ProofFailure::Disproved(_)) => {}
        _ => panic!("Expected Disproved, got {:?}", result),
    }
}

/// 端到端：Positive(b) + binding { b: -1 } → Disproved
///
/// 验证带负值的谓词检查：
/// b=-1，求值 -1 > 0 为 false
#[test]
fn test_e2e_positive_with_binding_disproved() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    let resolved =
        PredicateResolver::try_resolve(&env, "Positive", &[MonoType::TypeRef("b".into())])
            .expect("Positive(b) 应被识别为已注册谓词（#263 三值语义）")
            .expect("Positive(b) 合法实参应解析成功（#263 三值语义）");

    let ctx = ProofContext::new(&env);
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(-1));
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(!result.is_proved(), "-1 > 0 应该被证伪");
    match result {
        Err(ProofFailure::Disproved(_)) => {}
        _ => panic!("Expected Disproved"),
    }
}

/// 端到端：非谓词类型（如 Int）不被 PredicateResolver 识别
///
/// PredicateResolver::try_resolve 对非谓词名返回 None
#[test]
fn test_e2e_unknown_predicate_returns_none() {
    let env = TypeEnvironment::new();
    let result = PredicateResolver::try_resolve(&env, "UnknownPred", &[MonoType::Int(64)]);
    assert!(result.is_none());
}

// Layer 0: 类型等式测试

/// 端到端：类型等式——结构等价
#[test]
fn test_e2e_type_equivalence_structural() {
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Int(64) == Int(64)
    let result = check_type_equivalence(&ctx, &MonoType::Int(64), &MonoType::Int(64));
    assert!(result.is_proved());
}

// 回归测试：MonoType 基础操作不受影响

/// 普通类型检查不变（回归测试）
#[test]
fn test_plain_type_check_still_works() {
    // 验证 MonoType 的基础操作不受影响
    let int_type = MonoType::Int(64);
    assert!(int_type.is_numeric());
    assert!(!int_type.is_indexable());

    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Bool),
    };
    let display = format!("{}", fn_type);
    assert!(
        display.contains("int64"),
        "display should contain 'int64', got: {}",
        display
    );
    assert!(
        display.contains("bool"),
        "display should contain 'bool', got: {}",
        display
    );
}

/// 验证 TypeEnvironment 的 predicate_defs 字段可用
#[test]
fn test_type_environment_predicate_defs() {
    let mut env = TypeEnvironment::new();
    assert!(env.predicate_defs.is_empty());

    env.predicate_defs
        .insert("Test".into(), make_positive_def());
    assert_eq!(env.predicate_defs.len(), 1);
    assert!(env.predicate_defs.contains_key("Test"));
}
