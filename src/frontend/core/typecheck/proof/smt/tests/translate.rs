//! 翻译层测试 — 基于 RFC-027 §8
//!
//! §8: SMT-LIB 翻译层——ConstExpr → SMTCommand
//! 验证假设栈成为背景断言、目标取反做 check-sat 的蕴含判定模式。

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue, UnOp};
use crate::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTSort};
use crate::frontend::core::typecheck::proof::smt::translate::{
    infer_var_sorts, translate_constraint, translate_expr,
};

fn make_gt(var: &str, n: i128) -> ConstExpr {
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

    let commands = translate_constraint(&constraint, &[], &var_sorts);

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
fn test_translate_with_assumptions_emits_background_assertions() {
    let constraint = make_gt("y", 0);
    let assumptions = vec![make_gt("y", 5)];
    let mut var_sorts = HashMap::new();
    var_sorts.insert("y".into(), SMTSort::Int);

    let commands = translate_constraint(&constraint, &assumptions, &var_sorts);

    assert_eq!(
        commands.len(),
        5,
        "应有 5 条命令: declare-const + assert(假设) + assert(not 目标) + check-sat + get-model"
    );
}

#[test]
fn test_translate_expr_gt_to_smt_gt() {
    let expr = make_gt("x", 10);
    let result = translate_expr(&expr);
    assert_eq!(
        result.to_string(),
        "(> x 10)",
        "Gt 比较应翻译为 SMT > 运算符"
    );
}

#[test]
fn test_translate_expr_and_produces_nested_smt() {
    let expr = ConstExpr::BinOp {
        op: BinOp::And,
        left: Box::new(make_gt("x", 0)),
        right: Box::new(ConstExpr::BinOp {
            op: BinOp::Lt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
        }),
    };
    let result = translate_expr(&expr);
    let s = result.to_string();
    assert!(
        s.contains("and"),
        "And 表达式应包含 and 操作符: {s}"
    );
    assert!(
        s.contains("(> x 0)"),
        "And 左子树应包含 (> x 0): {s}"
    );
    assert!(
        s.contains("(< x 10)"),
        "And 右子树应包含 (< x 10): {s}"
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

#[test]
fn test_translate_expr_not_yields_smt_not() {
    let expr = ConstExpr::UnOp {
        op: UnOp::Not,
        expr: Box::new(make_gt("z", 0)),
    };
    let result = translate_expr(&expr);
    assert_eq!(
        result.to_string(),
        "(not (> z 0))",
        "Not 一元运算应翻译为 SMT not 表达式"
    );
}
