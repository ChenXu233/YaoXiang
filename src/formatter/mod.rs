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

use crate::util::diagnostic::Diagnostic;

/// 格式化错误
#[derive(Debug)]
pub enum FormatError {
    /// 用户代码含有语义错误
    Semantic(Vec<Diagnostic>),
    /// 格式化器输出无效（post-verify 捕获）
    FormatterBug {
        input: Vec<Diagnostic>,
        output: Vec<Diagnostic>,
    },
}

impl std::fmt::Display for FormatError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            FormatError::Semantic(diags) => {
                for d in diags {
                    writeln!(f, "{}", d)?;
                }
                Ok(())
            }
            FormatError::FormatterBug { .. } => {
                write!(
                    f,
                    "Formatter internal error: output is invalid.\n\
                     This is a bug. Please report it at:\n\
                     https://github.com/ChenXu233/YaoXiang/issues/new"
                )
            }
        }
    }
}

impl std::error::Error for FormatError {}

// Needed for LSP (Send + Sync)
unsafe impl Send for FormatError {}
unsafe impl Sync for FormatError {}

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
/// let result = format_source(source, &options);
/// assert!(result.is_err(), "semantic errors should be rejected");
/// ```
pub fn format_source(
    source: &str,
    options: &FormatOptions,
) -> std::result::Result<String, FormatError> {
    // 1. Pre-validate
    let vr = crate::frontend::validate::validate_source(source);
    if vr.diagnostics.iter().any(|d| d.severity.is_error()) {
        return Err(FormatError::Semantic(vr.diagnostics));
    }
    let module = vr.module.expect("validate_source passed but no module");

    // 2. Format using validate_source's AST (skip re-lex/re-parse)
    let source_map = SourceMap::build(source);
    let formatter = Formatter::new(options.clone(), source_map);
    let formatted = formatter.format_module(&module);

    // 3. Post-verify (if enabled)
    if options.verify {
        let post_vr = crate::frontend::validate::validate_source(&formatted);
        if post_vr.diagnostics.iter().any(|d| d.severity.is_error()) {
            return Err(FormatError::FormatterBug {
                input: vr.diagnostics,
                output: post_vr.diagnostics,
            });
        }
    }

    Ok(formatted)
}

/// 检查源代码是否已格式化
///
/// 返回 `true` 表示代码已按规则格式化，`false` 表示需要格式化。
pub fn check_formatted(
    source: &str,
    options: &FormatOptions,
) -> anyhow::Result<bool> {
    let formatted = format_source(source, options).map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(formatted == source)
}
