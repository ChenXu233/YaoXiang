//! 判定点协议测试（RFC-027 §12.1 / RFC-039 D62 的 I2 + I5 + I7，6.9.3d）
//!
//! §12.1：判定点先统一；失败必须先回滚统一尝试产生的部分绑定，再允许改用判等。
//! D62-I2：判等不绑定变量，两侧 ground 才谈「同一」。
//! D62-I5：Unproven ≠ False——算不出来走 E4023（带命题与位置），不得改写成「不等」。

use std::collections::HashMap;

use crate::frontend::core::sema::judgement::{decide, decide_with_budget, JudgementFailure};
use crate::frontend::core::sema::proof::budget::BudgetTracker;
use crate::frontend::core::types::const_data::ConstValue;
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::frontend::core::types::mono::{MonoType, PolyType};
use crate::frontend::core::types::TypeConstraintSolver;
use crate::util::diagnostic::push_current_span;
use crate::util::span::Span;

fn literal(n: i128) -> MonoType {
    MonoType::literal_of_value(&ConstValue::Int(n)).expect("整数字面量必有字面量类型")
}

fn array_of_len(n: i128) -> MonoType {
    MonoType::Generic {
        name: "Array".to_string(),
        args: vec![MonoType::Int(64), literal(n)],
    }
}

fn poly(ty: MonoType) -> PolyType {
    PolyType::mono(ty)
}

/// 空 δ 表（无别名可展开）：判等退化为结构同一。
fn empty_types() -> HashMap<String, PolyType> {
    HashMap::new()
}

/// δ 别名成功路径：`Array(Int, Five)` 与 `Array(Int, int64::5)` 经判等判为同一。
#[test]
fn test_decide_alias_representation_is_proved() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let mut types = empty_types();
    types.insert("Five".to_string(), poly(literal(5)));
    let dep_env = DependentTypeEnv::new();
    let lhs = MonoType::Generic {
        name: "Array".to_string(),
        args: vec![MonoType::Int(64), MonoType::TypeRef("Five".to_string())],
    };

    // Act
    let result = decide(
        &mut solver,
        &lhs,
        &array_of_len(5),
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
    );

    // Assert
    assert!(
        result.is_ok(),
        "δ 展开后两侧同一，判定点应放行；实际: {:?}",
        result
            .err()
            .map(|failure| failure.into_diagnostic().message)
    );
}

/// 回滚：`Fn` 前位成功、后位失败时，判定失败后形参位必须回到未绑定。
#[test]
fn test_decide_failure_rolls_back_partial_binding() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let t = solver.new_var();
    let lhs = MonoType::Fn {
        params: vec![t.clone()],
        return_type: Box::new(MonoType::Int(64)),
    };
    let rhs = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::make_string()),
    };
    let types = empty_types();
    let dep_env = DependentTypeEnv::new();

    // Act
    let result = decide(
        &mut solver,
        &lhs,
        &rhs,
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
    );

    // Assert
    assert!(result.is_err(), "返回位 int64 与 string 应判定失败");
    assert!(
        matches!(solver.resolve_type(&t), MonoType::TypeVar(_)),
        "判定失败必须回滚本次尝试的绑定，实际: {}",
        solver.resolve_type(&t)
    );
}

/// 真不等：`Array(Int,5)` 与 `Array(Int,6)` 归约后仍不等 ⇒ `Mismatch`（站点可沿用 E1002 报文）。
#[test]
fn test_decide_true_mismatch_stays_mismatch() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let types = empty_types();
    let dep_env = DependentTypeEnv::new();

    // Act
    let result = decide(
        &mut solver,
        &array_of_len(5),
        &array_of_len(6),
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
    );

    // Assert
    assert!(
        matches!(result, Err(JudgementFailure::Mismatch(_))),
        "真不等必须是 Mismatch（不得落 Unproven）"
    );
}

/// 算不出来：别名环导致归约失败 ⇒ `Unproven`，报文为 E4023 且带 `l ≡ r` 命题。
#[test]
fn test_decide_unproven_reports_e4023_with_proposition() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let mut types = empty_types();
    types.insert("A".to_string(), poly(MonoType::TypeRef("B".to_string())));
    types.insert("B".to_string(), poly(MonoType::TypeRef("A".to_string())));
    let dep_env = DependentTypeEnv::new();
    let lhs = MonoType::Generic {
        name: "Array".to_string(),
        args: vec![MonoType::Int(64), MonoType::TypeRef("A".to_string())],
    };

    // Act
    let result = decide(
        &mut solver,
        &lhs,
        &array_of_len(5),
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
    );

    // Assert
    let failure = result.expect_err("别名环无法归约，应报「无法证明」");
    assert!(
        matches!(failure, JudgementFailure::Unproven(_)),
        "归约失败必须落 Unproven（不得降格为 Mismatch）"
    );
    let diagnostic = failure.into_diagnostic();
    assert_eq!(
        diagnostic.code.as_str(),
        "E4023",
        "一般性「无法证明」走 E4023；实际: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains("≡"),
        "命题必须有类型等式形态（subject 覆写为 l ≡ r）；实际: {}",
        diagnostic.message
    );
}

/// 预算：一次判定一份——注入 1 步预算时归约即耗尽 ⇒ E4023（BudgetExceeded）。
#[test]
fn test_decide_budget_exhaustion_reports_e4023() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let types = empty_types();
    let dep_env = DependentTypeEnv::new();
    let budget = BudgetTracker::with_steps(1);

    // Act
    let result = decide_with_budget(
        &mut solver,
        &array_of_len(5),
        &array_of_len(6),
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
        &budget,
    );

    // Assert
    let failure = result.expect_err("预算耗尽时不得宣称「不等」");
    assert_eq!(
        failure.into_diagnostic().code.as_str(),
        "E4023",
        "预算耗尽走 E4023（原因 BudgetExceeded）"
    );
    assert_eq!(
        budget.report().steps_used,
        1,
        "预算按「一次判定一份」扣，用满注入的 1 步"
    );
}

/// ground 门：失败尝试留下半截绑定、回滚后一侧仍含类型变量 ⇒ 判等不尝试。
///
/// 形态取「形参位绑定成功、返回位失败」：若先测 ground 再回滚，`t` 已被绑成 `Int`，
/// 两侧看起来都 ground，判等就会建立在被作废的绑定上。
#[test]
fn test_decide_open_side_skips_defeq() {
    // Arrange
    // unify 的 E1002 缺省取 walk 上下文 span（#324）；直驱测试须显式进上下文
    let _walk = push_current_span(Span::dummy());
    let mut solver = TypeConstraintSolver::new();
    let t = solver.new_var();
    let open = MonoType::Fn {
        params: vec![t.clone()],
        return_type: Box::new(MonoType::Int(64)),
    };
    let closed = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::make_string()),
    };
    let types = empty_types();
    let dep_env = DependentTypeEnv::new();

    // Act
    let result = decide(
        &mut solver,
        &open,
        &closed,
        Some(Span::dummy()),
        &types,
        Some(&dep_env),
    );

    // Assert
    assert!(
        matches!(result, Err(JudgementFailure::Mismatch(_))),
        "回滚后一侧仍含类型变量 ⇒ 不做判等，必须落 Mismatch（不得出现 E4023）"
    );
    assert!(
        matches!(solver.resolve_type(&t), MonoType::TypeVar(_)),
        "判等前必须先回滚，形参位不得带着半截绑定参与判等：{}",
        solver.resolve_type(&t)
    );
}
