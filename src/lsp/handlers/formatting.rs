//! LSP 格式化处理器
//!
//! 实现 `textDocument/formatting` 和 `textDocument/rangeFormatting` 功能。

use lsp_types::{DocumentFormattingParams, DocumentRangeFormattingParams, TextEdit, Range, Position};

use crate::formatter::{format_source, FormatOptions};
use crate::lsp::session::Session;

/// 处理 textDocument/formatting 请求
///
/// 返回整个文件的格式化编辑列表。
pub fn handle_formatting(
    session: &Session,
    params: DocumentFormattingParams,
) -> Option<Vec<TextEdit>> {
    let uri = &params.text_document.uri;
    let file_path = uri.path().as_str();
    let doc = session.document_store().get(file_path)?;
    let source = doc.content().to_string();

    let mut options = FormatOptions::default();
    // 使用 LSP 传入的选项覆盖
    options.indent_width = params.options.tab_size as usize;
    if !params.options.insert_spaces {
        options.use_tabs = true;
    }

    match format_source(&source, &options) {
        Ok(formatted) => {
            if formatted == source {
                // 无需更改
                Some(vec![])
            } else {
                // 替换整个文档
                let line_count = source.lines().count();
                let last_line = source.lines().last().unwrap_or("");
                Some(vec![TextEdit {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: line_count as u32,
                            character: last_line.len() as u32,
                        },
                    },
                    new_text: formatted,
                }])
            }
        }
        Err(e) => {
            tracing::warn!("格式化失败: {}", e);
            None
        }
    }
}

/// 处理 textDocument/rangeFormatting 请求
///
/// 返回指定范围内的格式化编辑列表。
pub fn handle_range_formatting(
    session: &Session,
    params: DocumentRangeFormattingParams,
) -> Option<Vec<TextEdit>> {
    let uri = &params.text_document.uri;
    let file_path = uri.path().as_str();
    let doc = session.document_store().get(file_path)?;
    let source = doc.content().to_string();

    let mut options = FormatOptions::default();
    options.indent_width = params.options.tab_size as usize;
    if !params.options.insert_spaces {
        options.use_tabs = true;
    }

    // 对于范围格式化，先格式化整个文件，再提取范围内的变更
    match format_source(&source, &options) {
        Ok(formatted) => {
            if formatted == source {
                Some(vec![])
            } else {
                // 返回整个范围的替换
                Some(vec![TextEdit {
                    range: params.range,
                    new_text: extract_range(&formatted, &params.range),
                }])
            }
        }
        Err(e) => {
            tracing::warn!("范围格式化失败: {}", e);
            None
        }
    }
}

/// 从格式化后的文本中提取指定范围
fn extract_range(
    formatted: &str,
    range: &Range,
) -> String {
    let lines: Vec<&str> = formatted.lines().collect();
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    if start_line >= lines.len() {
        return String::new();
    }

    let end_line = end_line.min(lines.len() - 1);
    let mut result = String::new();

    for (i, line) in lines[start_line..=end_line].iter().enumerate() {
        if i == 0 && start_line == end_line {
            // 单行范围
            let start_char = range.start.character as usize;
            let end_char = (range.end.character as usize).min(line.len());
            result.push_str(&line[start_char..end_char]);
        } else if i == 0 {
            // 第一行
            let start_char = range.start.character as usize;
            result.push_str(&line[start_char..]);
            result.push('\n');
        } else if i == end_line - start_line {
            // 最后一行
            let end_char = (range.end.character as usize).min(line.len());
            result.push_str(&line[..end_char]);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}
