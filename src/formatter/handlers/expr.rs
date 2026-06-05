//! 表达式格式化处理器

use crate::frontend::core::parser::ast::*;

use super::super::context::FormatContext;
use super::super::source_map::SourceMap;

/// 格式化表达式
pub fn format_expr(
    expr: &Expr,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match expr {
        Expr::Lit(lit, _span) => format_literal(lit, ctx),
        Expr::Var(name, _span) => name.clone(),
        Expr::BinOp {
            op,
            left,
            right,
            span: _,
        } => format_binop(op, left, right, ctx, source_map),
        Expr::UnOp {
            op,
            expr: inner,
            span: _,
        } => format_unop(op, inner, ctx, source_map),
        Expr::Call {
            func,
            args,
            named_args,
            span: _,
        } => format_call(func, args, named_args, ctx, source_map),
        Expr::FnDef {
            name,
            params,
            return_type,
            body,
            span: _,
        } => format_fn_def(name, params, return_type, body, ctx, source_map),
        Expr::If {
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
        Expr::Match {
            expr: match_expr,
            arms,
            span: _,
        } => format_match_expr(match_expr, arms, ctx, source_map),
        Expr::While {
            condition,
            body,
            label,
            span: _,
        } => super::common::format_while_loop(condition, body, label, ctx, source_map),
        Expr::For {
            var,
            var_mut,
            iterable,
            body,
            label,
            span: _,
        } => super::common::format_for_loop(var, *var_mut, iterable, body, label, ctx, source_map),
        Expr::Block(block) => format_block(block, ctx, source_map),
        Expr::Return(expr_opt, _span) => {
            if let Some(e) = expr_opt {
                format!("return {}", format_expr(e, ctx, source_map))
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
                format_expr(inner, ctx, source_map),
                super::types::format_type(target_type, source_map)
            )
        }
        Expr::Tuple(exprs, _span) => {
            let items: Vec<String> = exprs
                .iter()
                .map(|e| format_expr(e, ctx, source_map))
                .collect();
            format!("({})", items.join(", "))
        }
        Expr::List(exprs, _span) => format_list(exprs, ctx, source_map),
        Expr::ListComp {
            element,
            var,
            iterable,
            condition,
            span: _,
        } => {
            let base = format!(
                "[{} for {} in {}",
                format_expr(element, ctx, source_map),
                var,
                format_expr(iterable, ctx, source_map)
            );
            if let Some(cond) = condition {
                format!("{} if {}]", base, format_expr(cond, ctx, source_map))
            } else {
                format!("{}]", base)
            }
        }
        Expr::Dict(pairs, _span) => format_dict(pairs, ctx, source_map),
        Expr::Index {
            expr: inner,
            index,
            span: _,
        } => {
            format!(
                "{}[{}]",
                format_expr(inner, ctx, source_map),
                format_expr(index, ctx, source_map)
            )
        }
        Expr::FieldAccess {
            expr: inner,
            field,
            span: _,
        } => format_field_access(inner, field, ctx, source_map),
        Expr::Try {
            expr: inner,
            span: _,
        } => {
            format!("{}?", format_expr(inner, ctx, source_map))
        }
        Expr::Ref {
            expr: inner,
            span: _,
        } => {
            format!("ref {}", format_expr(inner, ctx, source_map))
        }
        Expr::Unsafe { body, span: _ } => {
            format!("unsafe {}", format_block(body, ctx, source_map))
        }
        Expr::Spawn { body, .. } => {
            format!("spawn {}", format_block(body, ctx, source_map))
        }
        Expr::Lambda {
            params,
            body,
            span: _,
        } => format_lambda(params, body, ctx, source_map),
        Expr::FString { segments, span: _ } => format_fstring(segments, ctx, source_map),
        Expr::Borrow {
            mutable,
            expr: inner,
            span: _,
        } => {
            if *mutable {
                format!("&mut {}", format_expr(inner, ctx, source_map))
            } else {
                format!("&{}", format_expr(inner, ctx, source_map))
            }
        }
        Expr::Error(_span) => "/* error */".to_string(),
    }
}

/// 格式化字面量
pub(crate) fn format_literal(
    lit: &Literal,
    ctx: &FormatContext,
) -> String {
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
        Literal::Char(c) => {
            let escaped = match c {
                '\'' => "\\'".to_string(),
                '\\' => "\\\\".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                '\0' => "\\0".to_string(),
                _ => c.to_string(),
            };
            format!("'{}'", escaped)
        }
        Literal::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
                .replace('\0', "\\0");
            if ctx.options.single_quote {
                let escaped = escaped.replace('\'', "\\'");
                format!("'{}'", escaped)
            } else {
                let escaped = escaped.replace('"', "\\\"");
                format!("\"{}\"", escaped)
            }
        }
    }
}

