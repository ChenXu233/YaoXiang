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

// Public modules
pub mod backends;
pub mod frontend;
pub mod middle;
pub mod package;
pub mod std;

pub mod util;

// Re-exports
pub use anyhow::{Context, Result};
pub use thiserror::Error;

// Backend re-exports
pub use backends::{Executor, DebuggableExecutor, ExecutorError, ExecutorResult, ExecutorConfig};
pub use backends::common::{RuntimeValue, Opcode, Heap, Handle, BumpAllocator};
pub use backends::interpreter::Interpreter;
pub use backends::dev::{DevShell, Debugger, REPL, TuiREPL};

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
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
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
    use crate::middle::passes::codegen::CodegenContext;

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
    use crate::middle::passes::codegen::CodegenContext;

    let path_str = path.display().to_string();
    tracing::info!("{}", t_cur(MSG::BytecodeDumpHeader, Some(&[&path_str])));
    tracing::info!("");

    // Read source file
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Compile
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile_with_source(&path_str, &source)?;

    // Generate bytecode
    let mut ctx = CodegenContext::new(module);
    let bytecode_file: crate::middle::passes::codegen::bytecode::BytecodeFile = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    // Dump header information
    tracing::info!("{}", t_cur_simple(MSG::BytecodeFileHeader));
    tracing::info!(
        "{}",
        t_cur(MSG::BytecodeMagic, Some(&[&bytecode_file.header.magic]))
    );
    tracing::info!(
        "{}",
        t_cur(MSG::BytecodeVersion, Some(&[&bytecode_file.header.version]))
    );
    tracing::info!(
        "{}",
        t_cur(MSG::BytecodeFlags, Some(&[&bytecode_file.header.flags]))
    );
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeEntryPoint,
            Some(&[&bytecode_file.header.entry_point])
        )
    );
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeSectionCount,
            Some(&[&bytecode_file.header.section_count])
        )
    );
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeFileSize,
            Some(&[&bytecode_file.header.file_size])
        )
    );
    tracing::info!("");

    // Dump type table
    if !bytecode_file.type_table.is_empty() {
        tracing::info!(
            "{}",
            t_cur(
                MSG::BytecodeDumpTypeTable,
                Some(&[&bytecode_file.type_table.len()])
            )
        );
        for (idx, ty) in bytecode_file.type_table.iter().enumerate() {
            tracing::info!("[{:04}] {}", idx, dump_type_detail(ty));
        }
        tracing::info!("");
    }

    // Dump constants
    if !bytecode_file.const_pool.is_empty() {
        tracing::info!(
            "{}",
            t_cur(
                MSG::BytecodeDumpConstants,
                Some(&[&bytecode_file.const_pool.len()])
            )
        );
        for (idx, constant) in bytecode_file.const_pool.iter().enumerate() {
            tracing::info!(
                "[{:04}] {} = {:?}",
                idx,
                dump_const_detail(constant),
                constant
            );
        }
        tracing::info!("");
    }

    // Dump functions
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeDumpFunctions,
            Some(&[&bytecode_file.code_section.functions.len()])
        )
    );
    for (func_idx, func) in bytecode_file.code_section.functions.iter().enumerate() {
        tracing::info!("");
        tracing::info!("Function #{}: {}", func_idx, func.name);
        tracing::info!(
            "{}",
            t_cur(
                MSG::BytecodeFuncParams,
                Some(&[&dump_params_detail(&func.params)])
            )
        );
        tracing::info!(
            "{}",
            t_cur(
                MSG::BytecodeFuncReturnType,
                Some(&[&dump_type_detail(&func.return_type)])
            )
        );
        tracing::info!(
            "{}",
            t_cur(MSG::BytecodeFuncLocalCount, Some(&[&func.local_count]))
        );
        tracing::info!(
            "{}",
            t_cur(
                MSG::BytecodeFuncInstrCount,
                Some(&[&func.instructions.len()])
            )
        );

        // Dump instructions in a more readable format
        if !func.instructions.is_empty() {
            tracing::info!("{}", t_cur_simple(MSG::BytecodeFuncCode));
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
        crate::frontend::typecheck::MonoType::Weak(inner) => {
            format!("Weak<{}>", dump_type_detail(inner))
        }
        crate::frontend::typecheck::MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => {
            let args_str = if assoc_args.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    assoc_args
                        .iter()
                        .map(dump_type_detail)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            format!(
                "{}::{}{}",
                dump_type_detail(host_type),
                assoc_name,
                args_str
            )
        }
        crate::frontend::typecheck::MonoType::Literal {
            name: _,
            base_type,
            value,
        } => {
            format!("{}::{}", dump_type_detail(base_type), value)
        }
        crate::frontend::typecheck::MonoType::MetaType {
            universe_level,
            type_params,
        } => {
            if type_params.is_empty() {
                format!("Type{}", universe_level)
            } else {
                let params_str: Vec<String> = type_params.iter().map(|p| p.type_name()).collect();
                format!("Type{}<{}>", universe_level, params_str.join(", "))
            }
        }
    }
}

