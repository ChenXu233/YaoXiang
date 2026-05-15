//! 编译期常量数据测试 — 基于 RFC-011 §4

use crate::frontend::core::types::base::{BinOp, ConstKind, ConstValue, ConstVarDef, UnOp};

#[test]
fn test_const_value_int_creation() {
    let v = ConstValue::Int(42);
    assert!(v.is_numeric());
    assert_eq!(v.kind(), ConstKind::Int(None));
    assert_eq!(format!("{}", v), "42");
}

#[test]
fn test_const_value_bool_creation() {
    let v = ConstValue::Bool(true);
    assert!(!v.is_numeric());
    assert_eq!(v.kind(), ConstKind::Bool);
    assert_eq!(format!("{}", v), "true");
}

#[test]
fn test_const_value_float_creation() {
    let v = ConstValue::Float(3.14);
    assert!(v.is_numeric());
    assert_eq!(v.kind(), ConstKind::Float(None));
}

#[test]
fn test_const_value_equality() {
    assert_ne!(ConstValue::Int(5), ConstValue::Float(5.0));
    assert_ne!(ConstValue::Int(5), ConstValue::Bool(true));
    let a = ConstValue::Float(f32::NAN);
    let b = ConstValue::Float(f32::NAN);
    assert_eq!(a, b);
}

#[test]
fn test_const_value_from_literal_name() {
    assert_eq!(
        ConstValue::from_literal_name("42"),
        Some(ConstValue::Int(42))
    );
    assert!(ConstValue::from_literal_name("3.14").is_some());
    assert_eq!(
        ConstValue::from_literal_name("true"),
        Some(ConstValue::Bool(true))
    );
    assert_eq!(
        ConstValue::from_literal_name("false"),
        Some(ConstValue::Bool(false))
    );
    assert_eq!(ConstValue::from_literal_name("hello"), None);
    assert_eq!(ConstValue::from_literal_name(""), None);
}

#[test]
fn test_const_value_is_valid_literal_name() {
    assert!(ConstValue::is_valid_literal_name("42"));
    assert!(ConstValue::is_valid_literal_name("3.14"));
    assert!(ConstValue::is_valid_literal_name("true"));
    assert!(!ConstValue::is_valid_literal_name("hello"));
}

#[test]
fn test_const_kind_matches() {
    assert!(ConstKind::Int(None).matches(&ConstValue::Int(5)));
    assert!(ConstKind::Bool.matches(&ConstValue::Bool(true)));
    assert!(!ConstKind::Int(None).matches(&ConstValue::Bool(true)));
}

#[test]
fn test_const_kind_type_name() {
    assert_eq!(ConstKind::Int(None).type_name(), "Int");
    assert_eq!(ConstKind::Bool.type_name(), "Bool");
    assert_eq!(ConstKind::Float(None).type_name(), "Float");
}

#[test]
fn test_const_var_def_new() {
    let def = ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0);
    assert_eq!(def.name, "N");
    assert_eq!(def.kind, ConstKind::Int(None));
    assert_eq!(def.index, 0);
    assert_eq!(format!("{}", def), "N");
}

#[test]
fn test_bin_op_is_arithmetic() {
    assert!(BinOp::Add.is_arithmetic());
    assert!(BinOp::Sub.is_arithmetic());
    assert!(BinOp::Mul.is_arithmetic());
    assert!(BinOp::Div.is_arithmetic());
    assert!(BinOp::Mod.is_arithmetic());
    assert!(!BinOp::Eq.is_arithmetic());
}

#[test]
fn test_bin_op_is_comparison() {
    assert!(BinOp::Eq.is_comparison());
    assert!(BinOp::Ne.is_comparison());
    assert!(BinOp::Lt.is_comparison());
    assert!(!BinOp::Add.is_comparison());
}

#[test]
fn test_bin_op_is_logical() {
    assert!(BinOp::And.is_logical());
    assert!(BinOp::Or.is_logical());
    assert!(!BinOp::Add.is_logical());
}

#[test]
fn test_bin_op_is_bitwise() {
    assert!(BinOp::BitAnd.is_bitwise());
    assert!(BinOp::BitOr.is_bitwise());
    assert!(BinOp::BitXor.is_bitwise());
    assert!(BinOp::Shl.is_bitwise());
    assert!(BinOp::Shr.is_bitwise());
    assert!(!BinOp::Add.is_bitwise());
}

#[test]
fn test_un_op_classification() {
    assert!(UnOp::Pos.is_arithmetic());
    assert!(UnOp::Neg.is_arithmetic());
    assert!(UnOp::Not.is_logical());
    assert!(UnOp::BitNot.is_bitwise());
}
