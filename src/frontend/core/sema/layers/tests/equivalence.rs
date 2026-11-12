//! 类型等价性检查单元测试（6.9.3b：`defeq` 内核）
//!
//! RFC-027 Section 2: Type Equivalence Tests
//!
//! 覆盖验收清单：同一性短路、ι 规范形态、`Generic` 实参递归归约、
//! `Refined` 谓词不同 ⇒ Unproven（先红后绿）、Unproven 不降格、预算生效。

use crate::frontend::core::sema::layers::equivalence::{check_type_equivalence, defeq, normalize_type};
use crate::frontend::core::sema::proof::budget::BudgetTracker;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, UnprovenReason};
use crate::frontend::core::sema::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

/// `<var> <op> <n>` 形态的约束表达式。
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

/// `Int(64)` + 给定约束的精化类型。
fn refined_int(constraint: ConstExpr) -> MonoType {
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    }
}

/// 字面量类型：`name` 与 `value` 分开给，便于构造**非规范**名字（ι 用例）。
fn literal(
    name: &str,
    value: i128,
) -> MonoType {
    MonoType::Literal {
        name: name.to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(value),
    }
}

/// `Generic(name, args)` 构造助手（`Array(Int, 3)` 这类）。
fn generic(
    name: &str,
    args: Vec<MonoType>,
) -> MonoType {
    MonoType::Generic {
        name: name.to_string(),
        args,
    }
}

/// 同一性（派生 `PartialEq`）短路：结构完全相同的类型不经归约即判等。
#[test]
fn test_defeq_syntactic_identity_is_proved() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let int64 = MonoType::Int(64);

    // Act
    let same = defeq(&int64, &MonoType::Int(64), &ctx);
    let different = defeq(&int64, &MonoType::Int(32), &ctx);

    // Assert
    assert!(same.is_ok(), "结构相同的 Int(64) 应判等；实际: {same:?}");
    assert!(
        matches!(different, Err(ProofFailure::Disproved(_))),
        "两侧都归约成功且不等 ⇒ Disproved；实际: {different:?}"
    );
}

/// 6.9.3b 验收：`defeq(Generic(Int,[Literal 3]), 同形)` = Proved。
#[test]
fn test_defeq_generic_with_same_literal_argument_is_proved() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let lhs = generic("Array", vec![MonoType::Int(64), literal("3", 3)]);
    let rhs = generic("Array", vec![MonoType::Int(64), literal("3", 3)]);

    // Act
    let result = defeq(&lhs, &rhs, &ctx);

    // Assert
    assert!(
        result.is_ok(),
        "同形 Generic 且字面量相同应判等；实际: {result:?}"
    );
}

/// 6.9.3b 验收：`defeq(Generic(Int,[Literal 3]), Generic(Int,[Literal 5]))` = Disproved。
#[test]
fn test_defeq_generic_with_different_literal_argument_is_disproved() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let lhs = generic("Array", vec![MonoType::Int(64), literal("3", 3)]);
    let rhs = generic("Array", vec![MonoType::Int(64), literal("5", 5)]);

    // Act
    let result = defeq(&lhs, &rhs, &ctx);

    // Assert
    assert!(
        matches!(result, Err(ProofFailure::Disproved(_))),
        "两侧归约成功但实参值不等 ⇒ Disproved；实际: {result:?}"
    );
}

/// 6.9.3b 验收：`defeq(Literal 3, Int64)` = Disproved（单值类型不是基类型）。
#[test]
fn test_defeq_literal_versus_base_type_is_disproved() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = defeq(&literal("3", 3), &MonoType::Int(64), &ctx);

    // Assert
    assert!(
        matches!(result, Err(ProofFailure::Disproved(_))),
        "`int64::3` 与 `int64` 不是同一类型 ⇒ Disproved；实际: {result:?}"
    );
}

/// 6.9.3b（先红后绿）：`Refined` 不同谓词**不是**等价——旧 `structurally_equal`
/// 只比 base，会把它判成 `Proved`（t47 E3 的假等价，属 D59 A 类错答）。
#[test]
fn test_defeq_refined_different_predicates_is_unproven() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let positive = refined_int(cmp_const("x", BinOp::Gt, 0));
    let negative = refined_int(cmp_const("x", BinOp::Lt, 0));

    // Act
    let result = defeq(&positive, &negative, &ctx);

    // Assert
    assert!(
        matches!(result, Err(ProofFailure::Unproven { .. })),
        "不同谓词的精化类型判等超出内核 ⇒ Unproven；实际: {result:?}"
    );
}

