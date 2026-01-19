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
pub mod util;

// Re-exports
pub use anyhow::{Context, Result};
pub use thiserror::Error;

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
    let mut vm = vm::VM::new();
    debug!("{}", t_cur_simple(MSG::VmStart));
    vm.execute_module(&module)?;
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
    use crate::middle::codegen::CodegenContext;
    use crate::vm::opcode::TypedOpcode;

    let path_str = path.display().to_string();

    println!("=== Bytecode Dump for {} ===\n", path_str);

    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Compile
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile(&source)?;

    // Generate bytecode
    let mut ctx = CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    // Header
    println!("--- Header ---");
    println!("Magic: 0x{:08X} (YXBC)", bytecode_file.header.magic);
    println!("Version: {}", bytecode_file.header.version);
    println!("Entry Point: {}", bytecode_file.header.entry_point);
    println!("File Size: {} bytes", bytecode_file.header.file_size);
    println!("Flags: 0x{:08X}\n", bytecode_file.header.flags);

    // Type Table
    println!(
        "--- Type Table ({} entries) ---",
        bytecode_file.type_table.len()
    );
    for (i, ty) in bytecode_file.type_table.iter().enumerate() {
        println!("  [{}] {:?}", i, ty);
    }
    if bytecode_file.type_table.is_empty() {
        println!("  (empty)");
    }
    println!();

    // Constants
    println!(
        "--- Constants ({} entries) ---",
        bytecode_file.const_pool.len()
    );
    for (i, c) in bytecode_file.const_pool.iter().enumerate() {
        println!("  [{}] {:?}", i, c);
    }
    if bytecode_file.const_pool.is_empty() {
        println!("  (empty)");
    }
    println!();

    // Functions
    println!(
        "--- Functions ({} entries) ---",
        bytecode_file.code_section.functions.len()
    );
    for (i, func) in bytecode_file.code_section.functions.iter().enumerate() {
        println!(
            "Function {}: params={:?}, return={}, locals={}, instructions={}",
            i,
            func.params,
            func.return_type,
            func.local_count,
            func.instructions.len()
        );

        for (j, instr) in func.instructions.iter().enumerate() {
            let opcode_value = instr.opcode;
            let opcode_name = TypedOpcode::try_from(opcode_value)
                .map(|o| format!("{:?}", o))
                .unwrap_or_else(|_| format!("Unknown(0x{:02X})", opcode_value));

            if instr.operands.is_empty() {
                println!("  [{:3}] {}", j, opcode_name);
            } else {
                let operands: Vec<String> =
                    instr.operands.iter().map(|b| format!("{}", b)).collect();
                println!("  [{:3}] {} [{}]", j, opcode_name, operands.join(", "));
            }
        }
    }

    Ok(())
}
