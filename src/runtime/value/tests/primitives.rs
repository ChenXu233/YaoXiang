//! Primitive type tests for RuntimeValue

use std::sync::Arc;
use crate::runtime::value::{RuntimeValue, ValueType, IntWidth, FloatWidth};

#[test]
fn test_unit_value() {
    let v = RuntimeValue::Unit;
    assert_eq!(v.value_type(), ValueType::Unit);
    assert!(matches!(v, RuntimeValue::Unit));
}

#[test]
fn test_bool_values() {
    let t = RuntimeValue::Bool(true);
    let f = RuntimeValue::Bool(false);

    assert_eq!(t.value_type(), ValueType::Bool);
    assert_eq!(f.value_type(), ValueType::Bool);

    assert_eq!(t.to_bool(), Some(true));
    assert_eq!(f.to_bool(), Some(false));

    assert!(t.to_int().is_none());
    assert!(t.to_float().is_none());
}

#[test]
fn test_int_values() {
    let v = RuntimeValue::Int(42);
    assert_eq!(v.value_type(), ValueType::Int(IntWidth::I64));
    assert_eq!(v.to_int(), Some(42));
    assert_eq!(v.to_int(), Some(42));
}

#[test]
fn test_float_values() {
    let v = RuntimeValue::Float(3.14);
    assert_eq!(v.value_type(), ValueType::Float(FloatWidth::F64));
    assert_eq!(v.to_float(), Some(3.14));
    assert!(v.to_int().is_none());
}

#[test]
fn test_char_values() {
    let v = RuntimeValue::Char('A' as u32);
    assert_eq!(v.value_type(), ValueType::Char);
    // Can't use assert_eq! without PartialEq, just check it's Char with value 65
    if let RuntimeValue::Char(c) = v {
        assert_eq!(c, 65);
    } else {
        panic!("Expected Char");
    }
}

#[test]
fn test_char_display() {
    // Test that char display works correctly
    let a = RuntimeValue::Char('A' as u32);
    assert_eq!(format!("{}", a), "A");

    // Test invalid unicode
    let invalid = RuntimeValue::Char(0x10FFFF + 1);
    assert!(format!("{}", invalid).starts_with("U+"));
}

#[test]
fn test_string_values() {
    let s = RuntimeValue::String(Arc::from("hello"));
    assert_eq!(s.value_type(), ValueType::String);
    assert!(matches!(s, RuntimeValue::String(_)));
}

#[test]
fn test_bytes_values() {
    let bytes: Arc<[u8]> = Arc::new([1, 2, 3, 4]);
    let b = RuntimeValue::Bytes(bytes);
    assert_eq!(b.value_type(), ValueType::Bytes);
    assert!(matches!(b, RuntimeValue::Bytes(_)));
}

#[test]
fn test_is_type_check() {
    let int_val = RuntimeValue::Int(42);
    let float_val = RuntimeValue::Float(3.14);

    assert!(int_val.is_type(&ValueType::Int(IntWidth::I64)));
    assert!(!int_val.is_type(&ValueType::Float(FloatWidth::F64)));
    assert!(float_val.is_type(&ValueType::Float(FloatWidth::F64)));
}
