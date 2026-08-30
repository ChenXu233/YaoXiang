//! 公共格式化函数

use crate::frontend::core::parser::ast::*;
use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::expr::{format_expr, format_block};

/// 格式化 if-else-if-else 结构
pub fn format_if(
    condition: &Expr,
    then_branch: &Block,
    else_if_branches: &[(Box<Expr>, Box<Block>)],
    else_branch: &Option<Box<Block>>,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = format!(
        "if {} {}",
        format_expr(condition, ctx, source_map),
        format_block(then_branch, ctx, source_map)
    );

    for (else_if_cond, else_if_body) in else_if_branches {
        result.push_str(&format!(
            " else if {} {}",
            format_expr(else_if_cond, ctx, source_map),
            format_block(else_if_body, ctx, source_map)
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
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut_str = if var_mut { "mut " } else { "" };
    format!(
        "for {}{} in {} {}",
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
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    format!(
        "while {} {}",
        format_expr(condition, ctx, source_map),
        format_block(body, ctx, source_map)
    )
}
