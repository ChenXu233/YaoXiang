//! RFC-027 §4/§8 Phase 2 SMT 集成测试
//!
//! 测试三级分派：
//!   §4.1: Evaluator 直接求值（Phase 1）
//!   §3.2: 假设栈蕴含（Phase 2A）
//!   §8:   Z3 SMT 求解（Phase 2B）

use crate::frontend::core::sema::test_util::{binop, refined_int};
use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, UnOp};
use crate::frontend::core::sema::layers::predicate::check_predicate;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, ProofResultExt};
use crate::frontend::core::sema::TypeEnvironment;

/// 辅助：构造 GT 约束 (var > n)
fn constraint_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    binop(
        BinOp::Gt,
        ConstExpr::NamedVar(var.into()),
        ConstExpr::Lit(ConstValue::Int(n)),
    )
}

/// 辅助：构造 Refined 类型
// =========== §4.1: Evaluator 直接求值（Phase 1 回归） ===========

#[test]
fn test_direct_eval_proved_for_true_literal_comparison() {
    let refined = refined_int(binop(
        BinOp::Gt,
        ConstExpr::Lit(ConstValue::Int(5)),
        ConstExpr::Lit(ConstValue::Int(0)),
    ));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "纯字面量 5>0 应直接求值为 Proved");
}

// =========== §3.2: 假设栈蕴含 ===========

// =========== §8: Z3 SMT 求解 ===========

#[test]
fn test_smt_symbolic_variable_without_binding_detects_counterexample() {
    // 目标: y > 0，无 bindings
    // Z3: (not (> y 0)) → sat (y=0 是反例)
    let refined = refined_int(constraint_gt("y", 0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(!result.is_proved(), "Z3 应找到 y>0 的反例 (如 y=0)");
    match result {
        Err(ProofFailure::Disproved(model)) => {
            assert!(
                !model.assignments.is_empty(),
                "反例模型应包含至少一个赋值: {model:?}"
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

// ---- 整数字面量保真端到端锚（issue #561，病史门）----
//
// 病史：修前 SMT 翻译面把超 i64 字面量落成自由变量、超 i32 字面量
// `as i32` 截断、一元负号（`App("-")` 单参数）按 Z3_mk_sub 官方语义退化为
// 恒等而丢失——真实整数语义恒真的约束被假拒绝为 Disproved（issue #559
// 直评面 E4018 带伪变量反例同根）。以下锚断言修复后的真实语义判向。

/// 超 i32 字面量进假设蕴含必须按真实整数语义判 Proved（issue #561）
///
/// 假设 `x > 4000000000` 蕴含目标 `x > 0`（真实整数语义）。修前 4e9 截断为
/// -294967296（负数），假设不再蕴含目标 → Level 3 编码可满足 → Disproved
/// 伪反例。注意两侧同截断的比较不翻转（同减 2³² 保序），蕴含对（一侧截断
/// 一侧不截断）才是翻转形态。
#[test]
fn test_smt_truncated_large_literal_assumption_implication_judged_proved() {
    // Arrange — 假设 x > 4e9，目标约束 x > 0，x 无绑定（Level 1 失败升级 SMT）
    let refined = refined_int(binop(
        BinOp::Gt,
        ConstExpr::NamedVar("x".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    ));

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(binop(
        BinOp::Gt,
        ConstExpr::NamedVar("x".into()),
        ConstExpr::Lit(ConstValue::Int(4_000_000_000)),
    ));

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 真实语义蕴含成立必须 Proved；修前截断判 Disproved（伪反例）
    assert!(
        result.is_proved(),
        "假设 x > 4e9 蕴含 x > 0 必须 Proved（修前 4e9 截断为 -294967296 使蕴含失效判 Disproved），实际: {result:?}"
    );
}

/// 一元负号必须取负：`x + (-x) = 0` 恒真必须 Proved（issue #561）
///
/// 病史：修前 `UnOp::Neg` 翻译成 `App("-")` 单参数 → `Z3_mk_sub` 官方语义
/// 单参数恒等 → 负号丢失，约束被翻译成 `x + x = 0`（非恒真）→ Disproved。
#[test]
fn test_smt_unary_neg_term_cancels_to_zero_proved() {
    // Arrange — 约束 x + (-x) = 0：真实整数语义恒真，x 未绑定
    let refined = refined_int(binop(
        BinOp::Eq,
        binop(
            BinOp::Add,
            ConstExpr::NamedVar("x".into()),
            ConstExpr::UnOp {
                op: UnOp::Neg,
                expr: Box::new(ConstExpr::NamedVar("x".into())),
            },
        ),
        ConstExpr::Lit(ConstValue::Int(0)),
    ));

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 恒真约束必须 Proved；修前负号丢失判 Disproved
    assert!(
        result.is_proved(),
        "x + (-x) = 0 恒真必须 Proved（修前一元负号翻译丢失判 Disproved），实际: {result:?}"
    );
}
