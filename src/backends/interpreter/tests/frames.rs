//! 调用栈帧测试
//!
//! 测试覆盖内容：
//! - Frame 的创建和初始化
//! - 局部变量的访问和修改

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::frames::Frame;
use crate::middle::bytecode::BytecodeFunction;
use std::collections::HashMap;

fn make_test_function() -> BytecodeFunction {
    BytecodeFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 2,
        upvalue_count: 0,
        instructions: vec![],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

#[test]
fn test_frame_new() {
    let func = make_test_function();
    let frame = Frame::new(func);
    assert_eq!(frame.local_count(), 2);
    assert_eq!(frame.ip, 0);
}

#[test]
fn test_frame_local_access() {
    let func = make_test_function();
    let mut frame = Frame::new(func);
    frame.set_slot(0, RuntimeValue::Int(42));
    assert_eq!(frame.get_slot(0).unwrap().to_int(), Some(42));
}

#[test]
fn test_slots_unified_read_write() {
    let func = BytecodeFunction {
        name: "test_slots".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        upvalue_count: 0,
        instructions: vec![],
        labels: std::collections::HashMap::new(),
        exception_handlers: vec![],
        debug_map: std::collections::HashMap::new(),
    };
    let mut frame = Frame::new(func);
    // 通过 set_slot 写入，通过 get_slot 读取 — 同一个数组
    frame.set_slot(0, RuntimeValue::Int(42));
    frame.set_slot(1, RuntimeValue::Int(7));
    assert_eq!(frame.get_slot(0).unwrap().to_int(), Some(42));
    assert_eq!(frame.get_slot(1).unwrap().to_int(), Some(7));
    // 自动 resize
    frame.set_slot(10, RuntimeValue::Int(99));
    assert_eq!(frame.get_slot(10).unwrap().to_int(), Some(99));
    assert_eq!(frame.local_count(), 11);
}
