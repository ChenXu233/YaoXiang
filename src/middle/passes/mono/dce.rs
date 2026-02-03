//! 死代码消除 (Dead Code Elimination)
//!
//! 通过实例化图和可达性分析消除未使用的泛型实例。
//!
//! 工作流程：
//! 1. 构建实例化图
//! 2. 执行可达性分析
//! 3. 消除不可达实例
//! 4. 代码膨胀控制

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{FunctionIR, ModuleIR};
use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId, GenericTypeId, TypeId};

use super::instantiation_graph::{InstantiationGraph, InstanceNode};
use super::reachability::{DeadCodeEliminator, ReachabilityAnalysis};

/// DCE 配置
#[derive(Debug, Clone)]
pub struct DceConfig {
    /// 是否启用 DCE
    pub enabled: bool,
    /// 是否保留入口点
    pub keep_entry_points: bool,
    /// 是否保留导出函数
    pub keep_exported: bool,
    /// 是否启用代码膨胀控制
    pub enable_bloat_control: bool,
    /// 代码膨胀阈值（实例数量）
    pub bloat_threshold: usize,
    /// 是否输出统计信息
    pub print_stats: bool,
}

impl Default for DceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_entry_points: true,
            keep_exported: true,
            enable_bloat_control: true,
            bloat_threshold: 10000,
            print_stats: false,
        }
    }
}

impl DceConfig {
    /// 创建开发配置
    pub fn development() -> Self {
        Self {
            enabled: true,
            keep_entry_points: true,
            keep_exported: true,
            enable_bloat_control: false, // 开发时禁用膨胀控制
            bloat_threshold: 10000,
            print_stats: true,
        }
    }

    /// 创建发布配置
    pub fn release() -> Self {
        Self {
            enabled: true,
            keep_entry_points: true,
            keep_exported: true,
            enable_bloat_control: true,
            bloat_threshold: 5000, // 更严格的发布配置
            print_stats: false,
        }
    }
}

/// DCE 统计信息
#[derive(Debug, Default)]
pub struct DceStats {
    /// 总实例数
    pub total_instances: usize,
    /// 消除的实例数
    pub eliminated_instances: usize,
    /// 保留的实例数
    pub kept_instances: usize,
    /// 函数实例数
    pub function_instances: usize,
    /// 类型实例数
    pub type_instances: usize,
    /// 实例化图节点数
    pub graph_nodes: usize,
    /// 实例化图边数
    pub graph_edges: usize,
    /// 入口点数
    pub entry_points: usize,
    /// 最大深度
    pub max_depth: Option<usize>,
    /// 消除率
    pub elimination_rate: f64,
}

impl DceStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self::default()
    }

    /// 从分析结果更新统计
    pub fn update_from_analysis(
        &mut self,
        analysis: &ReachabilityAnalysis,
    ) {
        self.entry_points = analysis.entry_points().len();
        self.max_depth = analysis.max_depth();
        self.elimination_rate = analysis.elimination_rate();
    }

    /// 格式化统计信息
    pub fn format(&self) -> String {
        format!(
            "DCE Statistics:\n\
             - Total instances: {}\n\
             - Eliminated: {} ({:.1}%)\n\
             - Kept: {}\n\
             - Function instances: {}\n\
             - Type instances: {}\n\
             - Graph nodes: {}\n\
             - Graph edges: {}\n\
             - Entry points: {}\n\
             - Max depth: {:?}\n\
             - Elimination rate: {:.1}%",
            self.total_instances,
            self.eliminated_instances,
            self.elimination_rate * 100.0,
            self.kept_instances,
            self.function_instances,
            self.type_instances,
            self.graph_nodes,
            self.graph_edges,
            self.entry_points,
            self.max_depth,
            self.elimination_rate * 100.0,
        )
    }
}

/// 死代码消除器
#[derive(Debug)]
pub struct DcePass {
    /// 配置
    config: DceConfig,
    /// 统计信息
    stats: DceStats,
}

impl Default for DcePass {
    fn default() -> Self {
        Self::new(DceConfig::default())
    }
}

