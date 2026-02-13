//! 实例化图构建
//!
//! 跟踪泛型实例化的依赖关系，用于死代码消除。
//! 图的节点表示泛型实例化，边表示使用关系。

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{ConstValue, FunctionIR, Instruction, Operand};
use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId, GenericTypeId, TypeId};

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
    pub fn new(
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    ) -> Self {
        Self {
            generic_id,
            type_args,
            ir: None,
        }
    }

    /// 获取特化后的名称
    pub fn specialized_name(&self) -> String {
        let type_args_str: Vec<String> = self.type_args.iter().map(|t| t.type_name()).collect();
        format!("{}_{}", self.generic_id.name(), type_args_str.join("_"))
    }

    /// 获取类型参数的哈希键
    pub fn type_args_key(&self) -> Vec<String> {
        self.type_args.iter().map(|t| t.type_name()).collect()
    }
}

impl Hash for FunctionInstanceNode {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.generic_id.name().hash(state);
        self.type_args_key().hash(state);
    }
}

impl PartialEq for FunctionInstanceNode {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
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
    pub fn new(
        generic_id: GenericTypeId,
        type_args: Vec<MonoType>,
    ) -> Self {
        Self {
            generic_id,
            type_args,
            mono_type: None,
        }
    }

    /// 获取特化后的名称
    pub fn specialized_name(&self) -> String {
        let type_args_str: Vec<String> = self.type_args.iter().map(|t| t.type_name()).collect();
        format!("{}_{}", self.generic_id.name(), type_args_str.join("_"))
    }

    /// 获取类型参数的哈希键
    pub fn type_args_key(&self) -> Vec<String> {
        self.type_args.iter().map(|t| t.type_name()).collect()
    }
}

impl Hash for TypeInstanceNode {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.generic_id.name().hash(state);
        self.type_args_key().hash(state);
    }
}

