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
fn test_with_std_has_io_functions() {
    let registry = FfiRegistry::with_std();
    assert!(!registry.is_empty());
    // Only fully qualified names are registered by default
    assert!(registry.has("std.io.print"));
    assert!(registry.has("std.io.println"));
    assert!(registry.has("std.io.read_line"));
    assert!(registry.has("std.io.read_file"));
    assert!(registry.has("std.io.write_file"));
    assert!(registry.has("std.io.append_file"));
    // Short names are NOT registered - users must use `use std.io` to bring them into scope
    assert!(!registry.has("print"));
    assert!(!registry.has("println"));
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
fn test_call_custom_function() {
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

    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = registry
        .call(
            "my_add",
            &[RuntimeValue::Int(3), RuntimeValue::Int(7)],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(result, RuntimeValue::Int(10));
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
    // println should accept any number of args and return Unit
    let result = registry
        .call(
            "std.io.println",
            &[RuntimeValue::String("hello from FFI".into())],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(result, RuntimeValue::Unit);
}

#[test]
fn test_repeated_calls_work() {
    let mut registry = FfiRegistry::new();
    fn identity(
        args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        Ok(args.first().cloned().unwrap_or(RuntimeValue::Unit))
    }
    registry.register("identity", identity);

    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let r1 = registry
        .call("identity", &[RuntimeValue::Int(42)], &mut ctx)
        .unwrap();
    assert_eq!(r1, RuntimeValue::Int(42));

    let r2 = registry
        .call("identity", &[RuntimeValue::Int(99)], &mut ctx)
        .unwrap();
    assert_eq!(r2, RuntimeValue::Int(99));
}

#[test]
fn test_register_overwrites_existing() {
    let mut registry = FfiRegistry::new();
    fn handler_v1(
        _args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        Ok(RuntimeValue::Int(1))
    }
    fn handler_v2(
        _args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        Ok(RuntimeValue::Int(2))
    }
    registry.register("func", handler_v1);
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let r1 = registry.call("func", &[], &mut ctx).unwrap();
    assert_eq!(r1, RuntimeValue::Int(1));

    registry.register("func", handler_v2);
    let r2 = registry.call("func", &[], &mut ctx).unwrap();
    assert_eq!(r2, RuntimeValue::Int(2));
}

#[test]
fn test_registered_functions_list() {
    let mut registry = FfiRegistry::new();
    fn noop(
        _args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        Ok(RuntimeValue::Unit)
    }
    registry.register("alpha", noop);
    registry.register("beta", noop);

    let names = registry.registered_functions();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
    assert_eq!(names.len(), 2);
}

#[test]
fn test_write_and_read_file() {
    let registry = FfiRegistry::with_std();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let test_path = std::env::temp_dir().join("yx_ffi_test.txt");
    let path_str = test_path.to_string_lossy().to_string();

    // Cleanup before test
    let _ = std::fs::remove_file(&test_path);

    // Write file
    let write_result = registry
        .call(
            "std.io.write_file",
            &[
                RuntimeValue::String(path_str.clone().into()),
                RuntimeValue::String("FFI test content".into()),
            ],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(write_result, RuntimeValue::Bool(true));

    // Read file
    let read_result = registry
        .call(
            "std.io.read_file",
            &[RuntimeValue::String(path_str.clone().into())],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(read_result, RuntimeValue::String("FFI test content".into()));

    // Append file
    let append_result = registry
        .call(
            "std.io.append_file",
            &[
                RuntimeValue::String(path_str.clone().into()),
                RuntimeValue::String(" appended".into()),
            ],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(append_result, RuntimeValue::Bool(true));

    // Read again to verify append
    let read_result2 = registry
        .call(
            "std.io.read_file",
            &[RuntimeValue::String(path_str.clone().into())],
            &mut ctx,
        )
        .unwrap();
    assert_eq!(
        read_result2,
        RuntimeValue::String("FFI test content appended".into())
    );

    // Cleanup
    let _ = std::fs::remove_file(&test_path);
}

#[test]
fn test_read_file_missing_args() {
    let registry = FfiRegistry::with_std();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = registry.call("std.io.read_file", &[], &mut ctx);
    assert!(result.is_err());
}

#[test]
fn test_write_file_missing_args() {
    let registry = FfiRegistry::with_std();
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = registry.call(
        "std.io.write_file",
        &[RuntimeValue::String("path".into())],
        &mut ctx,
    );
    assert!(result.is_err());
}

// =============================================================================
// std.result 和 std.string.parse_int/parse_float 测试
// =============================================================================

use crate::std::result::{
    error_new, native_result_is_err, native_result_is_ok, native_result_unwrap,
    native_result_unwrap_or, result_err, result_ok,
};
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
    let err = result_err(error_new("fail", &mut ctx));

    // Act
    let result = native_result_unwrap(&[err], &mut ctx);

    // Assert
    assert!(
        result.is_err(),
        "unwrap on an Err value should return ExecutorError"
    );
}

/// 规范 §1.3：Result.ok → is_ok === true
#[test]
fn test_result_is_ok_on_ok_returns_true() {
    // Arrange
    let ok = result_ok(RuntimeValue::Int(1));
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);

    // Act
    let result = native_result_is_ok(&[ok], &mut ctx).unwrap();

    // Assert
    assert_eq!(
        result,
        RuntimeValue::Bool(true),
        "is_ok on an Ok value should be true"
    );
}

/// 规范 §1.3：Result.err → is_ok === false
#[test]
fn test_result_is_ok_on_err_returns_false() {
    // Arrange
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let err = result_err(error_new("x", &mut ctx));

    // Act
    let result = native_result_is_ok(&[err], &mut ctx).unwrap();

    // Assert
    assert_eq!(
        result,
        RuntimeValue::Bool(false),
        "is_ok on an Err value should be false"
    );
}

/// 规范 §1.3：Result.err(e) → unwrap_or(default) === default
#[test]
fn test_result_unwrap_or_with_err_returns_default() {
    // Arrange
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let err = result_err(error_new("x", &mut ctx));

    // Act
    let result = native_result_unwrap_or(&[err, RuntimeValue::Int(0)], &mut ctx).unwrap();

    // Assert
    assert_eq!(
        result,
        RuntimeValue::Int(0),
        "unwrap_or on an Err should return the default value"
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

/// 规范 §4.2：parse_int("abc") → Result.err(Error)
#[test]
fn test_parse_int_returns_err_for_invalid_input() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_int(&[RuntimeValue::String("abc".into())], &mut ctx).unwrap();
    assert_eq!(
        native_result_is_err(&[result], &mut ctx).unwrap(),
        RuntimeValue::Bool(true),
        "parse_int('abc') should return an Err"
    );
}

/// 规范 §4.2：parse_int("") → Result.err(Error)
#[test]
fn test_parse_int_returns_err_for_empty_string() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_int(&[RuntimeValue::String("".into())], &mut ctx).unwrap();
    assert_eq!(
        native_result_is_err(&[result], &mut ctx).unwrap(),
        RuntimeValue::Bool(true),
        "parse_int('') should return an Err"
    );
}

/// 规范 §4.2：parse_int(" -7 ") → Result.ok(-7)（自动 trim + 负数）
#[test]
fn test_parse_int_parses_negative_number_with_whitespace() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_int(&[RuntimeValue::String(" -7 ".into())], &mut ctx).unwrap();
    let val = native_result_unwrap(&[result], &mut ctx).unwrap();
    assert_eq!(
        val,
        RuntimeValue::Int(-7),
        "parse_int(' -7 ') should return Ok(-7) despite whitespace"
    );
}

/// 规范 §4.2：parse_float("3.14") → Result.ok(3.14)
#[test]
#[allow(clippy::approx_constant)]
fn test_parse_float_returns_ok_for_valid_float() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result = native_parse_float(&[RuntimeValue::String("3.14".into())], &mut ctx).unwrap();
    let val = native_result_unwrap(&[result], &mut ctx).unwrap();
    assert_eq!(
        val,
        RuntimeValue::Float(3.14),
        "parse_float('3.14') should return Ok(3.14)"
    );
}

/// 规范 §4.2：parse_float("not_a_number") → Result.err(Error)
#[test]
fn test_parse_float_returns_err_for_invalid_input() {
    let mut heap = Heap::new();
    let mut ctx = test_ctx(&mut heap);
    let result =
        native_parse_float(&[RuntimeValue::String("not_a_number".into())], &mut ctx).unwrap();
    assert_eq!(
        native_result_is_err(&[result], &mut ctx).unwrap(),
        RuntimeValue::Bool(true),
        "parse_float('not_a_number') should return an Err"
    );
}
