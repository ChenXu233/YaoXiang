//! 对象安全
//!
//! 实现特质对象安全性检查

use crate::frontend::shared::error::Result;

/// 对象安全错误
#[derive(Debug, Clone)]
pub struct ObjectSafetyError {
    pub message: String,
}

/// 对象安全检查器
pub struct ObjectSafetyChecker;

impl Default for ObjectSafetyChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectSafetyChecker {
    /// 创建新的检查器
    pub fn new() -> Self {
        Self
    }

    /// 检查对象安全性
    pub fn check(
        &self,
        trait_name: &str,
    ) -> Result<(), ObjectSafetyError> {
        // 检查特质是否满足对象安全要求
        if !self.is_object_safe(trait_name) {
            return Err(ObjectSafetyError {
                message: format!("Trait {} is not object-safe", trait_name),
            });
        }

        // 检查是否有不安全的关联类型
        self.check_associated_types(trait_name)?;

        // 检查方法签名
        self.check_method_signatures(trait_name)?;

        Ok(())
    }

    /// 检查特质是否对象安全
    fn is_object_safe(
        &self,
        trait_name: &str,
    ) -> bool {
        // 简化实现：假设一些基本特质是对象安全的
        match trait_name {
            "Clone" | "Debug" | "Send" | "Sync" => true,
            _ => false, // 其他特质需要进一步检查
        }
    }

    /// 检查关联类型
    fn check_associated_types(
        &self,
        _trait_name: &str,
    ) -> Result<(), ObjectSafetyError> {
        // 检查是否有不安全的关联类型
        // 在实际实现中，这里会检查每个关联类型是否满足对象安全要求

        // 简化实现：假设没有关联类型或关联类型都是安全的
        Ok(())
    }

    /// 检查方法签名
    fn check_method_signatures(
        &self,
        _trait_name: &str,
    ) -> Result<(), ObjectSafetyError> {
        // 检查所有方法是否满足对象安全要求
        // 1. 方法不能返回 Self
        // 2. 方法不能有泛型参数（除了一些特殊情况）
        // 3. 方法不能使用 `self: Box<Self>` 之外的形式

        // 简化实现：假设基本特质的方法都是对象安全的
        Ok(())
    }
}
