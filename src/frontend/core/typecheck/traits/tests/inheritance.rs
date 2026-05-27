//! 继承测试 — 基于语言规范 §3.5 & RFC-011 §2
//!
//! §3.5: 接口类型
//! RFC-011 §2: 类型约束系统

use std::collections::HashSet;

use crate::frontend::core::typecheck::traits::inheritance::{
    InheritanceChecker, InheritanceError, TraitInheritanceGraph,
};
use crate::frontend::core::parser::ast::Type;
use crate::util::span::Span;

/// 辅助：构造 `Type::Name` 节点
fn ast_name(name: &str) -> Type {
    Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

// ===================================================================
// TraitInheritanceGraph — Happy path 测试
// ===================================================================

#[test]
fn test_graph_new_creates_empty_graph() {
    // Arrange & Act
    let graph = TraitInheritanceGraph::new();

    // Assert
    assert!(graph.nodes().is_empty(), "新建图的节点集应为空");
}

#[test]
fn test_graph_add_edge_registers_nodes() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();

    // Act
    graph.add_edge("Child", "Parent");

    // Assert
    let nodes = graph.nodes();
    assert!(nodes.contains("Child"), "添加边后应包含子节点");
    assert!(nodes.contains("Parent"), "添加边后应包含父节点");
    assert_eq!(nodes.len(), 2, "添加一条边后应恰好有 2 个节点");
}

#[test]
fn test_graph_parents_returns_direct_parents() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("A", "C");

    // Act
    let parents = graph.parents("A");

    // Assert
    assert!(parents.is_some(), "A 应该有父节点");
    let parents = parents.unwrap();
    assert_eq!(parents.len(), 2, "A 应该有 2 个直接父节点");
    assert!(parents.contains(&"B".to_string()), "应包含父节点 B");
    assert!(parents.contains(&"C".to_string()), "应包含父节点 C");
}

#[test]
fn test_graph_parents_returns_none_for_leaf() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");

    // Act
    let parents = graph.parents("B");

    // Assert
    assert!(parents.is_none(), "叶子节点 B 不应有父节点");
}

#[test]
fn test_graph_all_ancestors_collects_transitive() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("B", "C");
    graph.add_edge("C", "D");

    // Act
    let ancestors = graph.all_ancestors("A");

    // Assert
    assert_eq!(ancestors.len(), 3, "A 应有 3 个祖先 (B, C, D)");
    assert!(ancestors.contains(&"B".to_string()), "应包含直接父节点 B");
    assert!(ancestors.contains(&"C".to_string()), "应包含间接祖先 C");
    assert!(ancestors.contains(&"D".to_string()), "应包含间接祖先 D");
}

#[test]
fn test_graph_all_ancestors_deduplicates_diamond() {
    // Arrange — 菱形继承: A -> B, A -> C, B -> D, C -> D
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("A", "C");
    graph.add_edge("B", "D");
    graph.add_edge("C", "D");

    // Act
    let ancestors = graph.all_ancestors("A");

    // Assert
    let d_count = ancestors.iter().filter(|a| *a == "D").count();
    assert_eq!(d_count, 1, "菱形继承中 D 只应出现一次");
}

#[test]
fn test_graph_is_in_ancestors_direct() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");

    // Act & Assert
    assert!(graph.is_in_ancestors("A", "B"), "B 是 A 的直接父节点");
}

#[test]
fn test_graph_is_in_ancestors_transitive() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("B", "C");

    // Act & Assert
    assert!(graph.is_in_ancestors("A", "C"), "C 是 A 的间接祖先");
}

#[test]
fn test_graph_is_in_ancestors_negative() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");

    // Act & Assert
    assert!(!graph.is_in_ancestors("B", "A"), "父节点不应是子节点的祖先");
}

#[test]
fn test_graph_find_cycle_returns_none_when_acyclic() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("B", "C");

    // Act
    let cycle = graph.find_cycle();

    // Assert
    assert!(cycle.is_none(), "无环图不应检测到循环");
}

#[test]
fn test_graph_find_cycle_detects_self_loop() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "A");

    // Act
    let cycle = graph.find_cycle();

    // Assert
    assert!(cycle.is_some(), "自环应被检测到");
    let cycle = cycle.unwrap();
    assert_eq!(cycle.first(), cycle.last(), "循环路径应闭合");
}

