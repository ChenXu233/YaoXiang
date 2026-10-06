//! Standard String library (YaoXiang)
//!
//! This module provides string manipulation functions for YaoXiang programs.

use crate::backends::common::{Heap, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::io::format_value_with_prefix;
use crate::std::{NativeContext, NativeExport, StdModule};
use crate::std::result::{error_new, result_err, result_ok};

// StringModule - StdModule Implementation

/// String module implementation.
#[derive(Default)]
pub struct StringModule;

impl StdModule for StringModule {
    fn module_path(&self) -> &str {
        "std.string"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!(
                "split",
                "std.string.split",
                "(s: &String, sep: &String) -> Vec(String)",
                native_split
            ),
            export!(
                "trim",
                "std.string.trim",
                "(s: &String) -> String",
                native_trim
            ),
            export!(
                "upper",
                "std.string.upper",
                "(s: &String) -> String",
                native_upper
            ),
            export!(
                "lower",
                "std.string.lower",
                "(s: &String) -> String",
                native_lower
            ),
            export!(
                "replace",
                "std.string.replace",
                "(s: &String, old: &String, new: &String) -> String",
                native_replace
            ),
            export!(
                "contains",
                "std.string.contains",
                "(s: &String, sub: &String) -> Bool",
                native_contains
            ),
            export!(
                "starts_with",
                "std.string.starts_with",
                "(s: &String, prefix: &String) -> Bool",
                native_starts_with
            ),
            export!(
                "ends_with",
                "std.string.ends_with",
                "(s: &String, suffix: &String) -> Bool",
                native_ends_with
            ),
            export!(
                "index_of",
                "std.string.index_of",
                "(s: &String, sub: &String) -> Int",
                native_index_of
            ),
            export!(
                "substring",
                "std.string.substring",
                "(s: &String, start: Int, end: Int) -> String",
                native_substring
            ),
            export!(
                "is_empty",
                "std.string.is_empty",
                "(s: &String) -> Bool",
                native_is_empty
            ),
            export!("len", "std.string.len", "(s: &String) -> Int", native_len),
            export!(
                "chars",
                "std.string.chars",
                "(s: &String) -> Vec(String)",
                native_chars
            ),
            export!(
                "concat",
                "std.string.concat",
                "(s1: &String, s2: &String) -> String",
                native_concat
            ),
            export!(
                "repeat",
                "std.string.repeat",
                "(s: &String, n: Int) -> String",
                native_repeat
            ),
            export!(
                "reverse",
                "std.string.reverse",
                "(s: &String) -> String",
                native_reverse
            ),
            export!(
                "format",
                "std.string.format",
                "(format: &String, ...args) -> String",
                native_format
            ),
            export!(
                "parse_int",
                "std.string.parse_int",
                "(s: &String) -> Result(Int, Error)",
                native_parse_int
            ),
            export!(
                "parse_float",
                "std.string.parse_float",
                "(s: &String) -> Result(Float, Error)",
                native_parse_float
            ),
            export!(
                "char_code",
                "std.string.char_code",
                "(s: &String, i: Int) -> Int",
                native_char_code
            ),
            export!(
                "from_char_code",
                "std.string.from_char_code",
                "(n: Int) -> Result(String, Error)",
                native_from_char_code
            ),
        ]
    }
}

/// Singleton instance for std.string module.
pub const STRING_MODULE: StringModule = StringModule;

// Helper functions

/// Extract String from RuntimeValue
pub(crate) fn extract_string(arg: &RuntimeValue) -> String {
    match arg {
        RuntimeValue::String(s) => s.to_string(),
        _ => String::new(),
    }
}

/// Extract Int from RuntimeValue
fn extract_int(arg: &RuntimeValue) -> i64 {
    arg.to_int().unwrap_or(0)
}

// Native function implementations

/// Native implementation: split - split string by separator
/// Now uses ctx.heap to allocate a proper List
fn native_split(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
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
    let s = args.first().map(extract_string).unwrap_or_default();
    let trimmed = s.trim();
    Ok(RuntimeValue::String(trimmed.to_string().into()))
}

