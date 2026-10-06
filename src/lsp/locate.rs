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

/// LSP Position（0-indexed，UTF-16 列）→ 内部 (line, col)（1-indexed，char 列）
///
/// 列单位换算依赖源码：LSP `character` 按 UTF-16 code unit 计数，lexer 列按
/// char 计数（#433）——扩展平面字符（emoji 等）一个 char 占两个 UTF-16 单元，
/// 不换算则该行跳转/hover/引用全部错位。位置越出行尾/文末时按实际行宽收敛，
/// 容忍客户端基于陈旧文本发出的位置。
pub fn position_to_internal_utf16(
    source: &str,
    position: &LspPosition,
) -> (usize, usize) {
    let target_line = position.line as usize;
    let line_str = source.split('\n').nth(target_line).unwrap_or("");

    let mut utf16_seen = 0usize;
    let mut col = line_str.chars().count() + 1;
    for (chars_seen, ch) in line_str.chars().enumerate() {
        if utf16_seen >= position.character as usize {
            col = chars_seen + 1;
            break;
        }
        utf16_seen += ch.len_utf16();
    }

    (target_line + 1, col)
}

/// 在源码中查找光标位置处的标识符
///
/// LSP Position（0-indexed，UTF-16 列）由 [`position_to_internal_utf16`]
/// 换算为内部 1-indexed char 列。
///
/// 返回 `None` 如果：
/// - 词法分析失败
/// - 光标位置不在任何标识符上
pub fn find_identifier_at_position(
    source: &str,
    position: &LspPosition,
) -> Option<IdentAtPosition> {
    let tokens = tokenize(source).ok()?;

    // LSP 0-indexed UTF-16 → 内部 1-indexed char
    let (target_line, target_col) = position_to_internal_utf16(source, position);

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
