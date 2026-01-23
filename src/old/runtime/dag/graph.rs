//! Computation DAG for tracking lazy computations
//!
//! A directed acyclic graph (DAG) that represents the computation
//! dependencies and enables parallel execution.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use super::node::{DAGNode, DAGNodeKind};
use super::node_id::{NodeId, NodeIdGenerator};

/// Errors that can occur when manipulating the computation DAG.
#[derive(Debug, PartialEq)]
pub enum DAGError {
    /// Tried to access a node that doesn't exist
    NodeNotFound(NodeId),

    /// Tried to add an edge that would create a cycle
    CycleDetected,

    /// Tried to add a duplicate edge
    DuplicateEdge,

    /// Tried to modify a frozen graph
    GraphFrozen,
}

/// A directed acyclic graph representing lazy computations.
///
/// The `ComputationDAG` tracks all computation nodes and their
/// dependencies, enabling the scheduler to determine which
/// computations can run in parallel.
#[derive(Debug)]
pub struct ComputationDAG {
    /// All nodes in the graph
    nodes: HashMap<NodeId, DAGNode>,

    /// Generator for new node IDs
    id_generator: NodeIdGenerator,

    /// Whether the graph is frozen (no more modifications)
    frozen: bool,

    /// Cached set of root nodes (nodes with no dependencies)
    roots: HashSet<NodeId>,

    /// Cached set of leaf nodes (nodes with no dependents)
    leaves: HashSet<NodeId>,

    /// Topological sort order (cached)
    topo_order: Vec<NodeId>,
}

