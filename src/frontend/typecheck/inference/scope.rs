//! 统一作用域管理
//!
//! 单一职责：管理变量作用域栈
//! 被 StatementChecker 和 ExpressionInferrer 共享使用

use std::collections::HashMap;

use crate::frontend::core::type_system::PolyType;

/// 作用域管理器
///
/// 管理变量的作用域栈，支持嵌套作用域的进入与退出。
/// 整个类型检查流程共享同一个 ScopeManager 实例。
pub struct ScopeManager {
    scopes: Vec<HashMap<String, PolyType>>,
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeManager {
    /// 创建新的作用域管理器（带一个全局作用域）
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.scopes.last_mut().unwrap().insert(name, poly);
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        for scope in self.scopes.iter().rev() {
            if let Some(poly) = scope.get(name) {
                return Some(poly);
            }
        }
        None
    }

    /// 在现有作用域中更新变量（从内层到外层搜索）
    pub fn update_var(
        &mut self,
        name: &str,
        poly: PolyType,
    ) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), poly);
                return;
            }
        }
        // 未找到则添加到当前作用域
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), poly);
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scopes.last().is_some_and(|s| s.contains_key(name))
    }

    /// 检查变量是否存在于任何作用域
    pub fn var_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scopes.iter().any(|scope| scope.contains_key(name))
    }

    /// 获取所有变量（内层覆盖外层）
    pub fn vars(&self) -> HashMap<String, PolyType> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            for (name, poly) in scope {
                result.insert(name.clone(), poly.clone());
            }
        }
        result
    }

    /// 获取当前作用域层级
    pub fn scope_level(&self) -> usize {
        self.scopes.len()
    }
}
