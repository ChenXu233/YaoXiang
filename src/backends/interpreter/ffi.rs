//! FFI (Foreign Function Interface) Registry for YaoXiang
//!
//! This module provides the `FfiRegistry` which manages native function bindings,
//! allowing YaoXiang code to call Rust functions. It supports:
//! - Pre-registered standard library functions (std.io, etc.)
//! - User-defined native function registration
//! - Cached function lookup for zero-overhead repeated calls
//!
//! # Architecture
//!
//! ```text
//! CallNative { "std.io.println" }
//!       │
//!       ▼
//! FfiRegistry.call() → cache lookup → handler execution
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::NativeContext;

/// Type alias for native function handlers.
///
/// A `NativeHandler` takes a slice of `RuntimeValue` arguments and a
/// `NativeContext` that provides heap access and function call capability.
pub type NativeHandler =
    fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>;

/// FFI Registry that manages native function bindings.
///
/// The registry holds a mapping from function names (e.g., `"std.io.println"`)
/// to their native Rust implementations. It uses a cache layer for fast repeated
/// lookups.
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
    /// Runtime cache for accelerated lookup (thread-safe)
    cache: Mutex<HashMap<String, NativeHandler>>,
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
            cache: Mutex::new(HashMap::new()),
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
        // Invalidate cache entry if it exists
        if let Ok(mut cache) = self.cache.lock() {
            cache.remove(name);
        }
    }

    /// Call a registered native function by name.
    ///
    /// This method first checks the cache, then falls back to the handler table.
    /// Successfully resolved handlers are cached for subsequent calls.
    ///
    /// # Arguments
    ///
    /// * `name` - The fully qualified function name
    /// * `args` - The arguments to pass to the function
    ///
    /// # Errors
    ///
    /// Returns `ExecutorError::FunctionNotFound` if no handler is registered
    /// for the given name.
    /// Call a registered native function by name.
    ///
    /// This method first checks the cache, then falls back to the handler table.
    /// Successfully resolved handlers are cached for subsequent calls.
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
        // Fast path: check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(handler) = cache.get(name) {
                return handler(args, ctx);
            }
        }

        // Slow path: look up in handler table
        if let Some(handler) = self.handlers.get(name) {
            // Cache the handler for future calls
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(name.to_string(), *handler);
            }
            return handler(args, ctx);
        }

        Err(ExecutorError::FunctionNotFound(format!(
            "Native function not found: {}",
            name
        )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::common::Heap;

    /// Helper to create a test NativeContext
    fn test_ctx(heap: &mut Heap) -> NativeContext {
        NativeContext::new(heap)
    }

    #[test]
    fn test_new_registry_is_empty() {
        let registry = FfiRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_with_std_has_io_functions() {
        let registry = FfiRegistry::with_std();
        assert!(!registry.is_empty());
        // Only fully qualified names are registered by default
        assert!(registry.has("std.io.print"));
        assert!(registry.has("std.io.println"));
        assert!(registry.has("std.io.read_line"));
        assert!(registry.has("std.io.read_file"));
        assert!(registry.has("std.io.write_file"));
        assert!(registry.has("std.io.append_file"));
        // Short names are NOT registered - users must use `use std.io` to bring them into scope
        assert!(!registry.has("print"));
        assert!(!registry.has("println"));
    }

    #[test]
    fn test_register_custom_function() {
        let mut registry = FfiRegistry::new();
        fn my_add(
            args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            Ok(RuntimeValue::Int(a + b))
        }
        registry.register("my_add", my_add);
        assert!(registry.has("my_add"));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_call_custom_function() {
        let mut registry = FfiRegistry::new();
        fn my_add(
            args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            Ok(RuntimeValue::Int(a + b))
        }
        registry.register("my_add", my_add);

        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let result = registry
            .call(
                "my_add",
                &[RuntimeValue::Int(3), RuntimeValue::Int(7)],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(result, RuntimeValue::Int(10));
    }

    #[test]
    fn test_call_nonexistent_function_returns_error() {
        let registry = FfiRegistry::new();
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let result = registry.call("nonexistent", &[], &mut ctx);
        assert!(result.is_err());
        match result {
            Err(ExecutorError::FunctionNotFound(msg)) => {
                assert!(msg.contains("nonexistent"));
            }
            _ => panic!("Expected FunctionNotFound error"),
        }
    }

    #[test]
    fn test_call_println_via_registry() {
        let registry = FfiRegistry::with_std();
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        // println should accept any number of args and return Unit
        let result = registry
            .call(
                "std.io.println",
                &[RuntimeValue::String("hello from FFI".into())],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(result, RuntimeValue::Unit);
    }

    #[test]
    fn test_cache_accelerates_repeated_calls() {
        let mut registry = FfiRegistry::new();
        fn identity(
            args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            Ok(args.get(0).cloned().unwrap_or(RuntimeValue::Unit))
        }
        registry.register("identity", identity);

        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        // First call populates cache
        let r1 = registry
            .call("identity", &[RuntimeValue::Int(42)], &mut ctx)
            .unwrap();
        assert_eq!(r1, RuntimeValue::Int(42));

        // Second call should hit cache
        let r2 = registry
            .call("identity", &[RuntimeValue::Int(99)], &mut ctx)
            .unwrap();
        assert_eq!(r2, RuntimeValue::Int(99));
    }

    #[test]
    fn test_register_overwrites_existing() {
        let mut registry = FfiRegistry::new();
        fn handler_v1(
            _args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            Ok(RuntimeValue::Int(1))
        }
        fn handler_v2(
            _args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            Ok(RuntimeValue::Int(2))
        }
        registry.register("func", handler_v1);
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let r1 = registry.call("func", &[], &mut ctx).unwrap();
        assert_eq!(r1, RuntimeValue::Int(1));

        registry.register("func", handler_v2);
        let r2 = registry.call("func", &[], &mut ctx).unwrap();
        assert_eq!(r2, RuntimeValue::Int(2));
    }

    #[test]
    fn test_registered_functions_list() {
        let mut registry = FfiRegistry::new();
        fn noop(
            _args: &[RuntimeValue],
            _ctx: &mut NativeContext<'_>,
        ) -> Result<RuntimeValue, ExecutorError> {
            Ok(RuntimeValue::Unit)
        }
        registry.register("alpha", noop);
        registry.register("beta", noop);

        let names = registry.registered_functions();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_write_and_read_file() {
        let registry = FfiRegistry::with_std();
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let test_path = std::env::temp_dir().join("yx_ffi_test.txt");
        let path_str = test_path.to_string_lossy().to_string();

        // Write file
        let write_result = registry
            .call(
                "std.io.write_file",
                &[
                    RuntimeValue::String(path_str.clone().into()),
                    RuntimeValue::String("FFI test content".into()),
                ],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(write_result, RuntimeValue::Bool(true));

        // Read file
        let read_result = registry
            .call(
                "std.io.read_file",
                &[RuntimeValue::String(path_str.clone().into())],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(read_result, RuntimeValue::String("FFI test content".into()));

        // Append file
        let append_result = registry
            .call(
                "std.io.append_file",
                &[
                    RuntimeValue::String(path_str.clone().into()),
                    RuntimeValue::String(" appended".into()),
                ],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(append_result, RuntimeValue::Bool(true));

        // Read again to verify append
        let read_result2 = registry
            .call(
                "std.io.read_file",
                &[RuntimeValue::String(path_str.clone().into())],
                &mut ctx,
            )
            .unwrap();
        assert_eq!(
            read_result2,
            RuntimeValue::String("FFI test content appended".into())
        );

        // Cleanup
        let _ = std::fs::remove_file(&test_path);
    }

    #[test]
    fn test_read_file_missing_args() {
        let registry = FfiRegistry::with_std();
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let result = registry.call("std.io.read_file", &[], &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_file_missing_args() {
        let registry = FfiRegistry::with_std();
        let mut heap = Heap::new();
        let mut ctx = test_ctx(&mut heap);
        let result = registry.call(
            "std.io.write_file",
            &[RuntimeValue::String("path".into())],
            &mut ctx,
        );
        assert!(result.is_err());
    }
}
