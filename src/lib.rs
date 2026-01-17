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
    debug!("run() called");
    let mut compiler = frontend::Compiler::new();
    debug!("Starting compilation...");
    let module = compiler.compile(source)?;
    debug!("Compilation successful!");
    let mut vm = vm::VM::new();
    debug!("VM created, executing module...");
    vm.execute_module(&module)?;
    debug!("Module execution completed!");
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

/// Build bytecode file (.42)
pub fn build_bytecode(
    source_path: &Path,
    output_path: &Path,
) -> Result<()> {
    use crate::middle::codegen::CodegenContext;

    let source = fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read source: {}", source_path.display()))?;

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
    bytecode_file
        .write_to(&mut file)
        .with_context(|| format!("Failed to write bytecode: {}", output_path.display()))?;

    eprintln!("[INFO] Bytecode written to: {}", output_path.display());
    Ok(())
}

/// Dump bytecode for debugging
pub fn dump_bytecode(path: &Path) -> Result<()> {
    use crate::middle::codegen::CodegenContext;
    use crate::vm::opcode::TypedOpcode;

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

    // Dump human-readable format
    println!("=== Bytecode Dump for {} ===", path.display());
    println!("\n--- Header ---");
    println!("Magic: 0x{:08X} (YXBC)", bytecode_file.header.magic);
    println!("Version: {}", bytecode_file.header.version);
    println!("Entry Point: {}", bytecode_file.header.entry_point);
    println!("File Size: {} bytes", bytecode_file.header.file_size);
    println!("Flags: 0x{:08X}", bytecode_file.header.flags);

    println!("\n--- Type Table ({}) ---", bytecode_file.type_table.len());
    for (i, ty) in bytecode_file.type_table.iter().enumerate() {
        println!("  [{}] {:?}", i, ty);
    }

    println!("\n--- Constants ({}) ---", bytecode_file.const_pool.len());
    for (i, c) in bytecode_file.const_pool.iter().enumerate() {
        println!("  [{}] {:?}", i, c);
    }

    println!(
        "\n--- Functions ({}) ---",
        bytecode_file.code_section.functions.len()
    );
    for (i, func) in bytecode_file.code_section.functions.iter().enumerate() {
        println!("\n  Function {}: {}", i, func.name);
        println!("    Params: {:?}", func.params);
        println!("    Return: {:?}", func.return_type);
        println!("    Local count: {}", func.local_count);
        println!("    Instructions ({}):", func.instructions.len());

        for (j, instr) in func.instructions.iter().enumerate() {
            let opcode_value = instr.opcode;
            let opcode_name = TypedOpcode::try_from(opcode_value)
                .map(|o| format!("{:?}", o))
                .unwrap_or_else(|_| format!("Unknown(0x{:02X})", opcode_value));

            if instr.operands.is_empty() {
                println!("    [{:4}] {}", j, opcode_name);
            } else {
                let operands: Vec<String> =
                    instr.operands.iter().map(|b| format!("{}", b)).collect();
                println!("    [{:4}] {} [{}]", j, opcode_name, operands.join(", "));
            }
        }
    }

    Ok(())
}
