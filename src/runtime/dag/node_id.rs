//! Node ID for the computation DAG
//!
//! Represents a unique identifier for each computation node in the DAG.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A unique identifier for a computation node in the DAG.
///
/// `NodeId` is used to reference nodes within the computation graph.
/// It is generated atomically to ensure uniqueness across threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(pub usize);

impl NodeId {
    /// Create a new NodeId with the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang_runtime::dag::NodeId;
    ///
    /// let id = NodeId(42);
    /// assert_eq!(id.0, 42);
    /// ```
    #[inline]
    pub fn new(value: usize) -> Self {
        NodeId(value)
    }

    /// Returns the inner value of the node ID.
    #[inline]
    pub fn value(&self) -> usize {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

impl Hash for NodeId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Generator for creating unique node IDs.
///
/// `NodeIdGenerator` provides a thread-safe way to generate
/// unique identifiers for nodes in the computation DAG.
#[derive(Debug)]
pub struct NodeIdGenerator {
    next_id: AtomicUsize,
}

impl NodeIdGenerator {
    /// Create a new node ID generator.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang_runtime::dag::NodeIdGenerator;
    ///
    /// let generator = NodeIdGenerator::new();
    /// let id1 = generator.generate();
    /// let id2 = generator.generate();
    /// assert_ne!(id1, id2);
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
        }
    }

    /// Generate a new unique node ID.
    ///
    /// This method atomically increments the internal counter
    /// and returns the new value as a `NodeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang_runtime::dag::NodeIdGenerator;
    ///
    /// let generator = NodeIdGenerator::new();
    /// let id = generator.generate();
    /// assert!(id.value() > 0);
    /// ```
    #[inline]
    pub fn generate(&self) -> NodeId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        NodeId(id)
    }

    /// Reset the generator to start from the given value.
    ///
    /// This is primarily useful for testing or when reusing
    /// a generator in a controlled environment.
    ///
    /// # Examples
    ///
    /// ```
    /// use yaoxiang_runtime::dag::NodeIdGenerator;
    ///
    /// let generator = NodeIdGenerator::new();
    /// generator.generate();
    /// generator.generate();
    /// generator.reset(100);
    /// let id = generator.generate();
    /// assert_eq!(id.value(), 100);
    /// ```
    #[inline]
    pub fn reset(&self, value: usize) {
        self.next_id.store(value, Ordering::SeqCst);
    }
}

impl Default for NodeIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}


