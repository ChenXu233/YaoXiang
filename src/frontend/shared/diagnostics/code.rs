//! 错误码定义
//!
//! 定义各种错误码

/// 错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(pub u32);

impl ErrorCode {
    /// 创建新的错误码
    pub fn new(code: u32) -> Self {
        Self(code)
    }

    /// 获取错误码的值
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// 预定义的错误码
pub mod codes {
    use super::ErrorCode;

    /// 类型错误
    pub const TYPE_ERROR: ErrorCode = ErrorCode(1000);

    /// 解析错误
    pub const PARSE_ERROR: ErrorCode = ErrorCode(2000);

    /// 泛型错误
    pub const GENERIC_ERROR: ErrorCode = ErrorCode(3000);
}
