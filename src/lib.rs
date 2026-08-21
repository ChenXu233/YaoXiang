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
//! - `cli`: CLI-only dependencies (REPL, LSP, hot-reload)

#![doc(html_root_url = "https://docs.rs/yaoxiang")]
#![warn(rust_2018_idioms)]
// ponytail: RuntimeValue 含 Arc<Mutex> 但作为 dict key 是有意设计（按值 Hash/Eq），
// clippy 的 mutable_key_type 对本 crate 全是误报。
#![allow(clippy::mutable_key_type)]

// Public modules
pub mod backends;
pub mod formatter;
pub mod frontend;
#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;
pub mod middle;
#[cfg(not(target_arch = "wasm32"))]
pub mod package;
#[cfg(not(target_arch = "wasm32"))]
pub mod repl;
pub mod std;

pub mod util;

// Re-exports
pub use anyhow::{Context, Result};
pub use thiserror::Error;

// Backend re-exports
pub use backends::{Executor, DebuggableExecutor, ExecutorError, ExecutorResult, ExecutorConfig};
pub use backends::common::{RuntimeValue, Heap, Handle};
pub use backends::interpreter::Interpreter;
#[cfg(not(target_arch = "wasm32"))]
pub use repl::Repl;

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
/// Evaluate YaoXiang code (eval mode: auto-wrap if no main function)
///
/// Unlike `run()`, this function:
/// - Checks if the code has a top-level `main =` binding
/// - If yes: compiles and executes as-is
/// - If no: wraps the code in `main = { ... }` automatically
pub fn eval_code(source: &str) -> Result<()> {
    let tokens = crate::frontend::core::tokenize(source)
        .map_err(|e| anyhow::anyhow!("Lexer error: {:?}", e))?;
    let parse_result = crate::frontend::core::parser::parse(&tokens);
    let has_main = parse_result.module.items.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            crate::frontend::core::parser::ast::StmtKind::Assign { target, .. }
            if matches!(target.as_ref(), crate::frontend::core::parser::ast::Expr::Var(name, _) if name == "main")
        )
    });
    let compile_source: String = if has_main {
        source.to_string()
    } else {
        format!("main = {{\n{}}}", source)
    };
    run_with_source_name("<eval>", &compile_source)
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

#[cfg(not(target_arch = "wasm32"))]
use ::std::fs;
#[cfg(not(target_arch = "wasm32"))]
use ::std::path::Path;

/// Run the interpreter on a file
#[cfg(not(target_arch = "wasm32"))]
pub fn run_file(path: &Path) -> Result<()> {
    let path_str = path.display().to_string();
    debug!("{}", t_cur(MSG::RunFile, Some(&[&path_str])));
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    debug!("{}", t_cur(MSG::ReadingFile, Some(&[&path_str])));
    run_with_source_name(&path_str, &source)
}

/// RFC-029: 以项目方式运行（多文件编排）。
///
/// 从入口文件发现同目录所有 `.yx` 文件，构建共享 Registry，逐文件编译并合并 IR 后执行。
/// 入口文件的 `main` 函数为程序入口。
#[cfg(not(target_arch = "wasm32"))]
pub fn run_project(entry: &Path) -> Result<()> {
    let module = frontend::module::orchestrator::compile_project(entry)
        .map_err(|e| anyhow::anyhow!("Project compilation failed: {}", e))?;
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);
    let mut interpreter = backends::interpreter::Interpreter::new();
    interpreter.execute_module(&bytecode_module)?;
    Ok(())
}

/// Build bytecode file (.42)
#[cfg(not(target_arch = "wasm32"))]
pub fn build_bytecode(
    source_path: &Path,
    output_path: &Path,
) -> Result<()> {
    build_bytecode_with_options(source_path, output_path, false)
}