/// 格式化二元运算
pub(crate) fn format_binop(
    op: &BinOp,
    left: &Expr,
    right: &Expr,
    ctx: &FormatContext,
    source_map: &SourceMap,
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

    let left_str = format_expr(left, ctx, source_map);
    let right_str = format_expr(right, ctx, source_map);

    // 计算完整表达式的预估长度
    let total_len = left_str.len() + op_str.len() + right_str.len() + ctx.indent_width();

    // 如果不超过行宽，使用单行格式
    if total_len <= ctx.options.line_width {
        return format!("{} {} {}", left_str, op_str, right_str);
    }

    // 需要换行时，根据运算符优先级决定换行位置
    let indent = ctx.indent_str();
    let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));

    // §2.2 换行策略优先级：
    // 1. 低优先级运算符后（+、-、||、&&、=）
    // 2. 函数参数列表（已在 format_call 中实现）
    // 3. 列表/字典元素（已在 format_list/format_dict 中实现）
    // 4. 高优先级运算符后（*、/、%、==、!=）

    let is_low_priority = matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Or | BinOp::And | BinOp::Assign
    );

    if is_low_priority {
        // 低优先级运算符：运算符放在新行行首
        format!("{}\n{}{} {}", left_str, inner_indent, op_str, right_str)
    } else {
        // 高优先级运算符：运算符放在新行行首
        format!("{}\n{}{} {}", left_str, inner_indent, op_str, right_str)
    }
}

/// 格式化一元运算
fn format_unop(
    op: &UnOp,
    inner: &Expr,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match op {
        UnOp::Neg => format!("-{}", format_expr(inner, ctx, source_map)),
        UnOp::Pos => format!("+{}", format_expr(inner, ctx, source_map)),
        UnOp::Not => format!("!{}", format_expr(inner, ctx, source_map)),
        UnOp::Deref => format!("*{}", format_expr(inner, ctx, source_map)),
    }
}

/// 格式化函数调用
pub(crate) fn format_call(
    func: &Expr,
    args: &[Expr],
    named_args: &[(String, Expr)],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let func_str = format_expr(func, ctx, source_map);
    let mut all_args: Vec<String> = args
        .iter()
        .map(|a| format_expr(a, ctx, source_map))
        .collect();
    for (name, expr) in named_args {
        all_args.push(format!("{}={}", name, format_expr(expr, ctx, source_map)));
    }
    super::delimited::format_delimited_list("(", ")", &all_args, Some(&func_str), ctx)
}

/// 格式化函数定义（表达式形式）
fn format_fn_def(
    name: &str,
    params: &[Param],
    return_type: &Option<Type>,
    body: &Block,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let params_str = format_params(params, ctx, source_map);
    let ret_str = if let Some(ty) = return_type {
        format!(" -> {}", super::types::format_type(ty, source_map))
    } else {
        String::new()
    };
    format!(
        "fn {}{}{} {}",
        name,
        params_str,
        ret_str,
        format_block(body, ctx, source_map)
    )
}

/// 格式化参数列表
pub fn format_params(
    params: &[Param],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
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
                s.push_str(&super::types::format_type(ty, source_map));
            }
            s
        })
        .collect();
    super::delimited::format_delimited_list("(", ")", &items, None, ctx)
}

