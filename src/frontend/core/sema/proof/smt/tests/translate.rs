//! 翻译层测试 — 基于 RFC-027 §8
//!
//! §8: SMT-LIB 翻译层——ConstExpr → SMTCommand
//! 验证假设栈成为背景断言、目标取反做 check-sat 的蕴含判定模式。

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::sema::proof::smt::ast::{SMTCommand, SMTSort};
use crate::frontend::core::sema::proof::smt::translate::{
    infer_var_sorts, translate_constraint, translate_expr,
};

fn make_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar(var.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

#[test]
fn test_translate_simple_constraint() {
    let constraint = make_gt("x", 0);
    let mut var_sorts = HashMap::new();
    var_sorts.insert("x".into(), SMTSort::Int);

    let commands = translate_constraint(&constraint, &[], &var_sorts)
        .expect("可翻译约束必须给出命令序列（不是 None）");

    assert!(
        commands.len() >= 3,
        "简单约束应至少产生 declare-const + assert(not goal) + check-sat + get-model，实际: {commands:?}"
    );
    match &commands[0] {
        SMTCommand::DeclareConst(name, sort) => {
            assert_eq!(name, "x", "声明常量名应为 x");
            assert_eq!(*sort, SMTSort::Int, "声明常量排序应为 Int");
        }
        other => panic!("第一个命令应为 DeclareConst，实际: {other:?}"),
    }
}

#[test]
fn test_translate_expr_gt_to_smt_gt() {
    let expr = make_gt("x", 10);
    let result = translate_expr(&expr).expect("GT 必须可翻译");
    assert_eq!(
        result.to_string(),
        "(> x 10)",
        "Gt 比较应翻译为 SMT > 运算符"
    );
}

#[test]
fn test_infer_var_sorts_from_arithmetic_constraints() {
    let constraint = ConstExpr::BinOp {
        op: BinOp::And,
        left: Box::new(make_gt("x", 0)),
        right: Box::new(make_gt("y", 100)),
    };

    let sorts = infer_var_sorts(&constraint, &HashMap::new());

    assert!(
        sorts.contains_key("x"),
        "约束中的 x 变量应被推断: {sorts:?}"
    );
    assert!(
        sorts.contains_key("y"),
        "约束中的 y 变量应被推断: {sorts:?}"
    );
}
/// T16/R5：证据字段路径不是 SMT 项——显式拒绝，不产出伪造 atom。
#[test]
fn test_translate_field_path_is_not_translatable() {
    // Arrange
    let path = ConstExpr::FieldPath {
        base: Box::new(ConstExpr::NamedVar("bb".into())),
        field: "w".into(),
    };
    let composite = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(path.clone()),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    // Act + Assert
    assert!(
        translate_expr(&path).is_none(),
        "translate_expr(FieldPath) 必须返回 None（不产物化 atom）"
    );
    assert!(
        translate_expr(&composite).is_none(),
        "含 FieldPath 的组合式同样不可翻译（不得伪造 atom）"
    );
}

/// F10：不可翻译的**目标**必须显式 None（不得是空表——空表在求解器侧恒 sat ⇒ 伪反例）。
#[test]
fn test_translate_constraint_untranslatable_goal_returns_none() {
    // Arrange — 位运算不是 SMT 项（translate_expr 返回 None）
    let goal = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::BinOp {
            op: BinOp::BitAnd,
            left: Box::new(ConstExpr::NamedVar("y".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(3))),
        }),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    // Act
    let commands = translate_constraint(&goal, &[], &HashMap::new());

    // Assert
    assert!(
        commands.is_none(),
        "不可翻译的目标必须返回 None，而不是空命令表；实际: {commands:?}"
    );
}

/// F10：任一**假设**不可翻译 ⇒ 整组命令 None（假设无法作为背景断言时不能假装无关）。
#[test]
fn test_translate_constraint_untranslatable_assumption_returns_none() {
    // Arrange
    let goal = make_gt("y", 0);
    let assumption = ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::BinOp {
            op: BinOp::BitAnd,
            left: Box::new(ConstExpr::NamedVar("y".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(3))),
        }),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    // Act
    let commands = translate_constraint(&goal, &[assumption], &HashMap::new());

    // Assert
    assert!(
        commands.is_none(),
        "不可翻译的假设必须让整组命令为 None；实际: {commands:?}"
    );
}

/// F10 反向不变量：**成功翻译**的命令表永不为空（至少 CheckSat + GetModel）。
///
/// 这条钉住「None 与空表」「翻译成功」三态互不混同：空表若出现，调用方无法与
/// 「不可翻译」区分，正是 F10 的根因。
#[test]
fn test_translate_constraint_success_is_never_empty() {
    // Arrange
    let constraint = make_gt("x", 0);
    let assumption = make_gt("y", 5);
    let mut var_sorts = HashMap::new();
    var_sorts.insert("x".into(), SMTSort::Int);
    var_sorts.insert("y".into(), SMTSort::Int);

    // Act
    let commands = translate_constraint(&constraint, &[assumption], &var_sorts)
        .expect("可翻译约束不应为 None");

    // Assert
    assert!(
        !commands.is_empty(),
        "成功翻译的命令表不得为空（空表无法与不可翻译区分）"
    );
    assert!(
        matches!(commands.last(), Some(SMTCommand::GetModel)),
        "命令表必须以实现 check-sat + get-model 收尾；实际: {commands:?}"
    );
}
