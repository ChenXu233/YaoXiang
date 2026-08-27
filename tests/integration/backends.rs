//! Backend integration tests
//!
//! Tests for the new backend architecture including interpreter,
//! common components, and executor functionality.

use yaoxiang::backends::common::{RuntimeValue, Heap, Handle, heap::HeapValue};
use yaoxiang::backends::{ExecutorConfig, ExecutionState};
use yaoxiang::middle::bytecode::{BytecodeModule, BytecodeFunction};
use yaoxiang::middle::{ConstValue, Type};

#[test]
fn test_executor_config_default() {
    let config = ExecutorConfig::default();

    assert_eq!(config.max_stack_depth, 1024);
}

#[test]
fn test_executor_config_custom() {
    let config = ExecutorConfig {
        max_stack_depth: 2048,
    };

    assert_eq!(config.max_stack_depth, 2048);
}

#[test]
fn test_heap_creation() {
    let _heap = Heap::new();

    // Verify heap can be created
}

#[test]
fn test_execution_state_default() {
    let state = ExecutionState::default();

    assert_eq!(state.call_depth, 0);
    assert_eq!(state.ip, 0);
    assert!(state.current_function.is_none());
    assert!(!state.is_complete);
}

#[test]
fn test_runtime_value_types() {
    // Test RuntimeValue can be created
    let val1 = RuntimeValue::Int(42);
    let val2 = RuntimeValue::Int(42);
    let val3 = RuntimeValue::Int(100);

    assert_eq!(val1, val2);
    assert_ne!(val1, val3);
}

#[test]
fn test_bytecode_module_creation() {
    let module = BytecodeModule::new("test".to_string());

    assert_eq!(module.name, "test");
    assert!(module.constants.is_empty());
    assert!(module.functions.is_empty());
    assert!(module.type_table.is_empty());
    assert!(module.globals.is_empty());
    assert!(module.entry_point.is_none());
}

#[test]
fn test_bytecode_module_add_constant() {
    let mut module = BytecodeModule::new("test".to_string());

    let idx1 = module.add_constant(ConstValue::Int(42));
    let idx2 = module.add_constant(ConstValue::Int(100));

    assert_eq!(idx1, 0);
    assert_eq!(idx2, 1);
    assert_eq!(module.constants.len(), 2);
}

#[test]
fn test_bytecode_module_add_function() {
    let mut module = BytecodeModule::new("test".to_string());

    let func = BytecodeFunction {
        name: "test_func".to_string(),
        params: vec![Type::Int(0), Type::Int(0)],
        return_type: Type::Int(0),
        local_count: 0,
        upvalue_count: 0,
        instructions: vec![],
        labels: std::collections::HashMap::new(),
        exception_handlers: vec![],
        debug_map: std::collections::HashMap::new(),
    };

    let idx = module.add_function(func.clone());

    assert_eq!(idx, 0);
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "test_func");
}

#[test]
fn test_handle_creation() {
    // Arrange
    let handle = Handle::new(HeapValue::List(vec![]));
    let other = Handle::new(HeapValue::List(vec![]));

    // Act
    let raw = handle.raw();
    let raw_other = other.raw();

    // Assert：raw 返回唯一标识（Arc 指针地址），不同分配必须不同
    assert!(
        raw != 0,
        "raw should return a non-zero identity for a live handle"
    );
    assert_ne!(
        raw, raw_other,
        "distinct handles should have distinct raw ids"
    );
}

#[test]
fn test_handle_display() {
    // Arrange
    let handle = Handle::new(HeapValue::List(vec![]));

    // Act
    let rendered = format!("{}", handle);

    // Assert：Display 以 handle@ 前缀输出指针地址
    assert!(
        rendered.starts_with("handle@0x"),
        "Display should render as handle@0x..., got '{rendered}'"
    );
}

#[test]
fn test_const_value_types() {
    use yaoxiang::middle::ConstValue;

    // Test various constant types
    let int_val = ConstValue::Int(42);
    let float_val = ConstValue::Float(std::f64::consts::PI);
    let string_val = ConstValue::String("test".to_string());
    let bool_val = ConstValue::Bool(true);

    // These should all be constructible
    assert_eq!(int_val, ConstValue::Int(42));
    assert_eq!(float_val, ConstValue::Float(std::f64::consts::PI));
    assert_eq!(string_val, ConstValue::String("test".to_string()));
    assert_eq!(bool_val, ConstValue::Bool(true));
}
