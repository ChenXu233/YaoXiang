//! 工作区符号搜索处理器测试
//!
//! 测试覆盖：
//! - 空查询返回所有符号
//! - 精确匹配
//! - 前缀匹配
//! - 模糊匹配
//! - 无匹配
//! - 模糊匹配算法
//! - 匹配得分算法

use lsp_types::{SymbolInformation, SymbolKind as LspSymbolKind, WorkspaceSymbolParams};

use crate::frontend::core::typecheck::semantic_db::{DefId, DefinitionInfo, DefinitionKind};
use crate::lsp::handlers::workspace_symbol::{
    handle_workspace_symbol, fuzzy_match, match_score,
};
use crate::lsp::world::World;
use crate::util::span::{Position, Span};

fn make_world_with_symbols() -> World {
    let mut world = World::new();

    let uri = "file:///test.yx";
    let span = Span {
        start: Position {
            line: 1,
            column: 1,
            offset: 0,
        },
        end: Position {
            line: 1,
            column: 10,
            offset: 9,
        },
    };

    world.semantic_db_mut().add_definition(
        uri,
        DefinitionInfo {
            def_id: DefId {
                file_path: uri.to_string(),
                span,
            },
            name: "fibonacci".to_string(),
            kind: DefinitionKind::Function,
            span,
            file_path: uri.to_string(),
            type_info: Some("(Int) -> Int".to_string()),
            signature: Some("fibonacci: (n: Int) -> Int".to_string()),
        },
    );

    world.semantic_db_mut().add_definition(
        uri,
        DefinitionInfo {
            def_id: DefId {
                file_path: uri.to_string(),
                span,
            },
            name: "find_by_id".to_string(),
            kind: DefinitionKind::Function,
            span,
            file_path: uri.to_string(),
            type_info: Some("(Int) -> Item".to_string()),
            signature: Some("find_by_id: (id: Int) -> Item".to_string()),
        },
    );

    world.semantic_db_mut().add_definition(
        uri,
        DefinitionInfo {
            def_id: DefId {
                file_path: uri.to_string(),
                span,
            },
            name: "User".to_string(),
            kind: DefinitionKind::Type,
            span,
            file_path: uri.to_string(),
            type_info: Some("Type".to_string()),
            signature: None,
        },
    );

    world
}

#[test]
fn test_workspace_symbol_empty_query() {
    let world = make_world_with_symbols();
    let params = WorkspaceSymbolParams {
        query: String::new(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = handle_workspace_symbol(&world, params);
    assert!(result.is_some());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 3, "空查询应返回所有符号");
}

#[test]
fn test_workspace_symbol_exact_match() {
    let world = make_world_with_symbols();
    let params = WorkspaceSymbolParams {
        query: "fibonacci".to_string(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = handle_workspace_symbol(&world, params);
    assert!(result.is_some());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1, "精确匹配应返回 1 个结果");
    assert_eq!(symbols[0].name, "fibonacci");
}

#[test]
fn test_workspace_symbol_prefix_match() {
    let world = make_world_with_symbols();
    let params = WorkspaceSymbolParams {
        query: "fibon".to_string(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = handle_workspace_symbol(&world, params);
    assert!(result.is_some());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1, "前缀匹配应返回 1 个结果");
    assert_eq!(symbols[0].name, "fibonacci");
}

#[test]
fn test_workspace_symbol_fuzzy_match() {
    let world = make_world_with_symbols();
    let params = WorkspaceSymbolParams {
        query: "fid".to_string(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = handle_workspace_symbol(&world, params);
    assert!(result.is_some());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 1, "模糊匹配应返回 1 个结果");
    assert_eq!(symbols[0].name, "find_by_id");
}

#[test]
fn test_workspace_symbol_no_match() {
    let world = make_world_with_symbols();
    let params = WorkspaceSymbolParams {
        query: "nonexistent".to_string(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = handle_workspace_symbol(&world, params);
    assert!(result.is_none(), "无匹配应返回 None");
}

#[test]
fn test_fuzzy_match() {
    assert!(fuzzy_match("fib", "fibonacci"));
    assert!(fuzzy_match("fid", "find_by_id"));
    assert!(fuzzy_match("fib", "Fibonacci"));
    assert!(!fuzzy_match("fib", "binary"));
}

#[test]
fn test_match_score() {
    assert_eq!(match_score("fibonacci", "fibonacci"), 0);
    assert_eq!(match_score("fib", "fibonacci"), 2);
    assert_eq!(match_score("fid", "find_by_id"), 4);
}
