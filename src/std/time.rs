//! Standard Time library (YaoXiang)
//!
//! This module provides time-related functionality for YaoXiang programs.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// TimeModule - StdModule Implementation

/// Time module implementation.
#[derive(Default)]
pub struct TimeModule;

impl StdModule for TimeModule {
    fn module_path(&self) -> &str {
        "std.time"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            export!("now", "std.time.now", "() -> DateTime", native_now),
            export!(
                "timestamp",
                "std.time.timestamp",
                "() -> Int",
                native_timestamp
            ),
            export!(
                "timestamp_ms",
                "std.time.timestamp_ms",
                "() -> Int",
                native_timestamp_ms
            ),
            #[cfg(not(target_arch = "wasm32"))]
            export!(
                "sleep",
                "std.time.sleep",
                "(seconds: Float) -> Void",
                native_sleep
            ),
            export!(
                "format_time",
                "std.time.format_time",
                "(dt: Int, fmt: String) -> String",
                native_format_time
            ),
            export!(
                "parse_time",
                "std.time.parse_time",
                "(fmt: String, s: String) -> DateTime",
                native_parse_time
            ),
            export!(
                "datetime_year",
                "std.time.datetime_year",
                "(dt: Int) -> Int",
                native_datetime_year
            ),
            export!(
                "datetime_month",
                "std.time.datetime_month",
                "(dt: Int) -> Int",
                native_datetime_month
            ),
            export!(
                "datetime_day",
                "std.time.datetime_day",
                "(dt: Int) -> Int",
                native_datetime_day
            ),
            export!(
                "datetime_hour",
                "std.time.datetime_hour",
                "(dt: Int) -> Int",
                native_datetime_hour
            ),
            export!(
                "datetime_minute",
                "std.time.datetime_minute",
                "(dt: Int) -> Int",
                native_datetime_minute
            ),
            export!(
                "datetime_second",
                "std.time.datetime_second",
                "(dt: Int) -> Int",
                native_datetime_second
            ),
            export!(
                "datetime_weekday",
                "std.time.datetime_weekday",
                "(dt: Int) -> Int",
                native_datetime_weekday
            ),
            export!(
                "datetime_to_string",
                "std.time.datetime_to_string",
                "(dt: Int) -> String",
                native_datetime_to_string
            ),
        ]
    }
}

/// Singleton instance for std.time module.
pub const TIME_MODULE: TimeModule = TimeModule;

// Helper Functions

/// Get current Unix timestamp in seconds.
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Convert timestamp to datetime components (local time).
/// Returns (year, month, day, hour, minute, second, weekday, timestamp)
fn timestamp_to_datetime(timestamp: u64) -> (i64, i64, i64, i64, i64, i64, i64, i64) {
    let secs = timestamp as i64;

    // Days since epoch
    let days = secs / 86400;
    // Seconds of day
    let secs_of_day = secs % 86400;

    // Hour, minute, second
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    // Calculate year, month, day from days since epoch
    let days_since_epoch = days;

    // 求年份与年内偏移（“1970-01-01 起第 N 天” → year + day-of-year）。
    //
    // 旧实现先取近似年 `1970 + days/365`，却把 `remaining_days` 仍初始化为
    // **全部** days_since_epoch（而非减去近似年之前的那些天），于是循环在近似年
    // 之上又加了一遍年份——1705276800（2024-01-15）被算成 2078-01-14。
    // 注意 `timestamp_to_datetime` 假设输入非负，故 days 也非负。
    let mut year = 1970i64;
    let mut remaining_days = days_since_epoch;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Calculate month and day
    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in month_days.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }
    let day = remaining_days + 1;

    // Calculate weekday (0 = Sunday, 6 = Saturday)
    // 1970-01-01 is Thursday (4)
    let weekday = (days_since_epoch + 4) % 7;

    (year, month, day, hour, minute, second, weekday, secs)
}

/// Check if a year is a leap year.
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get timestamp from argument (accepts Int or treats missing as current time).
fn get_timestamp_arg(
    args: &[RuntimeValue],
    default_to_now: bool,
) -> Result<u64, ExecutorError> {
    if args.is_empty() {
        if default_to_now {
            Ok(get_current_timestamp())
        } else {
            Err(ExecutorError::runtime_only(
                "Expected timestamp argument".to_string(),
            ))
        }
    } else {
        match &args[0] {
            RuntimeValue::Int(ts) => Ok(*ts as u64),
            other => Err(ExecutorError::type_only(format!(
                "Expected Int timestamp, got {:?}",
                other
            ))),
        }
    }
}

