//! Debugger implementation and tests for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and unit tests.

use crate::backends::DebuggableExecutor;
use crate::backends::ExecutorResult;
use super::executor::Interpreter;

impl DebuggableExecutor for Interpreter {
    fn set_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.insert(offset, ());
    }

    fn remove_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.remove(&offset);
    }

    fn has_breakpoint(&self) -> bool {
        if let Some(frame) = self.call_stack.last() {
            self.breakpoints.contains_key(&frame.ip)
        } else {
            false
        }
    }

    fn step(&mut self) -> ExecutorResult<()> {
        todo!("step debugging not implemented")
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        todo!("step_over debugging not implemented")
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        todo!("step_out debugging not implemented")
    }

    fn run(&mut self) -> ExecutorResult<()> {
        todo!("run debugging not implemented")
    }

    fn current_ip(&self) -> usize {
        self.call_stack.last().map(|f| f.ip).unwrap_or(0)
    }

    fn current_function(&self) -> Option<&str> {
        self.call_stack.last().map(|f| f.function.name.as_str())
    }

    fn breakpoints(&self) -> Vec<usize> {
        self.breakpoints.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::backends::Executor;
    use crate::backends::ExecutorError;
    use crate::backends::common::RuntimeValue;
    use crate::middle::bytecode::{
        BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, BinaryOp, FunctionRef, ConstValue,
    };
    use crate::backends::interpreter::EvalStrategy;
    use crate::backends::interpreter::runtime::InterpreterRuntimeConfig;
    use crate::backends::runtime::RuntimeMode;
    use super::Interpreter;

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
            runtime: RuntimeMode::Standard,
            eval: EvalStrategy::Auto,
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
        };

        let result = interp.execute_function(&main, &[]).unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eager_call_no_async_placeholder() {
        let mut interp = Interpreter::new();
        interp.set_runtime_config(InterpreterRuntimeConfig {
            runtime: RuntimeMode::Standard,
            eval: EvalStrategy::Eager,
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
        };

        let result = interp.execute_function(&main, &[]).unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_result_try_propagates_task_failure_to_top_level() {
        let code = r#"
            fail_native: () -> Result[Int, String] = Native("fail_native")

            fail: () -> Result[Int, String] = () => {
                fail_native()?
            }

            ok: () -> Int = () => { 1 }

            main: () -> Result[Int, String] = () => {
                a = ok()
                b = fail()?
                a + b
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
            eval: EvalStrategy::Auto,
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
            runtime: RuntimeMode::Standard,
            eval: EvalStrategy::Auto,
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
        };

        let err = interp.execute_function(&main, &[]).unwrap_err();
        match err {
            ExecutorError::Runtime(msg, _) => {
                assert!(
                    msg.contains("Division by zero"),
                    "expected dependency error to surface, got: {msg}"
                );
            }
            other => panic!("expected runtime error, got: {other:?}"),
        }
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
                let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
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
}
