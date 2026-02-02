#![allow(clippy::result_large_err)]

//! 特质求解器
//!
//! 实现特质约束求解

use crate::frontend::shared::error::Result;
use crate::frontend::core::type_system::MonoType;
use std::collections::HashMap;

/// 特质约束
#[derive(Debug, Clone)]
pub struct TraitConstraint {
    pub name: String,
    pub args: Vec<MonoType>,
}

/// 特质求解器
pub struct TraitSolver {
    /// 已解析的特质约束
    resolved_constraints: HashMap<String, TraitConstraint>,
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
            resolved_constraints: HashMap::new(),
        }
    }

    /// 求解特质约束
    pub fn solve(
        &mut self,
        constraint: &TraitConstraint,
    ) -> Result<()> {
        // 检查是否已经解决
        if self.resolved_constraints.contains_key(&constraint.name) {
            return Ok(());
        }

        // 检查约束是否可以满足
        if self.can_satisfy_constraint(constraint) {
            self.resolved_constraints
                .insert(constraint.name.clone(), constraint.clone());
            Ok(())
        } else {
            Err(
                crate::frontend::shared::error::diagnostic::Diagnostic::error(
                    "E0602".to_string(),
                    format!("Cannot satisfy trait constraint: {}", constraint.name),
                    None,
                ),
            )
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

    /// 检查特质是否满足
    pub fn check_trait(
        &mut self,
        ty: &MonoType,
        trait_name: &str,
    ) -> bool {
        // 简化的实现：根据类型检查特质可用性
        match trait_name {
            "Clone" => self.check_clone_trait(ty),
            "Debug" => self.check_debug_trait(ty),
            "Send" => self.check_send_trait(ty),
            "Sync" => self.check_sync_trait(ty),
            _ => true, // 其他特质默认为可满足
        }
    }
}
