//! REPL Module
//!
//! Provides interactive evaluation capabilities for YaoXiang.
//!
//! This module contains:
//! - [`backend_trait::REPLBackend`] - Abstract interface for REPL backends
//! - [`engine::Evaluator`] - Core evaluation engine
//! - [`engine::REPLContext`] - Execution context
//! - [`session::SessionREPL`] - Session-based REPL with rustyline
//! - [`commands::CommandHandler`] - Command processor

pub mod backend_trait;
pub mod commands;
pub mod engine;
pub mod session;

pub use backend_trait::{REPLBackend, EvalResult, SymbolInfo, ExecutionStats};
pub use commands::{CommandHandler, CommandResult};
pub use engine::{Evaluator, REPLContext};
pub use session::SessionREPL;
