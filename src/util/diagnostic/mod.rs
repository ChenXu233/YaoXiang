//! 统一诊断系统
//!
//! 提供错误处理、诊断渲染和源码位置跟踪
//!
//! # 模块结构
//!
//! - diagnostic - 诊断数据结构 (Diagnostic, Severity)
//! - codes - 错误码注册表
//! - emitter - 诊断输出渲染器
//! - suggest - 智能建议引擎
//! - collect - 错误收集器
//! - result - 统一 Result 类型
//!
//! # 示例
//!
//! ```ignore
//! use yaoxiang::util::diagnostic::{Diagnostic, TextEmitter};
//!
//! let emitter = TextEmitter::new();
//! let output = emitter.render(&diagnostic, &source_file);
//! println!("{}", output);
//! ```

pub mod codes;
pub mod collect;
pub mod command;
pub mod emitter;
pub mod error;
#[macro_use]
pub mod error_macro;
pub mod result;
pub mod suggest;

// 重新导出
pub use codes::{ErrorCategory, ErrorCodeDefinition, I18nRegistry, DiagnosticBuilder, ErrorInfo};
pub use collect::{ErrorCollector, Warning, ErrorFormatter};
pub use command::render_explain_output;
#[cfg(feature = "cli")]
pub use command::{run_check_command_once, run_check_watch_command};
pub use emitter::{TextEmitter, JsonEmitter, EmitterConfig};
pub use error::{Diagnostic, Severity};
pub use result::{Result, ResultExt};
pub use suggest::SuggestionEngine;

// 渲染器
use crate::util::span::{DebugSpan, SourceFile, SourceMap};
use std::collections::HashMap;

/// 单个检查诊断（包含所属文件）
#[derive(Debug, Clone)]
pub struct CheckDiagnostic {
    pub file: String,
    pub diagnostic: Diagnostic,
}

/// `yaoxiang check` 的聚合结果
#[derive(Debug, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<CheckDiagnostic>,
    pub source_files: HashMap<String, SourceFile>,
    pub error_count: usize,
    pub warning_count: usize,
}

/// 渲染编译错误
///
/// 从错误消息解析并渲染为 Rust 风格的诊断输出
pub fn render_compile_error(
    error: &str,
    source_file: &SourceFile,
    diagnostic: Option<&Diagnostic>,
) -> String {
    let emitter = TextEmitter::new();

    // 如果有诊断信息，使用它；否则从消息解析
    let diagnostic = match diagnostic {
        Some(d) => d.clone(),
        None => parse_compile_error(error),
    };

    emitter.render_with_source(&diagnostic, Some(source_file))
}

/// 解析编译错误为诊断信息（通过注册表路径）
pub fn parse_compile_error(error: &str) -> Diagnostic {
    ErrorCodeDefinition::internal_error(error).build()
}

/// 渲染运行时错误（带源码高亮）
pub fn render_runtime_error(
    error: &crate::backends::ExecutorError,
    module: &crate::middle::bytecode::BytecodeModule,
    sources: Option<&SourceMap>,
) -> String {
    let emitter = TextEmitter::new();

    let primary_span = error
        .stack_trace()
        .and_then(|stack| stack.first())
        .and_then(|frame| resolve_runtime_span(module, frame))
        .filter(|span| !span.is_dummy());

    let primary_source = primary_span.and_then(|ds| sources.and_then(|sm| sm.get(ds.file_id)));
    let diagnostic = build_runtime_diagnostic(error, primary_span, primary_source);

    let mut output = emitter.render_with_source(&diagnostic, primary_source);
    let stack_text = format_runtime_stack_trace(error, module, sources);
    if !stack_text.is_empty() {
        output.push('\n');
        output.push_str(&stack_text);
    }

    output
}

fn resolve_runtime_span(
    module: &crate::middle::bytecode::BytecodeModule,
    frame: &crate::backends::StackFrame,
) -> Option<DebugSpan> {
    module
        .functions
        .iter()
        .find(|f| f.name == frame.function_name)
        .and_then(|f| f.debug_map.get(&frame.ip).copied())
}

