//! DAG 模块单元测试
//!
//! 测试计算图的所有核心功能

use crate::runtime::dag::{ComputationDAG, DAGError, DAGNodeKind};
use crate::runtime::dag::node_id::NodeId;

#[cfg(test)]
mod node_id_tests {
    use super::*;

    #[test]
    fn test_node_id_new() {
        let id = NodeId(1);
        assert_eq!(id.value(), 1);
    }

    #[test]
    fn test_node_id_clone() {
        let id = NodeId(42);
        let cloned = id;
        assert_eq!(id.value(), cloned.value());
    }

    #[test]
    fn test_node_id_partial_eq() {
        assert_eq!(NodeId(1), NodeId(1));
        assert_ne!(NodeId(1), NodeId(2));
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId(42);
        let display = format!("{}", id);
        assert!(display.contains("42"));
    }
}

#[cfg(test)]
mod dag_node_tests {
    use super::*;
    use crate::runtime::dag::DAGNode;

    #[test]
    fn test_dag_node_new() {
        let node = DAGNode::new(
            NodeId(0),
            DAGNodeKind::Compute { name: "add".to_string() },
        );
        assert_eq!(node.id(), NodeId(0));
        assert!(node.dependencies().is_empty());
        assert!(node.dependents().is_empty());
    }

    #[test]
    fn test_dag_node_add_dependency() {
        let mut node = DAGNode::new(
            NodeId(0),
            DAGNodeKind::Compute { name: "c".to_string() },
        );
        node.add_dependency(NodeId(1));
        node.add_dependency(NodeId(2));
        assert_eq!(node.dependencies().len(), 2);
        assert!(node.depends_on(NodeId(1)));
        assert!(node.depends_on(NodeId(2)));
        assert!(!node.depends_on(NodeId(3)));
    }

    #[test]
    fn test_dag_node_add_dependent() {
        let mut node = DAGNode::new(
            NodeId(0),
            DAGNodeKind::Compute { name: "a".to_string() },
        );
        node.add_dependent(NodeId(1));
        node.add_dependent(NodeId(2));
        assert_eq!(node.dependents().len(), 2);
        assert!(node.has_dependent(NodeId(1)));
        assert!(node.has_dependent(NodeId(2)));
    }

    #[test]
    fn test_dag_node_is_root_and_leaf() {
        let mut root = DAGNode::new(
            NodeId(0),
            DAGNodeKind::Constant { value: "42".to_string() },
        );
        assert!(root.is_root());
        assert!(root.is_leaf());

        let mut leaf = DAGNode::new(
            NodeId(1),
            DAGNodeKind::Compute { name: "result".to_string() },
        );
        leaf.add_dependency(NodeId(0));
        assert!(!leaf.is_root());
        assert!(leaf.is_leaf());
    }

    #[test]
    fn test_dag_node_priority() {
        let mut node = DAGNode::new(
            NodeId(0),
            DAGNodeKind::Compute { name: "high".to_string() },
        );
        assert_eq!(node.priority(), 1);

        node.set_priority(200);
        assert_eq!(node.priority(), 200);

        // Priority should be clamped to 0-255
        node.set_priority(255);
        assert_eq!(node.priority(), 255);

        node.set_priority(0);
        assert_eq!(node.priority(), 0);
    }

    #[test]
    fn test_dag_node_parallel_region() {
        let mut node = DAGNode::new(
            NodeId(0),
            DAGNodeKind::ParallelBlock { num_exprs: 2 },
        );
        assert!(!node.is_in_parallel_region());

        node.set_in_parallel_region(true);
        assert!(node.is_in_parallel_region());
    }

    #[test]
    fn test_dag_node_kind_is_parallel() {
        let compute = DAGNodeKind::Compute { name: "add".to_string() };
        let parallel_block = DAGNodeKind::ParallelBlock { num_exprs: 3 };
        let data_parallel = DAGNodeKind::DataParallel {
            iterator_id: NodeId(0),
            body_id: NodeId(1),
            num_iterations: 10,
        };

        assert!(!compute.is_parallel());
        assert!(parallel_block.is_parallel());
        assert!(data_parallel.is_parallel());
    }
}

#[cfg(test)]
mod computation_dag_tests {
    use super::*;

    #[test]
    fn test_dag_new() {
        let dag = ComputationDAG::new();
        assert!(dag.is_empty());
        assert_eq!(dag.num_nodes(), 0);
    }

    #[test]
    fn test_dag_with_capacity() {
        let dag = ComputationDAG::with_capacity(100);
        assert!(dag.is_empty());
    }

    #[test]
    fn test_dag_add_node() {
        let mut dag = ComputationDAG::new();
        let id = dag.add_node(DAGNodeKind::Constant { value: "42".to_string() }).unwrap();
        assert!(dag.contains_node(id));
        assert_eq!(dag.num_nodes(), 1);
    }

