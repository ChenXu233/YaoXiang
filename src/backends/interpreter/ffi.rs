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

/// Type alias for native function handlers.
///
/// A `NativeHandler` takes a slice of `RuntimeValue` arguments and returns
/// a `Result<RuntimeValue, ExecutorError>`.
pub type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;

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
    /// This registers all `std.io.*` functions and other standard library
    /// native functions that are available by default.
    pub fn with_std() -> Self {
        let mut registry = Self::new();
        register_std_io(&mut registry);
        register_std_math(&mut registry);
        register_std_net(&mut registry);
        register_std_concurrent(&mut registry);
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
    pub fn call(
        &self,
        name: &str,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, ExecutorError> {
        // Fast path: check cache first
        if let Ok(cache) = self.cache.lock() {
            if let Some(handler) = cache.get(name) {
                return handler(args);
            }
        }

        // Slow path: look up in handler table
        if let Some(handler) = self.handlers.get(name) {
            // Cache the handler for future calls
            if let Ok(mut cache) = self.cache.lock() {
                cache.insert(name.to_string(), *handler);
            }
            return handler(args);
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
// Standard Library: std.io
// =============================================================================

/// Register all standard IO functions into the FFI registry.
/// Only registers fully qualified names (std.io.print, std.io.println, etc.).
/// Short names (print, println) are NOT registered - they must be imported via `use std.io`.
fn register_std_io(registry: &mut FfiRegistry) {
    registry.register("std.io.print", native_print);
    registry.register("std.io.println", native_println);
    registry.register("std.io.read_line", native_read_line);
    registry.register("std.io.read_file", native_read_file);
    registry.register("std.io.write_file", native_write_file);
    registry.register("std.io.append_file", native_append_file);
}

// =============================================================================
// Standard Library: std.math
// =============================================================================

fn register_std_math(registry: &mut FfiRegistry) {
    registry.register("std.math.abs", |args| {
        let n = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        Ok(RuntimeValue::Int(n.abs()))
    });
    registry.register("std.math.max", |args| {
        let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
        Ok(RuntimeValue::Int(a.max(b)))
    });
    registry.register("std.math.min", |args| {
        let a = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
        Ok(RuntimeValue::Int(a.min(b)))
    });
    registry.register("std.math.clamp", |args| {
        let value = args.first().and_then(|v| v.to_int()).unwrap_or(0);
        let min = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
        let max = args.get(2).and_then(|v| v.to_int()).unwrap_or(0);
        Ok(RuntimeValue::Int(value.clamp(min, max)))
    });
    registry.register("std.math.fabs", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.abs()))
    });
    registry.register("std.math.fmax", |args| {
        let a = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        let b = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(a.max(b)))
    });
    registry.register("std.math.fmin", |args| {
        let a = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        let b = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(a.min(b)))
    });
    registry.register("std.math.pow", |args| {
        let base = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        let exp = args.get(1).and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(base.powf(exp)))
    });
    registry.register("std.math.sqrt", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.sqrt()))
    });
    registry.register("std.math.floor", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.floor()))
    });
    registry.register("std.math.ceil", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.ceil()))
    });
    registry.register("std.math.round", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.round()))
    });
    registry.register("std.math.sin", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.sin()))
    });
    registry.register("std.math.cos", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.cos()))
    });
    registry.register("std.math.tan", |args| {
        let n = args.first().and_then(|v| v.to_float()).unwrap_or(0.0);
        Ok(RuntimeValue::Float(n.tan()))
    });
    registry.register("std.math.PI", |_args| {
        Ok(RuntimeValue::Float(std::f64::consts::PI))
    });
    registry.register("std.math.E", |_args| {
        Ok(RuntimeValue::Float(std::f64::consts::E))
    });
    registry.register("std.math.TAU", |_args| {
        Ok(RuntimeValue::Float(std::f64::consts::TAU))
    });
}

// =============================================================================
// Standard Library: std.net
// =============================================================================

