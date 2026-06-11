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
    pub fn add_dep(&mut self, dependant: &str, dependency: &str) {
        self.edges
            .entry(dependency.to_string())
            .or_default()
            .insert(dependant.to_string());
    }

    /// 查询被某个变量变更影响的变量集合
    pub fn affected_by(&self, var: &str) -> Vec<&String> {
        self.edges
            .get(var)
            .map(|set| set.iter().collect())
            .unwrap_or_default()
    }

    /// 清除某个变量的所有依赖者（变量离开作用域时）
    pub fn remove_dependant(&mut self, dependant: &str) {
        for deps in self.edges.values_mut() {
            deps.remove(dependant);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_query() {
        let mut graph = TypeDepGraph::new();
        graph.add_dep("s", "i");
        graph.add_dep("t", "i");
        let affected = graph.affected_by("i");
        assert_eq!(affected.len(), 2);
        assert!(affected.iter().any(|s| s.as_str() == "s"));
        assert!(affected.iter().any(|s| s.as_str() == "t"));
    }

    #[test]
    fn test_no_deps() {
        let graph = TypeDepGraph::new();
        assert!(graph.affected_by("unknown").is_empty());
    }

    #[test]
    fn test_remove_dependant() {
        let mut graph = TypeDepGraph::new();
        graph.add_dep("s", "i");
        graph.add_dep("t", "i");
        graph.remove_dependant("s");
        let affected = graph.affected_by("i");
        assert_eq!(affected.len(), 1);
        assert!(affected.iter().any(|s| s.as_str() == "t"));
    }
}
