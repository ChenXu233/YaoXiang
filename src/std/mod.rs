//! Standard library
//!
//! This module contains built-in functions and types.

pub mod concurrent;
pub mod ffi;
pub mod io;
pub mod math;
pub mod net;
pub mod os;
pub mod time;
pub mod weak;

use crate::backends::interpreter::ffi::FfiRegistry;
use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};

/// Type alias for native function handlers.
pub type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;

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

/// Register all std modules into the FFI registry.
///
/// This is the single entry point that ffi.rs should call.
/// New std modules only need to be added to this function.
pub fn register_all(registry: &mut FfiRegistry) {
    io::IoModule.register_ffi(registry);
    math::MathModule.register_ffi(registry);
    net::NetModule.register_ffi(registry);
    concurrent::ConcurrentModule.register_ffi(registry);
    time::TimeModule.register_ffi(registry);
    os::OsModule.register_ffi(registry);
}

/// Get ModuleInfo for all std modules.
///
/// This is used by the frontend module system.
pub fn all_module_infos() -> Vec<ModuleInfo> {
    vec![
        io::IoModule.to_module_info(),
        math::MathModule.to_module_info(),
        net::NetModule.to_module_info(),
        concurrent::ConcurrentModule.to_module_info(),
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
