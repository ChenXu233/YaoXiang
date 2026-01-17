//! Frontend compilation pipeline
//!
//! This module contains the lexer, parser, and type checker.
//! The frontend transforms source code into an intermediate representation (IR).

use crate::middle;
use crate::util::i18n::{t, t_simple, MSG};
use crate::util::logger::get_lang;
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

    /// Compile source code to IR
    pub fn compile(
        &mut self,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        let lang = get_lang();
        let source_len = source.len();
        debug!("{}", t(MSG::CompilingSource, lang, Some(&[&source_len])));
        // Lexical analysis
        let tokens = lexer::tokenize(source).map_err(|e| CompileError::LexError(e.to_string()))?;
        debug!("Tokenized into {} tokens", tokens.len()); // Internal message

        // Parsing
        debug!("{}", t_simple(MSG::ParserStart, lang));
        let ast = parser::parse(&tokens).map_err(|e| CompileError::ParseError(e.to_string()))?;
        debug!("Parsing successful, got {} statements", ast.items.len()); // Internal message

        // Type checking
        debug!("{}", t_simple(MSG::TypeCheckStart, lang));
        let module = typecheck::check_module(&ast, Some(&mut self.type_env))
            .map_err(|e| CompileError::TypeError(format!("{:?}", e)))?;
        debug!("{}", t_simple(MSG::TypeCheckComplete, lang));

        Ok(module)
    }
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
