//! Standard String library (YaoXiang)
//!
//! This module provides string manipulation functions for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule, NativeHandler};

// ============================================================================
// StringModule - StdModule Implementation
// ============================================================================

/// String module implementation.
pub struct StringModule;

impl Default for StringModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for StringModule {
    fn module_path(&self) -> &str {
        "std.string"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "split",
                "std.string.split",
                "(s: String, sep: String) -> List",
                native_split as NativeHandler,
            ),
            NativeExport::new(
                "trim",
                "std.string.trim",
                "(s: String) -> String",
                native_trim as NativeHandler,
            ),
            NativeExport::new(
                "upper",
                "std.string.upper",
                "(s: String) -> String",
                native_upper as NativeHandler,
            ),
            NativeExport::new(
                "lower",
                "std.string.lower",
                "(s: String) -> String",
                native_lower as NativeHandler,
            ),
            NativeExport::new(
                "replace",
                "std.string.replace",
                "(s: String, old: String, new: String) -> String",
                native_replace as NativeHandler,
            ),
            NativeExport::new(
                "contains",
                "std.string.contains",
                "(s: String, sub: String) -> Bool",
                native_contains as NativeHandler,
            ),
            NativeExport::new(
                "starts_with",
                "std.string.starts_with",
                "(s: String, prefix: String) -> Bool",
                native_starts_with as NativeHandler,
            ),
            NativeExport::new(
                "ends_with",
                "std.string.ends_with",
                "(s: String, suffix: String) -> Bool",
                native_ends_with as NativeHandler,
            ),
            NativeExport::new(
                "index_of",
                "std.string.index_of",
                "(s: String, sub: String) -> Int",
                native_index_of as NativeHandler,
            ),
            NativeExport::new(
                "substring",
                "std.string.substring",
                "(s: String, start: Int, end: Int) -> String",
                native_substring as NativeHandler,
            ),
            NativeExport::new(
                "is_empty",
                "std.string.is_empty",
                "(s: String) -> Bool",
                native_is_empty as NativeHandler,
            ),
            NativeExport::new(
                "len",
                "std.string.len",
                "(s: String) -> Int",
                native_len as NativeHandler,
            ),
            NativeExport::new(
                "chars",
                "std.string.chars",
                "(s: String) -> List",
                native_chars as NativeHandler,
            ),
            NativeExport::new(
                "concat",
                "std.string.concat",
                "(s1: String, s2: String) -> String",
                native_concat as NativeHandler,
            ),
            NativeExport::new(
                "repeat",
                "std.string.repeat",
                "(s: String, n: Int) -> String",
                native_repeat as NativeHandler,
            ),
            NativeExport::new(
                "reverse",
                "std.string.reverse",
                "(s: String) -> String",
                native_reverse as NativeHandler,
            ),
        ]
    }
}

/// Singleton instance for std.string module.
pub const STRING_MODULE: StringModule = StringModule;

// ============================================================================
// Helper functions
// ============================================================================

/// Extract String from RuntimeValue
fn extract_string(arg: &RuntimeValue) -> String {
    match arg {
        RuntimeValue::String(s) => s.to_string(),
        _ => String::new(),
    }
}

/// Extract Int from RuntimeValue
fn extract_int(arg: &RuntimeValue) -> i64 {
    arg.to_int().unwrap_or(0)
}

// ============================================================================
// Native function implementations
// ============================================================================

/// Native implementation: split - split string by separator
/// Now uses ctx.heap to allocate a proper List
fn native_split(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let sep = args.get(1).map(extract_string).unwrap_or_default();

    let parts: Vec<RuntimeValue> = if sep.is_empty() {
        // Split by each character when separator is empty
        s.chars()
            .map(|c| RuntimeValue::String(c.to_string().into()))
            .collect()
    } else {
        s.split(&sep)
            .map(|p| RuntimeValue::String(p.to_string().into()))
            .collect()
    };

    let handle = ctx
        .heap
        .allocate(crate::backends::common::HeapValue::List(parts));
    Ok(RuntimeValue::List(handle))
}

/// Native implementation: trim - remove leading and trailing whitespace
fn native_trim(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let trimmed = s.trim();
    Ok(RuntimeValue::String(trimmed.to_string().into()))
}

/// Native implementation: upper - convert to uppercase
fn native_upper(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let upper = s.to_uppercase();
    Ok(RuntimeValue::String(upper.into()))
}

/// Native implementation: lower - convert to lowercase
fn native_lower(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let lower = s.to_lowercase();
    Ok(RuntimeValue::String(lower.into()))
}

/// Native implementation: replace - replace all occurrences
fn native_replace(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let old = args.get(1).map(extract_string).unwrap_or_default();
    let new = args.get(2).map(extract_string).unwrap_or_default();

    if old.is_empty() {
        // Edge case: empty separator
        return Ok(RuntimeValue::String(s.into()));
    }

    let replaced = s.replace(&old, &new);
    Ok(RuntimeValue::String(replaced.into()))
}

/// Native implementation: contains - check if substring exists
fn native_contains(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let sub = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.contains(&sub)))
}

/// Native implementation: starts_with - check prefix
fn native_starts_with(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let prefix = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.starts_with(&prefix)))
}

/// Native implementation: ends_with - check suffix
fn native_ends_with(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let suffix = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.ends_with(&suffix)))
}

/// Native implementation: index_of - find substring position
fn native_index_of(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let sub = args.get(1).map(extract_string).unwrap_or_default();

    match s.find(&sub) {
        Some(pos) => Ok(RuntimeValue::Int(pos as i64)),
        None => Ok(RuntimeValue::Int(-1)),
    }
}

/// Native implementation: substring - extract substring by indices
fn native_substring(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let start = args.get(1).map(extract_int).unwrap_or(0) as usize;
    let end = args.get(2).map(extract_int).unwrap_or(s.len() as i64) as usize;

    let chars: Vec<char> = s.chars().collect();
    let end = end.min(chars.len());
    let start = start.min(end);

    let substring: String = chars[start..end].iter().collect();
    Ok(RuntimeValue::String(substring.into()))
}

/// Native implementation: is_empty - check if string is empty
fn native_is_empty(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    Ok(RuntimeValue::Bool(s.is_empty()))
}

/// Native implementation: len - get string length
fn native_len(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    Ok(RuntimeValue::Int(s.len() as i64))
}

/// Native implementation: chars - get character list
/// Uses ctx.heap to allocate a proper List
fn native_chars(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let chars: Vec<RuntimeValue> = s
        .chars()
        .map(|c| RuntimeValue::String(c.to_string().into()))
        .collect();
    let handle = ctx
        .heap
        .allocate(crate::backends::common::HeapValue::List(chars));
    Ok(RuntimeValue::List(handle))
}

/// Native implementation: concat - concatenate two strings
fn native_concat(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s1 = args.get(0).map(extract_string).unwrap_or_default();
    let s2 = args.get(1).map(extract_string).unwrap_or_default();

    let result = format!("{}{}", s1, s2);
    Ok(RuntimeValue::String(result.into()))
}

/// Native implementation: repeat - repeat string n times
fn native_repeat(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let n = args.get(1).map(extract_int).unwrap_or(0) as usize;

    let result = s.repeat(n);
    Ok(RuntimeValue::String(result.into()))
}

/// Native implementation: reverse - reverse string
fn native_reverse(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.get(0).map(extract_string).unwrap_or_default();
    let reversed: String = s.chars().rev().collect();
    Ok(RuntimeValue::String(reversed.into()))
}
