//! 模块格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::stmt::format_stmt;
use super::comment::format_comments_before;

/// 格式化模块
pub fn format_module(
    module: &Module,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = String::new();

    // 输出文件头注释
    // 头部注释同样要走账本：它与导入注释/语句前注释的区间可能重叠，重叠时只能输出一次。
    let header_comments = if module.items.is_empty() {
        source_map.claim_unemitted(source_map.comments_between_lines(1, usize::MAX))
    } else if let Some((first_import_start, _)) = source_map.import_line_range {
        source_map.claim_unemitted(
            source_map.comments_between_lines(1, first_import_start.saturating_sub(1)),
        )
    } else {
        let first_stmt_line = module.items.first().map(|s| s.span.start.line).unwrap_or(1);
        source_map.claim_unemitted(
            source_map.comments_between_lines(1, first_stmt_line.saturating_sub(1)),
        )
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
        // 导入注释与头部注释区间可能重叠（重排后的分组边界），同样走账本去重。
        let group =
            source_map.claim_unemitted(source_map.import_comment_groups[i].iter().collect());
        for comment in &group {
            result.push_str(&comment.content);
            if !comment.content.ends_with('\n') {
                result.push('\n');
            }
        }
        let stmt_str = format_stmt(&stmt.kind, ctx, source_map);
        result.push_str(&stmt_str);
        result.push('\n');
        source_map.append_trailing_comment(&mut result, stmt.span.end.line, stmt.span.end.offset);
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

    let mut prev_stmt_start_line = 0usize;
    for stmt in module.items.iter().skip(import_count) {
        let stmt_start_line = stmt.span.start.line;

        // 没有 import 时，第一个语句前的注释由 header 处理，跳过
        if prev_end_line > 0 {
            // 上界用语句**起点偏移**而非行号：与语句同行的行末注释属于该语句
            // （第 82 行的 append_trailing_comment 输出），不能在这里再输出一次。
            //
            // 下界用**上一条语句的起始行**而不是 prev_end_line + 1：lambda 单表达式体
            // （`f = (x) => x`）的体是解析器合成的 block，其内部语句 span 会落在后续 token
            // 上（实测会越过其后的注释行），据此设下界会把该注释整条跳过（注释丢失）。
            // 放宽下界不会造成重复输出——重复由 emitted_comments 账本兜底
            // （上一条语句内部的注释早已被块内输出点认领）。
            let comments_between =
                format_comments_before(source_map, prev_stmt_start_line, stmt.span.start.offset);
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
        source_map.append_trailing_comment(&mut result, stmt.span.end.line, stmt.span.end.offset);
        prev_stmt_start_line = stmt.span.start.line;
        prev_end_line = match &stmt.kind {
            // Binding 的 span.end 只覆盖声明头，不包含 body 块，
            // 故块结束行要从 body 最后一条语句算。
            //
            // 但**不能无条件 +1**：只有闭合 `}` 独占一行时，块结束行才是
            // 「最后一条语句所在行 + 1」。当块与最后一条语句同行时
            // （`name: T = (x) => { return x }`、`main: () -> Void = { b = 3 }`），
            // +1 会越过紧随其后的那一行，把该行上的注释跳过（注释丢失），
            // 或在空行判定里把空行吃掉（空行侵蚀）——两者都会破坏格式化幂等性。
            StmtKind::Assign { value: Some(v), .. } => {
                let body_block = match v.as_ref() {
                    Expr::Lambda { body, .. } => Some(body.as_ref()),
                    Expr::Block(block) => Some(block),
                    _ => None,
                };
                match body_block {
                    Some(block) if !block.stmts.is_empty() => {
                        let last_line = block.stmts.last().unwrap().span.end.line;
                        if source_map.next_line_starts_with_block_close(last_line) {
                            last_line + 1
                        } else {
                            last_line
                        }
                    }
                    _ => stmt.span.end.line,
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
