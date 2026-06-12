//! LSP 光标位置定位测试
//!
//! 测试覆盖：
//! - 标识符查找功能
//! - 光标位置转换
//! - 标识符出现位置查找
//! - Span 到 Range 转换

use lsp_types::Position as LspPosition;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::lsp::locate::{find_identifier_at_position, span_to_range, find_all_identifier_occurrences, IdentAtPosition};
use crate::util::span::Span;

#[test]
fn test_find_identifier_simple() {
    let source = "x = 42\n";
    let pos = LspPosition {
        line: 0,
        character: 0,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "x");
}

#[test]
fn test_find_identifier_not_on_ident() {
    let source = "x = 42\n";
    // 光标在 '=' 上
    let pos = LspPosition {
        line: 0,
        character: 2,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_none());
}

#[test]
fn test_find_identifier_second_line() {
    let source = "x = 1\ny = 2\n";
    let pos = LspPosition {
        line: 1,
        character: 0,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "y");
}

#[test]
fn test_find_identifier_multichar() {
    let source = "hello = 42\n";
    // 光标在 'hello' 中间（字符位置 2 → 'l'）
    let pos = LspPosition {
        line: 0,
        character: 2,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "hello");
}

#[test]
fn test_find_identifier_end_of_ident() {
    let source = "abc = 1\n";
    // 光标在 'abc' 最后一个字符 'c' 上（character=2）
    let pos = LspPosition {
        line: 0,
        character: 2,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "abc");
}

#[test]
fn test_find_identifier_past_end() {
    let source = "abc = 1\n";
    // 光标在 'abc' 之后的空格上（character=3）
    let pos = LspPosition {
        line: 0,
        character: 3,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_none());
}

#[test]
fn test_find_identifier_invalid_source() {
    // 完全无法词法分析的源码
    let source = "";
    let pos = LspPosition {
        line: 0,
        character: 0,
    };
    let result = find_identifier_at_position(source, &pos);
    assert!(result.is_none());
}

#[test]
fn test_find_all_occurrences() {
    let source = "x = 1\ny = x + x\n";
    let spans = find_all_identifier_occurrences(source, "x");
    assert_eq!(spans.len(), 3, "x 应出现 3 次");
}

#[test]
fn test_find_all_occurrences_no_match() {
    let source = "x = 1\n";
    let spans = find_all_identifier_occurrences(source, "y");
    assert!(spans.is_empty());
}

#[test]
fn test_span_to_range_conversion() {
    use crate::util::span::Position;
    let span = Span {
        start: Position {
            line: 1,
            column: 1,
            offset: 0,
        },
        end: Position {
            line: 1,
            column: 4,
            offset: 3,
        },
    };
    let range = span_to_range(&span);
    assert_eq!(range.start.line, 0);
    assert_eq!(range.start.character, 0);
    assert_eq!(range.end.line, 0);
    assert_eq!(range.end.character, 3);
}