/// 格式化 match 表达式
fn format_match_expr(
    match_expr: &Expr,
    arms: &[MatchArm],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let indent = ctx.indent_str();
    let mut inner_ctx = ctx.clone();
    inner_ctx.indent();
    let inner_indent = inner_ctx.indent_str();

    // 计算最长的 pattern 长度，用于对齐
    let max_pattern_len = arms
        .iter()
        .map(|arm| format_pattern(&arm.pattern, ctx, source_map).len())
        .max()
        .unwrap_or(0);

    let mut result = format!("match {} {{\n", format_expr(match_expr, ctx, source_map));

    for arm in arms {
        let pattern_str = format_pattern(&arm.pattern, ctx, source_map);
        let body_str = format_block_inline(&arm.body, &inner_ctx, source_map);
        let pattern_len = pattern_str.len();

        // 超过行宽时 pattern 换行
        let line_len = inner_indent.len() + pattern_len + 4 + body_str.len();
        if line_len > ctx.options.line_width && pattern_len > 10 {
            // pattern 过长，换行
            result.push_str(&format!(
                "{}{}\n{}    => {},\n",
                inner_indent, pattern_str, inner_indent, body_str
            ));
        } else {
            // 对齐 pattern
            let padding = " ".repeat(max_pattern_len.saturating_sub(pattern_len));
            result.push_str(&format!(
                "{}{}{} => {},\n",
                inner_indent, pattern_str, padding, body_str
            ));
        }
    }

    result.push_str(&indent);
    result.push('}');
    result
}

