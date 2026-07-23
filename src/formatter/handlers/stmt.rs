//! 语句格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::expr::{format_block, format_expr, format_params, format_signature_params};
use super::types::format_type;

/// 格式化语句
pub fn format_stmt(
    kind: &StmtKind,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match kind {
        StmtKind::Expr(expr) => format_expr(expr, ctx, source_map),
        StmtKind::For {
            var,
            var_span: _,
            var_mut,
            iterable,
            body,
            label,
        } => super::common::format_for_loop(var, *var_mut, iterable, body, label, ctx, source_map),
        StmtKind::Use {
            path, items, alias, ..
        } => {
            let mut result = format!("use {}", path);
            if let Some(items) = items {
                if items.len() == 1 {
                    result.push_str("::");
                    result.push_str(&items[0]);
                } else {
                    result.push_str("::{ ");
                    result.push_str(&items.join(", "));
                    result.push_str(" }");
                }
            }
            if let Some(aliases) = alias {
                result.push_str(" as ");
                result.push_str(&aliases.join(", "));
            }
            result
        }
        StmtKind::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            span: _,
        } => super::common::format_if(
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ctx,
            source_map,
        ),
        StmtKind::Assign {
            target,
            type_annotation,
            signature_params,
            value,
            is_pub,
            is_mut,
            ..
        } => format_assign(
            target,
            type_annotation,
            signature_params,
            value,
            *is_pub,
            *is_mut,
            ctx,
            source_map,
        ),
        StmtKind::Error(_span) => "/* error */".to_string(),
        StmtKind::DestructureAssign { names, rhs, .. } => {
            format!(
                "{} = {}",
                names
                    .iter()
                    .map(|n| n.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                format_expr(rhs, ctx, source_map)
            )
        }
        StmtKind::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
                format!("return {}", format_expr(expr, ctx, source_map))
            } else {
                "return".to_string()
            }
        }
        StmtKind::TypeDefinition {
            name,
            signature_params,
            definition,
            is_pub,
        } => {
            let pub_prefix = if *is_pub { "pub " } else { "" };
            if signature_params.is_empty() {
                format!(
                    "{}{}: Type = {}",
                    pub_prefix,
                    name,
                    format_type(definition, ctx, source_map)
                )
            } else {
                format!(
                    "{}{}: {} -> Type = {}",
                    pub_prefix,
                    name,
                    format_signature_params(signature_params, ctx, source_map),
                    format_type(definition, ctx, source_map)
                )
            }
        }
    }
}

fn format_assign(
    target: &Expr,
    type_annotation: &Option<Type>,
    signature_params: &[Param],
    value: &Option<Box<Expr>>,
    is_pub: bool,
    is_mut: bool,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let pub_str = if is_pub {
        "pub "
    } else {
        if is_mut {
            "mut "
        } else {
            ""
        }
    };
    let target_str = format_expr(target, ctx, source_map);

    let lambda_params: &[Param] = match value.as_ref().map(|v| v.as_ref()) {
        Some(Expr::Lambda { params, .. }) => params,
        _ => &[],
    };
    let type_str = format_type_annotation(
        type_annotation,
        signature_params,
        lambda_params,
        ctx,
        source_map,
    );

    if let Some(val) = value {
        let val_str = match val.as_ref() {
            // Lambda value: 函数定义
            Expr::Lambda { params, body, .. } => {
                if params.is_empty() {
                    // 空参数: name = { body }（不加 () =>）
                    format_lambda_body(body, type_annotation.is_some(), ctx, source_map)
                } else {
                    // 有参数: name = (params) => { body }
                    let params_str = format_params(params, ctx, source_map);
                    format!(
                        "{} => {}",
                        params_str,
                        format_lambda_body(body, type_annotation.is_some(), ctx, source_map)
                    )
                }
            }
            // Block value: 直接输出块
            Expr::Block(block) => format_block(block, ctx, source_map),
            // 其他表达式
            other => format_expr(other, ctx, source_map),
        };
        format!("{}{}{} = {}", pub_str, target_str, type_str, val_str)
    } else {
        format!("{}{}{}", pub_str, target_str, type_str)
    }
}

/// 格式化 Lambda 函数体：单条 Return 语句去掉 return 关键字
///
/// 有类型标注时输出 `=> expr`（与无标注一致，确保 re-parse 后类型不变）；
/// 无类型标注时输出 `{ expr }`（保留块语义，避免歧义）。
fn format_lambda_body(
    body: &Block,
    has_type_annotation: bool,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    if body.stmts.len() == 1 {
        if let Stmt {
            kind: StmtKind::Return(Some(expr)),
            ..
        } = &body.stmts[0]
        {
            if has_type_annotation {
                return format_expr(expr, ctx, source_map);
            } else {
                return format!("{{ {} }}", format_expr(expr, ctx, source_map));
            }
        }
    }
    format_block(body, ctx, source_map)
}

/// 格式化类型标注字符串（含冒号前缀）
///
/// Fn 类型走 format_fn_signature，用 value_params 补全内层参数名；
/// 其他类型直接 format_type。无标注返回空串。
fn format_type_annotation(
    type_annotation: &Option<Type>,
    signature_params: &[Param],
    value_params: &[Param],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match type_annotation {
        Some(ty) if matches!(ty, Type::Fn { .. }) => format!(
            ": {}",
            super::expr::format_fn_signature(signature_params, ty, value_params, ctx, source_map)
        ),
        Some(ty) => format!(": {}", format_type(ty, ctx, source_map)),
        None => String::new(),
    }
}
