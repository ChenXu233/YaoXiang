//! RFC-029g 词法测试 — `pub` 不再是关键字
//!
//! 规范：reference/language-spec/syntax.md §1.3 关键字表（18 → 17）
//! RFC-029g（已接受 2026-10-02）：删除 `pub` 关键字与自动绑定
//!
//! 删除后 `pub` 退回普通标识符：既不产生可见性语义，也不再被词法层
//! 识别为 `KwPub`。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;

#[test]
fn test_pub_tokenizes_as_identifier_after_rfc029g() {
    // Arrange
    let source = "pub";

    // Act
    let tokens = tokenize(source).expect("`pub` 应能正常词法化");
    let first = tokens.first().expect("至少应产出一个 token");

    // Assert
    assert!(
        matches!(&first.kind, TokenKind::Identifier(name) if name == "pub"),
        "RFC-029g 后 `pub` 必须退回普通标识符，实际: {:?}",
        first.kind
    );
}
