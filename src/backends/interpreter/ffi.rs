//! FFI (Foreign Function Interface) Registry for YaoXiang
//!
//! This module provides the `FfiRegistry` which manages native function bindings,
//! allowing YaoXiang code to call Rust functions directly ("rs" mechanism) and
//! C functions via dynamically loaded libraries ("c" mechanism).
//!
//! # Architecture
//!
//! ```text
//! CallNative { mechanism="c", lib="libc.so.6", symbol="getpid" }
//!       │
//!       ├── "rs" → FfiRegistry.call() → direct handler lookup
//!       └── "c"  → FfiRegistry.call_c() → libloading → transmute → call
//! ```
//!
//! # Safety
//!
//! C ABI calls involve transmuting function pointer addresses obtained from
//! `dlsym`/`GetProcAddress`. This is inherently unsafe but encapsulated within
//! the registry. The `OpaqueHandle` type stores a `NonNull<c_void>` pointer
//! without ever dereferencing it.
//!
//! Phase 1 only supports `() -> i32` C functions. String/struct marshalling
//! is not yet implemented.

#[cfg(not(target_arch = "wasm32"))]
use libloading::Library;
use std::collections::HashMap;
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeHandler};

/// FFI Registry that manages native function bindings.
/// The registry holds a mapping from function names (e.g., `"std.io.println"`)
/// to their native Rust implementations.
///
/// # Example
///
/// ```ignore
/// let mut registry = FfiRegistry::new();
/// registry.register("my_func", |args| {
///     Ok(RuntimeValue::Unit)
/// });
/// let result = registry.call("my_func", &[]);
/// ```
#[derive(Clone)]
#[allow(dead_code)]
pub struct FfiRegistry {
    /// Function handler table: name -> handler
    handlers: HashMap<String, NativeHandler>,
    /// Cached loaded libraries (lib_name -> Library)
    #[cfg(not(target_arch = "wasm32"))]
    loaded_libs: HashMap<String, Arc<Library>>,
    /// Registered opaque type names
    opaque_types: HashSet<String>,
}

impl std::fmt::Debug for FfiRegistry {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("FfiRegistry")
            .field("handlers_count", &self.handlers.len())
            .field(
                "registered_functions",
                &self.handlers.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Default for FfiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FfiRegistry {
    /// Create a new empty FFI registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            loaded_libs: HashMap::new(),
            opaque_types: HashSet::new(),
        }
    }

    /// Create a new FFI registry pre-populated with standard library functions.
    ///
    /// This registers all `std.*` native functions that are available by default.
    pub fn with_std() -> Self {
        let mut registry = Self::new();
        crate::std::register_all(&mut registry);
        registry
    }

    /// Register a new native function handler.
    ///
    /// # Arguments
    ///
    /// * `name` - The fully qualified function name (e.g., `"std.io.println"`)
    /// * `handler` - The native function handler
    ///
    /// If a function with the same name already exists, it will be overwritten.
    pub fn register(
        &mut self,
        name: &str,
        handler: NativeHandler,
    ) {
        self.handlers.insert(name.to_string(), handler);
    }

    /// Call a registered native function by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The fully qualified function name
    /// * `args` - The arguments to pass to the function
    /// * `ctx` - The native context providing heap access and function call capability
    ///
    /// # Errors
    ///
    /// Returns `ExecutorError::FunctionNotFound` if no handler is registered
    /// for the given name.
    pub fn call(
        &self,
        name: &str,
        args: &[RuntimeValue],
        ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        match self.handlers.get(name) {
            Some(handler) => handler(args, ctx),
            None => Err(ExecutorError::FunctionNotFound(
                format!("Native function not found: {}", name),
                None,
            )),
        }
    }

    /// Check if a function is registered.
    pub fn has(
        &self,
        name: &str,
    ) -> bool {
        self.handlers.contains_key(name)
    }

    /// Get the number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Get a list of all registered function names.
    pub fn registered_functions(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Call a native function by mechanism and name.
    ///
    /// Dispatches to either:
    /// - `"rs"` — calls a registered Rust handler via `call()`
    /// - `"c"` — calls a C function from a dynamically loaded library via `call_c()`
    pub fn call_with_mechanism(
        &self,
        mechanism: &str,
        #[cfg_attr(target_arch = "wasm32", allow(unused_variables))] lib: &str,
        #[cfg_attr(target_arch = "wasm32", allow(unused_variables))] symbol: &str,
        func_name: &str,
        args: &[RuntimeValue],
        ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        match mechanism {
            "rs" => self.call(func_name, args, ctx),
            #[cfg(not(target_arch = "wasm32"))]
            "c" => self.call_c(lib, symbol, args, ctx),
            #[cfg(target_arch = "wasm32")]
            "c" => Err(ExecutorError::runtime_only(
                "C ABI (dynamic library loading) is not supported on wasm32".to_string(),
            )),
            _ => Err(ExecutorError::runtime_only(format!(
                "unknown FFI mechanism: {mechanism}"
            ))),
        }
    }

    /// Call a C function from a dynamically loaded library.
    ///
    /// Phase 1 only supports `() -> i32` (void → int32) C functions.
    /// The library must be pre-loaded via [`load_library`].
    ///
    /// # Safety
    ///
    /// Transmutes function pointers from `dlsym`/`GetProcAddress` addresses.
    /// This is inherently unsafe but encapsulated in this method.
    #[cfg(not(target_arch = "wasm32"))]
    fn call_c(
        &self,
        lib_name: &str,
        symbol: &str,
        args: &[RuntimeValue],
        _ctx: &mut NativeContext<'_>,
    ) -> Result<RuntimeValue, ExecutorError> {
        // Get or load the library
        let lib = self.loaded_libs.get(lib_name).ok_or_else(|| {
            ExecutorError::runtime_only(format!(
                "C library not found: {lib_name}. Libraries must be pre-loaded."
            ))
        })?;

        // Simple case: () -> Int32 (e.g., getpid, rand)
        if args.is_empty() {
            type CIntFn = unsafe extern "C" fn() -> i32;
            let func: libloading::Symbol<'_, CIntFn> = unsafe {
                lib.get(symbol.as_bytes()).map_err(|e| {
                    ExecutorError::runtime_only(format!(
                        "symbol not found in {lib_name}: {symbol}: {e}"
                    ))
                })?
            };
            let result = unsafe { func() };
            return Ok(RuntimeValue::Int(result as i64));
        }
        Err(ExecutorError::runtime_only(
            "C ABI calls with arguments not yet implemented".to_string(),
        ))
    }

    /// Pre-load a dynamic library by name for C ABI calls.
    ///
    /// Example libraries: `"libc.so.6"`, `"libsqlite3.so"`, `"sqlite3.dll"` (Windows).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_library(
        &mut self,
        name: &str,
    ) -> Result<(), ExecutorError> {
        if !self.loaded_libs.contains_key(name) {
            let lib = Arc::new(unsafe { Library::new(name) }.map_err(|e| {
                ExecutorError::runtime_only(format!("failed to load library {name}: {e}"))
            })?);
            self.loaded_libs.insert(name.to_string(), lib);
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================
