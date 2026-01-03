//! Intermediate Representation (IR) and code generation
//!
//! This module handles the transformation from AST to bytecode.

pub mod codegen;
pub mod escape_analysis;
pub mod ir;
pub mod lifetime;
pub mod monomorphize;
pub mod optimizer;

pub use ir::*;
pub use codegen::*;
pub use escape_analysis::*;
pub use lifetime::*;
pub use monomorphize::*;
pub use monomorphize::instance::*;