fn build_runtime_diagnostic(
    error: &crate::backends::ExecutorError,
    primary_span: Option<DebugSpan>,
    source_file: Option<&SourceFile>,
) -> Diagnostic {
    use crate::backends::ExecutorError;

    let mut builder = match error {
        ExecutorError::FunctionNotFound(name, _) => {
            ErrorCodeDefinition::runtime_function_not_found(name.as_str())
        }
        ExecutorError::DivisionByZero(_) => {
            let expr = primary_span
                .and_then(|ds| {
                    source_file
                        .and_then(|sf| sf.source_text(ds.span))
                        .map(|s| s.trim())
                })
                .filter(|s| !s.is_empty())
                .unwrap_or("<unknown>");
            ErrorCodeDefinition::division_by_zero(expr)
        }
        ExecutorError::Runtime(message, _) => ErrorCodeDefinition::runtime_error(message.as_str()),
        ExecutorError::Type(message, _) => ErrorCodeDefinition::runtime_error(message.as_str()),
        ExecutorError::StackOverflow(_) => ErrorCodeDefinition::stack_overflow(0),
        other => ErrorCodeDefinition::runtime_error(&other.to_string()),
    };

    if let Some(span) = primary_span {
        builder = builder.at(span.span);
    }

    builder.build()
}

fn format_runtime_stack_trace(
    error: &crate::backends::ExecutorError,
    module: &crate::middle::bytecode::BytecodeModule,
    sources: Option<&SourceMap>,
) -> String {
    let Some(stack) = error.stack_trace() else {
        return String::new();
    };

    let mut out = String::from("stack trace:\n");
    for frame in stack {
        if let Some(ds) = resolve_runtime_span(module, frame).filter(|s| !s.is_dummy()) {
            let loc = match sources.and_then(|sm| sm.get(ds.file_id)) {
                Some(sf) => format!(
                    "{}:{}:{}",
                    sf.name, ds.span.start.line, ds.span.start.column
                ),
                None => format!("{}:{}", ds.span.start.line, ds.span.start.column),
            };
            out.push_str(&format!(
                "  at {} ({}) (ip: {})\n",
                frame.function_name, loc, frame.ip
            ));
        } else {
            out.push_str(&format!(
                "  at {} (ip: {})\n",
                frame.function_name, frame.ip
            ));
        }
    }
    out
}