/// Native implementation: upper - convert to uppercase
fn native_upper(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let upper = s.to_uppercase();
    Ok(RuntimeValue::String(upper.into()))
}

/// Native implementation: lower - convert to lowercase
fn native_lower(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let lower = s.to_lowercase();
    Ok(RuntimeValue::String(lower.into()))
}

/// Native implementation: replace - replace all occurrences
fn native_replace(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
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
    let s = args.first().map(extract_string).unwrap_or_default();
    let sub = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.contains(&sub)))
}

/// Native implementation: starts_with - check prefix
fn native_starts_with(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let prefix = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.starts_with(&prefix)))
}

/// Native implementation: ends_with - check suffix
fn native_ends_with(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let suffix = args.get(1).map(extract_string).unwrap_or_default();

    Ok(RuntimeValue::Bool(s.ends_with(&suffix)))
}

/// Native implementation: index_of - find substring position
/// #385 定案 D1/D2：返回标量值（码点）下标——`s.find` 给字节偏移，换算成
/// 前缀字符数，与 substring/char_code/s[i] 同域，组合使用不再在多字节
/// 字符处错位。找不到返回 -1。此前返回字节偏移，是三套口径分裂的成员之一。
fn native_index_of(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let sub = args.get(1).map(extract_string).unwrap_or_default();

    match s.find(&sub) {
        Some(pos) => Ok(RuntimeValue::Int(s[..pos].chars().count() as i64)),
        None => Ok(RuntimeValue::Int(-1)),
    }
}

/// Native implementation: substring - extract substring by indices
fn native_substring(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    // #385：端点缺省为字符数（标量值口径）；此前缺省 `s.len()` 是字节
    // 长度，靠下方 clamp 恰好等价，这里改成显式同口径。
    let start = args.get(1).map(extract_int).unwrap_or(0) as usize;
    let end = args
        .get(2)
        .map(extract_int)
        .unwrap_or(s.chars().count() as i64) as usize;

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
    let s = args.first().map(extract_string).unwrap_or_default();
    Ok(RuntimeValue::Bool(s.is_empty()))
}

/// Native implementation: len - get string length
/// #385 定案 D1/D2：Unicode 标量值（码点）口径，与 substring/chars/
/// char_code/index_of 同单位；此前返回 UTF-8 字节长度，是三套口径分裂
/// 的成员之一。UTF-8 承载下为 O(n) 扫描。
fn native_len(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    Ok(RuntimeValue::Int(s.chars().count() as i64))
}

/// Native implementation: chars - get character list
/// Uses ctx.heap to allocate a proper List
fn native_chars(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
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
    let s1 = args.first().map(extract_string).unwrap_or_default();
    let s2 = args.get(1).map(extract_string).unwrap_or_default();

    let result = format!("{}{}", s1, s2);
    Ok(RuntimeValue::String(result.into()))
}

/// Native implementation: repeat - repeat string n times
fn native_repeat(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let n = args.get(1).map(extract_int).unwrap_or(0) as usize;

    let result = s.repeat(n);
    Ok(RuntimeValue::String(result.into()))
}

/// Native implementation: reverse - reverse string
fn native_reverse(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let reversed: String = s.chars().rev().collect();
    Ok(RuntimeValue::String(reversed.into()))
}

/// Native implementation: format - Python-style string formatting (RFC-012)
///
/// Placeholders `{0}`, `{1}`, ... with optional format spec `{0:.2f}`;
/// `{{` / `}}` are literal-brace escapes. Arguments stay typed (#402) so
/// numeric format specs see the underlying value instead of a pre-stringified
/// one.
fn native_format(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let format_str = args.first().map(extract_string).unwrap_or_default();

    let format_args = args.get(1..).unwrap_or(&[]);

    let result = parse_format(&format_str, format_args, ctx.heap)?;

    Ok(RuntimeValue::String(result.into()))
}