impl PartialEq for TypeInstanceNode {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
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
        self.edges.entry(node.clone()).or_default();
        self.reverse_edges.entry(node.clone()).or_default();

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
        self.edges.entry(node.clone()).or_default();
        self.reverse_edges.entry(node.clone()).or_default();

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
    pub fn add_dependency(
        &mut self,
        user: &InstanceNode,
        used: &InstanceNode,
    ) {
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
    pub fn add_dependencies(
        &mut self,
        user: &InstanceNode,
        used: &[InstanceNode],
    ) {
        for u in used {
            self.add_dependency(user, u);
        }
    }

    /// 标记入口点
    pub fn add_entry_point(
        &mut self,
        node: InstanceNode,
    ) {
        if self.nodes.contains(&node) {
            self.entry_points.insert(node);
        } else {
            self.nodes.insert(node.clone());
            self.edges.insert(node.clone(), HashSet::new());
            self.reverse_edges.entry(node.clone()).or_default();
            self.entry_points.insert(node);
        }
    }

    /// 从函数IR中提取依赖的类型
    ///
    /// 需要传入参数类型和局部变量类型才能正确提取
    pub fn extract_dependencies_from_function(
        &self,
        ir: &FunctionIR,
    ) -> Vec<MonoType> {
        let mut dependencies = Vec::new();

        for block in &ir.blocks {
            for inst in &block.instructions {
                self.extract_types_from_instruction(
                    inst,
                    &ir.params,
                    &ir.locals,
                    &mut dependencies,
                );
            }
        }

        dependencies
    }

    /// 从指令中提取类型
    ///
    /// # Arguments
    /// * `inst` - 指令
    /// * `params` - 函数参数类型
    /// * `locals` - 局部变量类型
    /// * `deps` - 收集依赖类型
    fn extract_types_from_instruction(
        &self,
        inst: &Instruction,
        params: &[MonoType],
        locals: &[MonoType],
        deps: &mut Vec<MonoType>,
    ) {
        match inst {
            // ==================== 调用指令 ====================
            // 函数调用 (dst 是 Option<Operand>)
            Instruction::Call { func, args, dst } => {
                self.extract_type_from_operand(func, params, locals, deps);
                if let Some(d) = dst {
                    self.extract_type_from_operand(d, params, locals, deps);
                }
                for arg in args {
                    self.extract_type_from_operand(arg, params, locals, deps);
                }
            }
            // 动态调用 (dst 是 Option<Operand>)
            Instruction::CallDyn { func, args, dst } => {
                self.extract_type_from_operand(func, params, locals, deps);
                if let Some(d) = dst {
                    self.extract_type_from_operand(d, params, locals, deps);
                }
                for arg in args {
                    self.extract_type_from_operand(arg, params, locals, deps);
                }
            }
            // 虚方法调用 (dst 是 Option<Operand>)
            Instruction::CallVirt { obj, args, dst, .. } => {
                self.extract_type_from_operand(obj, params, locals, deps);
                if let Some(d) = dst {
                    self.extract_type_from_operand(d, params, locals, deps);
                }
                for arg in args {
                    self.extract_type_from_operand(arg, params, locals, deps);
                }
            }

            // ==================== 内存操作指令 ====================
            // 分配指令
            Instruction::Alloc { dst, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
            }
            // 数组分配
            Instruction::AllocArray { dst, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
            }
            // 堆分配（包含类型ID）
            Instruction::HeapAlloc { dst, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
            }
            // 释放
            Instruction::Free(op) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }

            // ==================== 字段操作指令 ====================
            // 加载字段
            Instruction::LoadField { dst, src, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // 存储字段
            Instruction::StoreField { dst, src, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // 加载索引
            Instruction::LoadIndex { dst, src, index } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
                self.extract_type_from_operand(index, params, locals, deps);
            }
            // 存储索引
            Instruction::StoreIndex { dst, index, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(index, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }

            // ==================== 闭包指令 ====================
            // 创建闭包
            Instruction::MakeClosure { dst, env, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                for e in env {
                    self.extract_type_from_operand(e, params, locals, deps);
                }
            }

            // ==================== 引用计数指令 ====================
            // Arc new
            Instruction::ArcNew { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // Arc clone
            Instruction::ArcClone { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }

            // ==================== 类型转换指令 ====================
            // 类型转换
            Instruction::Cast { dst, src, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }

            // ==================== 其他指令 ====================
            // 移动
            Instruction::Move { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // 加载
            Instruction::Load { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // 存储
            Instruction::Store { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }
            // Push
            Instruction::Push(op) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }
            // Dup/Swap - 无操作数
            Instruction::Dup | Instruction::Swap => {}
            // Drop
            Instruction::Drop(op) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }
            // 返回指令
            Instruction::Ret(Some(op)) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }
            Instruction::Ret(None) => {}

            // 二元运算（提取dst和操作数）
            Instruction::Add { dst, lhs, rhs }
            | Instruction::Sub { dst, lhs, rhs }
            | Instruction::Mul { dst, lhs, rhs }
            | Instruction::Div { dst, lhs, rhs }
            | Instruction::Mod { dst, lhs, rhs }
            | Instruction::And { dst, lhs, rhs }
            | Instruction::Or { dst, lhs, rhs }
            | Instruction::Xor { dst, lhs, rhs }
            | Instruction::Shl { dst, lhs, rhs }
            | Instruction::Shr { dst, lhs, rhs } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(lhs, params, locals, deps);
                self.extract_type_from_operand(rhs, params, locals, deps);
            }

            // 一元运算
            Instruction::Neg { dst, src } => {
                self.extract_type_from_operand(dst, params, locals, deps);
                self.extract_type_from_operand(src, params, locals, deps);
            }

            // Jumps and branches - 无需提取类型
            Instruction::Jmp(_) => {}
            Instruction::JmpIf(op, _) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }
            Instruction::JmpIfNot(op, _) => {
                self.extract_type_from_operand(op, params, locals, deps);
            }

            // 其他指令（保守处理）
            _ => {}
        }
    }

