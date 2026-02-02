//! 特质解析
//!
//! 实现特质解析和查找

use crate::util::diagnostic::Result;

/// 特质解析错误
#[derive(Debug, Clone)]
pub struct TraitResolutionError {
    pub message: String,
}

/// 特质解析器
pub struct TraitResolver;

impl Default for TraitResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TraitResolver {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self
    }

    /// 解析特质
    pub fn resolve(
        &self,
        name: &str,
    ) -> Result<String, TraitResolutionError> {
        // 解析特质名称并返回其完整路径
        match self.find_trait_definition(name) {
            Some(definition) => Ok(definition),
            None => Err(TraitResolutionError {
                message: format!("Cannot find trait definition: {}", name),
            }),
        }
    }

    /// 查找特质定义
    fn find_trait_definition(
        &self,
        name: &str,
    ) -> Option<String> {
        // 简化实现：查找已知的特质定义
        // 在实际实现中，这里会遍历模块系统查找特质

        match name {
            "Clone" => Some("std:: Clone".to_string()),
            "Debug" => Some("std::fmt::Debug".to_string()),
            "Send" => Some("std::marker::Send".to_string()),
            "Sync" => Some("std::marker::Sync".to_string()),
            _ => None,
        }
    }

    /// 解析特质路径
    pub fn resolve_trait_path(
        &self,
        path: &str,
    ) -> Result<String, TraitResolutionError> {
        // 解析特质路径（如 `std::fmt::Debug`）
        let parts: Vec<&str> = path.split("::").collect();

        if parts.is_empty() {
            return Err(TraitResolutionError {
                message: format!("Invalid trait path: {}", path),
            });
        }

        let trait_name = parts.last().unwrap();
        self.resolve(trait_name)
    }

    /// 检查特质是否已定义
    pub fn is_trait_defined(
        &self,
        name: &str,
    ) -> bool {
        self.find_trait_definition(name).is_some()
    }
}
