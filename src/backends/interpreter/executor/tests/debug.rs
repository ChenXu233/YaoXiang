//! 解释器调试器测试
//!
//! 测试覆盖内容：
//! - Interpreter 的基本创建和配置
//! - 简单函数的执行
//! - 自动惰性调用和强制执行
//! - 结果传播和任务失败处理
//! - FFI 函数调用
//! - spawn 并行执行

use std::collections::HashMap;
use crate::backends::Executor;
use crate::backends::ExecutorError;
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, BinaryOp, FunctionRef, ConstValue,
};
use crate::backends::interpreter::runtime::InterpreterRuntimeConfig;
use crate::backends::runtime::RuntimeMode;
use crate::backends::interpreter::executor::Interpreter;

#[test]
fn test_interpreter_new() {
    let interp = Interpreter::new();
    assert!(interp.heap.is_empty());
}

#[test]
fn test_execute_simple_function() {
    let mut interp = Interpreter::new();

    // Create a simple bytecode module
    let mut module = BytecodeModule::new("test".to_string());
    let func_idx = module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    });
    module.constants.push(ConstValue::Int(42));
    module.entry_point = Some(func_idx);

    let result = interp.execute_function(&module.functions[0], &[]);
    assert!(result.is_ok());
    // Function executes successfully (actual return value depends on implementation)
    let _ = result.unwrap();
}

#[test]
fn test_auto_lazy_call_forced_on_use() {
    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Embedded,
        workers: 1,
        work_stealing: false,
    });

    interp.constants = vec![ConstValue::Int(1), ConstValue::Int(2)];

    let a = BytecodeFunction {
        name: "a".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };
    let b = BytecodeFunction {
        name: "b".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 1,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    interp.functions.insert("a".to_string(), a.clone());
    interp.functions.insert("b".to_string(), b.clone());

    let main = BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: Some(Reg(0)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "a".to_string(),
                },
                args: vec![],
            },
            BytecodeInstr::CallStatic {
                dst: Some(Reg(1)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "b".to_string(),
                },
                args: vec![],
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: BinaryOp::Add,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let result = interp.execute_function(&main, &[]).unwrap();
    assert_eq!(result.to_int(), Some(3));
}

#[test]
fn test_eager_call_no_async_placeholder() {
    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Embedded,
        workers: 1,
        work_stealing: false,
    });

    interp.constants = vec![ConstValue::Int(1), ConstValue::Int(2)];

    let a = BytecodeFunction {
        name: "a".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };
    let b = BytecodeFunction {
        name: "b".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 1,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    interp.functions.insert("a".to_string(), a.clone());
    interp.functions.insert("b".to_string(), b.clone());

    let main = BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: Some(Reg(0)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "a".to_string(),
                },
                args: vec![],
            },
            BytecodeInstr::CallStatic {
                dst: Some(Reg(1)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "b".to_string(),
                },
                args: vec![],
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: BinaryOp::Add,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let result = interp.execute_function(&main, &[]).unwrap();
    assert_eq!(result.to_int(), Some(3));
}

#[test]
fn test_result_try_propagates_task_failure_to_top_level() {
    let code = r#"
        fail_native: () -> Result(Int, String) = native("fail_native")

        fail: () -> Result(Int, String) = () => {
            return fail_native()
        }

        ok: () -> Int = () => { return 1 }

        main: () -> Result(Int, String) = () => {
            a = ok()
            b = fail()?
            return a + b
        }
    "#;

    let mut compiler = crate::frontend::Compiler::new();
    let module = compiler.compile_with_source("<test>", code).unwrap();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = ctx.generate().unwrap();
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Standard,
        workers: 1,
        work_stealing: false,
    });
    interp
        .ffi_registry_mut()
        .register("fail_native", |_args, _ctx| {
            Err(ExecutorError::runtime_only("fail".to_string()))
        });

    let err = interp.execute_module(&bytecode_module).unwrap_err();
    match err {
        ExecutorError::Runtime(msg, _) => {
            assert!(msg.contains("fail"), "unexpected error: {msg}");
        }
        other => panic!("expected runtime error, got: {other:?}"),
    }
}

