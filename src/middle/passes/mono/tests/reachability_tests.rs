//! 可达性分析测试

use crate::middle::passes::mono::instantiation_graph::{
    FunctionInstanceNode, InstanceNode, InstantiationGraph,
};
use crate::middle::passes::mono::instance::GenericFunctionId;
use crate::middle::passes::mono::reachability::{DeadCodeEliminator, ReachabilityAnalyzer};

fn create_test_graph() -> InstantiationGraph {
    let mut graph = InstantiationGraph::new();

    // main -> foo -> bar
    //      \-> baz
    // 其中 bar 是死代码（未被任何入口点引用）

    let main = InstanceNode::Function(FunctionInstanceNode::new(
        GenericFunctionId::new("main".to_string(), vec![]),
        vec![],
    ));

    let foo = InstanceNode::Function(FunctionInstanceNode::new(
        GenericFunctionId::new("foo".to_string(), vec![]),
        vec![],
    ));

    let bar = InstanceNode::Function(FunctionInstanceNode::new(
        GenericFunctionId::new("bar".to_string(), vec![]),
        vec![],
    ));

    let baz = InstanceNode::Function(FunctionInstanceNode::new(
        GenericFunctionId::new("baz".to_string(), vec![]),
        vec![],
    ));

    graph.add_function_node(GenericFunctionId::new("main".to_string(), vec![]), vec![]);
    graph.add_function_node(GenericFunctionId::new("foo".to_string(), vec![]), vec![]);
    graph.add_function_node(GenericFunctionId::new("bar".to_string(), vec![]), vec![]);
    graph.add_function_node(GenericFunctionId::new("baz".to_string(), vec![]), vec![]);

    graph.add_dependency(&main, &foo);
    graph.add_dependency(&main, &baz);
    graph.add_dependency(&foo, &bar); // bar 只被 foo 引用，但 foo 被 main 引用

    // bar 未被任何入口点直接引用，但我们通过 foo 引用了它
    // 所以实际上 bar 也应该是可达的

    graph.add_entry_point(main);

    graph
}

#[test]
fn test_reachability_analysis() {
    let graph = create_test_graph();
    let analyzer = ReachabilityAnalyzer::new();
    let analysis = analyzer.analyze(&graph);

    assert_eq!(analysis.reachable_count(), 4);
    assert_eq!(analysis.unreachable_count(), 0);
}

#[test]
fn test_dead_code_elimination() {
    let graph = create_test_graph();
    let eliminator = DeadCodeEliminator::new();
    let kept = eliminator.eliminate(&graph);

    assert_eq!(kept.len(), 4);
}

#[test]
fn test_elimination_rate() {
    let graph = create_test_graph();
    let analyzer = ReachabilityAnalyzer::new();
    let analysis = analyzer.analyze(&graph);

    assert_eq!(analysis.elimination_rate(), 0.0); // 没有死代码
}

#[test]
fn test_depth_analysis() {
    let graph = create_test_graph();
    let analyzer = ReachabilityAnalyzer::new();
    let analysis = analyzer.analyze(&graph);

    // main 深度为 0
    // foo 和 baz 深度为 1
    // bar 深度为 2

    assert_eq!(analysis.max_depth(), Some(2));
}
