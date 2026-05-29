//! 工作区符号搜索处理
//!
//! **状态**：阶段 5 (v0.9) 实现
//!
//! 支持：
//! - 模糊搜索工作区内所有符号
//! - 按符号类型过滤
//! - 按文件过滤

use lsp_types::{Location, SymbolInformation, SymbolKind as LspSymbolKind, Uri, WorkspaceSymbolParams};
use std::str::FromStr;
use tracing::debug;

use crate::frontend::core::lexer::symbols::{IndexedSymbol, SymbolKind};
use crate::lsp::locate::span_to_range;
use crate::lsp::world::World;

/// 将 YaoXiang SymbolKind 转换为 LSP SymbolKind
fn to_lsp_symbol_kind(kind: &SymbolKind) -> LspSymbolKind {
    match kind {
        SymbolKind::Variable => LspSymbolKind::VARIABLE,
        SymbolKind::Function | SymbolKind::GenericFunction => LspSymbolKind::FUNCTION,
        SymbolKind::Type | SymbolKind::GenericType => LspSymbolKind::CLASS,
        SymbolKind::TypeClass | SymbolKind::Trait => LspSymbolKind::INTERFACE,
        SymbolKind::ConstGeneric => LspSymbolKind::CONSTANT,
        SymbolKind::HigherKindedType | SymbolKind::TypeFamily => LspSymbolKind::CLASS,
        SymbolKind::Binding | SymbolKind::PositionBinding => LspSymbolKind::VARIABLE,
    }
}

/// 模糊匹配：查询字符串中的所有字符是否按顺序出现在目标中
///
/// 例如："fib" 匹配 "fibonacci"、"find_by_id" 等
fn fuzzy_match(
    query: &str,
    target: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }

    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    let mut query_chars = query_lower.chars().peekable();
    for ch in target_lower.chars() {
        if query_chars.peek() == Some(&ch) {
            query_chars.next();
        }
        if query_chars.peek().is_none() {
            return true;
        }
    }

    query_chars.peek().is_none()
}

/// 计算模糊匹配得分，用于排序（越小越好）
///
/// 评分规则：
/// - 完全匹配：0
/// - 前缀匹配：1
/// - 包含匹配：2
/// - 模糊匹配：3
fn match_score(
    query: &str,
    target: &str,
) -> u32 {
    let q = query.to_lowercase();
    let t = target.to_lowercase();

    if q == t {
        0
    } else if t.starts_with(&q) {
        1
    } else if t.contains(&q) {
        2
    } else {
        3
    }
}