/// `Refined` 谓词**相同** ⇒ 归约基类型后判等（Proved）。
#[test]
fn test_defeq_refined_same_predicate_is_proved() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let lhs = refined_int(cmp_const("x", BinOp::Gt, 0));
    let rhs = refined_int(cmp_const("x", BinOp::Gt, 0));

    // Act
    let result = defeq(&lhs, &rhs, &ctx);

    // Assert
    assert!(result.is_ok(), "同谓词精化类型应判等；实际: {result:?}");
}

/// 一侧精化 Conversely：精化类型 vs 非精化类型 ⇒ Unproven（判等超出内核，不是「不等」）。
#[test]
fn test_defeq_refined_versus_plain_is_unproven() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let refined = refined_int(cmp_const("x", BinOp::Gt, 0));

    // Act
    let result = defeq(&refined, &MonoType::Int(64), &ctx);

    // Assert
    assert!(
        matches!(result, Err(ProofFailure::Unproven { .. })),
        "精化类型与基类型判等超出内核 ⇒ Unproven（若判 Disproved 即降格）；实际: {result:?}"
    );
}

/// `normalize_type` 的 ι：`Literal` 重建为**规范形态**（只读 value，不读 name）。
#[test]
fn test_normalize_type_canonicalizes_literal_name() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let noncanonical = literal("0x3", 3);

    // Act
    let normalized = normalize_type(&noncanonical, &ctx);

    // Assert
    match normalized {
        Ok(MonoType::Literal { name, value, .. }) => {
            assert_eq!(name, "3", "ι 必须按 value 重建 name（不读旧的 '0x3'）");
            assert_eq!(value, ConstValue::Int(3), "值本身不变");
        }
        other => panic!("Literal 归一化应得规范 Literal，实际: {other:?}"),
    }
}

/// `normalize_type` 递归进 `Generic` 实参（Evaluator 不归约实参）。
#[test]
fn test_normalize_type_recurses_into_generic_arguments() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let ty = generic("Array", vec![MonoType::Int(64), literal("0x3", 3)]);

    // Act
    let normalized = normalize_type(&ty, &ctx);

    // Assert
    match normalized {
        Ok(MonoType::Generic { args, .. }) => match &args[1] {
            MonoType::Literal { name, .. } => {
                assert_eq!(name, "3", "Generic 实参里的 Literal 也必须被 ι 规范");
            }
            other => panic!("第 2 个实参应是 Literal，实际: {other:?}"),
        },
        other => panic!("Generic 归一化应保持 Generic，实际: {other:?}"),
    }
}

/// 预算生效：`normalize_type` 归约时吃 `ctx.budget`，耗尽即 `Unproven{BudgetExceeded}`。
#[test]
fn test_normalize_type_budget_exhaustion_is_unproven() {
    // Arrange
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.budget = BudgetTracker::with_steps(0);
    let ty = MonoType::Int(64);

    // Act
    let result = normalize_type(&ty, &ctx);

    // Assert
    match result {
        Err(ProofFailure::Unproven { reason, budget, .. }) => {
            assert!(
                matches!(reason, UnprovenReason::BudgetExceeded),
                "预算耗尽必须报 BudgetExceeded，实际 reason: {reason:?}"
            );
            assert_eq!(budget.steps_limit, 0, "报告应带出注入的预算上限");
        }
        // 预算耗尽**不得**降格成 Disproved（那是「算出不等」的断言）
        other => panic!("预算耗尽不得降格成 Disproved，实际: {other:?}"),
    }
}

/// 兼容入口 `check_type_equivalence` 与 `defeq` 同判据（同一实现）。
#[test]
fn test_check_type_equivalence_delegates_to_defeq() {
    // Arrange
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let lhs = generic("Array", vec![MonoType::Int(64), literal("3", 3)]);
    let rhs = generic("Array", vec![MonoType::Int(64), literal("3", 3)]);

    // Act
    let via_legacy = check_type_equivalence(&ctx, &lhs, &rhs);
    let via_defeq = defeq(&lhs, &rhs, &ctx);

    // Assert
    assert_eq!(
        via_legacy, via_defeq,
        "兼容入口必须与 defeq 同判据（同一实现，不得有第二条判定路径）"
    );
}
