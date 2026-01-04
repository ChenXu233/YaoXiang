//! Parser tests module

mod basic;
mod boundary;
mod state;
mod type_parser;
mod fn_def;
mod syntax_validation;

pub use basic::*;
pub use boundary::*;
pub use state::*;
pub use type_parser::*;
pub use fn_def::*;
pub use syntax_validation::*;
