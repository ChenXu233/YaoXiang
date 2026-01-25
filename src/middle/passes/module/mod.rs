//! 模块系统基础类型
//!
//! 管理模块ID、模块依赖图和模块节点

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// 模块ID - 唯一标识一个编译模块
///
/// 使用 newtype 模式，内部是 usize，支持从 0 开始的索引
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModuleId(pub usize);

impl ModuleId {
    /// 创建新的模块ID
    pub fn new(id: usize) -> Self {
        ModuleId(id)
    }

    /// 获取内部索引
    pub fn index(&self) -> usize {
        self.0
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "module#{}", self.0)
    }
}

/// 模块依赖边
///
/// 表示模块间的依赖关系，包含可见性信息
#[derive(Debug, Clone)]
pub struct ModuleEdge {
    /// 源模块（依赖方）
    pub from: ModuleId,
    /// 目标模块（被依赖方）
    pub to: ModuleId,
    /// 是否公开导入
    ///
    /// 公开导入意味着依赖会传递
    /// 私有导入仅限当前模块使用
    pub is_public: bool,
}

impl ModuleEdge {
    /// 创建新的模块边
    pub fn new(
        from: ModuleId,
        to: ModuleId,
        is_public: bool,
    ) -> Self {
        ModuleEdge {
            from,
            to,
            is_public,
        }
    }
}

/// 模块节点状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleStatus {
    /// 刚创建，等待解析
    Created,
    /// 正在解析
    Parsing,
    /// 解析完成
    Parsed,
    /// 正在类型检查
    TypeChecking,
    /// 类型检查完成
    TypeChecked,
    /// 正在单态化
    Monomorphizing,
    /// 单态化完成
    Monomorphized,
    /// 编译失败
    Failed,
}

/// 模块节点
///
/// 包含模块的所有信息：源路径、IR、状态等
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// 模块ID
    pub id: ModuleId,
    /// 源文件路径
    pub source_path: PathBuf,
    /// 模块名称（从路径推导）
    pub name: String,
    /// 解析后的IR（延迟填充）
    pub ir: Option<crate::middle::core::ir::ModuleIR>,
    /// 模块状态
    pub status: ModuleStatus,
    /// 直接依赖的模块ID列表
    pub dependencies: Vec<ModuleId>,
    /// 导入该模块的模块ID列表
    pub dependents: Vec<ModuleId>,
    /// 错误信息（如果有）
    pub errors: Vec<String>,
}

impl ModuleNode {
    /// 创建新的模块节点
    pub fn new(
        id: ModuleId,
        source_path: PathBuf,
    ) -> Self {
        let name = Self::derive_module_name(&source_path);
        ModuleNode {
            id,
            source_path,
            name,
            ir: None,
            status: ModuleStatus::Created,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// 从路径推导模块名称
    fn derive_module_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "anonymous".to_string())
    }

    /// 标记解析完成
    pub fn mark_parsed(&mut self) {
        self.status = ModuleStatus::Parsed;
    }

    /// 标记类型检查完成
    pub fn mark_typechecked(&mut self) {
        self.status = ModuleStatus::TypeChecked;
    }

    /// 标记单态化完成
    pub fn mark_monomorphized(&mut self) {
        self.status = ModuleStatus::Monomorphized;
    }

    /// 添加依赖
    pub fn add_dependency(
        &mut self,
        module_id: ModuleId,
    ) {
        if !self.dependencies.contains(&module_id) {
            self.dependencies.push(module_id);
        }
    }

    /// 添加依赖者（被依赖）
    pub fn add_dependent(
        &mut self,
        module_id: ModuleId,
    ) {
        if !self.dependents.contains(&module_id) {
            self.dependents.push(module_id);
        }
    }

    /// 检查模块是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 添加错误信息
    pub fn add_error(
        &mut self,
        error: String,
    ) {
        self.errors.push(error);
        self.status = ModuleStatus::Failed;
    }
}

/// 模块图错误
#[derive(Debug, Error, PartialEq)]
pub enum ModuleGraphError {
    #[error("模块不存在: {0:?}")]
    ModuleNotFound(ModuleId),

    #[error("循环依赖检测到")]
    CycleDetected,

    #[error("模块路径重复: {0:?}")]
    DuplicatePath(PathBuf),

