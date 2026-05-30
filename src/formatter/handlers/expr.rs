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
        } => super::common::format_if(condition, then_branch, elif_branches, else_branch, ctx),
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
        } => super::common::format_while_loop(condition, body, label, ctx),
        Expr::For {
            var,
            var_mut,
            iterable,
            body,
            label,
            span: _,
        } => super::common::format_for_loop(var, *var_mut, iterable, body, label, ctx),
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
        Expr::List(exprs, _span) => format_list(exprs, ctx),
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
        Expr::Dict(pairs, _span) => format_dict(pairs, ctx),
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
        } => format_field_access(inner, field, ctx),
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
        Expr::Eval { mode, body, .. } => {
            let mode_str = match mode {
                EvalMode::Block => "block",
                EvalMode::Auto => "auto",
                EvalMode::Eager => "eager",
            };
            format!("@{} {}", mode_str, format_block(body, ctx))
        }
        Expr::Spawn { body, .. } => {
            format!("spawn {}", format_block(body, ctx))
        }
        Expr::Lambda {
            params,
            body,
            span: _,
        } => format_lambda(params, body, ctx),
        Expr::FString { segments, span: _ } => format_fstring(segments, ctx),
        Expr::Borrow {
            mutable,
            expr: inner,
            span: _,
        } => {
            if *mutable {
                format!("&mut {}", format_expr(inner, ctx))
            } else {
                format!("&{}", format_expr(inner, ctx))
            }
        }
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

    let left_str = format_expr(left, ctx);
    let right_str = format_expr(right, ctx);

    // 计算完整表达式的预估长度
    let total_len = left_str.len() + op_str.len() + right_str.len() + ctx.indent_width();

    // 低优先级运算符（需要更多换行）
    let is_low_priority = matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Or | BinOp::And | BinOp::Assign
    );

    // 如果不超过行宽，使用单行格式
    if total_len <= ctx.options.line_width {
        return format!("{} {} {}", left_str, op_str, right_str);
    }

    // 需要换行时，根据优先级决定策略
    let indent = ctx.indent_str();
    let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));

    // 低优先级运算符：运算符放行首，保持对齐
    if is_low_priority {
        return format!("{}\n{}{} {}", left_str, inner_indent, op_str, right_str);
    }

    // 高优先级运算符：换行后运算符放行首
    format!("{}\n{}{} {}", left_str, inner_indent, op_str, right_str)
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

    if all_args.is_empty() {
        return format!("{}()", func_str);
    }

    let single_line = format!("{}({})", func_str, all_args.join(", "));

    // 如果单行不超过行宽，使用单行格式
    if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
        single_line
    } else {
        // 多行格式：尾随逗号风格，参数与开括号对齐
        let indent = ctx.indent_str();
        let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));
        let mut result = format!("{}(\n", func_str);
        for arg in all_args.iter() {
            result.push_str(&inner_indent);
            result.push_str(arg);
            result.push_str(",\n");
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
        "{}fn {}({}){} {}",
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

    // 计算最长的 pattern 长度，用于对齐
    let max_pattern_len = arms
        .iter()
        .map(|arm| format_pattern(&arm.pattern).len())
        .max()
        .unwrap_or(0);

    let mut result = format!("match {} {{\n", format_expr(match_expr, ctx));

    for arm in arms {
        let pattern_str = format_pattern(&arm.pattern);
        let body_str = format_block_inline(&arm.body, &inner_ctx);
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

/// 格式化列表，支持元素过多时换行
fn format_list(
    exprs: &[Expr],
    ctx: &FormatContext,
) -> String {
    if exprs.is_empty() {
        return "[]".to_string();
    }

    let items: Vec<String> = exprs.iter().map(|e| format_expr(e, ctx)).collect();
    let single_line = format!("[{}]", items.join(", "));

    // 如果单行不超过行宽，使用单行格式
    if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
        single_line
    } else {
        // 多行格式：每个元素一行，保持对齐
        let indent = ctx.indent_str();
        let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));
        let mut result = "[\n".to_string();
        for item in items.iter() {
            result.push_str(&inner_indent);
            result.push_str(item);
            result.push_str(",\n");
        }
        result.push_str(&indent);
        result.push(']');
        result
    }
}

