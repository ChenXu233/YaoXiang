//! lib crate 测试 — 基于 Feature Flag 重构设计
//!
//! 验证 lib.rs 中的 cfg gate 转换正确：
//! - CLI 模块 (lsp, repl, package) 由 cfg(not(target_arch = "wasm32")) 门控
//! - wasm 模块 (playground) 由 cfg(target_arch = "wasm32") 门控
//! - Z3 模块在非 wasm32 target 下默认可用

mod feature_gate;
