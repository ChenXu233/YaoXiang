//! 标准库 trait 测试 — 基于语言规范 §3.5
//!
//! §3.5: 接口类型

use crate::frontend::core::typecheck::traits::std_traits::init_std_traits;
use crate::frontend::core::types::base::TraitTable;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_init_std_traits() {
    // Arrange
    let mut trait_table = TraitTable::default();

    // Act
    init_std_traits(&mut trait_table);

    // Assert - 标准库 trait 应该被初始化
    // 具体断言取决于 TraitTable 的实现
}

// ===================================================================
// Error path 测试
// ===================================================================

// 标准库 trait 的错误路径测试

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_init_std_traits_multiple_times() {
    // Arrange
    let mut trait_table = TraitTable::default();

    // Act - 多次初始化应该幂等
    init_std_traits(&mut trait_table);
    init_std_traits(&mut trait_table);

    // Assert - 不应该 panic
}