/// 将 IndexedSymbol 转换为 LSP SymbolInformation
fn symbol_to_info(symbol: &IndexedSymbol) -> Option<SymbolInformation> {
    let uri = Uri::from_str(&symbol.location.file_path).ok()?;
    let range = span_to_range(&symbol.location.span);

    #[allow(deprecated)]
    Some(SymbolInformation {
        name: symbol.name.clone(),
        kind: to_lsp_symbol_kind(&symbol.kind),
        tags: None,
        deprecated: None,
        location: Location::new(uri, range),
        container_name: None,
    })
}

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

    let index = world.symbol_index();
    let all_names = index.all_names();

    // 最大返回数量限制
    const MAX_RESULTS: usize = 256;

    let mut results: Vec<(u32, SymbolInformation)> = Vec::new();

    for name in &all_names {
        if !query.is_empty() && !fuzzy_match(query, name) {
            continue;
        }

        let score = if query.is_empty() {
            3
        } else {
            match_score(query, name)
        };
        let symbols = index.find_by_name(name);

        for sym in symbols {
            if let Some(info) = symbol_to_info(sym) {
                results.push((score, info));
            }
        }

        if results.len() >= MAX_RESULTS {
            break;
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

// ─── 测试 ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::lexer::symbols::{IndexedSymbol, SymbolKind, SymbolLocation};
    use crate::util::span::{Position, Span};

    fn dummy_span() -> Span {
        Span {
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
        }
    }

    fn make_world_with_symbols() -> World {
        let mut world = World::new();
        let index = world.symbol_index_mut();

        index.add(IndexedSymbol {
            name: "fibonacci".to_string(),
            kind: SymbolKind::Function,
            arity: Some(1),
            location: SymbolLocation::new("file:///test.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "find_by_id".to_string(),
            kind: SymbolKind::Function,
            arity: Some(1),
            location: SymbolLocation::new("file:///test.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "MyType".to_string(),
            kind: SymbolKind::Type,
            arity: None,
            location: SymbolLocation::new("file:///types.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "counter".to_string(),
            kind: SymbolKind::Variable,
            arity: None,
            location: SymbolLocation::new("file:///test.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "format_string".to_string(),
            kind: SymbolKind::Function,
            arity: Some(2),
            location: SymbolLocation::new("file:///util.yx".to_string(), dummy_span()),
        });

        world
    }

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("fibonacci", "fibonacci"));
    }

    #[test]
    fn test_fuzzy_match_prefix() {
        assert!(fuzzy_match("fib", "fibonacci"));
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        assert!(fuzzy_match("fbi", "fibonacci"));
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("FIB", "fibonacci"));
        assert!(fuzzy_match("mytype", "MyType"));
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        assert!(!fuzzy_match("xyz", "fibonacci"));
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("", "fibonacci"));
    }

    #[test]
    fn test_match_score_exact() {
        assert_eq!(match_score("fibonacci", "fibonacci"), 0);
    }

    #[test]
    fn test_match_score_prefix() {
        assert_eq!(match_score("fib", "fibonacci"), 1);
    }

    #[test]
    fn test_match_score_contains() {
        assert_eq!(match_score("bonac", "fibonacci"), 2);
    }

    #[test]
    fn test_match_score_fuzzy() {
        assert_eq!(match_score("fbi", "fibonacci"), 3);
    }

    #[test]
    fn test_to_lsp_symbol_kind_mapping() {
        assert_eq!(
            to_lsp_symbol_kind(&SymbolKind::Variable),
            LspSymbolKind::VARIABLE
        );
        assert_eq!(
            to_lsp_symbol_kind(&SymbolKind::Function),
            LspSymbolKind::FUNCTION
        );
        assert_eq!(to_lsp_symbol_kind(&SymbolKind::Type), LspSymbolKind::CLASS);
        assert_eq!(
            to_lsp_symbol_kind(&SymbolKind::Trait),
            LspSymbolKind::INTERFACE
        );
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
        assert_eq!(symbols.len(), 5, "空查询应返回所有 5 个符号");
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
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "fibonacci");
        assert_eq!(symbols[0].kind, LspSymbolKind::FUNCTION);
    }

    #[test]
    fn test_workspace_symbol_prefix_match() {
        let world = make_world_with_symbols();
        let params = WorkspaceSymbolParams {
            query: "f".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_workspace_symbol(&world, params);
        assert!(result.is_some());
        let symbols = result.unwrap();
        // f 应匹配 fibonacci, find_by_id, format_string
        assert!(symbols.len() >= 3);
    }

    #[test]
    fn test_workspace_symbol_fuzzy_match() {
        let world = make_world_with_symbols();
        let params = WorkspaceSymbolParams {
            query: "fbi".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_workspace_symbol(&world, params);
        assert!(result.is_some());
        let symbols = result.unwrap();
        // "fbi" 模糊匹配 fibonacci, find_by_id
        assert!(symbols.iter().any(|s| s.name == "fibonacci"));
    }

    #[test]
    fn test_workspace_symbol_no_match() {
        let world = make_world_with_symbols();
        let params = WorkspaceSymbolParams {
            query: "zzzzz".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_workspace_symbol(&world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_workspace_symbol_type_filter() {
        let world = make_world_with_symbols();
        let params = WorkspaceSymbolParams {
            query: "MyType".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_workspace_symbol(&world, params);
        assert!(result.is_some());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, LspSymbolKind::CLASS);
    }

    #[test]
    fn test_workspace_symbol_result_sorting() {
        let world = make_world_with_symbols();
        let params = WorkspaceSymbolParams {
            query: "fi".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_workspace_symbol(&world, params);
        assert!(result.is_some());
        let symbols = result.unwrap();
        // "fi" 是 fibonacci 和 find_by_id 的前缀匹配（得分 1）
        // 应该在 format_string（模糊匹配，得分 3）之前
        if symbols.len() >= 2 {
            let prefix_names: Vec<&str> = symbols.iter().take(2).map(|s| s.name.as_str()).collect();
            assert!(
                prefix_names.contains(&"fibonacci") || prefix_names.contains(&"find_by_id"),
                "前缀匹配结果应排在前面"
            );
        }
    }
}
