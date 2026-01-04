//! DAG Node for the computation graph
//!
//! Represents a computation unit in the dependency graph.

use std::fmt;

use super::node_id::NodeId;

/// Kind of computation represented by a DAG node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DAGNodeKind {
    /// A function computation
    Compute {
        /// Name of the computation for debugging
        name: String,
    },

    /// A parallel block (contains multiple sub-nodes)
    ParallelBlock {
        /// Number of sub-expressions in the block
        num_exprs: usize,
    },

    /// A data parallel loop (spawn for)
    DataParallel {
        /// Iterator expression node ID
        iterator_id: NodeId,
        /// Body expression node ID
        body_id: NodeId,
        /// Number of iterations
        num_iterations: usize,
    },

    /// A value placeholder (lazy value)
    Value {
        /// The value type for debugging
        type_name: String,
    },

    /// A constant literal
    Constant {
        /// The constant value as string for debugging
        value: String,
    },

    /// An effect/void function (has side effects)
    Effect {
        /// Name of the effect for debugging
        name: String,
    },
}

impl DAGNodeKind {
    /// Check if this node type represents a parallel operation.
    #[inline]
    pub fn is_parallel(&self) -> bool {
        matches!(
            self,
            DAGNodeKind::ParallelBlock { .. } | DAGNodeKind::DataParallel { .. }
        )
    }

    /// Check if this node type represents a computation.
    #[inline]
    pub fn is_compute(&self) -> bool {
        matches!(self, DAGNodeKind::Compute { .. })
    }
}

/// A node in the computation DAG.
///
/// Each node represents a computation unit that may depend on
/// other nodes and may be depended upon by other nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct DAGNode {
    /// Unique identifier for this node
    id: NodeId,

    /// Kind of computation this node represents
    kind: DAGNodeKind,

    /// Nodes that this node depends on (edges pointing FROM dependencies)
    dependencies: Vec<NodeId>,

    /// Nodes that depend on this node (edges pointing TO dependents)
    dependents: Vec<NodeId>,

    /// Whether this node is in a parallel evaluation region
    in_parallel_region: bool,

    /// Priority of this node (for scheduling)
    priority: u8,
}

impl DAGNode {
    /// Create a new DAG node with the given ID and kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::{DAGNode, DAGNodeKind, NodeId};
    ///
    /// let node = DAGNode::new(
    ///     NodeId(0),
    ///     DAGNodeKind::Compute { name: "add".to_string() }
    /// );
    /// ```
    #[inline]
    pub fn new(id: NodeId, kind: DAGNodeKind) -> Self {
        Self {
            id,
            kind,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            in_parallel_region: false,
            priority: 1,
        }
    }

    /// Get the node's unique identifier.
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get the kind of computation this node represents.
    #[inline]
    pub fn kind(&self) -> &DAGNodeKind {
        &self.kind
    }

    /// Get a mutable reference to the node's kind.
    #[inline]
    pub fn kind_mut(&mut self) -> &mut DAGNodeKind {
        &mut self.kind
    }

    /// Get the list of nodes this node depends on.
    #[inline]
    pub fn dependencies(&self) -> &[NodeId] {
        &self.dependencies
    }

    /// Get the list of nodes that depend on this node.
    #[inline]
    pub fn dependents(&self) -> &[NodeId] {
        &self.dependents
    }

    /// Add a dependency edge (this node depends on `dependency`).
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang::runtime::dag::{DAGNode, DAGNodeKind, NodeId};
    ///
    /// let mut node = DAGNode::new(NodeId(0), DAGNodeKind::Compute { name: "a".to_string() });
    /// node.add_dependency(NodeId(1));
    /// node.add_dependency(NodeId(2));
    /// assert_eq!(node.dependencies().len(), 2);
    /// ```
    #[inline]
    pub fn add_dependency(&mut self, dependency: NodeId) {
        self.dependencies.push(dependency);
    }

    /// Add a dependent edge (`dependent` depends on this node).
    #[inline]
    pub fn add_dependent(&mut self, dependent: NodeId) {
        self.dependents.push(dependent);
    }

    /// Check if this node depends on the given node.
    #[inline]
    pub fn depends_on(&self, node_id: NodeId) -> bool {
        self.dependencies.contains(&node_id)
    }

    /// Check if the given node depends on this node.
    #[inline]
    pub fn has_dependent(&self, node_id: NodeId) -> bool {
        self.dependents.contains(&node_id)
    }

    /// Get the number of dependencies.
    #[inline]
    pub fn num_dependencies(&self) -> usize {
        self.dependencies.len()
    }

    /// Get the number of dependents.
    #[inline]
    pub fn num_dependents(&self) -> usize {
        self.dependents.len()
    }

    /// Check if this node is a leaf (no dependents).
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.dependents.is_empty()
    }

    /// Check if this node is a root (no dependencies).
    #[inline]
    pub fn is_root(&self) -> bool {
        self.dependencies.is_empty()
    }

    /// Mark this node as being in a parallel evaluation region.
    #[inline]
    pub fn set_in_parallel_region(&mut self, in_region: bool) {
        self.in_parallel_region = in_region;
    }

    /// Check if this node is in a parallel evaluation region.
    #[inline]
    pub fn is_in_parallel_region(&self) -> bool {
        self.in_parallel_region
    }

    /// Set the priority of this node.
    #[inline]
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority.clamp(0, 255);
    }

    /// Get the priority of this node.
    #[inline]
    pub fn priority(&self) -> u8 {
        self.priority
    }

    /// Get the number of edges in and out of this node.
    #[inline]
    pub fn degree(&self) -> usize {
        self.dependencies.len() + self.dependents.len()
    }
}

impl fmt::Display for DAGNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DAGNode({}: {:?})", self.id, self.kind)
    }
}