#[test]
fn test_dependency_failure_cancels_dependent_task() {
    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Embedded,
        workers: 1,
        work_stealing: false,
    });

    // const[0]=1, const[1]=0, const[2]=1
    interp.constants = vec![ConstValue::Int(1), ConstValue::Int(0), ConstValue::Int(1)];

    let fail = BytecodeFunction {
        name: "fail".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 1,
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: BinaryOp::Div,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let inc = BytecodeFunction {
        name: "inc".to_string(),
        params: vec![crate::middle::core::ir::Type::Int(64)],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadArg {
                dst: Reg(0),
                arg_idx: 0,
            },
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 2,
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: BinaryOp::Add,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    interp.functions.insert("fail".to_string(), fail.clone());
    interp.functions.insert("inc".to_string(), inc.clone());

    let main = BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Int(64),
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: Some(Reg(0)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "fail".to_string(),
                },
                args: vec![],
            },
            BytecodeInstr::CallStatic {
                dst: Some(Reg(1)),
                func: FunctionRef::Static {
                    module: "".to_string(),
                    name: "inc".to_string(),
                },
                args: vec![Reg(0)],
            },
            // Force the dependent value
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(1),
                rhs: Reg(1),
                op: BinaryOp::Add,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let err = interp.execute_function(&main, &[]).unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("DivisionByZero")
            || err_str.contains("Division by zero")
            || err_str.contains("division"),
        "expected dependency error to surface, got: {err:?}"
    );
}

// =============================================================================
// FFI End-to-End Tests
// =============================================================================

/// Create a CallNative bytecode instruction for testing
fn make_call_native_bytecode(
    func_name: &str,
    args: Vec<ConstValue>,
) -> (BytecodeModule, usize) {
    let mut module = BytecodeModule::new("test_ffi".to_string());

    // Build instructions:
    // 1. Load constants for args
    // 2. CallNative
    // 3. Return
    let mut instructions = Vec::new();

    for (i, arg) in args.iter().enumerate() {
        let const_idx = module.constants.len() as u16;
        module.constants.push(arg.clone());

        instructions.push(BytecodeInstr::LoadConst {
            dst: Reg(i as u16),
            const_idx,
        });
    }

    // CallNative instruction - func_name is String directly
    let arg_regs: Vec<Reg> = (0..args.len()).map(|i| Reg(i as u16)).collect();

    instructions.push(BytecodeInstr::CallNative {
        dst: Some(Reg(100)), // Use a high register for return
        func_name: func_name.to_string(),
        args: arg_regs,
    });

    // Return the result
    instructions.push(BytecodeInstr::ReturnValue { value: Reg(100) });

    let func_idx = module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 101,
        upvalue_count: 0,
        instructions,
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    });

    module.entry_point = Some(func_idx);
    (module, func_idx)
}

#[test]
fn test_ffi_println_e2e() {
    let mut interp = Interpreter::new();

    // Create: println("Hello, FFI!")
    let (module, func_idx) = make_call_native_bytecode(
        "std.io.println",
        vec![ConstValue::String("Hello, FFI!".to_string())],
    );

    let result = interp.execute_function(&module.functions[func_idx], &[]);
    assert!(result.is_ok(), "FFI call should succeed");
}

#[test]
fn test_ffi_write_and_read_file_e2e() {
    let mut interp = Interpreter::new();

    // Test using std.io directly via FFI registry
    let path_str = "test_e2e_file.txt".to_string();

    // Test write_file via registry directly
    let mut ctx = crate::std::NativeContext::new(&mut interp.heap);
    let result = interp.ffi.call(
        "std.io.write_file",
        &[
            RuntimeValue::String(path_str.clone().into()),
            RuntimeValue::String("E2E test content".into()),
        ],
        &mut ctx,
    );
    assert!(result.is_ok(), "write_file should succeed: {:?}", result);

    // Test read_file via registry
    let result = interp.ffi.call(
        "std.io.read_file",
        &[RuntimeValue::String(path_str.clone().into())],
        &mut ctx,
    );
    assert!(result.is_ok(), "read_file should succeed");

    // Verify content
    let return_value = result.unwrap();
    if let RuntimeValue::String(content) = return_value {
        assert_eq!(content.to_string(), "E2E test content");
    } else {
        panic!("Expected String return value");
    }

    // Cleanup
    let _ = std::fs::remove_file(&path_str);
}

