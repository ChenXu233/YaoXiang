//! 实例化图测试

use crate::middle::passes::mono::instantiation_graph::{
    FunctionInstanceNode, InstantiationGraph, InstanceNode,
};
use crate::middle::passes::mono::instance::{GenericFunctionId, GenericTypeId};
use crate::middle::passes::mono::reachability::ReachabilityAnalyzer;
use crate::frontend::typecheck::MonoType;

#[test]
fn test_function_instance_node() {
    let generic_id = GenericFunctionId::new("map".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::Int(32), MonoType::String];

    let node = FunctionInstanceNode::new(generic_id, type_args.clone());
    assert_eq!(node.specialized_name(), "map_int32_string");

    let key = node.type_args_key();
    assert_eq!(key, vec!["int32".to_string(), "string".to_string()]);
}

#[test]
fn test_type_instance_node() {
    let generic_id = GenericTypeId::new("Option".to_string(), vec!["T".to_string()]);
    let type_args = vec![MonoType::Int(32)];

    let node = crate::middle::passes::mono::instantiation_graph::TypeInstanceNode::new(
        generic_id,
        type_args.clone(),
    );
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

    let _map_string_int = graph.add_function_node(
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
    let node_a = graph.add_function_node(GenericFunctionId::new("a".to_string(), vec![]), vec![]);
    let node_b = graph.add_function_node(GenericFunctionId::new("b".to_string(), vec![]), vec![]);
    let node_c = graph.add_function_node(GenericFunctionId::new("c".to_string(), vec![]), vec![]);

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