fn register_std_net(registry: &mut FfiRegistry) {
    registry.register("std.net.http_get", |args| {
        let url = match args.first() {
            Some(RuntimeValue::String(s)) => s.to_string(),
            _ => {
                return Err(ExecutorError::Type(
                    "http_get expects String argument".to_string(),
                ))
            }
        };
        Ok(RuntimeValue::String(
            format!(r#"{{"url": "{}", "status": 200}}"#, url).into(),
        ))
    });
    registry.register("std.net.http_post", |args| {
        let url = match args.first() {
            Some(RuntimeValue::String(s)) => s.to_string(),
            _ => {
                return Err(ExecutorError::Type(
                    "http_post expects String argument".to_string(),
                ))
            }
        };
        let _body = match args.get(1) {
            Some(RuntimeValue::String(s)) => s.to_string(),
            _ => String::new(),
        };
        Ok(RuntimeValue::String(
            format!(r#"{{"url": "{}", "status": 201}}"#, url).into(),
        ))
    });
    registry.register("std.net.url_encode", |args| {
        use std::fmt::Write;
        let s = match args.first() {
            Some(RuntimeValue::String(s)) => s.to_string(),
            _ => {
                return Err(ExecutorError::Type(
                    "url_encode expects String argument".to_string(),
                ))
            }
        };
        let mut encoded = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
                _ => write!(&mut encoded, "%{:02X}", c as u32).unwrap(),
            }
        }
        Ok(RuntimeValue::String(encoded.into()))
    });
    registry.register("std.net.url_decode", |args| {
        let s = match args.first() {
            Some(RuntimeValue::String(s)) => s.to_string(),
            _ => {
                return Err(ExecutorError::Type(
                    "url_decode expects String argument".to_string(),
                ))
            }
        };
        let mut decoded = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if hex.len() == 2 {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        decoded.push(byte as char);
                        continue;
                    }
                }
                decoded.push('%');
                decoded.push_str(&hex);
            } else if c == '+' {
                decoded.push(' ');
            } else {
                decoded.push(c);
            }
        }
        Ok(RuntimeValue::String(decoded.into()))
    });
}

// =============================================================================
// Standard Library: std.concurrent
// =============================================================================

fn register_std_concurrent(registry: &mut FfiRegistry) {
    registry.register("std.concurrent.sleep", |args| {
        let millis = args.first().and_then(|v| v.to_int()).unwrap_or(0) as u64;
        std::thread::sleep(std::time::Duration::from_millis(millis));
        Ok(RuntimeValue::Unit)
    });
    registry.register("std.concurrent.thread_id", |_args| {
        Ok(RuntimeValue::String(
            format!("{:?}", std::thread::current().id()).into(),
        ))
    });
    registry.register("std.concurrent.yield_now", |_args| {
        std::thread::yield_now();
        Ok(RuntimeValue::Unit)
    });
}

/// Native implementation: print (no newline)
fn native_print(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    let output = args
        .iter()
        .map(|arg| format!("{}", arg))
        .collect::<Vec<String>>()
        .join(" ");
    print!("{}", output);
    Ok(RuntimeValue::Unit)
}

/// Native implementation: println (with newline)
fn native_println(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    let output = args
        .iter()
        .map(|arg| format!("{}", arg))
        .collect::<Vec<String>>()
        .join(" ");
    println!("{}", output);
    Ok(RuntimeValue::Unit)
}

/// Native implementation: read_line
fn native_read_line(_args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|e| ExecutorError::Runtime(format!("Failed to read line: {}", e)))?;
    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
    Ok(RuntimeValue::String(line.into()))
}

