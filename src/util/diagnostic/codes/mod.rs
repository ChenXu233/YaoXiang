//! 错误码注册表
//!
//! 提供所有编译器错误码的集中定义和管理

macro_rules! code_helpers {
    ($(
        $(#[$doc:meta])*
        ($code:expr, $name:ident($($p:ident: $t:ty),*) => $($chain:tt)*)
    ),* $(,)?) => {
        $(
            $(#[$doc])*
            pub fn $name($($p: $t),*) -> DiagnosticBuilder {
                Self::find($code).unwrap().builder() $($chain)*
            }
        )*
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
        !SPAN_EXEMPT.contains(&self.code)
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

/// span 豁免码：纯内部诊断，无用户源码位置可指。新增必须在下方注释理由（#324），
/// 并优先考虑补 span——豁免是最后手段
const SPAN_EXEMPT: &[&str] = &[
    // E8001 ICE：内部一致性问题可能发生在无 AST 对应的任意阶段（如求解器深处）
    "E8001",
    // E4014 编译期常量求值失败：const 表达式经 convert_expr_to_const_expr 进入求值时
    // 已剥离 AST 位置，发射点（ConstGenericEval）无用户位置可指；定位由触发求值的上层诊断承载
    "E4014",
    // E4018 精化谓词违反（证明函数执行场景）：ProofFunctionCall 未携带 span（RFC-027
    // proof 管线无位置传递链）；verdict 的 Disproved 路径带模型 span 时仍照常 .at
    "E4018",
    // E6xxx 运行时错误族：span 来自可选的 --debug-info 帧信息，无 debug-info 时合法无位置
    // （运行时错误的"位置"语义由运行时栈帧呈现，而非编译期诊断 span）
    "E6001", "E6003", "E6004", "E6005", "E6006", "E6007", "E6008", "E6009", "E6010", "E6011",
];

/// 按码字符串判断是否强制要求 span（builder 内部使用，未知码从严要求）
pub fn code_requires_span(code: &str) -> bool {
    !SPAN_EXEMPT.contains(&code)
}

#[cfg(test)]
mod tests;
