//! Module 系统测试

use std::path::PathBuf;
use crate::middle::passes::module::{ModuleId, ModuleGraph, ModuleGraphError};

#[test]
fn test_module_id() {
    let id1 = ModuleId::new(0);
    let id2 = ModuleId::new(1);

    assert_eq!(id1.index(), 0);
    assert_eq!(id2.index(), 1);
    assert_ne!(id1, id2);
}

#[test]
fn test_module_graph_basic() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));

    // a 依赖 b，b 依赖 c
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();

    assert_eq!(graph.len(), 3);

    // 检查依赖关系
    let deps_a = graph.get_dependencies(id_a).unwrap();
    assert_eq!(deps_a, vec![id_b]);

    let deps_b = graph.get_dependencies(id_b).unwrap();
    assert_eq!(deps_b, vec![id_c]);
}

#[test]
fn test_topological_sort() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));

    // a 依赖 b，b 依赖 c
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();

    let sorted = graph.topological_sort().unwrap();

    // c 应该在最前面（没有依赖）
    // b 在中间（依赖 c）
    // a 在最后（依赖 b）
    assert_eq!(sorted[0], id_c);
    assert_eq!(sorted[1], id_b);
    assert_eq!(sorted[2], id_a);
}

#[test]
fn test_cycle_detection() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));

    // a 依赖 b，b 依赖 c，c 依赖 a
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();
    graph.add_dependency(id_c, id_a, true).unwrap();

    let result = graph.topological_sort();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), ModuleGraphError::TopologySortFailed);
}

#[test]
fn test_self_dependency() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));

    let result = graph.add_dependency(id_a, id_a, true);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ModuleGraphError::InvalidDependency { .. }
    ));
}

#[test]
fn test_public_vs_private_dependency() {
    let mut graph = ModuleGraph::new();

    let id_a = graph.add_module(PathBuf::from("a.yx"));
    let id_b = graph.add_module(PathBuf::from("b.yx"));
    let id_c = graph.add_module(PathBuf::from("c.yx"));

    // a 公开依赖 b，b 公开依赖 c
    graph.add_dependency(id_a, id_b, true).unwrap();
    graph.add_dependency(id_b, id_c, true).unwrap();

    // a 私有依赖 c
    graph.add_dependency(id_a, id_c, false).unwrap();

    // 检查公开依赖
    let public_deps_a = graph.get_public_dependencies(id_a).unwrap();
    assert_eq!(public_deps_a, vec![id_b]);

    // 检查传递闭包
    let closure = graph.get_public_dependency_closure(id_a).unwrap();
    assert!(closure.contains(&id_b));
    assert!(closure.contains(&id_c));
}
