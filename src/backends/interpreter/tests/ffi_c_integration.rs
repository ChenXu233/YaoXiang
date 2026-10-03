//! C ABI FFI 集成测试 — 基于 RFC-026 §3.1（临时区隔离编组）
//!
//! §3.1: 默认走临时区复制（TempZone），`() -> i32` 签名直接走寄存器
//! §3.2: 编组规则表——Int32 直接放寄存器，零转换
//! §6:   运行时通过 libloading 实现 C 函数调用
//!
//! 覆盖场景：
//! - 系统库加载（kernel32.dll / libc.so.6 / libc.dylib）
//! - 无参 C 函数调用，i32 返回值
//! - 跨平台兼容（Windows / Linux / macOS）
//! - 失败场景：库不存在、符号不存在

use crate::backends::common::Heap;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::ffi::FfiRegistry;
use crate::std::NativeContext;

/// Helper: test_c_ffi_call_getpid 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 当前平台可测的系统库与符号名；不支持的平台返回 `None`。
fn pid_lib_and_symbol() -> Option<(&'static str, &'static str)> {
    if cfg!(target_os = "windows") {
        Some(("kernel32.dll", "GetCurrentProcessId"))
    } else if cfg!(target_os = "linux") {
        Some(("libc.so.6", "getpid"))
    } else if cfg!(target_os = "macos") {
        Some(("libc.dylib", "getpid"))
    } else {
        // 无法测试——不 panic，返回 None 由调用方跳过
        None
    }
}
/// Assert: 返回值是正数 PID（非 Int 或非正数即失败）。
fn assert_positive_pid(
    result: Result<RuntimeValue, crate::backends::ExecutorError>,
    lib_name: &str,
    sym_name: &str,
) {
    assert!(
        result.is_ok(),
        "C ABI call to {lib_name}::{sym_name} should succeed"
    );
    let pid = match result.expect("C ABI 调用结果应可取回 RuntimeValue") {
        RuntimeValue::Int(pid) => pid,
        other => panic!("Expected RuntimeValue::Int, got {other:?} for PID call"),
    };
    assert!(
        pid > 0,
        "PID from {lib_name}::{sym_name} should be positive, got {pid}"
    );
}

#[test]
fn test_c_ffi_call_getpid() {
    // Arrange: 运行时上下文 + 加载系统库（不支持的平台直接跳过）
    let mut registry = FfiRegistry::new();
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);
    let Some((lib_name, sym_name)) = pid_lib_and_symbol() else {
        return;
    };
    registry
        .load_library(lib_name)
        .expect("registry.load_library(lib_name) 应成功");

    // Act: 调用 C 函数 — RFC-026 §3.1 无参签名直接寄存器传递
    let result = registry.call_with_mechanism("c", lib_name, sym_name, "", &[], &mut ctx);

    // Assert: 返回值是正数 PID
    assert_positive_pid(result, lib_name, sym_name);
}

#[test]
fn test_c_ffi_call_nonexistent_library_returns_error() {
    // Arrange: 空注册表，无预加载
    let registry = FfiRegistry::new();
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);

    // Act: 调用一个不存在的库
    let result = registry.call_with_mechanism(
        "c",
        "nonexistent_lib_12345.so",
        "some_symbol",
        "",
        &[],
        &mut ctx,
    );

    // Assert: 应该返回错误，不是 panic
    assert!(
        result.is_err(),
        "Calling nonexistent library should return error"
    );
}

#[test]
fn test_c_ffi_call_nonexistent_symbol_returns_error() {
    // Arrange: 加载真实系统库
    let mut registry = FfiRegistry::new();

    let lib_name = if cfg!(target_os = "windows") {
        "kernel32.dll"
    } else if cfg!(target_os = "linux") {
        "libc.so.6"
    } else if cfg!(target_os = "macos") {
        "libc.dylib"
    } else {
        return;
    };

    registry.load_library(lib_name).unwrap();
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);

    // Act: 调用一个不存在的符号
    let result = registry.call_with_mechanism(
        "c",
        lib_name,
        "symbol_that_does_not_exist_98765",
        "",
        &[],
        &mut ctx,
    );

    // Assert: 符号不存在返回错误
    assert!(
        result.is_err(),
        "Calling nonexistent symbol on {lib_name} should return error"
    );
}
