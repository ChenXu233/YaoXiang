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
    match val {
        RuntimeValue::List(handle) => {
            if let Some(HeapValue::List(items)) = heap.get(*handle) {
                let items_str: Vec<String> = items
                    .iter()
                    .map(|item| format_runtime_value(item, heap))
                    .collect();
                format!("[{}]", items_str.join(", "))
            } else {
                format!("list@{}", handle.raw())
            }
        }
        RuntimeValue::Dict(handle) => {
            if let Some(HeapValue::Dict(entries)) = heap.get(*handle) {
                let entries_str: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "{}: {}",
                            format_runtime_value(k, heap),
                            format_runtime_value(v, heap)
                        )
                    })
                    .collect();
                format!("{{{}}}", entries_str.join(", "))
            } else {
                format!("dict@{}", handle.raw())
            }
        }
        _ => format!("{}", val),
    }
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
