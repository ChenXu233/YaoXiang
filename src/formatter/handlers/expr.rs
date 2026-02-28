//! 表达式格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;

/// 格式化表达式
pub fn format_expr(
    expr: &Expr,
    ctx: &FormatContext,
) -> String {
    match expr {
        Expr::Lit(lit, _span) => format_literal(lit),
        Expr::Var(name, _span) => name.clone(),
        Expr::BinOp {
            op,
            left,
            right,
            span: _,
        } => format_binop(op, left, right, ctx),
        Expr::UnOp {
            op,
            expr: inner,
            span: _,
        } => format_unop(op, inner, ctx),
        Expr::Call {
            func,
            args,
            named_args,
            span: _,
        } => format_call(func, args, named_args, ctx),
        Expr::FnDef {
            name,
            params,
            return_type,
            body,
            is_async,
            span: _,
        } => format_fn_def(name, params, return_type, body, *is_async, ctx),
        Expr::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            span: _,
        } => format_if_expr(condition, then_branch, elif_branches, else_branch, ctx),
        Expr::Match {
            expr: match_expr,
            arms,
            span: _,
        } => format_match_expr(match_expr, arms, ctx),
        Expr::While {
            condition,
            body,
            label,
            span: _,
        } => format_while(condition, body, label, ctx),
        Expr::For {
            var,
            var_mut,
            iterable,
            body,
            label,
            span: _,
        } => format_for(var, *var_mut, iterable, body, label, ctx),
        Expr::Block(block) => format_block(block, ctx),
        Expr::Return(expr_opt, _span) => {
            if let Some(e) = expr_opt {
                format!("return {}", format_expr(e, ctx))
            } else {
                "return".to_string()
            }
        }
        Expr::Break(label, _span) => {
            if let Some(l) = label {
                format!("break {}", l)
            } else {
                "break".to_string()
            }
        }
        Expr::Continue(label, _span) => {
            if let Some(l) = label {
                format!("continue {}", l)
            } else {
                "continue".to_string()
            }
        }
        Expr::Cast {
            expr: inner,
            target_type,
            span: _,
        } => {
            format!(
                "{} as {}",
                format_expr(inner, ctx),
                super::types::format_type(target_type)
            )
        }
        Expr::Tuple(exprs, _span) => {
            let items: Vec<String> = exprs.iter().map(|e| format_expr(e, ctx)).collect();
            format!("({})", items.join(", "))
        }
        Expr::List(exprs, _span) => {
            let items: Vec<String> = exprs.iter().map(|e| format_expr(e, ctx)).collect();
            format!("[{}]", items.join(", "))
        }
        Expr::ListComp {
            element,
            var,
            iterable,
            condition,
            span: _,
        } => {
            let base = format!(
                "[{} for {} in {}",
                format_expr(element, ctx),
                var,
                format_expr(iterable, ctx)
            );
            if let Some(cond) = condition {
                format!("{} if {}]", base, format_expr(cond, ctx))
            } else {
                format!("{}]", base)
            }
        }
        Expr::Dict(pairs, _span) => {
            if pairs.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = pairs
                    .iter()
                    .map(|(k, v)| format!("{}: {}", format_expr(k, ctx), format_expr(v, ctx)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
        }
        Expr::Index {
            expr: inner,
            index,
            span: _,
        } => {
            format!("{}[{}]", format_expr(inner, ctx), format_expr(index, ctx))
        }
        Expr::FieldAccess {
            expr: inner,
            field,
            span: _,
        } => {
            format!("{}.{}", format_expr(inner, ctx), field)
        }
        Expr::Try {
            expr: inner,
            span: _,
        } => {
            format!("{}?", format_expr(inner, ctx))
        }
        Expr::Ref {
            expr: inner,
            span: _,
        } => {
            format!("ref {}", format_expr(inner, ctx))
        }
        Expr::Unsafe { body, span: _ } => {
            format!("unsafe {}", format_block(body, ctx))
        }
        Expr::Lambda {
            params,
            body,
            span: _,
        } => format_lambda(params, body, ctx),
        Expr::FString { segments, span: _ } => format_fstring(segments, ctx),
        Expr::Error(_span) => "/* error */".to_string(),
    }
}

/// 格式化字面量
fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => n.to_string(),
        Literal::Float(f) => {
            let s = f.to_string();
            // 确保浮点数总是有小数点
            if s.contains('.') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        Literal::Bool(b) => b.to_string(),
        Literal::Char(c) => format!("'{}'", c),
        Literal::String(s) => format!("\"{}\"", s),
    }
}

