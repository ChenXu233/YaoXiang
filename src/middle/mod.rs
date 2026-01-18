//! Intermediate Representation (IR) and code generation
//!
//! This module handles the transformation from AST to bytecode.

#![allow(ambiguous_glob_reexports)]

pub mod codegen;
pub mod escape_analysis;
pub mod ir;
pub mod lifetime;
pub mod module;
pub mod monomorphize;
pub mod optimizer;

pub use codegen::*;
pub use escape_analysis::*;
pub use ir::*;
pub use lifetime::*;
pub use module::*;
pub use monomorphize::instance::*;
pub use monomorphize::*;
