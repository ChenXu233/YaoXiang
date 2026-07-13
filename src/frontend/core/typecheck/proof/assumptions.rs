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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::const_data::{BinOp, ConstValue};

    fn make_gt(
        var: &str,
        n: i128,
    ) -> ConstExpr {
        ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar(var.into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
        }
    }

    // === 原有 test 改为 inject/exit_scope ===

    #[test]
    fn test_push_pop() {
        let mut stack = AssumptionStack::new();
        assert!(stack.is_empty());
        stack.enter_scope();
        stack.inject(make_gt("y", 0));
        assert_eq!(stack.current().len(), 1);
        stack.exit_scope();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_nested_push() {
        let mut stack = AssumptionStack::new();
        stack.enter_scope();
        stack.inject(make_gt("y", 0));
        stack.inject(make_gt("z", 5));
        assert_eq!(stack.current().len(), 2);
        stack.exit_scope();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_contains_true() {
        let mut stack = AssumptionStack::new();
        let cond = make_gt("y", 0);
        stack.inject(cond.clone());
        assert!(stack.contains(&cond));
    }

    #[test]
    fn test_contains_false() {
        let mut stack = AssumptionStack::new();
        stack.inject(make_gt("y", 0));
        assert!(!stack.contains(&make_gt("z", 5)));
    }

    #[test]
    fn test_contains_empty_stack() {
        let stack = AssumptionStack::new();
        assert!(!stack.contains(&make_gt("y", 0)));
    }

    // === 7 个新 test ===

    #[test]
    fn test_gamma_inject_and_current() {
        let mut gamma = FlowSensitiveGamma::new();
        assert!(gamma.is_empty());
        gamma.inject(make_gt("x", 0));
        gamma.inject(make_gt("y", 5));
        let alive = gamma.current();
        assert_eq!(alive.len(), 2);
        assert!(alive.contains(&make_gt("x", 0)));
        assert!(alive.contains(&make_gt("y", 5)));
    }

    #[test]
    fn test_gamma_kill_removes_dependent() {
        let mut gamma = FlowSensitiveGamma::new();
        gamma.inject(make_gt("x", 0));
        gamma.inject(make_gt("x", 5));
        assert_eq!(gamma.current().len(), 2);
        gamma.kill("x");
        assert!(gamma.current().is_empty());
    }

    #[test]
    fn test_gamma_kill_preserves_independent() {
        let mut gamma = FlowSensitiveGamma::new();
        gamma.inject(make_gt("x", 0));
        gamma.inject(make_gt("y", 5));
        assert_eq!(gamma.current().len(), 2);
        gamma.kill("x");
        let alive = gamma.current();
        assert_eq!(alive.len(), 1);
        assert!(alive.contains(&make_gt("y", 5)));
    }

    #[test]
    fn test_gamma_exit_scope_removes_all() {
        let mut gamma = FlowSensitiveGamma::new();
        gamma.enter_scope();
        gamma.inject(make_gt("x", 0));
        gamma.inject(make_gt("y", 5));
        assert_eq!(gamma.current().len(), 2);
        gamma.exit_scope();
        assert!(gamma.is_empty());
    }

    #[test]
    fn test_gamma_contains_ignores_dead() {
        let mut gamma = FlowSensitiveGamma::new();
        let cond = make_gt("x", 0);
        gamma.inject(cond.clone());
        assert!(gamma.contains(&cond));
        gamma.kill("x");
        assert!(!gamma.contains(&cond));
    }

    #[test]
    fn test_gamma_is_empty_with_dead() {
        let mut gamma = FlowSensitiveGamma::new();
        gamma.inject(make_gt("x", 0));
        assert!(!gamma.is_empty());
        gamma.kill("x");
        assert!(gamma.is_empty());
    }

    #[test]
    fn test_gamma_nested_scope_kill_only_inner() {
        let mut gamma = FlowSensitiveGamma::new();
        gamma.inject(make_gt("x", 0));
        gamma.enter_scope();
        gamma.inject(make_gt("x", 5));
        gamma.kill("x");
        // 两层的 x 都被 kill 了
        assert!(gamma.current().is_empty());
        gamma.exit_scope();
        // 外层 scope 的 x 也是 dead
        assert!(gamma.current().is_empty());
    }
}