/// 格式化二元运算
fn format_binop(
    op: &BinOp,
    left: &Expr,
    right: &Expr,
    ctx: &FormatContext,
) -> String {
    let op_str = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Neq => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Range => "..",
        BinOp::Assign => "=",
    };
    format!(
        "{} {} {}",
        format_expr(left, ctx),
        op_str,
        format_expr(right, ctx)
    )
}

/// 格式化一元运算
fn format_unop(
    op: &UnOp,
    inner: &Expr,
    ctx: &FormatContext,
) -> String {
    match op {
        UnOp::Neg => format!("-{}", format_expr(inner, ctx)),
        UnOp::Pos => format!("+{}", format_expr(inner, ctx)),
        UnOp::Not => format!("!{}", format_expr(inner, ctx)),
        UnOp::Deref => format!("*{}", format_expr(inner, ctx)),
    }
}

/// 格式化函数调用
fn format_call(
    func: &Expr,
    args: &[Expr],
    named_args: &[(String, Expr)],
    ctx: &FormatContext,
) -> String {
    let func_str = format_expr(func, ctx);
    let mut all_args: Vec<String> = args.iter().map(|a| format_expr(a, ctx)).collect();
    for (name, expr) in named_args {
        all_args.push(format!("{}={}", name, format_expr(expr, ctx)));
    }

    let single_line = format!("{}({})", func_str, all_args.join(", "));

    // 如果单行不超过行宽，使用单行格式
    if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
        single_line
    } else {
        // 多行格式
        let indent = ctx.indent_str();
        let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));
        let mut result = format!("{}(\n", func_str);
        for (i, arg) in all_args.iter().enumerate() {
            result.push_str(&inner_indent);
            result.push_str(arg);
            if i < all_args.len() - 1 {
                result.push(',');
            } else {
                result.push(',');
            }
            result.push('\n');
        }
        result.push_str(&indent);
        result.push(')');
        result
    }
}

/// 格式化函数定义（表达式形式）
fn format_fn_def(
    name: &str,
    params: &[Param],
    return_type: &Option<Type>,
    body: &Block,
    is_async: bool,
    ctx: &FormatContext,
) -> String {
    let async_prefix = if is_async { "async " } else { "" };
    let params_str = format_params(params);
    let ret_str = if let Some(ty) = return_type {
        format!(" -> {}", super::types::format_type(ty))
    } else {
        String::new()
    };
    format!(
        "{}fn {}({}){}  {}",
        async_prefix,
        name,
        params_str,
        ret_str,
        format_block(body, ctx)
    )
}

/// 格式化参数列表
pub fn format_params(params: &[Param]) -> String {
    let items: Vec<String> = params
        .iter()
        .map(|p| {
            let mut s = String::new();
            if p.is_mut {
                s.push_str("mut ");
            }
            s.push_str(&p.name);
            if let Some(ty) = &p.ty {
                s.push_str(": ");
                s.push_str(&super::types::format_type(ty));
            }
            s
        })
        .collect();
    items.join(", ")
}

/// 格式化 if 表达式
fn format_if_expr(
    condition: &Expr,
    then_branch: &Block,
    elif_branches: &[(Box<Expr>, Box<Block>)],
    else_branch: &Option<Box<Block>>,
    ctx: &FormatContext,
) -> String {
    let indent = ctx.indent_str();
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

    let _ = indent;
    result
}

/// 格式化 match 表达式
fn format_match_expr(
    match_expr: &Expr,
    arms: &[MatchArm],
    ctx: &FormatContext,
) -> String {
    let indent = ctx.indent_str();
    let mut inner_ctx = ctx.clone();
    inner_ctx.indent();
    let inner_indent = inner_ctx.indent_str();

    let mut result = format!("match {} {{\n", format_expr(match_expr, ctx));

    for arm in arms {
        let pattern_str = format_pattern(&arm.pattern);
        let body_str = format_block_inline(&arm.body, &inner_ctx);
        result.push_str(&format!(
            "{}{} => {},\n",
            inner_indent, pattern_str, body_str
        ));
    }

    result.push_str(&indent);
    result.push('}');
    result
}