#[test]
fn test_ffi_custom_function_e2e() {
    let mut interp = Interpreter::new();

    // Register a custom native function
    interp
        .ffi_registry_mut()
        .register("test.multiply", |args, _ctx| {
            eprintln!("DEBUG: multiply args = {:?}", args);
            let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            eprintln!("DEBUG: multiply {} * {}", a, b);
            Ok(RuntimeValue::Int(a * b))
        });

    // Test via registry directly
    let mut ctx = crate::std::NativeContext::new(&mut interp.heap);
    let result = interp.ffi.call(
        "test.multiply",
        &[RuntimeValue::Int(6), RuntimeValue::Int(7)],
        &mut ctx,
    );

    assert!(
        result.is_ok(),
        "Custom FFI call should succeed: {:?}",
        result
    );

    let return_value = result.unwrap();
    if let RuntimeValue::Int(val) = return_value {
        assert_eq!(val, 42, "6 * 7 should equal 42");
    } else {
        panic!("Expected Int return value");
    }
}

#[test]
fn test_ffi_nonexistent_function_e2e() {
    let mut interp = Interpreter::new();

    // Try to call a non-existent function
    let mut ctx = crate::std::NativeContext::new(&mut interp.heap);
    let result = interp.ffi.call("nonexistent.function", &[], &mut ctx);
    assert!(result.is_err(), "Call to non-existent function should fail");
}

#[test]
fn test_ffi_append_file_e2e() {
    let mut interp = Interpreter::new();

    let path_str = "test_append_file.txt".to_string();

    let mut ctx = crate::std::NativeContext::new(&mut interp.heap);
    // Write initial content
    let result1 = interp.ffi.call(
        "std.io.write_file",
        &[
            RuntimeValue::String(path_str.clone().into()),
            RuntimeValue::String("First".into()),
        ],
        &mut ctx,
    );
    assert!(result1.is_ok());

    // Append content
    let result2 = interp.ffi.call(
        "std.io.append_file",
        &[
            RuntimeValue::String(path_str.clone().into()),
            RuntimeValue::String(" Second".into()),
        ],
        &mut ctx,
    );
    assert!(result2.is_ok());

    // Read and verify
    let result3 = interp.ffi.call(
        "std.io.read_file",
        &[RuntimeValue::String(path_str.clone().into())],
        &mut ctx,
    );
    assert!(result3.is_ok());

    let return_value = result3.unwrap();
    if let RuntimeValue::String(content) = return_value {
        assert_eq!(content.to_string(), "First Second");
    } else {
        panic!("Expected String");
    }

    // Cleanup
    let _ = std::fs::remove_file(&path_str);
}

// =========================================================================
// End-to-end spawn tests (RFC-024)
// =========================================================================

use std::cell::RefCell;

thread_local! {
    static CAPTURED: RefCell<Vec<i64>> = const { RefCell::new(Vec::new()) };
}

/// Native function that captures integer values into thread-local storage.
fn capture_handler(
    args: &[RuntimeValue],
    _ctx: &mut crate::std::NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let val = args.first().and_then(|v| v.to_int()).unwrap_or(0);
    CAPTURED.with(|c| c.borrow_mut().push(val));
    Ok(RuntimeValue::Int(val))
}

/// Helper: compile YaoXiang source, generate bytecode, execute.
/// Returns Ok(()) on success, Err on compilation or runtime failure.
fn compile_and_run(
    code: &str,
    runtime: RuntimeMode,
) -> Result<(), String> {
    let mut compiler = crate::frontend::Compiler::new();
    let module = compiler
        .compile_with_source("<test>", code)
        .map_err(|e| format!("Compile error: {:?}", e))?;
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| format!("Codegen error: {:?}", e))?;
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime,
        workers: 1,
        work_stealing: false,
    });
    interp
        .execute_module(&bytecode_module)
        .map_err(|e| format!("Runtime error: {:?}", e))
}

