//! ref 表达式 Parser 测试
//!
//! 测试 ref 表达式的解析器功能是否正常

use crate::frontend::lexer::tokens::TokenKind;

/// 测试 ref 关键字被正确识别
#[test]
fn test_ref_keyword_recognized() {
    // ref 应该是独立的关键字
    let tokens = crate::frontend::lexer::tokenize("ref x").unwrap();

    // 检查 tokens 包含 KwRef
    let has_ref = tokens.iter().any(|t| matches!(t.kind, TokenKind::KwRef));
    assert!(has_ref, "Expected KwRef token in 'ref x'");
}

/// 测试 ref 后跟标识符
#[test]
fn test_ref_with_identifier() {
    let tokens = crate::frontend::lexer::tokenize("ref myvar").unwrap();

    // 应该包含 KwRef 和 Identifier
    let has_ref = tokens.iter().any(|t| matches!(t.kind, TokenKind::KwRef));
    let has_id = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::Identifier(_)));

    assert!(has_ref, "Expected KwRef token");
    assert!(has_id, "Expected Identifier token");
}

/// 测试完整的 ref 表达式可以被解析
#[test]
fn test_ref_expression_parsed() {
    // 测试 parse 函数能处理包含 ref 的表达式
    let tokens = crate::frontend::lexer::tokenize("x = ref y").unwrap();
    let result = crate::frontend::parser::parse(&tokens);

    assert!(
        result.is_ok(),
        "Failed to parse 'x = ref y': {:?}",
        result.err()
    );
}

/// 测试 ref 关键字后跟赋值
#[test]
fn test_ref_assignment() {
    let tokens = crate::frontend::lexer::tokenize("shared = ref obj").unwrap();
    let result = crate::frontend::parser::parse(&tokens);

    assert!(
        result.is_ok(),
        "Failed to parse 'shared = ref obj': {:?}",
        result.err()
    );
}

/// 测试 ref 关键字在简单赋值语句中
#[test]
fn test_ref_in_simple_assignment() {
    let tokens = crate::frontend::lexer::tokenize("r = ref x").unwrap();
    let result = crate::frontend::parser::parse(&tokens);

    assert!(
        result.is_ok(),
        "Failed to parse 'r = ref x': {:?}",
        result.err()
    );
}
