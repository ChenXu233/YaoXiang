//! Feature gate 测试 — 基于 Feature Flag 重构设计
//!
//! 验证 Z3 模块在非 wasm32 target 下默认可用，不被额外 feature gate 排除。
//! 设计文档: docs/superpowers/specs/2026-06-11-feature-flag-refactor-design.md

use crate::frontend::core::typecheck::proof::smt::z3_backend::Z3Backend;

/// Z3Backend 应该在非 wasm32 target 下可用（不被额外 cfg gate 排除）
#[test]
fn test_z3_backend_always_available() {
    // Arrange & Act
    let _backend = Z3Backend::new();

    // Assert — Z3 可能未安装导致初始化失败，但类型必须存在且可构造
}

/// z3_ffi 模块的 FFI 类型应该在非 wasm32 target 下可用
#[test]
fn test_z3_ffi_types_always_available() {
    // Arrange & Act — 引用 z3_ffi 中的类型，验证模块没有被额外 cfg gate 排除
    use crate::frontend::core::typecheck::proof::smt::z3_ffi::Z3_context;

    // Assert — 如果编译通过，说明 z3_ffi 模块在非 wasm32 target 下可用
    let _: Z3_context = std::ptr::null_mut();
}
