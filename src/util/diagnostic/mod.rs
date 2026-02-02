//! 统一诊断系统
//!
//! 提供错误处理、诊断渲染和源码位置跟踪
//!
//! # 模块结构
//!
//! - [`diagnostic`] - 诊断数据结构 (Diagnostic, Severity)
//! - [`collect`] - 错误收集器
//! - [`result`] - 统一 Result 类型
//! - [`conversion`] - 错误转换
//! - [`span`] - Span 支持
//!
//! # 示例
//!
//! ```ignore
//! use yaoxiang::util::diagnostic::{Diagnostic, DiagnosticRenderer};
//!
//! let renderer = DiagnosticRenderer::new();
//! let output = renderer.render(&diagnostic, &source_file);
//! println!("{}", output);
//! ```

pub mod collect;
pub mod conversion;
pub mod error;
pub mod result;
pub mod span;

// 重新导出
pub use error::{Diagnostic, DiagnosticBuilder, Severity};
pub use collect::{ErrorCollector, Warning, ErrorFormatter};
pub use result::{Result, ResultExt};
pub use conversion::ErrorConvert;
pub use span::SpannedError;

// 渲染器
use crate::util::span::SourceFile;
use crate::util::diagnostic::error::Diagnostic as CoreDiagnostic;
use crate::util::diagnostic::error::Severity as CoreSeverity;

/// 渲染器配置
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    /// 是否启用颜色输出
    pub use_colors: bool,
    /// 是否显示源码片段
    pub show_source: bool,
    /// 是否显示帮助信息
    pub show_help: bool,
    /// 是否显示相关诊断
    pub show_related: bool,
    /// 是否显示 Unicode 字符
    pub unicode: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_source: true,
            show_help: true,
            show_related: true,
            unicode: true,
        }
    }
}

/// 诊断渲染器
#[derive(Debug, Clone)]
pub struct DiagnosticRenderer {
    /// 渲染配置
    config: EmitterConfig,
}

impl DiagnosticRenderer {
    /// 创建新的渲染器
    pub fn new() -> Self {
        Self {
            config: EmitterConfig::default(),
        }
    }

    /// 使用自定义配置创建渲染器
    pub fn with_config(config: EmitterConfig) -> Self {
        Self { config }
    }

    /// 渲染单个诊断信息
    pub fn render(
        &self,
        diagnostic: &CoreDiagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        self.render_internal(diagnostic, source_file, 0)
    }

    /// 渲染多个诊断信息
    pub fn render_all<'a>(
        &self,
        diagnostics: impl IntoIterator<Item = &'a CoreDiagnostic>,
        source_files: &[(&str, &SourceFile)],
    ) -> String {
        let mut output = String::new();
        for diagnostic in diagnostics {
            let source_file = diagnostic.span.as_ref().and_then(|_span| {
                source_files
                    .iter()
                    .find(|(name, _)| name.is_empty())
                    .map(|(_, sf)| *sf)
            });
            output.push_str(&self.render_internal(diagnostic, source_file, 0));
            output.push('\n');
        }
        output
    }

    /// 内部渲染方法（递归）
    fn render_internal(
        &self,
        diagnostic: &CoreDiagnostic,
        source_file: Option<&SourceFile>,
        _indent: usize,
    ) -> String {
        let mut output = String::new();

        // 1. 渲染错误头部
        output.push_str(&self.render_header(diagnostic));

        // 2. 渲染源码位置和片段
        if self.config.show_source {
            output.push_str(&self.render_source_location(diagnostic, source_file));
            if let Some(source_snippet) = self.render_source_snippet(diagnostic, source_file) {
                output.push_str(&source_snippet);
            }
        }

        // 3. 渲染帮助信息
        if self.config.show_help {
            if let Some(help) = self.render_help(diagnostic) {
                output.push_str(&format!("{}\n", help));
            }
        }

        // 4. 渲染相关诊断
        if self.config.show_related {
            for related in &diagnostic.related {
                output.push_str(&self.render_internal(related, source_file, _indent + 1));
            }
        }

        output
    }

    /// 渲染错误头部
    fn render_header(
        &self,
        diagnostic: &CoreDiagnostic,
    ) -> String {
        let severity = match diagnostic.severity {
            CoreSeverity::Error => "error",
            CoreSeverity::Warning => "warning",
            CoreSeverity::Info => "info",
            CoreSeverity::Hint => "hint",
        };

        let code = if diagnostic.code.is_empty() {
            String::new()
        } else {
            format!("[{}]", diagnostic.code)
        };

        if code.is_empty() {
            format!("{}{}\n", self.color(severity, severity), diagnostic.message)
        } else {
            format!(
                "{}{} {}\n",
                self.color(severity, severity),
                self.color("bold", &code),
                diagnostic.message
            )
        }
    }

    /// 渲染源码位置
    fn render_source_location(
        &self,
        diagnostic: &CoreDiagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        if let Some(span) = &diagnostic.span {
            if span.is_dummy() {
                return String::new();
            }

            let file_name = source_file
                .map(|sf| sf.name.as_str())
                .unwrap_or("<unknown>");

            let location = format!(
                " --> {}:{}:{}\n",
                file_name, span.start.line, span.start.column
            );

            return location;
        }
        String::new()
    }

    /// 渲染源码片段
    fn render_source_snippet(
        &self,
        diagnostic: &CoreDiagnostic,
        source_file: Option<&SourceFile>,
    ) -> Option<String> {
        let span = diagnostic.span.as_ref()?;
        if span.is_dummy() {
            return None;
        }

        let source_file = source_file?;
        let start_line = span.start.line;
        let end_line = span.end.line;

        // 渲染错误行
        if let Some(line) = self.get_line(source_file, start_line) {
            let mut output = String::new();

            // 添加行号
            let line_number = format!("{:>4} | ", start_line);
            output.push_str(&line_number);
            output.push_str(&line);
            output.push('\n');

            // 添加错误下划线
            if start_line == end_line {
                // 单行错误
                let spaces = " ".repeat(span.start.column - 1);
                let carets = "^".repeat(span.end.column.saturating_sub(span.start.column).max(1));
                output.push_str(&format!("{} | {}{}\n", " ".repeat(4), spaces, carets));
            } else {
                // 多行错误
                let spaces = " ".repeat(span.start.column - 1);
                let carets = "^".repeat(line.len().saturating_sub(span.start.column - 1).max(1));
                output.push_str(&format!("{} | {}{}\n", " ".repeat(4), spaces, carets));

                // 中间行（如果有）
                for line_num in (start_line + 1)..end_line {
                    if let Some(middle_line) = self.get_line(source_file, line_num) {
                        let line_number = format!("{:>4} | ", line_num);
                        output.push_str(&line_number);
                        output.push_str(&middle_line);
                        output.push('\n');
                    }
                }

                // 最后行
                if let Some(last_line) = self.get_line(source_file, end_line) {
                    let line_number = format!("{:>4} | ", end_line);
                    output.push_str(&line_number);
                    output.push_str(&last_line);
                    output.push('\n');
                    let spaces = " ".repeat(span.end.column - 1);
                    output.push_str(&format!("{} | {}\n", " ".repeat(4), spaces));
                }
            }

            return Some(output);
        }

        None
    }

    /// 渲染帮助信息
    fn render_help(
        &self,
        _diagnostic: &CoreDiagnostic,
    ) -> Option<String> {
        // TODO: 实现帮助信息渲染
        None
    }

    /// 获取指定行
    fn get_line(
        &self,
        source_file: &SourceFile,
        line_num: usize,
    ) -> Option<String> {
        let lines: Vec<&str> = source_file.content.lines().collect();
        lines.get(line_num - 1).map(|s| s.to_string())
    }

    /// 简单的颜色渲染
    fn color(
        &self,
        style: &str,
        text: &str,
    ) -> String {
        if !self.config.use_colors {
            return text.to_string();
        }

        match style {
            "error" => format!("\x1b[31m{}\x1b[0m", text),
            "warning" => format!("\x1b[33m{}\x1b[0m", text),
            "info" => format!("\x1b[34m{}\x1b[0m", text),
            "hint" => format!("\x1b[36m{}\x1b[0m", text),
            "bold" => format!("\x1b[1m{}\x1b[0m", text),
            _ => text.to_string(),
        }
    }
}

