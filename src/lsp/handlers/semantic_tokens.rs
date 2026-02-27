//! LSP 语义 tokens 处理器
//!
//! 实现 `textDocument/semanticTokens/full` 和 `textDocument/semanticTokens/full/delta` 请求。
//! 将 SemanticDB 中的语义 token 转换为 LSP 协议要求的相对编码格式。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lsp_types::{
    SemanticToken, SemanticTokens, SemanticTokensDelta, SemanticTokensEdit,
    SemanticTokensFullDeltaResult, SemanticTokensParams, SemanticTokensResult,
};

use crate::frontend::typecheck::semantic_db::SemanticDB;

// ============ 版本缓存 ============

/// 语义 tokens 版本缓存
///
/// 跟踪每个文件上次返回的 tokens，用于增量 delta 计算。
#[derive(Debug, Default)]
pub struct SemanticTokensCache {
    /// result_id → (file_uri, cached tokens)
    results: HashMap<String, CachedResult>,
    /// 自增计数器，用于生成唯一 result_id
    next_id: AtomicU64,
}

#[derive(Debug, Clone)]
struct CachedResult {
    /// 文件 URI
    file_uri: String,
    /// 缓存的 LSP tokens
    tokens: Vec<SemanticToken>,
}

impl SemanticTokensCache {
    /// 创建新的缓存
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成新的 result_id 并缓存 tokens
    fn store(
        &mut self,
        file_uri: &str,
        tokens: Vec<SemanticToken>,
    ) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let result_id = format!("yx-{}", id);

        // 移除该文件的旧缓存
        self.results
            .retain(|_, v| v.file_uri != file_uri);

        self.results.insert(
            result_id.clone(),
            CachedResult {
                file_uri: file_uri.to_string(),
                tokens,
            },
        );

        result_id
    }

    /// 获取缓存的 tokens
    fn get(
        &self,
        result_id: &str,
    ) -> Option<&[SemanticToken]> {
        self.results
            .get(result_id)
            .map(|r| r.tokens.as_slice())
    }
}

// ============ Full 请求 ============

/// 处理 `textDocument/semanticTokens/full` 请求
///
/// 从 SemanticDB 获取指定文件的所有语义 token，
/// 转换为 LSP 协议要求的 delta 编码格式后返回。
/// 同时缓存结果以支持后续 delta 请求。
pub fn handle_semantic_tokens_full(
    semantic_db: &SemanticDB,
    cache: &mut SemanticTokensCache,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = params.text_document.uri.as_str();

    let db_tokens = semantic_db.get_tokens(uri)?;

    if db_tokens.is_empty() {
        let result_id = cache.store(uri, vec![]);
        return Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some(result_id),
            data: vec![],
        }));
    }

    let lsp_tokens = convert_to_lsp_tokens(db_tokens);
    let result_id = cache.store(uri, lsp_tokens.clone());

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: Some(result_id),
        data: lsp_tokens,
    }))
}

// ============ Delta 请求 ============

/// 处理 `textDocument/semanticTokens/full/delta` 请求
///
/// 对比上次缓存的 tokens 与当前最新状态，返回差异编辑。
/// 如果找不到上次缓存（result_id 失效），回退返回全量 tokens。
pub fn handle_semantic_tokens_full_delta(
    semantic_db: &SemanticDB,
    cache: &mut SemanticTokensCache,
    params: lsp_types::SemanticTokensDeltaParams,
) -> Option<SemanticTokensFullDeltaResult> {
    let uri = params.text_document.uri.as_str();
    let previous_result_id = &params.previous_result_id;

    // 获取当前最新 tokens
    let db_tokens = semantic_db.get_tokens(uri);
    let new_tokens = match db_tokens {
        Some(tokens) if !tokens.is_empty() => convert_to_lsp_tokens(tokens),
        _ => vec![],
    };

    // 尝试获取缓存的旧 tokens
    let old_tokens = cache.get(previous_result_id);

    match old_tokens {
        Some(old) => {
            // 计算 delta
            let edits = diff_semantic_tokens(old, &new_tokens);
            let result_id = cache.store(uri, new_tokens);

            Some(SemanticTokensFullDeltaResult::TokensDelta(
                SemanticTokensDelta {
                    result_id: Some(result_id),
                    edits,
                },
            ))
        }
        None => {
            // 缓存失效 → 回退全量返回
            let result_id = cache.store(uri, new_tokens.clone());
            Some(SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
                result_id: Some(result_id),
                data: new_tokens,
            }))
        }
    }
}

// ============ 内部工具函数 ============

