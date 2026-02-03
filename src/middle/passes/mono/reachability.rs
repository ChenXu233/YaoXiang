//! 可达性分析
//!
//! 从入口点分析哪些泛型实例是被使用的，用于死代码消除。

use std::collections::{HashMap, HashSet, VecDeque};

use super::instantiation_graph::{InstanceNode, InstantiationGraph};

/// 可达性分析结果
#[derive(Debug, Clone, Default)]
pub struct ReachabilityAnalysis {
    /// 可达的节点
    reachable: HashSet<InstanceNode>,
    /// 不可达的节点（死代码）
    unreachable: HashSet<InstanceNode>,
    /// 每个节点的深度（距离入口点的步数）
    depth: HashMap<InstanceNode, usize>,
    /// 被标记为入口点的节点
    entry_points: HashSet<InstanceNode>,
}

impl ReachabilityAnalysis {
    /// 创建新的分析结果
    pub fn new() -> Self {
        Self {
            reachable: HashSet::new(),
            unreachable: HashSet::new(),
            depth: HashMap::new(),
            entry_points: HashSet::new(),
        }
    }

    /// 获取可达节点
    pub fn reachable(&self) -> &HashSet<InstanceNode> {
        &self.reachable
    }

    /// 获取不可达节点
    pub fn unreachable(&self) -> &HashSet<InstanceNode> {
        &self.unreachable
    }

    /// 获取节点的深度
    pub fn depth(&self, node: &InstanceNode) -> Option<&usize> {
        self.depth.get(node)
    }

    /// 获取所有入口点
    pub fn entry_points(&self) -> &HashSet<InstanceNode> {
        &self.entry_points
    }

    /// 检查节点是否可达
    pub fn is_reachable(&self, node: &InstanceNode) -> bool {
        self.reachable.contains(node)
    }

    /// 检查节点是否不可达
    pub fn is_unreachable(&self, node: &InstanceNode) -> bool {
        self.unreachable.contains(node)
    }

    /// 获取最大深度
    pub fn max_depth(&self) -> Option<usize> {
        self.depth.values().max().copied()
    }

    /// 获取平均深度
    pub fn average_depth(&self) -> Option<f64> {
        if self.depth.is_empty() {
            return None;
        }
        let sum: usize = self.depth.values().sum();
        Some(sum as f64 / self.depth.len() as f64)
    }

    /// 获取可达节点数量
    pub fn reachable_count(&self) -> usize {
        self.reachable.len()
    }

    /// 获取不可达节点数量
    pub fn unreachable_count(&self) -> usize {
        self.unreachable.len()
    }

    /// 获取消除率
    pub fn elimination_rate(&self) -> f64 {
        let total = self.reachable.len() + self.unreachable.len();
        if total == 0 {
            0.0
        } else {
            self.unreachable.len() as f64 / total as f64
        }
    }
}

/// 可达性分析器
#[derive(Debug, Default)]
pub struct ReachabilityAnalyzer {}

impl ReachabilityAnalyzer {
    /// 创建新的分析器
    pub fn new() -> Self {
        Self {}
    }

