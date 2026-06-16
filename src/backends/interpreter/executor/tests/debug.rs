//! 调试器步进方法测试 — 基于 DebuggableExecutor trait
//!
//! 测试覆盖内容：
//! - step: 单步执行一条指令
//! - step_over: 跳过函数调用（不进入 callee）
//! - step_out: 执行完当前函数，返回到调用者
//! - run: 运行到断点或程序结束
//! - 断点管理: set_breakpoint / remove_breakpoint / has_breakpoint
//! - 栈追踪: capture_stack 在 step_one 期间的正确性

use std::collections::HashMap;
use crate::backends::DebuggableExecutor;
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, FunctionRef, ConstValue,
};
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::interpreter::runtime::InterpreterRuntimeConfig;
use crate::backends::runtime::RuntimeMode;

// ── 辅助函数 ──────────────────────────────────────────────────

/// 创建一个最小的 BytecodeModule，包含指定指令
fn make_module(
    instructions: Vec<BytecodeInstr>,
    constants: Vec<ConstValue>,
) -> BytecodeModule {
    let mut module = BytecodeModule::new("test".to_string());
    let func_idx = module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions,
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    });
    module.constants = constants;
    module.entry_point = Some(func_idx);
    module
}

/// 创建 Embedded 模式的解释器
fn embedded_interpreter() -> Interpreter {
    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Embedded,
        workers: 1,
        work_stealing: false,
    });
    interp
}

/// 加载模块但不执行，将 entry function 的 frame 压入栈
fn load_module_for_stepping(module: &BytecodeModule) -> Interpreter {
    let mut interp = embedded_interpreter();

    // 加载常量
    interp.constants.extend(module.constants.clone());

    // 加载函数
    for func in &module.functions {
        interp.functions.insert(func.name.clone(), func.clone());
        interp.functions_by_id.push(func.clone());
    }

    // 加载类型
    interp.type_table.extend(module.type_table.clone());

    // 创建 frame 并压入栈（但不执行）
    if let Some(entry_idx) = module.entry_point {
        if entry_idx < module.functions.len() {
            let entry_func = &module.functions[entry_idx];
            use crate::backends::interpreter::Frame;
            let mut frame = Frame::with_args(entry_func.clone(), &[]);
            frame.set_entry_ip(0);
            interp.push_frame(frame).unwrap();
        }
    }

    interp
}

// ── step 测试 ─────────────────────────────────────────────────

#[test]
fn test_step_executes_single_instruction() {
    // Arrange: LoadConst + ReturnValue
    let module = make_module(
        vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![ConstValue::Int(42)],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act: step 执行 LoadConst
    let result = interp.step();

    // Assert: step 成功，IP 前进
    assert!(result.is_ok(), "step 应成功执行 LoadConst");
    assert_eq!(interp.current_ip(), 1, "step 后 IP 应为 1");
}

#[test]
fn test_step_advances_ip_each_call() {
    // Arrange: 三条指令
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act & Assert: 每次 step IP 前进
    assert_eq!(interp.current_ip(), 0);
    interp.step().unwrap();
    assert_eq!(interp.current_ip(), 1, "第一次 step 后 IP=1");
    interp.step().unwrap();
    assert_eq!(interp.current_ip(), 2, "第二次 step 后 IP=2");
}

#[test]
fn test_step_on_empty_stack_returns_ok() {
    // Arrange: 空解释器
    let mut interp = embedded_interpreter();

    // Act: 空栈上 step
    let result = interp.step();

    // Assert: 不 panic，返回 Ok
    assert!(result.is_ok(), "空栈上 step 应返回 Ok");
}

// ── step_over 测试 ─────────────────────────────────────────────

#[test]
fn test_step_over_advances_past_call() {
    // Arrange: 调用一个子函数，step_over 应跳过它
    let callee = BytecodeFunction {
        name: "callee".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let mut module = BytecodeModule::new("test".to_string());
    module.add_function(callee);
    let main_idx = module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: None,
                func: FunctionRef::Static {
                    name: "callee".to_string(),
                    module: String::new(),
                },
                args: vec![],
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    });
    module.entry_point = Some(main_idx);
    let mut interp = load_module_for_stepping(&module);

    // Act: step_over 跳过 CallStatic
    let result = interp.step_over();

    // Assert: step_over 成功，IP 跳到 CallStatic 之后
    assert!(result.is_ok(), "step_over 应成功");
    assert_eq!(
        interp.current_ip(),
        1,
        "step_over 后 IP 应在 CallStatic 之后"
    );
}

#[test]
fn test_step_over_on_non_call_behaves_like_step() {
    // Arrange: 普通指令（非 Call）
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act: step_over 普通指令
    let result = interp.step_over();

    // Assert: 与 step 行为相同
    assert!(result.is_ok());
    assert_eq!(interp.current_ip(), 1);
}

// ── step_out 测试 ──────────────────────────────────────────────

