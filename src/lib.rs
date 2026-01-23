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
pub mod backends;
pub mod frontend;
pub mod middle;
pub mod std;

// Utility modules
pub mod util;

// Re-exports
pub use anyhow::{Context, Result};
pub use thiserror::Error;

// Backend re-exports
pub use backends::{Executor, DebuggableExecutor, ExecutorError, ExecutorResult, ExecutorConfig};
pub use backends::common::{RuntimeValue, Opcode, Heap, Handle, BumpAllocator};
pub use backends::interpreter::Interpreter;
pub use backends::dev::{DevShell, Debugger, REPL};

// Logging
use crate::util::i18n::{t_cur, t_cur_simple, MSG};
use tracing::debug;

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
///         main() -> () = () => {
///             print("Hello, World!")
///         }
///     "#;
///     run(code)?;
///     Ok(())
/// }
/// ```
pub fn run(source: &str) -> Result<()> {
    debug!("{}", t_cur_simple(MSG::DebugRunCalled));
    let mut compiler = frontend::Compiler::new();
    debug!("{}", t_cur_simple(MSG::CompilationStart));
    let module = compiler.compile(source)?;
    // Generate BytecodeModule using the new backend architecture
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    // Convert BytecodeFile to BytecodeModule
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    // Use the new Interpreter backend
    let mut interpreter = backends::interpreter::Interpreter::new();
    debug!("{}", t_cur_simple(MSG::VmStart));
    interpreter.execute_module(&bytecode_module)?;
    debug!("{}", t_cur_simple(MSG::VmComplete));
    Ok(())
}

use ::std::fs;
use ::std::path::Path;

/// Run the interpreter on a file
pub fn run_file(path: &Path) -> Result<()> {
    let path_str = path.display().to_string();
    debug!("{}", t_cur(MSG::RunFile, Some(&[&path_str])));
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    debug!("{}", t_cur(MSG::ReadingFile, Some(&[&path_str])));
    run(&source)
}

/// Build bytecode file (.42)
pub fn build_bytecode(
    source_path: &Path,
    output_path: &Path,
) -> Result<()> {
    use crate::middle::codegen::CodegenContext;

    let source_path_str = source_path.display().to_string();
    let output_path_str = output_path.display().to_string();

    debug!("{}", t_cur_simple(MSG::BuildBytecode));
    let source = fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read source: {}", source_path.display()))?;
    debug!("{}", t_cur(MSG::ReadingFile, Some(&[&source_path_str])));

    // Compile
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile(&source)?;

    // Generate bytecode
    let mut ctx = CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    // Write to file
    let mut file = fs::File::create(output_path)
        .with_context(|| format!("Failed to create output: {}", output_path.display()))?;
    debug!("{}", t_cur(MSG::WritingBytecode, Some(&[&output_path_str])));
    bytecode_file
        .write_to(&mut file)
        .with_context(|| format!("Failed to write bytecode: {}", output_path.display()))?;

    Ok(())
}

/// Dump bytecode for debugging
pub fn dump_bytecode(path: &Path) -> Result<()> {
    // TODO: Update to use new backend architecture
    // This function is temporarily disabled during migration
    let path_str = path.display().to_string();
    println!("=== Bytecode Dump for {} ===\n", path_str);
    println!("Note: This function is being updated to use the new backend architecture.");
    println!("Please use the new debugging tools in backends/dev/ instead.");
    Ok(())
}