    /// 分析实例化图的可达性
    ///
    /// 使用 BFS 从所有入口点开始遍历，标记所有可达节点
    pub fn analyze(&self, graph: &InstantiationGraph) -> ReachabilityAnalysis {
        let mut analysis = ReachabilityAnalysis::new();

        // 收集所有节点
        let all_nodes: HashSet<InstanceNode> = graph.all_nodes().clone();

        // 如果没有入口点，所有节点都是不可达的
        if graph.entry_points().is_empty() {
            analysis.unreachable = all_nodes;
            return analysis;
        }

        // BFS 遍历
        let mut queue: VecDeque<InstanceNode> = VecDeque::new();
        let mut visited: HashSet<InstanceNode> = HashSet::new();

        // 从所有入口点开始
        for entry in graph.entry_points() {
            queue.push_back(entry.clone());
            visited.insert(entry.clone());
            analysis.entry_points.insert(entry.clone());
            analysis.reachable.insert(entry.clone());
            analysis.depth.insert(entry.clone(), 0);
        }

        // BFS
        while let Some(current) = queue.pop_front() {
            let current_depth = *analysis
                .depth
                .get(&current)
                .expect("节点已在分析中");

            // 获取当前节点的依赖
            if let Some(dependencies) = graph.dependencies(&current) {
                for dep in dependencies {
                    if !visited.contains(dep) {
                        visited.insert(dep.clone());
                        analysis.reachable.insert(dep.clone());
                        analysis.depth.insert(dep.clone(), current_depth + 1);
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // 不可达节点 = 所有节点 - 可达节点
        analysis.unreachable = all_nodes
            .difference(&analysis.reachable)
            .cloned()
            .collect();

        analysis
    }

    /// 增量分析
    ///
    /// 当添加新节点后，局部更新可达性分析
    pub fn incremental_analyze(
        &self,
        graph: &InstantiationGraph,
        current_analysis: &ReachabilityAnalysis,
        new_nodes: &[InstanceNode],
    ) -> ReachabilityAnalysis {
        let mut analysis = current_analysis.clone();

        // 只对新节点进行 BFS
        let mut queue: VecDeque<InstanceNode> = VecDeque::new();
        let mut visited: HashSet<InstanceNode> = analysis.reachable.clone();

        for node in new_nodes {
            if !visited.contains(node) {
                queue.push_back(node.clone());
                visited.insert(node.clone());
            }
        }

        while let Some(current) = queue.pop_front() {
            let current_depth = analysis
                .depth
                .get(&current)
                .copied()
                .unwrap_or(0);

            if let Some(dependencies) = graph.dependencies(&current) {
                for dep in dependencies {
                    if !visited.contains(dep) {
                        visited.insert(dep.clone());
                        analysis.reachable.insert(dep.clone());
                        analysis.depth.insert(dep.clone(), current_depth + 1);
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // 更新不可达节点
        let all_nodes: HashSet<InstanceNode> = graph.all_nodes().clone();
        analysis.unreachable = all_nodes
            .difference(&analysis.reachable)
            .cloned()
            .collect();

        analysis
    }
}

/// 死代码消除器
///
/// 使用可达性分析结果消除未使用的泛型实例
#[derive(Debug)]
pub struct DeadCodeEliminator {
    /// 分析器
    analyzer: ReachabilityAnalyzer,
    /// 是否保留入口点（即使未被引用）
    keep_entry_points: bool,
    /// 保留的额外节点
    preserved_nodes: HashSet<InstanceNode>,
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadCodeEliminator {
    /// 创建新的消除器
    pub fn new() -> Self {
        Self {
            analyzer: ReachabilityAnalyzer::new(),
            keep_entry_points: true,
            preserved_nodes: HashSet::new(),
        }
    }

    /// 设置是否保留入口点
    pub fn with_keep_entry_points(mut self, keep: bool) -> Self {
        self.keep_entry_points = keep;
        self
    }

    /// 添加保留的节点
    pub fn preserve(&mut self, node: InstanceNode) {
        self.preserved_nodes.insert(node);
    }

    /// 批量添加保留的节点
    pub fn preserve_all(&mut self, nodes: &[InstanceNode]) {
        for node in nodes {
            self.preserved_nodes.insert(node.clone());
        }
    }

    /// 消除死代码
    ///
    /// 返回应该保留的节点
    pub fn eliminate(&self, graph: &InstantiationGraph) -> HashSet<InstanceNode> {
        let analysis = self.analyzer.analyze(graph);

        // 保留可达的节点
        let mut kept = analysis.reachable.clone();

        // 保留入口点（如果配置）
        if self.keep_entry_points {
            kept.extend(analysis.entry_points.clone());
        }

        // 保留额外指定的节点
        for node in &self.preserved_nodes {
            if graph.all_nodes().contains(node) {
                kept.insert(node.clone());
            }
        }

        kept
    }

    /// 消除死代码并获取分析结果
    pub fn eliminate_with_analysis(
        &self,
        graph: &InstantiationGraph,
    ) -> (HashSet<InstanceNode>, ReachabilityAnalysis) {
        let analysis = self.analyzer.analyze(graph);

        let mut kept = analysis.reachable.clone();

        if self.keep_entry_points {
            kept.extend(analysis.entry_points.clone());
        }

        for node in &self.preserved_nodes {
            if graph.all_nodes().contains(node) {
                kept.insert(node.clone());
            }
        }

        (kept, analysis)
    }

    /// 仅进行分析，不消除
    pub fn analyze(&self, graph: &InstantiationGraph) -> ReachabilityAnalysis {
        self.analyzer.analyze(graph)
    }
}

/// 代码膨胀估算器
///
/// 估算泛型实例化的代码膨胀程度
#[derive(Debug, Default)]
pub struct CodeBloatEstimator {
    /// 每个实例的平均大小（估算）
    average_instance_size: HashMap<String, usize>,
    /// 膨胀阈值（字节）
   膨胀_threshold: usize,
}

impl CodeBloatEstimator {
    /// 创建新的估算器
    pub fn new() -> Self {
        Self {
            average_instance_size: HashMap::new(),
            膨胀_threshold: 10 * 1024 * 1024, // 默认 10MB
        }
    }

    /// 设置膨胀阈值
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.膨胀_threshold = threshold;
        self
    }

    /// 注册实例大小
    pub fn register_instance_size(&mut self, generic_name: &str, size: usize) {
        self.average_instance_size
            .insert(generic_name.to_string(), size);
    }

    /// 估算代码膨胀
    pub fn estimate_bloat(
        &self,
        graph: &InstantiationGraph,
        instances: &HashSet<InstanceNode>,
    ) -> usize {
        let mut total_size = 0;

        for node in instances {
            let name = node.name();
            if let Some(size) = self.average_instance_size.get(&name) {
                total_size += size;
            } else {
                // 默认估算：每个实例约 100 字节
                total_size += 100;
            }
        }

        total_size
    }

    /// 检查是否超过膨胀阈值
    pub fn exceeds_threshold(&self, graph: &InstantiationGraph, instances: &HashSet<InstanceNode>) -> bool {
        self.estimate_bloat(graph, instances) > self.膨胀_threshold
    }
}

/// 跨模块可达性分析
///
/// 分析模块间的实例化依赖
#[derive(Debug)]
pub struct CrossModuleReachabilityAnalyzer {
    /// 模块图：模块名 -> 实例化图
    module_graphs: HashMap<String, InstantiationGraph>,
    /// 模块导出：模块名 -> 导出的实例
    module_exports: HashMap<String, HashSet<InstanceNode>>,
    /// 模块导入：模块名 -> 使用的外部实例
    module_imports: HashMap<String, HashSet<InstanceNode>>,
}

impl Default for CrossModuleReachabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossModuleReachabilityAnalyzer {
    /// 创建新的分析器
    pub fn new() -> Self {
        Self {
            module_graphs: HashMap::new(),
            module_exports: HashMap::new(),
            module_imports: HashMap::new(),
        }
    }

    /// 注册模块
    pub fn register_module(
        &mut self,
        module_name: String,
        graph: InstantiationGraph,
        exports: HashSet<InstanceNode>,
        imports: HashSet<InstanceNode>,
    ) {
        self.module_graphs.insert(module_name.clone(), graph);
        self.module_exports.insert(module_name.clone(), exports);
        self.module_imports.insert(module_name, imports);
    }

    /// 分析跨模块可达性
    pub fn analyze_cross_module(&self) -> HashMap<String, HashSet<InstanceNode>> {
        let mut module_reachable: HashMap<String, HashSet<InstanceNode>> = HashMap::new();

        // TODO: 实现跨模块分析
        // 1. 从入口模块开始（包含 main 的模块）
        // 2. 递归分析依赖的模块
        // 3. 收集所有可达实例

        module_reachable
    }

    /// 获取模块的可达实例
    pub fn get_module_reachable(&self, module_name: &str) -> Option<&HashSet<InstanceNode>> {
        self.module_graphs
            .get(module_name)
            .map(|g| g.all_nodes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::instantiation_graph::FunctionInstanceNode;
    use crate::middle::passes::mono::instance::GenericFunctionId;

    fn create_test_graph() -> InstantiationGraph {
        let mut graph = InstantiationGraph::new();

        // main -> foo -> bar
        //      \-> baz
        // 其中 bar 是死代码（未被任何入口点引用）

        let main = InstanceNode::Function(FunctionInstanceNode::new(
            GenericFunctionId::new("main".to_string(), vec![]),
            vec![],
        ));

        let foo = InstanceNode::Function(FunctionInstanceNode::new(
            GenericFunctionId::new("foo".to_string(), vec![]),
            vec![],
        ));

        let bar = InstanceNode::Function(FunctionInstanceNode::new(
            GenericFunctionId::new("bar".to_string(), vec![]),
            vec![],
        ));

        let baz = InstanceNode::Function(FunctionInstanceNode::new(
            GenericFunctionId::new("baz".to_string(), vec![]),
            vec![],
        ));

        graph.add_function_node(GenericFunctionId::new("main".to_string(), vec![]), vec![]);
        graph.add_function_node(GenericFunctionId::new("foo".to_string(), vec![]), vec![]);
        graph.add_function_node(GenericFunctionId::new("bar".to_string(), vec![]), vec![]);
        graph.add_function_node(GenericFunctionId::new("baz".to_string(), vec![]), vec![]);

        graph.add_dependency(&main, &foo);
        graph.add_dependency(&main, &baz);
        graph.add_dependency(&foo, &bar); // bar 只被 foo 引用，但 foo 被 main 引用

        // bar 未被任何入口点直接引用，但我们通过 foo 引用了它
        // 所以实际上 bar 也应该是可达的

        graph.add_entry_point(main);

        graph
    }

    #[test]
    fn test_reachability_analysis() {
        let graph = create_test_graph();
        let analyzer = ReachabilityAnalyzer::new();
        let analysis = analyzer.analyze(&graph);

        assert_eq!(analysis.reachable_count(), 4);
        assert_eq!(analysis.unreachable_count(), 0);
    }

    #[test]
    fn test_dead_code_elimination() {
        let graph = create_test_graph();
        let eliminator = DeadCodeEliminator::new();
        let kept = eliminator.eliminate(&graph);

        assert_eq!(kept.len(), 4);
    }

    #[test]
    fn test_elimination_rate() {
        let graph = create_test_graph();
        let analyzer = ReachabilityAnalyzer::new();
        let analysis = analyzer.analyze(&graph);

        assert_eq!(analysis.elimination_rate(), 0.0); // 没有死代码
    }

    #[test]
    fn test_depth_analysis() {
        let graph = create_test_graph();
        let analyzer = ReachabilityAnalyzer::new();
        let analysis = analyzer.analyze(&graph);

        // main 深度为 0
        // foo 和 baz 深度为 1
        // bar 深度为 2

        assert_eq!(analysis.max_depth(), Some(2));
    }
}