/// 将 SemanticDB tokens 转换为 LSP delta 编码格式
fn convert_to_lsp_tokens(
    db_tokens: &[crate::frontend::typecheck::semantic_db::SemanticToken],
) -> Vec<SemanticToken> {
    // 按行、列排序
    let mut sorted: Vec<_> = db_tokens.iter().collect();
    sorted.sort_by(|a, b| {
        let a_line = a.span.start.line;
        let a_col = a.span.start.column;
        let b_line = b.span.start.line;
        let b_col = b.span.start.column;
        a_line.cmp(&b_line).then(a_col.cmp(&b_col))
    });

    // 转换为 LSP delta 编码
    let mut lsp_tokens = Vec::with_capacity(sorted.len());
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for token in &sorted {
        // SemanticDB 中的 line 是 1-indexed，LSP 是 0-indexed
        let line = token.span.start.line.saturating_sub(1) as u32;
        let start = token.span.start.column.saturating_sub(1) as u32;
        let length = token.name.len() as u32;

        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            start - prev_start
        } else {
            start
        };

        // 计算修饰符 bit mask
        let modifier_bits: u32 = token
            .modifiers
            .iter()
            .fold(0u32, |acc, m| acc | m.bit_flag());

        lsp_tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: token.token_type.index(),
            token_modifiers_bitset: modifier_bits,
        });

        prev_line = line;
        prev_start = start;
    }

    lsp_tokens
}

/// 比较两个 SemanticToken 是否相等
fn tokens_eq(
    a: &SemanticToken,
    b: &SemanticToken,
) -> bool {
    a.delta_line == b.delta_line
        && a.delta_start == b.delta_start
        && a.length == b.length
        && a.token_type == b.token_type
        && a.token_modifiers_bitset == b.token_modifiers_bitset
}

