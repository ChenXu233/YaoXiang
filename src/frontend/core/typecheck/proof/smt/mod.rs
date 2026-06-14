//! SMT 加速模块 — 基于 RFC-027 §8
//!
//! ast.rs     — 纯数据结构（SMTExpr, SMTCommand, SMTResult）
//! translate.rs — ConstExpr → SMTCommand 翻译器（纯函数）
//! z3_ffi.rs   — Z3 C API FFI 绑定（预生成，不依赖 z3-sys）
//! z3_backend.rs — Z3 后端封装

pub mod ast;
pub mod translate;
#[cfg(not(feature = "wasm"))]
pub mod z3_backend;
#[cfg(not(feature = "wasm"))]
pub mod z3_ffi;

#[cfg(test)]
mod tests;
