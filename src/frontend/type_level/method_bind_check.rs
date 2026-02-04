//! RFC-011 Trait 方法绑定检查
//!
//! 实现 RFC-011 方法绑定的验证：
//! - 验证方法签名符合 trait 定义
//! - 验证必需方法已实现
//!
//! RFC-011 风格示例：
//! ```yaoxiang
//! # 标准库定义 Clone trait
//! type Clone = {
//!     clone: (self: Self) -> Self,
//! }
//!
//! # 用户为 Point 实现 Clone 方法
//! Point.clone: (self: Point) -> Point = {
//!     return Point(self.x, self.y)
//! }
//! ```

use crate::frontend::core::parser::ast::StmtKind;
use crate::frontend::type_level::trait_bounds::{TraitTable, TraitDefinition};

/// Trait 方法绑定检查器
///
/// 检查方法绑定是否符合 trait 定义
#[derive(Debug)]
pub struct TraitImplChecker<'a> {
    /// Trait 表
    trait_table: &'a TraitTable,
}

impl<'a> TraitImplChecker<'a> {
    /// 创建新的检查器
    pub fn new(trait_table: &'a TraitTable) -> Self {
        Self { trait_table }
    }

    /// 检查方法绑定是否符合 trait 定义
    ///
    /// 返回 Ok(()) 表示检查通过
    pub fn check_method_bind(
        &self,
        method_bind: &StmtKind,
    ) -> Result<(), TraitImplError> {
        // 提取 MethodBind 数据
        let (method_name, params) = match method_bind {
            StmtKind::MethodBind {
                type_name: _,
                method_name,
                method_type: _,
                params,
                body: _,
            } => (method_name, params),
            _ => return Err(TraitImplError::InvalidMethodBind),
        };

        // 1. 检查 trait 是否已定义
        let trait_def = match self.trait_table.get_trait(method_name) {
            Some(def) => def,
            None => {
                return Err(TraitImplError::TraitNotFound {
                    trait_name: method_name.clone(),
                });
            }
        };

        // 2. 检查必需方法是否都实现了（简化：只检查当前绑定的方法）
        self.check_required_methods_simple(trait_def, method_name)?;

        // 3. 检查方法签名是否兼容
        self.check_signature(trait_def, params)?;

        Ok(())
    }

    /// 检查必需方法是否都已实现（简化版本）
    fn check_required_methods_simple(
        &self,
        trait_def: &TraitDefinition,
        method_name: &str,
    ) -> Result<(), TraitImplError> {
        // 检查该方法是否是 trait 的必需方法
        if !trait_def.methods.contains_key(method_name) {
            return Err(TraitImplError::NotRequiredMethod {
                trait_name: trait_def.name.clone(),
                method_name: method_name.to_string(),
            });
        }

        Ok(())
    }

    /// 检查方法签名是否兼容
    fn check_signature(
        &self,
        _trait_def: &TraitDefinition,
        _params: &[crate::frontend::core::parser::ast::Param],
    ) -> Result<(), TraitImplError> {
        // 从 trait 定义获取预期签名
        // 这里简化处理，实际应该从 method_type 解析
        Ok(())
    }
}

/// Trait 实现错误
#[derive(Debug, Clone)]
pub enum TraitImplError {
    /// Trait 未定义
    TraitNotFound { trait_name: String },
    /// 缺失必需方法
    MissingMethod {
        trait_name: String,
        methods: Vec<String>,
    },
    /// 方法签名不匹配
    SignatureMismatch {
        method_name: String,
        message: String,
    },
    /// 不是必需方法
    NotRequiredMethod {
        trait_name: String,
        method_name: String,
    },
    /// 无效的方法绑定
    InvalidMethodBind,
}

impl std::fmt::Display for TraitImplError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::TraitNotFound { trait_name } => {
                write!(f, "Trait not found: `{}`", trait_name)
            }
            Self::MissingMethod {
                trait_name,
                methods,
            } => {
                write!(
                    f,
                    "Missing method(s) `{}` in implementation of trait `{}`",
                    methods.join(", "),
                    trait_name
                )
            }
            Self::SignatureMismatch {
                method_name,
                message,
            } => {
                write!(f, "Signature mismatch for `{}`: {}", method_name, message)
            }
            Self::NotRequiredMethod {
                trait_name,
                method_name,
            } => {
                write!(
                    f,
                    "Method `{}` is not required by trait `{}`",
                    method_name, trait_name
                )
            }
            Self::InvalidMethodBind => {
                write!(f, "Invalid method bind")
            }
        }
    }
}

impl std::error::Error for TraitImplError {}