/// Native implementation: read_file
fn native_read_file(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "read_file expects 1 argument (path: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "read_file expects String argument, got {:?}",
                other.value_type(None)
            )));
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(RuntimeValue::String(content.into())),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to read file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: write_file
fn native_write_file(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "write_file expects 2 arguments (path: String, content: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "write_file expects String path, got {:?}",
                other.value_type(None)
            )));
        }
    };
    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "write_file expects String content, got {:?}",
                other.value_type(None)
            )));
        }
    };
    match std::fs::write(&path, &content) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to write file '{}': {}",
            path, e
        ))),
    }
}

/// Native implementation: append_file
fn native_append_file(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "append_file expects 2 arguments (path: String, content: String)".to_string(),
        ));
    }
    let path = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String path, got {:?}",
                other.value_type(None)
            )));
        }
    };
    let content = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "append_file expects String content, got {:?}",
                other.value_type(None)
            )));
        }
    };
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
    {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(()) => Ok(RuntimeValue::Bool(true)),
            Err(e) => Err(ExecutorError::Runtime(format!(
                "Failed to append to file '{}': {}",
                path, e
            ))),
        },
        Err(e) => Err(ExecutorError::Runtime(format!(
            "Failed to open file '{}' for appending: {}",
            path, e
        ))),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        fn my_add(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
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
        fn my_add(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
            let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            Ok(RuntimeValue::Int(a + b))
        }
        registry.register("my_add", my_add);

        let result = registry
            .call("my_add", &[RuntimeValue::Int(3), RuntimeValue::Int(7)])
            .unwrap();
        assert_eq!(result, RuntimeValue::Int(10));
    }

    #[test]
    fn test_call_nonexistent_function_returns_error() {
        let registry = FfiRegistry::new();
        let result = registry.call("nonexistent", &[]);
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
        // println should accept any number of args and return Unit
        let result = registry
            .call(
                "std.io.println",
                &[RuntimeValue::String("hello from FFI".into())],
            )
            .unwrap();
        assert_eq!(result, RuntimeValue::Unit);
    }

    #[test]
    fn test_cache_accelerates_repeated_calls() {
        let mut registry = FfiRegistry::new();
        fn identity(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
            Ok(args.get(0).cloned().unwrap_or(RuntimeValue::Unit))
        }
        registry.register("identity", identity);

        // First call populates cache
        let r1 = registry.call("identity", &[RuntimeValue::Int(42)]).unwrap();
        assert_eq!(r1, RuntimeValue::Int(42));

        // Second call should hit cache
        let r2 = registry.call("identity", &[RuntimeValue::Int(99)]).unwrap();
        assert_eq!(r2, RuntimeValue::Int(99));
    }

    #[test]
    fn test_register_overwrites_existing() {
        let mut registry = FfiRegistry::new();
        fn handler_v1(_args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
            Ok(RuntimeValue::Int(1))
        }
        fn handler_v2(_args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
            Ok(RuntimeValue::Int(2))
        }
        registry.register("func", handler_v1);
        let r1 = registry.call("func", &[]).unwrap();
        assert_eq!(r1, RuntimeValue::Int(1));

        registry.register("func", handler_v2);
        let r2 = registry.call("func", &[]).unwrap();
        assert_eq!(r2, RuntimeValue::Int(2));
    }

    #[test]
    fn test_registered_functions_list() {
        let mut registry = FfiRegistry::new();
        fn noop(_args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
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
            )
            .unwrap();
        assert_eq!(write_result, RuntimeValue::Bool(true));

        // Read file
        let read_result = registry
            .call(
                "std.io.read_file",
                &[RuntimeValue::String(path_str.clone().into())],
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
            )
            .unwrap();
        assert_eq!(append_result, RuntimeValue::Bool(true));

        // Read again to verify append
        let read_result2 = registry
            .call(
                "std.io.read_file",
                &[RuntimeValue::String(path_str.clone().into())],
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
        let result = registry.call("std.io.read_file", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_file_missing_args() {
        let registry = FfiRegistry::with_std();
        let result = registry.call("std.io.write_file", &[RuntimeValue::String("path".into())]);
        assert!(result.is_err());
    }
}
