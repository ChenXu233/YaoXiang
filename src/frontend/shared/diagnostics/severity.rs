//! 严重级别
//!
//! 定义诊断信息的严重级别

/// 严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Debug,
}

impl Severity {
    /// 获取严重级别对应的数字值
    pub fn as_u8(&self) -> u8 {
        match self {
            Severity::Error => 4,
            Severity::Warning => 3,
            Severity::Info => 2,
            Severity::Debug => 1,
        }
    }

    /// 检查是否为错误级别
    pub fn is_error(&self) -> bool {
        matches!(self, Severity::Error)
    }
}
