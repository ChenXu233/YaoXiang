//! SMTExpr 测试 — 基于 RFC-027 §8
//!
//! §8: SMT-LIB 2.6 中间表示（纯数据结构）
//! 验证 SMTExpr/SMTCommand/SMTSort 的 Display 输出符合 SMT-LIB 2.6 标准格式。

use crate::frontend::core::typecheck::proof::smt::ast::{
    SMTCommand, SMTExpr, SMTModel, SMTResult, SMTSort,
};

#[test]
fn test_display_atom_int() {
    assert_eq!(
        SMTExpr::Atom("42".into()).to_string(),
        "42",
        "原子整数字面量应直接输出数值"
    );
}

#[test]
fn test_display_atom_var() {
    assert_eq!(
        SMTExpr::Atom("x".into()).to_string(),
        "x",
        "原子变量名应直接输出名称"
    );
}

#[test]
fn test_display_app_gt() {
    let expr = SMTExpr::App(
        ">".into(),
        vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("0".into())],
    );
    assert_eq!(
        expr.to_string(),
        "(> x 0)",
        "SMT 函数应用应输出为 (op args...)"
    );
}

#[test]
fn test_display_app_and() {
    let expr = SMTExpr::App(
        "and".into(),
        vec![
            SMTExpr::App(
                ">".into(),
                vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("0".into())],
            ),
            SMTExpr::App(
                "<".into(),
                vec![SMTExpr::Atom("x".into()), SMTExpr::Atom("10".into())],
            ),
        ],
    );
    let result = expr.to_string();
    assert!(
        result.contains("and"),
        "复合表达式应包含 and 操作符: {result}"
    );
    assert!(
        result.contains("> x 0"),
        "复合表达式应包含 (> x 0) 子表达式: {result}"
    );
    assert!(
        result.contains("< x 10"),
        "复合表达式应包含 (< x 10) 子表达式: {result}"
    );
}

#[test]
fn test_display_command_assert() {
    let cmd = SMTCommand::Assert(SMTExpr::App(
        ">".into(),
        vec![SMTExpr::Atom("y".into()), SMTExpr::Atom("0".into())],
    ));
    assert_eq!(
        cmd.to_string(),
        "(assert (> y 0))",
        "SMT assert 命令应输出为 (assert expr)"
    );
}

#[test]
fn test_display_forall_no_trailing_space() {
    let expr = SMTExpr::Forall {
        vars: vec![("i".into(), SMTSort::Int)],
        body: Box::new(SMTExpr::App(
            ">=".into(),
            vec![SMTExpr::Atom("i".into()), SMTExpr::Atom("0".into())],
        )),
    };
    let result = expr.to_string();
    assert_eq!(
        result, "(forall ((i Int)) (>= i 0))",
        "forall 单变量不应有尾随空格: {result}"
    );
}

#[test]
fn test_display_forall_multiple_vars_no_trailing_space() {
    let expr = SMTExpr::Forall {
        vars: vec![("x".into(), SMTSort::Int), ("y".into(), SMTSort::Int)],
        body: Box::new(SMTExpr::Atom("true".into())),
    };
    let result = expr.to_string();
    assert_eq!(
        result, "(forall ((x Int) (y Int)) true)",
        "forall 多变量应以空格分隔，无多余空格: {result}"
    );
}

#[test]
fn test_smt_result_variants() {
    // Unsat
    let _ = SMTResult::Unsat;
    // Sat with model
    let _ = SMTResult::Sat {
        model: SMTModel {
            assignments: vec![("x".into(), "0".into())],
        },
    };
    // Unknown
    let _ = SMTResult::Unknown {
        reason: "timeout".into(),
    };
}

#[test]
fn test_smt_sort_display() {
    assert_eq!(SMTSort::Bool.to_string(), "Bool");
    assert_eq!(SMTSort::Int.to_string(), "Int");
    assert_eq!(SMTSort::Real.to_string(), "Real");
}
