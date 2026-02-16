//! Standard FFI library (YaoXiang)
//!
//! This module provides the Foreign Function Interface (FFI) for YaoXiang,
//! allowing users to declare and use native (Rust) functions from YaoXiang code.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────┐
//! │  YaoXiang Source                                          │
//! │                                                           │
//! │  my_add: (a: Int, b: Int) -> Int = Native("my_add")     │
//! │                │                                          │
//! │  ┌─────────────┘                                          │
//! │  │                                                        │
//! │  ▼  Compile Time                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ IR Gen: detect Native("my_add")      │                 │
//! │  │ → record native_binding:             │                 │
//! │  │   { func: "my_add",                  │                 │
//! │  │     native: "my_add" }               │                 │
//! │  │ → skip function body generation      │                 │
//! │  └──────────────┬───────────────────────┘                 │
//! │                 │                                          │
//! │                 ▼                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ Codegen: register "my_add" as native │                 │
//! │  │ → any call to my_add(1, 2) emits     │                 │
//! │  │   CallNative { "my_add" }             │                │
//! │  └──────────────┬───────────────────────┘                 │
//! │                 │                                          │
//! │                 ▼  Runtime                                 │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ FfiRegistry.call("my_add", args)     │                 │
//! │  │ → execute registered Rust handler    │                 │
//! │  └──────────────────────────────────────┘                 │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage from YaoXiang
//!
//! ```yaoxiang
//! # Declare a native function binding
//! my_add: (a: Int, b: Int) -> Int = Native("my_add")
//!
//! # Call it (dispatches to Rust handler via FFI)
//! result = my_add(1, 2)
//! println(result)   # → 3
//! ```
//!
//! # Usage from Rust (embedding API)
//!
//! ```rust,ignore
//! use yaoxiang::backends::interpreter::ffi::FfiRegistry;
//! use yaoxiang::backends::common::RuntimeValue;
//!
//! // Create an interpreter and register custom native functions
//! let mut interpreter = Interpreter::new();
//! interpreter.ffi_registry_mut().register("my_add", |args| {
//!     let a = args[0].to_int().unwrap_or(0);
//!     let b = args[1].to_int().unwrap_or(0);
//!     Ok(RuntimeValue::Int(a + b))
//! });
//! ```

use std::collections::HashMap;

// ============================================================================
// Native Binding Infrastructure
// ============================================================================

/// Represents a user-defined native function binding.
///
/// A `NativeBinding` maps a YaoXiang function name to an FFI native symbol.
/// When the compiler encounters `name: type = Native("symbol")`, it creates
/// a `NativeBinding` and skips generating a function body. Instead, calls
/// to this function emit `CallNative` bytecode instructions.
///
/// # Example
///
/// ```rust
/// # use yaoxiang::std::ffi::NativeBinding;
/// let binding = NativeBinding::new("my_add", "my_add");
/// assert_eq!(binding.func_name(), "my_add");
/// assert_eq!(binding.native_symbol(), "my_add");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBinding {
    /// The YaoXiang function name (as declared in source)
    func_name: String,
    /// The native symbol name (passed to FfiRegistry)
    native_symbol: String,
}

impl NativeBinding {
    /// Create a new native binding.
    ///
    /// # Arguments
    ///
    /// * `func_name` - The YaoXiang function name
    /// * `native_symbol` - The FFI symbol name (from `Native("...")`)
    pub fn new(
        func_name: &str,
        native_symbol: &str,
    ) -> Self {
        Self {
            func_name: func_name.to_string(),
            native_symbol: native_symbol.to_string(),
        }
    }

    /// Get the YaoXiang function name.
    pub fn func_name(&self) -> &str {
        &self.func_name
    }

    /// Get the native FFI symbol name.
    pub fn native_symbol(&self) -> &str {
        &self.native_symbol
    }
}

/// Detects if an AST expression is a `Native("symbol")` pattern.
///
/// Returns `Some(symbol_name)` if the expression matches the pattern
/// `Native("...")`, otherwise returns `None`.
///
/// This is used by the IR generator to detect native function declarations
/// and record them as native bindings instead of generating function bodies.
pub fn detect_native_binding(
    func_expr: &crate::frontend::core::parser::ast::Expr,
    args: &[crate::frontend::core::parser::ast::Expr],
) -> Option<String> {
    use crate::frontend::core::parser::ast::Expr;

    // Check if the function is Var("Native")
    if let Expr::Var(name, _) = func_expr {
        if name == "Native" && args.len() == 1 {
            // Extract the string literal argument
            if let Expr::Lit(crate::frontend::core::lexer::tokens::Literal::String(symbol), _) =
                &args[0]
            {
                return Some(symbol.clone());
            }
        }
    }
    None
}

/// Collects all native binding function names for use as a lookup set.
///
/// Given a list of `NativeBinding`s, returns a `HashMap` mapping
/// function names to their native symbols.
pub fn bindings_to_native_map(bindings: &[NativeBinding]) -> HashMap<String, String> {
    bindings
        .iter()
        .map(|b| (b.func_name.clone(), b.native_symbol.clone()))
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_binding_creation() {
        let binding = NativeBinding::new("my_add", "my_add");
        assert_eq!(binding.func_name(), "my_add");
        assert_eq!(binding.native_symbol(), "my_add");
    }

    #[test]
    fn test_native_binding_different_names() {
        let binding = NativeBinding::new("add", "math.add");
        assert_eq!(binding.func_name(), "add");
        assert_eq!(binding.native_symbol(), "math.add");
    }

    #[test]
    fn test_bindings_to_native_map() {
        let bindings = vec![
            NativeBinding::new("my_add", "my_add"),
            NativeBinding::new("my_sub", "math.sub"),
        ];
        let map = bindings_to_native_map(&bindings);
        assert_eq!(map.get("my_add"), Some(&"my_add".to_string()));
        assert_eq!(map.get("my_sub"), Some(&"math.sub".to_string()));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_detect_native_binding() {
        use crate::frontend::core::lexer::tokens::Literal;
        use crate::frontend::core::parser::ast::Expr;
        use crate::util::span::Span;

        let func = Expr::Var("Native".to_string(), Span::dummy());
        let args = vec![Expr::Lit(
            Literal::String("my_func".to_string()),
            Span::dummy(),
        )];

        assert_eq!(
            detect_native_binding(&func, &args),
            Some("my_func".to_string())
        );
    }

    #[test]
    fn test_detect_native_binding_not_native() {
        use crate::frontend::core::parser::ast::Expr;
        use crate::util::span::Span;

        let func = Expr::Var("regular_fn".to_string(), Span::dummy());
        let args: Vec<Expr> = vec![];

        assert_eq!(detect_native_binding(&func, &args), None);
    }

    #[test]
    fn test_detect_native_binding_wrong_args() {
        use crate::frontend::core::lexer::tokens::Literal;
        use crate::frontend::core::parser::ast::Expr;
        use crate::util::span::Span;

        // Native with non-string argument
        let func = Expr::Var("Native".to_string(), Span::dummy());
        let args = vec![Expr::Lit(Literal::Int(42), Span::dummy())];

        assert_eq!(detect_native_binding(&func, &args), None);
    }
}
