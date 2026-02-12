#![allow(clippy::result_large_err)]

//! 特质求解器
//!
//! 实现特质约束求解
//!
//! 集成 RFC-011 Trait 系统，支持：
//! - 内置特质 (Clone, Debug, Send, Sync)
//! - 用户定义特质
//! - 约束传播

use crate::util::diagnostic::{ErrorCodeDefinition, I18nRegistry, Result};
use crate::frontend::core::type_system::MonoType;
use crate::frontend::type_level::trait_bounds::{TraitTable, TraitBound, TraitSolver as AdvancedSolver};
use std::collections::HashMap;

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

    /// 高级求解器（用于用户定义 Trait）
    advanced_solver: AdvancedSolver,

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
            advanced_solver: AdvancedSolver::new(),
            trait_table: None,
        }
    }

    /// 设置 Trait 表
    pub fn set_trait_table(
        &mut self,
        table: TraitTable,
    ) {
        self.trait_table = Some(table.clone());
        self.advanced_solver.set_trait_table(table);
    }

    /// 求解特质约束
    pub fn solve(
        &mut self,
        constraint: &TraitConstraint,
    ) -> Result<()> {
        // 检查是否已经解决
        if self.simple_constraints.contains_key(&constraint.name) {
            return Ok(());
        }

        // 尝试使用高级求解器（支持用户定义 Trait）
        if let Some(ref table) = self.trait_table {
            if table.has_trait(&constraint.name) {
                // 用户定义的 Trait，使用高级求解器
                let trait_bound = TraitBound {
                    trait_name: constraint.name.clone(),
                    self_type: constraint.args.first().cloned().unwrap_or(MonoType::Void),
                };
                self.advanced_solver.add_constraint(trait_bound);
                match self.advanced_solver.solve() {
                    Ok(_) => {
                        self.simple_constraints
                            .insert(constraint.name.clone(), constraint.clone());
                        Ok(())
                    }
                    Err(_e) => Err(ErrorCodeDefinition::trait_bound_not_satisfied(
                        &constraint
                            .args
                            .first()
                            .map(|a| format!("{}", a))
                            .unwrap_or_default(),
                        &constraint.name,
                    )
                    .build(I18nRegistry::en())),
                }
            } else {
                // 内置 Trait，使用简化求解
                self.solve_builtin_trait(constraint)
            }
        } else {
            // 没有 Trait 表，使用简化求解
            self.solve_builtin_trait(constraint)
        }
    }

    /// 求解内置特质约束
    fn solve_builtin_trait(
        &mut self,
        constraint: &TraitConstraint,
    ) -> Result<()> {
        // 检查约束是否可以满足
        if self.can_satisfy_constraint(constraint) {
            self.simple_constraints
                .insert(constraint.name.clone(), constraint.clone());
            Ok(())
        } else {
            Err(ErrorCodeDefinition::trait_bound_not_satisfied(
                &constraint
                    .args
                    .first()
                    .map(|a| format!("{}", a))
                    .unwrap_or_default(),
                &constraint.name,
            )
            .build(I18nRegistry::en()))
        }
    }

    /// 检查是否可以满足约束
    fn can_satisfy_constraint(
        &self,
        constraint: &TraitConstraint,
    ) -> bool {
        // 简化的实现：根据类型检查特质可用性
        // 特质约束的第一个参数通常是类型本身
        if let Some(ty) = constraint.args.first() {
            match constraint.name.as_str() {
                "Clone" => self.check_clone_trait(ty),
                "Debug" => self.check_debug_trait(ty),
                "Send" => self.check_send_trait(ty),
                "Sync" => self.check_sync_trait(ty),
                _ => true, // 其他特质默认为可满足
            }
        } else {
            false // 没有参数的约束无法满足
        }
    }

    /// 检查Clone特质
    fn check_clone_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        // 简化的实现：检查类型是否可克隆
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
        )
    }

    /// 检查Debug特质
    fn check_debug_trait(
        &self,
        _ty: &MonoType,
    ) -> bool {
        // 简化的实现：所有类型都可以Debug
        true
    }

    /// 检查Send特质
    fn check_send_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        // 简化的实现：基本类型都是Send
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
        )
    }

    /// 检查Sync特质
    fn check_sync_trait(
        &self,
        ty: &MonoType,
    ) -> bool {
        // 简化的实现：基本类型都是Sync
        matches!(
            ty,
            MonoType::Int(_)
                | MonoType::Float(_)
                | MonoType::Bool
                | MonoType::Char
                | MonoType::String
        )
    }

    /// 检查特质是否满足（支持用户定义 Trait）
    pub fn check_trait(
        &mut self,
        ty: &MonoType,
        trait_name: &str,
    ) -> bool {
        // 首先检查是否是用户定义的 Trait
        if let Some(ref table) = self.trait_table {
            if table.has_trait(trait_name) {
                // 使用高级求解器
                return self.advanced_solver.satisfies_trait(trait_name, ty);
            }
        }

        // 内置 Trait
        match trait_name {
            "Clone" => self.check_clone_trait(ty),
            "Debug" => self.check_debug_trait(ty),
            "Send" => self.check_send_trait(ty),
            "Sync" => self.check_sync_trait(ty),
            _ => true,
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
    /// 如果类型 T 实现了 Trait A，且 B: A，则 T 也实现了 B
    pub fn propagate_constraints_to_type_args(
        &self,
        _ty: &MonoType,
        _trait_name: &str,
    ) -> Vec<TraitConstraint> {
        // 简化实现：返回空列表
        // 完整的实现需要：
        // 1. 分析类型的泛型参数
        // 2. 检查父 Trait 继承关系
        // 3. 生成传播后的约束
        Vec::new()
    }
}
