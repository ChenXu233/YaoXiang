//! Handle type tests for RuntimeValue
//!
//! Tests the interaction between RuntimeValue and the Handle/Heap system.

use std::sync::Arc;
use std::collections::HashMap;
use crate::runtime::value::{RuntimeValue, ValueType, FloatWidth, TypeId, FunctionId, FunctionValue};
use crate::runtime::value::heap::{Heap, HeapValue, Handle};

#[test]
fn test_handle_creation() {
    let handle = Handle::new(42);
    assert_eq!(handle.raw(), 42);
}

#[test]
fn test_handle_display() {
    let handle = Handle::new(123);
    assert_eq!(format!("{}", handle), "handle@123");
}

#[test]
fn test_tuple_with_heap() {
    let mut heap = Heap::new();
    let tuple = RuntimeValue::Tuple(heap.allocate(HeapValue::Tuple(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Int(2),
        RuntimeValue::Int(3),
    ])));

    // Note: value_type() returns List for handle types (known limitation)
    // Use value_type(Some(heap)) for full type information
    let full_type = tuple.value_type(Some(&heap));
    match full_type {
        ValueType::Tuple(types) => {
            assert_eq!(types.len(), 3);
        }
        _ => panic!("expected Tuple type with heap access"),
    }
}

#[test]
fn test_array_with_heap() {
    let mut heap = Heap::new();
    let array = RuntimeValue::Array(heap.allocate(HeapValue::Array(vec![
        RuntimeValue::Float(1.0),
        RuntimeValue::Float(2.0),
        RuntimeValue::Float(3.0),
    ])));

    // Verify array value type
    let full_type = array.value_type(Some(&heap));
    match full_type {
        ValueType::Array { element } => {
            assert!(matches!(*element, ValueType::Float(FloatWidth::F64)));
        }
        _ => panic!("expected Array type"),
    }
}

#[test]
fn test_list_with_heap() {
    let mut heap = Heap::new();
    let list = RuntimeValue::List(heap.allocate(HeapValue::List(vec![
        RuntimeValue::String(Arc::from("hello")),
        RuntimeValue::String(Arc::from("world")),
    ])));

    assert_eq!(list.value_type(None), ValueType::List);

    // Verify with heap access
    let full_type = list.value_type(Some(&heap));
    match full_type {
        ValueType::List => {
            // List type doesn't contain element info
        }
        _ => panic!("expected List type"),
    }
}

#[test]
fn test_dict_with_heap() {
    let mut heap = Heap::new();
    let mut map = HashMap::new();
    map.insert(RuntimeValue::Int(1), RuntimeValue::String(Arc::from("one")));
    map.insert(RuntimeValue::Int(2), RuntimeValue::String(Arc::from("two")));

    let dict = RuntimeValue::Dict(heap.allocate(HeapValue::Dict(map)));

    // Verify dict contents via heap access
    if let RuntimeValue::Dict(handle) = dict {
        if let Some(HeapValue::Dict(m)) = heap.get(handle) {
            assert_eq!(m.len(), 2);
        } else {
            panic!("expected Dict in heap");
        }
    } else {
        panic!("expected Dict");
    }
}

#[test]
fn test_struct_with_heap() {
    let mut heap = Heap::new();
    let type_id = TypeId(1);

    let struct_val = RuntimeValue::Struct {
        type_id,
        fields: heap.allocate(HeapValue::Tuple(vec![
            RuntimeValue::Float(1.0),
            RuntimeValue::Float(2.0),
        ])),
        vtable: Vec::new(),
    };

    assert_eq!(struct_val.value_type(None), ValueType::Struct(type_id));
}

#[test]
fn test_struct_field_access() {
    let mut heap = Heap::new();
    let type_id = TypeId(1);

    let struct_val = RuntimeValue::Struct {
        type_id,
        fields: heap.allocate(HeapValue::Tuple(vec![
            RuntimeValue::Float(10.0),
            RuntimeValue::Float(20.0),
        ])),
        vtable: Vec::new(),
    };

    // Access field through heap
    let field = struct_val.struct_field_with_heap(0, &heap);
    match field {
        Some(RuntimeValue::Float(f)) => {
            assert_eq!(*f, 10.0);
        }
        _ => panic!("expected first field"),
    }

    let field = struct_val.struct_field_with_heap(1, &heap);
    match field {
        Some(RuntimeValue::Float(f)) => {
            assert_eq!(*f, 20.0);
        }
        _ => panic!("expected second field"),
    }
}

