#![allow(clippy::result_large_err)]

//! 替换逻辑
//!
//! 提供类型替换接口，复用 core/type_system/substitute.rs 中的公共实现

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::{MonoType, TypeVar, Substituter as CoreSubstituter};

/// 替换结果
pub struct SubstitutionResult {
    pub substituted: MonoType,
    pub success: bool,
}

/// 替换器（包装公共实现，添加额外方法）
#[derive(Debug, Clone, Default)]
pub struct Substituter(CoreSubstituter);

impl Substituter {
    /// 创建新的替换器
    pub fn new() -> Self {
        Self(CoreSubstituter::new())
    }

    /// 执行类型替换
    pub fn substitute(
        &self,
        ty: &MonoType,
        var: &TypeVar,
        replacement: &MonoType,
    ) -> Result<SubstitutionResult> {
        let substituted = self.0.substitute_var(ty, var, replacement);
        Ok(SubstitutionResult {
            substituted,
            success: true,
        })
    }

    /// 批量替换
    pub fn substitute_batch(
        &self,
        ty: &MonoType,
        substitutions: &[(TypeVar, MonoType)],
    ) -> Result<MonoType> {
        let mut result = ty.clone();

        // 依次应用所有替换
        for (var, replacement) in substitutions {
            result = self.0.substitute_var(&result, var, replacement);
        }

        Ok(result)
    }
}
