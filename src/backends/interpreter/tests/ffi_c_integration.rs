//! FFI C ABI 集成测试
//!
//! 验证 C ABI 调用在 Windows 和 Unix 上能正常工作。
//! 使用系统库中无参数的纯函数（PID 获取）进行测试。

use crate::backends::common::Heap;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::ffi::FfiRegistry;
use crate::std::NativeContext;

#[test]
fn test_c_ffi_call_getpid() {
    let mut registry = FfiRegistry::new();
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);

    if cfg!(target_os = "windows") {
        registry.load_library("kernel32.dll").unwrap();

        // GetCurrentProcessId: () -> DWORD (u32)
        // Phase 1 只支持 () -> i32 签名，Windows 的 DWORD 就是 u32/i32
        let result = registry.call_with_mechanism(
            "c",
            "kernel32.dll",
            "GetCurrentProcessId",
            "",
            &[],
            &mut ctx,
        );

        assert!(result.is_ok());
        match result.unwrap() {
            RuntimeValue::Int(pid) => {
                assert!(pid > 0, "PID should be positive, got {pid}");
            }
            other => panic!("Expected Int, got {:?}", other),
        }
    } else if cfg!(target_os = "linux") {
        registry.load_library("libc.so.6").unwrap();

        // getpid: () -> pid_t (i32)
        let result = registry.call_with_mechanism(
            "c",
            "libc.so.6",
            "getpid",
            "",
            &[],
            &mut ctx,
        );

        assert!(result.is_ok());
        match result.unwrap() {
            RuntimeValue::Int(pid) => {
                assert!(pid > 0, "PID should be positive, got {pid}");
            }
            other => panic!("Expected Int, got {:?}", other),
        }
    } else if cfg!(target_os = "macos") {
        registry.load_library("libc.dylib").unwrap();

        let result = registry.call_with_mechanism(
            "c",
            "libc.dylib",
            "getpid",
            "",
            &[],
            &mut ctx,
        );

        assert!(result.is_ok());
        match result.unwrap() {
            RuntimeValue::Int(pid) => {
                assert!(pid > 0, "PID should be positive, got {pid}");
            }
            other => panic!("Expected Int, got {:?}", other),
        }
    } else {
        // 无覆盖平台，测试无法运行
        eprintln!("Skipping C FFI test on unsupported platform");
    }
}
