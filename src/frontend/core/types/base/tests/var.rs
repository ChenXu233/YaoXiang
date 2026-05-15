//! 类型变量测试 — 基于语言规范 §3
//!
//! TypeVar: 类型推断中的变量标识
//! ConstVar: Const泛型参数变量标识

use crate::frontend::core::types::base::{ConstVar, TypeVar};

#[test]
fn test_type_var_new_and_index() {
    let v = TypeVar::new(0);
    assert_eq!(v.index(), 0);
}

#[test]
fn test_type_var_index_tracking() {
    let v0 = TypeVar::new(0);
    let v1 = TypeVar::new(1);
    let v5 = TypeVar::new(5);
    assert_eq!(v0.index(), 0);
    assert_eq!(v1.index(), 1);
    assert_eq!(v5.index(), 5);
}

#[test]
fn test_type_var_display() {
    assert_eq!(format!("{}", TypeVar::new(42)), "t42");
}

#[test]
fn test_type_var_equality() {
    assert_eq!(TypeVar::new(0), TypeVar::new(0));
    assert_ne!(TypeVar::new(0), TypeVar::new(1));
}

#[test]
fn test_const_var_new_and_index() {
    assert_eq!(ConstVar::new(0).index(), 0);
}

#[test]
fn test_const_var_display() {
    assert_eq!(format!("{}", ConstVar::new(7)), "c7");
}

#[test]
fn test_const_var_equality() {
    assert_eq!(ConstVar::new(3), ConstVar::new(3));
    assert_ne!(ConstVar::new(3), ConstVar::new(4));
}
