//! 精化谓词证明测试 — 基于 RFC-027 §3-4
//!
//! RFC-027 §4.1: 四级分派证明管道（check_predicate）
//! RFC-027 §3.1: 直接求值（Level 1 — Phase 1）
//! RFC-027 §3.2: 假设栈蕴含（Level 2a/2b — Phase 2A + 3.2）
//! RFC-027 §8:   SMT 求解（Level 3 — Phase 2B）
//! RFC-027 §4.2: 证明函数调用（Level 4 — Phase 2.5）
//!
//! 测试覆盖：
//! - Phase 1: Evaluator 直接求值（绑定变量 + 纯字面量）
//! - Phase 2A: 假设栈精确匹配
//! - Phase 3.2: SMT 假设蕴含（强假设→弱约束 / 无关假设退至 Level 3）

use std::collections::HashMap;

use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

// ===================================================================
// RFC-027 §4.1: Phase 1 — Evaluator 直接求值（Level 1）
// ===================================================================

/// RFC-027 §4.1 Level 1: 绑定变量有具体值 → Evaluator 直接求值 Proved
#[test]
fn test_direct_eval_with_bound_variable_proved() {
    // Arrange
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

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(result.is_proved(), "b=5 时 b>0 应直接求值为 Proved");
}

/// RFC-027 §4.1 Level 1: 绑定变量有具体值 → Evaluator 直接求值 Disproved
#[test]
fn test_direct_eval_with_bound_variable_disproved() {
    // Arrange
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

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(!result.is_proved(), "b=0 时 b>0 应直接求值为 Disproved");
    match result {
        ProofResult::Disproved(model) => {
            assert!(
                model.assignments.iter().any(|(k, _)| k == "b"),
                "反例模型应包含变量 b"
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// RFC-027 §4.1: 非 Refined 类型直接返回 Proved（无事可证）
#[test]
fn test_non_refined_type_passes_immediately() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &MonoType::Int(64), &HashMap::new());

    // Assert
    assert!(result.is_proved(), "非 Refined 类型应直接返回 Proved");
}

// ===================================================================
// RFC-027 §3.2: Phase 2A — 假设栈精确匹配（Level 2a）
// ===================================================================

/// RFC-027 §3.2 Level 2a: 约束正好在假设栈中 → 零开销直接 Proved
#[test]
fn test_assumption_stack_direct_match_proves_immediately() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint.clone(),
    };
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(constraint);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    assert!(result.is_proved(), "约束正好在假设栈中应直接返回 Proved");
}

/// RFC-027 §4.1: 纯字面量 5>0 → Evaluator 直接求值 Proved
#[test]
fn test_direct_eval_with_concrete_literals() {
    // Arrange
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    assert!(result.is_proved(), "5>0 纯字面量应直接求值为 Proved");
}

// ===================================================================
// RFC-027 §3.2: Phase 3.2 — SMT 假设蕴含（Level 2b）
// ===================================================================

/// RFC-027 §3.2 Level 2b: 假设 y >= 5 蕴含约束 y > 0，SMT 判断 unsat → Proved
#[test]
fn test_implication_stronger_assumption_proves_weaker_constraint() {
    // Arrange — 约束: y > 0
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint.clone(),
    };
    // 假设: y >= 5（比 y > 0 更强）
    let assumption = ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    };

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(assumption);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    assert!(
        result.is_proved(),
        "y >= 5 蕴含 y > 0，假设蕴含应返回 Proved，实际: {result:?}"
    );
}

/// RFC-027 §3.2 Level 2b→3: 假设 z > 0 不蕴含 y > 0 → Level 2b None → Level 3 Disproved
#[test]
fn test_implication_unrelated_assumption_falls_through_to_level3() {
    // Arrange — 约束: y > 0
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    // 假设: z > 0（与约束无关）
    let assumption = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("z".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(assumption);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — z > 0 不蕴含 y > 0 → Level 2b 返回 None → Level 3 找到反例 (y=0, z=1)
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "z>0 不蕴含 y>0，Level 3 应找到反例返回 Disproved，实际: {result:?}"
    );
}

/// RFC-027 §3.2 Level 2b: 多假设组合蕴含 — y >= 5 和 y < 10 共同蕴含 y > 0
#[test]
fn test_implication_multiple_assumptions_combined_imply() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    });
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Lt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
    });
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Lt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
    });

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 多假设作为背景断言，SMT 应判蕴含
    assert!(
        result.is_proved(),
        "y>=5 且 y<10 共同蕴含 y>0，应返回 Proved，实际: {result:?}"
    );
}

/// RFC-027 §3.2: 假设栈为空时 Level 2b 被跳过
#[test]
fn test_implication_empty_assumptions_skips_level2b() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env); // 空假设栈

    // Act — Level 2b 应跳过（assumptions.is_empty()），进入 Level 3
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — Level 3 找到反例 y=0
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "空假设栈应跳过 Level 2b，进入 Level 3 找反例，实际: {result:?}"
    );
}

/// RFC-027 §3.2: 嵌套 push/pop 后假设栈恢复，蕴含失效
#[test]
fn test_implication_nested_push_pop_restores_stack() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);

    // 外层 if：压入
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    });
    // 内层 block：进入 scope，压入后退出
    ctx.assumptions.enter_scope();
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("z".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(3))),
    });
    ctx.assumptions.exit_scope(); // 离开内层，z>3 消失，y>=5 仍在

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — pop 后只剩 y>=5，仍蕴含 y>0
    assert!(
        result.is_proved(),
        "pop 后外层假设 y>=5 仍应蕴含 y>0，实际: {result:?}"
    );
}

/// RFC-027 §3.2: pop 所有假设后回到空栈，Level 2b 跳过
#[test]
fn test_implication_all_popped_falls_through() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.enter_scope();
    ctx.assumptions.inject(ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    });
    ctx.assumptions.exit_scope(); // 全部弹出

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 栈空，跳过 2b，进入 Level 3
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "pop 全部后栈空，应进入 Level 3，实际: {result:?}"
    );
}
