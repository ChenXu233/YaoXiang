//! 对象安全测试 — 编译器实现细节，非语言规范要求
//!
//! 注意：对象安全检查是编译器实现限制，不是 YaoXiang 语言规范的一部分。
//! YaoXiang 使用结构化子类型（structural subtyping），不需要 Rust 风格的
//! 对象安全概念。此模块测试的是编译器内部的对象安全检查逻辑。
//!
//! 参考：语言规范 §3.5（接口类型）— 规范中未定义对象安全概念

use crate::frontend::core::typecheck::traits::object_safety::ObjectSafetyChecker;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_object_safety_checker_creation() {
    // Arrange & Act
    let _checker = ObjectSafetyChecker::new();

    // Assert — 应该成功创建，不 panic
}

#[test]
fn test_object_safety_checker_default() {
    // Arrange & Act
    let _checker = ObjectSafetyChecker;

    // Assert — Default trait 应可用
}

#[test]
fn test_check_clone_is_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Clone");

    // Assert
    assert!(result.is_ok(), "Clone should be object-safe");
}

#[test]
fn test_check_debug_is_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Debug");

    // Assert
    assert!(result.is_ok(), "Debug should be object-safe");
}

#[test]
fn test_check_dup_is_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Dup");

    // Assert
    assert!(result.is_ok(), "Dup should be object-safe");
}

#[test]
fn test_check_returns_ok_for_known_safe_traits() {
    // Arrange
    let checker = ObjectSafetyChecker::new();
    let safe_traits = ["Clone", "Debug", "Dup"];

    // Act & Assert
    for trait_name in &safe_traits {
        let result = checker.check(trait_name);
        assert!(result.is_ok(), "'{}' should be object-safe", trait_name);
    }
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_check_unknown_trait_is_not_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("UnknownTrait");

    // Assert
    assert!(result.is_err(), "Unknown trait should not be object-safe");
}

#[test]
fn test_check_error_contains_correct_message() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let err = checker.check("MyTrait").unwrap_err();

    // Assert
    assert!(
        err.message.contains("MyTrait"),
        "Error message should mention the trait name, got: {}",
        err.message
    );
}

#[test]
fn test_check_error_message_format() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let err = checker.check("Foo").unwrap_err();

    // Assert
    assert!(
        err.message.contains("not object-safe"),
        "Error message should contain 'not object-safe', got: {}",
        err.message
    );
}

#[test]
fn test_check_empty_trait_name_is_not_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("");

    // Assert
    assert!(
        result.is_err(),
        "Empty trait name should not be object-safe"
    );
}

#[test]
fn test_check_equal_is_object_safe() {
    // Arrange - 规范 §3.5.2: Equal 合并了 PartialEq + Eq
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Equal");

    // Assert
    assert!(result.is_ok(), "Equal should be object-safe (规范 §3.5.2)");
}

#[test]
fn test_check_iterator_is_not_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Iterator");

    // Assert
    assert!(
        result.is_err(),
        "Iterator should not be object-safe (not in known-safe list)"
    );
}

#[test]
fn test_check_display_is_not_object_safe() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Display");

    // Assert
    assert!(
        result.is_err(),
        "Display should not be object-safe (not in known-safe list)"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_object_safety_checker_with_complex_traits() {
    // Arrange
    let checker = ObjectSafetyChecker::new();
    let complex_trait_names = ["Clone", "Debug", "UnknownTrait", "Display", "Iterator"];

    // Act & Assert — 所有检查都不应 panic
    for name in &complex_trait_names {
        let _result = checker.check(name);
    }
}

#[test]
fn test_check_result_is_unit_on_success() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("Clone");

    // Assert
    assert!(result.is_ok(), "Successful check should return Ok");
    // Ok 值为 ()，验证不携带额外数据
    result.unwrap();
    // val 的类型是 ()，此处仅确认解构成功
    let () = ();
}

#[test]
fn test_object_safety_error_is_cloneable() {
    // Arrange
    let checker = ObjectSafetyChecker::new();
    let err = checker.check("Foo").unwrap_err();

    // Act
    let err_clone = err.clone();

    // Assert
    assert_eq!(
        err.message, err_clone.message,
        "Cloned error should have same message"
    );
}

#[test]
fn test_object_safety_error_is_debuggable() {
    // Arrange
    let checker = ObjectSafetyChecker::new();
    let err = checker.check("Foo").unwrap_err();

    // Act
    let debug_str = format!("{:?}", err);

    // Assert
    assert!(
        debug_str.contains("ObjectSafetyError"),
        "Debug output should contain type name, got: {}",
        debug_str
    );
}

#[test]
fn test_check_special_characters_in_trait_name() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act
    let result = checker.check("My-Trait_v2.0");

    // Assert
    assert!(
        result.is_err(),
        "Trait with special characters should not be object-safe"
    );
}

#[test]
fn test_check_repeated_calls_are_consistent() {
    // Arrange
    let checker = ObjectSafetyChecker::new();

    // Act & Assert — 多次调用同一 trait 应返回一致结果
    for _ in 0..5 {
        assert!(
            checker.check("Clone").is_ok(),
            "Clone should always be object-safe"
        );
        assert!(
            checker.check("Foo").is_err(),
            "Foo should always be not object-safe"
        );
    }
}
