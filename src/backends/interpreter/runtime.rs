//! Interpreter runtime configuration.
//!
//! This is the interpreter-side entry for selecting runtime tier and evaluation
//! strategy. It defaults to the current behavior (global `@block`).

use crate::backends::runtime::RuntimeMode;

/// Evaluation strategy used by the interpreter (global policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalStrategy {
    /// Equivalent to global `@block`: synchronous, no lazy placeholders.
    Block,
    /// Equivalent to global `@auto`: maximize laziness/concurrency.
    Auto,
    /// Equivalent to global `@eager`: prefer eager evaluation.
    Eager,
}

/// Interpreter runtime configuration.
#[derive(Debug, Clone)]
pub struct InterpreterRuntimeConfig {
    /// Runtime tier (Embedded / Standard / Full).
    pub runtime: RuntimeMode,
    /// Evaluation strategy (`@block/@auto/@eager`).
    pub eval: EvalStrategy,
    /// Worker count (only meaningful for Full runtime).
    pub workers: usize,
    /// Work-stealing toggle (only meaningful for Full runtime).
    pub work_stealing: bool,
}

impl Default for InterpreterRuntimeConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeMode::Embedded,
            eval: EvalStrategy::Block,
            workers: 1,
            work_stealing: false,
        }
    }
}
