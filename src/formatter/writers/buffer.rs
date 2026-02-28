//! 格式化输出缓冲
//!
//! 提供受控的字符串构建器，跟踪行宽和缩进。

/// 格式化输出缓冲
#[derive(Debug, Clone)]
pub struct FormatBuffer {
    /// 输出内容
    output: String,
    /// 当前行的字符数
    current_line_width: usize,
}

impl FormatBuffer {
    /// 创建新的缓冲
    pub fn new() -> Self {
        Self {
            output: String::new(),
            current_line_width: 0,
        }
    }

    /// 写入文本
    pub fn write(
        &mut self,
        text: &str,
    ) {
        for ch in text.chars() {
            if ch == '\n' {
                self.current_line_width = 0;
            } else {
                self.current_line_width += 1;
            }
        }
        self.output.push_str(text);
    }

    /// 写入换行
    pub fn newline(&mut self) {
        self.output.push('\n');
        self.current_line_width = 0;
    }

    /// 获取当前行宽
    pub fn current_line_width(&self) -> usize {
        self.current_line_width
    }

    /// 获取输出内容
    pub fn finish(self) -> String {
        self.output
    }

    /// 检查缓冲是否为空
    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }

    /// 检查最后一个字符是否是换行符
    pub fn ends_with_newline(&self) -> bool {
        self.output.ends_with('\n')
    }

    /// 获取内容引用
    pub fn as_str(&self) -> &str {
        &self.output
    }
}

impl Default for FormatBuffer {
    fn default() -> Self {
        Self::new()
    }
}
