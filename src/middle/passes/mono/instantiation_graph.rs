//! 实例化图构建
//!
//! 跟踪泛型实例化的依赖关系，用于死代码消除。
//! 图的节点表示泛型实例化，边表示使用关系。

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::middle::passes::mono::instance::{
    FunctionId, GenericFunctionId, GenericTypeId, TypeId,
};

/// 实例化图中的节点
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum InstanceNode {
    /// 函数实例化节点
    Function(FunctionInstanceNode),
    /// 类型实例化节点
    Type(TypeInstanceNode),
}

impl InstanceNode {
    /// 获取节点的名称（用于调试）
    pub fn name(&self) -> String {
        match self {
            InstanceNode::Function(f) => f.specialized_name(),
            InstanceNode::Type(t) => t.specialized_name(),
        }
    }

    /// 检查是否是函数节点
    pub fn is_function(&self) -> bool {
        matches!(self, InstanceNode::Function(_))
    }

    /// 检查是否是类型节点
    pub fn is_type(&self) -> bool {
        matches!(self, InstanceNode::Type(_))
    }

    /// 转换为函数节点（如果可能）
    pub fn as_function(&self) -> Option<&FunctionInstanceNode> {
        match self {
            InstanceNode::Function(f) => Some(f),
            _ => None,
        }
    }

    /// 转换为类型节点（如果可能）
    pub fn as_type(&self) -> Option<&TypeInstanceNode> {
        match self {
            InstanceNode::Type(t) => Some(t),
            _ => None,
        }
    }
}

/// 函数实例化节点
#[derive(Debug, Clone)]
pub struct FunctionInstanceNode {
    /// 泛型函数ID
    pub generic_id: GenericFunctionId,
    /// 类型参数
    pub type_args: Vec<MonoType>,
    /// 生成的函数IR
    pub ir: Option<FunctionIR>,
}

impl FunctionInstanceNode {
    /// 创建新的函数实例化节点
    pub fn new(generic_id: GenericFunctionId, type_args: Vec<MonoType>) -> Self {
        Self {
            generic_id,
            type_args,
            ir: None,
        }
    }

    /// 获取特化后的名称
    pub fn specialized_name(&self) -> String {
        let type_args_str: Vec<String> = self
            .type_args
            .iter()
            .map(|t| t.type_name())
            .collect();
        format!("{}_{}", self.generic_id.name(), type_args_str.join("_"))
    }

    /// 获取类型参数的哈希键
    pub fn type_args_key(&self) -> Vec<String> {
        self.type_args.iter().map(|t| t.type_name()).collect()
    }
}

impl Hash for FunctionInstanceNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.generic_id.name().hash(state);
        self.type_args_key().hash(state);
    }
}

impl PartialEq for FunctionInstanceNode {
    fn eq(&self, other: &Self) -> bool {
        self.generic_id.name() == other.generic_id.name()
            && self.type_args_key() == other.type_args_key()
    }
}

impl Eq for FunctionInstanceNode {}

/// 类型实例化节点
#[derive(Debug, Clone)]
pub struct TypeInstanceNode {
    /// 泛型类型ID
    pub generic_id: GenericTypeId,
    /// 类型参数
    pub type_args: Vec<MonoType>,
    /// 生成的类型（如果有）
    pub mono_type: Option<MonoType>,
}

impl TypeInstanceNode {
    /// 创建新的类型实例化节点
    pub fn new(generic_id: GenericTypeId, type_args: Vec<MonoType>) -> Self {
        Self {
            generic_id,
            type_args,
            mono_type: None,
        }
    }

    /// 获取特化后的名称
    pub fn specialized_name(&self) -> String {
        let type_args_str: Vec<String> = self
            .type_args
            .iter()
            .map(|t| t.type_name())
            .collect();
        format!("{}_{}", self.generic_id.name(), type_args_str.join("_"))
    }

    /// 获取类型参数的哈希键
    pub fn type_args_key(&self) -> Vec<String> {
        self.type_args.iter().map(|t| t.type_name()).collect()
    }
}

impl Hash for TypeInstanceNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.generic_id.name().hash(state);
        self.type_args_key().hash(state);
    }
}

impl PartialEq for TypeInstanceNode {
    fn eq(&self, other: &Self) -> bool {
        self.generic_id.name() == other.generic_id.name()
            && self.type_args_key() == other.type_args_key()
    }
}

impl Eq for TypeInstanceNode {}