/// Build bytecode file (.42) with options
#[cfg(not(target_arch = "wasm32"))]
pub fn build_bytecode_with_options(
    source_path: &Path,
    output_path: &Path,
    debug_info: bool,
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
    ctx.set_generate_debug_info(debug_info);
    let mut bytecode_file = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;

    if debug_info {
        let mut sources = crate::util::span::SourceMap::new();
        sources.add_file(source_path_str.clone(), source.clone());
        bytecode_file.debug_section = Some(
            crate::middle::passes::codegen::bytecode::DebugSection::from_sources_and_functions(
                sources,
                &bytecode_file.code_section.functions,
            ),
        );
    }

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
#[cfg(not(target_arch = "wasm32"))]
pub fn dump_bytecode(path: &Path) -> Result<()> {
    use crate::middle::passes::codegen::bytecode::BytecodeFile;
    use crate::middle::passes::codegen::CodegenContext;

    let path_str = path.display().to_string();
    tracing::info!("{}", t_cur(MSG::BytecodeDumpHeader, Some(&[&path_str])));
    tracing::info!("");

    // 内容即身份：字节码文件直接加载 dump，源码文件编译后 dump（与 run 的分流一致）
    if BytecodeFile::probe(path).unwrap_or(false) {
        let bytecode_file = BytecodeFile::load(path)
            .with_context(|| format!("Failed to load bytecode file: {}", path.display()))?;
        dump_bytecode_file(&bytecode_file);
        return Ok(());
    }

    // Read source file
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    // Compile
    let mut compiler = frontend::Compiler::new();
    let module = compiler.compile_with_source(&path_str, &source)?;

    // Generate bytecode
    let mut ctx = CodegenContext::new(module);
    let bytecode_file: BytecodeFile = ctx
        .generate()
        .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
    dump_bytecode_file(&bytecode_file);

    Ok(())
}

/// 打印 BytecodeFile 的完整结构（header / type table / constants / functions）
fn dump_bytecode_file(bytecode_file: &crate::middle::passes::codegen::bytecode::BytecodeFile) {
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeMagic,
            Some(&[&format!("{:08x}", bytecode_file.header.magic)])
        )
    );
    tracing::info!(
        "{}",
        t_cur(MSG::BytecodeVersion, Some(&[&bytecode_file.header.version]))
    );
    tracing::info!(
        "{}",
        t_cur(
            MSG::BytecodeFlags,
            Some(&[&format!("{:08x}", bytecode_file.header.flags)])
        )
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
}

