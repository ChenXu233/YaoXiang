//! Standard IO library (YaoXiang)
//!
//! This module provides input/output functionality for YaoXiang programs.
//! All IO functions are declared as `Native("std.io.xxx")` bindings, meaning
//! their actual implementations live in the FFI registry
//! ([`crate::backends::interpreter::ffi`]).
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────┐
//! │  YaoXiang Source Code        │
//! │  use std.io                  │
//! │  println("Hello!")           │
//! │            │                 │
//! │            ▼                 │
//! │  NativeDeclaration registry  │ ← this module
//! │  name="println"              │
//! │  native="std.io.println"     │
//! │            │                 │
//! │            ▼                 │
//! │  CodeGen: CallNative opcode  │
//! │            │                 │
//! │            ▼                 │
//! │  FfiRegistry.call()          │ ← ffi.rs
//! └──────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```yaoxiang
//! use std.io
//!
//! // Print values
//! println("Hello, World!")
//! println(42)
//! println(3.14)
//! println(true)
//!
//! // Read input
//! name = read_line()
//! println("Hello, " + name)
//!
//! // File operations
//! content = read_file("data.txt")
//! write_file("output.txt", content)
//! append_file("log.txt", "new entry\n")
//! ```

use std::collections::HashMap;

// ============================================================================
// Native Declaration Infrastructure
// ============================================================================

/// Represents a native function declaration.
///
/// Each `NativeDeclaration` maps a YaoXiang function name (e.g., `"println"`)
/// to its FFI native binding name (e.g., `"std.io.println"`), along with
/// documentation about the function's signature and behavior.
///
/// # Example
///
/// ```rust
/// # use yaoxiang::std::io::NativeDeclaration;
/// let decl = NativeDeclaration {
///     name: "println",
///     native_name: "std.io.println",
///     signature: "(value: Any) -> Void",
///     doc: "Print a value followed by a newline",
///     implemented: true,
/// };
/// assert_eq!(decl.name, "println");
/// assert_eq!(decl.native_name, "std.io.println");
/// ```
#[derive(Debug, Clone)]
pub struct NativeDeclaration {
    /// The YaoXiang-facing function name (e.g., `"println"`)
    pub name: &'static str,
    /// The FFI native binding name (e.g., `"std.io.println"`)
    pub native_name: &'static str,
    /// Human-readable type signature (e.g., `"(value: Any) -> Void"`)
    pub signature: &'static str,
    /// Documentation string
    pub doc: &'static str,
    /// Whether this function has a native implementation in FfiRegistry
    pub implemented: bool,
}

/// Returns all native IO function declarations.
///
/// This is the canonical registry of `std.io` functions. The code generator
/// and type checker use this to discover which functions should be compiled
/// as `CallNative` instructions.
///
/// # Implemented Functions
///
/// | Function | Native Name | Signature |
/// |----------|-------------|-----------|
/// | `print` | `std.io.print` | `(value: Any) -> Void` |
/// | `println` | `std.io.println` | `(value: Any) -> Void` |
/// | `read_line` | `std.io.read_line` | `() -> String` |
/// | `read_file` | `std.io.read_file` | `(path: String) -> String` |
/// | `write_file` | `std.io.write_file` | `(path: String, content: String) -> Bool` |
/// | `append_file` | `std.io.append_file` | `(path: String, content: String) -> Bool` |
pub fn native_declarations() -> Vec<NativeDeclaration> {
    vec![
        // ==================================================================
        // Basic Printing Functions
        // ==================================================================
        NativeDeclaration {
            name: "print",
            native_name: "std.io.print",
            signature: "(value: Any) -> Void",
            doc: "Print a value without newline. Supports all types that implement Display.",
            implemented: true,
        },
        NativeDeclaration {
            name: "println",
            native_name: "std.io.println",
            signature: "(value: Any) -> Void",
            doc: "Print a value followed by a newline. Supports all types that implement Display.",
            implemented: true,
        },
        // ==================================================================
        // Input Functions
        // ==================================================================
        NativeDeclaration {
            name: "read_line",
            native_name: "std.io.read_line",
            signature: "() -> String",
            doc: "Read a line from standard input. Returns the input as a String.",
            implemented: true,
        },
        // ==================================================================
        // File Operations
        // ==================================================================
        NativeDeclaration {
            name: "read_file",
            native_name: "std.io.read_file",
            signature: "(path: String) -> String",
            doc: "Read entire file as String. Returns empty string if file doesn't exist.",
            implemented: true,
        },
        NativeDeclaration {
            name: "write_file",
            native_name: "std.io.write_file",
            signature: "(path: String, content: String) -> Bool",
            doc: "Write content to file. Returns true if successful, false otherwise.",
            implemented: true,
        },
        NativeDeclaration {
            name: "append_file",
            native_name: "std.io.append_file",
            signature: "(path: String, content: String) -> Bool",
            doc: "Append content to file. Returns true if successful, false otherwise.",
            implemented: true,
        },
        // ==================================================================
        // Future Functions (not yet implemented in FfiRegistry)
        // ==================================================================
        NativeDeclaration {
            name: "print_sep",
            native_name: "std.io.print_sep",
            signature: "(values: List[Any], separator: String) -> Void",
            doc: "Print multiple values with custom separator.",
            implemented: false,
        },
        NativeDeclaration {
            name: "printf",
            native_name: "std.io.printf",
            signature: "(format: String, args: List[Any]) -> Void",
            doc: "Formatted print using printf-style format string. Supports %d, %s, %f, %x, %b.",
            implemented: false,
        },
        NativeDeclaration {
            name: "sprintf",
            native_name: "std.io.sprintf",
            signature: "(format: String, args: List[Any]) -> String",
            doc: "Format string using sprintf-style formatting. Returns formatted string.",
            implemented: false,
        },
        NativeDeclaration {
            name: "read_lines",
            native_name: "std.io.read_lines",
            signature: "(count: Int) -> List[String]",
            doc: "Read multiple lines from stdin. Returns a List of lines.",
            implemented: false,
        },
        NativeDeclaration {
            name: "read_file_lines",
            native_name: "std.io.read_file_lines",
            signature: "(path: String) -> List[String]",
            doc: "Read file as list of lines. Returns empty list if file doesn't exist.",
            implemented: false,
        },
    ]
}

