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

/// `x = value` 的绑定动作（spec §4.3「赋值优先」）。
///
/// 规范算法（`x = value` 沿作用域链向外查找）：
///
/// ```text
/// 找到 mut x           → 赋值 OK
/// 找到 x（不可变，存活）→ E2010 不可重新赋值
/// 找不到               → 在当前作用域新声明
/// ```
///
/// `mut x = value` 是**显式新声明**，同作用域撞名 → E2002；
/// 外层存在同名 → E2013（禁止遮蔽）。
///
/// # 「已 moved」分支已删除（2026-09-19）
///
/// spec 旧文有一行「找到 x（已 moved）→ 视为未找到有效绑定，重新声明」。
/// 该分支**不可达且不该存在**：
///
/// - **不可达**：判定依赖 `VarInfo::moved`，而其唯一写入点 `mark_moved` 零调用点，
///   故 `moved` 恒为 `false`。
/// - **不该存在**：move 是**路径相关**的数据流属性（`if c { move p }` 后 `p` 在
///   一条路径上已 move、另一条尚未），一个布尔字段结构上无法表达——它没有
///   分支概念。真正的 move 分析在 `layers/ownership.rs`：构建函数体 CFG，
///   以 `Alive < Moved < Dropped` 格做数据流，分支汇合取 max（保守），
///   读检查点报 E2014/E2018。
///
/// 所以「moved 后可重声明」由**重声明本身**承担：写 `mut x = v` 即新声明；
/// 想复用旧名而不写 `mut`，本来就需要一个能表达「这是新绑定」的记号。
///
/// 本函数是这条规则的**唯一实现**：`StatementChecker` 与 `ExpressionInferrer`
/// 都调它。此前两处各写一份，导致规则不一致（一处有 E2010、一处没有）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingAction {
    /// 同作用域（或跨作用域 mut）重赋值：写回既有绑定
    Reassign,
    /// 新声明：在当前作用域插入
    Declare,
    /// 不可变绑定被重赋值 → E2010
    ImmutableReassign,
    /// 显式 `mut` 重新声明同名（同作用域）→ E2002
    DuplicateDefinition,
    /// 显式 `mut` 声明撞外层同名 → E2013 禁止遮蔽
    Shadowing,
}

impl ScopeManager {
    /// 判定 `name = value` 应走哪个动作（spec §4.3）。
    ///
    /// `is_mut` 是**语句级**的 `mut` 标记（是显式新声明还是赋值）。
    /// `is_declaration_only`：纯注解声明（`x: Int` / `f: () -> Void`，无初值）。
    /// 这类语句是**声明**而非赋值，不参与 §4.3 的「沿作用域链查找」。
    /// （pass2 会预注册它们以供前向引用，若按真实绑定处理会误报 E2010。）
    ///
    /// 返回的动作由调用方执行（写回 / 插入 / 报错）——因为两个检查器的
    /// 写回方式不同（一个走 `assign_var` 带 unify 开关，一个直接 `update_var`）。
    pub fn classify_binding(
        &self,
        name: &str,
        is_mut: bool,
        is_declaration_only: bool,
    ) -> BindingAction {
        // 前向引用占位（pass2 预注册、声明语句未执行）与导入名
        // （`use` 来的 std 导出 / 跨模块符号）都**不算本文件的现有绑定**：
        // 否则顶层 `mut i = 0` 误报 E2013；`ok = ...`（撞 `std.result.ok`）误报 E2010。
        let info = self
            .get_var_info(name)
            .filter(|v| !v.forward_declared && !v.imported);
        let in_current = info.is_some() && self.var_in_current_scope(name);
        let anywhere = info.is_some();
        let existing_mut = info.map(|v| v.is_mut).unwrap_or(false);

        // 纯注解声明：总是声明（同作用域真撞名才报 E2002）
        if is_declaration_only {
            if in_current {
                return BindingAction::DuplicateDefinition;
            }
            return BindingAction::Declare;
        }

        if is_mut {
            // `mut x = value`：显式新声明
            if in_current {
                return BindingAction::DuplicateDefinition;
            }
            if anywhere {
                return BindingAction::Shadowing;
            }
            return BindingAction::Declare;
        }

        // `x = value`：赋值优先
        if in_current {
            if existing_mut {
                BindingAction::Reassign
            } else {
                BindingAction::ImmutableReassign
            }
        } else if anywhere {
            if existing_mut {
                BindingAction::Reassign // 外层 mut：赋值同一绑定
            } else {
                BindingAction::ImmutableReassign
            }
        } else {
            BindingAction::Declare
        }
    }
}

