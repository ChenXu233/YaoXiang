//! 模块依赖图
//!
//! 管理模块间的依赖关系，支持：
//! - 从 AST 的 `use` 语句构建依赖图
//! - 循环依赖检测
//! - 拓扑排序（确定编译顺序）
//! - 增量更新（单文件变更时只更新受影响的边）
//! - 影响分析（找出受变更影响的所有下游模块）

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

// ============ 核心数据结构 ============

/// 模块标识
///
/// 相等性基于 `name` 字段比较（模块名是唯一标识符）。
/// `path` 仅用于文件定位，不影响模块身份。
#[derive(Debug, Clone)]
pub struct ModuleId {
    /// 模块名（如 "std.io" 或 "my_module"）
    pub name: String,
    /// 对应的文件路径（标准库模块可能无文件路径）
    pub path: Option<PathBuf>,
}

impl PartialEq for ModuleId {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name
    }
}

impl Eq for ModuleId {}

impl std::hash::Hash for ModuleId {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
    }
}

impl ModuleId {
    /// 从名称创建模块 ID
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
        }
    }

    /// 从名称和路径创建模块 ID
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            path: Some(path.into()),
        }
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// 依赖边信息
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// 目标模块（被依赖的模块）
    pub target: ModuleId,
    /// 导入的具体项（None 表示导入整个模块）
    pub items: Option<Vec<String>>,
}

/// 模块依赖图
///
/// 有向图，边 A → B 表示模块 A 依赖模块 B。
/// 支持增量更新和循环依赖检测。
#[derive(Debug, Clone, Default)]
pub struct ModuleDependencyGraph {
    /// 模块 ID → 该模块依赖的模块列表（出边）
    deps: HashMap<ModuleId, Vec<DependencyEdge>>,
    /// 模块 ID → 依赖该模块的模块列表（入边/反向边）
    reverse_deps: HashMap<ModuleId, HashSet<ModuleId>>,
    /// 模块 ID → 导出的符号列表
    exports: HashMap<ModuleId, Vec<String>>,
    /// 所有已注册的模块
    modules: HashSet<ModuleId>,
}

impl ModuleDependencyGraph {
    /// 创建空的依赖图
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个模块到依赖图
    pub fn add_module(
        &mut self,
        module: ModuleId,
    ) {
        self.modules.insert(module.clone());
        self.deps.entry(module).or_default();
    }

    /// 添加依赖关系：from 依赖 edge.target
    pub fn add_dependency(
        &mut self,
        from: &ModuleId,
        edge: DependencyEdge,
    ) {
        self.modules.insert(from.clone());
        self.modules.insert(edge.target.clone());

        self.reverse_deps
            .entry(edge.target.clone())
            .or_default()
            .insert(from.clone());

        self.deps.entry(from.clone()).or_default().push(edge);
    }

    /// 设置模块的导出符号
    pub fn set_exports(
        &mut self,
        module: &ModuleId,
        exports: Vec<String>,
    ) {
        self.exports.insert(module.clone(), exports);
    }

