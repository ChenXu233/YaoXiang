//! Trait 继承处理
//!
//! 实现 RFC-011 Trait 继承验证：
//! - 验证父 Trait 已定义
//! - 检测循环继承
//! - 收集所有必需方法（包括从父 Trait 继承的）

use std::collections::{HashMap, HashSet};

/// Trait 继承边
#[derive(Debug, Clone)]
pub struct InheritanceEdge {
    pub child: String,
    pub parent: String,
}

/// Trait 继承图
#[derive(Debug, Default)]
pub struct TraitInheritanceGraph {
    /// 继承边: child -> parents
    edges: HashMap<String, Vec<String>>,
    /// 所有 Trait 节点
    nodes: HashSet<String>,
}

impl TraitInheritanceGraph {
    /// 创建新的继承图
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加继承边
    pub fn add_edge(
        &mut self,
        child: &str,
        parent: &str,
    ) {
        self.edges
            .entry(child.to_string())
            .or_default()
            .push(parent.to_string());
        self.nodes.insert(child.to_string());
        self.nodes.insert(parent.to_string());
    }

    /// 获取直接父 Trait
    pub fn parents(
        &self,
        trait_name: &str,
    ) -> Option<&Vec<String>> {
        self.edges.get(trait_name)
    }

    /// 获取所有祖先（包括间接继承）
    pub fn all_ancestors(
        &self,
        trait_name: &str,
    ) -> Vec<String> {
        let mut ancestors = Vec::new();
        self.collect_ancestors(trait_name, &mut ancestors, &mut HashSet::new());
        ancestors
    }

    fn collect_ancestors(
        &self,
        trait_name: &str,
        ancestors: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        if !visited.insert(trait_name.to_string()) {
            return;
        }

        if let Some(parents) = self.parents(trait_name) {
            for parent in parents {
                if !ancestors.contains(parent) {
                    ancestors.push(parent.clone());
                    self.collect_ancestors(parent, ancestors, visited);
                }
            }
        }
    }

    /// 检查 Trait 是否在继承链中
    pub fn is_in_ancestors(
        &self,
        trait_name: &str,
        ancestor: &str,
    ) -> bool {
        self.all_ancestors(trait_name)
            .contains(&ancestor.to_string())
    }

    /// 检查循环继承，返回循环路径（如果有）
    pub fn find_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for node in &self.nodes {
            if let Some(cycle) = self.detect_cycle(node, &mut visited, &mut stack) {
                return Some(cycle);
            }
        }
        None
    }

    fn detect_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        if stack.contains(node) {
            // 找到循环，返回从 node 开始的路径
            let cycle = vec![node.to_string()];
            return Some(cycle);
        }

        if visited.contains(node) {
            return None;
        }

        visited.insert(node.to_string());
        stack.insert(node.to_string());

        if let Some(parents) = self.parents(node) {
            for parent in parents {
                if self.nodes.contains(parent) {
                    if let Some(mut cycle) = self.detect_cycle(parent, visited, stack) {
                        // 将当前节点添加到循环路径的前面
                        cycle.insert(0, node.to_string());
                        // 检查循环是否闭合
                        if cycle.first() == cycle.last() {
                            return Some(cycle);
                        }
                        return Some(cycle);
                    }
                }
            }
        }

        stack.remove(node);
        None
    }

    /// 获取所有节点
    pub fn nodes(&self) -> &HashSet<String> {
        &self.nodes
    }
}

/// 继承检查器
#[derive(Debug)]
pub struct InheritanceChecker<'a> {
    graph: TraitInheritanceGraph,
    /// 已知 Trait 定义（用于验证父 Trait 是否存在）
    known_traits: HashSet<String>,
    /// Trait 定义引用
    trait_definitions: HashMap<String, &'a super::TraitDefinition>,
}

impl<'a> Default for InheritanceChecker<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> InheritanceChecker<'a> {
    /// 创建新的继承检查器
    pub fn new() -> Self {
        Self {
            graph: TraitInheritanceGraph::new(),
            known_traits: HashSet::new(),
            trait_definitions: HashMap::new(),
        }
    }

    /// 注册已知的 Trait 定义
    pub fn register_trait(
        &mut self,
        name: &str,
    ) {
        self.known_traits.insert(name.to_string());
    }

    /// 添加 Trait 定义及其父 Trait
    pub fn add_trait(
        &mut self,
        name: &str,
        parents: &[crate::frontend::core::parser::ast::Type],
    ) {
        self.graph.nodes.insert(name.to_string());

        for parent in parents {
            if let crate::frontend::core::parser::ast::Type::Name(n) = parent {
                self.graph.add_edge(name, n);
            }
        }
    }

    /// 验证所有父 Trait 是否已定义
    pub fn validate_parent_traits(&self) -> Result<(), InheritanceError> {
        let undefined: Vec<String> = self
            .graph
            .nodes()
            .iter()
            .filter_map(|node| {
                self.graph.parents(node).map(|parents| {
                    parents
                        .iter()
                        .filter(|p| !self.known_traits.contains(*p))
                        .cloned()
                        .collect::<Vec<_>>()
                })
            })
            .flatten()
            .collect();

        if undefined.is_empty() {
            Ok(())
        } else {
            Err(InheritanceError::ParentNotFound(undefined))
        }
    }

    /// 检查循环继承
    pub fn check_cycles(&self) -> Result<(), InheritanceError> {
        if let Some(cycle) = self.graph.find_cycle() {
            Err(InheritanceError::CyclicInheritance(cycle))
        } else {
            Ok(())
        }
    }

    /// 获取所有必需方法（包括从父 Trait 继承的）
    pub fn get_all_required_methods(
        &self,
        trait_name: &str,
    ) -> Vec<String> {
        let mut methods = Vec::new();
        self.collect_methods(trait_name, &mut methods, &mut HashSet::new());
        methods
    }

    fn collect_methods(
        &self,
        trait_name: &str,
        methods: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        if !visited.insert(trait_name.to_string()) {
            return;
        }

        // 首先收集父 Trait 的方法
        if let Some(parents) = self.graph.parents(trait_name) {
            for parent in parents {
                self.collect_methods(parent, methods, visited);
            }
        }

        // 添加当前 Trait 的方法
        if let Some(def) = self.trait_definitions.get(trait_name) {
            for method_name in def.methods.keys() {
                if !methods.contains(method_name) {
                    methods.push(method_name.clone());
                }
            }
        }
    }

    /// 完全验证
    pub fn validate(&self) -> Result<(), InheritanceError> {
        self.validate_parent_traits()?;
        self.check_cycles()?;
        Ok(())
    }
}

/// 继承错误
#[derive(Debug, Clone)]
pub enum InheritanceError {
    /// 循环继承
    CyclicInheritance(Vec<String>),
    /// 父 Trait 未定义
    ParentNotFound(Vec<String>),
}

impl std::fmt::Display for InheritanceError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::CyclicInheritance(cycle) => {
                write!(f, "Cyclic inheritance detected: {}", cycle.join(" -> "))
            }
            Self::ParentNotFound(parents) => {
                write!(f, "Undefined parent trait(s): {}", parents.join(", "))
            }
        }
    }
}

impl std::error::Error for InheritanceError {}
