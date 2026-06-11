//! RFC-027 阶段 1 端到端集成测试
//!
//! 验证从类型标注到证明结果的完整数据流：
//! PredicateResolver → Refined → check_predicate → ProofResult

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::predicate_resolver::{PredicateDef, PredicateResolver};
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::layers::equivalence::check_type_equivalence;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use std::collections::HashMap;

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建一个简单的 Positive 谓词定义
/// Positive(x): x > 0
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

/// 创建一个简单的 NonEmpty 谓词定义
/// NonEmpty(s): len(s) > 0
fn make_nonempty_def() -> PredicateDef {
    // 这里用 NamedVar 表达 len(s) —— 阶段 1 的简化表示
    // 实际上 len(s) 需要函数调用，但阶段 1 我们只测基础路径
    PredicateDef {
        param_name: "s".into(),
        param_type: MonoType::String,
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("s".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    }
}

// ===================================================================
// 数据流测试：PredicateResolver → ProofResult
// ===================================================================

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
    .expect("Positive(5) 应该被解析");

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
    .expect("Positive(0) 应该被解析");

    let ctx = ProofContext::new(&env);
    let bindings = HashMap::new();
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(!result.is_proved(), "0 > 0 应该被证伪");
    match result {
        ProofResult::Disproved(_) => {}
        _ => panic!("Expected Disproved, got {:?}", result),
    }
}

/// 端到端：Positive(b) + binding { b: 2 } → Proved
///
/// 验证带变量绑定的谓词检查：
/// 1. PredicateResolver 将 Positive(b) 正格化为 Refined { base: Int, constraint: b > 0 }
/// 2. check_predicate 代入 b=2，求值 2 > 0 为 true
#[test]
fn test_e2e_positive_with_binding_proved() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());

    let resolved =
        PredicateResolver::try_resolve(&env, "Positive", &[MonoType::TypeRef("b".into())])
            .expect("Positive(b) 应该被解析");

    let ctx = ProofContext::new(&env);
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(2));
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(result.is_proved(), "2 > 0 应该被证明");
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
            .expect("Positive(b) 应该被解析");

    let ctx = ProofContext::new(&env);
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(-1));
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(!result.is_proved(), "-1 > 0 应该被证伪");
    match result {
        ProofResult::Disproved(_) => {}
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

/// 端到端：check_refined_binding 对非 Refined 类型直接返回 Proved
///
/// 验证 check_predicate 对普通类型（Int, Fn 等）不做额外验证
#[test]
fn test_e2e_non_refined_always_proved() {
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let bindings = HashMap::new();

    // 普通 Int 类型
    let result = check_predicate(&ctx, &MonoType::Int(64), &bindings);
    assert!(result.is_proved());

    // 函数类型
    let fn_ty = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Bool),
    };
    let result = check_predicate(&ctx, &fn_ty, &bindings);
    assert!(result.is_proved());
}

// ===================================================================
// Layer 0: 类型等式测试
// ===================================================================

/// 端到端：类型等式——结构等价
#[test]
fn test_e2e_type_equivalence_structural() {
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Int(64) == Int(64)
    let result = check_type_equivalence(&ctx, &MonoType::Int(64), &MonoType::Int(64));
    assert!(result.is_proved());
}

/// 端到端：类型等式——Int != Bool
#[test]
fn test_e2e_type_equivalence_not_equal() {
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Int(64) != Bool
    let result = check_type_equivalence(&ctx, &MonoType::Int(64), &MonoType::Bool);
    assert!(!result.is_proved());
}

/// 端到端：类型等式——Refined 基类型相同但约束不同的视为结构等价
#[test]
fn test_e2e_refined_same_base_structural_equivalent() {
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Refined { base: Int, constraint: 5 > 0 }
    let refined_a = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };

    // Refined { base: Int, constraint: 3 > 0 }  -- 不同约束但相同基类型
    let refined_b = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(3))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };

    // 结构等价只看基类型
    let result = check_type_equivalence(&ctx, &refined_a, &refined_b);
    assert!(result.is_proved(), "相同基类型的 Refined 应结构等价");
}

// ===================================================================
// 回归测试：MonoType 基础操作不受影响
// ===================================================================

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

// ===================================================================
// 多谓词场景
// ===================================================================

/// 多个不同谓词共存
#[test]
fn test_multiple_predicates() {
    let mut env = TypeEnvironment::new();
    env.predicate_defs
        .insert("Positive".into(), make_positive_def());
    env.predicate_defs
        .insert("NonEmpty".into(), make_nonempty_def());

    // Positive(3) 应该证明
    let resolved = PredicateResolver::try_resolve(
        &env,
        "Positive",
        &[MonoType::Literal {
            name: "3".into(),
            base_type: Box::new(MonoType::Int(64)),
            value: ConstValue::Int(3),
        }],
    )
    .expect("Positive(3) 应该被解析");

    let ctx = ProofContext::new(&env);
    let bindings = HashMap::new();
    let result = check_predicate(&ctx, &resolved, &bindings);
    assert!(result.is_proved(), "3 > 0 应该被证明");

    // 非谓词名返回 None
    let unknown = PredicateResolver::try_resolve(&env, "UnknownPred", &[MonoType::Int(64)]);
    assert!(unknown.is_none());
}
