//! 错误码注册表
//!
//! 提供所有编译器错误码的集中定义和管理

/// 错误码注册表单源宏（#326）：一个条目同时生成注册表定义与快捷方法，
/// 消灭「statics / code_helpers 双写」漂移（W1005 先例）。
///
/// 条目形态（第三段 = span 豁免布尔；链与既有 code_helpers 同款）。
/// 条目上方的 /// 注释仅作源码阅读（宏 token 化即丢弃）；用户可读文档的唯一权威在
/// locales + `yaoxiang explain`，不在方法 doc。
///
/// ```ignore
/// define_codes!(E1XXX, {
///     /// E1001 未知变量
///     ("E1001", TypeCheck, false, unknown_variable(name: &str) => .param("name", name)),
///     /// E8001 ICE（span_exempt=true：无用户位置可指的豁免，须在该条目注释理由）
///     ("E8001", Internal, true, internal_error(message: &str) => .param("message", message)),
/// });
/// ```
macro_rules! define_codes {
    ($statics:ident, {
        $(
            ($code:expr, $cat:ident, $exempt:expr, $name:ident($($p:ident: $t:ty),*) => $($chain:tt)*)
        ),* $(,)?
    }) => {
        pub static $statics: &[ErrorCodeDefinition] = &[
            $(
                ErrorCodeDefinition {
                    code: $code,
                    category: ErrorCategory::$cat,
                    span_exempt: $exempt,
                },
            )*
        ];
        impl ErrorCodeDefinition {
            $(
                pub fn $name($($p: $t),*) -> DiagnosticBuilder {
                    Self::find($code).unwrap().builder() $($chain)*
                }
            )*
        }
    };
}

pub mod e0xxx;
pub mod e1xxx;
pub mod e2xxx;
pub mod e3xxx;
pub mod e4xxx;
pub mod e5xxx;
pub mod e6xxx;
pub mod e7xxx;
pub mod e8xxx;
pub mod w1xxx;

pub use e0xxx::*;
pub use e1xxx::*;
pub use e2xxx::*;
pub use e3xxx::*;
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
    Codegen,   // E3xxx: 代码生成
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
            ErrorCategory::Codegen => write!(f, "Codegen"),
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
    /// span 豁免（#324）：无用户源码位置可指的内部/运行时诊断。
    /// 由 define_codes! 条目的 no_span 标记填充；新增豁免必须在该码条目处注释理由
    pub span_exempt: bool,
}

use std::sync::LazyLock;

/// 完整的错误码注册表
static ERROR_CODES: LazyLock<Vec<ErrorCodeDefinition>> = LazyLock::new(|| {
    let mut codes: Vec<ErrorCodeDefinition> = Vec::new();

    // E0xxx: 词法和语法分析
    codes.extend_from_slice(e0xxx::E0XXX);
    // E1xxx: 类型检查
    codes.extend_from_slice(e1xxx::E1XXX);
    // E2xxx: 语义分析
    codes.extend_from_slice(e2xxx::E2XXX);
    // E3xxx: 代码生成
    codes.extend_from_slice(e3xxx::E3XXX);
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

    /// 该码是否强制要求 span（#324）：豁免表之外一律要求，
    /// 构造时既无显式 .at() 又无 walk 上下文即拒绝（debug panic / release E8001）
    pub fn requires_span(&self) -> bool {
        !self.span_exempt
    }

    /// 创建 DiagnosticBuilder
    pub fn builder(&self) -> DiagnosticBuilder {
        DiagnosticBuilder::new(self.code)
    }

    /// 使用 error_lang() 自动获取语言构建 Diagnostic
    pub fn build(&self) -> Diagnostic {
        self.builder().build()
    }
}

/// 按码字符串判断是否强制要求 span（builder 内部使用，未知码从严要求）
pub fn code_requires_span(code: &str) -> bool {
    ErrorCodeDefinition::find(code)
        .map(|d| d.requires_span())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests;
