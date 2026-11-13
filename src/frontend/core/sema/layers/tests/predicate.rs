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

use crate::frontend::core::sema::layers::predicate::check_predicate;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, ProofResultExt, UnprovenReason};
use crate::frontend::core::sema::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, UnOp};
use crate::frontend::core::types::mono::MonoType;

/// `<var> <op> <n>` 常量比较表达式（RFC-027 §3 各层分派用例的共同约束形态）。
fn cmp_const(
    var: &str,
    op: BinOp,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op,
        left: Box::new(ConstExpr::NamedVar(var.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

/// `Int(64)` 基类型 + 给定约束的精炼类型。
fn refined_int(constraint: ConstExpr) -> MonoType {
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    }
}

/// `UnOp{op, Lit(n)}` 一元字面量实参。
fn unop_lit(
    op: UnOp,
    n: i128,
) -> ConstExpr {
    ConstExpr::UnOp {
        op,
        expr: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

/// RFC-027 §4.1 Level 1: 绑定变量有具体值 → Evaluator 直接求值 Disproved
#[test]
fn test_direct_eval_with_bound_variable_disproved() {
    // Arrange
    let refined = refined_int(cmp_const("b", BinOp::Gt, 0));
    let mut bindings = HashMap::new();
    bindings.insert("b".into(), ConstValue::Int(0));

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(!result.is_proved(), "b=0 时 b>0 应直接求值为 Disproved");
    match result {
        Err(ProofFailure::Disproved(model)) => {
            assert!(
                model.assignments.iter().any(|(k, _)| k == "b"),
                "反例模型应包含变量 b"
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// RFC-027 §3.2 Level 2b→3: 假设 z > 0 不蕴含 y > 0 → Level 2b None → Level 3 Disproved
#[test]
fn test_implication_unrelated_assumption_falls_through_to_level3() {
    // Arrange — 约束: y > 0
    let constraint = cmp_const("y", BinOp::Gt, 0);
    let refined = refined_int(constraint);
    // 假设: z > 0（与约束无关）
    let assumption = cmp_const("z", BinOp::Gt, 0);

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.inject(assumption);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — z > 0 不蕴含 y > 0 → Level 2b 返回 None → Level 3 找到反例 (y=0, z=1)
    assert!(
        matches!(result, Err(ProofFailure::Disproved(_))),
        "z>0 不蕴含 y>0，Level 3 应找到反例返回 Disproved，实际: {result:?}"
    );
}

// WBS 3.3.1 / 02-stage-contract §改动清单：`.expect()` → SMTResult::Unknown + 诊断

/// WBS 3.3.1：求解器不可用的降级结果是 SMTResult::Unknown（保守方向）且原因
/// 文本指明 Z3——不再以 expect 崩溃进程。端到端（Z3 真实缺失下 check_predicate
/// 返回 Unproven）受进程级单例限制无法在本进程模拟，结构性证据见提交信息。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_solver_unavailable_degrades_to_unknown_not_panic() {
    // Arrange & Act
    let result = crate::frontend::core::sema::layers::predicate::solver_unavailable_result();

    // Assert
    match result {
        crate::frontend::core::sema::proof::smt::ast::SMTResult::Unknown { reason } => {
            assert!(
                reason.contains("Z3"),
                "降级原因应指明 Z3 缺失，实际: {reason}"
            );
        }
        other => panic!("求解器不可用必须降级为 SMTResult::Unknown，实际: {other:?}"),
    }
}

// ---- Level 4 实参折叠（issue #556：负字面量实参的证明调用不得被 E2031 假拒绝）----

/// RFC-027 §3.1 / issue #556：`Eq(-5, 0)` 的实参 `-5` 落到 Level 4 时是一元取负
/// 形态——必须折叠为 `Int(-5)` 进证明调用义务（修前：整条义务不进管道落 E2031）。
#[test]
fn test_proof_fn_call_folds_unary_negated_literal_args() {
    // Arrange — 约束 Call{Eq, [UnOp{Neg, Lit(5)}, Lit(0)]}，无绑定（Level 1–3 均不可判）
    let constraint = ConstExpr::Call {
        func: "Eq".into(),
        args: vec![unop_lit(UnOp::Neg, 5), ConstExpr::Lit(ConstValue::Int(0))],
    };
    let refined = refined_int(constraint);

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — Unproven 携带证明调用义务，实参为折叠后的 [Int(-5), Int(0)]
    match result {
        Err(ProofFailure::Unproven {
            reason,
            proof_calls,
            ..
        }) => {
            assert!(
                matches!(reason, UnprovenReason::ProofFunctionRequired),
                "负字面量实参应折叠为证明函数调用义务，实际: {reason:?}"
            );
            assert_eq!(proof_calls.len(), 1, "应产出恰好一条证明调用义务");
            assert_eq!(proof_calls[0].func_name, "Eq", "义务应指向约束里的被调函数");
            assert!(
                matches!(
                    &proof_calls[0].args[..],
                    [ConstValue::Int(-5), ConstValue::Int(0)]
                ),
                "实参应为折叠后的 [Int(-5), Int(0)]，实际: {:?}",
                proof_calls[0].args
            );
        }
        other => panic!("期望 Unproven 携带证明调用，实际: {other:?}"),
    }
}

/// 取负口径（issue #556 查证项 3）：`-9223372036854775808`（i64::MIN 形态）在
/// i128 域精确折叠——词法 `Literal::Int` 与 `ConstValue::Int` 同为 i128，
/// `2^63` 的取负无溢出，不得到达 #507 的饱和极端分支。
#[test]
fn test_proof_fn_call_folds_i64_min_negated_literal() {
    // Arrange — 约束 Call{Eq, [UnOp{Neg, Lit(2^63)}]}：i64::MIN 的取负形态
    let constraint = ConstExpr::Call {
        func: "Eq".into(),
        args: vec![unop_lit(UnOp::Neg, 9223372036854775808)],
    };
    let refined = refined_int(constraint);

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 折叠值 -2^63（i128 域），证明调用义务成立
    match result {
        Err(ProofFailure::Unproven { proof_calls, .. }) => {
            assert!(
                matches!(
                    &proof_calls[0].args[..],
                    [ConstValue::Int(-9223372036854775808)]
                ),
                "i64::MIN 形态应在 i128 域折叠为 Int(-9223372036854775808)，实际: {:?}",
                proof_calls[0].args
            );
        }
        other => panic!("期望 Unproven 携带证明调用，实际: {other:?}"),
    }
}

/// #507 饱和极端形态的保守边界：`Neg(i128::MIN)` 在 i128 域无表示——共享求值器
/// 报溢出，折叠返回 None，义务保持「不可折叠」的 Unproven（不 panic、不判伪）。
#[test]
fn test_proof_fn_call_unary_neg_of_i128_min_stays_unfoldable() {
    // Arrange — 实参字面量取负后超出 i128 表示域
    let constraint = ConstExpr::Call {
        func: "Eq".into(),
        args: vec![unop_lit(UnOp::Neg, i128::MIN)],
    };
    let refined = refined_int(constraint);

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 空证法 Unproven（判定单元据此落 E2031），进程不 panic
    match result {
        Err(ProofFailure::Unproven {
            reason,
            proof_calls,
            ..
        }) => {
            assert!(
                matches!(reason, UnprovenReason::BeyondKernel(_)),
                "溢出极端形态应保持保守 Unproven，实际: {reason:?}"
            );
            assert!(
                proof_calls.is_empty(),
                "折叠失败不得产出证明调用义务，实际: {proof_calls:?}"
            );
        }
        other => panic!("期望 Unproven 空证法，实际: {other:?}"),
    }
}

/// issue #559 约束面锚：闭合约束含取负溢出极端形态（`-(0 - i128::MAX - 1) > 0`，
/// 调用点实参位完整表达式文法可达）时判定单元不得 panic——修前实测探针 exit 101
/// （`panicked at evaluator.rs: attempt to negate with overflow`）。Evaluator 报
/// ArithmeticError 后 Level 1 直评失败，升级后续级别按 Disproved / Unproven
/// 保守出判（随 SMT 可用性分流），无 Ok。
#[test]
fn test_check_predicate_neg_overflow_constraint_stays_err_not_panic() {
    // Arrange — Gt(UnOp{Neg, Sub{Sub(0, MAX), 1}}, 0)：裸 Sub 链精确落 i128::MIN，
    // 随后一元取负撞溢出分支（#559 修后）/ panic（修前）。
    let neg_overflow = ConstExpr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(ConstExpr::BinOp {
            op: BinOp::Sub,
            left: Box::new(ConstExpr::BinOp {
                op: BinOp::Sub,
                left: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(i128::MAX))),
            }),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        }),
    };
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(neg_overflow),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = refined_int(constraint);

    // Act
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — Err（Disproved / Unproven，随 SMT 可用性分流），绝不 Ok 静默通过
    assert!(
        result.is_err(),
        "取负溢出极端形态必须保守报错（Disproved/Unproven），不得静默通过，实际: {result:?}"
    );
}
