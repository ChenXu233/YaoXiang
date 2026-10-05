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
        } => super::common::format_for_loop(var, *var_mut, iterable, body, ctx, source_map),
        StmtKind::Use {
            path,
            items,
            alias,
            item_aliases,
            ..
        } => {
            let mut result = format!("use {}", path);
            if let Some(items) = items {
                // 内联别名（#245）：item as alias。
                // 统一输出 `path.{ item }` 点形式：双冒号 `path::item` 无法被
                // parse_use_path 回解析（`::` 非路径分隔符），重解析会静默吞掉条目。
                let render = |i: usize, name: &str| -> String {
                    match item_aliases
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .and_then(|a| a.as_ref())
                    {
                        Some(a) => format!("{} as {}", name, a),
                        None => name.to_string(),
                    }
                };
                result.push_str(".{ ");
                let rendered: Vec<String> = items
                    .iter()
                    .enumerate()
                    .map(|(i, name)| render(i, name))
                    .collect();
                result.push_str(&rendered.join(", "));
                result.push_str(" }");
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
            else_if_branches,
            else_branch,
            span: _,
        } => super::common::format_if(
            condition,
            then_branch,
            else_if_branches,
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

#[allow(clippy::too_many_arguments)]
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

    let type_str = format_type_annotation(type_annotation, signature_params, ctx, source_map);

    if let Some(val) = value {
        let val_str = match val.as_ref() {
            // Lambda value: 函数定义。零参分叉：
            // 有 Fn 标注 → `= { body }` 规范形——parser 会把标注 Fn 的块值包成
            //   零参 Lambda，`main: () -> Void = { ... }` 与 `() => { ... }` 收敛于
            //   同一 AST，输出必须取规范形；
            // 无标注 → 恒保留 `() =>` 前缀——落成块值后不可调用
            //   （`f = () => 42` 物化成 `f = { 42 }` 后 f() 报 E1065，#424 探针）。
            Expr::Lambda { params, body, .. } => {
                if params.is_empty() {
                    if type_annotation.is_some() {
                        format_zero_param_body(body, ctx, source_map)
                    } else {
                        format!("() => {}", format_lambda_body(body, ctx, source_map))
                    }
                } else {
                    let params_str = format_params(params, ctx, source_map);
                    format!(
                        "{} => {}",
                        params_str,
                        format_lambda_body(body, ctx, source_map)
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

/// 格式化 Lambda 函数体：单条 Return 语句去掉 return 关键字（§12.1，#424）
///
/// 裸 body `=> expr` 由 parser 包成 `Block[Return(expr)]`（pratt/led.rs 的语法糖），
/// 带花括号的 body 其尾表达式是 `Expr` 语句——因此单条 `Return` ⇔ 源码裸 body，
/// 恒输出裸表达式，re-parse 后 AST 不变。
fn format_lambda_body(
    body: &Block,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    if let Some(expr) = single_return_expr(body) {
        return format_expr(expr, ctx, source_map);
    }
    format_block(body, ctx, source_map)
}

/// 零参 Lambda 在有 Fn 标注时的规范形：`= { body }`（不加 `() =>` 前缀）
///
/// parser 会把标注 Fn 的块值包成零参 Lambda，两种源码形态收敛于同一 AST，
/// 输出取 `= { body }` 规范形；单条 Return 仍去 return 关键字
/// （与 format_lambda_body 同一归一化）。
fn format_zero_param_body(
    body: &Block,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    if let Some(expr) = single_return_expr(body) {
        return format!("{{ {} }}", format_expr(expr, ctx, source_map));
    }
    format_block(body, ctx, source_map)
}

/// 单语句块若恰为一条 `return expr`，返回该表达式
fn single_return_expr(body: &Block) -> Option<&Expr> {
    if body.stmts.len() == 1 {
        if let Stmt {
            kind: StmtKind::Return(Some(expr)),
            ..
        } = &body.stmts[0]
        {
            return Some(expr.as_ref());
        }
    }
    None
}

/// 格式化类型标注字符串（含冒号前缀）
///
/// Fn 类型走 format_fn_signature；其他类型直接 format_type。无标注返回空串。
fn format_type_annotation(
    type_annotation: &Option<Type>,
    signature_params: &[Param],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match type_annotation {
        Some(ty) if matches!(ty, Type::Fn { .. }) => format!(
            ": {}",
            super::expr::format_fn_signature(signature_params, ty, ctx, source_map)
        ),
        Some(ty) => format!(": {}", format_type(ty, ctx, source_map)),
        None => String::new(),
    }
}
