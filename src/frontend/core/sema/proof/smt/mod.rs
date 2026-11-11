//! SMT 加速模块 — 基于 RFC-027 §8
//!
//! ast.rs     — 纯数据结构（SMTExpr, SMTCommand, SMTResult）
//! backend.rs — 求解器后端抽象（Solver trait，RFC §8 不绑定具体求解器）
//! translate.rs — ConstExpr → SMTCommand 翻译器（纯函数）
//! z3_ffi.rs   — Z3 C API FFI 绑定（预生成，不依赖 z3-sys）
//! z3_backend.rs — Z3 后端封装

pub mod ast;
pub mod backend;
pub mod translate;
// wasm32 下同样编译（#435：wasm 形态必须带 Z3——libz3.a 经 build.rs 链接，
// Emscripten 预编译资产由 z3-wasm release 供给）；无库时 build.rs 拒绝构建。
pub mod z3_backend;
pub mod z3_ffi;

#[cfg(test)]
mod tests;
