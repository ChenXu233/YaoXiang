//! Standard conversion library (YaoXiang)
//!
//! This module provides type conversion functionality.
//! All conversion functions are declared as `Native("std.convert.xxx")` bindings.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::io::format_value_with_prefix;
use crate::std::{NativeContext, NativeExport, StdModule};

// ConvertModule - StdModule Implementation

/// Convert module implementation.
#[derive(Default)]
pub struct ConvertModule;

impl StdModule for ConvertModule {
    fn module_path(&self) -> &str {
        "std.convert"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "to_string",
                "std.convert.to_string",
                "(value) -> String",
                native_to_string
            ),
            // Built-in Stringable bindings for primitive/runtime core types.
            export!(
                "int.to_string",
                "int.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "float.to_string",
                "float.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "bool.to_string",
                "bool.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "char.to_string",
                "char.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "string.to_string",
                "string.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "list.to_string",
                "list.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "dict.to_string",
                "dict.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "tuple.to_string",
                "tuple.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "set.to_string",
                "set.to_string",
                "(self) -> String",
                native_to_string
            ),
            export!(
                "range.to_string",
                "range.to_string",
                "(self) -> String",
                native_to_string
            ),
        ]
    }
}

/// Singleton instance for std::convert module.
pub const CONVERT_MODULE: ConvertModule = ConvertModule;

// Stringable Interface (YaoXiang source code)

/// Stringable interface definition - can be used in YaoXiang code
/// Note: This is the interface definition that users can reference in their code.
/// The actual runtime behavior is implemented in native functions.
pub const STRINGABLE_INTERFACE: &str = r#"
/// Stringable interface - similar to Python's __str__
/// Types that implement this interface can customize their string representation
Stringable: Type = {
    /// Convert the value to a string
    to_string: (self: Self) -> String
}
"#;

// Native Function Implementations

/// Native implementation: to_string
/// Tries to get custom string representation, falls back to type info
fn native_to_string(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Ok(RuntimeValue::String("()".into()));
    }

    let arg = &args[0];
    let result = format_value_with_stringable(arg, ctx.heap);

    Ok(RuntimeValue::String(result.into()))
}

/// Format a value, trying Stringable first, then falling back to type info
/// 直接复用 io 模块的公共函数
fn format_value_with_stringable(
    val: &RuntimeValue,
    heap: &crate::backends::common::Heap,
) -> String {
    // 直接使用 io 模块的公共格式化函数，prefix 为空
    format_value_with_prefix(val, heap, "")
}
