//! LSP 语义 tokens 处理器
//!
//! 实现 `textDocument/semanticTokens/full` 请求。
//! 将 SemanticDB 中的语义 token 转换为 LSP 协议要求的相对编码格式。

use lsp_types::{SemanticToken, SemanticTokens, SemanticTokensParams, SemanticTokensResult};

use crate::frontend::typecheck::semantic_db::SemanticDB;

/// 处理 `textDocument/semanticTokens/full` 请求
///
/// 从 SemanticDB 获取指定文件的所有语义 token，
/// 转换为 LSP 协议要求的 delta 编码格式后返回。
pub fn handle_semantic_tokens_full(
    semantic_db: &SemanticDB,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = params.text_document.uri.as_str();

    let db_tokens = semantic_db.get_tokens(uri)?;

    if db_tokens.is_empty() {
        return Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        }));
    }

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

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: lsp_tokens,
    }))
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
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///empty.yx").unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let result = handle_semantic_tokens_full(&db, params);
        // 文件不在 DB 中 → None
        assert!(result.is_none());
    }

    #[test]
    fn test_file_with_tokens() {
        let mut db = SemanticDB::new();
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

        let result = handle_semantic_tokens_full(&db, params).unwrap();
        if let SemanticTokensResult::Tokens(tokens) = result {
            assert_eq!(tokens.data.len(), 3);

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
}
