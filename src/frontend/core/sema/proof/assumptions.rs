//! 流敏感假设集 Γ —— 当前控制流路径上已成立的命题集合
//!
//! RFC-027 §3.2-3.3 + spec §4.1-4.2：
//! - 不可变绑定：假设永久有效，Γ 单调只增
//! - mut 变量赋值：kill 所有依赖该变量的假设（标记为 dead）
//! - 分支结束：exit_scope 移除当前 scope 内的所有假设

use crate::frontend::core::types::const_data::ConstExpr;

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
}

#[derive(Debug, Clone)]
struct GammaEntry {
    expr: ConstExpr,
    alive: bool,
    scope_depth: usize,
}

/// 向后兼容别名
pub type AssumptionStack = FlowSensitiveGamma;

impl FlowSensitiveGamma {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scope_depth: 0,
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
        self.entries.push(GammaEntry {
            expr: cond,
            alive: true,
            scope_depth: self.scope_depth,
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

    /// 当前所有活跃假设（owned Vec，dead 的被过滤）
    pub fn current(&self) -> Vec<ConstExpr> {
        self.entries
            .iter()
            .filter(|e| e.alive)
            .map(|e| e.expr.clone())
            .collect()
    }

    /// 直接包含检查（Level 2a 快速路径）
    pub fn contains(
        &self,
        expr: &ConstExpr,
    ) -> bool {
        self.entries.iter().any(|e| e.alive && &e.expr == expr)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|e| !e.alive)
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
        _ => false,
    }
}
