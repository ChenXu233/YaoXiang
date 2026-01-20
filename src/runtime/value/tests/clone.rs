//! clone() tests for RuntimeValue

use std::sync::Arc;
use crate::runtime::value::{RuntimeValue, TypeId, FunctionId, FunctionValue};

#[test]
fn test_clone_primitives() {
    // Clone of primitive types should be independent
    let v1 = RuntimeValue::Int(42);
    let v2 = v1.clone();

    assert_eq!(v1.to_int(), Some(42));
    assert_eq!(v2.to_int(), Some(42));

    // Both should be Int type
    assert!(matches!(v1, RuntimeValue::Int(_)));
    assert!(matches!(v2, RuntimeValue::Int(_)));
}

#[test]
fn test_clone_bool() {
    let t = RuntimeValue::Bool(true);
    let t_clone = t.clone();

    assert!(matches!(t_clone, RuntimeValue::Bool(true)));
    assert!(t_clone.to_bool().unwrap());
}

#[test]
fn test_clone_float() {
    let f = RuntimeValue::Float(3.14);
    let f_clone = f.clone();

    assert!(matches!(f_clone, RuntimeValue::Float(_)));
    assert!((f_clone.to_float().unwrap() - 3.14).abs() < 0.001);
}

#[test]
fn test_clone_string() {
    // String uses Arc internally, so clone should share the same Arc
    let s1 = RuntimeValue::String(Arc::from("hello"));
    let s2 = s1.clone();

    // Both should have the same content
    match (&s1, &s2) {
        (RuntimeValue::String(a), RuntimeValue::String(b)) => {
            assert_eq!(&**a, &**b);
        }
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_clone_list() {
    // Clone of list should be independent (deep copy)
    let list1 = RuntimeValue::List(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Int(2),
        RuntimeValue::Int(3),
    ]);
    let list2 = list1.clone();

    // Both should be List with same length
    match (&list1, &list2) {
        (RuntimeValue::List(l1), RuntimeValue::List(l2)) => {
            assert_eq!(l1.len(), 3);
            assert_eq!(l2.len(), 3);
        }
        _ => panic!("Expected List"),
    }

    // But independent (modifying one doesn't affect the other)
    match list1 {
        RuntimeValue::List(mut v) => {
            v.push(RuntimeValue::Int(4));
            assert_eq!(v.len(), 4);
        }
        _ => panic!(),
    }

    match list2 {
        RuntimeValue::List(v) => {
            assert_eq!(v.len(), 3); // Original unchanged
        }
        _ => panic!(),
    }
}

#[test]
fn test_clone_struct() {
    let s1 = RuntimeValue::Struct {
        type_id: TypeId(1),
        fields: vec![RuntimeValue::Int(10), RuntimeValue::Float(20.0)],
    };
    let s2 = s1.clone();

    // Both should be Struct with same field count
    match (&s1, &s2) {
        (RuntimeValue::Struct { fields: f1, .. }, RuntimeValue::Struct { fields: f2, .. }) => {
            assert_eq!(f1.len(), 2);
            assert_eq!(f2.len(), 2);
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_clone_enum() {
    let e1 = RuntimeValue::Enum {
        type_id: TypeId(2),
        variant_id: 0,
        payload: Box::new(RuntimeValue::Int(100)),
    };
    let e2 = e1.clone();

    assert_eq!(e1.enum_variant_id(), Some(0));
    assert_eq!(e2.enum_variant_id(), Some(0));
}

#[test]
fn test_clone_arc() {
    // Clone of Arc should increase reference count (shared)
    let inner = RuntimeValue::Int(42);
    let arc1 = RuntimeValue::Arc(Arc::new(inner));
    let arc2 = arc1.clone();

    // Both should point to the same underlying value
    assert_eq!(arc1.as_arc().unwrap().to_int(), Some(42));
    assert_eq!(arc2.as_arc().unwrap().to_int(), Some(42));
}

#[test]
fn test_clone_tuple() {
    let t1 = RuntimeValue::Tuple(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Bool(true),
        RuntimeValue::String(Arc::from("test")),
    ]);
    let t2 = t1.clone();

    match (t1, t2) {
        (RuntimeValue::Tuple(v1), RuntimeValue::Tuple(v2)) => {
            assert_eq!(v1.len(), 3);
            assert_eq!(v2.len(), 3);
        }
        _ => panic!("Expected Tuple"),
    }
}

#[test]
fn test_clone_unit() {
    let u1 = RuntimeValue::Unit;
    let u2 = u1.clone();

    assert!(matches!(u1, RuntimeValue::Unit));
    assert!(matches!(u2, RuntimeValue::Unit));
}

#[test]
fn test_clone_function() {
    let f1 = RuntimeValue::Function(FunctionValue {
        func_id: FunctionId(1),
        env: vec![RuntimeValue::Int(10)],
    });
    let f2 = f1.clone();

    match (f1, f2) {
        (RuntimeValue::Function(f1_val), RuntimeValue::Function(f2_val)) => {
            assert_eq!(f1_val.func_id, f2_val.func_id);
            assert_eq!(f1_val.env.len(), f2_val.env.len());
        }
        _ => panic!("Expected Function"),
    }
}