/// 实例化图
///
/// 存储所有泛型实例化及其依赖关系。
/// 用于死代码消除和代码膨胀控制。
#[derive(Debug, Default)]
pub struct InstantiationGraph {
    /// 所有节点
    nodes: HashSet<InstanceNode>,
    /// 邻接表：节点 -> 依赖的节点（该节点使用了哪些节点）
    edges: HashMap<InstanceNode, HashSet<InstanceNode>>,
    /// 逆向邻接表：节点 -> 被哪些节点使用
    reverse_edges: HashMap<InstanceNode, HashSet<InstanceNode>>,
    /// 入口点节点（main函数、导出的函数等）
    entry_points: HashSet<InstanceNode>,
}

impl InstantiationGraph {
    /// 创建新的实例化图
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            entry_points: HashSet::new(),
        }
    }

    /// 添加函数实例化节点
    pub fn add_function_node(
        &mut self,
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    ) -> FunctionInstanceNode {
        let node = InstanceNode::Function(FunctionInstanceNode::new(generic_id, type_args));
        self.nodes.insert(node.clone());
        self.edges.entry(node.clone()).or_insert_with(HashSet::new);
        self.reverse_edges
            .entry(node.clone())
            .or_insert_with(HashSet::new);

        // 返回函数节点（剥离包装）
        if let InstanceNode::Function(f) = node {
            f
        } else {
            unreachable!()
        }
    }

    /// 添加类型实例化节点
    pub fn add_type_node(
        &mut self,
        generic_id: GenericTypeId,
        type_args: Vec<MonoType>,
    ) -> TypeInstanceNode {
        let node = InstanceNode::Type(TypeInstanceNode::new(generic_id, type_args));
        self.nodes.insert(node.clone());
        self.edges.entry(node.clone()).or_insert_with(HashSet::new);
        self.reverse_edges
            .entry(node.clone())
            .or_insert_with(HashSet::new);

        // 返回类型节点（剥离包装）
        if let InstanceNode::Type(t) = node {
            t
        } else {
            unreachable!()
        }
    }

    /// 添加依赖边：A 使用 B
    ///
    /// 在实例化 A 时，需要实例化 B
    pub fn add_dependency(&mut self, user: &InstanceNode, used: &InstanceNode) {
        // 确保两个节点都存在
        if !self.nodes.contains(user) {
            self.nodes.insert(user.clone());
            self.edges.insert(user.clone(), HashSet::new());
        }
        if !self.nodes.contains(used) {
            self.nodes.insert(used.clone());
            self.edges.insert(used.clone(), HashSet::new());
        }

        // 添加边
        self.edges
            .get_mut(user)
            .expect("节点已存在")
            .insert(used.clone());

        // 添加逆向边
        self.reverse_edges
            .get_mut(used)
            .expect("节点已存在")
            .insert(user.clone());
    }

    /// 批量添加依赖边
    pub fn add_dependencies(&mut self, user: &InstanceNode, used: &[InstanceNode]) {
        for u in used {
            self.add_dependency(user, u);
        }
    }

    /// 标记入口点
    pub fn add_entry_point(&mut self, node: InstanceNode) {
        if self.nodes.contains(&node) {
            self.entry_points.insert(node);
        } else {
            self.nodes.insert(node.clone());
            self.edges.insert(node.clone(), HashSet::new());
            self.reverse_edges
                .entry(node.clone())
                .or_insert_with(HashSet::new);
            self.entry_points.insert(node);
        }
    }

    /// 从函数IR中提取依赖的类型
    pub fn extract_dependencies_from_function(&self, ir: &FunctionIR) -> Vec<MonoType> {
        let mut dependencies = Vec::new();

        for block in &ir.blocks {
            for inst in &block.instructions {
                self.extract_types_from_instruction(inst, &mut dependencies);
            }
        }

        dependencies
    }

    /// 从指令中提取类型
    fn extract_types_from_instruction(
        &self,
        inst: &Instruction,
        deps: &mut Vec<MonoType>,
    ) {
        match inst {
            // 函数调用
            Instruction::Call { func, args: _, dst: _ } => {
                // 从 func Operand 提取类型
                // 这需要解析 Operand 的类型信息
            }
            // 动态调用
            Instruction::CallDyn { func: _, args: _, dst: _ } => {
                // 从 func Operand 提取类型
            }
            // 虚方法调用
            Instruction::CallVirt {
                obj: _,
                method_name: _,
                args: _,
                dst: _,
            } => {
                // 从 obj 提取类型
            }
            // 返回指令可能包含类型
            Instruction::Ret(opt) => {
                if let Some(op) = opt {
                    // 从 Operand 提取类型
                    self.extract_type_from_operand(op, deps);
                }
            }
            // 分配指令包含类型信息
            Instruction::Alloc { size: _, dst } => {
                self.extract_type_from_operand(dst, deps);
            }
            // 数组分配包含元素类型信息（通过其他方式传递）
            Instruction::AllocArray {
                size: _,
                elem_size: _,
                dst,
            } => {
                self.extract_type_from_operand(dst, deps);
            }
            // 构造指令包含类型
            _ => {
                // 其他指令可能包含类型信息
            }
        }
    }

    /// 从操作数提取类型
    fn extract_type_from_operand(&self, _op: &Operand, _deps: &mut Vec<MonoType>) {
        // Operand 本身不直接包含类型信息
        // 类型信息需要从 FunctionIR.params 和 locals 获取
        // 这里留空实现，实际使用时需要传入类型上下文
    }

    /// 获取所有节点
    pub fn all_nodes(&self) -> &HashSet<InstanceNode> {
        &self.nodes
    }

    /// 获取入口点
    pub fn entry_points(&self) -> &HashSet<InstanceNode> {
        &self.entry_points
    }

    /// 获取节点的依赖（该节点使用了哪些节点）
    pub fn dependencies(&self, node: &InstanceNode) -> Option<&HashSet<InstanceNode>> {
        self.edges.get(node)
    }

    /// 获取使用该节点的节点（被哪些节点使用）
    pub fn dependents(&self, node: &InstanceNode) -> Option<&HashSet<InstanceNode>> {
        self.reverse_edges.get(node)
    }

    /// 获取节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取边数量
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|s| s.len()).sum()
    }

    /// 检查图是否为空
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 清空图
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.reverse_edges.clear();
        self.entry_points.clear();
    }
}

