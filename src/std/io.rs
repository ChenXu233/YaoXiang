//! Standard IO library (YaoXiang)
//!
//! This module provides input/output functionality for YaoXiang programs.
//! All IO functions are declared as `Native("std.io.xxx")` bindings, meaning
//! their actual implementations live in the FFI registry.

use std::io::BufRead;

use crate::backends::common::{RuntimeValue, HeapValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// IoModule - StdModule Implementation
// ============================================================================

/// IO module implementation.
pub struct IoModule;

impl Default for IoModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for IoModule {
    fn module_path(&self) -> &str {
        "std.io"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new("print", "std.io.print", "(...args) -> ()", native_print),
            NativeExport::new(
                "println",
                "std.io.println",
                "(...args) -> ()",
                native_println,
            ),
            NativeExport::new(
                "read_line",
                "std.io.read_line",
                "() -> String",
                native_read_line,
            ),
            NativeExport::new(
                "read_file",
                "std.io.read_file",
                "(path: String) -> String",
                native_read_file,
            ),
            NativeExport::new(
                "write_file",
                "std.io.write_file",
                "(path: String, content: String) -> Bool",
                native_write_file,
            ),
            NativeExport::new(
                "append_file",
                "std.io.append_file",
                "(path: String, content: String) -> Bool",
                native_append_file,
            ),
            NativeExport::new(
                "format_fallback",
                "std.io.format_fallback",
                "(value, type_name: String) -> String",
                native_format_fallback,
            ),
        ]
    }
}

/// Singleton instance for std::io module.
pub const IO_MODULE: IoModule = IoModule;

// ============================================================================
// Native Function Implementations
// ============================================================================

/// Native implementation: print (without newline)
fn native_print(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let output = args
        .iter()
        .map(|arg| format_runtime_value(arg, ctx.heap))
        .collect::<Vec<String>>()
        .join(" ");
    print!("{}", output);
    Ok(RuntimeValue::Unit)
}

/// Native implementation: println (with newline)
fn native_println(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let output = args
        .iter()
        .map(|arg| format_runtime_value(arg, ctx.heap))
        .collect::<Vec<String>>()
        .join(" ");
    println!("{}", output);
    Ok(RuntimeValue::Unit)
}

/// Format a runtime value, resolving heap references for List/Dict/Tuple
fn format_runtime_value(
    val: &RuntimeValue,
    heap: &crate::backends::common::Heap,
) -> String {
    // 使用公共格式化函数，prefix 为空
    format_value_with_prefix(val, heap, "")
}