impl Default for DiagnosticRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// 渲染编译错误
///
/// 从错误消息解析并渲染为 Rust 风格的诊断输出
pub fn render_compile_error(
    error: &str,
    source_file: &SourceFile,
    diagnostic: Option<&CoreDiagnostic>,
) -> String {
    let renderer = DiagnosticRenderer::new();

    // 如果有诊断信息，使用它；否则从消息解析
    let diagnostic = match diagnostic {
        Some(d) => d.clone(),
        None => parse_compile_error(error),
    };

    renderer.render(&diagnostic, Some(source_file))
}

/// 解析编译错误为诊断信息
pub fn parse_compile_error(error: &str) -> CoreDiagnostic {
    let message = error.to_string();

    // 提取错误代码
    let (code, message) = extract_error_code(&message);

    Diagnostic::new(CoreSeverity::Error, code, message, None)
}

/// 提取错误代码
fn extract_error_code(message: &str) -> (String, String) {
    if message.contains("Unknown variable") {
        ("E0002".to_string(), message.to_string())
    } else if message.contains("Type mismatch") {
        ("E0003".to_string(), message.to_string())
    } else if message.contains("Inference error") {
        ("E0001".to_string(), message.to_string())
    } else {
        ("E0000".to_string(), message.to_string())
    }
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

        let diagnostic = CoreDiagnostic::error(
            "E0002".to_string(),
            "Unknown variable: a".to_string(),
            Some(Span::new(
                Position::with_offset(5, 7, 65),
                Position::with_offset(5, 8, 66),
            )),
        );

        let renderer = DiagnosticRenderer::new();
        let output = renderer.render(&diagnostic, Some(&source_file));
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error[E0002]"), "{}", clean_output);
        assert!(
            clean_output.contains("Unknown variable: a"),
            "{}",
            clean_output
        );
        assert!(clean_output.contains("error.yx:5:7"), "{}", clean_output);
        assert!(clean_output.contains("print(a)"), "{}", clean_output);
        assert!(clean_output.contains("^"), "{}", clean_output);
    }

    #[test]
    fn test_render_no_source_file() {
        let diagnostic =
            CoreDiagnostic::error("E0001".to_string(), "Type mismatch".to_string(), None);

        let renderer = DiagnosticRenderer::new();
        let output = renderer.render(&diagnostic, None);
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error[E0001]"), "{}", clean_output);
        assert!(clean_output.contains("Type mismatch"), "{}", clean_output);
    }

    #[test]
    fn test_parse_compile_error() {
        // 测试 Unknown variable 优先
        let diagnostic = parse_compile_error("Inference error: Unknown variable: a");
        assert_eq!(diagnostic.code, "E0002");
        assert!(diagnostic.message.contains("Unknown variable: a"));

        // 测试纯 Inference error
        let diagnostic = parse_compile_error("Inference error: some other error");
        assert_eq!(diagnostic.code, "E0001");
    }
}
