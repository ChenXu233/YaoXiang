//! LSP 语义 tokens 处理器测试
//!
//! 测试覆盖：
//! - 空文件处理
//! - 有 tokens 的文件
//! - UTF-16 偏移量
//! - Delta 无变更
//! - Delta 添加 token
//! - Delta 删除 token
//! - Delta 修改 token
//! - Delta 回退机制
//! - Diff 算法
//! - 缓存管理

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use lsp_types::{
    SemanticToken, SemanticTokens, SemanticTokensDelta, SemanticTokensEdit,
    SemanticTokensFullDeltaResult, SemanticTokensParams, SemanticTokensResult,
};

use crate::frontend::core::typecheck::semantic_db::SemanticDB;
use crate::lsp::handlers::semantic_tokens::{
    handle_semantic_tokens_full, handle_semantic_tokens_full_delta, SemanticTokensCache,
    diff_semantic_tokens,
};

use crate::frontend::core::typecheck::semantic_db::{
    SemanticToken as DbToken, SemanticTokenModifier, SemanticTokenType,
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

    let result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
    // 文件不在 DB 中 → None
    if let SemanticTokensResult::Tokens(tokens) = result {
        assert!(tokens.data.is_empty());
        assert!(tokens.result_id.is_some());
    } else {
        panic!("Expected SemanticTokensResult::Tokens");
    }
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

    let result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
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
fn test_utf16_offsets_for_semantic_tokens() {
    let mut db = SemanticDB::new();
    let mut cache = SemanticTokensCache::new();
    let uri = "file:///utf16.yx";
    let document_text = "变量 = 1\n";

    db.add_token(
        uri,
        DbToken {
            name: "变量".to_string(),
            token_type: SemanticTokenType::Variable,
            modifiers: vec![SemanticTokenModifier::Declaration],
            span: Span {
                start: Position {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
                end: Position {
                    line: 1,
                    column: 3,
                    offset: 6,
                },
            },
        },
    );

    db.add_token(
        uri,
        DbToken {
            name: "1".to_string(),
            token_type: SemanticTokenType::Number,
            modifiers: vec![],
            span: Span {
                start: Position {
                    line: 1,
                    column: 10,
                    offset: 9,
                },
                end: Position {
                    line: 1,
                    column: 11,
                    offset: 10,
                },
            },
        },
    );

    let params = SemanticTokensParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: lsp_types::Uri::from_str(uri).unwrap(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result =
        handle_semantic_tokens_full(&db, &mut cache, Some(document_text), params).unwrap();

    let SemanticTokensResult::Tokens(tokens) = result else {
        panic!("Expected Tokens variant");
    };

    assert_eq!(tokens.data.len(), 2);

    assert_eq!(tokens.data[0].delta_line, 0);
    assert_eq!(tokens.data[0].delta_start, 0);
    assert_eq!(tokens.data[0].length, 2);

    assert_eq!(tokens.data[1].delta_line, 0);
    assert_eq!(tokens.data[1].delta_start, 5);
    assert_eq!(tokens.data[1].length, 1);
}

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
    let full_result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
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
        handle_semantic_tokens_full_delta(&db, &mut cache, None, delta_params).unwrap();

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
    let full_result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
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
        handle_semantic_tokens_full_delta(&db, &mut cache, None, delta_params).unwrap();

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
    let full_result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
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
        handle_semantic_tokens_full_delta(&db, &mut cache, None, delta_params).unwrap();

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
    let full_result = handle_semantic_tokens_full(&db, &mut cache, None, params).unwrap();
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
        handle_semantic_tokens_full_delta(&db, &mut cache, None, delta_params).unwrap();

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
        handle_semantic_tokens_full_delta(&db, &mut cache, None, delta_params).unwrap();

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
