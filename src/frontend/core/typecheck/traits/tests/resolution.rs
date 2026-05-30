//! 特质解析测试 — 基于语言规范 §3.5, RFC-011 §2
//!
//! §3.5: 接口类型
//! RFC-011 §2: 标准库 trait 定义与解析

use crate::frontend::core::typecheck::traits::resolution::TraitResolver;
use crate::frontend::core::typecheck::traits::resolution::TraitResolutionError;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_resolve_known_trait_debug() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve("Debug");

    // Assert
    assert!(result.is_ok(), "应该成功解析已知的 Debug trait");
    assert_eq!(
        result.unwrap(),
        "std::fmt::Debug",
        "Debug trait 的解析路径应匹配预期"
    );
}

#[test]
fn test_resolve_known_trait_dup() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve("Dup");

    // Assert
    assert!(result.is_ok(), "应该成功解析已知的 Dup trait");
    assert_eq!(
        result.unwrap(),
        "std::Dup",
        "Dup trait 的解析路径应匹配预期"
    );
}

#[test]
fn test_resolve_known_trait_clone() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve("Clone");

    // Assert
    assert!(result.is_ok(), "应该成功解析已知的 Clone trait");
    assert_eq!(
        result.unwrap(),
        "std:: Clone",
        "Clone trait 的解析路径应匹配预期"
    );
}

#[test]
fn test_is_trait_defined_known_traits() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act & Assert
    assert!(resolver.is_trait_defined("Clone"), "Clone 应为已定义 trait");
    assert!(resolver.is_trait_defined("Debug"), "Debug 应为已定义 trait");
    assert!(resolver.is_trait_defined("Dup"), "Dup 应为已定义 trait");
}

#[test]
fn test_resolve_trait_path_full_path() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve_trait_path("std::fmt::Debug");

    // Assert
    assert!(result.is_ok(), "应该成功解析完整 trait 路径");
    assert_eq!(
        result.unwrap(),
        "std::fmt::Debug",
        "完整路径解析后应返回正确结果"
    );
}

#[test]
fn test_resolve_trait_path_marker_path() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve_trait_path("std::marker::Dup");

    // Assert
    assert!(result.is_ok(), "应该成功解析 marker 模块下的 trait 路径");
    assert_eq!(
        result.unwrap(),
        "std::Dup",
        "marker 路径解析后应返回正确结果"
    );
}

#[test]
fn test_default_trait_resolver() {
    // Arrange & Act
    let resolver = TraitResolver;

    // Assert - Default 应创建可用的解析器
    assert!(
        resolver.is_trait_defined("Clone"),
        "通过 Default 创建的解析器应能正常工作"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_resolve_unknown_trait_returns_error() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve("NonExistentTrait");

    // Assert
    assert!(result.is_err(), "解析未知 trait 应返回错误");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("NonExistentTrait"),
        "错误消息应包含未知 trait 名称，实际: {}",
        err.message
    );
}

#[test]
fn test_resolve_empty_name_returns_error() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve("");

    // Assert
    assert!(result.is_err(), "空名称应返回错误");
}

#[test]
fn test_resolve_trait_path_invalid_returns_error() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result = resolver.resolve_trait_path("UnknownTrait");

    // Assert
    assert!(result.is_err(), "解析不存在的 trait 路径应返回错误");
}

#[test]
fn test_is_trait_defined_unknown_returns_false() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let defined = resolver.is_trait_defined("NonExistentTrait");

    // Assert
    assert!(!defined, "未知 trait 应返回 false");
}

#[test]
fn test_is_trait_defined_empty_name_returns_false() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let defined = resolver.is_trait_defined("");

    // Assert
    assert!(!defined, "空名称应返回 false");
}

#[test]
fn test_resolution_error_debug_format() {
    // Arrange
    let err = TraitResolutionError {
        message: "test error".to_string(),
    };

    // Act
    let debug_str = format!("{:?}", err);

    // Assert
    assert!(
        debug_str.contains("test error"),
        "TraitResolutionError 的 Debug 输出应包含错误消息"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_resolve_trait_path_with_deep_nesting() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act - 多层嵌套路径，取最后一段作为 trait 名称
    let result = resolver.resolve_trait_path("a::b::c::d::Clone");

    // Assert
    assert!(result.is_ok(), "多层嵌套路径应能正确提取最后的 trait 名称");
    assert_eq!(result.unwrap(), "std:: Clone", "深层路径应正确解析 Clone");
}

#[test]
fn test_resolve_trait_name_is_case_sensitive() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act
    let result_lower = resolver.resolve("clone");
    let result_upper = resolver.resolve("Clone");

    // Assert
    assert!(result_lower.is_err(), "小写 'clone' 不应匹配 'Clone'");
    assert!(result_upper.is_ok(), "大写 'Clone' 应正确匹配");
}

#[test]
fn test_resolver_multiple_resolves_consistent() {
    // Arrange
    let resolver = TraitResolver::new();

    // Act - 多次解析同一 trait 应返回相同结果
    let first = resolver.resolve("Debug");
    let second = resolver.resolve("Debug");

    // Assert
    assert_eq!(
        first.unwrap(),
        second.unwrap(),
        "多次解析同一 trait 应返回一致结果"
    );
}

#[test]
fn test_trait_resolution_error_is_clone() {
    // Arrange
    let err = TraitResolutionError {
        message: "clone test".to_string(),
    };

    // Act
    let cloned = err.clone();

    // Assert
    assert_eq!(
        err.message, cloned.message,
        "TraitResolutionError 应支持 Clone 且内容一致"
    );
}
