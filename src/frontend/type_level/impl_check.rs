//! Trait 实现检查
//!
//! 实现 RFC-011 Trait 实现验证：
//! - 验证实现包含所有必需方法
//! - 验证方法签名兼容
//! - 报错信息指出缺失的方法

use std::collections::{HashMap, HashSet};
use crate::frontend::core::parser::ast::{TraitImpl, MethodImpl};
use crate::frontend::type_level::trait_bounds::{TraitTable, TraitDefinition};

/// Trait 实现检查器
#[derive(Debug)]
pub struct TraitImplChecker<'a> {
    /// Trait 表
    trait_table: &'a TraitTable,
    /// 收集的所有必需方法
    all_required_methods: HashMap<String, HashSet<String>>,
}

impl<'a> TraitImplChecker<'a> {
    /// 创建新的实现检查器
    pub fn new(trait_table: &'a TraitTable) -> Self {
        Self {
            trait_table,
            all_required_methods: HashMap::new(),
        }
    }

    /// 检查 Trait 实现是否正确
    pub fn check_impl(
        &mut self,
        impl_: &TraitImpl,
    ) -> Result<(), TraitImplError> {
        // 1. 获取 Trait 定义
        let trait_def = match self.trait_table.get_trait(&impl_.trait_name) {
            Some(def) => def,
            None => {
                return Err(TraitImplError::TraitNotFound {
                    trait_name: impl_.trait_name.clone(),
                });
            }
        };

        // 2. 收集所有必需方法（包括从父 Trait 继承的）
        let required_methods = self.collect_all_required_methods(&impl_.trait_name);

        // 3. 检查是否实现了所有必需方法
        self.check_required_methods(&impl_.trait_name, &required_methods, &impl_.methods)?;

        // 4. 检查方法签名是否匹配
        self.check_method_signatures(trait_def, &impl_.methods)?;

        // 5. 检查 coherence（孤儿规则）- 简化版本
        self.check_coherence(&impl_.trait_name, &impl_.for_type)?;

        Ok(())
    }

    /// 收集所有必需方法（包括从父 Trait 继承的）
    fn collect_all_required_methods(
        &mut self,
        trait_name: &str,
    ) -> HashSet<String> {
        if let Some(cached) = self.all_required_methods.get(trait_name) {
            return cached.clone();
        }

        let mut methods = HashSet::new();

        // 递归收集父 Trait 的方法
        if let Some(def) = self.trait_table.get_trait(trait_name) {
            // 收集父 Trait 的方法
            for parent_name in &def.parent_traits {
                let parent_methods = self.collect_all_required_methods(parent_name);
                methods.extend(parent_methods);
            }

            // 添加当前 Trait 的方法
            for method_name in def.methods.keys() {
                methods.insert(method_name.clone());
            }
        }

        self.all_required_methods
            .insert(trait_name.to_string(), methods.clone());
        methods
    }

    /// 检查必需方法
    fn check_required_methods(
        &self,
        trait_name: &str,
        required_methods: &HashSet<String>,
        impl_methods: &[MethodImpl],
    ) -> Result<(), TraitImplError> {
        let impl_method_names: HashSet<_> = impl_methods.iter().map(|m| m.name.clone()).collect();

        let missing: Vec<_> = required_methods
            .iter()
            .filter(|name| !impl_method_names.contains(*name))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(TraitImplError::MissingMethod {
                trait_name: trait_name.to_string(),
                methods: missing,
            });
        }

        Ok(())
    }

    /// 检查方法签名
    fn check_method_signatures(
        &self,
        trait_def: &TraitDefinition,
        impl_methods: &[MethodImpl],
    ) -> Result<(), TraitImplError> {
        for impl_method in impl_methods {
            if let Some(expected_sig) = trait_def.methods.get(&impl_method.name) {
                // 比较参数数量
                let expected_params = expected_sig.params.len();

                // 注意：impl 方法可能有 self 参数，需要调整比较
                // 简化实现：只检查非 self 参数数量
                let actual_non_self = impl_method
                    .params
                    .iter()
                    .filter(|p| p.name != "self" && p.name != "Self")
                    .count();

                // 预期参数 = Self + 其他参数
                // 如果 trait 方法有 self，预期 impl 也有 self
                let expected_has_self = expected_params > 0
                    && impl_method
                        .params
                        .first()
                        .map(|p| p.name == "self" || p.name == "Self")
                        .unwrap_or(false);

                if expected_has_self && actual_non_self != expected_params.saturating_sub(1) {
                    return Err(TraitImplError::SignatureMismatch {
                        method_name: impl_method.name.clone(),
                        message: format!(
                            "Expected {} parameters, found {}",
                            expected_params.saturating_sub(1),
                            actual_non_self
                        ),
                    });
                } else if !expected_has_self && actual_non_self != expected_params {
                    return Err(TraitImplError::SignatureMismatch {
                        method_name: impl_method.name.clone(),
                        message: format!(
                            "Expected {} parameters, found {}",
                            expected_params, actual_non_self
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// 检查 coherence（孤儿规则）- 简化版本
    /// 检查是否已存在该实现
    fn check_coherence(
        &self,
        trait_name: &str,
        for_type: &crate::frontend::core::parser::ast::Type,
    ) -> Result<(), TraitImplError> {
        let type_name = extract_type_name(for_type);

        if self.trait_table.has_impl(trait_name, &type_name) {
            return Err(TraitImplError::ConflictingImpl {
                trait_name: trait_name.to_string(),
                type_name,
            });
        }

        Ok(())
    }
}

/// 从 AST Type 提取类型名称
fn extract_type_name(ty: &crate::frontend::core::parser::ast::Type) -> String {
    match ty {
        crate::frontend::core::parser::ast::Type::Name(n) => n.clone(),
        crate::frontend::core::parser::ast::Type::Generic { name, .. } => name.clone(),
        _ => "unknown".to_string(),
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
    /// 冲突实现（重复实现）
    ConflictingImpl {
        trait_name: String,
        type_name: String,
    },
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
            Self::ConflictingImpl {
                trait_name,
                type_name,
            } => {
                write!(
                    f,
                    "Conflicting implementation of trait `{}` for type `{}`",
                    trait_name, type_name
                )
            }
        }
    }
}

impl std::error::Error for TraitImplError {}
