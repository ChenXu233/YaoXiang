//! 错误码注册表
//!
//! 提供所有编译器错误码的集中定义和管理

pub mod e0xxx;
pub mod e1xxx;
pub mod e2xxx;
pub mod e4xxx;
pub mod e5xxx;
pub mod e6xxx;
pub mod e7xxx;
pub mod e8xxx;
pub mod w1xxx;

pub use e0xxx::*;
pub use e1xxx::*;
pub use e2xxx::*;
pub use e4xxx::*;
pub use e5xxx::*;
pub use e6xxx::*;
pub use e7xxx::*;
pub use e8xxx::*;
pub use w1xxx::*;

pub mod builder;
pub use builder::{DiagnosticBuilder, I18nRegistry, ErrorInfo};

use crate::util::diagnostic::Diagnostic;

/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,     // E0xxx: 词法分析
    Parser,    // E0xxx: 语法分析
    TypeCheck, // E1xxx: 类型检查
    Semantic,  // E2xxx: 语义分析
    Generic,   // E4xxx: 泛型与特质
    Module,    // E5xxx: 模块与导入
    Runtime,   // E6xxx: 运行时错误
    Io,        // E7xxx: I/O与系统错误
    Internal,  // E8xxx: 内部编译器错误
    Warning,   // W1xxx: 警告（死代码等）
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ErrorCategory::Lexer => write!(f, "Lexer"),
            ErrorCategory::Parser => write!(f, "Parser"),
            ErrorCategory::TypeCheck => write!(f, "Type Check"),
            ErrorCategory::Semantic => write!(f, "Semantic"),
            ErrorCategory::Generic => write!(f, "Generic"),
            ErrorCategory::Module => write!(f, "Module"),
            ErrorCategory::Runtime => write!(f, "Runtime"),
            ErrorCategory::Io => write!(f, "I/O"),
            ErrorCategory::Internal => write!(f, "Internal"),
            ErrorCategory::Warning => write!(f, "Warning"),
        }
    }
}

/// 错误码定义（仅元数据，展示文案在 i18n 文件）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    /// 错误码，如 "E1001"
    pub code: &'static str,
    /// 错误类别
    pub category: ErrorCategory,
    /// 消息模板，支持 {param} 占位符
    pub message_template: &'static str,
}

use once_cell::sync::Lazy;

/// 完整的错误码注册表
static ERROR_CODES: Lazy<Vec<ErrorCodeDefinition>> = Lazy::new(|| {
    let mut codes: Vec<ErrorCodeDefinition> = Vec::new();

    // E0xxx: 词法和语法分析
    codes.extend_from_slice(e0xxx::E0XXX);
    // E1xxx: 类型检查
    codes.extend_from_slice(e1xxx::E1XXX);
    // E2xxx: 语义分析
    codes.extend_from_slice(e2xxx::E2XXX);
    // E4xxx: 泛型与特质
    codes.extend_from_slice(e4xxx::E4XXX);
    // E5xxx: 模块与导入
    codes.extend_from_slice(e5xxx::E5XXX);
    // E6xxx: 运行时错误
    codes.extend_from_slice(e6xxx::E6XXX);
    // E7xxx: I/O与系统错误
    codes.extend_from_slice(e7xxx::E7XXX);
    // E8xxx: 内部编译器错误
    codes.extend_from_slice(e8xxx::E8XXX);
    // W1xxx: 警告（死代码等）
    codes.extend_from_slice(w1xxx::W1XXX);

    codes
});

impl ErrorCodeDefinition {
    /// 根据代码查找错误码定义
    pub fn find(code: &str) -> Option<&'static Self> {
        ERROR_CODES.iter().find(|c| c.code == code)
    }

    /// 获取所有错误码
    pub fn all() -> &'static [Self] {
        &ERROR_CODES
    }

    /// 按类别获取错误码
    pub fn by_category(category: ErrorCategory) -> impl Iterator<Item = &'static Self> {
        ERROR_CODES.iter().filter(move |c| c.category == category)
    }

    /// 创建 DiagnosticBuilder
    pub fn builder(&self) -> DiagnosticBuilder {
        DiagnosticBuilder::new(self.code, self.message_template)
    }

    /// 使用 error_lang() 自动获取语言构建 Diagnostic
    pub fn build(&self) -> Diagnostic {
        self.builder().build()
    }
}

#[cfg(test)]
mod tests;
