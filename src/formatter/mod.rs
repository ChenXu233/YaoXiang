#![allow(clippy::module_inception)]

//! YaoXiang 代码格式化工具
//!
//! 基于 AST + 源映射方案实现代码格式化。
//! 支持 CLI 命令行使用和 LSP 集成。
//!
//! # 架构
//!
//! 1. **SourceMap** - 记录注释/空白位置
//! 2. **Formatter** - 遍历 AST，根据配置输出格式化代码
//! 3. **FormatOptions** - 配置选项
//!
//! # 使用
//!
//! ```ignore
//! use yaoxiang::formatter::{format_source, FormatOptions};
//!
//! let options = FormatOptions::default();
//! let result = format_source("let x = 1", &options)?;
//! println!("{}", result);
//! ```

pub mod command;
pub mod context;
pub mod formatter;
pub mod handlers;
pub mod options;
pub mod rules;
pub mod source_map;

#[cfg(test)]
mod tests;

// Re-exports
pub use options::FormatOptions;
pub use command::{run_format_command, FormatRunResult};
pub use formatter::Formatter;
pub use source_map::SourceMap;

use anyhow::Result;

/// 格式化源代码
///
/// 这是格式化工具的主要入口函数。
///
/// # Example
///
/// ```
/// use yaoxiang::formatter::{format_source, FormatOptions};
///
/// let source = "let x = 1";
/// let options = FormatOptions::default();
/// let formatted = format_source(source, &options).unwrap();
/// assert_eq!(formatted, "let\nx = 1\n");
/// ```
pub fn format_source(
    source: &str,
    options: &FormatOptions,
) -> Result<String> {
    let source_map = SourceMap::build(source);
    let tokens = crate::frontend::core::lexer::tokenize(source)
        .map_err(|e| anyhow::anyhow!("Lex error: {}", e))?;
    let parse_result = crate::frontend::core::parser::parse(&tokens);

    // 如果解析有错误，收集所有错误并返回
    if parse_result.has_errors {
        let messages: Vec<String> = parse_result
            .errors
            .iter()
            .map(|e| format!("{}", e))
            .collect();
        return Err(anyhow::anyhow!("Parse errors:\n{}", messages.join("\n")));
    }

    let formatter = Formatter::new(options.clone(), source_map);
    Ok(formatter.format_module(&parse_result.module))
}

/// 检查源代码是否已格式化
///
/// 返回 `true` 表示代码已按规则格式化，`false` 表示需要格式化。
pub fn check_formatted(
    source: &str,
    options: &FormatOptions,
) -> Result<bool> {
    let formatted = format_source(source, options)?;
    Ok(formatted == source)
}
