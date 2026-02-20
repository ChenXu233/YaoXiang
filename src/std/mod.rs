//! Standard library
//!
//! This module contains built-in functions and types.

pub mod concurrent;
pub mod convert;
pub mod dict;
pub mod ffi;
pub mod io;
pub mod list;
pub mod math;
pub mod net;
pub mod os;
pub mod string;
pub mod time;
pub mod weak;

use crate::backends::interpreter::ffi::FfiRegistry;
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::backends::ExecutorError;
use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};

/// Execution context passed to native functions.
///
/// This gives native functions access to the heap (for allocating/reading
/// List/Dict values) and provides a callback for invoking YaoXiang functions
/// (enabling higher-order functions like map/filter/reduce).
pub struct NativeContext<'a> {
    /// Heap memory manager
    pub heap: &'a mut Heap,
    /// Callback to invoke a YaoXiang function value with given arguments.
    /// The closure takes (function_value, args) and returns a RuntimeValue.
    pub call_fn: Option<
        &'a mut dyn FnMut(&RuntimeValue, &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>,
    >,
}

impl<'a> NativeContext<'a> {
    /// Create a new NativeContext with heap access only (no function callback).
    pub fn new(heap: &'a mut Heap) -> Self {
        Self {
            heap,
            call_fn: None,
        }
    }

    /// Create a NativeContext with both heap access and function call capability.
    pub fn with_call_fn(
        heap: &'a mut Heap,
        call_fn: &'a mut dyn FnMut(
            &RuntimeValue,
            &[RuntimeValue],
        ) -> Result<RuntimeValue, ExecutorError>,
    ) -> Self {
        Self {
            heap,
            call_fn: Some(call_fn),
        }
    }

    /// Invoke a YaoXiang function value with the given arguments.
    ///
    /// Returns an error if no call_fn callback is available.
    pub fn call_function(
        &mut self,
        func: &RuntimeValue,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, ExecutorError> {
        if let Some(ref mut callback) = self.call_fn {
            callback(func, args)
        } else {
            Err(ExecutorError::Runtime(
                "Cannot call YaoXiang functions from this native context".to_string(),
            ))
        }
    }
}

/// Type alias for native function handlers.
///
/// Native handlers now receive a `NativeContext` which provides:
/// - `ctx.heap` for heap allocation (List/Dict creation)
/// - `ctx.call_function()` for invoking YaoXiang functions (higher-order functions)
pub type NativeHandler =
    fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>;

/// Native function export declaration (type-safe alternative to tuple).
///
/// This replaces the previous tuple format: (name, native_name, signature).
#[derive(Debug, Clone)]
pub struct NativeExport {
    /// Short name (e.g., "print")
    pub name: &'static str,
    /// Fully qualified FFI name (e.g., "std.io.print")
    pub native_name: &'static str,
    /// Function signature (e.g., "(value: Any) -> Void")
    pub signature: &'static str,
    /// Native handler function (optional, for FFI registration)
    pub handler: Option<NativeHandler>,
}

impl NativeExport {
    /// Create a new native export with a handler.
    pub const fn new(
        name: &'static str,
        native_name: &'static str,
        signature: &'static str,
        handler: NativeHandler,
    ) -> Self {
        Self {
            name,
            native_name,
            signature,
            handler: Some(handler),
        }
    }

    /// Create a constant export (without handler, e.g., for PI).
    pub const fn constant(
        name: &'static str,
        native_name: &'static str,
        signature: &'static str,
    ) -> Self {
        Self {
            name,
            native_name,
            signature,
            handler: None,
        }
    }
}

/// Trait for std modules to self-register.
///
/// Each std sub-module (io, math, net, etc.) implements this trait
/// to provide its exports and FFI handlers.
pub trait StdModule {
    /// Returns the module path (e.g., "std.io").
    fn module_path(&self) -> &str;

    /// Returns all exports declared by this module.
    fn exports(&self) -> Vec<NativeExport>;

    /// Registers this module's functions into the FFI registry.
    fn register_ffi(
        &self,
        registry: &mut FfiRegistry,
    ) {
        for export in self.exports() {
            if let Some(handler) = export.handler {
                registry.register(export.native_name, handler);
            }
        }
    }

