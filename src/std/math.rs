//! Standard Math library (YaoXiang)
//!
//! This module provides mathematical functions for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// MathModule - StdModule Implementation
// ============================================================================

/// Math module implementation.
pub struct MathModule;

impl Default for MathModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for MathModule {
    fn module_path(&self) -> &str {
        "std.math"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // Integer functions
            export!("abs", "std.math.abs", "(n: Int) -> Int", native_abs),
            export!("max", "std.math.max", "(a: Int, b: Int) -> Int", native_max),
            export!("min", "std.math.min", "(a: Int, b: Int) -> Int", native_min),
            export!(
                "clamp",
                "std.math.clamp",
                "(value: Int, min: Int, max: Int) -> Int",
                native_clamp
            ),
            // Float functions
            export!("fabs", "std.math.fabs", "(n: Float) -> Float", native_fabs),
            export!(
                "fmax",
                "std.math.fmax",
                "(a: Float, b: Float) -> Float",
                native_fmax
            ),
            export!(
                "fmin",
                "std.math.fmin",
                "(a: Float, b: Float) -> Float",
                native_fmin
            ),
            export!(
                "pow",
                "std.math.pow",
                "(base: Float, exp: Float) -> Float",
                native_pow
            ),
            export!("sqrt", "std.math.sqrt", "(n: Float) -> Float", native_sqrt),
            export!(
                "floor",
                "std.math.floor",
                "(n: Float) -> Float",
                native_floor
            ),
            export!("ceil", "std.math.ceil", "(n: Float) -> Float", native_ceil),
            export!(
                "round",
                "std.math.round",
                "(n: Float) -> Float",
                native_round
            ),
            // Trigonometric functions
            export!("sin", "std.math.sin", "(n: Float) -> Float", native_sin),
            export!("cos", "std.math.cos", "(n: Float) -> Float", native_cos),
            export!("tan", "std.math.tan", "(n: Float) -> Float", native_tan),
            // Constants (still need handlers to return values)
            export!("PI", "std.math.PI", "Float", native_pi),
            export!("E", "std.math.E", "Float", native_e),
            export!("TAU", "std.math.TAU", "Float", native_tau),
        ]
    }
}

/// Singleton instance for std.math module.
pub const MATH_MODULE: MathModule = MathModule;

// ============================================================================
// Native function implementations
// ============================================================================

/// Native implementation: abs (integer)
fn native_abs(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_int()).unwrap_or(0);
    Ok(RuntimeValue::Int(n.abs()))
}

/// Native implementation: max (integer)
fn native_max(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
    let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
    Ok(RuntimeValue::Int(a.max(b)))
}

/// Native implementation: min (integer)
fn native_min(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
    let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
    Ok(RuntimeValue::Int(a.min(b)))
}

/// Native implementation: clamp (integer)
fn native_clamp(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let value = args.first().and_then(|v| v.to_int()).unwrap_or(0);
    let min = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
    let max = args.get(2).and_then(|v| v.to_int()).unwrap_or(0);
    Ok(RuntimeValue::Int(value.clamp(min, max)))
}

/// Native implementation: fabs (float absolute value)
fn native_fabs(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.abs()))
}

/// Native implementation: fmax (float maximum)
fn native_fmax(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let a = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    let b = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(a.max(b)))
}

/// Native implementation: fmin (float minimum)
fn native_fmin(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let a = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    let b = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(a.min(b)))
}

/// Native implementation: pow (power)
fn native_pow(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let base = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    let exp = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(base.powf(exp)))
}

/// Native implementation: sqrt (square root)
fn native_sqrt(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.sqrt()))
}

/// Native implementation: floor
fn native_floor(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.floor()))
}

/// Native implementation: ceil
fn native_ceil(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.ceil()))
}

/// Native implementation: round
fn native_round(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.round()))
}

/// Native implementation: sin
fn native_sin(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.sin()))
}

/// Native implementation: cos
fn native_cos(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.cos()))
}

/// Native implementation: tan
fn native_tan(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
    Ok(RuntimeValue::Float(n.tan()))
}

/// Native implementation: PI constant
fn native_pi(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(RuntimeValue::Float(std::f64::consts::PI))
}

/// Native implementation: E constant
fn native_e(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(RuntimeValue::Float(std::f64::consts::E))
}

/// Native implementation: TAU constant
fn native_tau(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Ok(RuntimeValue::Float(std::f64::consts::TAU))
}
