//! 流敏感假设集 Γ —— 当前控制流路径上已成立的命题集合
//!
//! RFC-027 §3.2-3.3 + spec §4.1-4.2：
//! - 不可变绑定：假设永久有效，Γ 单调只增
//! - mut 变量赋值：kill 所有依赖该变量的假设（标记为 dead）
//! - 分支结束：exit_scope 移除当前 scope 内的所有假设
//!
//! 6.9.4（RFC-039 D62 / issue #534）起另有**证据边界（barrier）**：函数体 / 闭包体 /
//! spawn 体各自是一道边界。体外事实不入体、体内自产的事实只在体内可见、出体即亡
//! （见 enter_barrier / restore_barrier）。证据读取的唯一出口是 visible 与
//! const_equalities——两者都只认边界以上的事实。

use crate::frontend::core::types::const_data::ConstExpr;
use crate::util::span::Span;

/// 流敏感假设集 Γ
///
/// 与旧 AssumptionStack 的区别：
/// - 旧：纯栈 push/pop，无 kill，假设一旦压入只能等 pop 弹出
/// - 新：假设带 alive/dead 标记 + scope 深度，支持中途 kill
///
/// 为什么用标记式而非依赖图：
/// 假设数量在单条路径上通常 < 20，O(n) 遍历完全够。
/// 维护反向索引的成本远高于遍历成本。
#[derive(Debug, Default, Clone)]
pub struct FlowSensitiveGamma {
    entries: Vec<GammaEntry>,
    scope_depth: usize,
    /// 证据边界深度（6.9.4）：只有 scope_depth >= barrier_depth 的活跃事实是
    /// **当前边界可见**的证据。0 = 无边界（模块层 / 所有权层的分支守卫）。
    barrier_depth: usize,
}

/// 一条事实的**绑定身份**：该名字此刻解析到的绑定在其定义处的名源位置
/// （`VarInfo::definition_span`，与 LSP `resolves_to` / 类型账本同源）。
///
/// 为什么需要它：值环境按**绑定**解析（`ScopeManager::get_var_info` 取最内层绑定），
/// γ 事实却按**名字**索引——两者对齐的唯一办法是记下「这条事实说的是哪一次绑定」。
/// 口径与 6.9.2 轮 t23（F1'）同源：那里的值也是挂在 per-binding 的 `VarInfo` 上、
/// 由同一条解析链取最内层，而不是按名的旁表（按名旁表在遮蔽下会取到外层同名绑定）。
pub type BindingKey = Span;

/// 一条事实的**绑定身份快照**：`(名字, 注入时该名字解析到的绑定身份)`。
///
/// `None` = 注入时该名字无任何绑定（此后若出现绑定，事实即失效）。
pub type BindingSnapshot = Vec<(String, Option<BindingKey>)>;

#[derive(Debug, Clone)]
struct GammaEntry {
    expr: ConstExpr,
    alive: bool,
    scope_depth: usize,
    /// 注入时的绑定身份快照（`None` = 该事实由 [`FlowSensitiveGamma::inject`] 注入，
    /// 不参与绑定校验——只服务**不读**等式的消费者）。
    bindings: Option<BindingSnapshot>,
}

/// 向后兼容别名
pub type AssumptionStack = FlowSensitiveGamma;