/// Parse format string and replace placeholders with argument values.
///
/// `{{` / `}}` are literal-brace escapes (Python-consistent, #402);
/// `{index}` / `{index:spec}` are placeholders. An unmatched `{` / `}` stays
/// literal. Missing indices render as empty.
fn parse_format(
    format_str: &str,
    args: &[RuntimeValue],
    heap: &Heap,
) -> Result<String, ExecutorError> {
    let mut result = String::new();
    let mut chars = format_str.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                // Escaped {{ → literal {
                if chars.peek() == Some(&'{') {
                    chars.next();

                    result.push('{');

                    continue;
                }

                // Collect placeholder until closing brace
                let mut placeholder = String::new();

                let mut closed = false;

                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();

                        closed = true;

                        break;
                    }
                    placeholder.push(chars.next().unwrap());
                }

                if !closed {
                    // Unterminated `{`: keep literal
                    result.push('{');

                    result.push_str(&placeholder);

                    continue;
                }

                // {index} or {index:spec}
                let (index_str, spec) = match placeholder.split_once(':') {
                    Some((idx, s)) => (idx, Some(s)),

                    None => (placeholder.as_str(), None),
                };

                let index: usize = index_str.parse().unwrap_or(0);

                let formatted = match (args.get(index), spec) {
                    (Some(value), Some(s)) => apply_format_spec(value, s, heap)?,

                    (Some(value), None) => format_value_with_prefix(value, heap, ""),

                    (None, _) => String::new(),
                };

                result.push_str(&formatted);
            }
            '}' => {
                // Escaped }} → literal }; a lone } stays literal
                if chars.peek() == Some(&'}') {
                    chars.next();
                }

                result.push('}');
            }
            c => result.push(c),
        }
    }

    Ok(result)
}

/// Python-style format specifier (RFC-012, #402):
/// `[[fill]align][sign][#][0][width][.precision][type]`
///
/// type ∈ {b c d e E f F g G o s x X %}; omitted type uses the default
/// display of the value's type. Invalid specifiers raise a runtime error
/// instead of being silently ignored.
fn apply_format_spec(
    value: &RuntimeValue,
    spec: &str,
    heap: &Heap,
) -> Result<String, ExecutorError> {
    let chars: Vec<char> = spec.chars().collect();

    let mut i = 0usize;

    // [[fill]align]
    let mut fill: Option<char> = None;

    let mut align: Option<char> = None;

    if chars.len() >= 2 && matches!(chars[1], '<' | '>' | '^') {
        fill = Some(chars[0]);

        align = Some(chars[1]);

        i = 2;
    } else if !chars.is_empty() && matches!(chars[0], '<' | '>' | '^') {
        align = Some(chars[0]);

        i = 1;
    }

    // [sign]
    let mut sign = '-';

    if i < chars.len() && matches!(chars[i], '+' | '-' | ' ') {
        sign = chars[i];

        i += 1;
    }

    // [#] alternate form (0b/0o/0x prefix)
    let mut alt = false;

    if i < chars.len() && chars[i] == '#' {
        alt = true;

        i += 1;
    }

    // [0] zero flag (fill '0', '=' alignment for numbers)
    let mut zero_pad = false;

    if i < chars.len() && chars[i] == '0' {
        zero_pad = true;

        i += 1;
    }

    // [width]
    let width_start = i;

    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    let width: usize = chars[width_start..i]
        .iter()
        .collect::<String>()
        .parse()
        .unwrap_or(0);

    // [.precision]
    let mut precision: Option<usize> = None;

    if i < chars.len() && chars[i] == '.' {
        i += 1;

        let p_start = i;

        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        precision = Some(
            chars[p_start..i]
                .iter()
                .collect::<String>()
                .parse()
                .unwrap_or(0),
        );
    }

    // [type]
    let type_char = if i < chars.len() {
        Some(chars[i])
    } else {
        None
    };

    if i + 1 < chars.len() {
        return Err(ExecutorError::runtime_only(format!(
            "Invalid format specifier '{spec}'"
        )));
    }

    // (head, tail, numeric): head carries sign + alternate prefix so that
    // '=' alignment can pad between head and tail (Python style)
    let (head, tail, numeric) = match value {
        RuntimeValue::Int(n) => int_parts(*n, type_char, sign, alt, precision, spec)?,

        RuntimeValue::Float(f) => float_parts(*f, type_char, sign, precision, spec)?,

        RuntimeValue::Bool(b) => match type_char {
            None | Some('s') => (String::new(), b.to_string(), false),

            Some(t) => return Err(unknown_code_err(t, "bool", spec)),
        },

        RuntimeValue::Char(c) => match type_char {
            None | Some('s') | Some('c') => (
                String::new(),
                char::from_u32(*c)
                    .map(|ch| ch.to_string())
                    .unwrap_or_default(),
                false,
            ),

            Some(t) => return Err(unknown_code_err(t, "char", spec)),
        },

        RuntimeValue::String(s) => match type_char {
            None | Some('s') => {
                let mut text: String = s.to_string();

                if let Some(p) = precision {
                    text = text.chars().take(p).collect();
                }

                (String::new(), text, false)
            }

            Some(t) => return Err(unknown_code_err(t, "str", spec)),
        },

        other => match type_char {
            None | Some('s') => (
                String::new(),
                format_value_with_prefix(other, heap, ""),
                false,
            ),

            Some(t) => return Err(unknown_code_err(t, "value", spec)),
        },
    };

    // Default alignment: numbers right ('=' under the zero flag), text left
    let align = align.unwrap_or(match (numeric, zero_pad) {
        (true, true) => '=',

        (true, false) => '>',

        (false, _) => '<',
    });

    let fill = fill.unwrap_or(if zero_pad { '0' } else { ' ' });

    let body_len = head.chars().count() + tail.chars().count();

    if width <= body_len {
        return Ok(format!("{head}{tail}"));
    }

    let pad = width - body_len;

    let padding = fill.to_string().repeat(pad);

    Ok(match align {
        '<' => format!("{head}{tail}{padding}"),

        '^' => {
            let left = pad / 2;

            format!(
                "{}{head}{tail}{}",
                fill.to_string().repeat(left),
                fill.to_string().repeat(pad - left)
            )
        }

        '=' => format!("{head}{padding}{tail}"),

        _ => format!("{padding}{head}{tail}"),
    })
}