    #[error("无效的模块依赖: 从 {from:?} 到 {to:?}")]
    InvalidDependency { from: ModuleId, to: ModuleId },

    #[error("拓扑排序失败: 存在循环依赖")]
    TopologySortFailed,
}

/// 模块依赖图
///
/// 管理所有模块及其依赖关系，支持拓扑排序
#[derive(Debug, Default)]
pub struct ModuleGraph {
    /// 模块节点映射
    nodes: HashMap<ModuleId, ModuleNode>,
    /// 模块边列表
    edges: Vec<ModuleEdge>,
    /// 路径到模块ID的映射（用于快速查找）
    path_to_id: HashMap<PathBuf, ModuleId>,
    /// 下一个可用的模块ID
    next_module_id: usize,
}

impl ModuleGraph {
    /// 创建新的模块图
    pub fn new() -> Self {
        ModuleGraph {
            nodes: HashMap::new(),
            edges: Vec::new(),
            path_to_id: HashMap::new(),
            next_module_id: 0,
        }
    }

    /// 添加模块
    ///
    /// 返回新创建的模块ID
    pub fn add_module(
        &mut self,
        source_path: PathBuf,
    ) -> ModuleId {
        // 检查路径是否重复
        if let Some(existing_id) = self.path_to_id.get(&source_path) {
            return *existing_id;
        }

        let id = ModuleId(self.next_module_id);
        self.next_module_id += 1;

        let node = ModuleNode::new(id, source_path.clone());
        self.nodes.insert(id, node);
        self.path_to_id.insert(source_path, id);

        id
    }

    /// 获取模块
    pub fn get_module(
        &self,
        id: ModuleId,
    ) -> Option<&ModuleNode> {
        self.nodes.get(&id)
    }

    /// 获取可修改的模块
    pub fn get_module_mut(
        &mut self,
        id: ModuleId,
    ) -> Option<&mut ModuleNode> {
        self.nodes.get_mut(&id)
    }

