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
    run_with_source_name("<input>", source)
}

fn run_with_source_name(
    source_name: &str,
    source: &str,
) -> Result<()> {
    debug!("{}", t_cur_simple(MSG::DebugRunCalled));
    let mut compiler = frontend::Compiler::new();
    debug!("{}", t_cur_simple(MSG::CompilationStart));
    let module = compiler.compile_with_source(source_name, source)?;
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
    run_with_source_name(&path_str, &source)
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
    let module = compiler.compile_with_source(&source_path_str, &source)?;

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

    let path_str = path.display().to_string();
    println!("=== Bytecode Dump for {} ===\n", path_str);

    // Read source file
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Compile
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile_with_source(&path_str, &source)?;

    // Generate bytecode
    let mut ctx = CodegenContext::new(module);
    let bytecode_file: crate::middle::codegen::bytecode::BytecodeFile = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    // Dump header information
    println!("File Header:");
    println!("  Magic: 0x{:08x}", bytecode_file.header.magic);
    println!("  Version: {}", bytecode_file.header.version);
    println!("  Flags: 0x{:08x}", bytecode_file.header.flags);
    println!("  Entry Point: {}", bytecode_file.header.entry_point);
    println!("  Section Count: {}", bytecode_file.header.section_count);
    println!("  File Size: {} bytes", bytecode_file.header.file_size);
    println!();

    // Dump type table
    if !bytecode_file.type_table.is_empty() {
        println!(
            "=== Type Table ({} types) ===",
            bytecode_file.type_table.len()
        );
        for (idx, ty) in bytecode_file.type_table.iter().enumerate() {
            println!("[{:04}] {}", idx, dump_type_detail(ty));
        }
        println!();
    }

    // Dump constants
    if !bytecode_file.const_pool.is_empty() {
        println!(
            "=== Constants ({} items) ===",
            bytecode_file.const_pool.len()
        );
        for (idx, constant) in bytecode_file.const_pool.iter().enumerate() {
            println!(
                "[{:04}] {} = {:?}",
                idx,
                dump_const_detail(constant),
                constant
            );
        }
        println!();
    }

    // Dump functions
    println!(
        "=== Functions ({} functions) ===",
        bytecode_file.code_section.functions.len()
    );
    for (func_idx, func) in bytecode_file.code_section.functions.iter().enumerate() {
        println!("\nFunction #{}: {}", func_idx, func.name);
        println!("  Parameters: {}", dump_params_detail(&func.params));
        println!("  Return Type: {}", dump_type_detail(&func.return_type));
        println!("  Local Count: {}", func.local_count);
        println!("  Instructions: {}", func.instructions.len());

        // Dump instructions in a more readable format
        if !func.instructions.is_empty() {
            println!("  Code:");
            dump_instructions(&func.instructions);
        }
    }

    Ok(())
}

/// Dump type information in detail
fn dump_type_detail(ty: &crate::frontend::typecheck::MonoType) -> String {
    match ty {
        crate::frontend::typecheck::MonoType::Void => "void".to_string(),
        crate::frontend::typecheck::MonoType::Bool => "bool".to_string(),
        crate::frontend::typecheck::MonoType::Int(n) => format!("i{}", n),
        crate::frontend::typecheck::MonoType::Float(n) => format!("f{}", n),
        crate::frontend::typecheck::MonoType::Char => "char".to_string(),
        crate::frontend::typecheck::MonoType::String => "String".to_string(),
        crate::frontend::typecheck::MonoType::Bytes => "bytes".to_string(),
        crate::frontend::typecheck::MonoType::Struct(struct_type) => {
            format!("struct {:?}", struct_type)
        }
        crate::frontend::typecheck::MonoType::Enum(enum_type) => format!("enum {:?}", enum_type),
        crate::frontend::typecheck::MonoType::Tuple(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", inner)
        }
        crate::frontend::typecheck::MonoType::List(elem) => format!("[{}]", dump_type_detail(elem)),
        crate::frontend::typecheck::MonoType::Dict(key, value) => {
            format!("[{}: {}]", dump_type_detail(key), dump_type_detail(value))
        }
        crate::frontend::typecheck::MonoType::Set(elem) => {
            format!("{{{}}}", dump_type_detail(elem))
        }
        crate::frontend::typecheck::MonoType::Fn {
            params,
            return_type,
            is_async,
        } => {
            let params_str = params
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(", ");
            let ret_str = dump_type_detail(return_type);
            let async_str = if *is_async { "async " } else { "" };
            format!("{}fn({}) -> {}", async_str, params_str, ret_str)
        }
        crate::frontend::typecheck::MonoType::TypeRef(name) => name.clone(),
        crate::frontend::typecheck::MonoType::TypeVar(var) => format!("T{:?}", var),
        crate::frontend::typecheck::MonoType::Range { elem_type } => {
            format!("{}..", dump_type_detail(elem_type))
        }
        crate::frontend::typecheck::MonoType::Union(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(" | ");
            format!("({})", inner)
        }
        crate::frontend::typecheck::MonoType::Intersection(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(" & ");
            format!("({})", inner)
        }
        crate::frontend::typecheck::MonoType::Arc(inner) => {
            format!("Arc<{}>", dump_type_detail(inner))
        }
    }
}

/// Dump constant information in detail
fn dump_const_detail(constant: &crate::middle::ir::ConstValue) -> &'static str {
    match constant {
        crate::middle::ir::ConstValue::Void => "void",
        crate::middle::ir::ConstValue::Bool(_) => "bool",
        crate::middle::ir::ConstValue::Int(_) => "int",
        crate::middle::ir::ConstValue::Float(_) => "float",
        crate::middle::ir::ConstValue::Char(_) => "char",
        crate::middle::ir::ConstValue::String(_) => "String",
        crate::middle::ir::ConstValue::Bytes(_) => "bytes",
    }
}

/// Dump function parameters in detail
fn dump_params_detail(params: &[crate::frontend::typecheck::MonoType]) -> String {
    if params.is_empty() {
        "()".to_string()
    } else {
        let param_strs = params.iter().map(dump_type_detail).collect::<Vec<_>>();
        format!("({})", param_strs.join(", "))
    }
}

/// Dump instructions in a readable format with opcode names
fn dump_instructions(instructions: &[crate::middle::codegen::bytecode::BytecodeInstruction]) {
    for (instr_idx, instr) in instructions.iter().enumerate() {
        // Try to decode the opcode
        match Opcode::try_from(instr.opcode) {
            Ok(opcode) => {
                println!("    [{:04}] {:?}", instr_idx, opcode);
            }
            Err(_) => {
                println!(
                    "    [{:04}] Unknown opcode: 0x{:02x}",
                    instr_idx, instr.opcode
                );
            }
        }
    }
}
