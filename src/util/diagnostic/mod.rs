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
pub use codes::builder::{current_span, push_current_span, SpanGuard};
pub use collect::ErrorCollector;
pub use command::render_explain_output;
#[cfg(feature = "cli")]
pub use command::run_check_command_once;
pub use emitter::{TextEmitter, JsonEmitter, EmitterConfig};
pub use error::{Diagnostic, Severity};
pub use result::Result;
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
    _source_file: Option<&SourceFile>,
) -> Diagnostic {
    use crate::backends::ExecutorError;

    let mut builder = match error {
        ExecutorError::FunctionNotFound(name, _) => {
            ErrorCodeDefinition::runtime_function_not_found(name.as_str())
        }
        ExecutorError::DivisionByZero(expr, _) => {
            // #282：表达式由变体携带（原从 primary_span 提取，失败时 <unknown>）
            let expr = if expr.is_empty() {
                "<unknown>"
            } else {
                expr.as_str()
            };
            ErrorCodeDefinition::division_by_zero(expr)
        }
        ExecutorError::Runtime(message, _) => ErrorCodeDefinition::runtime_error(message.as_str()),
        ExecutorError::Type(message, _) => ErrorCodeDefinition::runtime_error(message.as_str()),
        ExecutorError::StackOverflow(_) => ErrorCodeDefinition::stack_overflow(0),
        // #280：专用变体映射专用码，不再落通用 E6007
        ExecutorError::IndexOutOfBounds { max, index, .. } => {
            ErrorCodeDefinition::runtime_index_out_of_bounds(*max, *index)
        }
        // #299 §4: 键缺失映射专用码 E6008
        ExecutorError::KeyNotFound { key, .. } => ErrorCodeDefinition::key_not_found(key.as_str()),
        ExecutorError::AssertionFailed(msg, _) => {
            ErrorCodeDefinition::assertion_failed(msg.as_str())
        }
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
        // #327：用户面向输出不暴露字节码 ip（形态对齐编译期；字节码定位归 RFC-034 工具链）
        if let Some(ds) = resolve_runtime_span(module, frame).filter(|s| !s.is_dummy()) {
            let loc = match sources.and_then(|sm| sm.get(ds.file_id)) {
                Some(sf) => format!(
                    "{}:{}:{}",
                    sf.name, ds.span.start.line, ds.span.start.column
                ),
                None => format!("{}:{}", ds.span.start.line, ds.span.start.column),
            };
            out.push_str(&format!("  at {} ({})\n", frame.function_name, loc));
        } else {
            out.push_str(&format!("  at {}\n", frame.function_name));
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
/// RFC-029: 判断文件是否位于一个 yaoxiang 项目内（沿目录向上查找 yaoxiang.toml）。
fn in_yaoxiang_project(file: &std::path::Path) -> bool {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.join("yaoxiang.toml").exists() {
            return true;
        }
        dir = d.parent();
    }
    false
}

pub fn run_file_with_diagnostics(
    file: &std::path::PathBuf,
    runtime_mode: &str,
    workers: usize,
) -> anyhow::Result<()> {
    use crate::frontend::Compiler;
    use crate::middle::passes::codegen::CodegenContext;

    // 通过文件头魔数判定字节码 vs 源码
    // 内容即身份：不依赖文件名，cat dist.42 > out.bin 也能执行
    // probe I/O 错误退化为源码路径（read_to_string 产生更好的错误信息）
    if crate::middle::passes::codegen::BytecodeFile::probe(file).unwrap_or(false) {
        let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(file)
            .map_err(|e| anyhow::anyhow!("Failed to load bytecode file: {}", e))?;
        let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

        // #327：贯通 debug_section.sources——带 --debug-info 构建的 .42 直跑时
        // 也能渲染栈帧源码上下文；无 DebugSection 时为 None，行为同前
        let debug_sources = bytecode_module.debug_sources.as_ref();
        return exec_module(&bytecode_module, runtime_mode, workers, debug_sources);
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

    // RFC-029: 位于 yaoxiang 项目内的文件走多文件编排器（发现同项目 .yx、构建共享
    // Registry、逐文件 typecheck、整体编译）；否则按单文件编译。两者都产出 ModuleIR，
    // 复用下方同一套 codegen + 执行路径（保留 runtime/workers 配置）。
    let module_result: Result<crate::middle::ModuleIR, crate::frontend::CompileError> =
        if in_yaoxiang_project(file) {
            match crate::frontend::module::orchestrator::compile_project(file) {
                Ok(m) => Ok(m),
                Err(e) => {
                    // 结构化诊断按其来源文件渲染，不再包成 E8001
                    render_orchestrator_error(&e);
                    return Err(anyhow::anyhow!("Compilation failed"));
                }
            }
        } else {
            Compiler::new().compile(&source_file.name, &source_file.content)
        };

    match module_result {
        Ok(module) => {
            // 多文件模式（#252）：按 orchestrator 发现顺序重建 SourceMap，
            // 使索引与 debug span 的 file_id 对齐。读不到文件也要占位，保持索引稳定。
            if !module.source_files.is_empty() {
                let mut multi = SourceMap::new();
                for path in &module.source_files {
                    // #327：嵌入 std 虚拟路径（<std/test>）读盘必失败，回填嵌入源文本，
                    // 使错误落在 std 模块内时也能渲染真实片段
                    let content =
                        match crate::std::yx_sources::embedded_source_by_virtual_path(path) {
                            Some(src) => src.to_string(),
                            None => std::fs::read_to_string(path).unwrap_or_default(),
                        };
                    multi.add_file(path.clone(), content);
                }
                sources = multi;
            }
            // Generate bytecode
            // #327：debug_map 默认生成——运行时错误默认携带栈帧与源码上下文，
            // 不再有"无位置"的运行时错误形态
            let mut ctx = CodegenContext::new(module);
            ctx.set_generate_debug_info(true);
            let bytecode_file = ctx
                .generate()
                .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
            let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

            // Execute
            exec_module(&bytecode_module, runtime_mode, workers, Some(&sources))?;
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

/// 解析运行时模式字符串（"full" 历史别名等价 Standard，work-stealing 从未实现）
fn parse_runtime_mode(mode: &str) -> crate::backends::runtime::RuntimeMode {
    match mode {
        "standard" | "full" => crate::backends::runtime::RuntimeMode::Standard,
        _ => crate::backends::runtime::RuntimeMode::Embedded,
    }
}

/// workers=0 时自动检测 CPU 核心数
fn effective_workers(workers: usize) -> usize {
    if workers > 0 {
        workers
    } else {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }
}

/// 执行字节码模块（配置运行时 + 渲染运行时错误）
fn exec_module(
    module: &crate::middle::bytecode::BytecodeModule,
    runtime_mode: &str,
    workers: usize,
    sources: Option<&SourceMap>,
) -> anyhow::Result<()> {
    let mut interp = crate::backends::interpreter::Interpreter::new();
    interp.set_runtime_config(crate::backends::runtime::RuntimeConfig {
        mode: parse_runtime_mode(runtime_mode),
        workers: effective_workers(workers),
    });
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    if let Err(e) = executor.execute_module(module) {
        eprintln!();
        let output = render_runtime_error(&e, module, sources);
        eprintln!("{}", output);
        return Err(anyhow::anyhow!("Runtime error"));
    }
    Ok(())
}

/// 渲染编排器错误：TypeCheck/Compile 携带结构化诊断（span 相对其来源文件），
/// 按该文件内容渲染带源码高亮的错误；其余变体（Parse/Io/Collision）直接 Display。
fn render_orchestrator_error(e: &crate::frontend::module::orchestrator::OrchestratorError) {
    use crate::frontend::module::orchestrator::OrchestratorError;

    let (path, diagnostics) = match e {
        OrchestratorError::TypeCheck {
            path, diagnostics, ..
        }
        | OrchestratorError::Compile {
            path, diagnostics, ..
        } => (path, diagnostics),
        other => {
            eprintln!();
            eprintln!("{}", other);
            return;
        }
    };

    eprintln!();
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let mut sources = SourceMap::new();
            let fid = sources.add_file(path.clone(), content);
            match sources.get(fid) {
                Some(source_file) => {
                    for d in diagnostics {
                        let output = render_compile_error(&d.message, source_file, Some(d));
                        eprintln!("{}", output);
                    }
                }
                None => {
                    for d in diagnostics {
                        eprintln!("{}", d);
                    }
                }
            }
        }
        // 源文件读不到（极端情况）：退化纯文本，至少诊断不丢
        Err(_) => {
            for d in diagnostics {
                eprintln!("{}", d);
            }
        }
    }
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

/// 按严重级别记账并收录诊断。
#[cfg(feature = "cli")]
fn push_diagnostic(
    result: &mut CheckResult,
    file: String,
    diagnostic: Diagnostic,
) {
    match diagnostic.severity {
        Severity::Error => result.error_count += 1,
        _ => result.warning_count += 1,
    }
    result
        .diagnostics
        .push(CheckDiagnostic { file, diagnostic });
}

/// 对多个文件进行静态检查并聚合诊断信息。
///
/// 项目内文件（向上能找到 `yaoxiang.toml`）走 orchestrator——与 `yaoxiang run`
/// 同一条多文件编译路径；非项目文件保持单文件检查。
#[cfg(feature = "cli")]
pub fn check_files_with_diagnostics(files: &[std::path::PathBuf]) -> anyhow::Result<CheckResult> {
    use crate::frontend::module::orchestrator;
    use std::collections::HashSet;

    let mut result = CheckResult::default();

    // 按项目根分组；同项目的多个入口各自发现一遍，用 seen 去重避免重复诊断。
    let mut project_groups: Vec<(std::path::PathBuf, Vec<std::path::PathBuf>)> = Vec::new();
    let mut standalone: Vec<&std::path::PathBuf> = Vec::new();
    for file in files {
        match orchestrator::find_project_root(file) {
            Some(root) => match project_groups.iter_mut().find(|(r, _)| *r == root) {
                Some((_, group)) => group.push(file.clone()),
                None => project_groups.push((root, vec![file.clone()])),
            },
            None => standalone.push(file),
        }
    }

    for (_, group) in &project_groups {
        let mut seen: HashSet<std::path::PathBuf> = HashSet::new();
        for entry in group {
            if seen.contains(entry) {
                continue;
            }
            let per_file =
                orchestrator::check_project(entry).map_err(|e| anyhow::anyhow!("{}", e))?;
            for (path, diagnostics) in per_file {
                // 已被前序入口的可达集覆盖：诊断收录过一次，再 push 会让
                // 输出重复、error_count/warning_count 翻倍（check 默认收集
                // 全项目文件时，共享依赖会被多个入口的可达集同时覆盖）
                if !seen.insert(path.clone()) {
                    continue;
                }
                // 读不回源码（竞态删除/权限）时不伪造空 SourceFile——
                // 渲染端对缺源文件本就按无片段处理
                if let Ok(source) = std::fs::read_to_string(&path) {
                    result.source_files.insert(
                        path.display().to_string(),
                        SourceFile::new(path.display().to_string(), source),
                    );
                }
                for diagnostic in diagnostics {
                    push_diagnostic(&mut result, path.display().to_string(), diagnostic);
                }
            }
        }
    }

    for file in standalone {
        check_single_file(file, &mut result)?;
    }

    Ok(result)
}

/// 单文件检查（无项目上下文）。
#[cfg(feature = "cli")]
fn check_single_file(
    file: &std::path::PathBuf,
    result: &mut CheckResult,
) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file.display(), e))?;
    let source_file = SourceFile::new(file.display().to_string(), source.clone());
    result
        .source_files
        .insert(file.display().to_string(), source_file);

    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source(&file.display().to_string(), &source) {
        Ok(_) => {
            // #321 M2：收割警告诊断（builder 按 W 前缀标注 Warning severity），
            // 计入 warning_count，不阻断编译
            for diag in compiler.take_warnings() {
                push_diagnostic(result, file.display().to_string(), diag);
            }
        }
        Err(e) if e.is_type_error() => {
            // #268：透传原始类型诊断（保留 E1002 与 span），与 run 一致；
            // 仅无原始诊断的 TypeError（如 IR 阶段）才用 E8001 兜底
            let diag = match e.diagnostic() {
                Some(diag) => diag.clone(),
                None => {
                    crate::util::diagnostic::ErrorCodeDefinition::internal_error(&format!("{}", e))
                        .build()
                }
            };
            push_diagnostic(result, file.display().to_string(), diag);
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            return Err(anyhow::anyhow!(err_msg));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
