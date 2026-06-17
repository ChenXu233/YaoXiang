//! Feature gate 测试 — 基于 Feature Flag 重构设计
//!
//! 验证 cfg gate 转换正确：
//! - CLI 模块 (lsp, repl, package) 由 cfg(not(target_arch = "wasm32")) 门控
//! - wasm 模块 (playground) 由 cfg(target_arch = "wasm32") 门控
//! - Z3 模块在非 wasm32 target 下默认可用

/// CLI 模块应该在非 wasm32 target 下可用
#[test]
fn test_cli_modules_available_on_native() {
    // Arrange & Act & Assert
    // 在非 wasm32 target 下，这些模块应该存在
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 验证 lsp 模块存在
        let _ = crate::lsp::capabilities::server_capabilities();
        // 验证 repl 模块存在
        let _ = crate::repl::Repl::new();
    }
}

/// wasm 模块应该在 wasm32 target 下可用
#[test]
fn test_wasm_modules_available_on_wasm32() {
    // Arrange & Act & Assert
    #[cfg(target_arch = "wasm32")]
    {
        // 验证 playground 模块存在
        let result = crate::playground::ping();
        assert_eq!(result, "pong", "playground::ping() 应该返回 'pong'");
    }
}

/// Z3 模块应该在非 wasm32 target 下可用（不被额外 feature gate 排除）
#[test]
fn test_z3_modules_available_on_native() {
    // Arrange & Act
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = crate::frontend::core::typecheck::proof::smt::z3_backend::Z3Backend::new();
        use crate::frontend::core::typecheck::proof::smt::z3_ffi::Z3_context;
        let _: Z3_context = std::ptr::null_mut();
    }

    // Assert - 如果编译通过，说明 Z3 模块没有被额外 cfg gate 排除
}
