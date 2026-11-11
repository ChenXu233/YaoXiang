//! Core algorithm layer
//! Contains the main compiler algorithms split into specialized modules

pub mod lexer;
pub mod parser;
pub mod sema;
pub mod spawn;
pub mod types;

pub use crate::frontend::core::parser::*;

// Re-export commonly used items
pub use lexer::tokenize;
pub use types::MonoType;
