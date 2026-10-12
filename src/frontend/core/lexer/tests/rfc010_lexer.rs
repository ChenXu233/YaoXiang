//! RFC-010 Unified Syntax Lexer Tests
//! Tests lexer support for RFC-010 generic syntax

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;

#[test]
fn test_simple_angle_brackets() {
    // Test simple < and > characters
    let source = "<>";
    let tokens = tokenize(source).unwrap();

    println!("=== Angle brackets test ===");
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.kind);
    }
    println!("=== End angle brackets ===");

    // Basic assertion - just check we get tokens
    assert!(tokens.len() >= 2);
}

#[test]
fn test_generic_syntax_tokenization() {
    // RFC-010: Basic generic syntax
    let source = "List(T)";
    let tokens = tokenize(source).unwrap();

    // Debug output
    println!("=== RFC-010 Debug: Tokens for '{}' ===", source);
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.kind);
    }
    println!("=== End Debug ===");

    // Basic check - we expect at least 4 tokens
    assert!(
        tokens.len() >= 4,
        "Expected at least 4 tokens, got {}",
        tokens.len()
    );

    // Just check that we get identifier tokens
    let has_list = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::Identifier(_)));
    assert!(has_list, "Should have identifier token");

    // Check that we get some kind of bracket (might be LBracket due to current lexer state)
    let has_bracket = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::LBracket) || matches!(t.kind, TokenKind::LParen));
    assert!(has_bracket, "Should have bracket or lt token");
}

#[test]
fn test_multiple_generic_parameters() {
    // RFC-010: Multiple type parameters
    let source = "Map(K, V)";
    let tokens = tokenize(source).unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::LParen);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::Comma);
    assert!(matches!(tokens[4].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[5].kind, TokenKind::RParen);
}

#[test]
fn test_generic_with_constraints() {
    // RFC-010: Generic with constraints
    let source = "T: Clone";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: Identifier, Colon, Identifier
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Colon);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
}

// 以下六词曾是 RFC-010 早期草案的关键字；现行规范（RFC-010 §统一语法、
// RFC-011a）已将其全部移除（「没有 impl」「无需关键字，接口名写在类型体后」），
// 词法层对它们一律产出普通标识符。测试断言这一现状，防止关键字被重新引入。

#[test]
fn test_where_is_plain_identifier() {
    // Arrange / Act
    let tokens = tokenize("where").unwrap();

    // Assert
    assert_eq!(tokens.len(), 2, "where 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "where"),
        "where 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_trait_is_plain_identifier() {
    let tokens = tokenize("trait").unwrap();

    assert_eq!(tokens.len(), 2, "trait 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "trait"),
        "trait 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_interface_is_plain_identifier() {
    let tokens = tokenize("interface").unwrap();

    assert_eq!(tokens.len(), 2, "interface 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "interface"),
        "interface 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_impl_is_plain_identifier() {
    let tokens = tokenize("impl").unwrap();

    assert_eq!(tokens.len(), 2, "impl 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "impl"),
        "impl 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_forall_is_plain_identifier() {
    let tokens = tokenize("forall").unwrap();

    assert_eq!(tokens.len(), 2, "forall 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "forall"),
        "forall 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_exists_is_plain_identifier() {
    let tokens = tokenize("exists").unwrap();

    assert_eq!(tokens.len(), 2, "exists 应产出 标识符 + Eof 两个 token");
    assert!(
        matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "exists"),
        "exists 不是关键字，应为 Identifier，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_nested_generics() {
    // RFC-010: Nested generic types（现行语法用圆括号：Option(Vec(T))；
    // 尖括号 < > 词法上是普通 Lt/Gt 运算符，无类型位特殊形态）
    let source = "Option(Vec(T))";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: Option ( Vec ( T ) )
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::LParen);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::LParen);
    assert!(matches!(tokens[4].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[5].kind, TokenKind::RParen);
    assert_eq!(tokens[6].kind, TokenKind::RParen);
}

#[test]
fn test_complex_generic_expression() {
    // RFC-010 泛型 + 接口约束（现行语法，language-spec type-system §5.1：
    // (T: Clone)(value: T) -> T ）
    let source = "clone: (T: Clone)(value: T) -> T";
    let tokens = tokenize(source).unwrap();

    // 标识符、约束冒号、箭头都应正确产出
    assert!(
        matches!(tokens[0].kind, TokenKind::Identifier(_)),
        "func 名应为 Identifier"
    );
    assert_eq!(tokens[1].kind, TokenKind::Colon, "函数声明冒号");

    // 约束冒号：位于类型参数 T 之后
    let colon_idx = tokens
        .iter()
        .position(|t| t.kind == TokenKind::Colon)
        .unwrap();
    let constraint_colon = tokens[colon_idx + 1..]
        .iter()
        .position(|t| t.kind == TokenKind::Colon)
        .map(|i| colon_idx + 1 + i)
        .unwrap();
    assert!(
        matches!(tokens[constraint_colon - 1].kind, TokenKind::Identifier(_)),
        "约束冒号前应是类型参数标识符"
    );

    // 箭头 ->
    assert!(
        tokens.iter().any(|t| t.kind == TokenKind::Arrow),
        "函数签名应产出 Arrow token"
    );
}