    /// 添加依赖关系
    ///
    /// from 依赖 to，表示 from -> to
    pub fn add_dependency(
        &mut self,
        from: ModuleId,
        to: ModuleId,
        is_public: bool,
    ) -> Result<(), ModuleGraphError> {
        // 检查模块是否存在
        if !self.nodes.contains_key(&from) {
            return Err(ModuleGraphError::ModuleNotFound(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(ModuleGraphError::ModuleNotFound(to));
        }

        // 不能依赖自己
        if from == to {
            return Err(ModuleGraphError::InvalidDependency { from, to });
        }

        // 添加边
        let edge = ModuleEdge::new(from, to, is_public);
        self.edges.push(edge);

        // 更新节点的依赖关系
        if let Some(node) = self.nodes.get_mut(&from) {
            node.add_dependency(to);
        }
        if let Some(node) = self.nodes.get_mut(&to) {
            node.add_dependent(from);
        }

        Ok(())
    }

    /// 获取模块的直接依赖
    pub fn get_dependencies(
        &self,
        module: ModuleId,
    ) -> Result<Vec<ModuleId>, ModuleGraphError> {
        self.nodes
            .get(&module)
            .map(|node| node.dependencies.clone())
            .ok_or(ModuleGraphError::ModuleNotFound(module))
    }

    /// 获取依赖该模块的所有模块
    pub fn get_dependents(
        &self,
        module: ModuleId,
    ) -> Result<Vec<ModuleId>, ModuleGraphError> {
        self.nodes
            .get(&module)
            .map(|node| node.dependents.clone())
            .ok_or(ModuleGraphError::ModuleNotFound(module))
    }

    /// 获取模块的名称
    pub fn get_module_name(
        &self,
        id: ModuleId,
    ) -> Option<&str> {
        self.nodes.get(&id).map(|n| n.name.as_str())
    }

    /// 获取模块的源路径
    pub fn get_source_path(
        &self,
        id: ModuleId,
    ) -> Option<&PathBuf> {
        self.nodes.get(&id).map(|n| &n.source_path)
    }

    /// 获取模块的数量
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 获取所有模块ID
    pub fn all_modules(&self) -> Vec<ModuleId> {
        self.nodes.keys().cloned().collect()
    }

    /// 检查模块是否存在
    pub fn contains(
        &self,
        id: ModuleId,
    ) -> bool {
        self.nodes.contains_key(&id)
    }

    /// 拓扑排序
    ///
    /// 返回模块ID的有序列表，确保依赖出现在被依赖之前
    pub fn topological_sort(&self) -> Result<Vec<ModuleId>, ModuleGraphError> {
        // 计算入度
        let mut in_degree: HashMap<ModuleId, usize> =
            self.nodes.keys().map(|&id| (id, 0)).collect();

        for edge in &self.edges {
            // edge.from 依赖 edge.to，所以 edge.to 的入度加 1
            *in_degree.get_mut(&edge.to).unwrap() += 1;
        }

        // 使用队列进行拓扑排序（Kahn's algorithm）
        let mut queue: VecDeque<ModuleId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result: Vec<ModuleId> = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);

            // 减少相邻节点的入度
            // edge.from 依赖 edge.to，所以当前节点处理完后，edge.to 的入度减 1
            for edge in &self.edges {
                if edge.from == node_id {
                    let to_id = edge.to;
                    if let Some(deg) = in_degree.get_mut(&to_id) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(to_id);
                        }
                    }
                }
            }
        }

        // 检查是否有循环依赖
        if result.len() != self.nodes.len() {
            return Err(ModuleGraphError::TopologySortFailed);
        }

        // 反转结果，使得依赖项在前，被依赖项在后
        // 例如：如果 A 依赖 B，则 B 应该先编译，所以 B 在前
        result.reverse();

        Ok(result)
    }

    /// 检测循环依赖
    ///
    /// 返回检测到的循环路径（如果有）
    pub fn detect_cycles(&self) -> Option<Vec<ModuleId>> {
        // 使用 DFS 检测循环
        let mut visited: HashSet<ModuleId> = HashSet::new();
        let mut recursion_stack: HashSet<ModuleId> = HashSet::new();

        for &node_id in self.nodes.keys() {
            if !visited.contains(&node_id) {
                if let Some(cycle) =
                    self.detect_cycles_dfs(node_id, &mut visited, &mut recursion_stack)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn detect_cycles_dfs(
        &self,
        node: ModuleId,
        visited: &mut HashSet<ModuleId>,
        recursion_stack: &mut HashSet<ModuleId>,
    ) -> Option<Vec<ModuleId>> {
        visited.insert(node);
        recursion_stack.insert(node);

        // 检查所有从当前节点出发的边
        for edge in &self.edges {
            if edge.from == node {
                let neighbor = edge.to;

                if !visited.contains(&neighbor) {
                    if let Some(path) = self.detect_cycles_dfs(neighbor, visited, recursion_stack) {
                        let mut full_path = vec![node];
                        full_path.extend(path);
                        return Some(full_path);
                    }
                } else if recursion_stack.contains(&neighbor) {
                    // 找到循环，直接返回两个节点
                    return Some(vec![node, neighbor]);
                }
            }
        }

        recursion_stack.remove(&node);
        None
    }

    /// 获取模块的直接依赖（公开依赖）
    ///
    /// 只返回 is_public = true 的依赖
    pub fn get_public_dependencies(
        &self,
        module: ModuleId,
    ) -> Result<Vec<ModuleId>, ModuleGraphError> {
        self.nodes
            .get(&module)
            .map(|_node| {
                self.edges
                    .iter()
                    .filter(|e| e.from == module && e.is_public)
                    .map(|e| e.to)
                    .collect()
            })
            .ok_or(ModuleGraphError::ModuleNotFound(module))
    }

    /// 导出该模块的公开依赖（传递性闭包）
    ///
    /// 返回所有公开依赖的传递闭包
    pub fn get_public_dependency_closure(
        &self,
        module: ModuleId,
    ) -> Result<HashSet<ModuleId>, ModuleGraphError> {
        let mut closure: HashSet<ModuleId> = HashSet::new();
        let mut to_visit: VecDeque<ModuleId> = VecDeque::new();

        // 初始公开依赖
        for dep in self.get_public_dependencies(module)? {
            if !closure.contains(&dep) {
                closure.insert(dep);
                to_visit.push_back(dep);
            }
        }

        // BFS 遍历
        while let Some(current) = to_visit.pop_front() {
            for dep in self.get_public_dependencies(current)? {
                if !closure.contains(&dep) {
                    closure.insert(dep);
                    to_visit.push_back(dep);
                }
            }
        }

        Ok(closure)
    }
}
