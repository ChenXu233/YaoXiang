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

    // 输出文件头注释
    let header_comments = if module.items.is_empty() {
        source_map.comments_between_lines(1, usize::MAX)
    } else if let Some((first_import_start, _)) = source_map.import_line_range {
        source_map.comments_between_lines(1, first_import_start.saturating_sub(1))
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
    if !header_comments.is_empty() && !module.items.is_empty() {
        result.push('\n');
    }

    // Phase 1: 导入语句 — 直接用 rebuild 算好的分组，不走 stale span
    let import_count = source_map.import_comment_groups.len();
    for (i, stmt) in module.items.iter().enumerate().take(import_count) {
        for comment in &source_map.import_comment_groups[i] {
            result.push_str(&comment.content);
            if !comment.content.ends_with('\n') {
                result.push('\n');
            }
        }
        let stmt_str = format_stmt(&stmt.kind, ctx, source_map);
        result.push_str(&stmt_str);
        result.push('\n');
        if let Some(trailing) = source_map.trailing_comment_on_line(stmt.span.end.line) {
            if trailing.span.start.offset > stmt.span.end.offset {
                let last_newline = result.rfind('\n').unwrap_or(0);
                result.truncate(last_newline);
                result.push_str(&format!(" {}\n", trailing.content));
            }
        }
    }

    // Phase 2: 非导入语句 — prev_end_line 从 import 区域末尾开始
    let import_region_end = source_map
        .import_line_range
        .map(|(_, end)| end)
        .unwrap_or(0);
    let mut prev_end_line = if import_count > 0 {
        import_region_end
    } else {
        0
    };

    for stmt in module.items.iter().skip(import_count) {
        let stmt_start_line = stmt.span.start.line;

        // 没有 import 时，第一个语句前的注释由 header 处理，跳过
        if prev_end_line > 0 {
            let comments_between =
                format_comments_between(source_map, prev_end_line + 1, stmt_start_line);
            if !comments_between.is_empty() {
                result.push_str(&comments_between);
            }
            let has_blank = has_blank_line_between(source_map, prev_end_line, stmt_start_line);
            if has_blank && !result.ends_with("\n\n") {
                result.push('\n');
            }
        }

        let stmt_str = format_stmt(&stmt.kind, ctx, source_map);
        result.push_str(&stmt_str);
        result.push('\n');
        if let Some(trailing) = source_map.trailing_comment_on_line(stmt.span.end.line) {
            if trailing.span.start.offset > stmt.span.end.offset {
                let last_newline = result.rfind('\n').unwrap_or(0);
                result.truncate(last_newline);
                result.push_str(&format!(" {}\n", trailing.content));
            }
        }
        prev_end_line = match &stmt.kind {
            // Binding 的 span.end 只覆盖声明头，不包含 body 块
            // 实际输出到 `}` 所在行，需要从 body 最后一个语句计算
            StmtKind::Assign { value: Some(v), .. } => {
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    if !body.stmts.is_empty() {
                        body.stmts.last().unwrap().span.end.line + 1
                    } else {
                        stmt.span.end.line
                    }
                } else if let Expr::Block(block) = v.as_ref() {
                    if !block.stmts.is_empty() {
                        block.stmts.last().unwrap().span.end.line + 1
                    } else {
                        stmt.span.end.line
                    }
                } else {
                    stmt.span.end.line
                }
            }
            _ => stmt.span.end.line,
        };
    }

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
