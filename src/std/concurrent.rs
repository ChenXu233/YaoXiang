//! Standard Concurrent library (YaoXiang)
//!
//! This module provides concurrency-related functionality for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// ConcurrentModule - StdModule Implementation
// ============================================================================

/// Concurrent module implementation.
pub struct ConcurrentModule;

impl Default for ConcurrentModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for ConcurrentModule {
    fn module_path(&self) -> &str {
        "std.concurrent"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "sleep",
                "std.concurrent.sleep",
                "(millis: Int) -> Void",
                native_sleep,
            ),
            NativeExport::new(
                "thread_id",
                "std.concurrent.thread_id",
                "() -> String",
                native_thread_id,
            ),
            NativeExport::new(
                "yield_now",
                "std.concurrent.yield_now",
                "() -> Void",
                native_yield_now,
            ),
        ]
    }
}

/// Singleton instance for std.concurrent module.
pub const CONCURRENT_MODULE: ConcurrentModule = ConcurrentModule;

// ============================================================================
// Native Function Implementations
// ============================================================================

/// Native implementation: sleep
fn native_sleep(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let millis = args.first().and_then(|v| v.to_int()).unwrap_or(0) as u64;
    std::thread::sleep(std::time::Duration::from_millis(millis));
    Ok(RuntimeValue::Unit)
}

/// Native implementation: thread_id
fn native_thread_id(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(RuntimeValue::String(
        format!("{:?}", std::thread::current().id()).into(),
    ))
}

/// Native implementation: yield_now
fn native_yield_now(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    std::thread::yield_now();
    Ok(RuntimeValue::Unit)
}