impl FlowSensitiveGamma {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scope_depth: 0,
            barrier_depth: 0,
        }
    }

    /// 进入新 scope（if/match 支入口）
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// 离开 scope —— 移除所有当前深度的假设
    pub fn exit_scope(&mut self) {
        self.entries.retain(|e| e.scope_depth < self.scope_depth);
        self.scope_depth -= 1;
    }

    /// 注入假设（assert 成功后调用）
    pub fn inject(
        &mut self,
        cond: ConstExpr,
    ) {
        self.push_entry(cond, None);
    }

    /// 注入一条**证据面**事实，并记下它的**绑定身份快照**（6.9.4 修复 F-1）。
    ///
    /// 读取侧（[`Self::const_equalities_resolved`]）逐名要求「此刻解析到同一次绑定」
    /// 才采用这条事实的等式——于是 `match` 臂的模式绑定遮蔽同名外层绑定时，外层
    /// 守卫事实不会串进臂内；出臂后解析回外层绑定，事实照旧可用（不误杀）。
    pub fn inject_boundings(
        &mut self,
        cond: ConstExpr,
        bindings: BindingSnapshot,
    ) {
        self.push_entry(cond, Some(bindings));
    }

    fn push_entry(
        &mut self,
        cond: ConstExpr,
        bindings: Option<BindingSnapshot>,
    ) {
        self.entries.push(GammaEntry {
            expr: cond,
            alive: true,
            scope_depth: self.scope_depth,
            bindings,
        });
    }

    /// kill —— mut 变量赋值时，标记所有依赖该变量的假设为 dead
    ///
    /// 保守 over-approximation：假设中出现该变量名就 kill 整条。
    /// sound：kill 多了只需重新 assert，不会用旧证明保证新值。
    pub fn kill(
        &mut self,
        var_name: &str,
    ) {
        for entry in &mut self.entries {
            if entry.alive && references_var(&entry.expr, var_name) {
                entry.alive = false;
            }
        }
    }

    /// 进入**证据边界**（函数体 / 闭包体 / spawn 体，6.9.4 P5），返回旧边界深度。
    ///
    /// 两件事一起做，缺一不可：
    /// 1. enter_scope：体内自产的事实落在新深度（体内自产 ⇒ 对新边界可见）；
    /// 2. 把 barrier_depth 设到该深度：更浅（体外）的事实从此不可见（体外不入）。
    pub fn enter_barrier(&mut self) -> usize {
        self.enter_scope();
        let prev = self.barrier_depth;
        self.barrier_depth = self.scope_depth;
        prev
    }

    /// 退出证据边界：先 exit_scope 再还原边界深度。
    ///
    /// 顺序不能颠倒：只还原 barrier_depth 而不退 scope，会让**体内自产**的事实
    /// 留在栈上继续当证据（体内事实不随体亡 = unsound）。
    pub fn restore_barrier(
        &mut self,
        prev: usize,
    ) {
        self.exit_scope();
        self.barrier_depth = prev;
    }

    /// 当前边界**可见**的活跃假设（owned Vec；dead 与边界外的都被过滤）。
    ///
    /// 这是 Γ 作为**证据来源**的唯一读取口：is_empty / current / const_equalities
    /// 共用同一判据（is_visible）。
    pub fn visible(&self) -> Vec<ConstExpr> {
        self.entries
            .iter()
            .filter(|e| Self::is_visible(e, self.barrier_depth))
            .map(|e| e.expr.clone())
            .collect()
    }

    /// Γ 中的**常量等式事实**（6.9.4 的第四证据源）：
    /// 一侧闭常量 == 一侧唯一 NamedVar（42 == y 与 y == 42 同义）。
    ///
    /// 返回 (名字, 值) 对——正是 TermEnv 值侧需要的形状（该名字此刻有值）：
    /// - 闭常量侧走**唯一归一化入口**（normalize，一次判定一份预算），不写第二个
    ///   求值器；闭项求值失败（如 1 / 0）或两侧都不是唯一 NamedVar ⇒ 该条不产值
    ///   （宁可少证，不可错证）。
    /// - 只看 visible()：边界外的事实不入体。
    /// - **绑定校验（F-1）**：`resolve` 把名字解析到此刻最内层绑定的身份（调用方
    ///   传 [`scope_binding_key`]，与值环境同一条链）；快照不是当前绑定的事实不产值
    ///   ——见 [`bindings_are_current`]。这是本方法成为**生产唯一读取口**的原因：
    ///   无 resolve 的读法无法区分「同名的另一次绑定」。
    pub fn const_equalities_resolved(
        &self,
        resolve: &dyn Fn(&str) -> Option<BindingKey>,
    ) -> Vec<(String, crate::frontend::core::types::const_data::ConstValue)> {
        let facts: Vec<&ConstExpr> = self
            .entries
            .iter()
            .filter(|entry| Self::is_visible(entry, self.barrier_depth))
            .filter(|entry| {
                // fail-closed：没有绑定身份快照的事实**不是证据**（只有 `inject`
                // 注入的事实无快照——它服务不读等式的消费者）。宁可少证，不可错证。
                entry
                    .bindings
                    .as_ref()
                    .is_some_and(|snapshot| bindings_are_current(snapshot, resolve))
            })
            .map(|entry| &entry.expr)
            .collect();
        Self::equalities_of(facts)
    }

    /// 直驱测试用读取口：**只读无绑定快照的事实**（[`Self::inject`] 注入的）。
    ///
    /// 生产读取必须走 [`Self::const_equalities_resolved`]：本方法看不到绑定身份，
    /// 在遮蔽形态下会取到外层同名绑定的事实（F-1）。故它只在 `cfg(test)` 下存在，
    /// 供不构造作用域的 γ 原语单测使用。
    #[cfg(test)]
    pub fn const_equalities(
        &self
    ) -> Vec<(String, crate::frontend::core::types::const_data::ConstValue)> {
        let facts: Vec<&ConstExpr> = self
            .entries
            .iter()
            .filter(|entry| Self::is_visible(entry, self.barrier_depth))
            .filter(|entry| entry.bindings.is_none())
            .map(|entry| &entry.expr)
            .collect();
        Self::equalities_of(facts)
    }

    /// 等式提取（单一实现：可见性/绑定校验在调用侧，这里只做形态 + 归一化）。
    fn equalities_of(
        facts: Vec<&ConstExpr>
    ) -> Vec<(String, crate::frontend::core::types::const_data::ConstValue)> {
        use crate::frontend::core::types::const_data::BinOp;

        let mut out = Vec::new();
        for fact in facts {
            let ConstExpr::BinOp {
                op: BinOp::Eq,
                left,
                right,
            } = &fact
            else {
                continue;
            };
            // 两侧都试：命名变量可以在左也可以在右
            for (closed, named) in [
                (left.as_ref(), right.as_ref()),
                (right.as_ref(), left.as_ref()),
            ] {
                let ConstExpr::NamedVar(name) = named else {
                    continue;
                };
                if !crate::frontend::core::types::eval::const_eval::const_expr_free_names(closed)
                    .is_empty()
                {
                    // 两侧都有自由名（y == z）不是常量事实
                    continue;
                }
                let budget =
                    crate::frontend::core::sema::proof::budget::BudgetTracker::for_judgment();
                if let Ok(value) = crate::frontend::core::types::eval::const_eval::normalize(
                    closed,
                    &crate::frontend::core::types::eval::const_eval::TermEnv::default(),
                    &budget,
                ) {
                    out.push((name.clone(), value));
                }
            }
        }
        out
    }

    /// 当前边界可见的活跃假设（owned Vec，dead 的被过滤）。
    ///
    /// 向后兼容名：6.9.4 起口径与 visible() 相同（「当前 barrier 以上」），
    /// 不再有第二套可见性判据。
    pub fn current(&self) -> Vec<ConstExpr> {
        self.visible()
    }

    /// 某条假设对当前边界是否可见：活着且不在边界之外。
    fn is_visible(
        entry: &GammaEntry,
        barrier_depth: usize,
    ) -> bool {
        entry.alive && entry.scope_depth >= barrier_depth
    }

    /// 直接包含检查（Level 2a 快速路径）
    pub fn contains(
        &self,
        expr: &ConstExpr,
    ) -> bool {
        self.entries
            .iter()
            .any(|e| Self::is_visible(e, self.barrier_depth) && &e.expr == expr)
    }

    /// 当前边界以上是否**没有**事实（证据视角；不是「整栈都空」）。
    pub fn is_empty(&self) -> bool {
        !self
            .entries
            .iter()
            .any(|e| Self::is_visible(e, self.barrier_depth))
    }

    /// 向后兼容 —— 等价于 inject
    #[deprecated(note = "use inject() instead")]
    pub fn push(
        &mut self,
        cond: ConstExpr,
    ) {
        self.inject(cond);
    }

    /// 向后兼容 —— 等价于 exit_scope
    #[deprecated(note = "use exit_scope() instead")]
    pub fn pop(&mut self) {
        self.exit_scope();
    }
}