/// 计算两组 tokens 的差异，生成 LSP SemanticTokensEdit
///
/// 使用前后缀匹配策略：找到第一个和最后一个不同的位置，
/// 中间部分作为一个 edit 返回。
fn diff_semantic_tokens(
    old: &[SemanticToken],
    new: &[SemanticToken],
) -> Vec<SemanticTokensEdit> {
    // 找到第一个不同的 token 位置
    let prefix = old
        .iter()
        .zip(new.iter())
        .take_while(|(a, b)| tokens_eq(a, b))
        .count();

    // 如果完全相同
    if prefix == old.len() && prefix == new.len() {
        return vec![];
    }

    // 从尾部找到最后一个不同的位置
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take_while(|(a, b)| tokens_eq(a, b))
        .count()
        .min(old.len() - prefix)
        .min(new.len() - prefix);

    let old_mid = old.len() - prefix - suffix;
    let new_mid_end = new.len() - suffix;

    // LSP SemanticTokensEdit 的 start 和 delete_count 基于 flat u32 数组
    // 每个 token = 5 个 u32 值
    vec![SemanticTokensEdit {
        start: (prefix * 5) as u32,
        delete_count: (old_mid * 5) as u32,
        data: if prefix < new_mid_end {
            Some(new[prefix..new_mid_end].to_vec())
        } else {
            None
        },
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::typecheck::semantic_db::{
        SemanticDB, SemanticToken as DbToken, SemanticTokenModifier, SemanticTokenType,
    };
    use crate::util::span::{Position, Span};
    use std::str::FromStr;

    fn make_db_token(
        name: &str,
        token_type: SemanticTokenType,
        modifiers: Vec<SemanticTokenModifier>,
        line: usize,
        col: usize,
    ) -> DbToken {
        DbToken {
            name: name.to_string(),
            token_type,
            modifiers,
            span: Span {
                start: Position {
                    line,
                    column: col,
                    offset: 0,
                },
                end: Position {
                    line,
                    column: col + name.len(),
                    offset: 0,
                },
            },
        }
    }

    #[test]
    fn test_empty_file() {
        let db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///empty.yx").unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_semantic_tokens_full(&db, &mut cache, params);
        // 文件不在 DB 中 → None
        assert!(result.is_none());
    }

    #[test]
    fn test_file_with_tokens() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///test.yx";

        // 添加一些 tokens: line/col 1-indexed
        db.add_token(
            uri,
            make_db_token(
                "add",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );
        db.add_token(
            uri,
            make_db_token("x", SemanticTokenType::Parameter, vec![], 1, 7),
        );
        db.add_token(
            uri,
            make_db_token(
                "y",
                SemanticTokenType::Variable,
                vec![SemanticTokenModifier::Readonly],
                2,
                1,
            ),
        );

        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_semantic_tokens_full(&db, &mut cache, params).unwrap();
        if let SemanticTokensResult::Tokens(tokens) = result {
            assert_eq!(tokens.data.len(), 3);
            assert!(tokens.result_id.is_some(), "应返回 result_id");

            // 第一个 token: "add" at (0,0) - delta line=0, delta start=0
            assert_eq!(tokens.data[0].delta_line, 0);
            assert_eq!(tokens.data[0].delta_start, 0);
            assert_eq!(tokens.data[0].length, 3);
            assert_eq!(
                tokens.data[0].token_type,
                SemanticTokenType::Function.index()
            );
            assert_eq!(
                tokens.data[0].token_modifiers_bitset,
                SemanticTokenModifier::Declaration.bit_flag()
            );

            // 第二个 token: "x" at (0,6) - same line, delta start=6
            assert_eq!(tokens.data[1].delta_line, 0);
            assert_eq!(tokens.data[1].delta_start, 6);
            assert_eq!(tokens.data[1].length, 1);
            assert_eq!(
                tokens.data[1].token_type,
                SemanticTokenType::Parameter.index()
            );
            assert_eq!(tokens.data[1].token_modifiers_bitset, 0);

            // 第三个 token: "y" at (1,0) - new line
            assert_eq!(tokens.data[2].delta_line, 1);
            assert_eq!(tokens.data[2].delta_start, 0);
            assert_eq!(tokens.data[2].length, 1);
            assert_eq!(
                tokens.data[2].token_type,
                SemanticTokenType::Variable.index()
            );
            assert_eq!(
                tokens.data[2].token_modifiers_bitset,
                SemanticTokenModifier::Readonly.bit_flag()
            );
        } else {
            panic!("Expected Tokens variant");
        }
    }

    // ============ Delta 测试 ============

    #[test]
    fn test_delta_no_change() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///delta_test.yx";

        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // 先执行一次 full 请求获取 result_id
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let full_result = handle_semantic_tokens_full(&db, &mut cache, params).unwrap();
        let result_id = match &full_result {
            SemanticTokensResult::Tokens(t) => t.result_id.clone().unwrap(),
            _ => panic!("Expected Tokens"),
        };

        // 无变更，请求 delta
        let delta_params = lsp_types::SemanticTokensDeltaParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            previous_result_id: result_id,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let delta_result =
            handle_semantic_tokens_full_delta(&db, &mut cache, delta_params).unwrap();

        match delta_result {
            SemanticTokensFullDeltaResult::TokensDelta(delta) => {
                assert!(delta.edits.is_empty(), "无变更时 delta 应为空");
                assert!(delta.result_id.is_some());
            }
            _ => panic!("Expected TokensDelta variant"),
        }
    }

    #[test]
    fn test_delta_add_token() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///delta_add.yx";

        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // Full 请求
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let full_result = handle_semantic_tokens_full(&db, &mut cache, params).unwrap();
        let result_id = match &full_result {
            SemanticTokensResult::Tokens(t) => t.result_id.clone().unwrap(),
            _ => panic!("Expected Tokens"),
        };

        // 添加新 token
        db.add_token(
            uri,
            make_db_token("bar", SemanticTokenType::Variable, vec![], 2, 1),
        );

        // Delta 请求
        let delta_params = lsp_types::SemanticTokensDeltaParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            previous_result_id: result_id,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let delta_result =
            handle_semantic_tokens_full_delta(&db, &mut cache, delta_params).unwrap();

        match delta_result {
            SemanticTokensFullDeltaResult::TokensDelta(delta) => {
                assert!(!delta.edits.is_empty(), "添加 token 后应有 edit");
            }
            _ => panic!("Expected TokensDelta variant"),
        }
    }

    #[test]
    fn test_delta_delete_token() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///delta_del.yx";

        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );
        db.add_token(
            uri,
            make_db_token("bar", SemanticTokenType::Variable, vec![], 2, 1),
        );

        // Full 请求
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let full_result = handle_semantic_tokens_full(&db, &mut cache, params).unwrap();
        let result_id = match &full_result {
            SemanticTokensResult::Tokens(t) => t.result_id.clone().unwrap(),
            _ => panic!("Expected Tokens"),
        };

        // 删除文件并重新建立，只保留第一个 token
        db.remove_file(uri);
        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // Delta 请求
        let delta_params = lsp_types::SemanticTokensDeltaParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            previous_result_id: result_id,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let delta_result =
            handle_semantic_tokens_full_delta(&db, &mut cache, delta_params).unwrap();

        match delta_result {
            SemanticTokensFullDeltaResult::TokensDelta(delta) => {
                assert!(!delta.edits.is_empty(), "删除 token 后应有 edit");
                // 删除了 1 个 token = 5 个 u32 值
                let total_deleted: u32 = delta.edits.iter().map(|e| e.delete_count).sum();
                assert!(total_deleted > 0, "应有删除操作");
            }
            _ => panic!("Expected TokensDelta variant"),
        }
    }

    #[test]
    fn test_delta_modify_token() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///delta_mod.yx";

        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // Full 请求
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let full_result = handle_semantic_tokens_full(&db, &mut cache, params).unwrap();
        let result_id = match &full_result {
            SemanticTokensResult::Tokens(t) => t.result_id.clone().unwrap(),
            _ => panic!("Expected Tokens"),
        };

        // 修改 token（改名为 "foobar"，行数不变）
        db.remove_file(uri);
        db.add_token(
            uri,
            make_db_token(
                "foobar",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // Delta 请求
        let delta_params = lsp_types::SemanticTokensDeltaParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            previous_result_id: result_id,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let delta_result =
            handle_semantic_tokens_full_delta(&db, &mut cache, delta_params).unwrap();

        match delta_result {
            SemanticTokensFullDeltaResult::TokensDelta(delta) => {
                assert!(!delta.edits.is_empty(), "修改 token 后应有 edit");
            }
            _ => panic!("Expected TokensDelta variant"),
        }
    }

    #[test]
    fn test_delta_fallback_on_invalid_result_id() {
        let mut db = SemanticDB::new();
        let mut cache = SemanticTokensCache::new();
        let uri = "file:///delta_fallback.yx";

        db.add_token(
            uri,
            make_db_token(
                "foo",
                SemanticTokenType::Function,
                vec![SemanticTokenModifier::Declaration],
                1,
                1,
            ),
        );

        // 使用无效 result_id 请求 delta
        let delta_params = lsp_types::SemanticTokensDeltaParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str(uri).unwrap(),
            },
            previous_result_id: "invalid-id".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let delta_result =
            handle_semantic_tokens_full_delta(&db, &mut cache, delta_params).unwrap();

        match delta_result {
            SemanticTokensFullDeltaResult::Tokens(tokens) => {
                assert!(!tokens.data.is_empty(), "回退应返回全量 tokens");
                assert!(tokens.result_id.is_some());
            }
            _ => panic!("无效 result_id 时应回退返回全量 Tokens"),
        }
    }

    // ============ Diff 算法测试 ============

    #[test]
    fn test_diff_identical() {
        let tokens = vec![SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 3,
            token_type: 0,
            token_modifiers_bitset: 0,
        }];
        let edits = diff_semantic_tokens(&tokens, &tokens);
        assert!(edits.is_empty());
    }

    #[test]
    fn test_diff_append() {
        let old = vec![SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 3,
            token_type: 0,
            token_modifiers_bitset: 0,
        }];
        let new = vec![
            SemanticToken {
                delta_line: 0,
                delta_start: 0,
                length: 3,
                token_type: 0,
                token_modifiers_bitset: 0,
            },
            SemanticToken {
                delta_line: 1,
                delta_start: 0,
                length: 3,
                token_type: 2,
                token_modifiers_bitset: 0,
            },
        ];
        let edits = diff_semantic_tokens(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 5); // after 1 token * 5
        assert_eq!(edits[0].delete_count, 0);
        assert!(edits[0].data.is_some());
        assert_eq!(edits[0].data.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_diff_prepend() {
        let old = vec![SemanticToken {
            delta_line: 1,
            delta_start: 0,
            length: 3,
            token_type: 2,
            token_modifiers_bitset: 0,
        }];
        let new = vec![
            SemanticToken {
                delta_line: 0,
                delta_start: 0,
                length: 3,
                token_type: 0,
                token_modifiers_bitset: 0,
            },
            SemanticToken {
                delta_line: 1,
                delta_start: 0,
                length: 3,
                token_type: 2,
                token_modifiers_bitset: 0,
            },
        ];
        let edits = diff_semantic_tokens(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 0);
    }

    #[test]
    fn test_diff_empty_to_tokens() {
        let old: Vec<SemanticToken> = vec![];
        let new = vec![SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 3,
            token_type: 0,
            token_modifiers_bitset: 0,
        }];
        let edits = diff_semantic_tokens(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 0);
        assert_eq!(edits[0].delete_count, 0);
        assert!(edits[0].data.is_some());
    }

    #[test]
    fn test_diff_tokens_to_empty() {
        let old = vec![SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 3,
            token_type: 0,
            token_modifiers_bitset: 0,
        }];
        let new: Vec<SemanticToken> = vec![];
        let edits = diff_semantic_tokens(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].start, 0);
        assert_eq!(edits[0].delete_count, 5); // 1 token * 5
    }

    // ============ Cache 测试 ============

    #[test]
    fn test_cache_store_and_get() {
        let mut cache = SemanticTokensCache::new();
        let tokens = vec![SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: 3,
            token_type: 0,
            token_modifiers_bitset: 0,
        }];

        let id = cache.store("file:///test.yx", tokens.clone());
        let cached = cache.get(&id);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_replaces_old_file_entry() {
        let mut cache = SemanticTokensCache::new();
        let id1 = cache.store("file:///test.yx", vec![]);
        let id2 = cache.store("file:///test.yx", vec![]);

        // 旧 ID 应失效
        assert!(cache.get(&id1).is_none());
        assert!(cache.get(&id2).is_some());
    }
}
