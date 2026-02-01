//! Span 处理
//!
//! 提供源码位置跟踪功能

use std::fmt;

/// 源码跨度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file_id: usize,
}

impl Span {
    pub fn new(
        start: usize,
        end: usize,
        file_id: usize,
    ) -> Self {
        Self {
            start,
            end,
            file_id,
        }
    }

    /// 创建默认的 Span（位置为 0:0）
    #[allow(clippy::new_ret_no_self)]
    pub fn new_default() -> Self {
        Self {
            start: 0,
            end: 0,
            file_id: 0,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}:{}", self.start, self.end)
    }
}
