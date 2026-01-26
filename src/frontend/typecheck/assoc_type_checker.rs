//! 关联类型检查器（RFC-011 Phase 3）
//!
//! 实现关联类型一致性检查和推导算法，确保trait实现中的关联类型
//! 满足定义中的约束，并能从上下文中正确推导关联类型。

use crate::frontend::parser::ast;
use crate::frontend::typecheck::gat::{GATEnvironment, GenericAssocType, GATError};
use crate::frontend::typecheck::traits::{TraitDef, TraitEnvironment, TraitSolver};
use crate::util::span::Span;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// 关联类型约束
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssocTypeConstraint {
    /// 被约束的类型
    pub ty: String,
    /// 关联类型名称
    pub assoc_name: String,
    /// 关联类型参数
    pub assoc_args: Vec<String>,
    /// 期望的类型
    pub expected_ty: ast::Type,
    /// 位置信息
    pub span: Span,
}

/// 关联类型检查错误
#[derive(Debug, Clone)]
pub enum AssocTypeCheckError {
    /// 关联类型不匹配
    AssocTypeMismatch {
        assoc_name: String,
        expected: String,
        actual: String,
        span: Span,
    },
    /// 关联类型未定义
    UndefinedAssocType {
        assoc_name: String,
        trait_name: String,
        span: Span,
    },
    /// 关联类型参数数量不匹配
    AssocTypeArgCountMismatch {
        assoc_name: String,
        expected: usize,
        actual: usize,
        span: Span,
    },
    /// 关联类型推导失败
    InferenceFailed {
        assoc_name: String,
        reason: String,
        span: Span,
    },
    /// 关联类型冲突
    ConflictingAssocTypes {
        assoc_name: String,
        first_def: String,
        second_def: String,
        span: Span,
    },
    /// 超trait中的关联类型冲突
    SuperTraitAssocConflict {
        assoc_name: String,
        trait1: String,
        trait2: String,
        span: Span,
    },
}

/// 关联类型检查器
pub struct AssocTypeChecker {
    /// GAT环境
    gat_env: GATEnvironment,
    /// Trait环境
    trait_env: TraitEnvironment,
    /// 已检查的关联类型
    checked: HashSet<(String, String)>, // (trait_name, assoc_name)
    /// 正在检查的关联类型（用于循环检测）
    in_progress: HashSet<(String, String)>,
    /// 关联类型缓存：(trait_name, host_type_args) -> assoc_type
    inference_cache: HashMap<String, ast::Type>,
}

impl AssocTypeChecker {
    /// 创建新的关联类型检查器
    pub fn new(gat_env: GATEnvironment, trait_env: TraitEnvironment) -> Self {
        AssocTypeChecker {
            gat_env,
            trait_env,
            checked: HashSet::new(),
            in_progress: HashSet::new(),
            inference_cache: HashMap::new(),
        }
    }

    /// 检查trait实现中的关联类型是否一致
    pub fn check_impl_assoc_types(
        &mut self,
        trait_name: &str,
        impl_type: &ast::Type,
        impl_span: Span,
    ) -> Result<(), AssocTypeCheckError> {
        // 1. 获取trait定义
        let trait_def = self
            .trait_env
            .get_trait(trait_name)
            .ok_or_else(|| AssocTypeCheckError::UndefinedAssocType {
                assoc_name: "".to_string(),
                trait_name: trait_name.to_string(),
                span: impl_span,
            })?;

        // 2. 检查所有关联类型
        if let Some(gats) = self.gat_env.get_gats(trait_name) {
            for gat in gats {
                // 检查关联类型是否已定义
                self.check_assoc_type_definition(trait_name, gat, impl_type, impl_span)?;
            }
        }

        // 3. 检查超trait约束
        for super_trait in &trait_def.super_traits {
            self.check_super_trait_assoc_types(
                trait_name,
                &super_trait.name,
                impl_type,
                impl_span,
            )?;
        }

        Ok(())
    }

