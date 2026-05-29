#![allow(clippy::result_large_err)]

//! 特质求解器
//!
//! 实现特质约束求解，集成 RFC-011 Trait 系统：
//! - 内置特质 (Clone, Debug, Send, Sync)
//! - 用户定义特质
//! - 约束传播

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::base::TraitTable;
use std::collections::{HashMap, HashSet};

/// 特质约束
#[derive(Debug, Clone)]
pub struct TraitConstraint {
    pub name: String,
    pub args: Vec<MonoType>,
}

/// 特质求解器
pub struct TraitSolver {
    /// 简化的约束存储
    simple_constraints: HashMap<String, TraitConstraint>,
    /// 已求解的边界缓存: "trait_name:type_name"
    solved_cache: HashSet<String>,
    /// Trait 表引用
    trait_table: Option<TraitTable>,
}

impl Default for TraitSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TraitSolver {
    /// 创建新的特质求解器
    pub fn new() -> Self {
        Self {
            simple_constraints: HashMap::new(),
            solved_cache: HashSet::new(),
            trait_table: None,
        }
    }

    /// 设置 Trait 表
    pub fn set_trait_table(
        &mut self,
        table: TraitTable,
    ) {
        self.trait_table = Some(table);
    }

    /// 获取 Trait 表引用
    pub fn trait_table(&self) -> Option<&TraitTable> {
        self.trait_table.as_ref()
    }

    /// 求解特质约束
    pub fn solve(
        &mut self,
        constraint: &TraitConstraint,
    ) -> Result<()> {
        // 检查是否已解决
        if self.simple_constraints.contains_key(&constraint.name) {
            return Ok(());
        }

        let cache_key = constraint
            .args
            .first()
            .map(|ty| format!("{}:{}", constraint.name, ty.type_name()))
            .unwrap_or_default();

        if self.solved_cache.contains(&cache_key) {
            return Ok(());
        }

        // 尝试使用用户定义的 Trait
        if let Some(ref table) = self.trait_table {
            if table.has_trait(&constraint.name) {
                let type_name = constraint
                    .args
                    .first()
                    .map(|t| t.type_name())
                    .unwrap_or_default();

                if !table.has_impl(&constraint.name, &type_name) {
                    return Err(ErrorCodeDefinition::trait_bound_not_satisfied(
                        &type_name,
                        &constraint.name,
                    )
                    .build());
                }

                self.solved_cache.insert(cache_key);
                self.simple_constraints
                    .insert(constraint.name.clone(), constraint.clone());
                return Ok(());
            }
        }

        // 内置 Trait
        self.solve_builtin_trait(constraint)
    }

    /// 求解内置特质约束
    fn solve_builtin_trait(
        &mut self,
        constraint: &TraitConstraint,
    ) -> Result<()> {
        if self.can_satisfy_constraint(constraint) {
            self.simple_constraints
                .insert(constraint.name.clone(), constraint.clone());
            Ok(())
        } else {
            let type_name = constraint
                .args
                .first()
                .map(|t| format!("{}", t))
                .unwrap_or_default();
            Err(
                ErrorCodeDefinition::trait_bound_not_satisfied(&type_name, &constraint.name)
                    .build(),
            )
        }
    }

    /// 检查是否可以满足约束
    fn can_satisfy_constraint(
        &self,
        constraint: &TraitConstraint,
    ) -> bool {
        if let Some(ty) = constraint.args.first() {
            match constraint.name.as_str() {
                "Clone" => self.check_clone_trait(ty),
                "Dup" => self.check_dup_trait(ty),
                "Equal" => self.check_equal_trait(ty),
                "Debug" => self.check_debug_trait(ty),
                "Send" => self.check_send_trait(ty),
                "Sync" => self.check_sync_trait(ty),
                _ => false,
            }
        } else {
            false
        }
    }

    /// 检查 Clone 特质（递归检查结构体字段、枚举变体、元组元素）
    ///
    /// 首先检查 TraitTable 是否有已注册的 Clone 实现，
    /// 然后对结构体字段、元组元素进行递归检查。
    /// 枚举变体仅包含名称（无关联数据），自动视为 Clone。
    fn check_clone_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        // 检查 TraitTable 是否有已注册的 Clone 实现
        if let Some(ref table) = self.trait_table {
            if table.has_trait("Clone") && table.has_impl("Clone", &ty.type_name()) {
                return true;
            }
        }

