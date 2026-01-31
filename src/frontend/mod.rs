//! Frontend compilation pipeline
//!
//! This module contains the lexer, parser, and type checker.
//! The frontend transforms source code into an intermediate representation (IR).

use crate::middle;
use crate::util::span::SourceFile;
use crate::util::i18n::{t_cur, MSG};
use thiserror::Error;
use tracing::debug;

pub mod typecheck;

// Refactored core modules with RFC support
pub mod core;

/// Compiler context
#[derive(Debug)]
pub struct Compiler {
    /// Type environment
    type_env: typecheck::TypeEnvironment,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    /// Create a new compiler
    #[inline]
    pub fn new() -> Self {
        let mut type_env = typecheck::TypeEnvironment::new();
        // 初始化内置类型和函数
        typecheck::add_builtin_types(&mut type_env);
        typecheck::add_builtin_functions(&mut type_env);

        Self { type_env }
    }

    /// Compile source code to IR with source name (for diagnostics)
    pub fn compile_with_source(
        &mut self,
        source_name: &str,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        let source_len = source.len();
        debug!("{}", t_cur(MSG::CompilingSource, Some(&[&source_len])));

        let source_file = SourceFile::new(source_name.to_string(), source.to_string());

        // Lexical analysis - Using new refactored lexer with RFC support
        let tokens =
            core::lexer::tokenize(source).map_err(|e| CompileError::LexError(e.to_string()))?;

        // Parsing - Using new refactored parser with RFC support
        let ast =
            core::parser::parse(&tokens).map_err(|e| CompileError::ParseError(e.to_string()))?;

        // 阶段1: 类型检查
        let type_result = typecheck::check_module(&ast, Some(&mut self.type_env)).map_err(|e| {
            let message = format_type_errors(&source_file, &e);
            CompileError::TypeError(message)
        })?;

        // 阶段2: IR 生成
        let module = typecheck::generate_ir(&ast, &type_result).map_err(|e| {
            let message = format_type_errors(&source_file, &e);
            CompileError::TypeError(message)
        })?;

        Ok(module)
    }

    /// Compile source code to IR (两阶段流程)
    ///
    /// 阶段1: 类型检查 - 检查类型正确性
    /// 阶段2: IR 生成 - 生成中间表示
    pub fn compile(
        &mut self,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        self.compile_with_source("<input>", source)
    }
}

fn format_type_errors(
    source_file: &SourceFile,
    errors: &[typecheck::TypeError],
) -> String {
    let mut out = String::new();
    for (idx, err) in errors.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let suggestions = err.get_suggestions(None);
        out.push_str(&format_diagnostic(source_file, err, suggestions.as_ref()));
    }
    out
}

fn format_diagnostic(
    source_file: &SourceFile,
    error: &typecheck::TypeError,
    suggestions: Option<&Vec<String>>,
) -> String {
    let span = error.span();
    let message = error.to_i18n_message();
    let error_code = error.error_code();

    if span.is_dummy() {
        return format!("error[{}]: {}\n", error_code, message);
    }

    let line_index = span.start.line.saturating_sub(1);
    let line_text = source_file.content.lines().nth(line_index).unwrap_or("");

    let line_len = line_text.chars().count();
    let col = span.start.column.max(1).min(line_len.saturating_add(1));
    let end_col = if span.end.line == span.start.line && span.end.column > span.start.column {
        span.end.column
    } else {
        col.saturating_add(1)
    }
    .min(line_len.saturating_add(1));
    let caret_len = end_col
        .saturating_sub(col)
        .max(1)
        .min(line_len.saturating_add(1));
    let caret = format!(
        "{}{}",
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(caret_len)
    );

    let line_no = span.start.line;
    let line_no_width = line_no.to_string().len().max(1);
    let gutter = format!("{:>width$} │", line_no, width = line_no_width);
    let empty_gutter = format!("{:>width$} │", "", width = line_no_width);
    let padding = " ".repeat(line_no_width + 1);

    let mut out = String::new();

    // Header line with error code
    out.push_str(&format!("error[{}]: {}\n", error_code, message));

    // Location and code snippet - 使用与行号相同的宽度格式化
    out.push_str(&format!(
        "{}┌─> {}:{:>width$}:{}\n",
        padding,
        source_file.name,
        span.start.line,
        span.start.column,
        width = line_no_width
    ));
    out.push_str(&format!("{}\n", empty_gutter));
    out.push_str(&format!("{} {}\n", gutter, line_text));
    out.push_str(&format!("{} {}\n", empty_gutter, caret));
    out.push_str(&format!("{}\n", empty_gutter));

    // Suggestions
    if let Some(suggestions) = suggestions {
        if !suggestions.is_empty() {
            out.push_str("  = help: ");
            out.push_str(&suggestions.join(", "));
            out.push('\n');
        }
    }

    out
}

/// Format parse errors (simpler format for now)
fn format_parse_error(
    source_file: &SourceFile,
    span: crate::util::span::Span,
    message: &str,
) -> String {
    if span.is_dummy() {
        return format!("error[E0100]: {}\n", message);
    }

    let line_index = span.start.line.saturating_sub(1);
    let line_text = source_file.content.lines().nth(line_index).unwrap_or("");

    let col = span.start.column.max(1);
    let end_col = if span.end.line == span.start.line && span.end.column > span.start.column {
        span.end.column
    } else {
        col + 1
    };
    let caret_len = end_col.saturating_sub(col).max(1);
    let caret = format!(
        "{}{}",
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(caret_len)
    );

    let line_no = span.start.line;
    let line_no_width = line_no.to_string().len().max(1);
    let gutter = format!("{:>width$} │", line_no, width = line_no_width);
    let empty_gutter = format!("{:>width$} │", "", width = line_no_width);
    let padding = " ".repeat(line_no_width + 1);

    let mut out = String::new();
    out.push_str(&format!("error[E0100]: {}\n", message));
    out.push_str(&format!(
        "{}┌─> {}:{:>width$}::{}\n",
        padding,
        source_file.name,
        span.start.line,
        span.start.column,
        width = line_no_width
    ));
    out.push_str(&format!("{}\n", empty_gutter));
    out.push_str(&format!("{} {}\n", gutter, line_text));
    out.push_str(&format!("{} {}\n", empty_gutter, caret));
    out.push_str(&format!("{}\n", empty_gutter));

    out
}

/// Compilation errors
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Lexical error: {0}")]
    LexError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Type error: {0}")]
    TypeError(String),
}