/// 格式化字典，支持元素过多时换行
fn format_dict(
    pairs: &[(Expr, Expr)],
    ctx: &FormatContext,
) -> String {
    if pairs.is_empty() {
        return "{}".to_string();
    }

    let items: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", format_expr(k, ctx), format_expr(v, ctx)))
        .collect();

    let single_line = format!("{{{}}}", items.join(", "));

    // 如果单行不超过行宽，使用单行格式
    if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
        single_line
    } else {
        // 多行格式：每个键值对一行，保持对齐
        let indent = ctx.indent_str();
        let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));
        let mut result = "{\n".to_string();
        for item in items.iter() {
            result.push_str(&inner_indent);
            result.push_str(item);
            result.push_str(",\n");
        }
        result.push_str(&indent);
        result.push('}');
        result
    }
}

/// 格式化字段访问，处理链式调用换行
fn format_field_access(
    inner: &Expr,
    field: &str,
    ctx: &FormatContext,
) -> String {
    let inner_str = format_expr(inner, ctx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatter::FormatOptions;
    use crate::util::span::Span;

    fn default_ctx() -> FormatContext {
        FormatContext::new(FormatOptions::default())
    }

    #[test]
    fn test_format_literal_int() {
        let lit = Literal::Int(42);
        assert_eq!(format_literal(&lit), "42");
    }

    #[test]
    fn test_format_literal_float() {
        let lit = Literal::Float(3.14);
        assert_eq!(format_literal(&lit), "3.14");
    }

    #[test]
    fn test_format_literal_float_no_decimal() {
        let lit = Literal::Float(42.0);
        let result = format_literal(&lit);
        assert!(
            result.contains('.'),
            "Float should have decimal point: {}",
            result
        );
    }

    #[test]
    fn test_format_literal_bool() {
        assert_eq!(format_literal(&Literal::Bool(true)), "true");
        assert_eq!(format_literal(&Literal::Bool(false)), "false");
    }

    #[test]
    fn test_format_literal_string() {
        let lit = Literal::String("hello".to_string());
        assert_eq!(format_literal(&lit), "\"hello\"");
    }

    #[test]
    fn test_format_binop_add() {
        let ctx = default_ctx();
        let left = Expr::Lit(Literal::Int(1), Span::dummy());
        let right = Expr::Lit(Literal::Int(2), Span::dummy());
        let result = format_binop(&BinOp::Add, &left, &right, &ctx);
        assert_eq!(result, "1 + 2");
    }

    #[test]
    fn test_format_binop_eq() {
        let ctx = default_ctx();
        let left = Expr::Var("x".to_string(), Span::dummy());
        let right = Expr::Lit(Literal::Int(0), Span::dummy());
        let result = format_binop(&BinOp::Eq, &left, &right, &ctx);
        assert_eq!(result, "x == 0");
    }

    #[test]
    fn test_format_call_no_args() {
        let ctx = default_ctx();
        let func = Expr::Var("foo".to_string(), Span::dummy());
        let result = format_call(&func, &[], &[], &ctx);
        assert_eq!(result, "foo()");
    }

    #[test]
    fn test_format_call_with_args() {
        let ctx = default_ctx();
        let func = Expr::Var("add".to_string(), Span::dummy());
        let arg1 = Expr::Lit(Literal::Int(1), Span::dummy());
        let arg2 = Expr::Lit(Literal::Int(2), Span::dummy());
        let result = format_call(&func, &[arg1, arg2], &[], &ctx);
        assert_eq!(result, "add(1, 2)");
    }

    #[test]
    fn test_format_list_empty() {
        let ctx = default_ctx();
        let result = format_list(&[], &ctx);
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_list_single() {
        let ctx = default_ctx();
        let items = vec![Expr::Lit(Literal::Int(1), Span::dummy())];
        let result = format_list(&items, &ctx);
        assert_eq!(result, "[1]");
    }

    #[test]
    fn test_format_dict_empty() {
        let ctx = default_ctx();
        let result = format_dict(&[], &ctx);
        assert_eq!(result, "{}");
    }

    #[test]
    fn test_format_return() {
        let ctx = default_ctx();
        let expr = Expr::Return(
            Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
            Span::dummy(),
        );
        let result = format_expr(&expr, &ctx);
        assert_eq!(result, "return 42");
    }

    #[test]
    fn test_format_return_none() {
        let ctx = default_ctx();
        let expr = Expr::Return(None, Span::dummy());
        let result = format_expr(&expr, &ctx);
        assert_eq!(result, "return");
    }

    #[test]
    fn test_format_cast() {
        let ctx = default_ctx();
        let inner = Expr::Var("x".to_string(), Span::dummy());
        let expr = Expr::Cast {
            expr: Box::new(inner),
            target_type: Type::Int(64),
            span: Span::dummy(),
        };
        let result = format_expr(&expr, &ctx);
        assert_eq!(result, "x as i64");
    }
}
