//! Interpreter runtime configuration.
//!
//! This is the interpreter-side entry for selecting runtime tier.

use crate::backends::runtime::RuntimeMode;

/// Interpreter runtime configuration.
#[derive(Debug, Clone)]
pub struct InterpreterRuntimeConfig {
    /// Runtime tier (Embedded / Standard).
    pub runtime: RuntimeMode,
    /// Worker count (only meaningful for Standard runtime).
    pub workers: usize,
}

impl Default for InterpreterRuntimeConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeMode::Embedded,
            #[cfg(not(target_arch = "wasm32"))]
            workers: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
            #[cfg(target_arch = "wasm32")]
            workers: 1,
        }
    }
}