        match ty {
            // 基本类型：自动 Clone
            MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Bool
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes
            | MonoType::Void => true,

            // Arc：引用计数，自动 Clone
            MonoType::Arc(_) => true,

            // 元组：递归检查所有元素
            MonoType::Tuple(elems) => elems.iter().all(|t| self.check_clone_trait(t)),

            // 结构体：递归检查所有字段
            MonoType::Struct(s) => s
                .fields
                .iter()
                .all(|(_, field_ty)| self.check_clone_trait(field_ty)),

            // 枚举：变体仅包含名称（无关联数据），自动视为 Clone
            MonoType::Enum(_) => true,

            // 其他类型（Fn、Weak 等）：不满足 Clone
            _ => false,
        }
    }

    /// 检查 Dup 特质（递归检查结构体字段、枚举变体、元组元素）
    ///
    /// Dup 表示类型可以被简单复制（类似 Copy trait）。
    /// 与 Clone 不同，Dup 不需要显式实现，仅依赖结构递归。
    fn check_dup_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型：自动 Dup
            MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Bool
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes
            | MonoType::Void => true,

            // Arc：引用计数，自动 Dup
            MonoType::Arc(_) => true,

            // 元组：递归检查所有元素
            MonoType::Tuple(elems) => elems.iter().all(|t| self.check_dup_trait(t)),

            // 结构体：递归检查所有字段
            MonoType::Struct(s) => s
                .fields
                .iter()
                .all(|(_, field_ty)| self.check_dup_trait(field_ty)),

            // 枚举：变体仅包含名称（无关联数据），自动视为 Dup
            MonoType::Enum(_) => true,

            // 其他类型（Fn、Weak、&mut T 等）：不满足 Dup
            _ => false,
        }
    }

    /// 检查 Equal 特质
    fn check_equal_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
                | MonoType::Void
        )
    }

    /// 检查 Debug 特质
    fn check_debug_trait(
        &self,
        _ty: &MonoType,
    ) -> bool {
        true
    }

    /// 检查 Send 特质
    fn check_send_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
                | MonoType::Void
        )
    }

    /// 检查 Sync 特质
    fn check_sync_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
                | MonoType::Void
        )
    }

    /// 检查特质是否满足（支持用户定义 Trait）
    pub fn check_trait(
        &mut self,
        ty: &MonoType,
        trait_name: &str,
    ) -> bool {
        if let Some(ref table) = self.trait_table {
            if table.has_trait(trait_name) {
                return table.has_impl(trait_name, &ty.type_name());
            }
        }

        match trait_name {
            "Clone" => self.check_clone_trait(ty),
            "Dup" => self.check_dup_trait(ty),
            "Equal" => self.check_equal_trait(ty),
            "Debug" => self.check_debug_trait(ty),
            "Send" => self.check_send_trait(ty),
            "Sync" => self.check_sync_trait(ty),
            _ => false,
        }
    }

    /// 获取所有未满足的约束
    pub fn unsatisfied_constraints(&self) -> Vec<&TraitConstraint> {
        self.simple_constraints
            .values()
            .filter(|c| !self.simple_constraints.contains_key(&c.name))
            .collect()
    }

    /// 批量求解约束
    pub fn solve_all(
        &mut self,
        constraints: &[TraitConstraint],
    ) -> Result<()> {
        for constraint in constraints {
            self.solve(constraint)?;
        }
        Ok(())
    }

    /// 传播约束到类型参数
    pub fn propagate_constraints_to_type_args(
        &self,
        _ty: &MonoType,
        _trait_name: &str,
    ) -> Vec<TraitConstraint> {
        Vec::new()
    }
}

/// Trait 求解错误
#[derive(Debug, Clone)]
pub enum TraitSolverError {
    UndefinedTrait {
        trait_name: String,
    },
    MissingImpl {
        trait_name: String,
        type_name: String,
    },
    UnsatisfiedConstraint {
        trait_name: String,
        type_name: String,
    },
    CyclicInheritance {
        trait_name: String,
    },
    MethodNotFound {
        trait_name: String,
        method_name: String,
    },
}

impl std::fmt::Display for TraitSolverError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::UndefinedTrait { trait_name } => {
                write!(f, "Undefined trait: `{}`", trait_name)
            }
            Self::MissingImpl {
                trait_name,
                type_name,
            } => {
                write!(
                    f,
                    "Type `{}` does not implement trait `{}`",
                    type_name, trait_name
                )
            }
            Self::UnsatisfiedConstraint {
                trait_name,
                type_name,
            } => {
                write!(
                    f,
                    "Constraint `{}: {}` is not satisfied",
                    type_name, trait_name
                )
            }
            Self::CyclicInheritance { trait_name } => {
                write!(f, "Cyclic inheritance detected in trait: {}", trait_name)
            }
            Self::MethodNotFound {
                trait_name,
                method_name,
            } => {
                write!(
                    f,
                    "Method `{}` not found in trait `{}`",
                    method_name, trait_name
                )
            }
        }
    }
}

impl std::error::Error for TraitSolverError {}
