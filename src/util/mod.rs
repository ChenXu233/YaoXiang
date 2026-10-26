//! Utility types and functions

pub mod arith;
pub mod cache;
pub mod config;
pub mod consumption;
pub mod diagnostic;
pub mod i18n;
pub mod logger;
pub mod plan_id;
pub mod span;
pub mod test_markers;
#[cfg(feature = "cli")]
pub mod test_runner;
/// 测试资源画像的执行侧助手（`cost: mode=exclusive` 独占锁，#486 第 2 项）；
/// 仅测试构建可见——它不是产品面 API。锁目录解析复用 `package::source::git::git_command`，
/// 而 `package` 在 wasm32 目标下不编译，故该目标下同样不编译本模块。
#[cfg(all(test, not(target_arch = "wasm32")))]
pub mod test_support;
pub mod time_compat;
/// wasm32 平台胶水（#435）：Emscripten libz3.a 的 musl/JS 符号 stub。
#[cfg(target_arch = "wasm32")]
mod wasm_z3_shims;

/// shim 日志裸导出 re-export（供 wasm cdylib 带出；guard 拦不到的观测口）。
#[cfg(target_arch = "wasm32")]
pub use wasm_z3_shims::{shim_log_len, shim_log_ptr};

#[cfg(test)]
mod tests;