/// Calculate days since Unix epoch.
fn days_since_epoch(
    year: i64,
    month: i64,
    day: i64,
) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };

    365 * y + y / 4 - y / 100 + y / 400 + (153 * (m - 3) + 2) / 5 + day - 719469
}

/// Calculate Unix timestamp from components.
fn calculate_timestamp(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
) -> i64 {
    let days = days_since_epoch(year, month, day);

    days * 86400 + hour * 3600 + minute * 60 + second
}

// Time Getting Functions

/// Native implementation: now
fn native_now(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let timestamp = get_current_timestamp();
    Ok(RuntimeValue::Int(timestamp as i64))
}

/// Native implementation: timestamp
fn native_timestamp(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let timestamp = get_current_timestamp();
    Ok(RuntimeValue::Int(timestamp as i64))
}

/// Native implementation: timestamp_ms
fn native_timestamp_ms(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64;
    Ok(RuntimeValue::Int(timestamp))
}

// Time Sleeping Function

/// Native implementation: sleep
#[cfg(not(target_arch = "wasm32"))]
fn native_sleep(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "sleep expects 1 argument (seconds: Float)".to_string(),
        ));
    }

    let seconds = match &args[0] {
        RuntimeValue::Float(f) => *f,
        RuntimeValue::Int(i) => *i as f64,
        other => {
            return Err(ExecutorError::type_only(format!(
                "sleep expects Float or Int argument, got {:?}",
                other
            )))
        }
    };

    std::thread::sleep(Duration::from_secs_f64(seconds));
    Ok(RuntimeValue::Void)
}

// Time Formatting and Parsing Functions

/// Native implementation: format_time
fn native_format_time(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::runtime_only(
            "format_time expects 2 arguments (timestamp: Int, fmt: String)".to_string(),
        ));
    }

    let timestamp = match &args[0] {
        RuntimeValue::Int(ts) => *ts as u64,
        other => {
            return Err(ExecutorError::type_only(format!(
                "format_time expects Int timestamp, got {:?}",
                other
            )))
        }
    };

    let fmt = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "format_time expects String format, got {:?}",
                other
            )))
        }
    };

    let (year, month, day, hour, minute, second, weekday, _) = timestamp_to_datetime(timestamp);

    // Simple strftime-like formatting
    let result = fmt
        .replace("%Y", &format!("{:04}", year))
        .replace("%m", &format!("{:02}", month))
        .replace("%d", &format!("{:02}", day))
        .replace("%H", &format!("{:02}", hour))
        .replace("%M", &format!("{:02}", minute))
        .replace("%S", &format!("{:02}", second))
        .replace("%w", &weekday.to_string())
        .replace("%F", &format!("{}-{:02}-{:02}", year, month, day))
        .replace("%T", &format!("{:02}:{:02}:{:02}", hour, minute, second));

    Ok(RuntimeValue::String(result.into()))
}

/// Native implementation: parse_time
fn native_parse_time(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::runtime_only(
            "parse_time expects 2 arguments (fmt: String, s: String)".to_string(),
        ));
    }

    let fmt = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "parse_time expects String format, got {:?}",
                other
            )))
        }
    };

    let s = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "parse_time expects String argument, got {:?}",
                other
            )))
        }
    };

    // 按 fmt 的指示符切分输入——与 `format_time` 支持同一组指示符（互为逆运算）。
    //
    // 此前 `fmt` 绑定为 `_fmt` 后**从未使用**：只硬识别 ISO 8601。
    // 于是 `parse_time("%d/%m/%Y", "15/01/2024")` 失败，而
    // `parse_time("totally-bogus", "2024-01-15T10:30:00")` 反而成功（#340）。
    let (year, month, day, hour, minute, second) = parse_by_format(&fmt, &s)?;

    let timestamp = calculate_timestamp(year, month, day, hour, minute, second);

    Ok(RuntimeValue::Int(timestamp))
}