/// 实例化图构建器
///
/// 从模块IR和实例化信息构建实例化图
#[derive(Debug, Default)]
pub struct InstantiationGraphBuilder<'a> {
    /// 泛型函数映射：名称 -> IR
    generic_functions: HashMap<String, (GenericFunctionId, FunctionIR)>,
    /// 泛型类型映射：名称 -> 定义
    generic_types: HashMap<String, (GenericTypeId, MonoType)>,
    /// 引用外部图（用于跨模块分析）
    external_graph: Option<&'a InstantiationGraph>,
}

impl<'a> InstantiationGraphBuilder<'a> {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            generic_functions: HashMap::new(),
            generic_types: HashMap::new(),
            external_graph: None,
        }
    }

    /// 设置外部图（用于跨模块分析）
    pub fn with_external_graph(mut self, graph: &'a InstantiationGraph) -> Self {
        self.external_graph = Some(graph);
        self
    }

    /// 注册泛型函数
    pub fn register_generic_function(
        &mut self,
        generic_id: GenericFunctionId,
        ir: FunctionIR,
    ) {
        self.generic_functions
            .insert(generic_id.name().to_string(), (generic_id, ir));
    }

    /// 注册泛型类型
    pub fn register_generic_type(&mut self, generic_id: GenericTypeId, ty: MonoType) {
        self.generic_types
            .insert(generic_id.name().to_string(), (generic_id, ty));
    }

    /// 构建实例化图
    ///
    /// 从已实例化的函数和入口点构建图
    pub fn build_from_instantiations(
        &self,
        instantiated_functions: &HashMap<FunctionId, FunctionIR>,
        instantiated_types: &HashMap<TypeId, MonoType>,
        entry_functions: &[FunctionId],
    ) -> InstantiationGraph {
        let mut graph = InstantiationGraph::new();

        // 1. 添加所有实例化节点
        for (func_id, ir) in instantiated_functions {
            let generic_id = GenericFunctionId::new(
                func_id.name().to_string(),
                vec![], // TODO: 从 func_id 提取类型参数
            );
            let type_args = Self::extract_type_args_from_id(func_id);
            graph.add_function_node(generic_id, type_args);
        }

        for (type_id, ty) in instantiated_types {
            let generic_id = GenericTypeId::new(type_id.name().to_string(), vec![]);
            let type_args = Self::extract_type_args_from_type(ty);
            graph.add_type_node(generic_id, type_args);
        }

        // 2. 从函数体提取依赖
        for (func_id, ir) in instantiated_functions {
            let node = InstanceNode::Function(FunctionInstanceNode::new(
                GenericFunctionId::new(func_id.name().to_string(), vec![]),
                Self::extract_type_args_from_id(func_id),
            ));

            let deps = graph.extract_dependencies_from_function(ir);
            let dep_nodes: Vec<InstanceNode> = deps
                .iter()
                .filter_map(|ty| Self::type_to_instance_node(ty))
                .collect();

            graph.add_dependencies(&node, &dep_nodes);
        }

        // 3. 标记入口点
        for entry_func in entry_functions {
            let node = InstanceNode::Function(FunctionInstanceNode::new(
                GenericFunctionId::new(entry_func.name().to_string(), vec![]),
                Self::extract_type_args_from_id(entry_func),
            ));
            graph.add_entry_point(node);
        }

        graph
    }

    /// 从 FunctionId 提取类型参数
    fn extract_type_args_from_id(_id: &FunctionId) -> Vec<MonoType> {
        // TODO: 实现从 FunctionId 提取类型参数
        vec![]
    }

    /// 从 MonoType 提取类型参数
    fn extract_type_args_from_type(ty: &MonoType) -> Vec<MonoType> {
        // TODO: 实现从 MonoType 提取类型参数
        vec![]
    }

    /// 将类型转换为实例化节点
    fn type_to_instance_node(ty: &MonoType) -> Option<InstanceNode> {
        // TODO: 实现类型到实例化节点的转换
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId, GenericTypeId, TypeId};
    use crate::middle::passes::mono::reachability::ReachabilityAnalyzer;

    #[test]
    fn test_function_instance_node() {
        let generic_id = GenericFunctionId::new("map".to_string(), vec!["T".to_string()]);
        let type_args = vec![
            MonoType::Int(32),
            MonoType::String,
        ];

        let node = FunctionInstanceNode::new(generic_id, type_args.clone());
        assert_eq!(node.specialized_name(), "map_int32_string");

        let key = node.type_args_key();
        assert_eq!(key, vec!["int32".to_string(), "string".to_string()]);
    }

    #[test]
    fn test_type_instance_node() {
        let generic_id = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
        let type_args = vec![MonoType::Int(32)];

        let node = TypeInstanceNode::new(generic_id, type_args.clone());
        assert_eq!(node.specialized_name(), "Option_int32");
    }

    #[test]
    fn test_instantiation_graph() {
        let mut graph = InstantiationGraph::new();

        // 添加节点
        let map_int_string = graph.add_function_node(
            GenericFunctionId::new("map".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(32), MonoType::String],
        );

        let map_string_int = graph.add_function_node(
            GenericFunctionId::new("map".to_string(), vec!["T".to_string()]),
            vec![MonoType::String, MonoType::Int(32)],
        );

        let option_int = graph.add_type_node(
            GenericTypeId::new("Option".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(32)],
        );

        // 添加依赖边
        let map_int_string_node = InstanceNode::Function(map_int_string);
        let option_int_node = InstanceNode::Type(option_int);
        graph.add_dependency(&map_int_string_node, &option_int_node);

        // 验证
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 1);

        let deps = graph.dependencies(&map_int_string_node).unwrap();
        assert!(deps.contains(&option_int_node));
    }

    #[test]
    fn test_reachability() {
        let mut graph = InstantiationGraph::new();

        // 创建链式依赖：A -> B -> C
        let node_a = graph.add_function_node(
            GenericFunctionId::new("a".to_string(), vec![]),
            vec![],
        );
        let node_b = graph.add_function_node(
            GenericFunctionId::new("b".to_string(), vec![]),
            vec![],
        );
        let node_c = graph.add_function_node(
            GenericFunctionId::new("c".to_string(), vec![]),
            vec![],
        );

        let node_a = InstanceNode::Function(node_a);
        let node_b = InstanceNode::Function(node_b);
        let node_c = InstanceNode::Function(node_c);

        graph.add_dependency(&node_a, &node_b);
        graph.add_dependency(&node_b, &node_c);

        // 标记 A 为入口点
        graph.add_entry_point(node_a.clone());

        // 使用 BFS 进行可达性分析
        let analyzer = ReachabilityAnalyzer::new();
        let analysis = analyzer.analyze(&graph);

        assert!(analysis.is_reachable(&node_a));
        assert!(analysis.is_reachable(&node_b));
        assert!(analysis.is_reachable(&node_c));
    }
}
