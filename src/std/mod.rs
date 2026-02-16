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
pub fn get_module_exports(module_path: &str) -> Option<Vec<ModuleExport>> {
    match module_path {
        // "std" 模块：返回所有子模块
        "std" => Some(vec![
            ModuleExport {
                short_name: "io",
                qualified_name: "std.io",
                signature: "Module",
            },
            ModuleExport {
                short_name: "math",
                qualified_name: "std.math",
                signature: "Module",
            },
            ModuleExport {
                short_name: "net",
                qualified_name: "std.net",
                signature: "Module",
            },
            ModuleExport {
                short_name: "concurrent",
                qualified_name: "std.concurrent",
                signature: "Module",
            },
        ]),
        "std.io" => Some(
            io::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| ModuleExport {
                    short_name: d.name,
                    qualified_name: d.native_name,
                    signature: d.signature,
                })
                .collect(),
        ),
        "std.math" => Some(
            math::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| ModuleExport {
                    short_name: d.name,
                    qualified_name: d.native_name,
                    signature: d.signature,
                })
                .collect(),
        ),
        "std.net" => Some(
            net::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| ModuleExport {
                    short_name: d.name,
                    qualified_name: d.native_name,
                    signature: d.signature,
                })
                .collect(),
        ),
        "std.concurrent" => Some(
            concurrent::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| ModuleExport {
                    short_name: d.name,
                    qualified_name: d.native_name,
                    signature: d.signature,
                })
                .collect(),
        ),
        _ => None,
    }
}