/// 按 `fmt` 的指示符从 `s` 中提取时间字段。
///
/// 与 `format_time` 支持的指示符一一对应：
/// `%Y` `%m` `%d` `%H` `%M` `%S`，以及组合形 `%F`（`%Y-%m-%d`）
/// 与 `%T`（`%H:%M:%S`）。未出现的字段默认 1 月 1 日 0 时 0 分 0 秒。
fn parse_by_format(
    fmt: &str,
    s: &str,
) -> Result<(i64, i64, i64, i64, i64, i64), ExecutorError> {
    // 先把组合指示符展开成基本指示符，使后续只剩一种形态要处理
    let expanded = fmt.replace("%F", "%Y-%m-%d").replace("%T", "%H:%M:%S");
    let expanded_s = s.to_string();

    let mut year = 1970i64;
    let mut month = 1i64;
    let mut day = 1i64;
    let mut hour = 0i64;
    let mut minute = 0i64;
    let mut second = 0i64;

    // 步进扫描：fmt 的字面量字符必须与输入逐一匹配，指示符处提取数字
    let fmt_chars: Vec<char> = expanded.chars().collect();
    let s_chars: Vec<char> = expanded_s.chars().collect();
    let mut fi = 0usize;
    let mut si = 0usize;

    while fi < fmt_chars.len() {
        if fmt_chars[fi] == '%' && fi + 1 < fmt_chars.len() {
            let spec = fmt_chars[fi + 1];
            fi += 2;
            // 该指示符要抽取的位数；%Y 允许 4 位或更多
            let width = match spec {
                'Y' => 4,
                'm' | 'd' | 'H' | 'M' | 'S' => 2,
                _ => {
                    return Err(ExecutorError::runtime_only(format!(
                        "parse_time: unsupported format specifier '%{spec}'"
                    )))
                }
            };
            let mut digits = String::new();
            while si < s_chars.len() && digits.len() < width && s_chars[si].is_ascii_digit() {
                digits.push(s_chars[si]);
                si += 1;
            }
            if digits.is_empty() {
                return Err(ExecutorError::runtime_only(format!(
                    "Invalid time format: '{s}' does not match '{fmt}' (expected %{spec} at position {si})"
                )));
            }
            let value: i64 = digits.parse().unwrap_or(0);
            match spec {
                'Y' => year = value,
                'm' => month = value,
                'd' => day = value,
                'H' => hour = value,
                'M' => minute = value,
                'S' => second = value,
                _ => unreachable!(),
            }
        } else {
            // 字面量字符：必须匹配（跳过输入中的空白差异）
            if si >= s_chars.len() || s_chars[si] != fmt_chars[fi] {
                return Err(ExecutorError::runtime_only(format!(
                    "Invalid time format: '{s}' does not match '{fmt}'"
                )));
            }
            fi += 1;
            si += 1;
        }
    }

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ExecutorError::runtime_only(format!(
            "Invalid time format: month/day out of range in '{s}' (got {month}/{day})"
        )));
    }

    Ok((year, month, day, hour, minute, second))
}

// DateTime Accessor Functions

/// Get field from timestamp.
fn get_datetime_field(
    args: &[RuntimeValue],
    field_index: usize,
) -> Result<RuntimeValue, ExecutorError> {
    let timestamp = get_timestamp_arg(args, false)?;
    let (year, month, day, hour, minute, second, weekday, ts) = timestamp_to_datetime(timestamp);

    let fields = [year, month, day, hour, minute, second, weekday, ts];
    Ok(RuntimeValue::Int(fields[field_index]))
}

/// Native implementation: DateTime::year
fn native_datetime_year(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 0)
}

/// Native implementation: DateTime::month
fn native_datetime_month(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 1)
}

/// Native implementation: DateTime::day
fn native_datetime_day(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 2)
}

/// Native implementation: DateTime::hour
fn native_datetime_hour(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 3)
}

/// Native implementation: DateTime::minute
fn native_datetime_minute(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 4)
}

/// Native implementation: DateTime::second
fn native_datetime_second(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 5)
}

/// Native implementation: DateTime::weekday
fn native_datetime_weekday(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    get_datetime_field(args, 6)
}

/// Native implementation: DateTime::to_string
fn native_datetime_to_string(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let timestamp = get_timestamp_arg(args, false)?;
    let (year, month, day, hour, minute, second, _, _) = timestamp_to_datetime(timestamp);

    let result = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    );

    Ok(RuntimeValue::String(result.into()))
}
