//! Intermediate Representation (IR) and code generation
//!
//! This module handles the transformation from AST to bytecode.

pub mod codegen;
pub mod ir;
pub mod optimizer;

pub use ir::*;
