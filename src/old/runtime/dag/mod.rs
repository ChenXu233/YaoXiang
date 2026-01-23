//! Computation DAG (Directed Acyclic Graph) for lazy evaluation
//!
//! This module provides the core data structures for representing
//! and manipulating the computation graph used in the lazy evaluation
//! and parallel execution system.
//!
//! # Architecture
//!
//! The DAG module is organized as follows:
//!
//! - [`NodeId`](node_id::NodeId) - Unique identifier for a computation node
//! - [`NodeIdGenerator`](node_id::NodeIdGenerator) - Thread-safe ID generator
//! - [`DAGNode`](node::DAGNode) - A single node in the computation graph
//! - [`DAGNodeKind`](node::DAGNodeKind) - Types of computations a node can represent
//! - [`ComputationDAG`](graph::ComputationDAG) - The complete computation graph
//! - [`DAGError`](graph::DAGError) - Errors that can occur when manipulating the graph

pub mod graph;
pub mod node;
pub mod node_id;

pub use graph::{ComputationDAG, DAGError};
pub use node::{DAGNode, DAGNodeKind};
pub use node_id::{NodeId, NodeIdGenerator};

#[cfg(test)]
mod tests;
