//! Type checker module

use super::parser::ast;
use crate::middle;
use std::collections::HashMap;
use crate::util::span::Span;

/// Type environment
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    vars: HashMap<String, ast::Type>,
    types: HashMap<String, ast::Type>,
    scope_level: usize,
    scopes: Vec<HashMap<String, ast::Type>>,
}

/// Check a module and generate IR
pub fn check_module(
    _ast: &ast::Module,
    _env: &mut TypeEnvironment,
) -> Result<middle::ModuleIR, TypeError> {
    // TODO: Implement type checking
    Ok(middle::ModuleIR {
        types: vec![],
        constants: vec![],
        globals: vec![],
        functions: vec![],
    })
}

#[derive(Debug, thiserror::Error)]
pub enum TypeError {
    #[error("Type mismatch: expected {0:?}, found {1:?}")]
    TypeMismatch(ast::Type, ast::Type),
    #[error("Unknown variable: {0}")]
    UnknownVariable(String),
    #[error("Unknown type: {0}")]
    UnknownType(String),
    #[error("Arity mismatch: expected {0} arguments, found {1}")]
    ArityMismatch(usize, usize),
    #[error("Recursive type definition")]
    RecursiveType,
    #[error("Unsupported operation: {0}")]
    UnsupportedOp(String),
}
