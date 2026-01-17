//! Parser tests module
#![allow(unused_imports)]
mod basic;
mod boundary;
mod coverage;
mod fn_def;
mod ref_test;
mod state;
mod syntax_validation;
mod type_parser;

pub use basic::*;
pub use boundary::*;
pub use coverage::*;
pub use fn_def::*;
pub use ref_test::*;
pub use state::*;
pub use syntax_validation::*;
pub use type_parser::*;