/// 运行文件并美化错误输出
///
/// # 参数
/// - `file`: 源文件路径
///
/// # 返回
/// 成功返回 `()`，失败返回错误
#[cfg(feature = "cli")]
pub fn run_file_with_diagnostics(
    file: &std::path::PathBuf,
    debug_info: bool,
    runtime_mode: &str,
    workers: usize,
) -> anyhow::Result<()> {
    use crate::frontend::Compiler;
    use crate::middle::passes::codegen::CodegenContext;
    use crate::Executor;
    use crate::Interpreter;

    // 检测 .42 字节码文件，跳过编译直接执行
    if file.extension().map(|e| e == "42").unwrap_or(false) {
        let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(file)
            .map_err(|e| anyhow::anyhow!("Failed to load bytecode file: {}", e))?;
        let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

        let mut interp = crate::backends::interpreter::Interpreter::new();
        let rt_mode = match runtime_mode {
            "standard" => crate::backends::runtime::RuntimeMode::Standard,
            "full" => crate::backends::runtime::RuntimeMode::Full,
            _ => crate::backends::runtime::RuntimeMode::Embedded,
        };
        let effective_workers = if workers > 0 {
            workers
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        };
        interp.set_runtime_config(
            crate::backends::interpreter::runtime::InterpreterRuntimeConfig {
                runtime: rt_mode,
                workers: effective_workers,
                work_stealing: false,
            },
        );
        let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
        if let Err(e) = executor.execute_module(&bytecode_module) {
            eprintln!();
            // 字节码加载模式下无 SourceMap，传入 None
            let output = render_runtime_error(&e, &bytecode_module, None);
            eprintln!("{}", output);
            return Err(anyhow::anyhow!("Runtime error"));
        }
        return Ok(());
    }

    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to read file {}: {}",
                file.display(),
                e
            ));
        }
    };

    let source_name = file.display().to_string();
    let mut sources = SourceMap::new();
    let entry_file_id = sources.add_file(source_name, source);
    let source_file = sources
        .get(entry_file_id)
        .ok_or_else(|| anyhow::anyhow!("Failed to load source file"))?;

    let mut compiler = Compiler::new();
    match compiler.compile(&source_file.name, &source_file.content) {
        Ok(module) => {
            // Generate bytecode
            let mut ctx = CodegenContext::new(module);
            ctx.set_generate_debug_info(debug_info);
            let bytecode_file = ctx
                .generate()
                .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
            let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

            // Execute
            let mut interp = Interpreter::new();
            let rt_mode = match runtime_mode {
                "standard" => crate::backends::runtime::RuntimeMode::Standard,
                "full" => crate::backends::runtime::RuntimeMode::Full,
                _ => crate::backends::runtime::RuntimeMode::Embedded,
            };
            let effective_workers = if workers > 0 {
                workers
            } else {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4)
            };
            interp.set_runtime_config(
                crate::backends::interpreter::runtime::InterpreterRuntimeConfig {
                    runtime: rt_mode,
                    workers: effective_workers,
                    work_stealing: false,
                },
            );
            let mut executor: Box<dyn Executor> = Box::new(interp);
            if let Err(e) = executor.execute_module(&bytecode_module) {
                eprintln!();
                let output = render_runtime_error(&e, &bytecode_module, Some(&sources));
                eprintln!("{}", output);
                return Err(anyhow::anyhow!("Runtime error"));
            }
        }
        Err(e) => {
            // 使用渲染器输出美化后的错误
            eprintln!();
            let output = render_compile_error(e.message(), source_file, e.diagnostic());
            eprintln!("{}", output);
            return Err(anyhow::anyhow!("Compilation failed"));
        }
    }

    Ok(())
}

/// 只进行类型检查，不执行代码
///
/// # 参数
/// - `file`: 源文件路径
///
/// # 返回
/// 检查成功返回 `()`，失败返回错误
#[cfg(feature = "cli")]
pub fn check_file_with_diagnostics(file: &std::path::PathBuf) -> anyhow::Result<()> {
    let result = check_files_with_diagnostics(std::slice::from_ref(file))?;
    if result.error_count > 0 {
        return Err(anyhow::anyhow!("Type check failed"));
    }

    println!("Type check passed for {}", file.display());
    Ok(())
}

/// 对多个文件进行静态检查并聚合诊断信息
///
/// 按顺序逐文件检查，不进行依赖分析。
#[cfg(feature = "cli")]
pub fn check_files_with_diagnostics(files: &[std::path::PathBuf]) -> anyhow::Result<CheckResult> {
    let mut result = CheckResult::default();

    for file in files {
        let source = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file.display(), e))?;
        let source_file = SourceFile::new(file.display().to_string(), source.clone());
        result
            .source_files
            .insert(file.display().to_string(), source_file);

        let mut compiler = crate::frontend::Compiler::new();
        match compiler.compile_with_source(&file.display().to_string(), &source) {
            Ok(_) => {}
            Err(e) if e.is_type_error() => {
                result.diagnostics.push(CheckDiagnostic {
                    file: file.display().to_string(),
                    diagnostic: crate::util::diagnostic::ErrorCodeDefinition::internal_error(
                        &format!("{}", e),
                    )
                    .build(),
                });
                result.error_count += 1;
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                return Err(anyhow::anyhow!(err_msg));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests;
