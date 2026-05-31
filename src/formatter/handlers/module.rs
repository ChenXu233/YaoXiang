//! 模块格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::stmt::format_stmt;
use super::comment::format_comments_between;

/// 格式化模块
pub fn format_module(
    module: &Module,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = String::new();

    // 输出文件头注释（在第一个语句之前的注释）
    let header_comments = if module.items.is_empty() {
        // 空模块：获取所有注释
        source_map.comments_between_lines(1, usize::MAX)
    } else {
        let first_stmt_line = module.items.first().map(|s| s.span.start.line).unwrap_or(1);
        source_map.comments_between_lines(1, first_stmt_line.saturating_sub(1))
    };
    for comment in &header_comments {
        result.push_str(&comment.content);
        if !comment.content.ends_with('\n') {
            result.push('\n');
        }
    }

    // 添加头部注释和第一个语句之间的空行
    if !header_comments.is_empty() && !module.items.is_empty() {
        result.push('\n');
    }

    let mut prev_end_line: usize = 0;

    for (i, stmt) in module.items.iter().enumerate() {
        let stmt_start_line = stmt.span.start.line;

        // 处理语句间的注释
        if i > 0 {
            let comments_between =
                format_comments_between(source_map, prev_end_line + 1, stmt_start_line);
            if !comments_between.is_empty() {
                result.push_str(&comments_between);
            }
        }

        // 添加语句间空行
        if i > 0 {
            // 检查原始代码中是否有空行
            let has_blank = has_blank_line_between(source_map, prev_end_line, stmt_start_line);
            if has_blank {
                // 最多保留一个空行
                if !result.ends_with("\n\n") {
                    result.push('\n');
                }
            }
        }

        // 格式化语句
        let stmt_str = format_stmt(&stmt.kind, ctx);
        result.push_str(&stmt_str);
        result.push('\n');

        // 处理行末注释
        if let Some(trailing) = source_map.trailing_comment_on_line(stmt.span.end.line) {
            // 只有当注释不在语句中时才添加
            if trailing.span.start.offset > stmt.span.end.offset {
                let last_newline = result.rfind('\n').unwrap_or(0);
                result.truncate(last_newline);
                result.push_str(&format!(" {}\n", trailing.content));
            }
        }

        prev_end_line = stmt.span.end.line;
    }

    // 确保文件以换行符结尾
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// 检查两个行号之间是否有空行
fn has_blank_line_between(
    source_map: &SourceMap,
    start_line: usize,
    end_line: usize,
) -> bool {
    for line_num in (start_line + 1)..end_line {
        if source_map.blank_lines.contains(&line_num) {
            return true;
        }
    }
    false
}
