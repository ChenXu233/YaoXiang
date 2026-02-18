//! Standard Time library (YaoXiang)
//!
//! This module provides time-related functionality for YaoXiang programs.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// TimeModule - StdModule Implementation
// ============================================================================

/// Time module implementation.
pub struct TimeModule;

impl Default for TimeModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for TimeModule {
    fn module_path(&self) -> &str {
        "std.time"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new("now", "std.time.now", "() -> DateTime", native_now),
            NativeExport::new(
                "timestamp",
                "std.time.timestamp",
                "() -> Int",
                native_timestamp,
            ),
            NativeExport::new(
                "timestamp_ms",
                "std.time.timestamp_ms",
                "() -> Int",
                native_timestamp_ms,
            ),
            NativeExport::new(
                "sleep",
                "std.time.sleep",
                "(seconds: Float) -> Void",
                native_sleep,
            ),
            NativeExport::new(
                "format_time",
                "std.time.format_time",
                "(dt: Int, fmt: String) -> String",
                native_format_time,
            ),
            NativeExport::new(
                "parse_time",
                "std.time.parse_time",
                "(fmt: String, s: String) -> DateTime",
                native_parse_time,
            ),
            NativeExport::new(
                "DateTime::year",
                "std.time.DateTime.year",
                "(dt: Int) -> Int",
                native_datetime_year,
            ),
            NativeExport::new(
                "DateTime::month",
                "std.time.DateTime.month",
                "(dt: Int) -> Int",
                native_datetime_month,
            ),
            NativeExport::new(
                "DateTime::day",
                "std.time.DateTime.day",
                "(dt: Int) -> Int",
                native_datetime_day,
            ),
            NativeExport::new(
                "DateTime::hour",
                "std.time.DateTime.hour",
                "(dt: Int) -> Int",
                native_datetime_hour,
            ),
            NativeExport::new(
                "DateTime::minute",
                "std.time.DateTime.minute",
                "(dt: Int) -> Int",
                native_datetime_minute,
            ),
            NativeExport::new(
                "DateTime::second",
                "std.time.DateTime.second",
                "(dt: Int) -> Int",
                native_datetime_second,
            ),
            NativeExport::new(
                "DateTime::weekday",
                "std.time.DateTime.weekday",
                "(dt: Int) -> Int",
                native_datetime_weekday,
            ),
            NativeExport::new(
                "DateTime::to_string",
                "std.time.DateTime.to_string",
                "(dt: Int) -> String",
                native_datetime_to_string,
            ),
        ]
    }
}

/// Singleton instance for std.time module.
pub const TIME_MODULE: TimeModule = TimeModule;

// ============================================================================
// Helper Functions
// ============================================================================

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
    let days_since_epoch = days as i64;

    // Approximate year
    let mut year = 1970 + days_since_epoch / 365;

    // Calculate day of year
    let mut remaining_days = days_since_epoch;

    // Adjust year until we find the right one
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
    let weekday = ((days_since_epoch + 4) % 7) as i64;

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
            Err(ExecutorError::Runtime(
                "Expected timestamp argument".to_string(),
            ))
        }
    } else {
        match &args[0] {
            RuntimeValue::Int(ts) => Ok(*ts as u64),
            other => Err(ExecutorError::Type(format!(
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

    let days = 365 * y + y / 4 - y / 100 + y / 400 + (153 * (m - 3) + 2) / 5 + day - 719469;
    days
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
    let secs = days * 86400 + hour * 3600 + minute * 60 + second;
    secs
}

// ============================================================================
// Time Getting Functions
// ============================================================================

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

// ============================================================================
// Time Sleeping Function
// ============================================================================

/// Native implementation: sleep
fn native_sleep(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "sleep expects 1 argument (seconds: Float)".to_string(),
        ));
    }

    let seconds = match &args[0] {
        RuntimeValue::Float(f) => *f,
        RuntimeValue::Int(i) => *i as f64,
        other => {
            return Err(ExecutorError::Type(format!(
                "sleep expects Float or Int argument, got {:?}",
                other
            )))
        }
    };

    std::thread::sleep(Duration::from_secs_f64(seconds));
    Ok(RuntimeValue::Unit)
}

// ============================================================================
// Time Formatting and Parsing Functions
// ============================================================================

/// Native implementation: format_time
fn native_format_time(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "format_time expects 2 arguments (timestamp: Int, fmt: String)".to_string(),
        ));
    }

    let timestamp = match &args[0] {
        RuntimeValue::Int(ts) => *ts as u64,
        other => {
            return Err(ExecutorError::Type(format!(
                "format_time expects Int timestamp, got {:?}",
                other
            )))
        }
    };

    let fmt = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
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
        return Err(ExecutorError::Runtime(
            "parse_time expects 2 arguments (fmt: String, s: String)".to_string(),
        ));
    }

    let _fmt = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "parse_time expects String format, got {:?}",
                other
            )))
        }
    };

    let s = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "parse_time expects String argument, got {:?}",
                other
            )))
        }
    };

    // Simple ISO 8601 format parsing: "2024-01-15T10:30:00"
    let parts: Vec<&str> = s.split(|c| c == 'T' || c == ' ').collect();

    if parts.len() < 2 {
        return Err(ExecutorError::Runtime(format!(
            "Invalid time format: {}",
            s
        )));
    }

    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() < 3 || time_parts.len() < 3 {
        return Err(ExecutorError::Runtime(format!(
            "Invalid time format: {}",
            s
        )));
    }

    let year: i64 = date_parts[0].parse().unwrap_or(0);
    let month: i64 = date_parts[1].parse().unwrap_or(0);
    let day: i64 = date_parts[2].parse().unwrap_or(0);
    let hour: i64 = time_parts[0].parse().unwrap_or(0);
    let minute: i64 = time_parts[1].parse().unwrap_or(0);
    let second: i64 = time_parts[2].parse().unwrap_or(0);

    let timestamp = calculate_timestamp(year, month, day, hour, minute, second);

    Ok(RuntimeValue::Int(timestamp))
}

// ============================================================================
// DateTime Accessor Functions
// ============================================================================

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
