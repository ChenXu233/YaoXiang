//! 格式化选项
//!
//! 定义代码格式化的配置参数。

use crate::util::config::FmtConfig;

/// 格式化选项
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// 最大行宽
    pub line_width: usize,
    /// 缩进宽度（空格数）
    pub indent_width: usize,
    /// 使用 tab 缩进
    pub use_tabs: bool,
    /// 使用单引号
    pub single_quote: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 120,
            indent_width: 4,
            use_tabs: false,
            single_quote: false,
        }
    }
}

impl From<&FmtConfig> for FormatOptions {
    fn from(config: &FmtConfig) -> Self {
        Self {
            line_width: config.line_width,
            indent_width: config.indent_width,
            use_tabs: config.use_tabs,
            single_quote: config.single_quote,
        }
    }
}

impl From<FmtConfig> for FormatOptions {
    fn from(config: FmtConfig) -> Self {
        Self::from(&config)
    }
}
