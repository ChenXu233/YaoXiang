//! 格式化上下文
//!
//! 跟踪格式化过程中的缩进、行位置等状态。

use super::options::FormatOptions;

/// 格式化上下文
#[derive(Debug, Clone)]
pub struct FormatContext {
    /// 当前缩进级别
    pub indent_level: usize,
    /// 格式化选项
    pub options: FormatOptions,
    /// 当前行的字符数（用于判断是否需要换行）
    pub current_line_width: usize,
}

impl FormatContext {
    /// 创建新的格式化上下文
    pub fn new(options: FormatOptions) -> Self {
        Self {
            indent_level: 0,
            options,
            current_line_width: 0,
        }
    }

    /// 增加缩进级别
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// 减少缩进级别
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// 获取当前缩进字符串
    pub fn indent_str(&self) -> String {
        if self.options.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(self.indent_level * self.options.indent_width)
        }
    }

    /// 获取缩进宽度（字符数）
    pub fn indent_width(&self) -> usize {
        self.indent_level * self.options.indent_width
    }

    /// 检查当前行是否超过行宽
    pub fn exceeds_line_width(&self) -> bool {
        self.current_line_width > self.options.line_width
    }

    /// 检查添加指定长度内容后是否超过行宽
    pub fn should_break(
        &self,
        additional_len: usize,
    ) -> bool {
        self.current_line_width + additional_len > self.options.line_width
    }
}
