//! 统一作用域管理
//!
//! 单一职责：管理变量作用域栈
//! 被 StatementChecker 和 ExpressionInferrer 共享使用
//!
//! #295 重构：三链分离模型——
//! - `globals`：模块级绑定（顶层变量/函数名/类型名），所有代码可见
//! - `param_scopes`：函数/lambda 参数层，跨函数边界累积可见（柯里化固化）
//! - `local_scopes`：局部变量层（函数体一层 + 嵌套块层），进入函数时推新层
//!   查找顺序 local → param → global，天然实现「闭包不捕获外层局部变量」：
//!   外层函数的局部变量不在当前 local 链上，参数链与全局链才是跨边界通道。

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

/// 作用域管理器（#295 三链模型）
///
/// 管理变量的作用域栈，支持嵌套作用域的进入与退出。
/// 整个类型检查流程共享同一个 ScopeManager 实例。
pub struct ScopeManager {
    /// 模块级绑定：顶层变量、函数名、类型名——所有代码可见
    globals: HashMap<String, VarInfo>,
    /// 参数链：函数/lambda 参数层，跨函数边界累积可见（柯里化固化）
    param_scopes: Vec<HashMap<String, VarInfo>>,
    /// 局部作用域：函数体层 + 嵌套块层；进入函数/lambda 时外层局部层整体移出
    /// （存入 saved_local_scopes），因此外层函数的局部变量不在当前链上（闭包不捕获）
    local_scopes: Vec<HashMap<String, VarInfo>>,
    /// 被 enter_fn 移出的外层局部层栈（嵌套函数支持）
    saved_local_scopes: Vec<Vec<HashMap<String, VarInfo>>>,
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
    /// 创建新的作用域管理器（三链皆空）
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            param_scopes: Vec::new(),
            local_scopes: Vec::new(),
            saved_local_scopes: Vec::new(),
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

    /// 进入函数体（函数定义 / lambda 表达式）：
    /// 外层函数的局部层整体移出（闭包不捕获），新局部层 + 新参数层（跨边界固化可见）。
    pub fn enter_fn(&mut self) {
        self.param_scopes.push(HashMap::new());
        let outer = std::mem::take(&mut self.local_scopes);
        self.saved_local_scopes.push(outer);
        self.local_scopes = vec![HashMap::new()];
    }

    /// 退出函数体：丢弃当前函数局部层（含防御性未退出的块层），恢复外层局部层
    pub fn exit_fn(&mut self) {
        self.param_scopes.pop();
        if let Some(outer) = self.saved_local_scopes.pop() {
            self.local_scopes = outer;
        } else {
            self.local_scopes.clear();
        }
    }

    /// 进入块作用域（if/while/for body 等）：块不逃逸，局部变量完全穿透
    pub fn enter_block(&mut self) {
        self.local_scopes.push(HashMap::new());
    }

    /// 退出块作用域
    pub fn exit_block(&mut self) {
        if !self.local_scopes.is_empty() {
            self.local_scopes.pop();
        }
    }

