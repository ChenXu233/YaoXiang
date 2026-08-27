//! 错误收集和格式化
//!
//! 提供通用的错误收集器和格式化工具

use super::Diagnostic;

/// 错误收集器
///
/// 收集多个错误，支持批量报告
#[derive(Debug, Default)]
pub struct ErrorCollector {
    /// 错误列表
    errors: Vec<Diagnostic>,
}

impl ErrorCollector {
    /// 创建新的错误收集器
    pub fn new() -> Self {
        ErrorCollector { errors: Vec::new() }
    }

    /// 添加错误
    pub fn add_error(
        &mut self,
        error: Diagnostic,
    ) {
        self.errors.push(error);
    }

    /// 添加多个错误
    pub fn extend_errors(
        &mut self,
        errors: impl IntoIterator<Item = Diagnostic>,
    ) {
        self.errors.extend(errors);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }
}