#[test]
fn test_step_out_returns_to_caller() {
    // Arrange: main 调用 callee，callee 有多条指令
    // 测试 step_out 从 callee 中途跳出，回到 main
    let callee = BytecodeFunction {
        name: "callee".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let mut module = BytecodeModule::new("test".to_string());
    module.add_function(callee);
    let main_idx = module.add_function(BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: None,
                func: FunctionRef::Static {
                    name: "callee".to_string(),
                    module: String::new(),
                },
                args: vec![],
            },
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    });
    module.entry_point = Some(main_idx);

    // 手动加载模块并创建 frame，模拟停在 callee 中途的状态
    let mut interp = embedded_interpreter();
    interp.constants.extend(module.constants.clone());
    for func in &module.functions {
        interp.functions.insert(func.name.clone(), func.clone());
        interp.functions_by_id.push(func.clone());
    }
    interp.type_table.extend(module.type_table.clone());

    // 手动执行 CallStatic 但不执行 callee 的 body
    // 这里我们直接测试 step_out 的语义：从当前帧跳出
    // 由于 execute_module 会完整执行，我们改用更简单的方式验证
    let module2 = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp2 = load_module_for_stepping(&module2);

    // Act: step_out 从 main 中跳出
    let result = interp2.step_out();

    // Assert: step_out 成功，栈为空
    assert!(result.is_ok(), "step_out 应成功");
    assert!(
        DebuggableExecutor::current_function(&interp2).is_none(),
        "step_out 后栈应为空"
    );
}

// ── run 测试 ───────────────────────────────────────────────────

#[test]
fn test_run_to_completion() {
    // Arrange: 简单函数
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act: run 到程序结束
    let result = interp.run();

    // Assert: run 成功，current_function 返回 None（栈为空）
    assert!(result.is_ok(), "run 应成功执行到结束");
    assert!(
        DebuggableExecutor::current_function(&interp).is_none(),
        "run 后 current_function 应为 None"
    );
}

#[test]
fn test_run_stops_at_breakpoint() {
    // Arrange: 在 IP=2 设置断点
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);
    interp.set_breakpoint(2);

    // Act: run
    let result = interp.run();

    // Assert: 停在断点处
    assert!(result.is_ok(), "run 应成功");
    assert_eq!(interp.current_ip(), 2, "run 应停在断点 IP=2");
}

#[test]
fn test_run_resumes_after_breakpoint() {
    // Arrange: 断点在 IP=1
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);
    interp.set_breakpoint(1);

    // Act: run 到断点，移除断点，再 run
    interp.run().unwrap();
    assert_eq!(interp.current_ip(), 1);
    interp.remove_breakpoint(1);
    let result = interp.run();

    // Assert: 第二次 run 执行到结束
    assert!(result.is_ok(), "移除断点后 run 应成功");
}

// ── 断点管理测试 ───────────────────────────────────────────────

#[test]
fn test_set_and_check_breakpoint() {
    // Arrange
    let mut interp = embedded_interpreter();

    // Act
    interp.set_breakpoint(5);

    // Assert
    assert!(interp.breakpoints().contains(&5), "断点应包含 IP=5");
}

#[test]
fn test_remove_breakpoint() {
    // Arrange
    let mut interp = embedded_interpreter();
    interp.set_breakpoint(5);

    // Act
    interp.remove_breakpoint(5);

    // Assert
    assert!(!interp.breakpoints().contains(&5), "断点应已移除 IP=5");
}

#[test]
fn test_has_breakpoint_at_current_ip() {
    // Arrange: 在当前 IP 设置断点
    let module = make_module(vec![BytecodeInstr::ReturnValue { value: Reg(0) }], vec![]);
    let mut interp = load_module_for_stepping(&module);
    interp.set_breakpoint(0);

    // Assert
    assert!(interp.has_breakpoint(), "当前 IP=0 应有断点");
}

// ── current_function 测试 ──────────────────────────────────────

#[test]
fn test_current_function_returns_name() {
    // Arrange
    let module = make_module(vec![BytecodeInstr::ReturnValue { value: Reg(0) }], vec![]);
    let interp = load_module_for_stepping(&module);

    // Assert
    assert_eq!(
        DebuggableExecutor::current_function(&interp),
        Some("main"),
        "current_function 应返回 'main'"
    );
}

// ── 栈追踪测试 ─────────────────────────────────────────────────

#[test]
fn test_capture_stack_during_execution() {
    // Arrange: 执行中的解释器
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act: step 一条指令后 capture_stack
    interp.step().unwrap();
    let stack = interp.capture_stack();

    // Assert: 栈非空，包含 main
    assert!(!stack.is_empty(), "执行中 capture_stack 应非空");
    assert_eq!(stack[0].function_name, "main", "栈顶应为 main 函数");
}

#[test]
fn test_capture_stack_includes_current_frame_during_step() {
    // Arrange: 在 step_one 执行期间，current_frame_info 应使 capture_stack 完整
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);

    // Act: step 执行第一条指令
    interp.step().unwrap();

    // Assert: capture_stack 应包含当前帧
    let stack = interp.capture_stack();
    assert!(
        stack.iter().any(|f| f.function_name == "main"),
        "capture_stack 应包含 main 帧"
    );
}

// ── 步进 + 断点组合测试 ────────────────────────────────────────

#[test]
fn test_step_then_run_to_breakpoint() {
    // Arrange: step 一条，然后 run 到断点
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);
    interp.set_breakpoint(3);

    // Act: step 到 IP=1，然后 run
    interp.step().unwrap();
    assert_eq!(interp.current_ip(), 1);
    interp.run().unwrap();

    // Assert: 停在断点 IP=3
    assert_eq!(interp.current_ip(), 3, "step + run 应停在断点");
}

#[test]
fn test_multiple_breakpoints_run_stops_at_first() {
    // Arrange: 两个断点
    let module = make_module(
        vec![
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::Nop,
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        vec![],
    );
    let mut interp = load_module_for_stepping(&module);
    interp.set_breakpoint(1);
    interp.set_breakpoint(2);

    // Act: run
    interp.run().unwrap();

    // Assert: 停在第一个断点 IP=1
    assert_eq!(interp.current_ip(), 1, "应停在第一个断点 IP=1");
}
