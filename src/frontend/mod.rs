//! Frontend compilation pipeline
//!
//! This module contains the lexer, parser, and type checker.
//! The frontend transforms source code into an intermediate representation (IR).

use crate::middle;
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

    /// Compile source code to IR (两阶段流程)
    ///
    /// 阶段1: 类型检查 - 检查类型正确性
    /// 阶段2: IR 生成 - 生成中间表示
    pub fn compile(
        &mut self,
        source: &str,
    ) -> Result<middle::ModuleIR, CompileError> {
        let source_len = source.len();
        debug!("{}", t_cur(MSG::CompilingSource, Some(&[&source_len])));
        // Lexical analysis
        let tokens = lexer::tokenize(source).map_err(|e| CompileError::LexError(e.to_string()))?;

        // Parsing
        let ast = parser::parse(&tokens).map_err(|e| CompileError::ParseError(e.to_string()))?;

        // 阶段1: 类型检查
        let _type_result = typecheck::check_module(&ast, Some(&mut self.type_env))
            .map_err(|e| CompileError::TypeError(format!("{:?}", e)))?;

        // 阶段2: IR 生成
        let module = typecheck::generate_ir(&ast, &_type_result)
            .map_err(|e| CompileError::TypeError(format!("{:?}", e)))?;

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