    /// 从指令的 dst 字段提取类型（通用方法）
    fn extract_type_from_inst_dst(
        &self,
        inst: &Instruction,
        params: &[MonoType],
        locals: &[MonoType],
        deps: &mut Vec<MonoType>,
    ) {
        // 使用模式匹配尝试提取 dst
        match inst {
            Instruction::LoadField { dst, .. }
            | Instruction::StoreField { dst, .. }
            | Instruction::LoadIndex { dst, .. }
            | Instruction::StoreIndex { dst, .. }
            | Instruction::Call { dst: Some(dst), .. }
            | Instruction::CallDyn { dst: Some(dst), .. }
            | Instruction::CallVirt { dst: Some(dst), .. }
            | Instruction::Alloc { dst, .. }
            | Instruction::AllocArray { dst, .. }
            | Instruction::HeapAlloc { dst, .. }
            | Instruction::MakeClosure { dst, .. }
            | Instruction::ArcNew { dst, .. }
            | Instruction::ArcClone { dst, .. }
            | Instruction::Move { dst, .. }
            | Instruction::Load { dst, .. }
            | Instruction::Cast { dst, .. }
            | Instruction::Neg { dst, .. } => {
                self.extract_type_from_operand(dst, params, locals, deps);
            }
            _ => {}
        }
    }

    /// 从操作数提取类型
    ///
    /// # Arguments
    /// * `op` - 操作数
    /// * `params` - 函数参数类型
    /// * `locals` - 局部变量类型
    /// * `deps` - 收集依赖类型
    fn extract_type_from_operand(
        &self,
        op: &Operand,
        params: &[MonoType],
        locals: &[MonoType],
        deps: &mut Vec<MonoType>,
    ) {
        match op {
            // 参数：使用索引从 params 获取类型
            Operand::Arg(idx) => {
                if *idx < params.len() {
                    deps.push(params[*idx].clone());
                }
            }
            // 局部变量：使用索引从 locals 获取类型
            Operand::Local(idx) => {
                if *idx < locals.len() {
                    deps.push(locals[*idx].clone());
                }
            }
            // 临时变量：跳过（临时变量类型由 SSA 形式确定）
            Operand::Temp(_) => {}
            // 全局变量：跳过（需要全局表信息）
            Operand::Global(_) => {}
            // 常量：基本类型直接添加
            Operand::Const(c) => {
                self.extract_type_from_const(c, deps);
            }
            // Label：无类型
            Operand::Label(_) => {}
            // Register：无类型
            Operand::Register(_) => {}
        }
    }

