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
pub use conversion::ErrorConvert;
pub use emitter::{TextEmitter, JsonEmitter, RichEmitter, EmitterConfig, RichConfig};
pub use error::{Diagnostic, Severity};
pub use result::{Result, ResultExt};
pub use suggest::SuggestionEngine;

// 渲染器
use crate::util::span::SourceFile;

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

/// 运行文件并美化错误输出
///
/// # 参数
/// - `file`: 源文件路径
///
/// # 返回
/// 成功返回 `()`，失败返回错误
pub fn run_file_with_diagnostics(file: &std::path::PathBuf) -> anyhow::Result<()> {
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
    let source_file = SourceFile::new(source_name.clone(), source.clone());

    let mut compiler = Compiler::new();
    match compiler.compile(&source_name, &source) {
        Ok(module) => {
            // 可变性检查：在代码生成之前检查不可变变量的重复赋值
            {
                use crate::middle::passes::lifetime::{MutChecker, OwnershipError};
                let mut mut_checker = MutChecker::new();
                let empty_set = std::collections::HashSet::new();
                for func in &module.functions {
                    let mut_locals = module.mut_locals.get(&func.name).unwrap_or(&empty_set);
                    let errors = mut_checker.check_function_with_mut_locals(func, mut_locals);
                    if !errors.is_empty() {
                        eprintln!();
                        for err in &errors {
                            match err {
                                OwnershipError::ImmutableAssign { value, span } => {
                                    // 提取变量名（去掉 local_ 前缀）
                                    let name = value.strip_prefix("local_").unwrap_or(value);
                                    let mut diag = ErrorCodeDefinition::immutable_assignment(name);

                                    // 如果有 span，使用它
                                    if let Some(span) = span {
                                        diag = diag.at(*span);
                                    }

                                    let diagnostic = diag.build();
                                    let output = render_compile_error(
                                        &diagnostic.message,
                                        &source_file,
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
                                        &source_file,
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
            let bytecode_file = ctx
                .generate()
                .map_err(|e| anyhow::anyhow!("Codegen failed: {:?}", e))?;
            let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

            // Execute
            let mut executor: Box<dyn Executor> = Box::new(Interpreter::new());
            executor
                .execute_module(&bytecode_module)
                .map_err(|e| anyhow::anyhow!("Runtime error: {}", e))?;
        }
        Err(e) => {
            // 使用渲染器输出美化后的错误
            eprintln!();
            let output = render_compile_error(e.message(), &source_file, e.diagnostic());
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
    use crate::frontend::Compiler;

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
    let source_file = SourceFile::new(source_name.clone(), source.clone());

    let mut compiler = Compiler::new();
    match compiler.compile(&source_name, &source) {
        Ok(_) => {
            // 类型检查成功
            println!("Type check passed for {}", file.display());
        }
        Err(e) => {
            // 使用渲染器输出美化后的错误
            eprintln!();
            let output = render_compile_error(e.message(), &source_file, e.diagnostic());
            eprintln!("{}", output);
            return Err(anyhow::anyhow!("Type check failed"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::span::{SourceFile, Span, Position};

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
}
