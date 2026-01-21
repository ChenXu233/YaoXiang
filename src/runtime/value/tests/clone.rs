//! clone() tests for RuntimeValue

use std::sync::Arc;
use crate::runtime::value::{RuntimeValue, TypeId, FunctionId, FunctionValue, Heap, HeapValue};

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
    // Clone of list with Handle system is shallow - both point to same heap data
    // This is expected behavior for Handle system (like pointers)
    let mut heap = Heap::new();
    let handle = heap.allocate(HeapValue::List(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Int(2),
        RuntimeValue::Int(3),
    ]));
    let list1 = RuntimeValue::List(handle);
    let list2 = list1.clone();

    // Both should be List with same length (pointing to same heap data)
    let l1 = heap.get(handle).unwrap();
    let l2 = if let RuntimeValue::List(h) = list2 {
        heap.get(h)
    } else {
        None
    }
    .unwrap();

    match (l1, l2) {
        (HeapValue::List(v1), HeapValue::List(v2)) => {
            assert_eq!(v1.len(), 3);
            assert_eq!(v2.len(), 3);
            // Both point to same data
            assert!(std::ptr::eq(&**v1, &**v2));
        }
        _ => panic!("Expected List"),
    }

    // Modifying via list2 affects the shared heap data
    match list2 {
        RuntimeValue::List(h) => {
            if let Some(HeapValue::List(v)) = heap.get_mut(h) {
                v.push(RuntimeValue::Int(4));
                assert_eq!(v.len(), 4);
            }
        }
        _ => panic!(),
    }

    // list1 now also sees the modification (same heap data)
    match heap.get(handle) {
        Some(HeapValue::List(v)) => {
            assert_eq!(v.len(), 4); // Modified via list2
        }
        _ => panic!(),
    }
}

#[test]
fn test_clone_struct() {
    let mut heap = Heap::new();
    let fields_handle = heap.allocate(HeapValue::Tuple(vec![
        RuntimeValue::Int(10),
        RuntimeValue::Float(20.0),
    ]));
    let s1 = RuntimeValue::Struct {
        type_id: TypeId(1),
        fields: fields_handle,
    };
    let s2 = s1.clone();

    // Both should be Struct with same field count
    match (&s1, &s2) {
        (RuntimeValue::Struct { fields: f1, .. }, RuntimeValue::Struct { fields: f2, .. }) => {
            // Check via heap that both have 2 fields
            let fields1 = heap.get(*f1).unwrap();
            let fields2 = heap.get(*f2).unwrap();
            match (fields1, fields2) {
                (HeapValue::Tuple(v1), HeapValue::Tuple(v2)) => {
                    assert_eq!(v1.len(), 2);
                    assert_eq!(v2.len(), 2);
                }
                _ => panic!("Expected Tuple"),
            }
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
    let mut heap = Heap::new();
    let tuple_handle = heap.allocate(HeapValue::Tuple(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Bool(true),
        RuntimeValue::String(Arc::from("test")),
    ]));
    let t1 = RuntimeValue::Tuple(tuple_handle);
    let t2 = t1.clone();

    match (t1, t2) {
        (RuntimeValue::Tuple(h1), RuntimeValue::Tuple(h2)) => {
            let v1 = heap.get(h1).unwrap();
            let v2 = heap.get(h2).unwrap();
            match (v1, v2) {
                (HeapValue::Tuple(v1_inner), HeapValue::Tuple(v2_inner)) => {
                    assert_eq!(v1_inner.len(), 3);
                    assert_eq!(v2_inner.len(), 3);
                }
                _ => panic!("Expected Tuple"),
            }
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
