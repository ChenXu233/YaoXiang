//! 语句格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::expr::{format_block, format_expr, format_params};
use super::types::format_type;

/// 格式化语句
pub fn format_stmt(
    kind: &StmtKind,
    ctx: &FormatContext,
) -> String {
    match kind {
        StmtKind::Expr(expr) => format_expr(expr, ctx),
        StmtKind::Var {
            name,
            name_span: _,
            type_annotation,
            initializer,
            is_mut,
        } => format_var_decl(name, type_annotation, initializer, *is_mut, ctx),
        StmtKind::For {
            var,
            var_span: _,
            var_mut,
            iterable,
            body,
            label,
        } => format_for_stmt(var, *var_mut, iterable, body, label, ctx),
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
        } => format_if_stmt(condition, then_branch, elif_branches, else_branch, ctx),
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => format_external_binding(type_name, method_name, binding, ctx),
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
) -> String {
    let mut result = String::new();

    if is_mut {
        result.push_str("mut ");
    }

    result.push_str(name);

    if let Some(ty) = type_annotation {
        result.push_str(": ");
        result.push_str(&format_type(ty));
    }

    if let Some(init) = initializer {
        result.push_str(" = ");
        result.push_str(&format_expr(init, ctx));
    }

    result
}

/// 格式化 for 语句
fn format_for_stmt(
    var: &str,
    var_mut: bool,
    iterable: &Expr,
    body: &Block,
    label: &Option<String>,
    ctx: &FormatContext,
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
        format_expr(iterable, ctx),
        format_block(body, ctx)
    )
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
) -> String {
    // 方法绑定: Type.method: (Type, ...) -> ReturnType = (params) => body
    if let Some(ty_name) = type_name {
        if let Some(mt) = method_type {
            let params_str = format_params(params);
            let body_block = Block {
                stmts: body.0.clone(),
                expr: body.1.clone(),
                span: crate::util::span::Span::dummy(),
            };
            return format!(
                "{}.{}: {} = ({}) => {}",
                ty_name,
                name,
                format_type(mt),
                params_str,
                format_block(&body_block, ctx)
            );
        }
    }

    // 类型定义: name: Type = { ... }
    if params.is_empty() {
        if let Some(ty) = type_annotation {
            let generics = if generic_params.is_empty() {
                String::new()
            } else {
                let items: Vec<String> = generic_params
                    .iter()
                    .map(|gp| {
                        let constraints = if gp.constraints.is_empty() {
                            String::new()
                        } else {
                            let cs: Vec<String> = gp.constraints.iter().map(format_type).collect();
                            format!(": {}", cs.join(" + "))
                        };
                        format!("{}{}", gp.name, constraints)
                    })
                    .collect();
                format!("({})", items.join(", "))
            };
            return format!("{}{}: Type = {}", name, generics, format_type(ty));
        }
    }

    // 函数定义: name: Type = (params) => body
    let pub_str = if is_pub { "pub " } else { "" };
    let generics = if generic_params.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = generic_params
            .iter()
            .map(|gp| {
                let constraints = if gp.constraints.is_empty() {
                    String::new()
                } else {
                    let cs: Vec<String> = gp.constraints.iter().map(format_type).collect();
                    format!(": {}", cs.join(" + "))
                };
                format!("{}{}", gp.name, constraints)
            })
            .collect();
        format!("({})", items.join(", "))
    };

    let type_str = if let Some(ty) = type_annotation {
        format!(": {}", format_type(ty))
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

    let params_str = format_params(params);

    let body_block = Block {
        stmts: body.0.clone(),
        expr: body.1.clone(),
        span: crate::util::span::Span::dummy(),
    };

    format!(
        "{}{}{}{}{} = ({}) => {}",
        pub_str,
        name,
        generics,
        type_str,
        eval_str,
        params_str,
        format_block(&body_block, ctx)
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

/// 格式化 if 语句
fn format_if_stmt(
    condition: &Expr,
    then_branch: &Block,
    elif_branches: &[(Box<Expr>, Box<Block>)],
    else_branch: &Option<Box<Block>>,
    ctx: &FormatContext,
) -> String {
    let mut result = format!(
        "if {} {}",
        format_expr(condition, ctx),
        format_block(then_branch, ctx)
    );

    for (elif_cond, elif_body) in elif_branches {
        result.push_str(&format!(
            " elif {} {}",
            format_expr(elif_cond, ctx),
            format_block(elif_body, ctx)
        ));
    }

    if let Some(else_body) = else_branch {
        result.push_str(&format!(" else {}", format_block(else_body, ctx)));
    }

    result
}

/// 格式化外部绑定
fn format_external_binding(
    type_name: &str,
    method_name: &str,
    binding: &BindingKind,
    ctx: &FormatContext,
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
            let params_str = format_params(params);
            let pos_strs: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
            format!(
                "{}.{}: (({}) -> {})[{}] = (({}) => {})",
                type_name,
                method_name,
                params_str,
                format_type(return_type),
                pos_strs.join(", "),
                params_str,
                format_expr(body, ctx)
            )
        }
        BindingKind::DefaultExternal { function } => {
            format!("{}.{} = {}", type_name, method_name, function)
        }
    }
}
