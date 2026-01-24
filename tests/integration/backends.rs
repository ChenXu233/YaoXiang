//! Backend integration tests
//!
//! Tests for the new backend architecture including interpreter,
//! common components, and executor functionality.

use yaoxiang::backends::common::{RuntimeValue, Heap, Handle};
use yaoxiang::backends::{ExecutorConfig, ExecutionState};
use yaoxiang::middle::bytecode::{BytecodeModule, BytecodeFunction};
use yaoxiang::middle::ir::ConstValue;
use yaoxiang::middle::ir;

#[test]
fn test_executor_config_default() {
    let config = ExecutorConfig::default();

    assert_eq!(config.max_stack_depth, 1024);
    assert_eq!(config.initial_heap_size, 64 * 1024);
    assert_eq!(config.max_heap_size, 64 * 1024 * 1024);
    assert!(config.enable_checks);
    assert!(config.enable_debug);
}

#[test]
fn test_executor_config_custom() {
    let config = ExecutorConfig {
        max_stack_depth: 2048,
        initial_heap_size: 128 * 1024,
        max_heap_size: 128 * 1024 * 1024,
        build_mode: yaoxiang::backends::BuildMode::Release,
        enable_checks: false,
        enable_debug: false,
    };

    assert_eq!(config.max_stack_depth, 2048);
    assert_eq!(config.initial_heap_size, 128 * 1024);
    assert_eq!(config.max_heap_size, 128 * 1024 * 1024);
    assert!(!config.enable_checks);
    assert!(!config.enable_debug);
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
        params: vec![ir::Type::Int(0), ir::Type::Int(0)],
        return_type: ir::Type::Int(0),
        local_count: 0,
        upvalue_count: 0,
        instructions: vec![],
        labels: std::collections::HashMap::new(),
        exception_handlers: vec![],
    };

    let idx = module.add_function(func.clone());

    assert_eq!(idx, 0);
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "test_func");
}

#[test]
fn test_build_mode_variants() {
    use yaoxiang::backends::BuildMode;

    let debug = BuildMode::Debug;
    let release = BuildMode::Release;
    let profile = BuildMode::Profile;

    // Verify they are different variants
    assert_ne!(debug, release);
    assert_ne!(debug, profile);
    assert_ne!(release, profile);
}

#[test]
fn test_handle_creation() {
    let handle = Handle::new(42);
    assert_eq!(handle.raw(), 42);
}

#[test]
fn test_handle_display() {
    let handle = Handle::new(42);
    assert_eq!(format!("{}", handle), "handle@42");
}

#[test]
fn test_const_value_types() {
    use yaoxiang::middle::ir::ConstValue;

    // Test various constant types
    let int_val = ConstValue::Int(42);
    let float_val = ConstValue::Float(3.14);
    let string_val = ConstValue::String("test".to_string());
    let bool_val = ConstValue::Bool(true);

    // These should all be constructible
    assert_eq!(int_val, ConstValue::Int(42));
    assert_eq!(float_val, ConstValue::Float(3.14));
    assert_eq!(string_val, ConstValue::String("test".to_string()));
    assert_eq!(bool_val, ConstValue::Bool(true));
}
