//! LSP 光标位置定位测试
//!
//! 规范来源：RFC-039 D55（UTF-16 列 → char 列换算为跳转/hover/引用的
//! 定位前提）；语言规范无 LSP 章节，D55 为本组测试的权威锚点。
//!
//! 测试覆盖：
//! - 标识符查找功能
//! - 光标位置转换（含扩展平面字符与越界钳制）
//! - 标识符出现位置查找
//! - Span 到 Range 转换

use lsp_types::Position as LspPosition;

use crate::lsp::locate::{find_identifier_at_position, span_to_range, find_all_identifier_occurrences};
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

#[test]
fn test_position_to_internal_utf16_maps_astral_char_columns() {
    use crate::lsp::locate::position_to_internal_utf16;

    // Arrange：行内两个 😀（各占 2 个 UTF-16 单元）后跟 abcd——
    // b 的 UTF-16 偏移是 10，char 偏移是 8，lexer 列（1-indexed）应为 9
    let source = "s = \"😀😀abcd\"\n";
    let pos = LspPosition {
        line: 0,
        character: 10,
    };

    // Act
    let (line, col) = position_to_internal_utf16(source, &pos);

    // Assert：不换算的旧实现会给出 11（落在 c 上），换算后应为 9（落在 b 上）
    assert_eq!(line, 1, "行号直传");
    assert_eq!(col, 9, "UTF-16 偏移 10 应换算为 char 列 9（b 字符）");
}

#[test]
fn test_position_to_internal_utf16_keeps_bmp_columns_unchanged() {
    use crate::lsp::locate::position_to_internal_utf16;

    // Arrange：BMP 中文字符每字符占 1 个 UTF-16 单元，与 char 计数一致
    let source = "值 = 一\n";
    let pos = LspPosition {
        line: 0,
        character: 4,
    };

    // Act
    let (line, col) = position_to_internal_utf16(source, &pos);

    // Assert：UTF-16 偏移 4 → char 列 5（第二行的换算同理按行取源）
    assert_eq!(line, 1);
    assert_eq!(col, 5, "BMP 字符的 UTF-16 偏移与 char 列一致");
}

#[test]
fn test_position_to_internal_utf16_clamps_past_line_end() {
    use crate::lsp::locate::position_to_internal_utf16;

    // Arrange：客户端可能基于陈旧文本发出越出行尾的位置
    let source = "ab\n";
    let pos = LspPosition {
        line: 0,
        character: 99,
    };

    // Act
    let (line, col) = position_to_internal_utf16(source, &pos);

    // Assert：按该行实际宽度收敛到行尾之后一格（len+1）
    assert_eq!(line, 1);
    assert_eq!(col, 3, "越界位置应收敛到行尾（len+1）");
}