    /// 添加局部变量：模块级（local_scopes 空）进 globals，函数体内进当前局部层
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
        definition_span: Span,
    ) {
        self.type_ledger
            .insert((self.current_stmt_key, name.clone()), poly.clone());
        let info = VarInfo {
            poly,
            is_mut,
            moved: false,
            definition_span,
        };
        if self.local_scopes.is_empty() {
            self.globals.insert(name, info);
        } else {
            self.local_scopes.last_mut().unwrap().insert(name, info);
        }
    }

    /// 添加参数（函数签名参数 / lambda 参数）：进参数链，跨函数边界可见（柯里化固化）
    pub fn add_param(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
        definition_span: Span,
    ) {
        self.type_ledger
            .insert((self.current_stmt_key, name.clone()), poly.clone());
        let info = VarInfo {
            poly,
            is_mut,
            moved: false,
            definition_span,
        };
        if let Some(scope) = self.param_scopes.last_mut() {
            scope.insert(name, info);
        } else {
            self.globals.insert(name, info);
        }
    }

    /// 获取变量：局部链（内→外）→ 参数链（内→外）→ 全局
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.get_var_info(name).map(|info| &info.poly)
    }

    /// 获取变量完整信息（含可变性）：局部 → 参数 → 全局
    pub fn get_var_info(
        &self,
        name: &str,
    ) -> Option<&VarInfo> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        for scope in self.param_scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        self.globals.get(name)
    }

    /// 检查变量是否可变（局部 → 参数 → 全局）
    pub fn var_is_mutable(
        &self,
        name: &str,
    ) -> Option<bool> {
        self.get_var_info(name).map(|info| info.is_mut)
    }

    /// 标记变量为已移动（局部 → 参数 → 全局）
    pub fn mark_moved(
        &mut self,
        name: &str,
    ) {
        for scope in self.local_scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.moved = true;
                return;
            }
        }
        for scope in self.param_scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.moved = true;
                return;
            }
        }
        if let Some(info) = self.globals.get_mut(name) {
            info.moved = true;
        }
    }

    /// 检查变量是否已移动（局部 → 参数 → 全局）
    pub fn var_is_moved(
        &self,
        name: &str,
    ) -> Option<bool> {
        self.get_var_info(name).map(|info| info.moved)
    }

    /// 从当前局部层移除变量
    pub fn remove_var(
        &mut self,
        name: &str,
    ) -> bool {
        self.local_scopes
            .last_mut()
            .map(|scope| scope.remove(name).is_some())
            .unwrap_or(false)
    }

    /// 更新变量（局部 → 参数 → 全局找到第一个）；未找到则按 add_var 添加
    pub fn update_var(
        &mut self,
        name: &str,
        poly: PolyType,
    ) {
        self.type_ledger
            .insert((self.current_stmt_key, name.to_string()), poly.clone());
        for scope in self.local_scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.poly = poly;
                return;
            }
        }
        for scope in self.param_scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.poly = poly;
                return;
            }
        }
        if let Some(info) = self.globals.get_mut(name) {
            info.poly = poly;
            return;
        }
        // 未找到：按局部变量添加
        self.add_var(name.to_string(), poly, false, Span::default());
    }

    /// 检查变量是否存在于当前局部层
    pub fn var_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.local_scopes
            .last()
            .is_some_and(|s| s.contains_key(name))
    }

    /// 检查变量是否存在于任何链
    pub fn var_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.get_var_info(name).is_some()
    }

    /// 获取所有非全局变量（局部 + 参数，内层覆盖外层）——
    /// 用于函数退出前的 function_local_vars 保存
    pub fn vars(&self) -> HashMap<String, PolyType> {
        let mut result = HashMap::new();
        for scope in &self.param_scopes {
            for (name, info) in scope {
                result.insert(name.clone(), info.poly.clone());
            }
        }
        for scope in &self.local_scopes {
            for (name, info) in scope {
                result.insert(name.clone(), info.poly.clone());
            }
        }
        result
    }

    /// 全局绑定（模块级变量/函数名/类型名）的只读访问
    pub fn globals(&self) -> &HashMap<String, VarInfo> {
        &self.globals
    }

    /// 获取所有非全局变量及可变性（局部 + 参数，内层覆盖外层）
    pub fn vars_with_mut(&self) -> HashMap<String, VarInfo> {
        let mut result = HashMap::new();
        for scope in &self.param_scopes {
            for (name, info) in scope {
                result.insert(name.clone(), info.clone());
            }
        }
        for scope in &self.local_scopes {
            for (name, info) in scope {
                result.insert(name.clone(), info.clone());
            }
        }
        result
    }

    /// 获取当前（最内层）局部层的变量，保留可变性——
    /// 用于 promote_loop_vars_to_parent_scope
    pub fn current_scope_vars(&self) -> HashMap<String, VarInfo> {
        self.local_scopes.last().cloned().unwrap_or_default()
    }

    /// 获取当前局部层深度
    pub fn scope_level(&self) -> usize {
        self.local_scopes.len()
    }
}
