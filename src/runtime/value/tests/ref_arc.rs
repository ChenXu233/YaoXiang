//! ref/Arc tests for RuntimeValue
//!
//! Tests for the Arc wrapper, which is the runtime representation
//! of the `ref` keyword per RFC-009.

use std::sync::Arc;
use crate::runtime::value::{RuntimeValue, TypeId, Heap, HeapValue};

#[test]
fn test_arc_creation() {
    // ref keyword → Arc(RuntimeValue) = Arc<RuntimeValue>
    let inner = RuntimeValue::Int(42);
    let arc = RuntimeValue::Arc(Arc::new(inner));

    assert!(matches!(arc, RuntimeValue::Arc(_)));
}

#[test]
fn test_arc_clone() {
    // Arc can be cloned (reference count increases)
    let inner = RuntimeValue::Int(42);
    let arc1 = RuntimeValue::Arc(Arc::new(inner));
    let arc2 = arc1.clone();

    assert!(matches!(arc1, RuntimeValue::Arc(_)));
    assert!(matches!(arc2, RuntimeValue::Arc(_)));
}

#[test]
fn test_arc_as_inner() {
    // Get inner value from Arc
    let inner = RuntimeValue::Int(42);
    let arc = RuntimeValue::Arc(Arc::new(inner));

    let extracted = arc.as_arc().unwrap();
    assert_eq!(extracted.to_int(), Some(42));
}

#[test]
fn test_arc_is_arc() {
    let int_val = RuntimeValue::Int(42);
    let arc_val = RuntimeValue::Arc(Arc::new(int_val.clone()));

    assert!(!int_val.is_arc());
    assert!(arc_val.is_arc());
}

#[test]
fn test_arc_not_arc() {
    let v = RuntimeValue::Int(42);
    assert!(!v.is_arc());
}

#[test]
fn test_into_arc() {
    // into_arc converts a value to Arc
    let v = RuntimeValue::Float(3.14);
    let arc = v.into_arc();

    assert!(arc.is_arc());
    assert!(matches!(arc, RuntimeValue::Arc(_)));

    // Inner value should be the same
    let inner = arc.as_arc().unwrap();
    assert!(matches!(inner, RuntimeValue::Float(_)));
}

#[test]
fn test_from_arc() {
    // Create Arc from Arc<RuntimeValue>
    let inner = RuntimeValue::Int(100);
    let std_arc: Arc<RuntimeValue> = Arc::new(inner);
    let rv = RuntimeValue::from_arc(std_arc);

    assert!(rv.is_arc());
    assert_eq!(rv.as_arc().unwrap().to_int(), Some(100));
}

#[test]
fn test_arc_clone_shared() {
    // Cloning Arc shares the same underlying data
    let inner = RuntimeValue::Int(100);
    let arc1 = RuntimeValue::Arc(Arc::new(inner));
    let arc2 = arc1.clone();

    // Both should see the same value
    assert_eq!(arc1.as_arc().unwrap().to_int(), Some(100));
    assert_eq!(arc2.as_arc().unwrap().to_int(), Some(100));
}

#[test]
fn test_arc_nested_values() {
    // Arc can contain any RuntimeValue
    let mut heap = Heap::new();
    let list_handle = heap.allocate(HeapValue::List(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Int(2),
        RuntimeValue::Int(3),
    ]));
    let list = RuntimeValue::List(list_handle);

    let arc_list = RuntimeValue::Arc(Arc::new(list));

    let inner_list = arc_list.as_arc().unwrap();
    match inner_list {
        RuntimeValue::List(handle) => {
            if let Some(HeapValue::List(items)) = heap.get(*handle) {
                assert_eq!(items.len(), 3);
            } else {
                panic!("Expected List in heap");
            }
        }
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_arc_struct() {
    // Arc can contain struct values
    let mut heap = Heap::new();
    let fields_handle = heap.allocate(HeapValue::Tuple(vec![
        RuntimeValue::Float(1.0),
        RuntimeValue::Float(2.0),
    ]));
    let point = RuntimeValue::Struct {
        type_id: TypeId(1),
        fields: fields_handle,
    };

    let arc_point = RuntimeValue::Arc(Arc::new(point));

    let inner = arc_point.as_arc().unwrap();
    match inner {
        RuntimeValue::Struct { fields, .. } => {
            if let Some(HeapValue::Tuple(items)) = heap.get(*fields) {
                assert_eq!(items.len(), 2);
            } else {
                panic!("Expected Tuple in heap");
            }
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_arc_ref_counting() {
    // Test that Arc properly shares data
    let inner = RuntimeValue::Int(42);
    let std_arc: Arc<RuntimeValue> = Arc::new(inner);

    // Strong count should be 1
    assert_eq!(Arc::strong_count(&std_arc), 1);

    let arc1 = RuntimeValue::from_arc(std_arc.clone());
    // Strong count should be 2
    assert_eq!(Arc::strong_count(&std_arc), 2);

    let arc2 = arc1.clone();
    // Strong count should be 3
    assert_eq!(Arc::strong_count(&std_arc), 3);

    // All arcs point to same value
    assert_eq!(arc1.as_arc().unwrap().to_int(), Some(42));
    assert_eq!(arc2.as_arc().unwrap().to_int(), Some(42));
}