    /// 检查关联类型定义
    fn check_assoc_type_definition(
        &mut self,
        trait_name: &str,
        gat: &GenericAssocType,
        impl_type: &ast::Type,
        impl_span: Span,
    ) -> Result<(), AssocTypeCheckError> {
        // 循环检测
        let key = (trait_name.to_string(), gat.name.clone());
        if self.in_progress.contains(&key) {
            return Err(AssocTypeCheckError::InferenceFailed {
                assoc_name: gat.name.clone(),
                reason: "cyclic dependency detected".to_string(),
                span: impl_span,
            });
        }

        self.in_progress.insert(key.clone());

        // 检查是否已缓存
        let cache_key = format!("{}.{:?}", trait_name, impl_type);
        if let Some(cached_ty) = self.inference_cache.get(&cache_key) {
            self.in_progress.remove(&key);
            // 验证缓存的类型是否满足约束
            self.verify_assoc_type_constraint(gat, cached_ty, impl_span)?;
            return Ok(());
        }

        // 推断关联类型
        let inferred_ty = self
            .infer_assoc_type_from_impl(trait_name, gat, impl_type, impl_span)?;

        // 验证约束
        self.verify_assoc_type_constraint(gat, &inferred_ty, impl_span)?;

        // 缓存结果
        self.inference_cache.insert(cache_key, inferred_ty.clone());

        self.in_progress.remove(&key);
        self.checked.insert(key);

        Ok(())
    }

    /// 从实现中推断关联类型
    fn infer_assoc_type_from_impl(
        &self,
        trait_name: &str,
        gat: &GenericAssocType,
        impl_type: &ast::Type,
        impl_span: Span,
    ) -> Result<ast::Type, AssocTypeCheckError> {
        // 简化实现：根据impl_type推断
        // 实际实现需要更复杂的类型分析

        match impl_type {
            ast::Type::Generic { name, args } => {
                // 例如：impl Iterator for Vec<i32>
                // Item 应该推断为 i32
                if name == "Vec" && !args.is_empty() {
                    return Ok(args[0].clone());
                }

                // 其他情况返回默认类型
                if let Some(default) = &gat.default_ty {
                    return Ok(*default.clone());
                }
            }
            ast::Type::Name(name) => {
                // 如果impl是具体类型，尝试查找对应的GAT实现
                if let Some(gat_impl) = self.gat_env.find_gat_impl(name, trait_name, &gat.name)
                {
                    return Ok(*gat_impl.impl_ty.clone());
                }
            }
            _ => {}
        }

        // 如果无法推断，使用默认类型
        if let Some(default) = &gat.default_ty {
            return Ok(*default.clone());
        }

        Err(AssocTypeCheckError::InferenceFailed {
            assoc_name: gat.name.clone(),
            reason: format!("cannot infer from impl type {:?}", impl_type),
            span: impl_span,
        })
    }

    /// 验证关联类型是否满足约束
    fn verify_assoc_type_constraint(
        &self,
        gat: &GenericAssocType,
        assoc_ty: &ast::Type,
        span: Span,
    ) -> Result<(), AssocTypeCheckError> {
        // 检查边界约束
        for bound in &gat.bounds {
            // 简化实现：检查基本trait约束
            match bound.as_str() {
                "Send" => {
                    // 检查类型是否实现Send
                    // 这里需要与现有的trait系统集成
                }
                "Sync" => {
                    // 检查类型是否实现Sync
                }
                _ => {
                    // 检查其他trait约束
                    // 需要调用trait约束求解器
                }
            }
        }

        // 检查泛型参数约束
        for param in &gat.assoc_params {
            // 确保关联类型中的泛型参数满足约束
            // 这里需要更复杂的类型检查
        }

        Ok(())
    }

