//! YaoXiang Programming Language
//!
//! A high-performance programming language with "everything is type" philosophy.
//!
//! # Example
//!
//! ```yaoxiang
//! fn main() {
//!     print("Hello, YaoXiang!")
//! }
//! ```
//!
//! # Crate Features
//!
//! - `wasm`: Enable WebAssembly support

#![doc(html_root_url = "https://docs.rs/yaoxiang")]
#![warn(rust_2018_idioms)]
#![allow(dead_code)]

// Public modules
pub mod frontend;
pub mod middle;
pub mod runtime;
pub mod std;
pub mod vm;

// Utility modules
mod util;

// Re-exports
pub use anyhow::{Context, Result};
pub use thiserror::Error;

/// Language version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Language name
pub const NAME: &str = "YaoXiang (爻象)";

/// Run the interpreter on source code
///
/// # Example
///
/// ```no_run
/// use yaoxiang::{run, Result};
///
/// fn main() -> Result<()> {
///     let code = r#"
///         fn main() {
///             print("Hello, World!")
///         }
///     "#;
///     run(code)?;
///     Ok(())
/// }
/// ```
pub fn run(source: &str) -> Result<()> {
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile(source)?;
    let mut vm = vm::VM::new();
    vm.execute_module(&module)?;
    Ok(())
}

use ::std::fs;
use ::std::path::Path;

/// Run the interpreter on a file
pub fn run_file(path: &Path) -> Result<()> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    run(&source)
}
