//! Float 比较运算回归测试 — 基于语言规范 §2.2
//!
//! §2.2 运算符优先级表第 8 级：`==` `!=` `<` `>` `<=` `>=`
//! RFC-011: Float 类型属于内建类型，必须支持全部比较运算符
//!
//! 回归背景：issue #226 的真实根因。`Interpreter::exec_compare` 曾缺少
//! `RuntimeValue::Float` 的所有 6 个比较分支，导致 Float 比较落到
//! `_ => RuntimeValue::Bool(false)` 通配分支，所有 Float 比较恒返回 false。
//! Int 和 String 的比较正常，只有 Float 是坏的。
//!
//! 本测试直接驱动 `BytecodeInstr::Compare` 指令，覆盖 Float 的全部 6 个
//! CompareOp，确保每个分支都返回正确的 Bool 值。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::Executor;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, CompareOp, ConstValue, Reg};
use crate::middle::core::ir::Type;
use std::collections::HashMap;

/// 构造一个函数：r0 = const[lhs], r1 = const[rhs], r2 = r0 <op> r1, return r2
fn make_compare_function(op: CompareOp) -> BytecodeFunction {
    BytecodeFunction {
        name: "test_float_compare".to_string(),
        params: vec![],
        return_type: Type::Bool,
        local_count: 3,
        upvalue_count: 0,
        instructions: vec![
            // r0 = const[lhs]
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // r1 = const[rhs]
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 1,
            },
            // r2 = r0 <op> r1
            BytecodeInstr::Compare {
                cmp: op,
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 跑一次 Float 比较，返回 Bool 值
fn run_float_compare(
    op: CompareOp,
    lhs: f64,
    rhs: f64,
) -> bool {
    // Arrange
    let func = make_compare_function(op);
    let mut interp = Interpreter::new();
    interp.constants.push(ConstValue::Float(lhs));
    interp.constants.push(ConstValue::Float(rhs));

    // Act
    let result = interp
        .execute_function(&func, &[])
        .expect("execute_function should not fail for Float comparison");

    // Assert: 返回值必须是 Bool
    match result {
        RuntimeValue::Bool(b) => b,
        other => panic!("Float comparison should return Bool, got {:?}", other),
    }
}

// Eq (==) — 规范 §2.2 第 8 级

#[test]
fn test_float_eq_returns_true_when_values_equal() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Eq, 1.0, 1.0);

    // Assert
    assert!(result, "1.0 == 1.0 should be true");
}

#[test]
fn test_float_eq_returns_false_when_values_differ() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Eq, 1.0, 2.0);

    // Assert
    assert!(!result, "1.0 == 2.0 should be false");
}

// Ne (!=) — 规范 §2.2 第 8 级

#[test]
fn test_float_ne_returns_true_when_values_differ() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Ne, 1.0, 2.0);

    // Assert
    assert!(result, "1.0 != 2.0 should be true");
}

#[test]
fn test_float_ne_returns_false_when_values_equal() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Ne, 1.0, 1.0);

    // Assert
    assert!(!result, "1.0 != 1.0 should be false");
}

// Lt (<) — 规范 §2.2 第 8 级

#[test]
fn test_float_lt_returns_true_when_lhs_smaller() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Lt, 1.0, 2.0);

    // Assert
    assert!(result, "1.0 < 2.0 should be true");
}

#[test]
fn test_float_lt_returns_false_when_lhs_greater() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Lt, 2.0, 1.0);

    // Assert
    assert!(!result, "2.0 < 1.0 should be false");
}

#[test]
fn test_float_lt_returns_false_when_values_equal() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Lt, 1.0, 1.0);

    // Assert
    assert!(!result, "1.0 < 1.0 should be false");
}

// Le (<=) — 规范 §2.2 第 8 级

#[test]
fn test_float_le_returns_true_when_lhs_smaller() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Le, 1.0, 2.0);

    // Assert
    assert!(result, "1.0 <= 2.0 should be true");
}

#[test]
fn test_float_le_returns_true_when_values_equal() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Le, 1.0, 1.0);

    // Assert
    assert!(result, "1.0 <= 1.0 should be true");
}

// Gt (>) — 规范 §2.2 第 8 级

#[test]
fn test_float_gt_returns_true_when_lhs_greater() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Gt, 2.0, 1.0);

    // Assert
    assert!(result, "2.0 > 1.0 should be true");
}

#[test]
fn test_float_gt_returns_false_when_lhs_smaller() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Gt, 1.0, 2.0);

    // Assert
    assert!(!result, "1.0 > 2.0 should be false");
}

// Ge (>=) — 规范 §2.2 第 8 级

#[test]
fn test_float_ge_returns_true_when_values_equal() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Ge, 1.0, 1.0);

    // Assert
    assert!(result, "1.0 >= 1.0 should be true");
}

#[test]
fn test_float_ge_returns_true_when_lhs_greater() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Ge, 2.0, 1.0);

    // Assert
    assert!(result, "2.0 >= 1.0 should be true");
}

// 边界：负数、零、小数（规范未特殊处理，应与正数一致）

#[test]
fn test_float_eq_zero_returns_true() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Eq, 0.0, 0.0);

    // Assert
    assert!(result, "0.0 == 0.0 should be true");
}

#[test]
fn test_float_lt_negative_returns_true() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Lt, -1.0, 0.0);

    // Assert
    assert!(result, "-1.0 < 0.0 should be true");
}

#[test]
fn test_float_ne_negative_returns_true() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Ne, -1.0, 1.0);

    // Assert
    assert!(result, "-1.0 != 1.0 should be true");
}

#[test]
fn test_float_gt_decimal_returns_true() {
    // Arrange
    // Act
    let result = run_float_compare(CompareOp::Gt, 3.5, 3.0);

    // Assert
    assert!(result, "3.5 > 3.0 should be true");
}