/// 作用域中存储的变量信息
#[derive(Debug, Clone)]
pub struct VarInfo {
    pub poly: PolyType,
    pub is_mut: bool,
    /// 变量定义位置的 span（用于 LSP 跳转定义）
    pub definition_span: Span,
    /// 是否只是「前向引用占位」——pass2 预注册的顶层绑定，
    /// 其声明语句还未被 pass3 执行。
    ///
    /// 区别为何必要（spec §4.3）：前向引用要求「使用前名字已可见」，
    /// 但声明判定要求「`mut x = v` 时 x 还未声明」。两者共存靠这个标记：
    /// 占位项在 `classify_binding` 里**不算现有绑定**，
    /// 只有 pass3 执行到声明语句才转成真实绑定。
    pub forward_declared: bool,
    /// 是否来自 `use` 导入（std 导出或跨模块符号）。
    ///
    /// 导入名不是本文件的绑定——spec §4.3 的「沿作用域链查找」只查
    /// 程序自身声明。否则用户写 `ok = ...`（撞 `std.result.ok` 构造器）
    /// 会被误判为重赋值不可变变量 → 误报 E2010。
    pub imported: bool,
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
    /// 当前函数上下文名（#335 路径 A）：泛型体内的嵌套泛型调用请求以此为
    /// containing_fn，mono 据此对占位类型实参（TypeRef(参数名)）求值。
    /// None = 上下文未知（eval/REPL 等场景）。
    fn_context: Option<String>,
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
            fn_context: None,
        }
    }

    /// 设置当前正在检查的语句（账本键来源）——由 check_stmt 在入口调用
    pub fn set_current_stmt(
        &mut self,
        span: Span,
    ) {
        self.current_stmt_key = span.start.offset;
    }

    /// 设置当前函数上下文名（#335 路径 A）
    pub fn set_fn_context(
        &mut self,
        name: Option<String>,
    ) {
        self.fn_context = name;
    }

    /// 获取当前函数上下文名
    pub fn fn_context(&self) -> Option<&str> {
        self.fn_context.as_deref()
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
            definition_span,
            forward_declared: false,
            imported: false,
        };
        if self.local_scopes.is_empty() {
            self.globals.insert(name, info);
        } else {
            self.local_scopes.last_mut().unwrap().insert(name, info);
        }
    }

    /// 注入「前向引用占位」：名字可见（供函数体引用后置绑定），
    /// 但 `classify_binding` 不把它当现有绑定。
    ///
    /// 用途：pass2 预注册顶层绑定时调用——前向引用（`main` 中引用后置绑定）
    /// 要求「使用前名字已可见」，而 spec §4.3 的声明判定要求
    /// 「`mut x = v` 时 x 还未声明」。两者靠这个标记共存。
    pub fn add_forward_declared_var(
        &mut self,
        name: String,
        poly: PolyType,
        definition_span: Span,
    ) {
        let info = VarInfo {
            poly,
            is_mut: false,
            definition_span,
            forward_declared: true,
            imported: false,
        };
        if self.local_scopes.is_empty() {
            self.globals.insert(name, info);
        } else {
            self.local_scopes.last_mut().unwrap().insert(name, info);
        }
    }

    /// 注入「导入名」：来自 `use` 的符号（std 导出 / 跨模块）。
    /// 可见可调用，但**不是本文件的绑定**——不参与 spec §4.3 的赋值判定。
    pub fn add_imported_var(
        &mut self,
        name: String,
        poly: PolyType,
        definition_span: Span,
    ) {
        let info = VarInfo {
            poly,
            is_mut: false,
            definition_span,
            forward_declared: false,
            imported: true,
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
            definition_span,
            forward_declared: false,
            imported: false,
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

    /// 变量是否在局部/参数作用域中（不含全局）。
    /// 赋值统一判据用：局部程序变量的重赋值强制类型统一（E1002）；
    /// 仅存在于全局（std/模块导出）的名字不走统一——全局导出是不可变导入，
    /// 与其撞名的局部赋值（如 `first = xs[0]` 撞 std.list.first）保持旧覆写行为。
    pub fn var_in_local_scopes(
        &self,
        name: &str,
    ) -> bool {
        self.local_scopes.iter().any(|s| s.contains_key(name))
            || self.param_scopes.iter().any(|s| s.contains_key(name))
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
        // 模块级（三链全空）：当前作用域就是 `globals`。
        //
        // 此前只看 `local_scopes`，于是顶层的 `mut d = 1; mut d = 2` 被当成
        // 「外层存在同名」→ 误报 E2013（禁止遮蔽），而正确诊断是 E2002
        //（同作用域重复定义）。函数体内同写法报 E2002，顶层报 E2013——
        // 同一语法两个诊断，因为顶层绑定住在 `globals`。
        if self.at_module_level() {
            return self.globals.contains_key(name);
        }
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

    /// 是否在模块级（三链全空 = 无函数边界、无块层，#295）
    pub fn at_module_level(&self) -> bool {
        self.local_scopes.is_empty() && self.param_scopes.is_empty()
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