#[test]
fn test_graph_find_cycle_detects_two_node_cycle() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("A", "B");
    graph.add_edge("B", "A");

    // Act
    let cycle = graph.find_cycle();

    // Assert
    assert!(cycle.is_some(), "A <-> B 循环应被检测到");
}

#[test]
fn test_graph_nodes_reflect_all_added() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("X", "Y");
    graph.add_edge("Y", "Z");

    // Act
    let nodes = graph.nodes();

    // Assert
    let expected: HashSet<String> = ["X", "Y", "Z"].iter().map(|s| s.to_string()).collect();
    assert_eq!(*nodes, expected, "节点集应包含所有添加的节点");
}

// ===================================================================
// TraitInheritanceGraph — Boundary 测试
// ===================================================================

#[test]
fn test_graph_single_node_no_edges() {
    // Arrange
    let mut graph = TraitInheritanceGraph::new();
    graph.add_edge("Solo", "Solo");

    // Act
    let ancestors = graph.all_ancestors("Solo");

    // Assert — 自环节点的祖先不应导致无限递归
    assert!(ancestors.len() <= 1, "单节点自环的祖先列表长度应 <= 1");
}

#[test]
fn test_graph_deep_chain_no_panic() {
    // Arrange — 构造 50 层深继承链
    let mut graph = TraitInheritanceGraph::new();
    for i in 0..50 {
        let child = format!("T{}", i);
        let parent = format!("T{}", i + 1);
        graph.add_edge(&child, &parent);
    }

    // Act
    let ancestors = graph.all_ancestors("T0");

    // Assert
    assert_eq!(ancestors.len(), 50, "50 层链应产生 50 个祖先");
}

// ===================================================================
// InheritanceChecker — Happy path 测试
// ===================================================================

#[test]
fn test_checker_new_creates_empty_checker() {
    // Arrange & Act
    let checker = InheritanceChecker::new();

    // Assert — validate 通过表示空检查器正常
    assert!(checker.validate().is_ok(), "空检查器的 validate 应通过");
}

#[test]
fn test_checker_register_and_validate_parent() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.register_trait("Drawable");
    checker.add_trait("Circle", &[ast_name("Drawable")]);

    // Act
    let result = checker.validate_parent_traits();

    // Assert
    assert!(
        result.is_ok(),
        "已注册父 Trait 的情况下 validate_parent_traits 应通过"
    );
}

#[test]
fn test_checker_check_cycles_passes_for_acyclic() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.register_trait("A");
    checker.register_trait("B");
    checker.add_trait("A", &[ast_name("B")]);

    // Act
    let result = checker.check_cycles();

    // Assert
    assert!(result.is_ok(), "无环继承不应报错");
}

#[test]
fn test_checker_validate_full_passes() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.register_trait("Serializable");
    checker.register_trait("Debug");
    checker.add_trait("MyStruct", &[ast_name("Serializable"), ast_name("Debug")]);

    // Act
    let result = checker.validate();

    // Assert
    assert!(
        result.is_ok(),
        "所有父 Trait 已注册且无环时 validate 应通过"
    );
}

#[test]
fn test_checker_add_trait_with_multiple_parents() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.register_trait("P1");
    checker.register_trait("P2");
    checker.register_trait("P3");
    checker.add_trait("Child", &[ast_name("P1"), ast_name("P2"), ast_name("P3")]);

    // Act
    let result = checker.validate();

    // Assert
    assert!(result.is_ok(), "具有多个已注册父 Trait 的验证应通过");
}

// ===================================================================
// InheritanceChecker — Error path 测试
// ===================================================================

#[test]
fn test_checker_validate_parent_traits_fails_when_undefined() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.add_trait("Circle", &[ast_name("Drawable")]);

    // Act
    let result = checker.validate_parent_traits();

    // Assert
    assert!(
        result.is_err(),
        "未注册的父 Trait 应导致 validate_parent_traits 失败"
    );
    match result.unwrap_err() {
        InheritanceError::ParentNotFound(parents) => {
            assert!(
                parents.contains(&"Drawable".to_string()),
                "错误信息应包含未定义的父 Trait 名称"
            );
        }
        other => panic!("期望 ParentNotFound 错误，实际得到: {}", other),
    }
}

