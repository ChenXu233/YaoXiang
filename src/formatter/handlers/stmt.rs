//! 语句格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::super::source_map::SourceMap;
use super::expr::{format_block, format_expr, format_params};
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
            generic_params,
            type_annotation,
            eval,
            params,
            body,
            is_pub,
        } => format_binding(
            name,
            type_name.as_deref(),
            method_type.as_ref(),
            generic_params,
            type_annotation.as_ref(),
            eval,
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
        result.push_str(&format_type(ty, source_map));
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
    generic_params: &[GenericParam],
    type_annotation: Option<&Type>,
    eval: &Option<EvalMode>,
    params: &[Param],
    body: &(Vec<Stmt>, Option<Box<Expr>>),
    is_pub: bool,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    // 方法绑定: Type.method: (Type, ...) -> ReturnType = (params) => body
    if let Some(ty_name) = type_name {
        if let Some(mt) = method_type {
            let params_str = format_params(params, ctx, source_map);
            let body_block = Block {
                stmts: body.0.clone(),
                expr: body.1.clone(),
                span: crate::util::span::Span::dummy(),
            };
            return format!(
                "{}.{}: {} = {} => {}",
                ty_name,
                name,
                format_type(mt, source_map),
                params_str,
                format_block(&body_block, ctx, source_map)
            );
        }
    }

    // 类型定义: name: Type = { ... }
    if params.is_empty() {
        if let Some(ty) = type_annotation {
            let generics = if generic_params.is_empty() {
                String::new()
            } else {
                super::common::format_generic_params(generic_params, source_map)
            };
            return format!(
                "{}{}: Type = {}",
                name,
                generics,
                format_type(ty, source_map)
            );
        }
    }

    // 函数定义: name: Type = (params) => body
    let pub_str = if is_pub { "pub " } else { "" };
    let generics = if generic_params.is_empty() {
        String::new()
    } else {
        super::common::format_generic_params(generic_params, source_map)
    };

    let type_str = if let Some(ty) = type_annotation {
        format!(": {}", format_type(ty, source_map))
    } else {
        String::new()
    };

    let eval_str = if type_annotation.is_some() {
        match eval {
            Some(EvalMode::Block) => " @block",
            Some(EvalMode::Auto) => " @auto",
            Some(EvalMode::Eager) => " @eager",
            None => "",
        }
        .to_string()
    } else {
        String::new()
    };

    let params_str = format_params(params, ctx, source_map);

    let body_block = Block {
        stmts: body.0.clone(),
        expr: body.1.clone(),
        span: crate::util::span::Span::dummy(),
    };

    format!(
        "{}{}{}{}{} = {} => {}",
        pub_str,
        name,
        generics,
        type_str,
        eval_str,
        params_str,
        format_block(&body_block, ctx, source_map)
    )
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
                format_type(return_type, source_map),
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
