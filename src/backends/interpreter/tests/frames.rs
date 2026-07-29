//! 调用栈帧测试 — 基于 VM 数据结构规范
//!
//! 测试覆盖内容：
//! - Frame 的创建和初始化（slots 数组分配）
//! - Frame 的 slots 统一读写（locals + registers 合并后的 API）
//!
//! 规范来源：
//! - `docs/superpowers/specs/2026-07-25-while-assign-not-persisted-design.md` §3
//!   Frame 合并 registers/locals 为 slots，消除 Operand::Local 二义性
//! - issue #223：while 循环内 mut 变量赋值不写回的根因是 registers/locals 双数组

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::frames::Frame;
use crate::middle::bytecode::BytecodeFunction;
use std::collections::HashMap;

/// 构造一个指定 local_count 的空 BytecodeFunction，用于 Frame 测试。
fn make_function_with_locals(local_count: usize) -> BytecodeFunction {
    BytecodeFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count,
        upvalue_count: 0,
        instructions: vec![],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

#[test]
fn test_frame_new_initializes_slots_to_local_count() {
    // Arrange
    let func = make_function_with_locals(2);

    // Act
    let frame = Frame::new(func);

    // Assert
    assert_eq!(
        frame.local_count(),
        2,
        "Frame::new 应按 function.local_count 初始化 slots 长度"
    );
    assert_eq!(frame.ip, 0, "新 Frame 的 ip 应为 0");
}

#[test]
fn test_frame_slot_set_and_get_roundtrip() {
    // Arrange
    let func = make_function_with_locals(2);
    let mut frame = Frame::new(func);

    // Act
    frame.set_slot(0, RuntimeValue::Int(42));

    // Assert
    assert_eq!(
        frame.get_slot(0).unwrap().to_int(),
        Some(42),
        "set_slot 写入的值应能通过 get_slot 读回（slots 合并的核心不变量）"
    );
}

#[test]
fn test_slots_unified_read_write_after_merge() {
    // Arrange
    // 规范 §3.1: Frame 合并 registers+locals 为单一 slots 数组。
    // 合并前 registers 和 locals 是两个独立数组，Mov 写 registers、LoadLocal 读 locals，
    // 导致赋值不写回。合并后同一 idx 索引同一数组，此 bug 物理上不可能发生。
    let func = make_function_with_locals(4);
    let mut frame = Frame::new(func);

    // Act — 通过统一 API 写入两个不同槽位
    frame.set_slot(0, RuntimeValue::Int(42));
    frame.set_slot(1, RuntimeValue::Int(7));

    // Assert — 读回的值必须与写入一致
    assert_eq!(
        frame.get_slot(0).unwrap().to_int(),
        Some(42),
        "slots[0] 应为写入的 42（registers/locals 合并后读写同一数组）"
    );
    assert_eq!(
        frame.get_slot(1).unwrap().to_int(),
        Some(7),
        "slots[1] 应为写入的 7"
    );

    // Assert — set_slot 应自动 resize slots 数组以容纳越界索引
    frame.set_slot(10, RuntimeValue::Int(99));
    assert_eq!(
        frame.get_slot(10).unwrap().to_int(),
        Some(99),
        "set_slot(10) 应触发自动 resize 并写入 99"
    );
    assert_eq!(
        frame.local_count(),
        11,
        "resize 后 local_count 应为 11（索引 0..10）"
    );
}
