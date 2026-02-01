//! RFC-011 类型边界检查
//!
//! 检查泛型类型边界和约束

use crate::frontend::shared::error::Result;
use crate::frontend::core::type_system::MonoType;
use crate::frontend::typecheck::traits::solver::TraitSolver;

/// 边界检查器
pub struct BoundsChecker {
    trait_solver: TraitSolver,
}

impl Default for BoundsChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundsChecker {
    /// 创建新的边界检查器
    pub fn new() -> Self {
        Self {
            trait_solver: TraitSolver::new(),
        }
    }

    /// 检查特质边界
    pub fn check_trait_bounds(
        &mut self,
        ty: &MonoType,
        bounds: &[String],
    ) -> Result<()> {
        // 检查每个边界
        for bound in bounds {
            if !self.trait_solver.check_trait(ty, bound) {
                return Err(
                    crate::frontend::shared::error::diagnostic::Diagnostic::error(format!(
                        "Type does not satisfy trait bound: {}",
                        bound
                    )),
                );
            }
        }
        Ok(())
    }

    /// 检查 Const 边界
    pub fn check_const_bounds(
        &self,
        _ty: &MonoType,
        _bounds: &[MonoType],
    ) -> Result<()> {
        // 简化的实现：const边界检查
        // TODO: 实现完整的const边界检查
        Ok(())
    }

    /// 检查生命周期边界
    pub fn check_lifetime_bounds(
        &self,
        _ty: &MonoType,
        _bounds: &[String],
    ) -> Result<()> {
        // 简化的实现：生命周期边界检查
        // TODO: 实现完整的生命周期边界检查
        Ok(())
    }

    /// 检查泛型参数边界
    pub fn check_generic_bounds(
        &mut self,
        ty: &MonoType,
        trait_bounds: &[String],
        const_bounds: &[MonoType],
    ) -> Result<()> {
        // 检查特质边界
        self.check_trait_bounds(ty, trait_bounds)?;

        // 检查const边界
        self.check_const_bounds(ty, const_bounds)?;

        Ok(())
    }
}