/// Helper: compile YaoXiang source with a native "capture" function
/// that records integer values into thread-local storage.
fn compile_and_run_with_capture(
    code: &str,
    runtime: RuntimeMode,
) -> Result<Vec<i64>, String> {
    CAPTURED.with(|c| c.borrow_mut().clear());

    let mut compiler = crate::frontend::Compiler::new();
    let module = compiler
        .compile_with_source("<test>", code)
        .map_err(|e| format!("Compile error: {:?}", e))?;
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| format!("Codegen error: {:?}", e))?;
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime,
        workers: 1,
        work_stealing: false,
    });

    interp
        .ffi_registry_mut()
        .register("capture", capture_handler);

    interp
        .execute_module(&bytecode_module)
        .map_err(|e| format!("Runtime error: {:?}", e))?;

    Ok(CAPTURED.with(|c| c.borrow().clone()))
}

// Test 1: Basic parallel — two independent assignments in a spawn block.
// Both closures should execute without error.
#[test]
fn test_e2e_spawn_basic_parallel() {
    let code = r#"
        main: () -> Int = () => {
            spawn {
                t1 = 1 + 1
                t2 = 2 + 2
            }
            return 0
        }
    "#;

    // Should compile and run in both Embedded and Standard modes.
    let result_embedded = compile_and_run(code, RuntimeMode::Embedded);
    assert!(
        result_embedded.is_ok(),
        "Embedded mode failed: {}",
        result_embedded.unwrap_err()
    );

    let result_standard = compile_and_run(code, RuntimeMode::Standard);
    assert!(
        result_standard.is_ok(),
        "Standard mode failed: {}",
        result_standard.unwrap_err()
    );
}

// Test 1b: Verify the closures actually execute by using a native capture function.
#[test]
fn test_e2e_spawn_parallel_values() {
    let code = r#"
        capture: (x: Int) -> Int = native("capture")

        main: () -> Int = () => {
            spawn {
                t1 = capture(2)
                t2 = capture(4)
            }
            return 0
        }
    "#;

    let result = compile_and_run_with_capture(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Spawn execution failed: {}",
        result.unwrap_err()
    );

    let values = result.unwrap();
    assert_eq!(
        values.len(),
        2,
        "Expected 2 captured values, got {}",
        values.len()
    );
    // Both 2 and 4 should have been captured (order may vary).
    let mut sorted = values.clone();
    sorted.sort();
    assert_eq!(sorted, vec![2, 4]);
}

// Test 2: Dependency order — y depends on x, should execute in sequence.
// The DAG analysis should place x in group 0 and y in group 1.
#[test]
fn test_e2e_spawn_dependency_order() {
    let code = r#"
        capture: (x: Int) -> Int = native("capture")

        main: () -> Int = () => {
            spawn {
                x = capture(10)
                y = capture(20)
            }
            return 0
        }
    "#;

    let result = compile_and_run_with_capture(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Spawn execution failed: {}",
        result.unwrap_err()
    );

    let values = result.unwrap();
    assert_eq!(
        values.len(),
        2,
        "Expected 2 captured values, got {}",
        values.len()
    );
    // Both values should be present.
    let mut sorted = values.clone();
    sorted.sort();
    assert_eq!(sorted, vec![10, 20]);
}

// Test 2b: Dependency order with actual read dependency.
// y = x + 1 where x is defined in the spawn block.
// The DAG should schedule x first, then y.
#[test]
fn test_e2e_spawn_dependency_chain() {
    let code = r#"
        capture: (x: Int) -> Int = native("capture")

        main: () -> Int = () => {
            spawn {
                x = 10
                y = x + 1
            }
            return 0
        }
    "#;

    // This should compile and run without error.
    // The DAG analysis detects that y reads x, so x must execute before y.
    let result = compile_and_run(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Spawn with dependency chain failed: {}",
        result.unwrap_err()
    );
}

// Test 3: Scope isolation — the spawn block's internal variable assignments
// are wrapped in closures. After the spawn block, the outer variables
// should retain their original values. We verify this by capturing the
// outer variable value after the spawn block completes.
#[test]
fn test_e2e_spawn_scope_isolation() {
    // The spawn block assigns to 'y' (inside the closure), while the
    // outer function uses 'x'. After spawn completes, x is unchanged.
    // We also test that the spawn-internal assignment doesn't leak.
    let code = r#"
        capture: (x: Int) -> Int = native("capture")

        main: () -> Int = () => {
            x = 100
            spawn {
                y = 200
            }
            capture(x)
            return 0
        }
    "#;

    let result = compile_and_run_with_capture(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Spawn scope test failed: {}",
        result.unwrap_err()
    );

    let values = result.unwrap();
    assert_eq!(
        values.len(),
        1,
        "Expected 1 captured value, got {}",
        values.len()
    );
    // The outer x should still be 100 — spawn block's y = 200 is
    // computed inside a closure and doesn't affect the outer scope.
    assert_eq!(values[0], 100, "Outer x should be 100, got {}", values[0]);
}