#[test]
fn test_checker_validate_parent_traits_reports_multiple_undefined() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.add_trait("Child", &[ast_name("MissingA"), ast_name("MissingB")]);

    // Act
    let result = checker.validate_parent_traits();

    // Assert
    assert!(result.is_err(), "多个未注册父 Trait 应报错");
    match result.unwrap_err() {
        InheritanceError::ParentNotFound(parents) => {
            assert_eq!(parents.len(), 2, "应报告 2 个未定义的父 Trait");
        }
        other => panic!("期望 ParentNotFound 错误，实际得到: {}", other),
    }
}

#[test]
fn test_checker_check_cycles_fails_for_cyclic() {
    // Arrange
    let mut checker = InheritanceChecker::new();
    checker.register_trait("A");
    checker.register_trait("B");
    checker.add_trait("A", &[ast_name("B")]);
    checker.add_trait("B", &[ast_name("A")]);

    // Act
    let result = checker.check_cycles();

    // Assert
    assert!(result.is_err(), "循环继承应导致 check_cycles 失败");
    match result.unwrap_err() {
        InheritanceError::CyclicInheritance(cycle) => {
            assert!(!cycle.is_empty(), "循环路径不应为空");
        }
        other => panic!("期望 CyclicInheritance 错误，实际得到: {}", other),
    }
}

#[test]
fn test_checker_validate_reports_parent_not_found_before_cycle() {
    // Arrange — 既有未定义父节点又有环
    let mut checker = InheritanceChecker::new();
    checker.add_trait("A", &[ast_name("B")]);
    checker.add_trait("B", &[ast_name("A")]);

    // Act
    let result = checker.validate();

    // Assert — validate 先调用 validate_parent_traits 再检查 cycle
    assert!(result.is_err(), "既有未定义父节点又有环时 validate 应失败");
}

// ===================================================================
// InheritanceError — Display 测试
// ===================================================================

#[test]
fn test_inheritance_error_display_cyclic() {
    // Arrange
    let err = InheritanceError::CyclicInheritance(vec![
        "A".to_string(),
        "B".to_string(),
        "A".to_string(),
    ]);

    // Act
    let msg = format!("{}", err);

    // Assert
    assert!(
        msg.contains("Cyclic"),
        "循环错误信息应包含 'Cyclic'，实际: {}",
        msg
    );
    assert!(
        msg.contains("A") && msg.contains("B"),
        "错误信息应包含循环中的节点名称"
    );
}

#[test]
fn test_inheritance_error_display_parent_not_found() {
    // Arrange
    let err = InheritanceError::ParentNotFound(vec!["Missing".to_string()]);

    // Act
    let msg = format!("{}", err);

    // Assert
    assert!(
        msg.contains("Undefined"),
        "未定义父 Trait 错误信息应包含 'Undefined'，实际: {}",
        msg
    );
    assert!(msg.contains("Missing"), "错误信息应包含缺失的父 Trait 名称");
}

#[test]
fn test_inheritance_error_is_std_error() {
    // Arrange
    let err = InheritanceError::ParentNotFound(vec!["X".to_string()]);

    // Act
    let _: &dyn std::error::Error = &err;

    // Assert — 编译通过即证明实现了 std::error::Error
}

// ===================================================================
// InheritanceChecker — Boundary 测试
// ===================================================================

#[test]
fn test_checker_empty_graph_validate_passes() {
    // Arrange
    let checker = InheritanceChecker::new();

    // Act
    let result = checker.validate();

    // Assert
    assert!(result.is_ok(), "空图的完整验证应通过");
}

#[test]
fn test_checker_register_trait_idempotent() {
    // Arrange
    let mut checker = InheritanceChecker::new();

    // Act — 重复注册同一 Trait
    checker.register_trait("Drawable");
    checker.register_trait("Drawable");
    checker.add_trait("Circle", &[ast_name("Drawable")]);

    // Assert
    assert!(
        checker.validate().is_ok(),
        "重复注册同名 Trait 不应导致验证失败"
    );
}

#[test]
fn test_checker_non_name_type_parent_ignored() {
    // Arrange — 添加一个非 Name 类型的父节点（应被忽略）
    let mut checker = InheritanceChecker::new();
    checker.add_trait("MyTrait", &[Type::Int(32)]);

    // Act
    let result = checker.validate_parent_traits();

    // Assert
    assert!(result.is_ok(), "非 Name 类型的父节点应被忽略，不报错");
}
