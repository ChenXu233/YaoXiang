//! Standard Math library

/// Absolute value
pub fn abs(n: i128) -> i128 {
    n.abs()
}

/// Maximum of two values
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

/// Minimum of two values
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

/// Clamp value to range
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Power (base^exp)
pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Square root
pub fn sqrt(n: f64) -> f64 {
    n.sqrt()
}

/// Floor
pub fn floor(n: f64) -> f64 {
    n.floor()
}

/// Ceiling
pub fn ceil(n: f64) -> f64 {
    n.ceil()
}

/// Round
pub fn round(n: f64) -> f64 {
    n.round()
}

/// Truncate
pub fn trunc(n: f64) -> f64 {
    n.trunc()
}

/// Fractional part
pub fn fract(n: f64) -> f64 {
    n.fract()
}

/// Absolute value (float)
pub fn fabs(n: f64) -> f64 {
    n.abs()
}

/// Sine
pub fn sin(n: f64) -> f64 {
    n.sin()
}

/// Cosine
pub fn cos(n: f64) -> f64 {
    n.cos()
}

/// Tangent
pub fn tan(n: f64) -> f64 {
    n.tan()
}

/// Pi constant
pub const PI: f64 = std::f64::consts::PI;

/// E constant
pub const E: f64 = std::f64::consts::E;

/// Tau constant (2*pi)
pub const TAU: f64 = std::f64::consts::TAU;
