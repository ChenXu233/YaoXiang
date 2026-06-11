//! SMT 加速模块
//!
//! ast.rs     — 纯数据结构（SMTExpr, SMTCommand, SMTResult）
//! translate.rs — ConstExpr → SMTCommand 翻译器（纯函数）
//! z3_backend.rs — Z3 C API 后端（唯一依赖 z3-sys）

pub mod ast;
pub mod translate;
#[cfg(feature = "z3")]
pub mod z3_backend;
