//! Utility types and functions

pub mod arith;
pub mod cache;
pub mod config;
pub mod diagnostic;
pub mod i18n;
pub mod logger;
pub mod span;
pub mod test_markers;
#[cfg(feature = "cli")]
pub mod test_runner;
pub mod time_compat;
/// wasm32 平台胶水（#435）：Emscripten libz3.a 的 musl/JS 符号 stub。
#[cfg(target_arch = "wasm32")]
mod wasm_z3_shims;

/// shim 日志裸导出 re-export（供 wasm cdylib 带出；guard 拦不到的观测口）。
#[cfg(target_arch = "wasm32")]
pub use wasm_z3_shims::{shim_log_len, shim_log_ptr};

#[cfg(test)]
mod tests;
