//! 光标位置定位工具
//!
//! 提供在源码中根据 LSP 光标位置查找标识符的功能。
//! 被 definition、references、hover 三个处理器共用。

use lsp_types::Position as LspPosition;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::util::span::Span;

/// 光标处的标识符信息
#[derive(Debug, Clone)]
pub struct IdentAtPosition {
    /// 标识符名称
    pub name: String,
    /// 标识符在源码中的 Span（1-indexed）
    pub span: Span,
}

/// 在源码中查找光标位置处的标识符
///
/// LSP Position 是 0-indexed，内部 Span 是 1-indexed，此函数负责转换。
///
/// 返回 `None` 如果：
/// - 词法分析失败
/// - 光标位置不在任何标识符上
pub fn find_identifier_at_position(
    source: &str,
    position: &LspPosition,
) -> Option<IdentAtPosition> {
    let tokens = tokenize(source).ok()?;

    // LSP 0-indexed → 内部 1-indexed
    let target_line = position.line as usize + 1;
    let target_col = position.character as usize + 1;

    for token in &tokens {
        let span = &token.span;
        if span.is_dummy() {
            continue;
        }

        // 判断光标是否落在此 token 的 span 内
        // Span.end 是 exclusive，所以用 < 比较 end column
        let after_start = target_line > span.start.line
            || (target_line == span.start.line && target_col >= span.start.column);
        let before_end = target_line < span.end.line
            || (target_line == span.end.line && target_col < span.end.column);

        if after_start && before_end {
            if let TokenKind::Identifier(ref name) = token.kind {
                return Some(IdentAtPosition {
                    name: name.clone(),
                    span: token.span,
                });
            }
            // 光标在非标识符 token 上，直接返回 None
            return None;
        }
    }

    None
}

/// 将 YaoXiang Span 转换为 LSP Range
///
/// YaoXiang Span 是 1-indexed，LSP Range 是 0-indexed。
pub fn span_to_range(span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: LspPosition {
            line: span.start.line.saturating_sub(1) as u32,
            character: span.start.column.saturating_sub(1) as u32,
        },
        end: LspPosition {
            line: span.end.line.saturating_sub(1) as u32,
            character: span.end.column.saturating_sub(1) as u32,
        },
    }
}

/// 在源码中查找所有指定名称的标识符出现位置
///
/// 返回每次出现的 Span 列表。
pub fn find_all_identifier_occurrences(
    source: &str,
    name: &str,
) -> Vec<Span> {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    tokens
        .iter()
        .filter_map(|token| {
            if let TokenKind::Identifier(ref ident) = token.kind {
                if ident == name && !token.span.is_dummy() {
                    return Some(token.span);
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
