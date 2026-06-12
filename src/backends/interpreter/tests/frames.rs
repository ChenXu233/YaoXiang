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
    frame.set_local(0, RuntimeValue::Int(42));
    assert_eq!(frame.get_local(0).unwrap().to_int(), Some(42));
}
