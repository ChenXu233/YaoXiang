//! 字节码缓冲区单元测试
//!
//! 测试 ConstantPool 和 BytecodeBuffer 的基本操作：
//! - 常量池的添加和获取
//! - 字节码缓冲区的发射和获取

use crate::middle::core::ir::ConstValue;
use crate::middle::passes::codegen::buffer::{BytecodeBuffer, ConstantPool};

#[test]
fn test_constant_pool_add_get() {
    let mut pool = ConstantPool::new();
    let idx1 = pool.add(ConstValue::Int(42));
    let idx2 = pool.add(ConstValue::String("hello".to_string()));

    assert_eq!(idx1, 0);
    assert_eq!(idx2, 1);

    assert_eq!(pool.get(0), Some(&ConstValue::Int(42)));
    assert_eq!(pool.get(1), Some(&ConstValue::String("hello".to_string())));
    assert_eq!(pool.get(2), None);
}

#[test]
fn test_bytecode_buffer() {
    let mut buffer = BytecodeBuffer::new();

    buffer.emit(&[0x01, 0x02, 0x03]);
    buffer.emit(&[0x04, 0x05]);

    assert_eq!(buffer.bytecode(), &[0x01, 0x02, 0x03, 0x04, 0x05]);

    let idx = buffer.add_constant(ConstValue::Int(100));
    assert_eq!(idx, 0);
    assert_eq!(buffer.get_constant(0), Some(&ConstValue::Int(100)));
}
