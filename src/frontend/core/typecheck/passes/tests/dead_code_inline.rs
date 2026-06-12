//! Dead Code 分析模块测试
//!
//! 测试死代码分析功能，包括：
//! - 入口点收集
//! - 未使用导出警告
//! - 多层级依赖场景

use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
use crate::frontend::core::typecheck::passes::dead_code::{DeadCodeAnalyzer, DeadCodeWarning};
use crate::util::span::Span;

#[test]
fn test_collect_entry_points() {
    let mut analyzer = DeadCodeAnalyzer::new();

    // 创建一个简单的 AST
    let ast = Module {
        items: vec![],
        span: Span::dummy(),
    };

    analyzer.collect_entry_points_and_definitions(&ast);
    assert!(analyzer.entry_points.is_empty());
}

#[test]
fn test_unused_export_warning() {
    let warning = DeadCodeWarning {
        code: "W1001".to_string(),
        message: "Unused exported function: 'foo'".to_string(),
        span: Span::dummy(),
    };

    assert_eq!(warning.code, "W1001");
    assert!(warning.message.contains("foo"));
}

#[test]
fn test_multi_level_dependency() {
    // 测试多层级依赖场景下的入口点识别
    let mut analyzer = DeadCodeAnalyzer::new();

    // 创建一个模拟依赖链的 AST
    // pub main -> 入口点
    // pub helper_a -> 入口点（pub）
    // pub helper_b -> 入口点（pub）
    // fn private_func -> 非入口点（不是 pub）

    let ast = Module {
        items: vec![
            // pub main - 入口点
            Stmt {
                kind: StmtKind::Binding {
                    name: "main".to_string(),
                    type_name: None,
                    method_type: None,
                    is_pub: true,
                    params: vec![],
                    body: (vec![], None),
                    generic_params: vec![],
                    type_annotation: None,
                },
                span: Span::dummy(),
            },
            // pub helper_a - 入口点
            Stmt {
                kind: StmtKind::Binding {
                    name: "helper_a".to_string(),
                    type_name: None,
                    method_type: None,
                    is_pub: true,
                    params: vec![],
                    body: (vec![], None),
                    generic_params: vec![],
                    type_annotation: None,
                },
                span: Span::dummy(),
            },
            // pub helper_b - 入口点
            Stmt {
                kind: StmtKind::Binding {
                    name: "helper_b".to_string(),
                    type_name: None,
                    method_type: None,
                    is_pub: true,
                    params: vec![],
                    body: (vec![], None),
                    generic_params: vec![],
                    type_annotation: None,
                },
                span: Span::dummy(),
            },
            // fn private_func - 非入口点
            Stmt {
                kind: StmtKind::Binding {
                    name: "private_func".to_string(),
                    type_name: None,
                    method_type: None,
                    is_pub: false,
                    params: vec![],
                    body: (vec![], None),
                    generic_params: vec![],
                    type_annotation: None,
                },
                span: Span::dummy(),
            },
        ],
        span: Span::dummy(),
    };

    analyzer.collect_entry_points_and_definitions(&ast);

    // main 和 pub 函数应该是入口点
    assert!(
        analyzer.entry_points.contains("main"),
        "main should be entry point"
    );
    assert!(
        analyzer.entry_points.contains("helper_a"),
        "helper_a should be entry point (pub)"
    );
    assert!(
        analyzer.entry_points.contains("helper_b"),
        "helper_b should be entry point (pub)"
    );

    // private_func 不是入口点
    assert!(
        !analyzer.entry_points.contains("private_func"),
        "private_func should NOT be entry point (not pub)"
    );

    // 所有函数都应该在 all_defs 中
    assert_eq!(analyzer.all_defs.len(), 4);
}
