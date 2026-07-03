//! Result 标准库模块
//!
//! 提供 `Result(T, E)` 类型的构造函数和实用方法，
//! 以及 `Error` 类型（作为 Result 的 Err 载体）。
//!
//! 运行时表示：
//! - Result.ok(value): RuntimeValue::Enum { type_id: ENUM, variant_id: 0, payload: value }
//! - Result.err(error): RuntimeValue::Enum { type_id: ENUM, variant_id: 1, payload: error }
//! - Error(msg): RuntimeValue::Struct { type_id: STRUCT, fields: [msg], vtable: [] }

use crate::backends::common::value::TypeId;
use crate::backends::common::{HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

pub struct ResultModule;

impl Default for ResultModule {
    fn default() -> Self {
        ResultModule
    }
}

impl StdModule for ResultModule {
    fn module_path(&self) -> &str {
        "std.result"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "is_ok",
                "std.result.is_ok",
                "(self: Result(T, E)) -> Bool",
                native_result_is_ok,
            ),
            NativeExport::new(
                "is_err",
                "std.result.is_err",
                "(self: Result(T, E)) -> Bool",
                native_result_is_err,
            ),
            NativeExport::new(
                "unwrap",
                "std.result.unwrap",
                "(self: Result(T, E)) -> T",
                native_result_unwrap,
            ),
            NativeExport::new(
                "unwrap_or",
                "std.result.unwrap_or",
                "(self: Result(T, E), default: T) -> T",
                native_result_unwrap_or,
            ),
        ]
    }
}

pub const RESULT_MODULE: ResultModule = ResultModule;

// ============================================================================
// 公共辅助函数（供 parse_int/parse_float 等复用）
// ============================================================================

/// 构造 Result.ok(value)，variant_id=0
pub fn result_ok(value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Enum {
        type_id: TypeId::ENUM,
        variant_id: 0,
        payload: Box::new(value),
    }
}

/// 构造 Result.err(error)，variant_id=1
pub fn result_err(error: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Enum {
        type_id: TypeId::ENUM,
        variant_id: 1,
        payload: Box::new(error),
    }
}

/// 构造 Error 值（Struct { message: String }），使用 ctx.heap 分配字段
pub fn error_new(
    message: &str,
    ctx: &mut NativeContext<'_>,
) -> RuntimeValue {
    let field_values = vec![RuntimeValue::String(message.into())];
    let handle = ctx.heap.allocate(HeapValue::Tuple(field_values));
    RuntimeValue::Struct {
        type_id: TypeId::STRUCT,
        fields: handle,
        vtable: vec![],
    }
}

// ============================================================================
// Result 方法 native 实现
// ============================================================================

pub(crate) fn native_result_is_ok(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Enum { variant_id: 0, .. }) => Ok(RuntimeValue::Bool(true)),
        _ => Ok(RuntimeValue::Bool(false)),
    }
}

pub(crate) fn native_result_is_err(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Enum { variant_id: 1, .. }) => Ok(RuntimeValue::Bool(true)),
        _ => Ok(RuntimeValue::Bool(false)),
    }
}

pub(crate) fn native_result_unwrap(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Enum {
            variant_id: 0,
            payload,
            ..
        }) => Ok((**payload).clone()),
        _ => Err(ExecutorError::runtime_only("unwrap called on Err value")),
    }
}

pub(crate) fn native_result_unwrap_or(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Enum {
            variant_id: 0,
            payload,
            ..
        }) => Ok((**payload).clone()),
        _ => Ok(args.get(1).cloned().unwrap_or(RuntimeValue::Unit)),
    }
}