/// 格式化模式
pub fn format_pattern(pat: &Pattern) -> String {
    match pat {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Identifier(name) => name.clone(),
        Pattern::Literal(lit) => format_literal(lit),
        Pattern::Tuple(pats) => {
            let items: Vec<String> = pats.iter().map(format_pattern).collect();
            format!("({})", items.join(", "))
        }
        Pattern::Struct { name, fields } => {
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(field_name, is_mut, pat)| {
                    let mut_str = if *is_mut { "mut " } else { "" };
                    format!("{}{}: {}", mut_str, field_name, format_pattern(pat))
                })
                .collect();
            format!("{} {{ {} }}", name, field_strs.join(", "))
        }
        Pattern::Union {
            name,
            variant,
            pattern,
        } => {
            if let Some(pat) = pattern {
                format!("{}::{} ({})", name, variant, format_pattern(pat))
            } else {
                format!("{}::{}", name, variant)
            }
        }
        Pattern::Or(pats) => {
            let items: Vec<String> = pats.iter().map(format_pattern).collect();
            items.join(" | ")
        }
        Pattern::Guard { pattern, condition } => {
            let ctx = FormatContext::new(super::super::options::FormatOptions::default());
            format!(
                "{} if {}",
                format_pattern(pattern),
                format_expr(condition, &ctx)
            )
        }
    }
}

/// 格式化 while 循环
fn format_while(
    condition: &Expr,
    body: &Block,
    label: &Option<String>,
    ctx: &FormatContext,
) -> String {
    let label_str = if let Some(l) = label {
        format!("{}: ", l)
    } else {
        String::new()
    };
    format!(
        "{}while {} {}",
        label_str,
        format_expr(condition, ctx),
        format_block(body, ctx)
    )
}

/// 格式化 for 循环
fn format_for(
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

/// 格式化代码块
pub fn format_block(
    block: &Block,
    ctx: &FormatContext,
) -> String {
    let mut inner_ctx = ctx.clone();
    inner_ctx.indent();
    let inner_indent = inner_ctx.indent_str();
    let outer_indent = ctx.indent_str();

    let mut result = "{\n".to_string();

    for stmt in &block.stmts {
        let stmt_str = super::stmt::format_stmt(&stmt.kind, &inner_ctx);
        result.push_str(&inner_indent);
        result.push_str(&stmt_str);
        result.push('\n');
    }

    if let Some(expr) = &block.expr {
        let expr_str = format_expr(expr, &inner_ctx);
        result.push_str(&inner_indent);
        result.push_str(&expr_str);
        result.push('\n');
    }

    result.push_str(&outer_indent);
    result.push('}');
    result
}

/// 格式化内联块（用于 match arm 等场景）
fn format_block_inline(
    block: &Block,
    ctx: &FormatContext,
) -> String {
    // 如果块只有一个表达式，返回内联形式
    if block.stmts.is_empty() {
        if let Some(expr) = &block.expr {
            return format_expr(expr, ctx);
        }
    }
    format_block(block, ctx)
}

/// 格式化 lambda 表达式
fn format_lambda(
    params: &[Param],
    body: &Block,
    ctx: &FormatContext,
) -> String {
    let params_str = format_params(params);
    // 如果 body 只有一个表达式，使用简洁形式
    if body.stmts.is_empty() {
        if let Some(expr) = &body.expr {
            return format!("({}) => {}", params_str, format_expr(expr, ctx));
        }
    }
    format!("({}) => {}", params_str, format_block(body, ctx))
}

/// 格式化 f-string
fn format_fstring(
    segments: &[FStringSegment],
    ctx: &FormatContext,
) -> String {
    let mut result = "f\"".to_string();
    for seg in segments {
        match seg {
            FStringSegment::Text(text) => result.push_str(text),
            FStringSegment::Interpolation { expr, format_spec } => {
                result.push('{');
                result.push_str(&format_expr(expr, ctx));
                if let Some(spec) = format_spec {
                    result.push(':');
                    result.push_str(spec);
                }
                result.push('}');
            }
        }
    }
    result.push('"');
    result
}
