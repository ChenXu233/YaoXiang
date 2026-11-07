//! FFI 注册表测试
//!
//! 对应规范章节：
//! - `docs/src/reference/language-spec/stdlib.md` §1.3: Result 标准库模块
//! - `docs/src/reference/language-spec/stdlib.md` §4.2: parse_int / parse_float
//!
//! 测试覆盖内容：
//! - FfiRegistry 的创建和配置
//! - 标准库函数的注册
//! - 自定义函数的注册和调用
//! - 文件读写操作
//! - 错误处理
//! - std.result 模块（is_ok, is_err, unwrap, unwrap_or）
//! - std.string.parse_int / parse_float

use crate::backends::common::RuntimeValue;
use crate::backends::common::Heap;
use crate::backends::ExecutorError;
use crate::backends::interpreter::ffi::FfiRegistry;
use crate::std::NativeContext;

/// Helper to create a test NativeContext
fn test_ctx(heap: &mut Heap) -> NativeContext<'_> {
    NativeContext::new(heap)
}

#[test]
fn test_new_registry_is_empty() {
    let registry = FfiRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn test_register_custom_function() {
    let mut registry = FfiRegistry::new();
    fn my_add(
        args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
        Ok(RuntimeValue::Int(a + b))
    }
    registry.register("my_add", my_add);
    assert!(registry.has("my_add"));
    assert_eq!(registry.len(), 1);
}

#[test]
fn test_call_nonexistent_function_returns_error() {
    let registry = FfiRegistry::new();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = registry.call("nonexistent", &[], &mut ctx);
    assert!(result.is_err());
    match result {
        Err(ExecutorError::FunctionNotFound(msg, _stack)) => {
            assert!(msg.contains("nonexistent"));
        }
        _ => panic!("Expected FunctionNotFound error"),
    }
}

#[test]
fn test_call_println_via_registry() {
    let registry = FfiRegistry::with_std();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    // println should accept any number of args and return Void
    let result = registry
        .call(
            "std.io.println",
            &[RuntimeValue::String("hello from FFI".into())],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(
        result,
        RuntimeValue::Void,
        "println 应返回 Void（空值），符合 RFC-010a 空块/无值尾表达式的约定"
    );
}

#[test]
fn test_read_file_missing_args() {
    let registry = FfiRegistry::with_std();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = registry.call("std.fs.read_file", &[], &mut ctx);
    assert!(result.is_err());
}

// std.result 和 std.string.parse_int/parse_float 测试

use crate::std::result::{error_new, native_result_is_ok, native_result_unwrap, result_err, result_ok};
use crate::std::string::{native_parse_float, native_parse_int};

/// 规范 §1.3：Result.ok(value) → unwrap → value
#[test]
fn test_result_ok_unwrap() {
    // Arrange
    let ok = result_ok(RuntimeValue::Int(42));
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);

    // Act
    let result = native_result_unwrap(&[ok], &mut ctx).unwrap();

    // Assert
    assert_eq!(
        result,
        RuntimeValue::Int(42),
        "unwrap on an Ok(42) should return 42"
    );
}

/// 规范 §1.3：Result.err(e) → unwrap → runtime error
#[test]
fn test_result_err_unwrap_returns_error() {
    // Arrange
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let err = result_err(error_new("E6007", "fail", &mut ctx));

    // Act
    let result = native_result_unwrap(&[err], &mut ctx);

    // Assert
    assert!(
        result.is_err(),
        "unwrap on an Err value should return ExecutorError"
    );
}

/// 规范 §1.3：Result.err → is_ok === false
#[test]
fn test_result_is_ok_on_err_returns_false() {
    // Arrange
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let err = result_err(error_new("E6007", "x", &mut ctx));

    // Act
    let result = native_result_is_ok(&[err], &mut ctx).unwrap();

    // Assert
    assert_eq!(
        result,
        RuntimeValue::Bool(false),
        "is_ok on an Err value should be false"
    );
}

/// 规范 §4.2：parse_int("42") → Result.ok(42)
#[test]
fn test_parse_int_returns_ok_for_valid_integer() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_int(&[RuntimeValue::String("42".into())], &mut ctx).unwrap();
    let val = native_result_unwrap(&[result], &mut ctx).unwrap();
    assert_eq!(
        val,
        RuntimeValue::Int(42),
        "parse_int('42') should return Ok(42)"
    );
}

/// 规范 §4.2：parse_float("3.14") → Result.ok(3.14)
#[test]
#[allow(clippy::approx_constant)]
fn test_parse_float_returns_ok_for_valid_float() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_float(&[RuntimeValue::String("3.14".into())], &mut ctx)
        .expect("native_parse_float(\"3.14\") 应成功");
    let val = native_result_unwrap(&[result], &mut ctx)
        .expect("let val = native_result_unwrap(&[result], &mut ctx) 应成功");
    assert_eq!(
        val,
        RuntimeValue::Float(3.14),
        "parse_float('3.14') should return Ok(3.14)"
    );
}
