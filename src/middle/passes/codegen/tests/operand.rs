//! 操作数解析器单元测试
//!
//! 测试 OperandResolver 的寄存器转换和溢出检查功能。

use crate::middle::core::ir::Operand;
use crate::middle::passes::codegen::operand::OperandResolver;

#[test]
fn test_local_reg() {
    let resolver = OperandResolver::new();
    assert_eq!(resolver.to_reg(&Operand::Local(0)).unwrap(), 0);
    assert_eq!(resolver.to_reg(&Operand::Local(100)).unwrap(), 100);
}

#[test]
fn test_register_overflow() {
    let resolver = OperandResolver::new();
    assert!(resolver.to_reg(&Operand::Local(256)).is_err());
}
