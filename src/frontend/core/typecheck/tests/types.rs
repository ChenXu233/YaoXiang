//! 类型定义测试 — 基于语言规范 §3 & RFC-010
//!
//! §3.1-§3.17: 类型系统定义
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::types::{TypeCheckResult, ImportInfo};
use crate::frontend::core::types::base::{MonoType, PolyType};

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_type_check_result_default() {
    // Arrange & Act
    let result = TypeCheckResult::default();

    // Assert
    assert!(result.module_name.is_empty(), "module_name should be empty");
    assert!(result.bindings.is_empty(), "bindings should be empty");
    assert!(
        result.local_var_types.is_empty(),
        "local_var_types should be empty"
    );
}

#[test]
fn test_type_check_result_with_bindings() {
    // Arrange
    let mut result = TypeCheckResult::default();

    // Act
    result
        .bindings
        .insert("x".to_string(), PolyType::mono(MonoType::Int(32)));
    result
        .local_var_types
        .insert("x".to_string(), MonoType::Int(32));

    // Assert
    assert_eq!(result.bindings.len(), 1);
    assert_eq!(result.local_var_types.len(), 1);
}

#[test]
fn test_import_info_creation() {
    // Arrange & Act
    let import = ImportInfo {
        path: "std.io".to_string(),
        items: Some(vec!["print".to_string(), "println".to_string()]),
        alias: None,
    };

    // Assert
    assert_eq!(import.path, "std.io");
    assert!(import.items.is_some());
    assert_eq!(import.items.unwrap().len(), 2);
    assert!(import.alias.is_none());
}

#[test]
fn test_import_info_with_alias() {
    // Arrange & Act
    let import = ImportInfo {
        path: "std.io".to_string(),
        items: None,
        alias: Some("io".to_string()),
    };

    // Assert
    assert_eq!(import.path, "std.io");
    assert!(import.items.is_none());
    assert_eq!(import.alias.unwrap(), "io");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_type_check_result_with_many_bindings() {
    // Arrange
    let mut result = TypeCheckResult::default();

    // Act
    for i in 0..1000 {
        result
            .bindings
            .insert(format!("var_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    assert_eq!(result.bindings.len(), 1000, "should have 1000 bindings");
}
