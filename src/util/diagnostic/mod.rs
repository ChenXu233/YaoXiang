//! 统一诊断系统
//!
//! 提供错误处理、诊断渲染和源码位置跟踪
//!
//! # 模块结构
//!
//! - [`diagnostic`] - 诊断数据结构 (Diagnostic, Severity)
//! - [`codes`] - 错误码注册表
//! - [`emitter`] - 诊断输出渲染器
//! - [`suggest`] - 智能建议引擎
//! - [`collect`] - 错误收集器
//! - [`result`] - 统一 Result 类型
//! - [`conversion`] - 错误转换
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
pub mod conversion;
pub mod emitter;
pub mod error;
#[macro_use]
pub mod error_macro;
pub mod result;
pub mod suggest;

// 重新导出
pub use codes::{ErrorCategory, ErrorCodeDefinition, I18nRegistry, DiagnosticBuilder, ErrorInfo};
pub use collect::{ErrorCollector, Warning, ErrorFormatter};
pub use command::{render_explain_output, run_check_command_once, run_check_watch_command};
pub use conversion::ErrorConvert;
pub use emitter::{TextEmitter, JsonEmitter, RichEmitter, EmitterConfig, RichConfig};
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
pub fn run_file_with_diagnostics(
    file: &std::path::PathBuf,
    debug_info: bool,
) -> anyhow::Result<()> {
    use crate::frontend::Compiler;
    use crate::middle::passes::codegen::CodegenContext;
    use crate::Executor;
    use crate::Interpreter;

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
            // 可变性检查：在代码生成之前检查不可变变量的重复赋值
            {
                use crate::middle::passes::lifetime::{MutChecker, OwnershipError};
                let mut mut_checker = MutChecker::new();
                let empty_set = std::collections::HashSet::new();
                for func in &module.functions {
                    let mut_locals = module.mut_locals.get(&func.name).unwrap_or(&empty_set);
                    let loop_binding_locals = module.loop_binding_locals.get(&func.name);
                    let local_names = module.local_names.get(&func.name);
                    let errors = mut_checker.check_function_with_mut_locals(
                        func,
                        mut_locals,
                        loop_binding_locals,
                        local_names,
                    );
                    if !errors.is_empty() {
                        eprintln!();
                        for err in &errors {
                            match err {
                                OwnershipError::ImmutableAssign { value, span } => {
                                    // 变量名已经是正确的（从 local_names 获取）
                                    let mut diag = ErrorCodeDefinition::immutable_assignment(value);

                                    // 如果有 span，使用它
                                    if let Some(span) = span {
                                        diag = diag.at(*span);
                                    }

                                    let diagnostic = diag.build();
                                    let output = render_compile_error(
                                        &diagnostic.message,
                                        source_file,
                                        Some(&diagnostic),
                                    );
                                    eprintln!("{}", output);
                                }
                                _ => {
                                    // 其他错误类型，使用原来的方式
                                    let diag =
                                        ErrorCodeDefinition::immutable_assignment(&err.to_string())
                                            .build();
                                    let output = render_compile_error(
                                        &diag.message,
                                        source_file,
                                        Some(&diag),
                                    );
                                    eprintln!("{}", output);
                                }
                            }
                        }
                        return Err(anyhow::anyhow!("Mutability check failed"));
                    }
                }
            }

            // Generate bytecode
            let mut ctx = CodegenContext::new(module);
            ctx.set_generate_debug_info(debug_info);
            let bytecode_file = ctx
                .generate()
                .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
            let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

            // Execute
            let mut executor: Box<dyn Executor> = Box::new(Interpreter::new());
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
/// 说明：当前编译管线会优先返回首个结构化诊断，因此每个失败文件
/// 通常会产生一个主诊断条目。
pub fn check_files_with_diagnostics(files: &[std::path::PathBuf]) -> anyhow::Result<CheckResult> {
    use crate::frontend::Compiler;

    let mut result = CheckResult::default();

    for file in files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                let diagnostic = ErrorCodeDefinition::internal_error(&format!(
                    "Failed to read file {}: {}",
                    file.display(),
                    e
                ))
                .build();
                result.error_count += 1;
                result.diagnostics.push(CheckDiagnostic {
                    file: file.display().to_string(),
                    diagnostic,
                });
                continue;
            }
        };

        let source_name = file.display().to_string();
        let source_file = SourceFile::new(source_name.clone(), source.clone());
        result
            .source_files
            .insert(source_name.clone(), source_file.clone());

        let mut compiler = Compiler::new();
        if let Err(e) = compiler.compile(&source_name, &source) {
            let diagnostic = e
                .diagnostic()
                .cloned()
                .unwrap_or_else(|| parse_compile_error(e.message()));

            if diagnostic.severity.is_error() {
                result.error_count += 1;
            } else {
                result.warning_count += 1;
            }

            result.diagnostics.push(CheckDiagnostic {
                file: source_name,
                diagnostic,
            });
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::span::{DebugSpan, SourceFile, SourceMap, Span, Position};
    use crate::backends::{ExecutorError, StackFrame};
    use crate::middle::bytecode::{BytecodeModule, BytecodeFunction, BytecodeInstr};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    /// 移除 ANSI 转义序列
    fn strip_ansi(s: &str) -> String {
        s.replace("\x1b[31m", "")
            .replace("\x1b[33m", "")
            .replace("\x1b[34m", "")
            .replace("\x1b[36m", "")
            .replace("\x1b[1m", "")
            .replace("\x1b[0m", "")
    }

    #[test]
    fn test_render_unknown_variable() {
        let source = r#"use std.io

main = () => {
  print("Testing error handling\n")
  print(a)
  print("All tests passed!\n")
}"#;

        let source_file = SourceFile::new("error.yx".to_string(), source.to_string());

        let diagnostic = ErrorCodeDefinition::unknown_variable("a")
            .at(Span::new(
                Position::with_offset(5, 7, 65),
                Position::with_offset(5, 8, 66),
            ))
            .build();

        let emitter = TextEmitter::new();
        let output = emitter.render_with_source(&diagnostic, Some(&source_file));
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error [E1001]"), "{}", clean_output);
        assert!(
            clean_output.contains("Unknown variable"),
            "{}",
            clean_output
        );
        assert!(clean_output.contains("error.yx:5:7"), "{}", clean_output);
        assert!(clean_output.contains("print(a)"), "{}", clean_output);
        assert!(clean_output.contains("^"), "{}", clean_output);
    }

    #[test]
    fn test_render_no_source_file() {
        let diagnostic = ErrorCodeDefinition::find("E0001")
            .unwrap()
            .builder()
            .param("char", "@")
            .build();

        let emitter = TextEmitter::new();
        let output = emitter.render(&diagnostic);
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error [E0001]"), "{}", clean_output);
        assert!(
            clean_output.contains("Invalid character"),
            "{}",
            clean_output
        );
    }

    #[test]
    fn test_parse_compile_error() {
        // parse_compile_error 现在统一使用 E8001 内部错误
        let diagnostic = parse_compile_error("Inference error: Unknown variable: a");
        assert_eq!(diagnostic.code, "E8001");
        assert!(diagnostic.message.contains("Unknown variable: a"));

        let diagnostic = parse_compile_error("Inference error: some other error");
        assert_eq!(diagnostic.code, "E8001");
    }

    #[test]
    fn test_error_code_lookup() {
        let code = ErrorCodeDefinition::find("E0001");
        assert!(code.is_some());
        assert_eq!(code.unwrap().code, "E0001");

        let code = ErrorCodeDefinition::find("E9999");
        assert!(code.is_none());
    }

    #[test]
    fn test_error_code_get_all() {
        let all = ErrorCodeDefinition::all();
        assert!(
            all.len() > 30,
            "Expected more than 30 error codes, got {}",
            all.len()
        );
    }

    #[test]
    fn test_render_runtime_function_not_found_with_span() {
        let source = r#"main = () => {
  foo()
}"#;
        let mut sources = SourceMap::new();
        let file_id = sources.add_file("error.yx".to_string(), source.to_string());

        let span = Span::new(
            Position::with_offset(2, 3, 0),
            Position::with_offset(2, 6, 0),
        );
        let debug_span = DebugSpan::new(file_id, span);

        let mut module = BytecodeModule::new("test".to_string());
        module.add_function(BytecodeFunction {
            name: "main".to_string(),
            params: vec![],
            return_type: crate::middle::core::ir::Type::Void,
            local_count: 0,
            upvalue_count: 0,
            instructions: vec![BytecodeInstr::Nop],
            labels: HashMap::new(),
            exception_handlers: vec![],
            debug_map: HashMap::from([(0usize, debug_span)]),
        });

        let err = ExecutorError::function_not_found(
            "foo".to_string(),
            vec![StackFrame {
                function_name: "main".to_string(),
                ip: 0,
            }],
        );

        let output = render_runtime_error(&err, &module, Some(&sources));
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error [E6006]"), "{}", clean_output);
        assert!(
            clean_output.contains("Function not found"),
            "{}",
            clean_output
        );
        assert!(clean_output.contains("error.yx:2:3"), "{}", clean_output);
        assert!(clean_output.contains("foo()"), "{}", clean_output);
        assert!(clean_output.contains("stack trace:"), "{}", clean_output);
        assert!(
            clean_output.contains("at main (error.yx:2:3) (ip: 0)"),
            "{}",
            clean_output
        );
    }

    #[test]
    fn test_check_files_with_diagnostics_ok() {
        let dir = tempdir().expect("create temp dir");
        let file = dir.path().join("ok.yx");
        fs::write(
            &file,
            r#"use std.io

main: () -> Void = {
  print("ok")
}
"#,
        )
        .expect("write yx file");

        let result = check_files_with_diagnostics(&vec![file]).expect("run check");
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_check_files_with_diagnostics_error() {
        let dir = tempdir().expect("create temp dir");
        let file = dir.path().join("bad.yx");
        fs::write(
            &file,
            r#"use std.io

main: () -> Void = {
  print(a)
}
"#,
        )
        .expect("write yx file");

        let result = check_files_with_diagnostics(&vec![file]).expect("run check");
        assert!(result.error_count > 0);
        assert!(!result.diagnostics.is_empty());
    }
}