    /// 获取模块的直接依赖
    pub fn get_dependencies(
        &self,
        module: &ModuleId,
    ) -> &[DependencyEdge] {
        self.deps.get(module).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 获取直接依赖某个模块的所有模块（反向依赖）
    pub fn get_dependents(
        &self,
        module: &ModuleId,
    ) -> Vec<&ModuleId> {
        self.reverse_deps
            .get(module)
            .map(|set| set.iter().collect())
            .unwrap_or_default()
    }

    /// 获取模块的导出符号
    pub fn get_exports(
        &self,
        module: &ModuleId,
    ) -> &[String] {
        self.exports
            .get(module)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取所有已注册的模块
    pub fn modules(&self) -> &HashSet<ModuleId> {
        &self.modules
    }

    /// 模块数量
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.deps.values().map(|v| v.len()).sum()
    }

    // ============ 增量更新 ============

    /// 更新单个模块的依赖关系
    ///
    /// 移除该模块的旧依赖，重新设置新依赖。
    /// 返回被影响的下游模块集合。
    pub fn update_module_deps(
        &mut self,
        module: &ModuleId,
        new_deps: Vec<DependencyEdge>,
    ) -> HashSet<ModuleId> {
        // 移除旧的反向依赖
        if let Some(old_deps) = self.deps.get(module) {
            for edge in old_deps {
                if let Some(rev) = self.reverse_deps.get_mut(&edge.target) {
                    rev.remove(module);
                }
            }
        }

        // 设置新依赖
        self.deps.insert(module.clone(), Vec::new());
        for edge in new_deps {
            self.add_dependency(module, edge);
        }

        // 返回受影响的模块（该模块自身 + 所有直接/间接依赖它的模块）
        self.get_all_dependents(module)
    }

    /// 移除模块及其所有依赖关系
    pub fn remove_module(
        &mut self,
        module: &ModuleId,
    ) {
        // 移除出边的反向引用
        if let Some(deps) = self.deps.remove(module) {
            for edge in deps {
                if let Some(rev) = self.reverse_deps.get_mut(&edge.target) {
                    rev.remove(module);
                }
            }
        }

        // 移除入边
        if let Some(dependents) = self.reverse_deps.remove(module) {
            for dep in dependents {
                if let Some(edges) = self.deps.get_mut(&dep) {
                    edges.retain(|e| e.target != *module);
                }
            }
        }

        self.exports.remove(module);
        self.modules.remove(module);
    }

    // ============ 循环依赖检测 ============

    /// 检测循环依赖
    ///
    /// 返回找到的所有环路。如果无环路，返回空列表。
    pub fn detect_cycles(&self) -> Vec<Vec<ModuleId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut cycles = Vec::new();
        let mut path = Vec::new();

        for module in &self.modules {
            if !visited.contains(module) {
                self.dfs_find_cycles(module, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// DFS 查找环路
    fn dfs_find_cycles(
        &self,
        node: &ModuleId,
        visited: &mut HashSet<ModuleId>,
        rec_stack: &mut HashSet<ModuleId>,
        path: &mut Vec<ModuleId>,
        cycles: &mut Vec<Vec<ModuleId>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(edges) = self.deps.get(node) {
            for edge in edges {
                if !visited.contains(&edge.target) {
                    self.dfs_find_cycles(&edge.target, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&edge.target) {
                    // 找到环路：从 edge.target 在 path 中的位置开始
                    if let Some(start) = path.iter().position(|n| n == &edge.target) {
                        let cycle: Vec<ModuleId> = path[start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    // ============ 拓扑排序 ============

    /// 拓扑排序
    ///
    /// 返回编译顺序：被依赖的模块排在前面。
    /// 如果存在循环依赖，返回 `Err(cycle)`。
    pub fn topological_sort(&self) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
        let cycles = self.detect_cycles();
        if !cycles.is_empty() {
            return Err(cycles.into_iter().next().unwrap());
        }

        // Kahn 算法
        let mut in_degree: HashMap<&ModuleId, usize> = HashMap::new();
        for module in &self.modules {
            in_degree.entry(module).or_insert(0);
        }

        for edges in self.deps.values() {
            for edge in edges {
                if self.modules.contains(&edge.target) {
                    // 只计算已注册模块的入度
                    // in_degree 的 key 是 from 模块，但这里我们需要 target 的入度
                }
            }
        }

        // 重新计算：入度 = 有多少模块依赖它
        // 但对于编译顺序，我们要反过来：入度 = 它依赖多少已注册的模块
        let mut dep_count: HashMap<&ModuleId, usize> = HashMap::new();
        for module in &self.modules {
            dep_count.insert(module, 0);
        }

        for (module, edges) in &self.deps {
            if self.modules.contains(module) {
                let count = edges
                    .iter()
                    .filter(|e| self.modules.contains(&e.target))
                    .count();
                dep_count.insert(module, count);
            }
        }

        let mut queue: VecDeque<&ModuleId> = dep_count
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(&module, _)| module)
            .collect();

        let mut result = Vec::new();

        while let Some(module) = queue.pop_front() {
            result.push(module.clone());

            // 对所有依赖该模块的模块，减少其依赖计数
            if let Some(dependents) = self.reverse_deps.get(module) {
                for dep in dependents {
                    if let Some(count) = dep_count.get_mut(dep) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    // ============ 影响分析 ============

    /// 获取所有直接和间接依赖某模块的模块（递归向上）
    ///
    /// 用于增量编译：当模块 M 变更时，所有返回的模块都需要重新编译。
    pub fn get_all_dependents(
        &self,
        module: &ModuleId,
    ) -> HashSet<ModuleId> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(module.clone());

        while let Some(current) = queue.pop_front() {
            if let Some(dependents) = self.reverse_deps.get(&current) {
                for dep in dependents {
                    if affected.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        affected
    }

    /// 根据变更文件列表，找出所有需要重新编译的模块
    ///
    /// 返回按拓扑排序的模块列表（依赖在前）。
    pub fn affected_modules(
        &self,
        changed_files: &[PathBuf],
    ) -> Vec<ModuleId> {
        let mut affected = HashSet::new();

        // 找出变更文件对应的模块
        for file in changed_files {
            for module in &self.modules {
                if module.path.as_ref() == Some(file) {
                    affected.insert(module.clone());
                    // 递归找出所有依赖该模块的模块
                    let dependents = self.get_all_dependents(module);
                    affected.extend(dependents);
                }
            }
        }

        // 按拓扑排序
        match self.topological_sort() {
            Ok(sorted) => sorted
                .into_iter()
                .filter(|m| affected.contains(m))
                .collect(),
            Err(_) => affected.into_iter().collect(),
        }
    }

    // ============ AST 构建 ============

    /// 从 AST 模块的 Use 语句构建依赖关系
    ///
    /// 解析 AST 中的所有 `use` 语句，提取依赖模块信息。
    pub fn build_from_ast(
        &mut self,
        module_id: &ModuleId,
        ast: &crate::frontend::core::parser::ast::Module,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;

        self.add_module(module_id.clone());

        // 收集导出符号
        let mut export_names = Vec::new();
        for stmt in &ast.items {
            match &stmt.kind {
                StmtKind::Binding {
                    name, is_pub: true, ..
                } => {
                    export_names.push(name.clone());
                }
                StmtKind::Binding {
                    name: _,
                    is_pub: false,
                    ..
                } => {
                    // 非公开绑定不导出
                }
                _ => {}
            }
        }
        self.set_exports(module_id, export_names);

        // 收集依赖（use 语句）
        for stmt in &ast.items {
            if let StmtKind::Use { path, items, .. } = &stmt.kind {
                let dep_id = ModuleId::from_name(path.clone());
                self.add_dependency(
                    module_id,
                    DependencyEdge {
                        target: dep_id,
                        items: items.clone(),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mod_id(name: &str) -> ModuleId {
        ModuleId::from_name(name)
    }

    fn mod_id_with_path(
        name: &str,
        path: &str,
    ) -> ModuleId {
        ModuleId::new(name, path)
    }

    // ============ 基础功能测试 ============

    #[test]
    fn test_empty_graph() {
        let graph = ModuleDependencyGraph::new();
        assert_eq!(graph.module_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_module() {
        let mut graph = ModuleDependencyGraph::new();
        graph.add_module(mod_id("main"));
        assert_eq!(graph.module_count(), 1);
        assert!(graph.modules().contains(&mod_id("main")));
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = ModuleDependencyGraph::new();
        let main = mod_id("main");
        let utils = mod_id("utils");

        graph.add_dependency(
            &main,
            DependencyEdge {
                target: utils.clone(),
                items: None,
            },
        );

        assert_eq!(graph.module_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.get_dependencies(&main).len(), 1);
        assert_eq!(graph.get_dependencies(&main)[0].target, utils);
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");
        let c = mod_id("c");

        // b → a, c → a
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &c,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );

        let dependents = graph.get_dependents(&a);
        assert_eq!(dependents.len(), 2);
    }

    #[test]
    fn test_set_and_get_exports() {
        let mut graph = ModuleDependencyGraph::new();
        let m = mod_id("mymod");
        graph.add_module(m.clone());
        graph.set_exports(&m, vec!["foo".to_string(), "bar".to_string()]);

        let exports = graph.get_exports(&m);
        assert_eq!(exports.len(), 2);
        assert!(exports.contains(&"foo".to_string()));
    }

    // ============ 单文件依赖测试 ============

    #[test]
    fn test_single_file_project() {
        let mut graph = ModuleDependencyGraph::new();
        graph.add_module(mod_id("main"));
        assert_eq!(graph.module_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.detect_cycles().is_empty());
    }

    // ============ 多文件依赖测试 ============

    #[test]
    fn test_multi_file_project() {
        let mut graph = ModuleDependencyGraph::new();
        let main = mod_id("main");
        let utils = mod_id("utils");
        let math = mod_id("math");

        // main → utils → math
        graph.add_dependency(
            &main,
            DependencyEdge {
                target: utils.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &utils,
            DependencyEdge {
                target: math.clone(),
                items: None,
            },
        );

        assert_eq!(graph.module_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }

    // ============ 循环依赖检测测试 ============

    #[test]
    fn test_no_cycle() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");

        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );

        assert!(graph.detect_cycles().is_empty());
    }

    #[test]
    fn test_direct_cycle() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");

        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_indirect_cycle() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");
        let c = mod_id("c");

        // a → b → c → a
        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: c.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &c,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }

    // ============ 拓扑排序测试 ============

    #[test]
    fn test_topological_sort_simple() {
        let mut graph = ModuleDependencyGraph::new();
        let main = mod_id("main");
        let utils = mod_id("utils");

        graph.add_dependency(
            &main,
            DependencyEdge {
                target: utils.clone(),
                items: None,
            },
        );

        let sorted = graph.topological_sort().unwrap();
        // utils 应该在 main 之前
        let utils_pos = sorted.iter().position(|m| m == &utils).unwrap();
        let main_pos = sorted.iter().position(|m| m == &main).unwrap();
        assert!(utils_pos < main_pos);
    }

    #[test]
    fn test_topological_sort_chain() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");
        let c = mod_id("c");

        // a → b → c
        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: c.clone(),
                items: None,
            },
        );

        let sorted = graph.topological_sort().unwrap();
        let c_pos = sorted.iter().position(|m| m == &c).unwrap();
        let b_pos = sorted.iter().position(|m| m == &b).unwrap();
        let a_pos = sorted.iter().position(|m| m == &a).unwrap();
        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_topological_sort_with_cycle_returns_error() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");

        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );

        assert!(graph.topological_sort().is_err());
    }

    // ============ 增量更新测试 ============

    #[test]
    fn test_update_module_deps() {
        let mut graph = ModuleDependencyGraph::new();
        let main = mod_id("main");
        let old_dep = mod_id("old_utils");
        let new_dep = mod_id("new_utils");

        // 初始: main → old_utils
        graph.add_dependency(
            &main,
            DependencyEdge {
                target: old_dep.clone(),
                items: None,
            },
        );

        assert_eq!(graph.get_dependencies(&main)[0].target, old_dep);

        // 更新: main → new_utils
        graph.update_module_deps(
            &main,
            vec![DependencyEdge {
                target: new_dep.clone(),
                items: None,
            }],
        );

        assert_eq!(graph.get_dependencies(&main).len(), 1);
        assert_eq!(graph.get_dependencies(&main)[0].target, new_dep);
        // old_utils 不再被 main 依赖
        assert!(graph.get_dependents(&old_dep).is_empty());
    }

    #[test]
    fn test_remove_module() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");

        graph.add_dependency(
            &a,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );

        graph.remove_module(&a);
        assert!(!graph.modules().contains(&a));
        assert!(graph.get_dependents(&b).is_empty());
    }

    // ============ 影响分析测试 ============

    #[test]
    fn test_get_all_dependents() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id("a");
        let b = mod_id("b");
        let c = mod_id("c");

        // c → b → a: 如果 a 变更，b 和 c 都受影响
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &c,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );

        let affected = graph.get_all_dependents(&a);
        assert!(affected.contains(&b));
        assert!(affected.contains(&c));
        assert!(!affected.contains(&a)); // 不包含自身
    }

    #[test]
    fn test_affected_modules_by_file_change() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id_with_path("a", "/src/a.yx");
        let b = mod_id_with_path("b", "/src/b.yx");
        let c = mod_id_with_path("c", "/src/c.yx");

        // c → b → a
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &c,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );

        let affected = graph.affected_modules(&[PathBuf::from("/src/a.yx")]);
        assert!(affected.iter().any(|m| m.name == "a"));
        assert!(affected.iter().any(|m| m.name == "b"));
        assert!(affected.iter().any(|m| m.name == "c"));
    }

    #[test]
    fn test_affected_modules_leaf_change() {
        let mut graph = ModuleDependencyGraph::new();
        let a = mod_id_with_path("a", "/src/a.yx");
        let b = mod_id_with_path("b", "/src/b.yx");
        let c = mod_id_with_path("c", "/src/c.yx");

        // c → b → a: 如果 c（叶子节点）变更，只有 c 需要重编
        graph.add_dependency(
            &b,
            DependencyEdge {
                target: a.clone(),
                items: None,
            },
        );
        graph.add_dependency(
            &c,
            DependencyEdge {
                target: b.clone(),
                items: None,
            },
        );

        let affected = graph.affected_modules(&[PathBuf::from("/src/c.yx")]);
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].name, "c");
    }
}
