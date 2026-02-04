//! REPL Backend Trait
//!
//! Defines the abstract interface for REPL backends.

use std::time::Duration;
use crate::backends::common::RuntimeValue;

/// Evaluation result
#[derive(Debug)]
pub enum EvalResult {
    /// Evaluation produced a value
    Value(RuntimeValue),
    /// Evaluation produced no value (unit)
    Ok,
    /// Evaluation had an error
    Error(String),
    /// More input needed (incomplete expression)
    Incomplete,
}

/// Symbol information for completion
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Symbol name
    pub name: String,
    /// Type signature
    pub type_signature: String,
    /// Documentation
    pub doc: Option<String>,
}

/// Execution statistics
#[derive(Debug, Default, Clone)]
pub struct ExecutionStats {
    /// Number of evaluations
    pub eval_count: usize,
    /// Total execution time
    pub total_time: Duration,
}

/// REPL Backend Trait
///
/// This trait defines the interface that all REPL backends must implement.
/// It provides abstract methods for evaluation, completion, and state management.
pub trait REPLBackend {
    /// Evaluate code and return result
    fn eval(
        &mut self,
        code: &str,
    ) -> EvalResult;

    /// Get completion candidates for a line
    fn complete(
        &self,
        line: &str,
        _pos: usize,
    ) -> Vec<String>;

    /// Get all available symbols
    fn get_symbols(&self) -> Vec<SymbolInfo>;

    /// Get type signature for a symbol
    fn get_type(
        &self,
        name: &str,
    ) -> Option<String>;

    /// Clear all state
    fn clear(&mut self);

    /// Get execution statistics
    fn stats(&self) -> ExecutionStats;
}
