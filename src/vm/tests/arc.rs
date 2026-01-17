//! VM Arc 指令执行测试

use crate::vm::executor::ArcValue;
use crate::vm::opcode::TypedOpcode;

/// 测试 ArcValue 基本功能
#[test]
fn test_arc_value_creation() {
    let target = 0x12345678usize;
    let arc = ArcValue::new(target);

    assert_eq!(arc.target(), target);
}

/// 测试 ArcValue Clone
#[test]
fn test_arc_value_clone() {
    let target = 0x12345678usize;
    let arc1 = ArcValue::new(target);
    let arc2 = arc1.clone();

    // Clone 后应该指向同一个目标
    assert_eq!(arc1.target(), arc2.target());
}

/// 测试 ArcValue target 方法
#[test]
fn test_arc_target_method() {
    let target = 42usize;
    let arc = ArcValue::new(target);

    assert_eq!(arc.target(), 42);
}

/// 测试 Arc 操作码存在
#[test]
fn test_arc_opcodes_exist() {
    // 验证 ArcNew, ArcClone, ArcDrop 操作码存在
    assert!(matches!(
        TypedOpcode::try_from(0x79),
        Ok(TypedOpcode::ArcNew)
    ));
    assert!(matches!(
        TypedOpcode::try_from(0x7A),
        Ok(TypedOpcode::ArcClone)
    ));
    assert!(matches!(
        TypedOpcode::try_from(0x7B),
        Ok(TypedOpcode::ArcDrop)
    ));
}

/// 测试 ArcNew 操作码值
#[test]
fn test_arc_new_opcode_value() {
    let opcode = TypedOpcode::ArcNew;
    let opcode_value = opcode as u8;
    assert_eq!(opcode_value, 0x79);
}

/// 测试 ArcClone 操作码值
#[test]
fn test_arc_clone_opcode_value() {
    let opcode = TypedOpcode::ArcClone;
    let opcode_value = opcode as u8;
    assert_eq!(opcode_value, 0x7A);
}

/// 测试 ArcDrop 操作码值
#[test]
fn test_arc_drop_opcode_value() {
    let opcode = TypedOpcode::ArcDrop;
    let opcode_value = opcode as u8;
    assert_eq!(opcode_value, 0x7B);
}

/// 测试 Arc 值相等性
#[test]
fn test_arc_equality() {
    let target = 0x12345678usize;
    let arc1 = ArcValue::new(target);
    let arc2 = ArcValue::new(target);

    // 不同的 Arc 实例但相同的目标
    assert_eq!(arc1.target(), arc2.target());
}

/// 测试 Arc 值边界值
#[test]
fn test_arc_boundary_values() {
    // 测试地址 0
    let arc_zero = ArcValue::new(0usize);
    assert_eq!(arc_zero.target(), 0);

    // 测试大地址
    let large_addr = usize::MAX;
    let arc_large = ArcValue::new(large_addr);
    assert_eq!(arc_large.target(), large_addr);
}
