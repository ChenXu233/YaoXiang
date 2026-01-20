//! Struct and Enum tests for RuntimeValue

use std::sync::Arc;
use crate::runtime::value::{
    RuntimeValue, ValueType, TypeId, FunctionId, FunctionValue, AsyncValue, AsyncState, TaskId,
    IntWidth, FloatWidth,
};

/// Helper to create a struct type ID for testing
fn test_type_id() -> TypeId {
    TypeId(1)
}

/// Helper to create a result type ID for testing
fn result_type_id() -> TypeId {
    TypeId(2)
}

/// Helper to create a task ID for testing
fn test_task_id() -> TaskId {
    TaskId(1)
}

#[test]
fn test_struct_value() {
    // type Point = Point(x: Float, y: Float)
    let p = RuntimeValue::Struct {
        type_id: test_type_id(),
        fields: vec![RuntimeValue::Float(1.0), RuntimeValue::Float(2.0)],
    };

    assert_eq!(p.value_type(), ValueType::Struct(test_type_id()));

    // Test field access
    assert!(p.struct_field(0).is_some());
    assert!(p.struct_field(1).is_some());
    assert!(p.struct_field(2).is_none()); // Out of bounds

    // Check field values
    if let Some(f) = p.struct_field(0) {
        assert!(matches!(f, RuntimeValue::Float(_)));
    }
}

#[test]
fn test_struct_nested() {
    // type Point = Point(x: Float, y: Float)
    // type Rectangle = Rectangle(top_left: Point, bottom_right: Point)
    let top_left = RuntimeValue::Struct {
        type_id: test_type_id(),
        fields: vec![RuntimeValue::Float(0.0), RuntimeValue::Float(1.0)],
    };
    let bottom_right = RuntimeValue::Struct {
        type_id: test_type_id(),
        fields: vec![RuntimeValue::Float(2.0), RuntimeValue::Float(3.0)],
    };

    let rectangle = RuntimeValue::Struct {
        type_id: TypeId(3), // Rectangle type
        fields: vec![top_left.clone(), bottom_right],
    };

    assert_eq!(rectangle.value_type(), ValueType::Struct(TypeId(3)));

    // Access nested struct
    let tl = rectangle.struct_field(0).unwrap();
    let x = tl.struct_field(0).unwrap();
    assert_eq!(x.to_float(), Some(0.0));
}

#[test]
fn test_enum_ok_variant() {
    // type Result[T, E] = ok(T) | err(E)
    let ok = RuntimeValue::Enum {
        type_id: result_type_id(),
        variant_id: 0, // ok
        payload: Box::new(RuntimeValue::Int(42)),
    };

    assert_eq!(ok.value_type(), ValueType::Enum(result_type_id()));
    assert_eq!(ok.enum_variant_id(), Some(0));
    assert_eq!(ok.enum_payload().map(|p| p.to_int()), Some(Some(42)));
}

#[test]
fn test_enum_err_variant() {
    // type Result[T, E] = ok(T) | err(E)
    let err = RuntimeValue::Enum {
        type_id: result_type_id(),
        variant_id: 1, // err
        payload: Box::new(RuntimeValue::String(Arc::from("error message"))),
    };

    assert_eq!(err.value_type(), ValueType::Enum(result_type_id()));
    assert_eq!(err.enum_variant_id(), Some(1));
    assert!(err.enum_payload().unwrap().to_int().is_none());
}

#[test]
fn test_enum_unit_payload() {
    // Variant without payload (like Some(()))
    let unit_variant = RuntimeValue::Enum {
        type_id: TypeId(4),
        variant_id: 0,
        payload: Box::new(RuntimeValue::Unit),
    };

    assert_eq!(unit_variant.enum_variant_id(), Some(0));
    assert!(matches!(
        unit_variant.enum_payload().unwrap(),
        RuntimeValue::Unit
    ));
}

#[test]
fn test_tuple_value() {
    let tuple = RuntimeValue::Tuple(vec![
        RuntimeValue::Int(1),
        RuntimeValue::Float(2.0),
        RuntimeValue::Bool(true),
    ]);

    let expected_type = ValueType::Tuple(vec![
        ValueType::Int(IntWidth::I64),
        ValueType::Float(FloatWidth::F64),
        ValueType::Bool,
    ]);

    assert_eq!(tuple.value_type(), expected_type);
    assert!(matches!(tuple, RuntimeValue::Tuple(_)));
}

#[test]
fn test_empty_tuple() {
    let empty = RuntimeValue::Tuple(vec![]);
    assert_eq!(empty.value_type(), ValueType::Tuple(vec![]));
}

#[test]
fn test_function_value() {
    let func = RuntimeValue::Function(FunctionValue {
        func_id: FunctionId(42),
        env: vec![RuntimeValue::Int(100)],
    });

    assert_eq!(func.value_type(), ValueType::Function(FunctionId(42)));
    assert!(matches!(func, RuntimeValue::Function(_)));
}

#[test]
fn test_async_ready_value() {
    // Synchronously ready Async value
    let async_val = RuntimeValue::Async(Box::new(AsyncValue {
        state: Box::new(AsyncState::Ready(Box::new(RuntimeValue::Int(42)))),
        value_type: ValueType::Int(IntWidth::I64),
    }));

    assert!(matches!(async_val, RuntimeValue::Async(_)));

    // Access the inner value using dereference
    if let RuntimeValue::Async(box_async) = &async_val {
        let state_ref = &*box_async.state;
        match state_ref {
            AsyncState::Ready(val) => {
                assert_eq!(val.to_int(), Some(42));
            }
            _ => panic!("Expected Ready state"),
        }
    } else {
        panic!("Expected Async");
    }
}

#[test]
fn test_async_pending_value() {
    // Pending computation Async value
    let pending = RuntimeValue::Async(Box::new(AsyncValue {
        state: Box::new(AsyncState::Pending(test_task_id())),
        value_type: ValueType::Int(IntWidth::I64),
    }));

    if let RuntimeValue::Async(box_async) = &pending {
        let state_ref = &*box_async.state;
        match state_ref {
            AsyncState::Pending(tid) => {
                assert_eq!(*tid, test_task_id());
            }
            _ => panic!("Expected Pending state"),
        }
    } else {
        panic!("Expected Async");
    }
}

#[test]
fn test_async_error_value() {
    let error_async = RuntimeValue::Async(Box::new(AsyncValue {
        state: Box::new(AsyncState::Error(Box::new(RuntimeValue::String(
            Arc::from("error"),
        )))),
        value_type: ValueType::String,
    }));

    if let RuntimeValue::Async(box_async) = &error_async {
        let state_ref = &*box_async.state;
        match state_ref {
            AsyncState::Error(val) => {
                assert!(matches!(**val, RuntimeValue::String(_)));
            }
            _ => panic!("Expected Error state"),
        }
    } else {
        panic!("Expected Async");
    }
}

#[test]
fn test_dict_value() {
    // Dict type exists and has correct type
    let dict = RuntimeValue::Dict(std::collections::HashMap::new());

    assert_eq!(dict.value_type(), ValueType::Dict);
    assert!(matches!(dict, RuntimeValue::Dict(_)));
}
