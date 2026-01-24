//! Frontend compilation pipeline
//!
//! This module contains the lexer, parser, and type checker.
//! The frontend transforms source code into an intermediate representation (IR).

use crate::middle;
use crate::util::span::{SourceFile, Span};
use crate::util::i18n::{t_cur, MSG};
use thiserror::Error;
use tracing::debug;

pub mod lexer;
pub mod parser;
pub mod typecheck;

/// Compiler context
#[derive(Debug, Default)]
pub struct Compiler {
    /// Type environment
    type_env: typecheck::TypeEnvironment,
}

impl Compiler {
    /// Create a new compiler
    #[inline]
    pub fn new() -> Self {
        Self::default()
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

        // Lexical analysis
        let tokens = lexer::tokenize(source)
            .map_err(|e| CompileError::LexError(e.to_string()))?;

        // Parsing
        let ast = parser::parse(&tokens).map_err(|e| {
            let message = format_diagnostic(&source_file, e.span(), &e.to_string());
            CompileError::ParseError(message)
        })?;

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
        out.push_str(&format_diagnostic(source_file, err.span(), &err.to_string()));
    }
    out
}

fn format_diagnostic(
    source_file: &SourceFile,
    span: Span,
    message: &str,
) -> String {
    if span.is_dummy() {
        return format!("{}: error: {}", source_file.name, message);
    }

    let line_index = span.start.line.saturating_sub(1);
    let line_text = source_file
        .content
        .lines()
        .nth(line_index)
        .unwrap_or("");

    let col = span.start.column.max(1);
    let end_col = if span.end.line == span.start.line && span.end.column > span.start.column {
        span.end.column
    } else {
        col + 1
    };
    let caret_len = end_col.saturating_sub(col).max(1);
    let caret = format!("{}{}", " ".repeat(col.saturating_sub(1)), "^".repeat(caret_len));

    let line_no = span.start.line;
    let line_no_width = line_no.to_string().len().max(1);
    let gutter = format!("{:>width$} |", line_no, width = line_no_width);
    let empty_gutter = format!("{:>width$} |", "", width = line_no_width);

    format!(
        "{}:{}:{}: error: {}\n{}\n{} {}\n{} {}",
        source_file.name,
        span.start.line,
        span.start.column,
        message,
        empty_gutter,
        gutter,
        line_text,
        empty_gutter,
        caret
    )
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