impl DcePass {
    /// 创建新的 DCE pass
    pub fn new(config: DceConfig) -> Self {
        Self {
            config,
            stats: DceStats::new(),
        }
    }

    /// 对模块执行 DCE
    pub fn run_on_module(
        &mut self,
        module: &ModuleIR,
        instantiated_functions: &HashMap<FunctionId, FunctionIR>,
        instantiated_types: &HashMap<TypeId, MonoType>,
        entry_points: &[FunctionId],
    ) -> DceResult {
        if !self.config.enabled {
            return DceResult {
                kept_functions: instantiated_functions.clone(),
                kept_types: instantiated_types.clone(),
                stats: std::mem::take(&mut self.stats),
                analysis: None,
            };
        }

        // 1. 构建实例化图
        let mut graph =
            self.build_instantiation_graph(module, instantiated_functions, instantiated_types);

        // 2. 标记入口点
        self.mark_entry_points(&mut graph, entry_points);

        // 3. 执行可达性分析
        let eliminator =
            DeadCodeEliminator::new().with_keep_entry_points(self.config.keep_entry_points);

        let (kept_nodes, analysis) = eliminator.eliminate_with_analysis(&graph);

        // 4. 收集保留的实例
        let kept_functions = self.collect_kept_functions(instantiated_functions, &kept_nodes);
        let kept_types = self.collect_kept_types(instantiated_types, &kept_nodes);

        // 5. 代码膨胀控制
        let kept_functions = if self.config.enable_bloat_control {
            self.apply_bloat_control(kept_functions, instantiated_functions)
        } else {
            kept_functions
        };

        // 6. 更新统计
        self.update_stats(
            instantiated_functions.len(),
            instantiated_types.len(),
            &graph,
            &analysis,
            kept_functions.len(),
            kept_types.len(),
        );

        // 7. 输出统计
        if self.config.print_stats {
            eprintln!("{}", self.stats.format());
        }

        DceResult {
            kept_functions,
            kept_types,
            stats: std::mem::take(&mut self.stats),
            analysis: Some(analysis),
        }
    }

    /// 构建实例化图
    fn build_instantiation_graph(
        &self,
        _module: &ModuleIR,
        instantiated_functions: &HashMap<FunctionId, FunctionIR>,
        instantiated_types: &HashMap<TypeId, MonoType>,
    ) -> InstantiationGraph {
        let mut graph = InstantiationGraph::new();

        // 添加函数实例化节点
        for (func_id, ir) in instantiated_functions {
            let type_args = self.extract_function_type_args(func_id, ir);
            let node = graph.add_function_node(
                GenericFunctionId::new(func_id.name().to_string(), vec![]),
                type_args,
            );

            // 从函数体提取依赖
            let deps = graph.extract_dependencies_from_function(ir);
            for dep in deps {
                if let Some(dep_node) = self.type_to_instance_node(&dep, &graph) {
                    graph.add_dependency(&InstanceNode::Function(node.clone()), &dep_node);
                }
            }
        }

        // 添加类型实例化节点
        for (type_id, ty) in instantiated_types {
            let type_args = self.extract_type_args(ty);
            let _node = graph.add_type_node(
                GenericTypeId::new(type_id.name().to_string(), vec![]),
                type_args,
            );
        }

        graph
    }

    /// 标记入口点
    fn mark_entry_points(
        &self,
        graph: &mut InstantiationGraph,
        entry_points: &[FunctionId],
    ) {
        for entry in entry_points {
            let type_args = self.extract_entry_type_args(entry);
            let node =
                InstanceNode::Function(super::instantiation_graph::FunctionInstanceNode::new(
                    GenericFunctionId::new(entry.name().to_string(), vec![]),
                    type_args,
                ));
            graph.add_entry_point(node);
        }
    }