    #[test]
    fn test_dag_add_edge() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "add".to_string() }).unwrap();

        dag.add_edge(a, c).unwrap();
        dag.add_edge(b, c).unwrap();

        assert!(dag.has_edge(a, c));
        assert!(dag.has_edge(b, c));
    }

    #[test]
    fn test_dag_edge_node_not_found() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();

        assert_eq!(dag.add_edge(a, NodeId(999)), Err(DAGError::NodeNotFound(NodeId(999))));
    }

    #[test]
    fn test_dag_cycle_detection() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();

        dag.add_edge(a, b).unwrap();

        // Adding b -> a would create a cycle
        assert_eq!(dag.add_edge(b, a), Err(DAGError::CycleDetected));
    }

    #[test]
    fn test_dag_self_cycle_detection() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();

        // Adding a -> a is a self-cycle
        assert_eq!(dag.add_edge(a, a), Err(DAGError::CycleDetected));
    }

    #[test]
    fn test_dag_duplicate_edge() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Compute { name: "x".to_string() }).unwrap();

        dag.add_edge(a, b).unwrap();
        assert_eq!(dag.add_edge(a, b), Err(DAGError::DuplicateEdge));
    }

    #[test]
    fn test_dag_get_node() {
        let mut dag = ComputationDAG::new();
        let id = dag.add_node(DAGNodeKind::Compute { name: "test".to_string() }).unwrap();
        let node = dag.get_node(id).unwrap();
        assert!(matches!(node.kind(), DAGNodeKind::Compute { .. }));
    }

    #[test]
    fn test_dag_get_node_not_found() {
        let dag = ComputationDAG::new();
        assert_eq!(dag.get_node(NodeId(999)), Err(DAGError::NodeNotFound(NodeId(999))));
    }

    #[test]
    fn test_dag_get_node_mut() {
        let mut dag = ComputationDAG::new();
        let id = dag.add_node(DAGNodeKind::Compute { name: "test".to_string() }).unwrap();
        let node = dag.get_node_mut(id).unwrap();
        node.set_priority(5);
        assert_eq!(dag.get_node(id).unwrap().priority(), 5);
    }

    #[test]
    fn test_dag_roots_and_leaves() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "add".to_string() }).unwrap();

        // Initially all nodes are both roots and leaves
        assert_eq!(dag.roots().len(), 3);
        assert_eq!(dag.leaves().len(), 3);

        dag.add_edge(a, c).unwrap();
        dag.add_edge(b, c).unwrap();

        // a and b are roots, c is a leaf
        assert!(dag.roots().contains(&a));
        assert!(dag.roots().contains(&b));
        assert!(!dag.roots().contains(&c));
        assert!(dag.leaves().contains(&c));
    }

    #[test]
    fn test_dag_ready_nodes() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "add".to_string() }).unwrap();

        // Initially all nodes are ready (no dependencies)
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 3);

        dag.add_edge(a, c).unwrap();
        dag.add_edge(b, c).unwrap();

        // Now only a and b are ready (c depends on them)
        let ready = dag.ready_nodes();
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&a));
        assert!(ready.contains(&b));
        assert!(!ready.contains(&c));
    }

    #[test]
    fn test_dag_topological_sort() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "add".to_string() }).unwrap();
        let d = dag.add_node(DAGNodeKind::Compute { name: "mult".to_string() }).unwrap();

        dag.add_edge(a, c).unwrap();
        dag.add_edge(b, c).unwrap();
        dag.add_edge(c, d).unwrap();

        let order = dag.topological_sort().to_vec();
        assert!(order.contains(&a));
        assert!(order.contains(&b));
        assert!(order.contains(&c));
        assert!(order.contains(&d));

        // a and b should come before c
        let a_pos = order.iter().position(|&x| x == a).unwrap();
        let b_pos = order.iter().position(|&x| x == b).unwrap();
        let c_pos = order.iter().position(|&x| x == c).unwrap();
        assert!(a_pos < c_pos);
        assert!(b_pos < c_pos);

        // c should come before d
        assert!(c_pos < order.iter().position(|&x| x == d).unwrap());
    }

    #[test]
    fn test_dag_max_parallelism() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "x".to_string() }).unwrap();
        let d = dag.add_node(DAGNodeKind::Compute { name: "y".to_string() }).unwrap();
        let e = dag.add_node(DAGNodeKind::Compute { name: "z".to_string() }).unwrap();

        // a and b can run in parallel
        dag.add_edge(a, c).unwrap();
        dag.add_edge(b, d).unwrap();
        // c and d must run before e
        dag.add_edge(c, e).unwrap();
        dag.add_edge(d, e).unwrap();

        // Max parallelism should be 2 (a and b, or c and d)
        assert_eq!(dag.max_parallelism(), 2);
    }

    #[test]
    fn test_dag_critical_path_length() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Compute { name: "a+1".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "b+1".to_string() }).unwrap();
        let d = dag.add_node(DAGNodeKind::Compute { name: "c+1".to_string() }).unwrap();

        dag.add_edge(a, b).unwrap();
        dag.add_edge(b, c).unwrap();
        dag.add_edge(c, d).unwrap();

        // Critical path: a -> b -> c -> d (length 4)
        assert_eq!(dag.critical_path_length(), 4);
    }

    #[test]
    fn test_dag_freeze() {
        let mut dag = ComputationDAG::new();
        let id = dag.add_node(DAGNodeKind::Constant { value: "42".to_string() }).unwrap();

        assert!(!dag.is_frozen());

        dag.freeze();
        assert!(dag.is_frozen());

        // Can't add nodes after freezing
        assert_eq!(
            dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }),
            Err(DAGError::GraphFrozen)
        );
    }

    #[test]
    fn test_dag_freeze_after_edges() {
        let mut dag = ComputationDAG::new();
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Compute { name: "x".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();

        dag.add_edge(a, b).unwrap();
        dag.freeze();

        // Can't add nodes after freezing
        assert_eq!(
            dag.add_node(DAGNodeKind::Constant { value: "3".to_string() }),
            Err(DAGError::GraphFrozen)
        );

        // Can't add edges after freezing
        assert_eq!(dag.add_edge(c, b), Err(DAGError::GraphFrozen));
    }

    #[test]
    fn test_dag_display() {
        let mut dag = ComputationDAG::new();
        dag.add_node(DAGNodeKind::Constant { value: "42".to_string() }).unwrap();
        let display = format!("{}", dag);
        assert!(display.contains("nodes: 1"));
    }

    #[test]
    fn test_dag_parallel_block() {
        let mut dag = ComputationDAG::new();
        let block = dag.add_node(DAGNodeKind::ParallelBlock { num_exprs: 3 }).unwrap();
        let a = dag.add_node(DAGNodeKind::Compute { name: "a".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Compute { name: "b".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Compute { name: "c".to_string() }).unwrap();

        // block -> a, block -> b, block -> c means a, b, c all depend on block
        dag.add_edge(block, a).unwrap();
        dag.add_edge(block, b).unwrap();
        dag.add_edge(block, c).unwrap();

        assert!(dag.get_node(block).unwrap().kind().is_parallel());
        // Level 0: block (1 node)
        // Level 1: a, b, c (3 nodes, all depend on block, can run in parallel)
        assert_eq!(dag.max_parallelism(), 3);
    }

    #[test]
    fn test_dag_data_parallel() {
        let mut dag = ComputationDAG::new();
        let iter = dag.add_node(DAGNodeKind::Constant { value: "range(10)".to_string() }).unwrap();
        let body = dag.add_node(DAGNodeKind::Compute { name: "process".to_string() }).unwrap();
        let dp = dag.add_node(DAGNodeKind::DataParallel {
            iterator_id: iter,
            body_id: body,
            num_iterations: 10,
        }).unwrap();

        dag.add_edge(iter, dp).unwrap();
        dag.add_edge(body, dp).unwrap();

        assert!(dag.get_node(dp).unwrap().kind().is_parallel());
    }

    #[test]
    fn test_dag_complex_graph() {
        // Simulate: a + b, c + d, then (a+b) * (c+d)
        let mut dag = ComputationDAG::new();

        // Leaf nodes (constants)
        let a = dag.add_node(DAGNodeKind::Constant { value: "1".to_string() }).unwrap();
        let b = dag.add_node(DAGNodeKind::Constant { value: "2".to_string() }).unwrap();
        let c = dag.add_node(DAGNodeKind::Constant { value: "3".to_string() }).unwrap();
        let d = dag.add_node(DAGNodeKind::Constant { value: "4".to_string() }).unwrap();

        // First level computations
        let ab = dag.add_node(DAGNodeKind::Compute { name: "add_ab".to_string() }).unwrap();
        let cd = dag.add_node(DAGNodeKind::Compute { name: "add_cd".to_string() }).unwrap();

        // Final computation
        let result = dag.add_node(DAGNodeKind::Compute { name: "mult".to_string() }).unwrap();

        // Build edges
        dag.add_edge(a, ab).unwrap();
        dag.add_edge(b, ab).unwrap();
        dag.add_edge(c, cd).unwrap();
        dag.add_edge(d, cd).unwrap();
        dag.add_edge(ab, result).unwrap();
        dag.add_edge(cd, result).unwrap();

        // Verify properties
        assert_eq!(dag.num_nodes(), 7);
        assert_eq!(dag.roots().len(), 4); // a, b, c, d
        assert_eq!(dag.leaves().len(), 1); // result
        // Level 0: a, b, c, d (4 nodes, all roots, can run in parallel)
        // Level 1: ab, cd (2 nodes)
        // Level 2: result (1 node)
        assert_eq!(dag.max_parallelism(), 4);
        assert_eq!(dag.critical_path_length(), 3); // leaf -> first -> result
    }
}