fn unknown_code_err(
    code: char,
    type_name: &str,
    spec: &str,
) -> ExecutorError {
    ExecutorError::runtime_only(format!(
        "Unknown format code '{code}' for {type_name} in format specifier '{spec}'"
    ))
}

fn sign_char(
    negative: bool,
    sign: char,
) -> &'static str {
    if negative {
        "-"
    } else {
        match sign {
            '+' => "+",

            ' ' => " ",

            _ => "",
        }
    }
}

/// Integer body per Python rules; returns (sign+prefix head, digits tail).
/// Integers accept the float presentations (e/E/f/F/g/G/%) like Python does.
fn int_parts(
    n: i64,
    type_char: Option<char>,
    sign: char,
    alt: bool,
    precision: Option<usize>,
    spec: &str,
) -> Result<(String, String, bool), ExecutorError> {
    let negative = n < 0;

    let mag = n.unsigned_abs();

    let (digits, prefix) = match type_char {
        None | Some('d') | Some('n') => (mag.to_string(), ""),

        Some('b') => (format!("{mag:b}"), if alt { "0b" } else { "" }),

        Some('o') => (format!("{mag:o}"), if alt { "0o" } else { "" }),

        Some('x') => (format!("{mag:x}"), if alt { "0x" } else { "" }),

        Some('X') => (format!("{mag:X}"), if alt { "0X" } else { "" }),

        Some('e') | Some('E') | Some('f') | Some('F') | Some('g') | Some('G') | Some('%') => {
            return float_parts(n as f64, type_char, sign, precision, spec);
        }

        Some('c') => {
            // 负数没有对应字符，不能用绝对值顶替
            if negative {
                return Err(ExecutorError::runtime_only(format!(
                    "Invalid codepoint {n} in format specifier '{spec}'"
                )));
            }

            return match u32::try_from(mag).ok().and_then(char::from_u32) {
                Some(ch) => Ok((String::new(), ch.to_string(), false)),

                None => Err(ExecutorError::runtime_only(format!(
                    "Invalid codepoint {mag} in format specifier '{spec}'"
                ))),
            };
        }

        Some(t) => return Err(unknown_code_err(t, "int", spec)),
    };

    // Python: precision on integers = minimum digit count (zero-fill)
    let digits = match precision {
        Some(p) if digits.chars().count() < p => {
            format!("{}{digits}", "0".repeat(p - digits.chars().count()))
        }

        _ => digits,
    };

    Ok((
        format!("{}{prefix}", sign_char(negative, sign)),
        digits,
        true,
    ))
}