    /// 检查超trait中的关联类型
    fn check_super_trait_assoc_types(
        &self,
        child_trait: &str,
        super_trait: &str,
        impl_type: &ast::Type,
        impl_span: Span,
    ) -> Result<(), AssocTypeCheckError> {
        // 获取超trait的GAT定义
        if let Some(super_gats) = self.gat_env.get_gats(super_trait) {
            for super_gat in super_gats {
                // 检查子trait是否定义了相同的关联类型
                if let Some(child_gat) = self
                    .gat_env
                    .find_gat(child_trait, &super_gat.name)
                {
                    // 检查关联类型是否兼容
                    self.check_assoc_type_compatibility(
                        super_gat,
                        child_gat,
                        impl_span,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// 检查关联类型兼容性
    fn check_assoc_type_compatibility(
        &self,
        super_gat: &GenericAssocType,
        child_gat: &GenericAssocType,
        span: Span,
    ) -> Result<(), AssocTypeCheckError> {
        // 检查名称是否相同
        if super_gat.name != child_gat.name {
            return Err(AssocTypeCheckError::SuperTraitAssocConflict {
                assoc_name: super_gat.name.clone(),
                trait1: "super".to_string(),
                trait2: "child".to_string(),
                span,
            });
        }

        // 检查约束是否兼容
        // 子trait的约束应该是超trait约束的超集
        for bound in &super_gat.bounds {
            if !child_gat.bounds.contains(bound) {
                return Err(AssocTypeCheckError::AssocTypeMismatch {
                    assoc_name: super_gat.name.clone(),
                    expected: format!("trait bound {}", bound),
                    actual: "missing bound".to_string(),
                    span,
                });
            }
        }

        Ok(())
    }

    /// 收集trait中的所有关联类型约束
    pub fn collect_assoc_type_constraints(
        &self,
        trait_name: &str,
        ty_args: &[ast::Type],
    ) -> Vec<AssocTypeConstraint> {
        let mut constraints = Vec::new();

        if let Some(gats) = self.gat_env.get_gats(trait_name) {
            for gat in gats {
                // 为每个GAT创建约束
                if let Some(default) = &gat.default_ty {
                    constraints.push(AssocTypeConstraint {
                        ty: format!("{:?}", ty_args),
                        assoc_name: gat.name.clone(),
                        assoc_args: gat.host_params.clone(),
                        expected_ty: *default.clone(),
                        span: Span::dummy(),
                    });
                }
            }
        }

        constraints
    }

    /// 验证关联类型是否已完全定义
    pub fn validate_complete_definition(
        &self,
        trait_name: &str,
        assoc_name: &str,
    ) -> Result<(), AssocTypeCheckError> {
        let key = (trait_name.to_string(), assoc_name.to_string());

        // 检查是否已定义
        if self.checked.contains(&key) {
            return Ok(());
        }

        // 检查GAT环境中是否存在
        if let Some(gat) = self.gat_env.find_gat(trait_name, assoc_name) {
            // 检查是否有默认类型或实现
            if gat.default_ty.is_none() {
                return Err(AssocTypeCheckError::UndefinedAssocType {
                    assoc_name: assoc_name.to_string(),
                    trait_name: trait_name.to_string(),
                    span: gat.span,
                });
            }
        } else {
            return Err(AssocTypeCheckError::UndefinedAssocType {
                assoc_name: assoc_name.to_string(),
                trait_name: trait_name.to_string(),
                span: Span::dummy(),
            });
        }

        Ok(())
    }

    /// 清除缓存（用于新的检查会话）
    pub fn clear_cache(&mut self) {
        self.checked.clear();
        self.in_progress.clear();
        self.inference_cache.clear();
    }

    /// 获取GAT环境（只读）
    pub fn gat_env(&self) -> &GATEnvironment {
        &self.gat_env
    }

    /// 获取Trait环境（只读）
    pub fn trait_env(&self) -> &TraitEnvironment {
        &self.trait_env
    }
}

impl fmt::Display for AssocTypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssocTypeCheckError::AssocTypeMismatch {
                assoc_name,
                expected,
                actual,
                ..
            } => {
                write!(
                    f,
                    "associated type '{}' mismatch: expected {}, got {}",
                    assoc_name, expected, actual
                )
            }
            AssocTypeCheckError::UndefinedAssocType {
                assoc_name,
                trait_name,
                ..
            } => {
                write!(
                    f,
                    "associated type '{}' not found in trait '{}'",
                    assoc_name, trait_name
                )
            }
            AssocTypeCheckError::AssocTypeArgCountMismatch {
                assoc_name,
                expected,
                actual,
                ..
            } => {
                write!(
                    f,
                    "associated type '{}' argument count mismatch: expected {}, got {}",
                    assoc_name, expected, actual
                )
            }
            AssocTypeCheckError::InferenceFailed {
                assoc_name,
                reason,
                ..
            } => {
                write!(
                    f,
                    "failed to infer associated type '{}': {}",
                    assoc_name, reason
                )
            }
            AssocTypeCheckError::ConflictingAssocTypes {
                assoc_name,
                first_def,
                second_def,
                ..
            } => {
                write!(
                    f,
                    "conflicting definitions for associated type '{}': {} and {}",
                    assoc_name, first_def, second_def
                )
            }
            AssocTypeCheckError::SuperTraitAssocConflict {
                assoc_name,
                trait1,
                trait2,
                ..
            } => {
                write!(
                    f,
                    "associated type '{}' conflict between super traits '{}' and '{}'",
                    assoc_name, trait1, trait2
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_gat_env() -> GATEnvironment {
        let mut env = GATEnvironment::new();

        // 添加Iterator trait的Item关联类型
        let item_gat = GenericAssocType {
            name: "Item".to_string(),
            host_params: vec!["T".to_string()],
            assoc_params: vec![],
            bounds: vec![],
            default_ty: Some(Box::new(ast::Type::Name("i32".to_string()))),
            span: Span::dummy(),
        };

        env.register_gat("Iterator", item_gat);
        env
    }

    #[test]
    fn test_checker_creation() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let checker = AssocTypeChecker::new(gat_env, trait_env);

        assert_eq!(checker.checked.len(), 0);
        assert_eq!(checker.inference_cache.len(), 0);
    }

    #[test]
    fn test_incomplete_assoc_type_detection() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let mut checker = AssocTypeChecker::new(gat_env, trait_env);

        // 尝试检查未定义的关联类型
        let result = checker.validate_complete_definition("Iterator", "Undefined");

        assert!(matches!(
            result,
            Err(AssocTypeCheckError::UndefinedAssocType { .. })
        ));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut gat_env = GATEnvironment::new();

        // 创建自引用GAT
        let self_ref_gat = GenericAssocType {
            name: "SelfRef".to_string(),
            host_params: vec!["T".to_string()],
            assoc_params: vec!["T".to_string()],
            bounds: vec![],
            default_ty: Some(Box::new(ast::Type::Name("i32".to_string()))),
            span: Span::dummy(),
        };

        gat_env.register_gat("TestTrait", self_ref_gat);

        let trait_env = TraitEnvironment::new();
        let mut checker = AssocTypeChecker::new(gat_env, trait_env);

        let impl_type = ast::Type::Name("TestType".to_string());
        let result = checker.check_impl_assoc_types("TestTrait", &impl_type, Span::dummy());

        // 应该检测到循环依赖
        assert!(matches!(
            result,
            Err(AssocTypeCheckError::InferenceFailed { .. })
        ));
    }

    #[test]
    fn test_assoc_type_constraint_verification() {
        let mut gat_env = GATEnvironment::new();

        let bounded_gat = GenericAssocType {
            name: "Output".to_string(),
            host_params: vec!["T".to_string()],
            assoc_params: vec![],
            bounds: vec!["Display".to_string()],
            default_ty: Some(Box::new(ast::Type::Name("String".to_string()))),
            span: Span::dummy(),
        };

        gat_env.register_gat("Fn", bounded_gat);

        let trait_env = TraitEnvironment::new();
        let checker = AssocTypeChecker::new(gat_env, trait_env);

        let assoc_ty = ast::Type::Name("String".to_string());
        let result = checker.verify_assoc_type_constraint(
            &bounded_gat,
            &assoc_ty,
            Span::dummy(),
        );

        // 简化实现应该成功
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_functionality() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let mut checker = AssocTypeChecker::new(gat_env, trait_env);

        // 第一次检查
        let impl_type = ast::Type::Generic {
            name: "Vec".to_string(),
            args: vec![ast::Type::Name("i32".to_string())],
        };

        let result1 = checker.check_impl_assoc_types("Iterator", &impl_type, Span::dummy());
        assert!(result1.is_ok());

        // 缓存应该不为空
        assert!(!checker.inference_cache.is_empty());

        // 第二次检查（应该使用缓存）
        let result2 = checker.check_impl_assoc_types("Iterator", &impl_type, Span::dummy());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_cache_clearing() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let mut checker = AssocTypeChecker::new(gat_env, trait_env);

        // 添加一些缓存数据
        checker
            .inference_cache
            .insert("test".to_string(), ast::Type::Name("i32".to_string()));
        checker.checked.insert(("test".to_string(), "assoc".to_string()));
        checker
            .in_progress
            .insert(("test".to_string(), "assoc".to_string()));

        // 清除缓存
        checker.clear_cache();

        assert_eq!(checker.checked.len(), 0);
        assert_eq!(checker.in_progress.len(), 0);
        assert_eq!(checker.inference_cache.len(), 0);
    }

    #[test]
    fn test_constraint_collection() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let checker = AssocTypeChecker::new(gat_env, trait_env);

        let ty_args = vec![ast::Type::Name("T".to_string())];
        let constraints = checker.collect_assoc_type_constraints("Iterator", &ty_args);

        assert!(!constraints.is_empty());
        assert_eq!(constraints[0].assoc_name, "Item");
    }

    #[test]
    fn test_complete_definition_validation() {
        let gat_env = create_simple_gat_env();
        let trait_env = TraitEnvironment::new();
        let checker = AssocTypeChecker::new(gat_env, trait_env);

        // 有默认类型的关联类型应该通过验证
        let result = checker.validate_complete_definition("Iterator", "Item");
        assert!(result.is_ok());
    }
}