    /// 收集保留的函数
    fn collect_kept_functions(
        &self,
        instantiated_functions: &HashMap<FunctionId, FunctionIR>,
        kept_nodes: &HashSet<InstanceNode>,
    ) -> HashMap<FunctionId, FunctionIR> {
        let mut kept = HashMap::new();

        for (func_id, ir) in instantiated_functions {
            let node =
                InstanceNode::Function(super::instantiation_graph::FunctionInstanceNode::new(
                    GenericFunctionId::new(func_id.name().to_string(), vec![]),
                    self.extract_function_type_args(func_id, ir),
                ));

            if kept_nodes.contains(&node) {
                kept.insert(func_id.clone(), ir.clone());
            }
        }

        kept
    }

    /// 收集保留的类型
    fn collect_kept_types(
        &self,
        instantiated_types: &HashMap<TypeId, MonoType>,
        kept_nodes: &HashSet<InstanceNode>,
    ) -> HashMap<TypeId, MonoType> {
        let mut kept = HashMap::new();

        for (type_id, ty) in instantiated_types {
            let node = InstanceNode::Type(super::instantiation_graph::TypeInstanceNode::new(
                GenericTypeId::new(type_id.name().to_string(), vec![]),
                self.extract_type_args(ty),
            ));

            if kept_nodes.contains(&node) {
                kept.insert(type_id.clone(), ty.clone());
            }
        }

        kept
    }

    /// 应用代码膨胀控制
    fn apply_bloat_control(
        &self,
        kept: HashMap<FunctionId, FunctionIR>,
        all: &HashMap<FunctionId, FunctionIR>,
    ) -> HashMap<FunctionId, FunctionIR> {
        if kept.len() <= self.config.bloat_threshold {
            return kept;
        }

        // 保留入口函数和被高频调用的函数
        let mut prioritized: Vec<_> = kept.into_iter().collect();
        prioritized.sort_by_key(|(id, _)| std::cmp::Reverse(self.estimate_call_frequency(id, all)));

        let keep_count = self.config.bloat_threshold;
        prioritized.into_iter().take(keep_count).collect()
    }

    /// 估算调用频率
    fn estimate_call_frequency(
        &self,
        _func_id: &FunctionId,
        _all: &HashMap<FunctionId, FunctionIR>,
    ) -> usize {
        // TODO: 实现调用频率估算
        1
    }

    /// 从函数ID提取类型参数
    fn extract_function_type_args(
        &self,
        _func_id: &FunctionId,
        _ir: &FunctionIR,
    ) -> Vec<MonoType> {
        // TODO: 从 FunctionId 和 FunctionIR 提取类型参数
        vec![]
    }

    /// 从入口函数ID提取类型参数
    fn extract_entry_type_args(
        &self,
        _func_id: &FunctionId,
    ) -> Vec<MonoType> {
        // TODO: 提取入口函数的类型参数
        vec![]
    }

    /// 从类型提取类型参数
    fn extract_type_args(
        &self,
        _ty: &MonoType,
    ) -> Vec<MonoType> {
        // TODO: 从 MonoType 提取类型参数
        vec![]
    }

    /// 将类型转换为实例化节点
    fn type_to_instance_node(
        &self,
        _ty: &MonoType,
        _graph: &InstantiationGraph,
    ) -> Option<InstanceNode> {
        // TODO: 实现类型到实例化节点的转换
        None
    }

    /// 更新统计信息
    fn update_stats(
        &mut self,
        total_functions: usize,
        total_types: usize,
        graph: &InstantiationGraph,
        analysis: &ReachabilityAnalysis,
        kept_functions: usize,
        kept_types: usize,
    ) {
        self.stats.total_instances = total_functions + total_types;
        self.stats.eliminated_instances =
            total_functions + total_types - kept_functions - kept_types;
        self.stats.kept_instances = kept_functions + kept_types;
        self.stats.function_instances = total_functions;
        self.stats.type_instances = total_types;
        self.stats.graph_nodes = graph.node_count();
        self.stats.graph_edges = graph.edge_count();
        self.stats.update_from_analysis(analysis);
    }
}

/// DCE 结果
#[derive(Debug)]
pub struct DceResult {
    /// 保留的函数实例
    pub kept_functions: HashMap<FunctionId, FunctionIR>,
    /// 保留的类型实例
    pub kept_types: HashMap<TypeId, MonoType>,
    /// 统计信息
    pub stats: DceStats,
    /// 分析结果（可选）
    pub analysis: Option<ReachabilityAnalysis>,
}

