//! Frontend compilation pipeline
//!
//! This module contains the lexer, parser, and type checker.
//! The frontend transforms source code into an intermediate representation (IR).

use crate::middle;
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
        debug!("Compiling source code ({} bytes)", source.len());
        // Lexical analysis
        let tokens = lexer::tokenize(source).map_err(|e| CompileError::LexError(e.to_string()))?;
        debug!("Tokenized into {} tokens", tokens.len());

        // Parsing
        debug!("Starting parsing...");
        let ast = parser::parse(&tokens).map_err(|e| CompileError::ParseError(e.to_string()))?;
        debug!("Parsing successful, got {} statements", ast.items.len());

        // Type checking
        debug!("Starting type checking...");
        let module = typecheck::check_module(&ast, Some(&mut self.type_env))
            .map_err(|e| CompileError::TypeError(format!("{:?}", e)))?;
        debug!("Type checking successful");

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
