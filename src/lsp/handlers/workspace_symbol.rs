//! 工作区符号搜索处理
//!
//! **状态**：阶段 3 (v0.8) 实现
//!
//! 支持：
//! - 空查询返回所有符号
//! - 模糊匹配查询
//! - 结果按匹配得分排序

use lsp_types::{SymbolInformation, SymbolKind as LspSymbolKind, WorkspaceSymbolParams};
use tracing::debug;

use crate::frontend::core::typecheck::semantic_db::{DefinitionInfo, DefinitionKind};
use crate::lsp::locate::span_to_range;
use crate::lsp::world::World;

/// 处理 `workspace/symbol` 请求
///
/// 搜索整个工作区中匹配查询的符号：
/// 1. 空查询返回所有符号（受数量限制）
/// 2. 非空查询进行模糊匹配
/// 3. 结果按匹配得分排序
pub fn handle_workspace_symbol(
    world: &World,
    params: WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
    let query = &params.query;
    debug!("工作区符号搜索: {:?}", query);

    let db = world.semantic_db();

    // 最大返回数量限制
    const MAX_RESULTS: usize = 256;

    let mut results: Vec<(u32, SymbolInformation)> = Vec::new();

    // 收集所有文件的定义
    for file_path in db.file_paths() {
        let defs = db.get_definitions(file_path);
        for def in defs {
            if !query.is_empty() && !fuzzy_match(query, &def.name) {
                continue;
            }

            let score = if query.is_empty() {
                3
            } else {
                match_score(query, &def.name)
            };

            if let Some(info) = def_to_info(def) {
                results.push((score, info));
            }

            if results.len() >= MAX_RESULTS {
                break;
            }
        }
    }

    // 按匹配得分排序（越小越好），同分按名称排序
    results.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));

    let symbols: Vec<SymbolInformation> = results
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, info)| info)
        .collect();

    debug!("工作区符号搜索结果: {} 个", symbols.len());

    if symbols.is_empty() {
        None
    } else {
        Some(symbols)
    }
}

/// 将 DefinitionKind 转换为 LSP SymbolKind
fn def_kind_to_symbol_kind(kind: DefinitionKind) -> LspSymbolKind {
    match kind {
        DefinitionKind::Function | DefinitionKind::Method => LspSymbolKind::FUNCTION,
        DefinitionKind::Type | DefinitionKind::Interface => LspSymbolKind::CLASS,
        DefinitionKind::Variable | DefinitionKind::Parameter => LspSymbolKind::VARIABLE,
        DefinitionKind::GenericParameter => LspSymbolKind::TYPE_PARAMETER,
    }
}

/// 将 DefinitionInfo 转换为 SymbolInformation
fn def_to_info(def: &DefinitionInfo) -> Option<SymbolInformation> {
    let uri = lsp_types::Uri::from_str(&def.file_path).ok()?;
    let range = span_to_range(&def.span);

    Some(SymbolInformation {
        name: def.name.clone(),
        kind: def_kind_to_symbol_kind(def.kind.clone()),
        tags: None,
        location: lsp_types::Location::new(uri, range),
        container_name: None,
        #[allow(deprecated)]
        deprecated: None,
    })
}

/// 模糊匹配：查询的每个字符按顺序出现在名称中
fn fuzzy_match(
    query: &str,
    name: &str,
) -> bool {
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();

    let mut qi = 0;
    for ch in name_lower.chars() {
        if qi < query_lower.len() && ch == query_lower.as_bytes()[qi] as char {
            qi += 1;
        }
    }

    qi == query_lower.len()
}

/// 匹配得分（越小越好）
fn match_score(
    query: &str,
    name: &str,
) -> u32 {
    if query == name {
        return 0;
    }

    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();

    if query_lower == name_lower {
        return 1;
    }

    if name_lower.starts_with(&query_lower) {
        return 2;
    }

    if name_lower.contains(&query_lower) {
        return 3;
    }

    // 模糊匹配得分
    4
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::typecheck::semantic_db::{DefId, DefinitionInfo, DefinitionKind};
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
}
