//! 富格式诊断渲染器
//!
//! 提供终端友好的彩色输出，带有更好的可视效果

use crate::util::span::SourceFile;
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::Severity;

/// 富格式配置
#[derive(Debug, Clone)]
pub struct RichConfig {
    /// 启用颜色
    pub colors: bool,
    /// 启用 Unicode 符号
    pub unicode: bool,
    /// 启用插图符号
    pub symbols: bool,
    /// 指示符字符
    pub indicator: char,
    /// 最大源码行数
    pub max_source_lines: usize,
    /// 启用内联提示
    pub inline_hints: bool,
}

impl Default for RichConfig {
    fn default() -> Self {
        Self {
            colors: true,
            unicode: true,
            symbols: true,
            indicator: '^',
            max_source_lines: 6,
            inline_hints: true,
        }
    }
}

/// 富格式诊断渲染器
#[derive(Debug, Clone)]
pub struct RichEmitter {
    config: RichConfig,
}

impl RichEmitter {
    /// 创建新的富格式渲染器
    pub fn new() -> Self {
        Self {
            config: RichConfig::default(),
        }
    }

    /// 使用自定义配置创建渲染器
    pub fn with_config(config: RichConfig) -> Self {
        Self { config }
    }

    /// 渲染诊断
    pub fn render(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        let mut output = String::new();

        // 1. 渲染头部（带图标）
        output.push_str(&self.render_header(diagnostic));

        // 2. 渲染位置
        output.push_str(&self.render_location(diagnostic, source_file));

        // 3. 渲染源码片段（带高亮）
        if self.config.max_source_lines > 0 {
            if let Some(snippet) = self.render_source(diagnostic, source_file) {
                output.push_str(&snippet);
            }
        }

        // 4. 渲染帮助
        if self.config.inline_hints {
            if let Some(help) = self.render_help(diagnostic) {
                output.push_str(&self.hint_prefix());
                output.push_str(&help);
                output.push('\n');
            }
        }

        output
    }

    /// 渲染头部
    fn render_header(&self, diagnostic: &Diagnostic) -> String {
        let severity_text = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };

        let style = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };

        if diagnostic.code.is_empty() {
            format!(
                "{}{}\n",
                self.color(style, severity_text),
                diagnostic.message
            )
        } else {
            format!(
                "{} [{}] {}\n",
                self.color(style, severity_text),
                self.color("bold", &diagnostic.code),
                diagnostic.message
            )
        }
    }

    /// 渲染位置
    fn render_location(
        &self,
        diagnostic: &Diagnostic,
        source_file: Option<&SourceFile>,
    ) -> String {
        if let Some(span) = &diagnostic.span {
            if span.is_dummy() {
                return String::new();
            }

            let file_name = source_file.map(|sf| sf.name.as_str()).unwrap_or("<unknown>");
            format!(
                "{}{}:{}:{}",
                self.color("muted", " --> "),
                file_name,
                span.start.line,
                span.start.column
            )
        } else {
            String::new()
        }
    }

    /// 获取源码行
    fn get_source_line(source_file: &SourceFile, line_num: usize) -> Option<String> {
        let lines: Vec<&str> = source_file.content.lines().collect();
        lines.get(line_num - 1).map(|s| s.to_string())
    }

    /// 渲染源码片段
    fn render_source(
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
        let lines_to_show = (end_line - start_line + 1).min(self.config.max_source_lines);

        let mut output = String::new();

        for i in 0..lines_to_show {
            let line_num = start_line + i;
            if let Some(line) = Self::get_source_line(source_file, line_num) {
                output.push_str(&format!("{:>4} {} ", line_num, self.vbar()));
                output.push_str(&line);
                output.push('\n');

                if i == 0 {
                    output.push('\n');
                    let spaces = " ".repeat(span.start.column + 4);
                    let len = if start_line == end_line {
                        (span.end.column - span.start.column).max(1)
                    } else {
                        line.len().saturating_sub(span.start.column - 1).max(1)
                    };
                    let indicators = self.config.indicator.to_string().repeat(len);
                    output.push_str(&format!("{} {}{}\n", " ".repeat(5), spaces, indicators));
                } else {
                    output.push('\n');
                }
            }
        }

        Some(output)
    }

    /// 渲染帮助
    fn render_help(&self, diagnostic: &Diagnostic) -> Option<String> {
        // 帮助信息已内联在 Diagnostic 中
        if diagnostic.help.is_empty() {
            return None;
        }
        Some(diagnostic.help.clone())
    }

    /// 垂直分隔符
    fn vbar(&self) -> &'static str {
        if self.config.unicode {
            "│"
        } else {
            "|"
        }
    }

    /// 提示前缀
    fn hint_prefix(&self) -> String {
        if self.config.symbols {
            self.color("hint", "💡 ")
        } else {
            self.color("hint", "hint: ")
        }
    }

    /// 颜色应用
    fn color(&self, style: &str, text: &str) -> String {
        if !self.config.colors {
            return text.to_string();
        }

        match style {
            "error" => format!("\x1b[31m{}\x1b[0m", text),
            "warning" => format!("\x1b[33m{}\x1b[0m", text),
            "info" => format!("\x1b[34m{}\x1b[0m", text),
            "hint" => format!("\x1b[36m{}\x1b[0m", text),
            "bold" => format!("\x1b[1m{}\x1b[0m", text),
            "muted" => format!("\x1b[90m{}\x1b[0m", text),
            _ => text.to_string(),
        }
    }
}

impl Default for RichEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::util::diagnostic::codes::{ErrorCodeDefinition, I18nRegistry};

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
    fn test_rich_render() {
        let i18n = I18nRegistry::en();
        let diagnostic = ErrorCodeDefinition::invalid_character("@")
            .build(i18n);

        let emitter = RichEmitter::new();
        let output = emitter.render(&diagnostic, None);
        let clean_output = strip_ansi(&output);

        // 输出格式为 "error [E0001]" (带空格)
        assert!(clean_output.contains("error [E0001]"), "{}", clean_output);
    }
}