    /// Converts exports to ModuleInfo for the frontend module system.
    fn to_module_info(&self) -> ModuleInfo {
        let path = self.module_path().to_string();
        let mut module = ModuleInfo::new(path, ModuleSource::Std);

        for export in self.exports() {
            let kind = if export.signature.starts_with('(') {
                ExportKind::Function
            } else {
                ExportKind::Constant
            };

            module.add_export(Export {
                name: export.name.to_string(),
                full_path: export.native_name.to_string(),
                kind,
                signature: export.signature.to_string(),
            });
        }

        module
    }
}

// ============================================================================
// Built-in generic functions (replacing hardcoded interpreter special cases)
// ============================================================================

/// Built-in generic `len` function that works on List, Tuple, Array, Dict, String, Bytes.
fn builtin_len(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() != 1 {
        return Err(ExecutorError::Type(
            "len expects exactly 1 argument".to_string(),
        ));
    }
    let len = match &args[0] {
        RuntimeValue::List(handle)
        | RuntimeValue::Tuple(handle)
        | RuntimeValue::Array(handle)
        | RuntimeValue::Dict(handle) => ctx
            .heap
            .get(*handle)
            .map(|value| value.len() as i64)
            .unwrap_or(0),
        RuntimeValue::String(s) => s.chars().count() as i64,
        RuntimeValue::Bytes(b) => b.len() as i64,
        _ => 0,
    };
    Ok(RuntimeValue::Int(len))
}

/// Built-in `dict_keys` function that returns a list of keys from a dictionary.
fn builtin_dict_keys(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() != 1 {
        return Err(ExecutorError::Type(
            "dict_keys expects exactly 1 argument".to_string(),
        ));
    }
    let keys = match &args[0] {
        RuntimeValue::Dict(handle) => match ctx.heap.get(*handle) {
            Some(HeapValue::Dict(map)) => map.keys().cloned().collect::<Vec<_>>(),
            _ => Vec::new(),
        },
        _ => {
            return Err(ExecutorError::Type(
                "dict_keys only supports dict".to_string(),
            ));
        }
    };
    let list_handle = ctx.heap.allocate(HeapValue::List(keys));
    Ok(RuntimeValue::List(list_handle))
}

/// Register all std modules into the FFI registry.
///
/// This is the single entry point that ffi.rs should call.
/// New std modules only need to be added to this function.
pub fn register_all(registry: &mut FfiRegistry) {
    convert::ConvertModule.register_ffi(registry);
    dict::DictModule.register_ffi(registry);
    io::IoModule.register_ffi(registry);
    list::ListModule.register_ffi(registry);
    math::MathModule.register_ffi(registry);
    net::NetModule.register_ffi(registry);
    concurrent::ConcurrentModule.register_ffi(registry);
    string::StringModule.register_ffi(registry);
    time::TimeModule.register_ffi(registry);
    os::OsModule.register_ffi(registry);

    // Register built-in generic functions (replacing hardcoded interpreter special cases)
    registry.register("len", builtin_len as NativeHandler);
    registry.register("dict_keys", builtin_dict_keys as NativeHandler);
}

/// Get ModuleInfo for all std modules.
///
/// This is used by the frontend module system.
pub fn all_module_infos() -> Vec<ModuleInfo> {
    vec![
        convert::ConvertModule.to_module_info(),
        dict::DictModule.to_module_info(),
        io::IoModule.to_module_info(),
        list::ListModule.to_module_info(),
        math::MathModule.to_module_info(),
        net::NetModule.to_module_info(),
        concurrent::ConcurrentModule.to_module_info(),
        string::StringModule.to_module_info(),
        time::TimeModule.to_module_info(),
        os::OsModule.to_module_info(),
    ]
}

/// Get all native function names (short name, native name pairs) from all std modules.
///
/// This is used by the code generator to discover native functions.
pub fn all_native_names() -> Vec<(String, String)> {
    let mut names = Vec::new();
    for module_info in all_module_infos() {
        for export in module_info.exports.values() {
            if export.kind == ExportKind::Function {
                names.push((export.name.clone(), export.full_path.clone()));
            }
        }
    }
    names
}