/// 公共格式化函数（模块内共享）：格式化值，可选添加类型前缀
/// 如果 prefix 为空，返回基础表示（如 "123"）
/// 如果 prefix 不为空，返回带前缀的表示（如 "int(123)"）
pub(crate) fn format_value_with_prefix(
    val: &RuntimeValue,
    heap: &crate::backends::common::Heap,
    prefix: &str,
) -> String {
    let prefix_fn = |s: &str| {
        if prefix.is_empty() {
            s.to_string()
        } else {
            format!("{}({})", prefix, s)
        }
    };

    match val {
        RuntimeValue::Unit => prefix_fn("unit"),
        RuntimeValue::Bool(b) => prefix_fn(&b.to_string()),
        RuntimeValue::Int(i) => prefix_fn(&i.to_string()),
        RuntimeValue::Float(f) => {
            let s = if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                f.to_string()
            };
            prefix_fn(&s)
        }
        RuntimeValue::Char(c) => {
            let s = char::from_u32(*c)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| format!("U+{:04X}", c));
            prefix_fn(&s)
        }
        RuntimeValue::String(s) => {
            // String 类型不添加前缀，直接返回内容
            s.to_string()
        }
        RuntimeValue::Bytes(b) => prefix_fn(&format!("bytes[{}]", b.len())),
        RuntimeValue::Tuple(handle) => {
            if let Some(HeapValue::Tuple(items)) = heap.get(*handle) {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|item| format_value_with_prefix(item, heap, ""))
                    .collect();
                let s = format!("({})", items_str.join(", "));
                prefix_fn(&s)
            } else {
                prefix_fn(&format!("tuple@{}", handle.raw()))
            }
        }
        RuntimeValue::Array(handle) => {
            if let Some(HeapValue::Array(arr)) = heap.get(*handle) {
                let items_str: Vec<String> = arr
                    .iter()
                    .map(|item| format_value_with_prefix(item, heap, ""))
                    .collect();
                let s = format!("[{}]", items_str.join(", "));
                prefix_fn(&s)
            } else {
                prefix_fn(&format!("array@{}", handle.raw()))
            }
        }
        RuntimeValue::List(handle) => {
            if let Some(HeapValue::List(items)) = heap.get(*handle) {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|item| format_value_with_prefix(item, heap, ""))
                    .collect();
                let s = format!("[{}]", items_str.join(", "));
                prefix_fn(&s)
            } else {
                prefix_fn(&format!("list@{}", handle.raw()))
            }
        }
        RuntimeValue::Dict(handle) => {
            if let Some(HeapValue::Dict(entries)) = heap.get(*handle) {
                let entries_str: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            format_value_with_prefix(k, heap, ""),
                            format_value_with_prefix(v, heap, "")
                        )
                    })
                    .collect();
                let s = format!("{{{}}}", entries_str.join(", "));
                prefix_fn(&s)
            } else {
                prefix_fn(&format!("dict@{}", handle.raw()))
            }
        }
        RuntimeValue::Struct { fields, .. } => prefix_fn(&format!("struct@{}", fields.raw())),
        RuntimeValue::Enum { variant_id, .. } => prefix_fn(&format!("enum::v{}", variant_id)),
        RuntimeValue::Function(_) => prefix_fn("function"),
        RuntimeValue::Arc(inner) => prefix_fn(&format!("arc({})", inner)),
        RuntimeValue::Weak(_) => prefix_fn("weak(...)"),
        RuntimeValue::Async(_) => prefix_fn("async"),
        RuntimeValue::Ptr { kind, address, .. } => {
            prefix_fn(&format!("ptr({:?}, {:#x})", kind, address))
        }
    }
}

/// Native implementation: format_fallback
/// Formats a value using the fallback type info (used when type doesn't implement Stringable)
fn native_format_fallback(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Ok(RuntimeValue::String("()".into()));
    }

    let type_name = args
        .get(1)
        .and_then(|v| match v {
            RuntimeValue::String(s) => Some(s.as_ref()),
            _ => None,
        })
        .unwrap_or("unknown");

    let formatted = format_value_with_prefix(&args[0], ctx.heap, type_name);

    Ok(RuntimeValue::String(formatted.into()))
}

/// Native implementation: read_line
fn native_read_line(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| ExecutorError::Runtime(format!("Failed to read line: {}", e)))?;
    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(RuntimeValue::String(line.into()))
}

/// Native implementation: read_file
fn native_read_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "read_file expects 1 argument (path: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "read_file expects String argument, got {:?}",
                other.value_type(None)
            )));
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(RuntimeValue::String(content.into())),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to read file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: write_file
fn native_write_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "write_file expects 2 arguments (path: String, content: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "write_file expects String path, got {:?}",
                other.value_type(None)
            )));
        }
    };
    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "write_file expects String content, got {:?}",
                other.value_type(None)
            )));
        }
    };
    match std::fs::write(&path, &content) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to write file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: append_file
fn native_append_file(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    use std::io::Write;

    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "append_file expects 2 arguments (path: String, content: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String path, got {:?}",
                other.value_type(None)
            )));
        }
    };
    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String content, got {:?}",
                other.value_type(None)
            )));
        }
    };
    match std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(()) => Ok(RuntimeValue::Bool(true)),
            Err(e) => Err(ExecutorError::Runtime(format!(
                "Failed to append to file '{}': {}",
                path, e
            ))),
        },
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to open file '{}' for appending: {}",
            path, e
        ))),
    }
}
