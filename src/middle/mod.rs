//! Intermediate Representation (IR) and code generation
//!
//! This module handles the transformation from AST to bytecode.

#![allow(ambiguous_glob_reexports)]

pub mod bytecode;
pub mod codegen;
pub mod ir;
pub mod ir_gen;
pub mod lifetime;
pub mod module;
pub mod monomorphize;

pub use bytecode::*;
pub use codegen::*;
pub use ir::*;
pub use ir_gen::*;
pub use lifetime::*;
pub use module::*;
pub use monomorphize::instance::*;
pub use monomorphize::*;
