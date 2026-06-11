//! 假设栈：当前程序点的路径条件集合
//!
//! RFC-027 §3.2-3.3：编译器在 if/match 分支中自动收集路径条件。
//! 阶段 1 只收集 if-guard。

use crate::frontend::core::types::const_data::ConstExpr;

/// 假设栈结构
#[derive(Debug, Default)]
pub struct AssumptionStack {
    assumptions: Vec<ConstExpr>,
}

impl AssumptionStack {
    pub fn new() -> Self {
        Self {
            assumptions: Vec::new(),
        }
    }

    /// 进入 if 分支时压入条件
    pub fn push(
        &mut self,
        cond: ConstExpr,
    ) {
        self.assumptions.push(cond);
    }

    /// 离开分支时弹出
    pub fn pop(&mut self) {
        self.assumptions.pop();
    }

    /// 当前所有活跃假设
    pub fn current(&self) -> &[ConstExpr] {
        &self.assumptions
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.assumptions.is_empty()
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

    #[test]
    fn test_push_pop() {
        let mut stack = AssumptionStack::new();
        assert!(stack.is_empty());
        stack.push(make_gt("y", 0));
        assert_eq!(stack.current().len(), 1);
        stack.pop();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_nested_push() {
        let mut stack = AssumptionStack::new();
        stack.push(make_gt("y", 0));
        stack.push(make_gt("z", 5));
        assert_eq!(stack.current().len(), 2);
        stack.pop();
        assert_eq!(stack.current().len(), 1);
    }
}
