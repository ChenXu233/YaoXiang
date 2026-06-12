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