/// Float body; returns (sign head, magnitude tail).
fn float_parts(
    f: f64,
    type_char: Option<char>,
    sign: char,
    precision: Option<usize>,
    spec: &str,
) -> Result<(String, String, bool), ExecutorError> {
    let negative = f.is_sign_negative() && !f.is_nan();

    let mag = f.abs();

    let head = sign_char(negative, sign).to_string();

    let tail = match type_char {
        // 'n'：locale 感知数字的本地化占位（当前 locale 中立，同缺省）
        None | Some('n') => match precision {
            // Python: precision without type = significant digits (g)
            Some(p) => format_g(mag, p.max(1), false),

            None => {
                // 与 format_value_with_prefix 的 Float 展示一致：整数值带一位小数
                if mag.is_finite() && mag.fract() == 0.0 {
                    format!("{mag:.1}")
                } else {
                    format!("{mag}")
                }
            }
        },

        Some('f') => format_f(mag, precision.unwrap_or(6), false),

        Some('F') => format_f(mag, precision.unwrap_or(6), true),

        Some('e') => format_e(mag, precision.unwrap_or(6), false),

        Some('E') => format_e(mag, precision.unwrap_or(6), true),

        Some('g') => format_g(mag, precision.unwrap_or(6).max(1), false),

        Some('G') => format_g(mag, precision.unwrap_or(6).max(1), true),

        Some('%') => format_percent(mag, precision.unwrap_or(6)),

        Some(t) => return Err(unknown_code_err(t, "float", spec)),
    };

    Ok((head, tail, true))
}

fn format_f(
    mag: f64,
    precision: usize,
    upper: bool,
) -> String {
    if mag.is_infinite() {
        return if upper { "INF" } else { "inf" }.to_string();
    }

    if mag.is_nan() {
        return if upper { "NAN" } else { "nan" }.to_string();
    }

    format!("{mag:.precision$}")
}

/// Fixed precision, exponent normalized to Python style: `e+00` (min 2 digits)
fn format_e(
    mag: f64,
    precision: usize,
    upper: bool,
) -> String {
    if mag.is_infinite() {
        return if upper { "INF" } else { "inf" }.to_string();
    }

    if mag.is_nan() {
        return if upper { "NAN" } else { "nan" }.to_string();
    }

    let s = format!("{mag:.precision$e}");

    let (mantissa, exp_str) = s.split_once('e').unwrap_or((s.as_str(), "0"));

    let exp: i32 = exp_str.parse().unwrap_or(0);

    let sep = if upper { 'E' } else { 'e' };

    let sign = if exp < 0 { '-' } else { '+' };

    format!("{mantissa}{sep}{sign}{:02}", exp.abs())
}

