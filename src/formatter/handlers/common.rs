//! 公共格式化函数

use crate::frontend::core::parser::ast::*;
use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::expr::{format_expr, format_block};
use super::types::format_type;

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

/// 格式化泛型参数列表
pub fn format_generic_params(
    generic_params: &[GenericParam],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let items: Vec<String> = generic_params
        .iter()
        .map(|gp| {
            let constraints = if gp.constraints.is_empty() {
                String::new()
            } else {
                let cs: Vec<String> = gp
                    .constraints
                    .iter()
                    .map(|c| format_type(c, ctx, source_map))
                    .collect();
                format!(": {}", cs.join(" + "))
            };
            format!("{}{}", gp.name, constraints)
        })
        .collect();
    format!("({})", items.join(", "))
}

/// 重建泛型类型定义的参数签名：(T: Type, N: Int)
///
/// 与 format_generic_params（函数定义分支用，仅输出名字）不同，
/// 类型定义分支需要完整的类型标注。
pub fn format_generic_type_params(
    generic_params: &[GenericParam],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let items: Vec<String> = generic_params
        .iter()
        .map(|gp| {
            let annotation = match &gp.kind {
                GenericParamKind::Type => {
                    if gp.constraints.is_empty() {
                        "Type".to_string()
                    } else {
                        gp.constraints
                            .iter()
                            .map(|c| format_type(c, ctx, source_map))
                            .collect::<Vec<_>>()
                            .join(" + ")
                    }
                }
                GenericParamKind::Const { const_type } => format_type(const_type, ctx, source_map),
                GenericParamKind::Platform => {
                    if gp.constraints.is_empty() {
                        String::new()
                    } else {
                        gp.constraints
                            .iter()
                            .map(|c| format_type(c, ctx, source_map))
                            .collect::<Vec<_>>()
                            .join(" + ")
                    }
                }
            };
            if annotation.is_empty() {
                gp.name.clone()
            } else {
                format!("{}: {}", gp.name, annotation)
            }
        })
        .collect();
    format!("({})", items.join(", "))
}
