//! Standard Math library (YaoXiang)
//!
//! This module provides mathematical functions for YaoXiang programs.
//! All math functions are declared as `Native("std.math.xxx")` bindings.

use std::collections::HashMap;

/// Represents a native function declaration for math module.
#[derive(Debug, Clone)]
pub struct NativeDeclaration {
    pub name: &'static str,
    pub native_name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
    pub implemented: bool,
}

pub fn native_declarations() -> Vec<NativeDeclaration> {
    vec![
        NativeDeclaration {
            name: "abs",
            native_name: "std.math.abs",
            signature: "(n: Int) -> Int",
            doc: "Returns the absolute value of an integer.",
            implemented: true,
        },
        NativeDeclaration {
            name: "max",
            native_name: "std.math.max",
            signature: "(a: Int, b: Int) -> Int",
            doc: "Returns the maximum of two integers.",
            implemented: true,
        },
        NativeDeclaration {
            name: "min",
            native_name: "std.math.min",
            signature: "(a: Int, b: Int) -> Int",
            doc: "Returns the minimum of two integers.",
            implemented: true,
        },
        NativeDeclaration {
            name: "clamp",
            native_name: "std.math.clamp",
            signature: "(value: Int, min: Int, max: Int) -> Int",
            doc: "Clamps a value between min and max.",
            implemented: true,
        },
        NativeDeclaration {
            name: "fabs",
            native_name: "std.math.fabs",
            signature: "(n: Float) -> Float",
            doc: "Returns the absolute value of a float.",
            implemented: true,
        },
        NativeDeclaration {
            name: "fmax",
            native_name: "std.math.fmax",
            signature: "(a: Float, b: Float) -> Float",
            doc: "Returns the maximum of two floats.",
            implemented: true,
        },
        NativeDeclaration {
            name: "fmin",
            native_name: "std.math.fmin",
            signature: "(a: Float, b: Float) -> Float",
            doc: "Returns the minimum of two floats.",
            implemented: true,
        },
        NativeDeclaration {
            name: "fclamp",
            native_name: "std.math.fclamp",
            signature: "(value: Float, min: Float, max: Float) -> Float",
            doc: "Clamps a float value.",
            implemented: true,
        },
        NativeDeclaration {
            name: "pow",
            native_name: "std.math.pow",
            signature: "(base: Float, exp: Float) -> Float",
            doc: "Returns base raised to the power of exp.",
            implemented: true,
        },
        NativeDeclaration {
            name: "sqrt",
            native_name: "std.math.sqrt",
            signature: "(n: Float) -> Float",
            doc: "Returns the square root of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "cbrt",
            native_name: "std.math.cbrt",
            signature: "(n: Float) -> Float",
            doc: "Returns the cube root of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "floor",
            native_name: "std.math.floor",
            signature: "(n: Float) -> Float",
            doc: "Returns the largest integer less than or equal to n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "ceil",
            native_name: "std.math.ceil",
            signature: "(n: Float) -> Float",
            doc: "Returns the smallest integer greater than or equal to n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "round",
            native_name: "std.math.round",
            signature: "(n: Float) -> Float",
            doc: "Returns the nearest integer to n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "trunc",
            native_name: "std.math.trunc",
            signature: "(n: Float) -> Float",
            doc: "Returns the integer part of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "fract",
            native_name: "std.math.fract",
            signature: "(n: Float) -> Float",
            doc: "Returns the fractional part of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "sin",
            native_name: "std.math.sin",
            signature: "(n: Float) -> Float",
            doc: "Returns the sine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "cos",
            native_name: "std.math.cos",
            signature: "(n: Float) -> Float",
            doc: "Returns the cosine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "tan",
            native_name: "std.math.tan",
            signature: "(n: Float) -> Float",
            doc: "Returns the tangent of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "asin",
            native_name: "std.math.asin",
            signature: "(n: Float) -> Float",
            doc: "Returns the arcsine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "acos",
            native_name: "std.math.acos",
            signature: "(n: Float) -> Float",
            doc: "Returns the arccosine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "atan",
            native_name: "std.math.atan",
            signature: "(n: Float) -> Float",
            doc: "Returns the arctangent of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "sinh",
            native_name: "std.math.sinh",
            signature: "(n: Float) -> Float",
            doc: "Returns the hyperbolic sine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "cosh",
            native_name: "std.math.cosh",
            signature: "(n: Float) -> Float",
            doc: "Returns the hyperbolic cosine of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "tanh",
            native_name: "std.math.tanh",
            signature: "(n: Float) -> Float",
            doc: "Returns the hyperbolic tangent of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "exp",
            native_name: "std.math.exp",
            signature: "(n: Float) -> Float",
            doc: "Returns e^n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "ln",
            native_name: "std.math.ln",
            signature: "(n: Float) -> Float",
            doc: "Returns the natural logarithm of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "log2",
            native_name: "std.math.log2",
            signature: "(n: Float) -> Float",
            doc: "Returns the base-2 logarithm of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "log10",
            native_name: "std.math.log10",
            signature: "(n: Float) -> Float",
            doc: "Returns the base-10 logarithm of n.",
            implemented: true,
        },
        NativeDeclaration {
            name: "PI",
            native_name: "std.math.PI",
            signature: "Float",
            doc: "The mathematical constant Pi.",
            implemented: true,
        },
        NativeDeclaration {
            name: "E",
            native_name: "std.math.E",
            signature: "Float",
            doc: "The mathematical constant e.",
            implemented: true,
        },
        NativeDeclaration {
            name: "TAU",
            native_name: "std.math.TAU",
            signature: "Float",
            doc: "The mathematical constant Tau.",
            implemented: true,
        },
    ]
}

pub fn native_name_map() -> HashMap<String, String> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name.to_string(), d.native_name.to_string()))
        .collect()
}

pub fn implemented_native_names() -> Vec<(&'static str, &'static str)> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name, d.native_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_declarations_not_empty() {
        let decls = native_declarations();
        assert!(!decls.is_empty());
    }
}
