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
    /// 是否排序导入语句
    pub sort_imports: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 120,
            indent_width: 4,
            use_tabs: false,
            single_quote: false,
            sort_imports: true,
        }
    }
}

impl From<&FmtConfig> for FormatOptions {
    fn from(config: &FmtConfig) -> Self {
        let default = FormatOptions::default();
        Self {
            line_width: config.line_width.unwrap_or(default.line_width),
            indent_width: config.indent_width.unwrap_or(default.indent_width),
            use_tabs: config.use_tabs.unwrap_or(default.use_tabs),
            single_quote: config.single_quote.unwrap_or(default.single_quote),
            sort_imports: config.sort_imports.unwrap_or(default.sort_imports),
        }
    }
}

impl From<FmtConfig> for FormatOptions {
    fn from(config: FmtConfig) -> Self {
        Self::from(&config)
    }
}
