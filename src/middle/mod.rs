//! Intermediate Representation (IR) and code generation
//!
//! This module handles the transformation from AST to bytecode.

pub mod ir;
pub mod codegen;
pub mod optimizer;

pub use ir::*;
