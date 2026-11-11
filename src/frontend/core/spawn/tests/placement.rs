//! spawn 位置检查测试 — 基于 RFC-024 §2.1
//!
//! RFC-024 §2.1: spawn 块可以出现在任何表达式位置

use crate::ast::{Module};
use crate::frontend::core::spawn::placement::check_spawn_placement;
use crate::util::span::Span;

// 辅助函数

fn empty_module() -> Module {
    Module {
        items: vec![],
        span: Span::dummy(),
    }
}

// Happy path

#[test]
fn test_empty_module_produces_no_diagnostics() {
    // Arrange
    let module = empty_module();

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "空模块不应产生任何诊断, 实际数量: {}",
        diagnostics.len()
    );
}
