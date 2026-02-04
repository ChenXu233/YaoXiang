//! REPL Engine Module
//!
//! Core evaluation engine for REPL sessions.

pub mod context;
pub mod evaluator;

pub use context::REPLContext;
pub use evaluator::Evaluator;