fn references_var(
    expr: &ConstExpr,
    var_name: &str,
) -> bool {
    match expr {
        ConstExpr::NamedVar(n) => n == var_name,
        ConstExpr::BinOp { left, right, .. } => {
            references_var(left, var_name) || references_var(right, var_name)
        }
        ConstExpr::UnOp { expr: operand, .. } => references_var(operand, var_name),
        ConstExpr::FieldPath { base, .. } => references_var(base, var_name),
        _ => false,
    }
}

/// 名字 → 此刻最内层绑定的身份（与值环境 `const_bindings` / `get_var_info`
/// **同一条**按名解析链：局部 → 参数 → 全局，捕获屏障已生效）。
///
/// 这是「绑定身份」的唯一来源：γ 的读取侧与两个注入点都走它，不另建名字表。
pub fn scope_binding_key(
    scope: &crate::frontend::core::sema::inference::scope::ScopeManager,
    name: &str,
) -> Option<BindingKey> {
    scope.get_var_info(name).map(|info| info.definition_span)
}

/// 为一条事实取**绑定身份快照**（自由名逐个按 [`scope_binding_key`] 解析）。
pub fn binding_snapshot(
    scope: &crate::frontend::core::sema::inference::scope::ScopeManager,
    fact: &ConstExpr,
) -> BindingSnapshot {
    crate::frontend::core::types::eval::const_eval::const_expr_free_names(fact)
        .into_iter()
        .map(|name| {
            let key = scope_binding_key(scope, &name);
            (name, key)
        })
        .collect()
}

/// 快照是否仍是**当前**绑定：逐名要求此刻解析到同一次绑定。
///
/// - 记录为 `None`（注入时无绑定）⇒ 要求此刻也无绑定；
/// - 记录为 dummy span（无可核定义事件）⇒ **不认为同一绑定**（fail-closed：
///   宁可少证，不可错证）；
/// - 其余 ⇒ 要求解析结果逐字相等。
fn bindings_are_current(
    bindings: &BindingSnapshot,
    resolve: &dyn Fn(&str) -> Option<BindingKey>,
) -> bool {
    bindings.iter().all(|(name, recorded)| match recorded {
        None => resolve(name).is_none(),
        Some(key) if key.is_dummy() => false,
        Some(key) => resolve(name) == Some(*key),
    })
}