/// 格式化模式
pub fn format_pattern(
    pat: &Pattern,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    match pat {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Identifier(name) => name.clone(),
        Pattern::Literal(lit) => format_literal(lit, ctx),
        Pattern::Tuple(pats) => {
            let items: Vec<String> = pats
                .iter()
                .map(|p| format_pattern(p, ctx, source_map))
                .collect();
            format!("({})", items.join(", "))
        }
        Pattern::Struct { name, fields } => {
            let field_strs: Vec<String> = fields
                .iter()
                .map(|(field_name, is_mut, pat)| {
                    let mut_str = if *is_mut { "mut " } else { "" };
                    format!(
                        "{}{}: {}",
                        mut_str,
                        field_name,
                        format_pattern(pat, ctx, source_map)
                    )
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
                format!(
                    "{}::{} ({})",
                    name,
                    variant,
                    format_pattern(pat, ctx, source_map)
                )
            } else {
                format!("{}::{}", name, variant)
            }
        }
        Pattern::Or(pats) => {
            let items: Vec<String> = pats
                .iter()
                .map(|p| format_pattern(p, ctx, source_map))
                .collect();
            items.join(" | ")
        }
        Pattern::Guard { pattern, condition } => {
            format!(
                "{} if {}",
                format_pattern(pattern, ctx, source_map),
                format_expr(condition, ctx, source_map)
            )
        }
    }
}

/// 格式化代码块
pub fn format_block(
    block: &Block,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut inner_ctx = ctx.clone();
    inner_ctx.indent();
    let inner_indent = inner_ctx.indent_str();
    let outer_indent = ctx.indent_str();

    // §6.3 空代码块：输出 {}
    if block.stmts.is_empty() && block.expr.is_none() {
        return "{}".to_string();
    }

    // §6.2 单行代码块：检查是否可以使用单行格式
    // 条件：只有一个语句，没有表达式，没有注释
    if block.stmts.len() == 1 && block.expr.is_none() {
        let stmt = &block.stmts[0];
        // 检查是否有前导注释
        let leading_comments =
            source_map.comments_between_lines(block.span.start.line, stmt.span.start.line);
        // 检查是否有行末注释
        let trailing_comment = source_map.trailing_comment_on_line(stmt.span.end.line);

        if leading_comments.is_empty() && trailing_comment.is_none() {
            let stmt_str = super::stmt::format_stmt(&stmt.kind, &inner_ctx, source_map);
            let single_line = format!("{{ {} }}", stmt_str);

            // 检查单行格式是否超过行宽
            if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
                return single_line;
            }
        }
    }

    let mut result = "{\n".to_string();

    for (i, stmt) in block.stmts.iter().enumerate() {
        // 输出语句前的注释
        if i == 0 {
            // 第一个语句：输出块开始到第一个语句之间的注释
            let comments =
                source_map.comments_between_lines(block.span.start.line, stmt.span.start.line);
            for comment in &comments {
                result.push_str(&inner_indent);
                result.push_str(&comment.content);
                result.push('\n');
            }
        } else {
            let prev_end = block.stmts[i - 1].span.end.line;
            let comments = source_map.comments_between_lines(prev_end + 1, stmt.span.start.line);
            for comment in &comments {
                result.push_str(&inner_indent);
                result.push_str(&comment.content);
                result.push('\n');
            }
        }

        let stmt_str = super::stmt::format_stmt(&stmt.kind, &inner_ctx, source_map);
        result.push_str(&inner_indent);
        result.push_str(&stmt_str);
        result.push('\n');

        // 处理行末注释
        if let Some(trailing) = source_map.trailing_comment_on_line(stmt.span.end.line) {
            if trailing.span.start.offset > stmt.span.end.offset {
                let last_newline = result.rfind('\n').unwrap_or(0);
                result.truncate(last_newline);
                result.push_str(&format!(" {}\n", trailing.content));
            }
        }
    }

    if let Some(expr) = &block.expr {
        let expr_str = format_expr(expr, &inner_ctx, source_map);
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
    source_map: &SourceMap,
) -> String {
    // 如果块只有一个表达式，返回内联形式
    if block.stmts.is_empty() {
        if let Some(expr) = &block.expr {
            return format_expr(expr, ctx, source_map);
        }
    }
    format_block(block, ctx, source_map)
}

/// 格式化 lambda 表达式
fn format_lambda(
    params: &[Param],
    body: &Block,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let params_str = format_params(params, ctx, source_map);
    // 如果 body 只有一个表达式，使用简洁形式
    if body.stmts.is_empty() {
        if let Some(expr) = &body.expr {
            return format!("{} => {}", params_str, format_expr(expr, ctx, source_map));
        }
    }
    format!("{} => {}", params_str, format_block(body, ctx, source_map))
}

/// 格式化 f-string
fn format_fstring(
    segments: &[FStringSegment],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let mut result = "f\"".to_string();
    for seg in segments {
        match seg {
            FStringSegment::Text(text) => result.push_str(text),
            FStringSegment::Interpolation { expr, format_spec } => {
                result.push('{');
                result.push_str(&format_expr(expr, ctx, source_map));
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

/// 格式化列表，支持元素过多时换行
pub(crate) fn format_list(
    exprs: &[Expr],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let items: Vec<String> = exprs
        .iter()
        .map(|e| format_expr(e, ctx, source_map))
        .collect();
    super::delimited::format_delimited_list("[", "]", &items, None, ctx)
}

/// 格式化字典，支持元素过多时换行
pub(crate) fn format_dict(
    pairs: &[(Expr, Expr)],
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let items: Vec<String> = pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "{}: {}",
                format_expr(k, ctx, source_map),
                format_expr(v, ctx, source_map)
            )
        })
        .collect();
    super::delimited::format_delimited_list("{", "}", &items, None, ctx)
}

/// 格式化字段访问，处理链式调用换行
fn format_field_access(
    inner: &Expr,
    field: &str,
    ctx: &FormatContext,
    source_map: &SourceMap,
) -> String {
    let inner_str = format_expr(inner, ctx, source_map);
    let field_str = format!(".{}", field);

    // 计算完整表达式的长度
    let total_len = inner_str.len() + field_str.len() + ctx.indent_width();

    // 如果不超过行宽，使用单行格式
    if total_len <= ctx.options.line_width {
        return format!("{}{}", inner_str, field_str);
    }

    // 需要换行时，每行一个方法调用，保持缩进
    let indent = ctx.indent_str();
    let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));

    // 检查是否是链式调用（FieldAccess 嵌套）
    if let Expr::FieldAccess { .. } = inner {
        // 链式调用：换行并增加缩进
        return format!("{}\n{}{}", inner_str, inner_indent, field_str);
    }

    // 普通字段访问
    format!("{}{}", inner_str, field_str)
}
