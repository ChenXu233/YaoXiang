//! 公共格式化函数

use crate::frontend::core::parser::ast::*;
use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::expr::{format_expr, format_block};

/// 格式化 if-elif-else 结构
pub fn format_if(
    condition: &Expr,
    then_branch: &Block,
    elif_branches: &[(Box<Expr>, Box<Block>)],
    else_branch: &Option<Box<Block>>,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = format!(
        "if {} {}",
        format_expr(condition, ctx, source_map),
        format_block(then_branch, ctx, source_map)
    );

    for (elif_cond, elif_body) in elif_branches {
        result.push_str(&format!(
            " elif {} {}",
            format_expr(elif_cond, ctx, source_map),
            format_block(elif_body, ctx, source_map)
        ));
    }

    if let Some(else_body) = else_branch {
        result.push_str(&format!(
            " else {}",
            format_block(else_body, ctx, source_map)
        ));
    }

    result
}

/// 格式化 for 循环
pub fn format_for_loop(
    var: &str,
    var_mut: bool,
    iterable: &Expr,
    body: &Block,
    label: &Option<String>,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let label_str = if let Some(l) = label {
        format!("{}: ", l)
    } else {
        String::new()
    };
    let mut_str = if var_mut { "mut " } else { "" };
    format!(
        "{}for {}{} in {} {}",
        label_str,
        mut_str,
        var,
        format_expr(iterable, ctx, source_map),
        format_block(body, ctx, source_map)
    )
}

/// 格式化 while 循环
pub fn format_while_loop(
    condition: &Expr,
    body: &Block,
    label: &Option<String>,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let label_str = if let Some(l) = label {
        format!("{}: ", l)
    } else {
        String::new()
    };
    format!(
        "{}while {} {}",
        label_str,
        format_expr(condition, ctx, source_map),
        format_block(body, ctx, source_map)
    )
}