/// Returns a mapping from YaoXiang function names to FFI native names
/// for all **implemented** std.io functions.
///
/// This is used by the code generator to determine which function calls
/// should emit `CallNative` instead of `CallStatic`.
///
/// # Returns
///
/// A `HashMap` where keys are YaoXiang-facing names (e.g., `"println"`)
/// and values are FFI native names (e.g., `"std.io.println"`).
pub fn native_name_map() -> HashMap<String, String> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name.to_string(), d.native_name.to_string()))
        .collect()
}

/// Returns a list of all implemented native function names (both short and
/// fully-qualified forms) for use in code generation.
///
/// Returns pairs of `(short_name, native_name)`.
pub fn implemented_native_names() -> Vec<(&'static str, &'static str)> {
    native_declarations()
        .into_iter()
        .filter(|d| d.implemented)
        .map(|d| (d.name, d.native_name))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_declarations_not_empty() {
        let decls = native_declarations();
        assert!(!decls.is_empty());
    }

    #[test]
    fn test_all_implemented_have_native_names() {
        for decl in native_declarations() {
            if decl.implemented {
                assert!(
                    decl.native_name.starts_with("std.io."),
                    "Implemented function '{}' should have 'std.io.' prefix, got '{}'",
                    decl.name,
                    decl.native_name
                );
            }
        }
    }

    #[test]
    fn test_native_name_map_contains_core_functions() {
        let map = native_name_map();
        assert_eq!(map.get("print"), Some(&"std.io.print".to_string()));
        assert_eq!(map.get("println"), Some(&"std.io.println".to_string()));
        assert_eq!(map.get("read_line"), Some(&"std.io.read_line".to_string()));
        assert_eq!(map.get("read_file"), Some(&"std.io.read_file".to_string()));
        assert_eq!(
            map.get("write_file"),
            Some(&"std.io.write_file".to_string())
        );
        assert_eq!(
            map.get("append_file"),
            Some(&"std.io.append_file".to_string())
        );
    }

    #[test]
    fn test_implemented_native_names() {
        let names = implemented_native_names();
        assert!(
            names.len() >= 6,
            "Should have at least 6 implemented functions"
        );

        let short_names: Vec<&str> = names.iter().map(|(s, _)| *s).collect();
        assert!(short_names.contains(&"print"));
        assert!(short_names.contains(&"println"));
        assert!(short_names.contains(&"read_line"));
        assert!(short_names.contains(&"read_file"));
        assert!(short_names.contains(&"write_file"));
        assert!(short_names.contains(&"append_file"));
    }

    #[test]
    fn test_name_map_excludes_unimplemented() {
        let map = native_name_map();
        // printf is listed but not yet implemented
        assert!(
            !map.contains_key("printf"),
            "Unimplemented functions should not appear in native_name_map"
        );
    }

    #[test]
    fn test_native_declaration_fields() {
        let decls = native_declarations();
        let println_decl = decls.iter().find(|d| d.name == "println").unwrap();
        assert_eq!(println_decl.native_name, "std.io.println");
        assert!(!println_decl.signature.is_empty());
        assert!(!println_decl.doc.is_empty());
        assert!(println_decl.implemented);
    }
}
