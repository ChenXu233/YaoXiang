//! Result 标准库模块
//!
//! 提供 `Result(T, E)` 类型的构造函数和实用方法，
//! 以及 `Error` 类型（作为 Result 的 Err 载体）。
//!
//! 运行时表示（#323 M4：Error 携带规范化错误码）：
//! - Result.ok(value): RuntimeValue::Enum { type_id: ENUM, variant_id: 0, payload: value }
//! - Result.err(error): RuntimeValue::Enum { type_id: ENUM, variant_id: 1, payload: error }
//! - Error: RuntimeValue::Struct { type_id: STRUCT, fields: [code, message], vtable: [] }
//!   - code: RFC-013 E6xxx/E7xxx 段注册码（跨版本稳定契约）
//!   - message: 人类可读描述
//!
//! 运行时错误值码注册表：[`RUNTIME_ERROR_CODES`]（与 RFC-013 码表、locales 三方对齐，
//! 由 scripts/check_error_codes.py 校验）。

use crate::backends::common::value::TypeId;
use crate::backends::common::{HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

/// 运行时错误值码注册表（码 → 默认语义）。
///
/// 码复用 RFC-013 E6xxx（运行时错误）段位；新码按真实触发面分配、先注册后使用。
pub static RUNTIME_ERROR_CODES: &[(&str, &str)] = &[
    ("E6009", "invalid range step（range 步长非法，如 step=0）"),
    ("E6010", "parse_int failed（整数解析失败）"),
    ("E6011", "parse_float failed（浮点解析失败）"),
];

#[derive(Default)]
pub struct ResultModule;

impl StdModule for ResultModule {
    fn module_path(&self) -> &str {
        "std.result"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "is_ok",
                "std.result.is_ok",
                "(T: Type, E: Type)(self: &Result(T, E)) -> Bool",
                native_result_is_ok
            ),
            export!(
                "is_err",
                "std.result.is_err",
                "(T: Type, E: Type)(self: &Result(T, E)) -> Bool",
                native_result_is_err
            ),
            export!(
                "unwrap",
                "std.result.unwrap",
                "(T: Type, E: Type)(self: &Result(T, E)) -> T",
                native_result_unwrap
            ),
            export!(
                "unwrap_or",
                "std.result.unwrap_or",
                "(T: Type, E: Type)(self: &Result(T, E), default: T) -> T",
                native_result_unwrap_or
            ),
            // #301：ok/err 构造器——用户代码重组 Result 值（? 解包后的 Ok 路径
            // 需要 result.ok(t) 重新包装才能沿 Result 返回类型传播）
            export!(
                "ok",
                "std.result.ok",
                "(T: Type, E: Type)(value: T) -> Result(T, E)",
                native_result_ok
            ),
            export!(
                "err",
                "std.result.err",
                "(T: Type, E: Type)(error: E) -> Result(T, E)",
                native_result_err
            ),
            // #323 M4：错误值可观测面——unwrap_err 取出 Err 载体，
            // code/message 读取规范化错误码与消息
            export!(
                "unwrap_err",
                "std.result.unwrap_err",
                "(T: Type, E: Type)(self: &Result(T, E)) -> E",
                native_result_unwrap_err
            ),
            export!(
                "code",
                "std.result.code",
                "(self: &Error) -> String",
                native_result_error_code
            ),
            export!(
                "message",
                "std.result.message",
                "(self: &Error) -> String",
                native_result_error_message
            ),
        ]
    }
}

pub const RESULT_MODULE: ResultModule = ResultModule;

// 公共辅助函数（供 parse_int/parse_float 等复用）

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

/// 构造 Error 值（Struct { code, message }），使用 ctx.heap 分配字段
pub fn error_new(
    code: &str,
    message: &str,
    ctx: &mut NativeContext<'_>,
) -> RuntimeValue {
    let field_values = vec![
        RuntimeValue::String(code.into()),
        RuntimeValue::String(message.into()),
    ];
    let handle = ctx.heap.allocate(HeapValue::Tuple(field_values));
    RuntimeValue::Struct {
        type_id: TypeId::STRUCT,
        fields: handle,
        vtable: vec![],
    }
}

// Result 方法 native 实现

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
        _ => Ok(args.get(1).cloned().unwrap_or(RuntimeValue::Void)),
    }
}

pub(crate) fn native_result_ok(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(result_ok(
        args.first().cloned().unwrap_or(RuntimeValue::Void),
    ))
}

pub(crate) fn native_result_err(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(result_err(
        args.first().cloned().unwrap_or(RuntimeValue::Void),
    ))
}

// #323 M4：错误值可观测面

pub(crate) fn native_result_unwrap_err(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Enum {
            variant_id: 1,
            payload,
            ..
        }) => Ok((**payload).clone()),
        _ => Err(ExecutorError::runtime_only("unwrap_err called on Ok value")),
    }
}

/// 读取 Error 值的字段（fields: [code, message]，堆上 Tuple）
fn error_field(
    args: &[RuntimeValue],
    index: usize,
) -> Result<RuntimeValue, ExecutorError> {
    match args.first() {
        Some(RuntimeValue::Struct {
            type_id: TypeId::STRUCT,
            fields,
            ..
        }) => {
            let value = fields.lock();
            match &*value {
                HeapValue::Tuple(v) if index < v.len() => Ok(v[index].clone()),
                _ => Err(ExecutorError::runtime_only(
                    "Error value has unexpected shape (expected [code, message])",
                )),
            }
        }
        _ => Err(ExecutorError::runtime_only(
            "code/message called on non-Error value",
        )),
    }
}

pub(crate) fn native_result_error_code(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    error_field(args, 0)
}

pub(crate) fn native_result_error_message(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    error_field(args, 1)
}
