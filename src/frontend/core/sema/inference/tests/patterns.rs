//! 模式匹配测试 — 基于语言规范 §4.8
//!
//! §4.8: 模式匹配

use crate::frontend::core::sema::inference::patterns::PatternInferrer;
use crate::ast::Pattern;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::types::{MonoType};

fn new_inferrer() -> PatternInferrer {
    PatternInferrer::new()
}

/// 整数字面量模式 `n`。
fn int_pattern(n: i128) -> Pattern {
    Pattern::Literal(Literal::Int(n))
}

// Happy path 测试

#[test]
fn test_pattern_infer_wildcard() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer.infer_pattern(&Pattern::Wildcard).unwrap();

    // Assert
    assert!(
        matches!(result, MonoType::TypeVar(_)),
        "Wildcard 模式应返回 TypeVar，实际得到: {:?}",
        result
    );
}

#[test]
fn test_pattern_infer_int_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer.infer_pattern(&int_pattern(42)).unwrap();

    // Assert
    assert_eq!(result, MonoType::Int(64), "Int 字面量模式应返回 Int(64)");
}
