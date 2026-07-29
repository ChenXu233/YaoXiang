//! 一元运算符回归测试 — 基于语言规范 §2.2
//!
//! §2.2 运算符优先级表第 9 级：`not`（逻辑非），右到左结合
//!
//! 回归背景：issue #229。`ir_gen.rs` 中 `Expr::UnOp { op: Not }` 分支
//! 未生成 `UnaryOp::Not` 指令，而是直接加载常量 `Int(0)`，导致 `!true` 和
//! `!false` 都恒返回 `0`（整数零）。
//!
//! 本测试直接驱动 `BytecodeInstr::UnaryOp { op: Not }` 指令，确保
//! `!true` 返回 `false`、`!false` 返回 `true`。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::Executor;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, ConstValue, Reg, UnaryOp};
use crate::middle::core::ir::Type;
use std::collections::HashMap;

/// 构造一个函数：r0 = const[val], r1 = !r0, return r1
fn make_not_function() -> BytecodeFunction {
    BytecodeFunction {
        name: "test_logical_not".to_string(),
        params: vec![],
        return_type: Type::Bool,
        local_count: 2,
        upvalue_count: 0,
        instructions: vec![
            // r0 = const[val]
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // r1 = !r0
            BytecodeInstr::UnaryOp {
                dst: Reg(1),
                src: Reg(0),
                op: UnaryOp::Not,
            },
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 跑一次逻辑非，返回 Bool 值
fn run_logical_not(val: bool) -> bool {
    // Arrange
    let func = make_not_function();
    let mut interp = Interpreter::new();
    interp.constants.push(ConstValue::Bool(val));

    // Act
    let result = interp
        .execute_function(&func, &[])
        .expect("execute_function should not fail for UnaryOp::Not");

    // Assert: 返回值必须是 Bool
    match result {
        RuntimeValue::Bool(b) => b,
        other => panic!("UnaryOp::Not should return Bool, got {:?}", other),
    }
}

// ============================================================================
// Not (!) — 规范 §2.2 第 9 级
// ============================================================================

#[test]
fn test_not_true_returns_false() {
    // Arrange
    // Act
    let result = run_logical_not(true);

    // Assert
    assert!(!result, "!true should be false");
}

#[test]
fn test_not_false_returns_true() {
    // Arrange
    // Act
    let result = run_logical_not(false);

    // Assert
    assert!(result, "!false should be true");
}
