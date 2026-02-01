//! 诊断特质
//!
//! 定义诊断相关的特质

use crate::frontend::shared::error::diagnostic::DiagnosticLevel;

/// 诊断特质
pub trait DiagnosticTrait {
    /// 获取诊断级别
    fn level(&self) -> DiagnosticLevel;

    /// 获取诊断消息
    fn message(&self) -> &str;

    /// 获取源码位置
    fn span(&self) -> Option<&crate::util::span::Span>;

    /// 转换为诊断信息
    fn to_diagnostic(&self) -> crate::frontend::shared::error::diagnostic::Diagnostic {
        crate::frontend::shared::error::diagnostic::Diagnostic {
            level: self.level(),
            message: self.message().to_string(),
            span: self.span().cloned(),
        }
    }
}
