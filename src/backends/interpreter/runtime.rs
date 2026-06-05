//! Interpreter runtime configuration.
//!
//! This is the interpreter-side entry for selecting runtime tier.
//! It defaults to the current behavior (global `@block`).

use crate::backends::runtime::RuntimeMode;

/// Interpreter runtime configuration.
#[derive(Debug, Clone)]
pub struct InterpreterRuntimeConfig {
    /// Runtime tier (Embedded / Standard / Full).
    pub runtime: RuntimeMode,
    /// Worker count (only meaningful for Full runtime).
    pub workers: usize,
    /// Work-stealing toggle (only meaningful for Full runtime).
    pub work_stealing: bool,
}

impl Default for InterpreterRuntimeConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeMode::Embedded,
            workers: 1,
            work_stealing: false,
        }
    }
}