impl ComputationDAG {
    /// Create a new empty computation DAG.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::ComputationDAG;
    ///
    /// let dag = ComputationDAG::new();
    /// assert!(dag.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            id_generator: NodeIdGenerator::new(),
            frozen: false,
            roots: HashSet::new(),
            leaves: HashSet::new(),
            topo_order: Vec::new(),
        }
    }

    /// Create a new computation DAG with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: HashMap::with_capacity(capacity),
            id_generator: NodeIdGenerator::new(),
            frozen: false,
            roots: HashSet::new(),
            leaves: HashSet::new(),
            topo_order: Vec::new(),
        }
    }

    /// Add a new node to the graph.
    ///
    /// Returns the ID assigned to the new node.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::{ComputationDAG, DAGNodeKind};
    ///
    /// let mut dag = ComputationDAG::new();
    /// let id = dag.add_node(DAGNodeKind::Constant { value: "42".to_string() }).unwrap();
    /// assert!(dag.contains_node(id));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DAGError::GraphFrozen` if the graph has been frozen.
    #[inline]
    pub fn add_node(
        &mut self,
        kind: DAGNodeKind,
    ) -> Result<NodeId, DAGError> {
        if self.frozen {
            return Err(DAGError::GraphFrozen);
        }

        let id = self.id_generator.generate();
        let node = DAGNode::new(id, kind);
        self.nodes.insert(id, node);
        self.roots.insert(id);
        self.leaves.insert(id);
        self.topo_order.clear(); // Invalidate cache
        Ok(id)
    }

    /// Add an edge from `from` to `to` (`to` depends on `from`).
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::{ComputationDAG, DAGNodeKind};
    ///
    /// let mut dag = ComputationDAG::new();
    /// let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
    /// let b = dag.add_node(DAGNodeKind::Compute { name: "add".to_string() }).unwrap();
    /// dag.add_edge(a, b).unwrap();
    /// assert!(dag.has_edge(a, b));
    /// ```
    ///
    /// # Errors
    ///
    /// - `DAGError::NodeNotFound` if either node doesn't exist
    /// - `DAGError::CycleDetected` if adding this edge would create a cycle
    /// - `DAGError::DuplicateEdge` if this edge already exists
    /// - `DAGError::GraphFrozen` if the graph has been frozen
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
    ) -> Result<(), DAGError> {
        if self.frozen {
            return Err(DAGError::GraphFrozen);
        }

        // Check if nodes exist
        let from_node = self.nodes.get(&from).ok_or(DAGError::NodeNotFound(from))?;
        let to_node = self.nodes.get(&to).ok_or(DAGError::NodeNotFound(to))?;

        // Check if edge already exists
        // Edge from -> to means to depends on from
        if to_node.depends_on(from) || from_node.has_dependent(to) {
            return Err(DAGError::DuplicateEdge);
        }

        // Check for cycle (simplified: if to already depends on from, it would be a cycle)
        if self.would_create_cycle(from, to) {
            return Err(DAGError::CycleDetected);
        }

        // Add the edge
        self.nodes.get_mut(&from).unwrap().add_dependent(to);
        self.nodes.get_mut(&to).unwrap().add_dependency(from);

        // Update caches
        self.roots.remove(&to); // to is no longer a root
        self.leaves.remove(&from); // from is no longer a leaf
        self.topo_order.clear(); // Invalidate cache

        Ok(())
    }

    /// Check if adding an edge from `from` to `to` would create a cycle.
    fn would_create_cycle(
        &self,
        from: NodeId,
        to: NodeId,
    ) -> bool {
        // Adding from -> to means to will depend on from.
        // If there's already a path from to to from (to -> ... -> from),
        // then adding from -> to would create a cycle: from -> to -> ... -> from
        // So we check: can we reach 'from' starting from 'to'?
        self.is_reachable(to, from)
    }

    /// Check if there's a path from `source` to `target`.
    /// Returns true if we can reach target starting from source.
    fn is_reachable(
        &self,
        source: NodeId,
        target: NodeId,
    ) -> bool {
        if source == target {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source);
        visited.insert(source);

        while let Some(current) = queue.pop_front() {
            if current == target {
                return true;
            }

            // Follow dependents (edges pointing TO dependents)
            // If current is depended on by someone, we can reach them
            if let Some(node) = self.nodes.get(&current) {
                for &dependent in node.dependents() {
                    if visited.insert(dependent) {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        false
    }

    /// Check if the graph contains a node.
    #[inline]
    pub fn contains_node(
        &self,
        id: NodeId,
    ) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Check if there's an edge from `from` to `to`.
    #[inline]
    pub fn has_edge(
        &self,
        from: NodeId,
        to: NodeId,
    ) -> bool {
        self.nodes
            .get(&from)
            .map(|node| node.has_dependent(to))
            .unwrap_or(false)
    }

    /// Get a reference to a node.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::{ComputationDAG, DAGNodeKind};
    ///
    /// let mut dag = ComputationDAG::new();
    /// let id = dag.add_node(DAGNodeKind::Constant { value: "42".to_string() }).unwrap();
    /// let node = dag.get_node(id).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `DAGError::NodeNotFound` if the node doesn't exist.
    #[inline]
    pub fn get_node(
        &self,
        id: NodeId,
    ) -> Result<&DAGNode, DAGError> {
        self.nodes.get(&id).ok_or(DAGError::NodeNotFound(id))
    }

    /// Get a mutable reference to a node.
    ///
    /// # Errors
    ///
    /// Returns `DAGError::NodeNotFound` if the node doesn't exist.
    #[inline]
    pub fn get_node_mut(
        &mut self,
        id: NodeId,
    ) -> Result<&mut DAGNode, DAGError> {
        self.nodes.get_mut(&id).ok_or(DAGError::NodeNotFound(id))
    }

    /// Get all nodes.
    #[inline]
    pub fn nodes(&self) -> &HashMap<NodeId, DAGNode> {
        &self.nodes
    }

    /// Get the number of nodes in the graph.
    #[inline]
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the graph is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get all root nodes (nodes with no dependencies).
    ///
    /// These are the starting points for evaluation.
    #[inline]
    pub fn roots(&self) -> &HashSet<NodeId> {
        &self.roots
    }

    /// Get all leaf nodes (nodes with no dependents).
    ///
    /// These are the final results of the computation.
    #[inline]
    pub fn leaves(&self) -> &HashSet<NodeId> {
        &self.leaves
    }

    /// Get nodes that are ready to execute (all dependencies satisfied).
    ///
    /// These are nodes whose dependencies have already been evaluated.
    #[inline]
    pub fn ready_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|node| {
                node.dependencies()
                    .iter()
                    .all(|dep| self.nodes.get(dep).map(|n| n.is_leaf()).unwrap_or(true))
            })
            .map(|node| node.id())
            .collect()
    }

    /// Get the topological sort order of nodes.
    ///
    /// Returns nodes in an order where all dependencies come before
    /// the nodes that depend on them.
    pub fn topological_sort(&mut self) -> &[NodeId] {
        if self.topo_order.is_empty() {
            self.compute_topological_sort();
        }
        &self.topo_order
    }

    /// Compute the topological sort using Kahn's algorithm.
    fn compute_topological_sort(&mut self) {
        if self.nodes.is_empty() {
            self.topo_order.clear();
            return;
        }

        // Clone to avoid borrowing issues
        let nodes: HashMap<NodeId, DAGNode> = self.nodes.clone();
        let mut in_degree: HashMap<NodeId, usize> = nodes
            .iter()
            .map(|(id, node)| (*id, node.num_dependencies()))
            .collect();

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = queue.pop_front() {
            result.push(node_id);

            // Add dependents with reduced in-degree
            if let Some(node) = nodes.get(&node_id) {
                for &dependent in node.dependents() {
                    if let Some(degree) = in_degree.get_mut(&dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent);
                        }
                    }
                }
            }
        }

        // Check for cycle (not all nodes processed)
        if result.len() != self.nodes.len() {
            // This shouldn't happen if add_edge checks for cycles
            // But we handle it gracefully
        }

        self.topo_order = result;
    }

    /// Freeze the graph to prevent further modifications.
    ///
    /// This is called when the graph is ready to be executed.
    #[inline]
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Check if the graph is frozen.
    #[inline]
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Get the maximum parallelism level for the graph.
    ///
    /// This is the maximum number of nodes that can be executed
    /// concurrently based on the dependency structure.
    pub fn max_parallelism(&self) -> usize {
        let mut max_width = 0;
        let mut current_level: Vec<NodeId> = self.roots.iter().copied().collect();

        while !current_level.is_empty() {
            max_width = max_width.max(current_level.len());

            // Get all dependents of current level nodes
            let mut next_level = Vec::new();
            for &node_id in &current_level {
                if let Some(node) = self.nodes.get(&node_id) {
                    for &dependent in node.dependents() {
                        // Only add if all dependencies are in current or previous levels
                        if let Some(dep_node) = self.nodes.get(&dependent) {
                            if dep_node
                                .dependencies()
                                .iter()
                                .all(|d| current_level.contains(d) || next_level.contains(d))
                                && !next_level.contains(&dependent)
                            {
                                next_level.push(dependent);
                            }
                        }
                    }
                }
            }

            current_level = next_level;
        }

        max_width
    }

    /// Get the critical path length (longest dependency chain).
    ///
    /// This represents the minimum number of sequential steps
    /// required to compute all results.
    pub fn critical_path_length(&self) -> usize {
        let mut memo: HashMap<NodeId, usize> = HashMap::new();

        fn compute_length(
            node_id: NodeId,
            dag: &ComputationDAG,
            memo: &mut HashMap<NodeId, usize>,
        ) -> usize {
            if let Some(&len) = memo.get(&node_id) {
                return len;
            }

            let node = dag.get_node(node_id).unwrap();
            if node.dependencies().is_empty() {
                memo.insert(node_id, 1);
                return 1;
            }

            let max_dep_len = node
                .dependencies()
                .iter()
                .map(|&dep| compute_length(dep, dag, memo))
                .max()
                .unwrap_or(0);

            let result = max_dep_len + 1;
            memo.insert(node_id, result);
            result
        }

        self.leaves
            .iter()
            .map(|&leaf| compute_length(leaf, self, &mut memo))
            .max()
            .unwrap_or(0)
    }
}

impl Default for ComputationDAG {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ComputationDAG {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        writeln!(f, "ComputationDAG {{")?;
        writeln!(f, "  nodes: {}", self.nodes.len())?;
        writeln!(f, "  roots: {}", self.roots.len())?;
        writeln!(f, "  leaves: {}", self.leaves.len())?;
        writeln!(f, "  frozen: {}", self.frozen)?;
        write!(f, "}}")
    }
}
