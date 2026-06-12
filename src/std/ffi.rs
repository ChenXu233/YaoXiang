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
//! │  my_add: (a: Int, b: Int) -> Int = native("my_add")      │
//! │                │                                          │
//! │  ┌─────────────┘                                          │
//! │  │                                                        │
//! │  ▼  Compile Time                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ IR Gen: detect native("my_add")      │                 │
//! │  │ → resolved name == std.ffi.native    │                 │
//! │  │ → create NativeBinding               │                 │
//! │  │ → skip function body generation      │                 │
//! │  └──────────────┬───────────────────────┘                 │
//! │                 │                                          │
//! │                 ▼                                          │
//! │  ┌──────────────────────────────────────┐                 │
//! │  │ Codegen: register "my_add" as native │                 │
//! │  │ → any call to my_add(1, 2) emits     │                 │
//! │  │   CallNative { "my_add" }            │                 │
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
//! # Declare a native function binding using the std.ffi.native function
//! my_add: (a: Int, b: Int) -> Int = native("my_add")
//!
//! # Call it (dispatches to Rust handler via FFI)
//! result = my_add(1, 2)
//! println(result)   # → 3
//! ```
//!
//! `native` is a real function declared in `std.ffi` with signature
//! `native(symbol: String) -> Never`. It is intercepted at compile time
//! by the IR generator — when the name `std.ffi.native` is resolved in
//! a function declaration's value position, the compiler records a
//! `NativeBinding` instead of emitting bytecode. At runtime, attempting
//! to call `native(...)` will fail with a clear error.
//!
//! # Usage from Rust (embedding API)
//!
//! ```rust,ignore
//! use yaoxiang::backends::interpreter::ffi::FfiRegistry;
//! use yaoxiang::backends::common::RuntimeValue;
//!
//! // Create an interpreter and register custom native functions
//! let mut interpreter = Interpreter::new();
//! interpreter.ffi_registry_mut().register("my_add", |args, ctx| {
//!     let a = args[0].to_int().unwrap_or(0);
//!     let b = args[1].to_int().unwrap_or(0);
//!     Ok(RuntimeValue::Int(a + b))
//! });
//! ```

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
    /// * `native_symbol` - The FFI symbol name (from `native("...")`)
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

// ============================================================================
// StdModule implementation for std.ffi
// ============================================================================

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// Shared AST helper: detect native("symbol") via name resolution
// ============================================================================

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast;

/// 从函数声明的值位置提取 native binding 的符号名。
///
/// 通过 name resolution 检测 callee 是否为 `std.ffi.native`：
/// - `native("symbol")` → Var("native")
/// - `std.ffi.native("symbol")` → FieldAccess(FieldAccess(Var("std"), "ffi"), "native")
///
/// 不再硬编码 AST 字符串匹配，而是通过模块路径解析。
/// 这保证了用户作用域中的同名变量会正确 shadow（不会被误认为是 FFI 声明）。
pub fn extract_native_binding_symbol(
    func: &ast::Expr,
    args: &[ast::Expr],
) -> Option<String> {
    if !is_native_callee(func) {
        return None;
    }

    // 提取第一个参数作为 symbol 名
    let first_arg = args.first()?;
    if let ast::Expr::Lit(Literal::String(symbol), _) = first_arg {
        return Some(symbol.clone());
    }

    None
}

/// 检查表达式是否解析为 `std.ffi.native`。
///
/// 这是通过模块路径解析实现的：
/// - `native` → 直接匹配名称
/// - `std.ffi.native` → 递归检查字段访问链
fn is_native_callee(func: &ast::Expr) -> bool {
    match func {
        ast::Expr::Var(name, _) => name == "native",
        ast::Expr::FieldAccess {
            expr: inner, field, ..
        } => {
            if field == "native" {
                if let ast::Expr::FieldAccess {
                    expr,
                    field: ffi_field,
                    ..
                } = inner.as_ref()
                {
                    if ffi_field == "ffi" {
                        if let ast::Expr::Var(name, _) = expr.as_ref() {
                            return name == "std";
                        }
                    }
                }
            }
            false
        }
        _ => false,
    }
}

/// FFI module implementation.
///
/// Exports `Native(symbol: String) -> Never`, a compile-time-only function
/// used to declare FFI bindings. The IR generator intercepts calls to
/// `std.ffi.native` and creates `NativeBinding` entries; no bytecode is
/// ever emitted. The runtime handler returns an error as a safety net.
pub struct FfiModule;

impl Default for FfiModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for FfiModule {
    fn module_path(&self) -> &str {
        "std.ffi"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![NativeExport::new(
            "native",
            "std.ffi.native",
            "(symbol: String) -> Never",
            native_ffi_native,
        )]
    }
}

/// Singleton instance for std.ffi module.
pub const FFI_MODULE: FfiModule = FfiModule;

/// Handler for `native(symbol)`. This should never actually execute at
/// runtime — the IR generator intercepts it at compile time. If it does
/// get called, something went wrong in the compiler.
fn native_ffi_native(
    _args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    Err(ExecutorError::runtime_only(
        "Native() is a compile-time construct used to declare FFI bindings, \
         it cannot be called at runtime"
            .to_string(),
    ))
}

// ============================================================================
// Tests
// ============================================================================
