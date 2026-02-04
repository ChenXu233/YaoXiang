//! REPL Module
//!
//! Provides interactive evaluation capabilities for YaoXiang.
//!
//! This module contains:
//! - [`backend_trait::REPLBackend`] - Abstract interface for REPL backends
//! - [`engine::Evaluator`] - Core evaluation engine
//! - [`engine::REPLContext`] - Execution context
//! - [`line::LineREPL`] - Line-based REPL with rustyline
//! - [`commands::CommandHandler`] - Command processor
//! - [`legacy::REPL`] - Simple line-mode REPL

pub mod backend_trait;
pub mod commands;
pub mod engine;
pub mod legacy;
pub mod line;

pub use backend_trait::{REPLBackend, EvalResult, SymbolInfo, ExecutionStats};
pub use commands::{CommandHandler, CommandResult};
pub use engine::{Evaluator, REPLContext};
pub use line::LineREPL;
pub use legacy::{REPL, REPLConfig};