/// Dump constant information in detail
fn dump_const_detail(constant: &crate::middle::core::ir::ConstValue) -> &'static str {
    match constant {
        crate::middle::core::ir::ConstValue::Void => "void",
        crate::middle::core::ir::ConstValue::Bool(_) => "bool",
        crate::middle::core::ir::ConstValue::Int(_) => "int",
        crate::middle::core::ir::ConstValue::Float(_) => "float",
        crate::middle::core::ir::ConstValue::Char(_) => "char",
        crate::middle::core::ir::ConstValue::String(_) => "String",
        crate::middle::core::ir::ConstValue::Bytes(_) => "bytes",
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
fn dump_instructions(
    instructions: &[crate::middle::passes::codegen::bytecode::BytecodeInstruction]
) {
    for (instr_idx, instr) in instructions.iter().enumerate() {
        // Try to decode the opcode
        match Opcode::try_from(instr.opcode) {
            Ok(opcode) => {
                tracing::info!(
                    "{}",
                    t_cur(MSG::BytecodeInstrIndex, Some(&[&instr_idx, &opcode]))
                );
            }
            Err(_) => {
                tracing::info!(
                    "{}",
                    t_cur(
                        MSG::BytecodeUnknownOpcode,
                        Some(&[&instr_idx, &instr.opcode])
                    )
                );
            }
        }
    }
}

// =============================================================================
// FFI End-to-End Tests
// =============================================================================

#[cfg(test)]
mod ffi_tests {
    use super::*;
    use crate::backends::common::RuntimeValue;
    use crate::backends::interpreter::ffi::FfiRegistry;

    /// Test compiling and running YaoXiang source with std.io functions
    #[test]
    fn test_e2e_std_io_compile_and_run() {
        // This test verifies the complete flow:
        // 1. Parse YaoXiang source with std.io calls
        // 2. Compile to bytecode (including CallNative generation)
        // 3. Execute with FFI registry

        // Test 1: Verify std.io functions are registered in FFI registry
        let registry = FfiRegistry::with_std();
        assert!(registry.has("std.io.println"));
        assert!(registry.has("std.io.print"));
        assert!(registry.has("std.io.read_file"));
        assert!(registry.has("std.io.write_file"));
        assert!(registry.has("std.io.append_file"));

        // Test 2: Short names are NOT registered - users must use `use std.io`
        assert!(!registry.has("println"));
        assert!(!registry.has("print"));

        // Test 3: Verify FFI call works
        let result = registry.call(
            "std.io.println",
            &[RuntimeValue::String("FFI test message".into())],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RuntimeValue::Unit);

        // Test 4: Verify file operations
        let test_path = "test_std_io_e2e.txt";
        let write_result = registry.call(
            "std.io.write_file",
            &[
                RuntimeValue::String(test_path.into()),
                RuntimeValue::String("Hello, YaoXiang!".into()),
            ],
        );
        assert!(write_result.is_ok());

        let read_result = registry.call(
            "std.io.read_file",
            &[RuntimeValue::String(test_path.into())],
        );
        assert!(read_result.is_ok());
        if let RuntimeValue::String(content) = read_result.unwrap() {
            assert_eq!(content.to_string(), "Hello, YaoXiang!");
        } else {
            panic!("Expected String");
        }

        // Cleanup
        let _ = fs::remove_file(test_path);
    }

    /// Test user-defined native function binding flow
    #[test]
    fn test_e2e_user_native_function() {
        // Test user can register custom native functions
        let mut registry = FfiRegistry::new();

        // Register a custom function
        registry.register("my_add", |args| {
            let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            Ok(RuntimeValue::Int(a + b))
        });

        // Verify registration
        assert!(registry.has("my_add"));

        // Call the function
        let result = registry.call("my_add", &[RuntimeValue::Int(10), RuntimeValue::Int(32)]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RuntimeValue::Int(42));
    }

    /// Test native function not found error
    #[test]
    fn test_e2e_nonexistent_native_function() {
        let registry = FfiRegistry::new();
        let result = registry.call("nonexistent.function", &[]);
        assert!(result.is_err());
    }
}
