//! Core algorithm layer - REFACTORING IN PROGRESS
//! Contains the main compiler algorithms split into specialized modules
//! Supports RFC-004, RFC-010, and RFC-011

pub mod lexer;
pub mod parser;

pub use crate::frontend::core::parser::*;

// TODO: Add other modules as they are refactored
pub mod type_system;
// pub mod const_eval;

// Re-export commonly used items
pub use lexer::tokenize;
