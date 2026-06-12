//! FFI (Foreign Function Interface) Registry for YaoXiang
//!
//! This module provides the `FfiRegistry` which manages native function bindings,
//! allowing YaoXiang code to call Rust functions. It supports:
//! - Pre-registered standard library functions (std.io, etc.)
//! - User-defined native function registration
//!
//! # Architecture
//!
//! ```text
//! CallNative { "std.io.println" }
//!       │
//!       ▼
//! FfiRegistry.call() → direct handler lookup
//! ```

use std::collections::HashMap;

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeHandler};

/// FFI Registry that manages native function bindings.
///
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
pub struct FfiRegistry {
    /// Function handler table: name -> handler
    handlers: HashMap<String, NativeHandler>,
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
}

// =============================================================================
// Tests
// =============================================================================
