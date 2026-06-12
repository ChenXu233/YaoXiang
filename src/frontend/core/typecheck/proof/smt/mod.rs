//! SMT 加速模块 — 基于 RFC-027 §8
//!
//! ast.rs     — 纯数据结构（SMTExpr, SMTCommand, SMTResult）
//! translate.rs — ConstExpr → SMTCommand 翻译器（纯函数）
//! z3_backend.rs — Z3 C API 后端（z3-sys FFI 静态链接）

pub mod ast;
pub mod translate;
pub mod z3_backend;

#[cfg(test)]
mod tests;