    /// 从常量值提取类型
    fn extract_type_from_const(
        &self,
        c: &ConstValue,
        deps: &mut Vec<MonoType>,
    ) {
        match c {
            ConstValue::Void => {}
            ConstValue::Bool(_) => deps.push(MonoType::Bool),
            ConstValue::Int(n) => {
                // 根据值大小推断整型宽度
                let width = if *n >= i128::from(i32::MIN) && *n <= i128::from(i32::MAX) {
                    32
                } else {
                    64
                };
                deps.push(MonoType::Int(width));
            }
            ConstValue::Float(_) => deps.push(MonoType::Float(64)),
            ConstValue::Char(_) => deps.push(MonoType::Char),
            ConstValue::String(_) => deps.push(MonoType::String),
            ConstValue::Bytes(_) => deps.push(MonoType::Bytes),
        }
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
    pub fn dependencies(
        &self,
        node: &InstanceNode,
    ) -> Option<&HashSet<InstanceNode>> {
        self.edges.get(node)
    }

    /// 获取使用该节点的节点（被哪些节点使用）
    pub fn dependents(
        &self,
        node: &InstanceNode,
    ) -> Option<&HashSet<InstanceNode>> {
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
    pub fn with_external_graph(
        mut self,
        graph: &'a InstantiationGraph,
    ) -> Self {
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
    pub fn register_generic_type(
        &mut self,
        generic_id: GenericTypeId,
        ty: MonoType,
    ) {
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
        for func_id in instantiated_functions.keys() {
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
                .filter_map(Self::type_to_instance_node)
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
    fn extract_type_args_from_id(id: &FunctionId) -> Vec<MonoType> {
        id.type_args().to_vec()
    }

    /// 从 MonoType 提取类型参数
    ///
    /// 递归提取泛型类型的类型参数
    fn extract_type_args_from_type(ty: &MonoType) -> Vec<MonoType> {
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

            // 联合和交集类型：提取所有成员的参数
            MonoType::Union(types) | MonoType::Intersection(types) => types
                .iter()
                .flat_map(Self::extract_type_args_from_type)
                .collect(),

            // 结构体：提取字段类型参数
            MonoType::Struct(s) => s
                .fields
                .iter()
                .map(|(_, ty)| ty)
                .flat_map(Self::extract_type_args_from_type)
                .collect(),

            // 枚举：没有类型参数（枚举变体不使用泛型）
            MonoType::Enum(_) => vec![],

            // 容器类型：递归提取元素类型
            MonoType::Tuple(types) => types
                .iter()
                .flat_map(Self::extract_type_args_from_type)
                .collect(),
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                Self::extract_type_args_from_type(elem)
            }
            MonoType::Dict(key, value) => {
                let mut args = Self::extract_type_args_from_type(key);
                args.extend(Self::extract_type_args_from_type(value));
                args
            }

            // 函数类型：提取参数和返回类型的参数
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                let mut args = params
                    .iter()
                    .flat_map(Self::extract_type_args_from_type)
                    .collect::<Vec<_>>();
                args.extend(Self::extract_type_args_from_type(return_type));
                args
            }

            // Arc 包装：提取内部类型参数
            MonoType::Arc(inner) => Self::extract_type_args_from_type(inner),

            // Weak 包装：提取内部类型参数
            MonoType::Weak(inner) => Self::extract_type_args_from_type(inner),

            // 关联类型：提取主机类型和关联类型的参数
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                let mut args = Self::extract_type_args_from_type(host_type);
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
    ///
    /// 尝试将 MonoType 转换为 InstanceNode，用于构建依赖图
    /// 注意：当前 StructType 和 EnumType 没有类型参数字段，
    /// 只处理已知的内置泛型类型
    fn type_to_instance_node(ty: &MonoType) -> Option<InstanceNode> {
        match ty {
            // 泛型列表
            MonoType::List(elem) => {
                let node = TypeInstanceNode::new(
                    GenericTypeId::new("List".to_string(), vec!["T".to_string()]),
                    vec![*elem.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型字典
            MonoType::Dict(key, value) => {
                let node = TypeInstanceNode::new(
                    GenericTypeId::new("Dict".to_string(), vec!["K".to_string(), "V".to_string()]),
                    vec![*key.clone(), *value.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Set
            MonoType::Set(elem) => {
                let node = TypeInstanceNode::new(
                    GenericTypeId::new("Set".to_string(), vec!["T".to_string()]),
                    vec![*elem.clone()],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Option (通过名称判断)
            MonoType::Enum(e) if e.name == "Option" => {
                let node = TypeInstanceNode::new(
                    GenericTypeId::new("Option".to_string(), vec!["T".to_string()]),
                    vec![],
                );
                Some(InstanceNode::Type(node))
            }

            // 泛型 Result (通过名称判断)
            MonoType::Enum(e) if e.name == "Result" => {
                let node = TypeInstanceNode::new(
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
                let node =
                    TypeInstanceNode::new(GenericTypeId::new(s.name.clone(), vec![]), vec![]);
                Some(InstanceNode::Type(node))
            }

            // 其他枚举
            MonoType::Enum(e) => {
                let node =
                    TypeInstanceNode::new(GenericTypeId::new(e.name.clone(), vec![]), vec![]);
                Some(InstanceNode::Type(node))
            }

            _ => None,
        }
    }
}
