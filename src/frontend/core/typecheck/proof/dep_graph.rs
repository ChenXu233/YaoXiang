//! 类型依赖图：记录变量间的类型标注依赖关系
//!
//! RFC-027 §6.1：当 mut v: Pred(... x ...) 的类型标注引用了 x，
//! x 变更时需要重验证 v。阶段 1 只构建图，重验证在阶段 3 实现。

use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct TypeDepGraph {
    /// 被依赖变量 → 依赖它的变量集合
    /// 例：{ "i": {"s", "t"}, "j": {"s"} }
    edges: HashMap<String, HashSet<String>>,
}

impl TypeDepGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// 记录依赖：`dependant` 的类型标注引用了 `dependency`
    /// dependency 变更时 dependant 需要重验证
    pub fn add_dep(
        &mut self,
        dependant: &str,
        dependency: &str,
    ) {
        self.edges
            .entry(dependency.to_string())
            .or_default()
            .insert(dependant.to_string());
    }

    /// 查询被某个变量变更影响的变量集合
    pub fn affected_by(
        &self,
        var: &str,
    ) -> Vec<&str> {
        self.edges
            .get(var)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// 清除某个变量的所有依赖者（变量离开作用域时）
    pub fn remove_dependant(
        &mut self,
        dependant: &str,
    ) {
        for deps in self.edges.values_mut() {
            deps.remove(dependant);
        }
    }
}
