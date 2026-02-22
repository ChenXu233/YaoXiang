//! Standard String library (YaoXiang)
//!
//! This module provides string manipulation functions for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::io::format_value_with_prefix;
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
            NativeExport::new(
                "format",
                "std.string.format",
                "(format: String, ...args) -> String",
                native_format as NativeHandler,
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

/// Native implementation: format - Python-style string formatting
/// Supports {0}, {1}, {2}... placeholders and {:03}, {:>3}, {:<3} format specifiers
fn native_format(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let format_str = args.get(0).map(extract_string).unwrap_or_default();
    let format_args = &args[1..];

    // Convert all args to strings upfront
    let arg_strings: Vec<String> = format_args
        .iter()
        .map(|arg| format_value_with_prefix(arg, ctx.heap, ""))
        .collect();

    // Parse and replace placeholders
    let result = parse_format(&format_str, &arg_strings);

    Ok(RuntimeValue::String(result.into()))
}

/// Parse format string and replace placeholders with argument values
fn parse_format(
    format_str: &str,
    args: &[String],
) -> String {
    let mut result = String::new();
    let mut chars = format_str.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            // Parse placeholder
            let mut placeholder = String::new();

            // Collect characters until closing brace
            while let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    break;
                }
                placeholder.push(chars.next().unwrap());
            }

            // Parse placeholder: {index} or {index:format}
            if let Some((index_str, format_spec)) = placeholder.split_once(':') {
                // Has format specifier: {0:03}
                let index: usize = index_str.parse().unwrap_or(0);
                let formatted = apply_format_spec(
                    args.get(index).map(|s| s.as_str()).unwrap_or(""),
                    format_spec,
                );
                result.push_str(&formatted);
            } else {
                // Simple placeholder: {0}
                let index: usize = placeholder.parse().unwrap_or(0);
                result.push_str(args.get(index).map(|s| s.as_str()).unwrap_or(""));
            }
        } else if c == '}' {
            // Escape }} as }
            if let Some(&next) = chars.peek() {
                if next == '}' {
                    chars.next();
                    result.push('}');
                } else {
                    // Unmatched }, treat as literal
                    result.push('}');
                }
            } else {
                result.push('}');
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Apply format specifier to a value
/// Supported: {:03} (zero-pad), {:>3} (right-align), {:<3} (left-align), {:^3} (center)
fn apply_format_spec(
    value: &str,
    spec: &str,
) -> String {
    // Parse format spec: [align][width]
    // align: <, >, ^
    // width: number
    // fill (optional): character before align

    if spec.is_empty() {
        return value.to_string();
    }

    let mut fill_char = ' ';
    let mut align = '>'; // Default right-align for numbers

    let spec_chars: Vec<char> = spec.chars().collect();

    // Check for fill character (before align)
    let width_str = if spec_chars.len() >= 2 {
        match spec_chars[0] {
            '<' | '>' | '^' => {
                align = spec_chars[0];
                &spec[1..]
            }
            _ if spec_chars[0].is_ascii_digit() => {
                fill_char = spec_chars[0];
                if spec_chars.len() >= 2 {
                    match spec_chars[1] {
                        '<' | '>' | '^' => {
                            align = spec_chars[1];
                            &spec[2..]
                        }
                        _ => spec,
                    }
                } else {
                    spec
                }
            }
            _ => spec,
        }
    } else {
        spec
    };

    // Parse width
    let width: usize = width_str.parse().unwrap_or(0);

    if width == 0 {
        return value.to_string();
    }

    let len = value.len();

    if len >= width {
        return value.to_string();
    }

    let padding_len = width - len;
    let padding: String = fill_char.to_string().repeat(padding_len);

    match align {
        '<' => format!("{}{}", value, padding), // Left align
        '^' => {
            // Center align
            let left_pad = padding_len / 2;
            let right_pad = padding_len - left_pad;
            format!(
                "{}{}{}",
                padding.repeat(left_pad),
                value,
                padding.repeat(right_pad)
            )
        }
        _ => format!("{}{}", padding, value), // Right align (default)
    }
}
