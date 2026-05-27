//! 签名解析测试 — 基于语言规范 §3.7 & RFC-010
//!
//! §3.7: 函数类型
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::signature::parse_signature;
use crate::frontend::core::types::base::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_parse_signature_simple_function() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Void", &mut env);

    // Assert - 应该返回零参数的函数类型
    match result {
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => {
            assert!(params.is_empty(), "零参数函数签名的 params 应为空");
            assert!(
                matches!(*return_type, MonoType::Void),
                "返回类型应为 Void，实际: {:?}",
                return_type
            );
            assert!(!is_async, "默认函数签名应为非异步");
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_with_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int, Float) -> String", &mut env);

    // Assert - 应该解析为包含两个参数的函数类型
    match result {
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => {
            assert_eq!(params.len(), 2, "应有 2 个参数，实际: {}", params.len());
            assert!(
                matches!(params[0], MonoType::Int(32)),
                "第 1 个参数应为 Int(32)，实际: {:?}",
                params[0]
            );
            assert!(
                matches!(params[1], MonoType::Float(64)),
                "第 2 个参数应为 Float(64)，实际: {:?}",
                params[1]
            );
            assert!(
                matches!(*return_type, MonoType::String),
                "返回类型应为 String，实际: {:?}",
                return_type
            );
            assert!(!is_async, "默认函数签名应为非异步");
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_parse_signature_invalid_syntax() {
    // Arrange - 不以 '(' 开头的非法签名，解析为常量类型名（TypeRef）
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("invalid -> syntax", &mut env);

    // Assert - 非法签名不应该是 Fn 类型，而是降级为 TypeRef
    assert!(
        !matches!(result, MonoType::Fn { .. }),
        "非法签名不应解析为 Fn 类型，实际: {:?}",
        result
    );
    assert!(
        matches!(result, MonoType::TypeRef(_)),
        "非法签名应降级为 TypeRef，实际: {:?}",
        result
    );
}

#[test]
fn test_parse_signature_unmatched_paren() {
    // Arrange - 缺少右括号的签名，触发 unmatched '(' 错误路径
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int", &mut env);

    // Assert - 错误路径返回带类型变量的降级 Fn
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 1, "降级 Fn 应有 1 个类型变量参数");
            assert!(
                matches!(*return_type, MonoType::Void),
                "降级 Fn 返回类型应为 Void，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望降级 Fn 类型，实际得到: {:?}", other),
    }
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_parse_signature_empty_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("() -> Int", &mut env);

    // Assert - 空参数列表应该有效
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert!(params.is_empty(), "空参数列表应解析为空 Vec");
            assert!(
                matches!(*return_type, MonoType::Int(32)),
                "返回类型应为 Int(32)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_many_params() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int, Int, Int, Int, Int) -> Int", &mut env);

    // Assert - 多参数应该有效
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 5, "应有 5 个参数，实际: {}", params.len());
            for (i, param) in params.iter().enumerate() {
                assert!(
                    matches!(param, MonoType::Int(32)),
                    "第 {} 个参数应为 Int(32)，实际: {:?}",
                    i + 1,
                    param
                );
            }
            assert!(
                matches!(*return_type, MonoType::Int(32)),
                "返回类型应为 Int(32)，实际: {:?}",
                return_type
            );
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}

#[test]
fn test_parse_signature_nested_function_type() {
    // Arrange - 嵌套函数类型: (Int) -> (Float) -> String
    let mut env = TypeEnvironment::new();

    // Act
    let result = parse_signature("(Int) -> (Float) -> String", &mut env);

    // Assert - 外层应为 Fn(Int) -> Fn(Float)->String
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            // 外层参数
            assert_eq!(params.len(), 1, "外层应有 1 个参数，实际: {}", params.len());
            assert!(
                matches!(params[0], MonoType::Int(32)),
                "外层参数应为 Int(32)，实际: {:?}",
                params[0]
            );

            // 内层返回类型应为 Fn(Float) -> String
            match *return_type {
                MonoType::Fn {
                    params: ref inner_params,
                    return_type: ref inner_return,
                    ..
                } => {
                    assert_eq!(
                        inner_params.len(),
                        1,
                        "内层应有 1 个参数，实际: {}",
                        inner_params.len()
                    );
                    assert!(
                        matches!(inner_params[0], MonoType::Float(64)),
                        "内层参数应为 Float(64)，实际: {:?}",
                        inner_params[0]
                    );
                    assert!(
                        matches!(**inner_return, MonoType::String),
                        "内层返回类型应为 String，实际: {:?}",
                        inner_return
                    );
                }
                ref other => panic!("返回类型应为嵌套 Fn，实际: {:?}", other),
            }
        }
        other => panic!("期望 Fn 类型，实际得到: {:?}", other),
    }
}
