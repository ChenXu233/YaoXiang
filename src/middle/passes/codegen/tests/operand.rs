//! 操作数解析器单元测试
//!
//! 测试 OperandResolver 的寄存器转换与溢出检查功能。

use crate::middle::ir::ir::{Operand, ValueId};
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

/// 7b.2：`Operand::Value` 经 `value_slots` 过渡映射解析为既有临时槽。
#[test]
fn test_value_resolves_through_value_slots() {
    // Arrange
    let mut resolver = OperandResolver::new();
    resolver.set_value_slots(&[7, 255]);

    // Act / Assert
    assert_eq!(
        resolver.to_reg(&Operand::Value(ValueId(0))).unwrap(),
        7,
        "ValueId 0 必须解析到 value_slots[0]"
    );
    assert_eq!(
        resolver.to_reg(&Operand::Value(ValueId(1))).unwrap(),
        255,
        "ValueId 1 必须解析到 value_slots[1]（上界 255 合法）"
    );
    assert!(
        resolver.is_register(&Operand::Value(ValueId(0))),
        "Value 是寄存器位（由值表映射到槽）"
    );
}

/// 7b.2：ValueId 无映射 / 槽超过 255 都必须显式诊断，不得静默落错槽。
#[test]
fn test_value_without_mapping_or_over_budget_is_rejected() {
    // Arrange：只有 1 个映射
    let mut resolver = OperandResolver::new();
    resolver.set_value_slots(&[0]);

    // Act / Assert：越界 ValueId
    assert!(
        resolver.to_reg(&Operand::Value(ValueId(1))).is_err(),
        "无映射的 ValueId 必须报 E3017"
    );

    // Arrange：槽号越预算
    resolver.set_value_slots(&[256]);
    // Act / Assert
    assert!(
        resolver.to_reg(&Operand::Value(ValueId(0))).is_err(),
        "槽号 > 255 必须报寄存器溢出"
    );

    // 清空后（模块 init 形态）同样显式报错
    resolver.clear_value_slots();
    assert!(
        resolver.to_reg(&Operand::Value(ValueId(0))).is_err(),
        "清空映射后 Value 必须显式报错，而不是落 0 号槽"
    );
}