/// Python `g`: significant digits, e-style when exp < -4 or exp >= precision,
/// trailing zeros stripped
fn format_g(
    mag: f64,
    precision: usize,
    upper: bool,
) -> String {
    if mag.is_infinite() {
        return if upper { "INF" } else { "inf" }.to_string();
    }

    if mag.is_nan() {
        return if upper { "NAN" } else { "nan" }.to_string();
    }

    if mag == 0.0 {
        return "0".to_string();
    }

    let e = format!("{:.*e}", precision - 1, mag);

    let (_, exp_str) = e.split_once('e').unwrap_or((e.as_str(), "0"));

    let exp: i32 = exp_str.parse().unwrap_or(0);

    if exp < -4 || exp >= precision as i32 {
        let mantissa = format!("{:.*e}", precision - 1, mag);

        let (mantissa, _) = mantissa.split_once('e').unwrap_or((mantissa.as_str(), "0"));

        let sep = if upper { 'E' } else { 'e' };

        let sign = if exp < 0 { '-' } else { '+' };

        format!("{}{sep}{sign}{:02}", trim_numeric(mantissa), exp.abs())
    } else {
        let frac_digits = (precision as i32 - 1 - exp).max(0) as usize;

        trim_numeric(&format!("{mag:.frac_digits$}"))
    }
}

fn format_percent(
    mag: f64,
    precision: usize,
) -> String {
    if mag.is_infinite() {
        return "inf%".to_string();
    }

    if mag.is_nan() {
        return "nan%".to_string();
    }

    format!("{:.precision$}%", mag * 100.0)
}

/// Strip trailing zeros (and a trailing dot) from a fixed-notation number
fn trim_numeric(s: &str) -> String {
    if s.contains('.') {
        let t = s.trim_end_matches('0');

        let t = t.strip_suffix('.').unwrap_or(t);

        t.to_string()
    } else {
        s.to_string()
    }
}

// Native implementations: parse_int / parse_float

/// Native implementation: parse_int - parse string to Int
pub(crate) fn native_parse_int(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args
        .first()
        .map(|v| match v {
            RuntimeValue::String(s) => s.as_ref().to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    match s.trim().parse::<i64>() {
        Ok(n) => Ok(result_ok(RuntimeValue::Int(n))),
        Err(e) => Ok(result_err(error_new(
            "E6010",
            &format!("parse_int: {}", e),
            ctx,
        ))),
    }
}

/// Native implementation: parse_float - parse string to Float
pub(crate) fn native_parse_float(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args
        .first()
        .map(|v| match v {
            RuntimeValue::String(s) => s.as_ref().to_string(),
            _ => String::new(),
        })
        .unwrap_or_default();
    match s.trim().parse::<f64>() {
        Ok(f) => {
            if f.is_finite() {
                Ok(result_ok(RuntimeValue::Float(f)))
            } else {
                Ok(result_err(error_new(
                    "E6011",
                    "parse_float: result is not finite",
                    ctx,
                )))
            }
        }
        Err(e) => Ok(result_err(error_new(
            "E6011",
            &format!("parse_float: {}", e),
            ctx,
        ))),
    }
}

// Native implementations: char_code / from_char_code

/// Native implementation: char_code - 取第 i 个字符的码点（Unicode 标量值口径，
/// 与 substring/chars 同域）。越界返回 -1（与 index_of 未命中同一惯例；
/// 码点非负，-1 无歧义）。
fn native_char_code(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let s = args.first().map(extract_string).unwrap_or_default();
    let i = args.get(1).map(extract_int).unwrap_or(0);
    let code = s
        .chars()
        .nth(i.max(0) as usize)
        .map(|c| c as u32 as i64)
        .unwrap_or(-1);
    Ok(RuntimeValue::Int(code))
}

/// Native implementation: from_char_code - 码点转单字符字符串。
/// 非法码点（负数 / 代理区 / > U+10FFFF）返回 E6012；校验用 `char::from_u32`，
/// 与词法层 `\u{...}` 转义同一套判定。0x10000..=0x10FFFF 直接产出合法
/// UTF-8（4 字节），UTF-16 代理对组合留给 yx 层。
fn native_from_char_code(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().map(extract_int).unwrap_or(0);
    match u32::try_from(n).ok().and_then(char::from_u32) {
        Some(c) => Ok(result_ok(RuntimeValue::String(c.to_string().into()))),
        None => Ok(result_err(error_new(
            "E6012",
            &format!("from_char_code: invalid codepoint {}", n),
            ctx,
        ))),
    }
}
