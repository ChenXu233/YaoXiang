//! 任务错误码保真测试 — WBS 3.11.1 / issue #448（src/backends/runtime 通道变更的解释器侧判据）
//!
//! 规范锚点：
//! - `docs/src/dev/architecture/09-execution-wbs.md` §3.11.1：Standard 运行时下
//!   任务内部错误必须**按值**跨任务边界（结构化错误），码身份保真；
//! - 修复前：`execute_scheduled_task_from_data` 先 `format!("{e}")` 成字符串，
//!   接收方统一 `ExecutorError::runtime(...)` ⇒ 一律落 E6007（码丢失）。
//! - 错误码定义见 `src/util/diagnostic/codes/e6xxx.rs`。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::runtime::{RuntimeConfig, RuntimeMode};
use crate::backends::{Executor, ExecutorConfig, ExecutorError};
use crate::middle::bytecode::{BinaryOp, BytecodeFunction, BytecodeInstr, ConstValue, Reg};
use std::collections::HashMap;

/// 构造 `boom()`：`return 1 / 0`（E6001 触发点）。
fn divide_by_zero_function() -> BytecodeFunction {
    BytecodeFunction {
        name: "boom".to_string(),
        params: vec![],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        local_count: 4,
        local_names: HashMap::new(),
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
    }
}

/// 构造 `main()`：调用 `boom()`（Standard 模式下经任务边界）。
fn call_boom_function(boom_idx: u32) -> BytecodeFunction {
    BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        local_count: 2,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::CallStatic {
                dst: Some(Reg(0)),
                func: boom_idx,
                args: vec![],
            },
            BytecodeInstr::Return,
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// `count_down(n) = if n == 0 { 0 } else { 1 + count_down(n - 1) }`（E6004 触发点，非尾）。
fn countdown_function() -> BytecodeFunction {
    BytecodeFunction {
        name: "count_down".to_string(),
        params: vec![],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        local_count: 8,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadArg {
                dst: Reg(0),
                arg_idx: 0,
            },
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 0,
            },
            BytecodeInstr::Compare {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                cmp: crate::middle::bytecode::CompareOp::Eq,
            },
            BytecodeInstr::JmpIfNot {
                cond: Reg(2),
                target: crate::middle::bytecode::Label(3),
            },
            BytecodeInstr::LoadConst {
                dst: Reg(3),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(3) },
            BytecodeInstr::LoadConst {
                dst: Reg(4),
                const_idx: 1,
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(5),
                lhs: Reg(0),
                rhs: Reg(4),
                op: BinaryOp::Sub,
            },
            BytecodeInstr::CallStatic {
                dst: Some(Reg(6)),
                func: 0,
                args: vec![Reg(5)],
            },
            BytecodeInstr::BinaryOp {
                dst: Reg(7),
                lhs: Reg(4),
                rhs: Reg(6),
                op: BinaryOp::Add,
            },
            BytecodeInstr::ReturnValue { value: Reg(7) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// Standard 模式（1 worker）+ 给定上限的解释器。
fn standard_interpreter(max_stack_depth: usize) -> Interpreter {
    let mut interp = Interpreter::with_config(ExecutorConfig { max_stack_depth });
    interp.set_runtime_config(RuntimeConfig {
        mode: RuntimeMode::Standard,
        workers: 1,
    });
    interp.reset();
    interp
}

#[test]
fn test_standard_task_error_keeps_division_by_zero_code() {
    // Arrange — Standard 模式下 main 调用 boom()（调用经任务边界）
    let mut interp = standard_interpreter(1024);
    let image = std::sync::Arc::get_mut(&mut interp.image).expect("image 独占");
    image.constants.push(ConstValue::Int(1));
    image.constants.push(ConstValue::Int(0));
    image.functions_by_id.push(divide_by_zero_function());
    image.functions_by_id.push(call_boom_function(0));
    let main = interp.image.functions_by_id[1].clone();

    // Act — 任务内的除零错误经任务边界返回
    let err = interp
        .execute_function(&main, &[])
        .expect_err("除零必须报错");

    // Assert — 码身份保真：E6001 变体（而非 E6007 的 Runtime 包装）
    assert!(
        matches!(&err, ExecutorError::DivisionByZero(expr, _) if expr == "1 / 0"),
        "任务内除零必须按值保真为 DivisionByZero（E6001），实际 {err:?}"
    );
}

#[test]
fn test_standard_task_error_keeps_stack_overflow_limit() {
    // Arrange — 任务内的解释器按默认上限 1024 建（`Interpreter::for_task`）；
    // 入口递归 2000 层 ⇒ 在任务内触发 E6004（Embedded 蹦床，不耗原生栈）。
    let mut interp = standard_interpreter(1024);
    let image = std::sync::Arc::get_mut(&mut interp.image).expect("image 独占");
    image.constants.push(ConstValue::Int(0));
    image.constants.push(ConstValue::Int(1));
    image.functions_by_id.push(countdown_function());
    let func = interp.image.functions_by_id[0].clone();

    // Act
    let err = interp
        .execute_function(&func, &[RuntimeValue::Int(2000)])
        .expect_err("深度超限必须报 E6004");

    // Assert — 结构化错误跨任务边界后 limit 仍是真实上限（不是 0、不是 E6007）
    assert!(
        matches!(&err, ExecutorError::StackOverflow { limit: 1024, .. }),
        "任务内栈超限必须保真为 StackOverflow{{limit: 1024}}，实际 {err:?}"
    );
}