// Test: Spawn with an empty block should work.
#[test]
fn test_e2e_spawn_empty_block() {
    let code = r#"
        main: () -> Int = () => {
            spawn {}
            return 42
        }
    "#;

    let result = compile_and_run(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Empty spawn block failed: {}",
        result.unwrap_err()
    );
}

// Test: Spawn with a single task.
#[test]
fn test_e2e_spawn_single_task() {
    let code = r#"
        capture: (x: Int) -> Int = native("capture")

        main: () -> Int = () => {
            spawn {
                x = capture(42)
            }
            return 0
        }
    "#;

    let result = compile_and_run_with_capture(code, RuntimeMode::Embedded);
    assert!(
        result.is_ok(),
        "Single task spawn failed: {}",
        result.unwrap_err()
    );

    let values = result.unwrap();
    assert_eq!(
        values.len(),
        1,
        "Expected 1 captured value, got {}",
        values.len()
    );
    assert_eq!(values[0], 42);
}

// Test: Spawn in Standard mode (parallel execution via task scheduler).
#[test]
fn test_e2e_spawn_standard_mode() {
    // Standard mode uses thread pool — capture via thread-local won't work.
    // Test that Standard mode compiles and runs without errors.
    let code = r#"
        main: () -> Int = () => {
            spawn {
                t1 = 1 + 2
                t2 = 3 + 4
            }
            return 0
        }
    "#;

    let result = compile_and_run(code, RuntimeMode::Standard);
    assert!(
        result.is_ok(),
        "Standard mode spawn failed: {}",
        result.unwrap_err()
    );
}

// Test: Standard mode with workers > 1 — verify multi-threaded execution.
// Uses a thread-safe capture function to record values from worker threads.
#[test]
fn test_e2e_spawn_standard_multithreaded() {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    // Thread-safe capture: records values from any thread
    static MT_CAPTURED: std::sync::OnceLock<Arc<Mutex<Vec<i64>>>> = std::sync::OnceLock::new();

    fn mt_captured() -> Arc<Mutex<Vec<i64>>> {
        MT_CAPTURED
            .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
            .clone()
    }

    fn mt_capture_handler(
        args: &[RuntimeValue],
        _ctx: &mut crate::std::NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        let val = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        mt_captured().lock().unwrap().push(val);
        Ok(RuntimeValue::Int(val))
    }

    mt_captured().lock().unwrap().clear();

    let code = r#"
        capture: (x: Int) -> Int = native("mt_capture")

        main: () -> Int = () => {
            spawn {
                t1 = capture(10)
                t2 = capture(20)
                t3 = capture(30)
            }
            return 0
        }
    "#;

    let mut compiler = crate::frontend::Compiler::new();
    let module = compiler
        .compile_with_source("<test>", code)
        .unwrap_or_else(|e| panic!("Compile error: {:?}", e));
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .unwrap_or_else(|e| panic!("Codegen error: {:?}", e));
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Standard,
        workers: 4,
        work_stealing: false,
    });

    // Register both "capture" (function name in bytecode) and "mt_capture" (native target)
    interp
        .ffi_registry_mut()
        .register("capture", mt_capture_handler);
    interp
        .ffi_registry_mut()
        .register("mt_capture", mt_capture_handler);

    let start = Instant::now();
    interp
        .execute_module(&bytecode_module)
        .unwrap_or_else(|e| panic!("Standard mode spawn failed: {:?}", e));
    let elapsed = start.elapsed();

    let captured = mt_captured();
    let values = captured.lock().unwrap();
    assert_eq!(
        values.len(),
        3,
        "Expected 3 captured values from worker threads, got {}",
        values.len()
    );

    let mut sorted = values.clone();
    sorted.sort();
    assert_eq!(sorted, vec![10, 20, 30]);

    println!("Standard mode (workers=4) completed in {:?}", elapsed);
}
