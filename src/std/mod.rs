//! Standard library
//!
//! This module contains built-in functions and types.

pub mod concurrent;
pub mod dict;
pub mod ffi;
pub mod io;
pub mod list;
pub mod math;
pub mod net;
pub mod string;
pub mod weak;

/// Represents a function exported from a std module.
#[derive(Debug, Clone)]
pub struct ModuleExport {
    /// Short name (e.g., "print")
    pub short_name: &'static str,
    /// Fully qualified name (e.g., "std.io.print")
    pub qualified_name: &'static str,
    /// Function signature (e.g., "(value: Any) -> Void")
    pub signature: &'static str,
}

/// Get all exports from a std module.
///
/// Returns None if the module doesn't exist or has no exports.
///
/// This function delegates to `ModuleRegistry` for a unified module lookup.
/// It converts the generic `Export` format back to `ModuleExport` for backward
/// compatibility with existing callers.
pub fn get_module_exports(module_path: &str) -> Option<Vec<ModuleExport>> {
    use crate::frontend::module::registry::ModuleRegistry;

    let registry = ModuleRegistry::with_std();

    let module = registry.get(module_path)?;

    let exports: Vec<ModuleExport> = module
        .exports
        .values()
        .map(|export| {
            // SAFETY: We leak static strings here as ModuleExport requires &'static str.
            // This is acceptable because std module metadata is effectively static program data.
            let short_name: &'static str = Box::leak(export.name.clone().into_boxed_str());
            let qualified_name: &'static str = Box::leak(export.full_path.clone().into_boxed_str());
            let signature: &'static str = Box::leak(export.signature.clone().into_boxed_str());

            ModuleExport {
                short_name,
                qualified_name,
                signature,
            }
        })
        .collect();

    if exports.is_empty() && module.is_namespace() {
        // For namespace modules like "std", return submodule list
        let submodule_exports: Vec<ModuleExport> = module
            .submodules
            .iter()
            .map(|name| {
                let short_name: &'static str = Box::leak(name.clone().into_boxed_str());
                let qualified_name: &'static str =
                    Box::leak(format!("{}.{}", module_path, name).into_boxed_str());
                ModuleExport {
                    short_name,
                    qualified_name,
                    signature: "Module",
                }
            })
            .collect();
        Some(submodule_exports)
    } else {
        Some(exports)
    }
}
