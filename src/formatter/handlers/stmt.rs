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
        StmtKind::Var {
            name,
            name_span: _,
            type_annotation,
            initializer,
            is_mut,
        } => format_var_decl(name, type_annotation, initializer, *is_mut, ctx, source_map),
        StmtKind::For {
            var,
            var_span: _,
            var_mut,
            iterable,
            body,
            label,
        } => super::common::format_for_loop(var, *var_mut, iterable, body, label, ctx, source_map),
        StmtKind::Binding {
            name,
            type_name,
            method_type,
            signature_params,
            type_annotation,
            params,
            body,
            is_pub,
            ..
        } => format_binding(
            name,
            type_name.as_deref(),
            method_type.as_ref(),
            signature_params,
            type_annotation.as_ref(),
            params,
            body,
            *is_pub,
            ctx,
            source_map,
        ),
        StmtKind::Use {
            path, items, alias, ..
        } => format_use(path, items, alias),
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
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => format_external_binding(type_name, method_name, binding, ctx, source_map),
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

/// 格式化变量声明
fn format_var_decl(
    name: &str,
    type_annotation: &Option<Type>,
    initializer: &Option<Box<Expr>>,
    is_mut: bool,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = String::new();

    if is_mut {
        result.push_str("mut ");
    }

    result.push_str(name);

    if let Some(ty) = type_annotation {
        result.push_str(": ");
        result.push_str(&format_type(ty, ctx, source_map));
    }

    if let Some(init) = initializer {
        result.push_str(" = ");
        result.push_str(&format_expr(init, ctx, source_map));
    }

    result
}

/// 格式化统一绑定语句 (函数/类型/方法)
#[allow(clippy::too_many_arguments)]
fn format_binding(
    name: &str,
    type_name: Option<&str>,
    method_type: Option<&Type>,
    signature_params: &[Param],
    type_annotation: Option<&Type>,
    params: &[Param],
    body: &[Stmt],
    is_pub: bool,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    // 方法绑定: Type.method: (Type, ...) -> ReturnType = (params) => body
    if let Some(ty_name) = type_name {
        if let Some(mt) = method_type {
            let params_str = format_params(params, ctx, source_map);
            let stmt_start = body.first().map(|s| s.span.start);
            let block_span = match stmt_start {
                Some(pos) if pos.line > 1 => crate::util::span::Span::new(
                    crate::util::span::Position::new(pos.line - 1, 0),
                    pos,
                ),
                _ => crate::util::span::Span::dummy(),
            };
            let body_block = Block {
                stmts: body.to_vec(),
                span: block_span,
            };
            return format!(
                "{}.{}: {} = {} => {}",
                ty_name,
                name,
                format_type(mt, ctx, source_map),
                params_str,
                format_block(&body_block, ctx, source_map)
            );
        }
    }

    // 类型定义: name: Type = { ... } 或 name: (T: Type, ...) -> Type = { ... }
    // 但排除函数类型 (Type::Fn)，函数类型应该格式化为函数定义
    if params.is_empty() {
        if let Some(ty) = type_annotation {
            // 检查是否是函数类型
            let is_fn_type = matches!(ty, Type::Fn { .. });
            if !is_fn_type {
                let signature = if signature_params.is_empty() {
                    "Type".to_string()
                } else {
                    format!(
                        "{} -> Type",
                        format_signature_params(signature_params, ctx, source_map)
                    )
                };
                return format!(
                    "{}: {} = {}",
                    name,
                    signature,
                    format_type(ty, ctx, source_map)
                );
            }
        }
    }

    // 函数定义: name: Type = (params) => body
    let pub_str = if is_pub { "pub " } else { "" };
    let type_str = if let Some(ty) = type_annotation {
        if matches!(ty, Type::Fn { .. }) {
            format!(
                ": {}",
                super::expr::format_fn_signature(signature_params, ty, params, ctx, source_map)
            )
        } else {
            format!(": {}", format_type(ty, ctx, source_map))
        }
    } else {
        String::new()
    };

    let stmt_start = body.first().map(|s| s.span.start);
    let block_span = match stmt_start {
        Some(pos) if pos.line > 1 => {
            crate::util::span::Span::new(crate::util::span::Position::new(pos.line - 1, 0), pos)
        }
        _ => crate::util::span::Span::dummy(),
    };
    let body_block = Block {
        stmts: body.to_vec(),
        span: block_span,
    };

    // 如果参数为空，直接输出 = { ... }，不输出 () =>
    if params.is_empty() {
        format!(
            "{}{}{} = {}",
            pub_str,
            name,
            type_str,
            format_block(&body_block, ctx, source_map)
        )
    } else {
        let params_str = format_params(params, ctx, source_map);
        format!(
            "{}{}{} = {} => {}",
            pub_str,
            name,
            type_str,
            params_str,
            format_block(&body_block, ctx, source_map)
        )
    }
}

/// 格式化 use 语句
fn format_use(
    path: &str,
    items: &Option<Vec<String>>,
    alias: &Option<Vec<String>>,
) -> String {
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

/// 格式化外部绑定
fn format_external_binding(
    type_name: &str,
    method_name: &str,
    binding: &BindingKind,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match binding {
        BindingKind::External {
            function,
            positions,
        } => {
            let pos_strs: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
            format!(
                "{}.{} = {}[{}]",
                type_name,
                method_name,
                function,
                pos_strs.join(", ")
            )
        }
        BindingKind::Anonymous {
            params,
            return_type,
            positions,
            body,
        } => {
            let params_str = format_params(params, ctx, source_map);
            let pos_strs: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
            format!(
                "{}.{}: ({} -> {})[{}] = ({} => {})",
                type_name,
                method_name,
                params_str,
                format_type(return_type, ctx, source_map),
                pos_strs.join(", "),
                params_str,
                format_expr(body, ctx, source_map)
            )
        }
        BindingKind::DefaultExternal { function } => {
            format!("{}.{} = {}", type_name, method_name, function)
        }
    }
}
