#![allow(clippy::result_large_err)]

//! GAT 检查器
//!
//! 实现 Generic Associated Types 检查

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::MonoType;

/// GAT 检查器
pub struct GATChecker;

impl Default for GATChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl GATChecker {
    /// 创建新的 GAT 检查器
    pub fn new() -> Self {
        Self
    }

    /// 检查 GAT 声明
    pub fn check_gat(
        &self,
        ty: &MonoType,
    ) -> Result<()> {
        // 检查类型是否包含泛型关联类型
        match ty {
            MonoType::Struct(struct_type) => {
                // 检查结构体字段中的 GAT
                for (field_name, field_type) in &struct_type.fields {
                    self.check_field_gat(field_name, field_type)?;
                }
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                // 检查函数类型中的 GAT
                for param_type in params {
                    self.check_type_gat(param_type)?;
                }
                self.check_type_gat(return_type)?;
            }
            _ => {
                // 基本类型不需要 GAT 检查
            }
        }

        Ok(())
    }

    /// 检查字段中的 GAT
    fn check_field_gat(
        &self,
        _field_name: &str,
        field_type: &MonoType,
    ) -> Result<()> {
        // 检查字段类型是否包含泛型参数
        if self.contains_generic_params(field_type) {
            // 验证泛型参数的使用是否合法
            self.validate_generic_usage(field_type)?;
        }

        Ok(())
    }

    /// 检查类型中的 GAT
    #[allow(clippy::only_used_in_recursion)]
    fn check_type_gat(
        &self,
        ty: &MonoType,
    ) -> Result<()> {
        match ty {
            MonoType::List(inner) => self.check_type_gat(inner)?,
            MonoType::Tuple(types) => {
                for ty in types {
                    self.check_type_gat(ty)?;
                }
            }
            MonoType::Struct(struct_type) => {
                for (_, field_type) in &struct_type.fields {
                    self.check_type_gat(field_type)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// 检查是否包含泛型参数
    #[allow(clippy::only_used_in_recursion)]
    pub fn contains_generic_params(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            MonoType::TypeVar(_) => true,
            MonoType::List(inner) => self.contains_generic_params(inner),
            MonoType::Tuple(types) => types.iter().any(|t| self.contains_generic_params(t)),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|p| self.contains_generic_params(p))
                    || self.contains_generic_params(return_type)
            }
            MonoType::Struct(struct_type) => struct_type
                .fields
                .iter()
                .any(|(_, t)| self.contains_generic_params(t)),
            _ => false,
        }
    }

    /// 验证泛型使用
    fn validate_generic_usage(
        &self,
        _ty: &MonoType,
    ) -> Result<()> {
        // 简化实现：验证泛型参数的使用是否合法
        // 在实际实现中，这里会检查更复杂的约束

        Ok(())
    }

    /// 检查 GAT 关联类型
    pub fn check_associated_type(
        &self,
        container: &str,
        assoc_type: &str,
    ) -> Result<()> {
        // 检查关联类型是否在容器中定义
        if !self.is_associated_type_defined(container, assoc_type) {
            return Err(crate::util::diagnostic::Diagnostic::error(
                "E0801".to_string(),
                format!(
                    "Associated type {} not found in container {}",
                    assoc_type, container
                ),
                None,
            ));
        }

        // 检查关联类型的约束
        self.check_associated_type_constraints(container, assoc_type)?;

        // 检查关联类型的泛型参数
        self.check_associated_type_generics(container, assoc_type)?;

        Ok(())
    }

    /// 检查关联类型是否定义
    pub fn is_associated_type_defined(
        &self,
        container: &str,
        assoc_type: &str,
    ) -> bool {
        // 简化实现：检查已知的关联类型
        match container {
            "Iterator" => assoc_type == "Item",
            "IntoIterator" => assoc_type == "Item",
            "Clone" => false, // Clone 没有关联类型
            "Debug" => false, // Debug 没有关联类型
            _ => false,
        }
    }

    /// 检查关联类型约束
    pub fn check_associated_type_constraints(
        &self,
        _container: &str,
        _assoc_type: &str,
    ) -> Result<()> {
        // 检查关联类型是否有任何约束
        // 简化实现：假设没有额外约束

        Ok(())
    }

    /// 检查关联类型泛型参数
    pub fn check_associated_type_generics(
        &self,
        _container: &str,
        _assoc_type: &str,
    ) -> Result<()> {
        // 检查关联类型是否使用正确的泛型参数
        // 简化实现：假设泛型参数使用正确

        Ok(())
    }

    /// 解析关联类型路径
    pub fn resolve_associated_type(
        &self,
        container: &str,
        assoc_type: &str,
    ) -> Option<String> {
        // 解析关联类型的完整路径
        if self.is_associated_type_defined(container, assoc_type) {
            Some(format!("{}::{}", container, assoc_type))
        } else {
            None
        }
    }
}
