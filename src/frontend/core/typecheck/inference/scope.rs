//! 统一作用域管理
//!
//! 单一职责：管理变量作用域栈
//! 被 StatementChecker 和 ExpressionInferrer 共享使用

use std::collections::HashMap;

use crate::frontend::core::types::PolyType;
use crate::util::span::Span;

/// 作用域中存储的变量信息
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub poly: PolyType,
    pub is_mut: bool,
    pub moved: bool,
    /// 变量定义位置的 span（用于 LSP 跳转定义）
    pub definition_span: Span,
}

// Need to import Span
// (already available via crate::util::span::Span)

/// 作用域管理器
///
/// 管理变量的作用域栈，支持嵌套作用域的进入与退出。
/// 整个类型检查流程共享同一个 ScopeManager 实例。
pub struct ScopeManager {
    scopes: Vec<HashMap<String, VarInfo>>,
    /// 当前正在检查的语句的键（span 起始 offset）——供 type_ledger 定位变量归属
    current_stmt_key: usize,
    /// 变量类型账本：(定义所在语句的 span offset, 变量名) → 推断类型。
    /// 作用域 pop 后条目保留，供下游（所有权检查）按位置查询（#256）。
    type_ledger: HashMap<(usize, String), PolyType>,
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
            current_stmt_key: 0,
            type_ledger: HashMap::new(),
        }
    }

    /// 设置当前正在检查的语句（账本键来源）——由 check_stmt 在入口调用
    pub fn set_current_stmt(
        &mut self,
        span: Span,
    ) {
        self.current_stmt_key = span.start.offset;
    }

    /// 变量类型账本（只读）——所有权检查的 Move/Dup 分类用（#256）
    pub fn type_ledger(&self) -> &HashMap<(usize, String), PolyType> {
        &self.type_ledger
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
        is_mut: bool,
        definition_span: Span,
    ) {
        // 账本登记：作用域 pop 后仍可查（#256）
        self.type_ledger
            .insert((self.current_stmt_key, name.clone()), poly.clone());
        self.scopes.last_mut().unwrap().insert(
            name,
            VarInfo {
                poly,
                is_mut,
                moved: false,
                definition_span,
            },
        );
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(&info.poly);
            }
        }
        None
    }

    /// 获取变量完整信息（含可变性）
    pub fn get_var_info(
        &self,
        name: &str,
    ) -> Option<&VarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    /// 检查变量是否可变（从内层到外层搜索）
    pub fn var_is_mutable(
        &self,
        name: &str,
    ) -> Option<bool> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.is_mut);
            }
        }
        None
    }

    /// 标记变量为已移动（从内层到外层搜索）
    pub fn mark_moved(
        &mut self,
        name: &str,
    ) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.moved = true;
                return;
            }
        }
    }

    /// 检查变量是否已移动（从内层到外层搜索）
    pub fn var_is_moved(
        &self,
        name: &str,
    ) -> Option<bool> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.moved);
            }
        }
        None
    }

    /// 从当前作用域移除变量
    pub fn remove_var(
        &mut self,
        name: &str,
    ) -> bool {
        self.scopes.last_mut().unwrap().remove(name).is_some()
    }

    /// 在现有作用域中更新变量（从内层到外层搜索），保留已有的可变性
    pub fn update_var(
        &mut self,
        name: &str,
        poly: PolyType,
    ) {
        // 账本同步：普通赋值绑定（view = &p）经此路径，同样登记（#256）
        self.type_ledger
            .insert((self.current_stmt_key, name.to_string()), poly.clone());
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.poly = poly;
                return;
            }
        }
        // 未找到则添加到当前作用域，默认不可变
        self.scopes.last_mut().unwrap().insert(
            name.to_string(),
            VarInfo {
                poly,
                is_mut: false,
                moved: false,
                definition_span: Span::default(),
            },
        );
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
            for (name, info) in scope {
                result.insert(name.clone(), info.poly.clone());
            }
        }
        result
    }

    /// 获取所有变量及其可变性（内层覆盖外层）
    pub fn vars_with_mut(&self) -> HashMap<String, VarInfo> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            for (name, info) in scope {
                result.insert(name.clone(), info.clone());
            }
        }
        result
    }

    /// 获取当前（最内层）作用域的变量，保留可变性
    /// 用于 promote_loop_vars_to_parent_scope
    pub fn current_scope_vars(&self) -> HashMap<String, VarInfo> {
        self.scopes.last().cloned().unwrap_or_default()
    }

    /// 获取当前作用域层级
    pub fn scope_level(&self) -> usize {
        self.scopes.len()
    }
}
