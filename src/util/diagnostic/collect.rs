//! 错误收集和格式化
//!
//! 提供通用的错误收集器和格式化工具

use crate::util::span::Span;
use super::span::SpannedError;

/// 错误收集器
///
/// 收集多个错误，支持批量报告
#[derive(Debug, Default)]
pub struct ErrorCollector<E: SpannedError> {
    /// 错误列表
    errors: Vec<E>,
    /// 警告列表
    warnings: Vec<Warning>,
}

impl<E: SpannedError> ErrorCollector<E> {
    /// 创建新的错误收集器
    pub fn new() -> Self {
        ErrorCollector {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// 添加错误
    pub fn add_error(
        &mut self,
        error: E,
    ) {
        self.errors.push(error);
    }

    /// 添加警告
    pub fn add_warning(
        &mut self,
        warning: Warning,
    ) {
        self.warnings.push(warning);
    }

    /// 添加多个错误
    pub fn extend_errors(
        &mut self,
        errors: impl IntoIterator<Item = E>,
    ) {
        self.errors.extend(errors);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 检查是否有警告
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 获取警告数量
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[E] {
        &self.errors
    }

    /// 获取所有警告
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// 消耗收集器，获取所有错误
    pub fn into_errors(self) -> Vec<E> {
        self.errors
    }

    /// 消耗收集器，获取所有警告
    pub fn into_warnings(self) -> Vec<Warning> {
        self.warnings
    }

    /// 清空所有错误
    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }
}

/// 警告
#[derive(Debug, Clone)]
pub enum Warning {
    /// 未使用的变量
    UnusedVariable { name: String, span: Span },
    /// 未使用的导入
    UnusedImport { path: String, span: Span },
    /// 类型推断可能不准确
    ImpreciseInference { span: Span },
    /// 可能的空指针解引用
    PotentialNullDereference { span: Span },
}

impl std::fmt::Display for Warning {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Warning::UnusedVariable { name, .. } => write!(f, "Unused variable: {}", name),
            Warning::UnusedImport { path, .. } => write!(f, "Unused import: {}", path),
            Warning::ImpreciseInference { .. } => write!(f, "Type inference may be imprecise"),
            Warning::PotentialNullDereference { .. } => write!(f, "Potential null dereference"),
        }
    }
}

/// 错误格式化器
#[derive(Debug)]
pub struct ErrorFormatter {
    /// 是否显示详细信息
    verbose: bool,
}

impl ErrorFormatter {
    /// 创建新的错误格式化器
    pub fn new(verbose: bool) -> Self {
        ErrorFormatter { verbose }
    }

    /// 格式化警告
    pub fn format_warning(
        &self,
        warning: &Warning,
    ) -> String {
        match warning {
            Warning::UnusedVariable { name, span } => {
                if self.verbose {
                    format!("Unused variable '{}' at {:?}", name, span)
                } else {
                    format!("Unused variable: {}", name)
                }
            }
            Warning::UnusedImport { path, span } => {
                if self.verbose {
                    format!("Unused import '{}' at {:?}", path, span)
                } else {
                    format!("Unused import: {}", path)
                }
            }
            Warning::ImpreciseInference { span } => {
                if self.verbose {
                    format!("Type inference may be imprecise at {:?}", span)
                } else {
                    "Type inference may be imprecise".to_string()
                }
            }
            Warning::PotentialNullDereference { span } => {
                if self.verbose {
                    format!("Potential null dereference at {:?}", span)
                } else {
                    "Potential null dereference".to_string()
                }
            }
        }
    }

    /// 格式化所有警告
    pub fn format_warnings(
        &self,
        warnings: &[Warning],
    ) -> Vec<String> {
        warnings.iter().map(|w| self.format_warning(w)).collect()
    }
}