/// Dump type information in detail
fn dump_type_detail(ty: &crate::frontend::core::typecheck::MonoType) -> String {
    match ty {
        crate::frontend::core::typecheck::MonoType::Void => "void".to_string(),
        crate::frontend::core::typecheck::MonoType::Never => "never".to_string(),
        crate::frontend::core::typecheck::MonoType::Bool => "bool".to_string(),
        crate::frontend::core::typecheck::MonoType::Int(n) => format!("i{}", n),
        crate::frontend::core::typecheck::MonoType::Float(n) => format!("f{}", n),
        crate::frontend::core::typecheck::MonoType::Char => "char".to_string(),
        crate::frontend::core::typecheck::MonoType::String => "String".to_string(),
        crate::frontend::core::typecheck::MonoType::Bytes => "bytes".to_string(),
        crate::frontend::core::typecheck::MonoType::Struct(struct_type) => {
            format!("struct {:?}", struct_type)
        }
        crate::frontend::core::typecheck::MonoType::Enum(enum_type) => {
            format!("enum {:?}", enum_type)
        }
        crate::frontend::core::typecheck::MonoType::Tuple(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", inner)
        }
        crate::frontend::core::typecheck::MonoType::List(elem) => {
            format!("List({})", dump_type_detail(elem))
        }
        crate::frontend::core::typecheck::MonoType::Dict(key, value) => {
            format!(
                "Dict({}, {})",
                dump_type_detail(key),
                dump_type_detail(value)
            )
        }
        crate::frontend::core::typecheck::MonoType::Set(elem) => {
            format!("{{{}}}", dump_type_detail(elem))
        }
        crate::frontend::core::typecheck::MonoType::Option(inner) => {
            format!("Option({})", dump_type_detail(inner))
        }
        crate::frontend::core::typecheck::MonoType::Result(ok, err) => {
            format!(
                "Result({}, {})",
                dump_type_detail(ok),
                dump_type_detail(err)
            )
        }
        crate::frontend::core::typecheck::MonoType::Fn {
            params,
            return_type,
        } => {
            let params_str = params
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(", ");
            let ret_str = dump_type_detail(return_type);
            format!("fn({}) -> {}", params_str, ret_str)
        }
        crate::frontend::core::typecheck::MonoType::TypeRef(name) => name.clone(),
        crate::frontend::core::typecheck::MonoType::TypeVar(var) => format!("T{:?}", var),
        crate::frontend::core::typecheck::MonoType::Range { elem_type } => {
            format!("{}..", dump_type_detail(elem_type))
        }
        crate::frontend::core::typecheck::MonoType::Union(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(" | ");
            format!("({})", inner)
        }
        crate::frontend::core::typecheck::MonoType::Intersection(types) => {
            let inner = types
                .iter()
                .map(dump_type_detail)
                .collect::<Vec<_>>()
                .join(" & ");
            format!("({})", inner)
        }
        crate::frontend::core::typecheck::MonoType::Arc(inner) => {
            format!("Arc({})", dump_type_detail(inner))
        }
        crate::frontend::core::typecheck::MonoType::Weak(inner) => {
            format!("Weak({})", dump_type_detail(inner))
        }
        crate::frontend::core::typecheck::MonoType::Ref { mutable, inner } => {
            if *mutable {
                format!("&mut {}", dump_type_detail(inner))
            } else {
                format!("&{}", dump_type_detail(inner))
            }
        }
        crate::frontend::core::typecheck::MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => {
            let args_str = if assoc_args.is_empty() {
                String::new()
            } else {
                format!(
                    "({})",
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
        crate::frontend::core::typecheck::MonoType::Literal {
            name: _,
            base_type,
            value,
        } => {
            format!("{}::{}", dump_type_detail(base_type), value)
        }
        crate::frontend::core::typecheck::MonoType::MetaType {
            universe_level,
            type_params,
        } => {
            if type_params.is_empty() {
                format!("Type{}", universe_level)
            } else {
                let params_str: Vec<String> = type_params.iter().map(|p| p.type_name()).collect();
                format!("Type{}({})", universe_level, params_str.join(", "))
            }
        }
        crate::frontend::core::typecheck::MonoType::Generic { name, args } => {
            let args_str: Vec<String> = args.iter().map(dump_type_detail).collect();
            format!("{}({})", name, args_str.join(", "))
        }
        crate::frontend::core::typecheck::MonoType::Refined { base, constraint } => {
            format!("{} {{{}}}", dump_type_detail(base), constraint)
        }
        crate::frontend::core::typecheck::MonoType::DepFn {
            params,
            return_type,
        } => {
            let params_str: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name, dump_type_detail(&p.ty)))
                .collect();
            format!(
                "({}) -> {}",
                params_str.join(", "),
                dump_type_detail(return_type)
            )
        }
        crate::frontend::core::typecheck::MonoType::LibraryRef { .. }
        | crate::frontend::core::typecheck::MonoType::ExternRef { .. } => todo!(),
    }
}

fn dump_const_detail(constant: &crate::middle::core::ir::ConstValue) -> &'static str {
    match constant {
        crate::middle::core::ir::ConstValue::Void => "void",
        crate::middle::core::ir::ConstValue::Bool(_) => "bool",
        crate::middle::core::ir::ConstValue::Int(_) => "int",
        crate::middle::core::ir::ConstValue::Float(_) => "float",
        crate::middle::core::ir::ConstValue::Char(_) => "char",
        crate::middle::core::ir::ConstValue::String(_) => "String",
        crate::middle::core::ir::ConstValue::Bytes(_) => "bytes",
        crate::middle::core::ir::ConstValue::LibraryRef { .. }
        | crate::middle::core::ir::ConstValue::ExternRef { .. } => todo!(),
    }
}

/// Dump function parameters in detail
fn dump_params_detail(params: &[crate::frontend::core::typecheck::MonoType]) -> String {
    if params.is_empty() {
        "()".to_string()
    } else {
        let param_strs = params.iter().map(dump_type_detail).collect::<Vec<_>>();
        format!("({})", param_strs.join(", "))
    }
}

/// Format operands as hex string for display
fn format_operands(operands: &[u8]) -> String {
    if operands.is_empty() {
        String::new()
    } else {
        operands
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Dump instructions in a readable format with opcode names
fn dump_instructions(
    instructions: &[crate::middle::passes::codegen::bytecode::BytecodeInstruction]
) {
    for (instr_idx, instr) in instructions.iter().enumerate() {
        // Try to decode the opcode
        let ops_str = format_operands(&instr.operands);
        let name = crate::backends::common::opcode_name(instr.opcode);
        if name != "Unknown" {
            tracing::info!(
                "{}",
                t_cur(
                    MSG::BytecodeInstrIndex,
                    Some(&[
                        &format!("{:04}", instr_idx),
                        &format!("{:<14}", name),
                        &ops_str
                    ])
                )
            );
        } else {
            tracing::info!(
                "{}",
                t_cur(
                    MSG::BytecodeUnknownOpcode,
                    Some(&[
                        &format!("{:04}", instr_idx),
                        &format!("{:02x}", instr.opcode),
                        &ops_str
                    ])
                )
            );
        }
    }
}

// FFI End-to-End Tests

#[cfg(test)]
mod tests;
