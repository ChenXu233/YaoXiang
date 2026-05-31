//! 通用分隔列表格式化器
//!
//! 消除 format_list / format_dict / format_call 中重复的
//! "单行/多行切换"逻辑。

use super::super::context::FormatContext;

/// 格式化分隔列表，自动在超出行宽时切换到多行格式。
///
/// - `open` / `close`: 开闭分隔符（如 `[]`、`{}`、`()`）
/// - `items`: 已格式化的元素字符串
/// - `prefix`: 可选的前缀（如函数调用的 `func_str`）
/// - `ctx`: 格式化上下文
pub fn format_delimited_list(
    open: &str,
    close: &str,
    items: &[String],
    prefix: Option<&str>,
    ctx: &FormatContext,
) -> String {
    if items.is_empty() {
        return match prefix {
            Some(p) => format!("{}{}{}", p, open, close),
            None => format!("{}{}", open, close),
        };
    }

    let prefix_str = prefix.unwrap_or("");
    let single_line = format!("{}{}{}{}", prefix_str, open, items.join(", "), close);

    if ctx.indent_width() + single_line.len() <= ctx.options.line_width {
        return single_line;
    }

    // 多行格式
    let indent = ctx.indent_str();
    let inner_indent = format!("{}{}", indent, " ".repeat(ctx.options.indent_width));
    let mut result = format!("{}{}\n", prefix_str, open);
    for item in items {
        result.push_str(&inner_indent);
        result.push_str(item);
        result.push_str(",\n");
    }
    result.push_str(&indent);
    result.push_str(close);
    result
}
