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
