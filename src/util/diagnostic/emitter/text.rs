//! 诊断渲染器

use crate::util::span::SourceFile;
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::Severity;

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
    /// 是否显示行号
    pub show_line_numbers: bool,
    /// 是否使用 Unicode 字符
    pub unicode: bool,
    /// 指示字符 (默认: "^")
    pub indicator: char,
    /// 最大显示行数
    pub max_lines: usize,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_source: true,
            show_help: true,
            show_related: true,
            show_line_numbers: true,
            unicode: true,
            indicator: '^',
            max_lines: 6,
        }
    }
}

/// 诊断渲染器 trait
pub trait DiagnosticEmitter {
    fn emit(
        &self,
        diagnostic: &Diagnostic,
    ) -> String;
}

/// 文本诊断渲染器
#[derive(Debug, Clone)]
pub struct TextEmitter {
    config: EmitterConfig,
}

impl TextEmitter {
    /// 创建新的文本渲染器
    pub fn new() -> Self {
        Self {
            config: EmitterConfig::default(),
        }
    }

    /// 使用自定义配置创建渲染器
    pub fn with_config(config: EmitterConfig) -> Self {
        Self { config }
    }

    /// 渲染单个诊断
    pub fn render(
        &self,
        diagnostic: &Diagnostic,
    ) -> String {
        self.render_internal(diagnostic, None, 0)
    }

    /// 渲染诊断到指定源码文件
    pub fn render_with_source(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        self.render_internal(diagnostic, source_file, 0)
    }

    /// 内部渲染方法（递归）
    fn render_internal(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
        _indent: usize,
    ) -> String {
        let mut output = String::new();

        // 1. 渲染头部
        output.push_str(&self.render_header(diagnostic));

        // 2. 渲染位置
        output.push_str(&self.render_location(diagnostic, source_file));

        // 3. 渲染源码片段
        if self.config.show_source {
            if let Some(snippet) = self.render_source_snippet(diagnostic, source_file) {
                output.push_str(&snippet);
            }
        }

        // 4. 渲染帮助信息
        if self.config.show_help {
            if let Some(help) = self.render_help(diagnostic) {
                output.push_str("help: ");
                output.push_str(&help);
                output.push('\n');
            }
        }

        // 5. 渲染相关诊断
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
        diagnostic: &Diagnostic,
    ) -> String {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };

        if diagnostic.code.is_empty() {
            format!("{}{}\n", self.color(severity, severity), diagnostic.message)
        } else {
            format!(
                "{} [{}] {}\n",
                self.color(severity, severity),
                self.color("bold", &diagnostic.code),
                diagnostic.message
            )
        }
    }

    /// 渲染位置信息
    fn render_location(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        if let Some(span) = &diagnostic.span {
            if span.is_dummy() {
                return String::new();
            }

            let file_name = source_file
                .map(|sf| sf.name.as_str())
                .unwrap_or("<unknown>");
            format!(
                " --> {}:{}:{}\n",
                file_name, span.start.line, span.start.column
            )
        } else {
            String::new()
        }
    }

    /// 获取源码行
    fn get_source_line(
        source_file: &SourceFile,
        line_num: usize,
    ) -> Option<String> {
        let lines: Vec<&str> = source_file.content.lines().collect();
        lines.get(line_num - 1).map(|s| s.to_string())
    }

    /// 渲染源码片段
    fn render_source_snippet(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
    ) -> Option<String> {
        let span = diagnostic.span.as_ref()?;
        if span.is_dummy() {
            return None;
        }

        let source_file = source_file?;
        let start_line = span.start.line;
        let end_line = span.end.line;

        // 限制显示行数
        let lines_to_show = (end_line - start_line + 1).min(self.config.max_lines);
        let mut output = String::new();

        for i in 0..lines_to_show {
            let line_num = start_line + i;
            if let Some(line) = Self::get_source_line(source_file, line_num) {
                if self.config.show_line_numbers {
                    output.push_str(&format!("{:>4} | ", line_num));
                } else {
                    output.push_str("     | ");
                }
                output.push_str(&line);
                output.push('\n');

                // 如果是第一行，添加错误指示
                if i == 0 {
                    let spaces = " ".repeat(span.start.column - 1);
                    let indicator_len = if start_line == end_line {
                        (span.end.column - span.start.column).max(1)
                    } else {
                        line.len().saturating_sub(span.start.column - 1).max(1)
                    };
                    let indicators = self.config.indicator.to_string().repeat(indicator_len);

                    output.push_str(&format!("     | {}{}\n", spaces, indicators));
                }
            }
        }

        Some(output)
    }

    /// 渲染帮助信息
    fn render_help(
        &self,
        diagnostic: &Diagnostic,
    ) -> Option<String> {
        // 帮助信息已内联在 Diagnostic 中
        if diagnostic.help.is_empty() {
            return None;
        }
        Some(diagnostic.help.clone())
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

impl Default for TextEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::diagnostic::codes::ErrorCodeDefinition;
    use crate::util::span::Span;

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
    fn test_render_basic_error() {
        let diagnostic = ErrorCodeDefinition::invalid_character("@").build();

        let emitter = TextEmitter::new();
        let output = emitter.render(&diagnostic);
        let clean_output = strip_ansi(&output);

        // 输出格式为 "error [E0001]" (带空格)
        assert!(clean_output.contains("error [E0001]"), "{}", clean_output);
        assert!(clean_output.contains("Invalid character"));
    }

    #[test]
    fn test_render_error_with_span() {
        let diagnostic = ErrorCodeDefinition::type_mismatch("Int", "String")
            .at(Span::dummy())
            .build();

        let emitter = TextEmitter::new();
        let output = emitter.render(&diagnostic);
        let clean_output = strip_ansi(&output);

        assert!(clean_output.contains("error [E1002]"), "{}", clean_output);
    }

    #[test]
    fn test_config_options() {
        let config = EmitterConfig {
            use_colors: false,
            show_help: true,
            ..Default::default()
        };

        let diagnostic = ErrorCodeDefinition::invalid_character("@").build();

        let emitter = TextEmitter::with_config(config);
        let output = emitter.render(&diagnostic);

        // 验证颜色被禁用
        assert!(!output.contains("\x1b[31m"));
    }
}
