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

use crate::frontend::core::typecheck::semantic_db::SemanticDB;
use tracing::debug;

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
        self.results.retain(|_, v| v.file_uri != file_uri);

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
        self.results.get(result_id).map(|r| r.tokens.as_slice())
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
    document_text: Option<&str>,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = params.text_document.uri.as_str();

    let empty: &[crate::frontend::core::typecheck::semantic_db::SemanticToken] = &[];
    let db_tokens = semantic_db.get_tokens(uri).unwrap_or(empty);

    debug!(
        "semanticTokens full: uri={}, tokens_count={}",
        uri,
        db_tokens.len()
    );

    if db_tokens.is_empty() {
        let result_id = cache.store(uri, vec![]);
        return Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: Some(result_id),
            data: vec![],
        }));
    }

    let lsp_tokens = convert_to_lsp_tokens(db_tokens, document_text);
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
    document_text: Option<&str>,
    params: lsp_types::SemanticTokensDeltaParams,
) -> Option<SemanticTokensFullDeltaResult> {
    let uri = params.text_document.uri.as_str();
    let previous_result_id = &params.previous_result_id;

    // 获取当前最新 tokens
    let db_tokens = semantic_db.get_tokens(uri);
    let all_files = semantic_db.all_files();
    debug!(
        "semanticTokens delta: uri={}, db_tokens={:?}, all_files_in_db={:?}",
        uri,
        db_tokens.map(|t| t.len()),
        all_files
    );
    let new_tokens = match db_tokens {
        Some(tokens) if !tokens.is_empty() => convert_to_lsp_tokens(tokens, document_text),
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
fn convert_to_lsp_tokens_fallback(
    db_tokens: &[crate::frontend::core::typecheck::semantic_db::SemanticToken]
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
        // 使用 span 计算实际长度，而不是 token 名字长度
        let length = token
            .span
            .end
            .column
            .saturating_sub(token.span.start.column) as u32;

        let delta_line = line.saturating_sub(prev_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(prev_start)
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
fn compute_line_starts(document_text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, ch) in document_text.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn utf16_col_in_line(
    document_text: &str,
    line_start: usize,
    byte_offset: usize,
) -> Option<u32> {
    document_text
        .get(line_start..byte_offset)
        .map(|s| s.encode_utf16().count() as u32)
}

fn utf16_len_between_offsets(
    document_text: &str,
    start: usize,
    end: usize,
) -> Option<u32> {
    document_text
        .get(start..end)
        .map(|s| s.encode_utf16().count() as u32)
}

fn convert_to_lsp_tokens(
    db_tokens: &[crate::frontend::core::typecheck::semantic_db::SemanticToken],
    document_text: Option<&str>,
) -> Vec<SemanticToken> {
    match document_text {
        Some(text) => convert_to_lsp_tokens_utf16(db_tokens, text),
        None => convert_to_lsp_tokens_fallback(db_tokens),
    }
}

fn convert_to_lsp_tokens_utf16(
    db_tokens: &[crate::frontend::core::typecheck::semantic_db::SemanticToken],
    document_text: &str,
) -> Vec<SemanticToken> {
    if db_tokens.is_empty() || document_text.is_empty() {
        return vec![];
    }

    let line_starts = compute_line_starts(document_text);

    let mut computed = Vec::with_capacity(db_tokens.len());
    for token in db_tokens {
        let start = token.span.start.offset;
        let end = token.span.end.offset;

        if start >= end || start > document_text.len() {
            continue;
        }

        let safe_end = end.min(document_text.len());

        let line_index = line_starts
            .partition_point(|&s| s <= start)
            .saturating_sub(1);
        let line_start = line_starts[line_index];
        let line_end = line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(document_text.len());

        let clamped_end = safe_end.min(line_end);
        if clamped_end <= start {
            continue;
        }

        let start_utf16 = match utf16_col_in_line(document_text, line_start, start) {
            Some(v) => v,
            None => continue,
        };
        let length_utf16 = match utf16_len_between_offsets(document_text, start, clamped_end) {
            Some(v) if v > 0 => v,
            _ => continue,
        };

        let modifier_bits: u32 = token
            .modifiers
            .iter()
            .fold(0u32, |acc, m| acc | m.bit_flag());

        computed.push((
            line_index as u32,
            start_utf16,
            length_utf16,
            token.token_type.index(),
            modifier_bits,
        ));
    }

    computed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut lsp_tokens = Vec::with_capacity(computed.len());
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for (line, start, length, token_type, modifiers) in computed {
        let delta_line = line.saturating_sub(prev_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(prev_start)
        } else {
            start
        };

        lsp_tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: modifiers,
        });

        prev_line = line;
        prev_start = start;
    }

    lsp_tokens
}

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