impl DceResult {
    /// 创建新的结果
    pub fn new() -> Self {
        Self {
            kept_functions: HashMap::new(),
            kept_types: HashMap::new(),
            stats: DceStats::new(),
            analysis: None,
        }
    }

    /// 检查是否有函数被消除
    pub fn has_eliminated_functions(&self) -> bool {
        self.stats.eliminated_instances > 0
    }

    /// 获取消除率
    pub fn elimination_rate(&self) -> f64 {
        self.stats.elimination_rate
    }
}

impl Default for DceResult {
    fn default() -> Self {
        Self::new()
    }
}

/// 跨模块 DCE
///
/// 分析模块间的依赖，消除跨模块的死代码
#[derive(Debug)]
pub struct CrossModuleDce {
    /// DCE 配置
    config: DceConfig,
    /// 模块 DCE 结果
    module_results: HashMap<PathBuf, DceResult>,
}

impl Default for CrossModuleDce {
    fn default() -> Self {
        Self::new(DceConfig::default())
    }
}

impl CrossModuleDce {
    /// 创建新的跨模块 DCE
    pub fn new(config: DceConfig) -> Self {
        Self {
            config,
            module_results: HashMap::new(),
        }
    }

    /// 注册模块的 DCE 结果
    pub fn register_module_result(
        &mut self,
        module_path: PathBuf,
        result: DceResult,
    ) {
        self.module_results.insert(module_path, result);
    }

    /// 执行跨模块 DCE
    ///
    /// 返回应该保留的跨模块实例
    pub fn run_cross_module(&self) -> HashSet<InstanceNode> {
        let mut kept: HashSet<InstanceNode> = HashSet::new();

        // 收集所有保留的实例
        for result in self.module_results.values() {
            for func_id in result.kept_functions.keys() {
                let node =
                    InstanceNode::Function(super::instantiation_graph::FunctionInstanceNode::new(
                        GenericFunctionId::new(func_id.name().to_string(), vec![]),
                        vec![],
                    ));
                kept.insert(node);
            }

            for type_id in result.kept_types.keys() {
                let node = InstanceNode::Type(super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new(type_id.name().to_string(), vec![]),
                    vec![],
                ));
                kept.insert(node);
            }
        }

        kept
    }

    /// 获取总统计信息
    pub fn total_stats(&self) -> DceStats {
        let mut total = DceStats::new();

        for result in self.module_results.values() {
            total.eliminated_instances += result.stats.eliminated_instances;
            total.kept_instances += result.stats.kept_instances;
            total.graph_nodes += result.stats.graph_nodes;
            total.graph_edges += result.stats.graph_edges;
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::passes::mono::instance::{FunctionId, TypeId};

    #[test]
    fn test_dce_pass() {
        let config = DceConfig::default();
        let mut dce = DcePass::new(config);

        let mut instantiated_functions = HashMap::new();
        instantiated_functions.insert(
            FunctionId::new("main".to_string(), vec![]),
            FunctionIR {
                name: "main".to_string(),
                params: vec![],
                return_type: MonoType::Void,
                is_async: false,
                locals: vec![],
                blocks: vec![],
                entry: 0,
            },
        );

        let result = dce.run_on_module(
            &ModuleIR::default(),
            &instantiated_functions,
            &HashMap::new(),
            &[FunctionId::new("main".to_string(), vec![])],
        );

        assert_eq!(result.kept_functions.len(), 1);
    }

    #[test]
    fn test_dce_result() {
        let result = DceResult::new();
        assert!(!result.has_eliminated_functions());
    }

    #[test]
    fn test_dce_config() {
        let dev_config = DceConfig::development();
        assert!(!dev_config.enable_bloat_control);
        assert!(dev_config.print_stats);

        let release_config = DceConfig::release();
        assert!(release_config.enable_bloat_control);
        assert!(!release_config.print_stats);
    }
}