#[test]
fn test_clone_with_heap() {
    let mut heap = Heap::new();
    let original = RuntimeValue::List(heap.allocate(HeapValue::List(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Int(2),
        RuntimeValue::Int(3),
    ])));

    let original_len = heap.len();

    // Clone should create a new heap allocation
    let cloned = original.explicit_clone_with_heap(&mut heap);

    // After cloning, we should have one more allocation
    assert_eq!(heap.len(), original_len + 1);

    // Original and cloned should have different handles
    if let (RuntimeValue::List(h1), RuntimeValue::List(h2)) = (original, cloned) {
        assert_ne!(h1, h2);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_empty_collection_with_heap() {
    let mut heap = Heap::new();

    // Empty tuple
    let empty_tuple = RuntimeValue::Tuple(heap.allocate(HeapValue::Tuple(vec![])));
    let type_info = empty_tuple.value_type(Some(&heap));
    match type_info {
        ValueType::Tuple(types) => {
            assert!(types.is_empty());
        }
        _ => panic!("expected empty Tuple"),
    }

    // Empty list
    let empty_list = RuntimeValue::List(heap.allocate(HeapValue::List(vec![])));
    if let RuntimeValue::List(handle) = empty_list {
        assert!(heap.get(handle).unwrap().is_empty());
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_handle_equality() {
    let h1 = Handle::new(1);
    let h2 = Handle::new(1);
    let h3 = Handle::new(2);

    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

#[test]
fn test_handle_copy() {
    let handle = Handle::new(42);
    let copied = handle; // Copy since Copy is derived
    assert_eq!(handle, copied);
}

#[test]
fn test_heap_operations() {
    let mut heap = Heap::new();

    // Allocate multiple values
    let h1 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    let h2 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(2)]));
    let h3 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(3)]));

    assert_eq!(heap.len(), 3);
    assert!(heap.is_valid(h1));
    assert!(heap.is_valid(h2));
    assert!(heap.is_valid(h3));

    // Deallocate and verify
    heap.deallocate(h2);
    assert_eq!(heap.len(), 2);
    assert!(!heap.is_valid(h2));
}

#[test]
fn test_heap_free_list_reuse() {
    let mut heap = Heap::new();

    let h1 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    let h2 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(2)]));

    // Deallocate h2 - it goes to free list
    heap.deallocate(h2);

    // Allocate again - should reuse handle from free list
    let h3 = heap.allocate(HeapValue::List(vec![RuntimeValue::Int(3)]));
    // h1 = 0, h2 = 1 (deallocated), h3 reuses 1 from free list
    assert_eq!(h1.raw(), 0);
    assert_eq!(h2.raw(), 1);
    assert_eq!(h3.raw(), 1); // Reused from free list
}

#[test]
fn test_heap_clear() {
    let mut heap = Heap::new();
    heap.allocate(HeapValue::List(vec![RuntimeValue::Int(1)]));
    heap.allocate(HeapValue::List(vec![RuntimeValue::Int(2)]));

    assert_eq!(heap.len(), 2);

    heap.clear();

    assert!(heap.is_empty());
}

#[test]
fn test_struct_vtable() {
    let type_id = TypeId(1);
    let func_id = FunctionId(100);

    let struct_val = RuntimeValue::Struct {
        type_id,
        fields: Handle::new(0), // placeholder
        vtable: vec![
            (
                "method1".to_string(),
                FunctionValue {
                    func_id,
                    env: vec![],
                },
            ),
            (
                "method2".to_string(),
                FunctionValue {
                    func_id: FunctionId(101),
                    env: vec![],
                },
            ),
        ],
    };

    // Get method by name
    let method = struct_val.get_method("method1");
    match method {
        Some(f) => {
            assert_eq!(f.func_id, func_id);
        }
        None => panic!("expected method1"),
    }

    // Non-existent method
    assert!(struct_val.get_method("nonexistent").is_none());
}

#[test]
fn test_struct_vtable_display() {
    let struct_val = RuntimeValue::Struct {
        type_id: TypeId(1),
        fields: Handle::new(42),
        vtable: Vec::new(),
    };

    // Display should show struct@42
    assert_eq!(format!("{}", struct_val), "struct@42");
}
