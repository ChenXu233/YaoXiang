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
use std::time::Instant;

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{FunctionIR, Instruction, ModuleIR, Operand};
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
    /// 平均深度
    pub avg_depth: Option<f64>,
    /// 消除率
    pub elimination_rate: f64,
    /// 膨胀阈值检查
    pub bloat_threshold: Option<usize>,
    /// 是否触发膨胀控制
    pub bloat_control_triggered: bool,
    /// 按函数名分组的实例数（用于膨胀分析）
    pub instances_by_function: HashMap<String, usize>,
    /// 调用频率统计
    pub call_frequencies: HashMap<String, usize>,
    /// 分析耗时（纳秒）
    pub analysis_time_ns: u64,
    /// 跨模块实例数
    pub cross_module_instances: usize,
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
        self.avg_depth = analysis.average_depth();
        self.elimination_rate = analysis.elimination_rate();
    }

    /// 添加函数实例统计
    pub fn add_function_instance(
        &mut self,
        func_name: &str,
    ) {
        self.function_instances += 1;
        *self
            .instances_by_function
            .entry(func_name.to_string())
            .or_insert(0) += 1;
    }

    /// 添加类型实例统计
    pub fn add_type_instance(
        &mut self,
        _type_name: &str,
    ) {
        self.type_instances += 1;
    }

    /// 记录调用频率
    pub fn record_call(
        &mut self,
        func_name: &str,
    ) {
        *self
            .call_frequencies
            .entry(func_name.to_string())
            .or_insert(0) += 1;
    }

    /// 设置膨胀控制状态
    pub fn set_bloat_control_status(
        &mut self,
        triggered: bool,
        threshold: usize,
    ) {
        self.bloat_control_triggered = triggered;
        self.bloat_threshold = Some(threshold);
    }

    /// 格式化统计信息（简洁版）
    pub fn format(&self) -> String {
        let bloat_info = if let Some(threshold) = self.bloat_threshold {
            format!(
                "\n  [Bloat Control] Threshold: {}, Triggered: {}",
                threshold,
                if self.bloat_control_triggered {
                    "Yes"
                } else {
                    "No"
                }
            )
        } else {
            String::new()
        };

        format!(
            "DCE Statistics:\n\
             === Instance Statistics ===\n\
             - Total instances: {}\n\
             - Eliminated: {} ({:.1}%)\n\
             - Kept: {}\n\
             - Function instances: {}\n\
             - Type instances: {}\n\
             === Graph Statistics ===\n\
             - Graph nodes: {}\n\
             - Graph edges: {}\n\
             - Entry points: {}\n\
             - Max depth: {:?}\n\
             - Avg depth: {:.1}{}\n\
             === Performance ===\n\
             - Analysis time: {} ns\n\
             - Cross-module instances: {}",
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
            self.avg_depth.unwrap_or(0.0),
            bloat_info,
            self.analysis_time_ns,
            self.cross_module_instances,
        )
    }

    /// 格式化统计信息（详细版）
    pub fn format_detailed(&self) -> String {
        let mut result = self.format();

        // 添加函数实例分布
        if !self.instances_by_function.is_empty() {
            result.push_str("\n\n=== Function Instance Distribution ===\n");
            let mut func_stats: Vec<_> = self.instances_by_function.iter().collect();
            func_stats.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

            for (name, count) in func_stats.iter().take(10) {
                result.push_str(&format!("  - {}: {}\n", name, count));
            }

            if func_stats.len() > 10 {
                result.push_str(&format!(
                    "  ... and {} more functions\n",
                    func_stats.len() - 10
                ));
            }
        }

        // 添加高频调用函数
        if !self.call_frequencies.is_empty() {
            result.push_str("\n=== Top Called Functions ===\n");
            let mut calls: Vec<_> = self.call_frequencies.iter().collect();
            calls.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

            for (name, count) in calls.iter().take(5) {
                result.push_str(&format!("  - {}: {} calls\n", name, count));
            }
        }

        result
    }

    /// 生成JSON格式统计信息
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "total_instances": self.total_instances,
            "eliminated_instances": self.eliminated_instances,
            "kept_instances": self.kept_instances,
            "function_instances": self.function_instances,
            "type_instances": self.type_instances,
            "graph_nodes": self.graph_nodes,
            "graph_edges": self.graph_edges,
            "entry_points": self.entry_points,
            "max_depth": self.max_depth,
            "avg_depth": self.avg_depth,
            "elimination_rate": self.elimination_rate,
            "bloat_threshold": self.bloat_threshold,
            "bloat_control_triggered": self.bloat_control_triggered,
            "analysis_time_ns": self.analysis_time_ns,
            "cross_module_instances": self.cross_module_instances,
            "function_distribution": self.instances_by_function,
            "call_frequencies": self.call_frequencies,
        }))
        .unwrap_or_else(|_| "{}".to_string())
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
        generic_functions: &HashMap<GenericFunctionId, FunctionIR>,
    ) -> DceResult {
        if !self.config.enabled {
            return DceResult {
                kept_functions: instantiated_functions.clone(),
                kept_types: instantiated_types.clone(),
                stats: std::mem::take(&mut self.stats),
                analysis: None,
            };
        }

        let start_time = Instant::now();

        // 1. 构建实例化图
        let mut graph = self.build_instantiation_graph(
            module,
            instantiated_functions,
            instantiated_types,
            generic_functions,
        );

        // 2. 标记入口点
        self.mark_entry_points(&mut graph, entry_points, generic_functions);

        // 3. 执行可达性分析
        let eliminator =
            DeadCodeEliminator::new().with_keep_entry_points(self.config.keep_entry_points);

        let (kept_nodes, analysis) = eliminator.eliminate_with_analysis(&graph);

        // 4. 收集保留的实例
        let kept_functions =
            self.collect_kept_functions(instantiated_functions, &kept_nodes, generic_functions);
        let kept_types = self.collect_kept_types(instantiated_types, &kept_nodes);

        // 5. 代码膨胀控制
        let bloat_triggered;
        let kept_functions = if self.config.enable_bloat_control {
            let before_count = kept_functions.len();
            let result = self.apply_bloat_control(kept_functions, instantiated_functions);
            bloat_triggered = result.len() < before_count;
            result
        } else {
            bloat_triggered = false;
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

        // 记录分析时间
        self.stats.analysis_time_ns = start_time.elapsed().as_nanos() as u64;

        // 设置膨胀控制状态
        if self.config.enable_bloat_control {
            self.stats
                .set_bloat_control_status(bloat_triggered, self.config.bloat_threshold);
        }

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
        generic_functions: &HashMap<GenericFunctionId, FunctionIR>,
    ) -> InstantiationGraph {
        let mut graph = InstantiationGraph::new();

        // 添加函数实例化节点
        for (func_id, ir) in instantiated_functions {
            let type_args = self.extract_function_type_args(func_id, ir);

            // 从特化名称提取基础泛型名称
            let base_name = self.extract_base_name(func_id.name());

            // 查找原始泛型函数以获取类型参数名
            let type_param_names =
                self.extract_type_param_names_from_generic(&base_name, generic_functions);

            let node = graph.add_function_node(
                GenericFunctionId::new(base_name, type_param_names),
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
        generic_functions: &HashMap<GenericFunctionId, FunctionIR>,
    ) {
        for entry in entry_points {
            let type_args = self.extract_entry_type_args(entry);

            // 从特化名称提取基础泛型名称
            let base_name = self.extract_base_name(entry.name());

            // 查找原始泛型函数以获取类型参数名
            let type_param_names =
                self.extract_type_param_names_from_generic(&base_name, generic_functions);

            let node =
                InstanceNode::Function(super::instantiation_graph::FunctionInstanceNode::new(
                    GenericFunctionId::new(base_name, type_param_names),
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
        generic_functions: &HashMap<GenericFunctionId, FunctionIR>,
    ) -> HashMap<FunctionId, FunctionIR> {
        let mut kept = HashMap::new();

        for (func_id, ir) in instantiated_functions {
            // 从特化名称提取基础泛型名称
            let base_name = self.extract_base_name(func_id.name());
            // 查找原始泛型函数以获取类型参数名
            let type_param_names =
                self.extract_type_param_names_from_generic(&base_name, generic_functions);

            let node =
                InstanceNode::Function(super::instantiation_graph::FunctionInstanceNode::new(
                    GenericFunctionId::new(base_name, type_param_names),
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
    ///
    /// 根据以下启发式规则估算函数被调用的频率：
    /// 1. 入口函数（main）优先级最高
    /// 2. 被多个函数调用的函数优先级更高
    /// 3. 函数体越小越可能被内联，优先级可以适当降低
    fn estimate_call_frequency(
        &self,
        func_id: &FunctionId,
        all: &HashMap<FunctionId, FunctionIR>,
    ) -> usize {
        let mut frequency = 1;

        // 1. 入口函数优先级最高
        if func_id.name() == "main" {
            frequency += 1000;
        }

        // 2. 统计被调用次数
        let mut call_count = 0;
        for (id, ir) in all {
            if id == func_id {
                continue;
            }
            // 检查函数体中是否调用了目标函数
            for inst in ir.all_instructions() {
                if self.inst_calls_function(inst, func_id) {
                    call_count += 1;
                    break;
                }
            }
        }
        frequency += call_count * 10;

        // 3. 小函数优先级略高（可能被内联）
        if let Some(ir) = all.get(func_id) {
            let size = ir
                .blocks
                .iter()
                .map(|b| b.instructions.len())
                .sum::<usize>();
            if size < 10 {
                frequency += 5;
            }
        }

        frequency
    }

    /// 检查指令是否调用了指定的函数
    fn inst_calls_function(
        &self,
        inst: &Instruction,
        func_id: &FunctionId,
    ) -> bool {
        match inst {
            Instruction::Call { func, .. } => {
                // 检查 func 是否引用了目标函数
                self.operand_references_function(func, func_id)
            }
            _ => false,
        }
    }

    /// 检查操作数是否引用了指定的函数
    fn operand_references_function(
        &self,
        operand: &Operand,
        func_id: &FunctionId,
    ) -> bool {
        match operand {
            Operand::Global(idx) => {
                // Global 索引指向全局表，需要进一步检查
                // 这里简化处理，假设 Global 0 可能是入口函数
                *idx == 0 && func_id.name() == "main"
            }
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    /// 从特化名称提取基础泛型名称
    ///
    /// 特化名称格式: "{base_name}_{type_args}"
    /// 例如: "map_Int" -> "map", "sum_Float_String" -> "sum"
    fn extract_base_name(
        &self,
        specialized_name: &str,
    ) -> String {
        // 尝试从后往前分割，提取基础名称
        // 处理格式: "map_Int", "sum_Float_String"
        if let Some(last_underscore) = specialized_name.rfind('_') {
            let potential_base = &specialized_name[..last_underscore];
            // 检查基础部分是否包含下划线，如果包含，说明后面还有类型参数
            // 需要继续往前找
            if potential_base.contains('_') {
                // 递归处理
                self.extract_base_name(potential_base)
            } else {
                // 找到基础名称
                potential_base.to_string()
            }
        } else {
            // 没有下划线，整个名称就是基础名称（可能是非泛型函数）
            specialized_name.to_string()
        }
    }

    /// 从函数ID提取类型参数
    fn extract_function_type_args(
        &self,
        func_id: &FunctionId,
        _ir: &FunctionIR,
    ) -> Vec<MonoType> {
        // 从 FunctionId 直接获取类型参数
        func_id.type_args().to_vec()
    }

    /// 从入口函数ID提取类型参数
    fn extract_entry_type_args(
        &self,
        func_id: &FunctionId,
    ) -> Vec<MonoType> {
        func_id.type_args().to_vec()
    }

    /// 从泛型函数映射中提取指定函数的类型参数名
    fn extract_type_param_names_from_generic(
        &self,
        base_name: &str,
        generic_functions: &HashMap<GenericFunctionId, FunctionIR>,
    ) -> Vec<String> {
        // 查找原始泛型函数
        for (generic_id, func_ir) in generic_functions {
            if generic_id.name() == base_name {
                return self.extract_type_params_from_ir(func_ir);
            }
        }
        // 如果找不到泛型函数，返回空列表
        vec![]
    }

    /// 从 FunctionIR 提取类型参数名
    fn extract_type_params_from_ir(
        &self,
        func: &FunctionIR,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for param_ty in &func.params {
            if let MonoType::TypeVar(tv) = param_ty {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
        }

        if let MonoType::TypeVar(tv) = &func.return_type {
            let name = format!("T{}", tv.index());
            if seen.insert(name.clone()) {
                type_params.push(name);
            }
        }

        type_params
    }

    /// 从类型提取类型参数
    ///
    /// 递归提取泛型类型的类型参数
    #[allow(clippy::only_used_in_recursion)]
    fn extract_type_args(
        &self,
        ty: &MonoType,
    ) -> Vec<MonoType> {
        match ty {
            // 基本类型没有类型参数
            MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes
            | MonoType::TypeVar(_)
            | MonoType::TypeRef(_) => vec![],

            // Arc 和 Weak：提取内部类型参数
            MonoType::Arc(t) | MonoType::Weak(t) => self.extract_type_args(t),

            // 联合和交集类型
            MonoType::Union(types) | MonoType::Intersection(types) => types
                .iter()
                .flat_map(|t| self.extract_type_args(t))
                .collect(),

            // 结构体：提取字段类型参数
            MonoType::Struct(s) => s
                .fields
                .iter()
                .map(|(_, ty)| ty)
                .flat_map(|t| self.extract_type_args(t))
                .collect(),

            // 枚举：没有类型参数
            MonoType::Enum(_) => vec![],

            // 容器类型
            MonoType::Tuple(types) => types
                .iter()
                .flat_map(|t| self.extract_type_args(t))
                .collect(),
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                self.extract_type_args(elem)
            }
            MonoType::Dict(key, value) => {
                let mut args = self.extract_type_args(key);
                args.extend(self.extract_type_args(value));
                args
            }

            // 函数类型
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                let mut args = params
                    .iter()
                    .flat_map(|t| self.extract_type_args(t))
                    .collect::<Vec<_>>();
                args.extend(self.extract_type_args(return_type));
                args
            }

            // 关联类型
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                let mut args = self.extract_type_args(host_type);
                args.extend(assoc_args.iter().cloned());
                args
            }

            // 字面量类型：没有类型参数
            MonoType::Literal { .. } => vec![],

            // 元类型：没有类型参数
            MonoType::MetaType { .. } => vec![],
        }
    }

    /// 将类型转换为实例化节点
    fn type_to_instance_node(
        &self,
        ty: &MonoType,
        _graph: &InstantiationGraph,
    ) -> Option<InstanceNode> {
        match ty {
            // 泛型列表
            MonoType::List(elem) => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new("List".to_string(), vec!["T".to_string()]),
                    vec![*elem.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型字典
            MonoType::Dict(key, value) => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new("Dict".to_string(), vec!["K".to_string(), "V".to_string()]),
                    vec![*key.clone(), *value.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Set
            MonoType::Set(elem) => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new("Set".to_string(), vec!["T".to_string()]),
                    vec![*elem.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Option (通过名称判断)
            MonoType::Enum(e) if e.name == "Option" => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new("Option".to_string(), vec!["T".to_string()]),
                    vec![],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Result (通过名称判断)
            MonoType::Enum(e) if e.name == "Result" => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new(
                        "Result".to_string(),
                        vec!["T".to_string(), "E".to_string()],
                    ),
                    vec![],
                );
                Some(InstanceNode::Type(node))
            }

            // 结构体
            MonoType::Struct(s) => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new(s.name.clone(), vec![]),
                    vec![],
                );
                Some(InstanceNode::Type(node))
            }

            // 其他枚举
            MonoType::Enum(e) => {
                let node = super::instantiation_graph::TypeInstanceNode::new(
                    GenericTypeId::new(e.name.clone(), vec![]),
                    vec![],
                );
                Some(InstanceNode::Type(node))
            }

            _ => None,
        }
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
