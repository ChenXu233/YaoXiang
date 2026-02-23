//! 文档同步处理
//!
//! 处理 `textDocument/didOpen`、`textDocument/didChange`、`textDocument/didClose`。
//!
//! **状态**：阶段 2 实现

use lsp_types::{DidOpenTextDocumentParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams};
use tracing::info;

use crate::lsp::session::Session;

/// 处理 `textDocument/didOpen`
pub fn handle_did_open(
    session: &mut Session,
    params: DidOpenTextDocumentParams,
) {
    let uri = params.text_document.uri.as_str().to_string();
    let version = params.text_document.version as u32;
    let content = params.text_document.text;

    info!("文档打开: {} (v{})", uri, version);

    session.document_store_mut().open(uri, content, version);
}

/// 处理 `textDocument/didChange`
pub fn handle_did_change(
    session: &mut Session,
    params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri.as_str().to_string();
    let version = params.text_document.version as u32;

    // Full sync 模式：取最后一个变更（即全量内容）
    if let Some(change) = params.content_changes.into_iter().last() {
        let changed = session
            .document_store_mut()
            .update(&uri, change.text, version);

        if changed {
            info!("文档更新: {} (v{})", uri, version);
        }
    }
}

/// 处理 `textDocument/didClose`
pub fn handle_did_close(
    session: &mut Session,
    params: DidCloseTextDocumentParams,
) {
    let uri = params.text_document.uri.as_str().to_string();
    info!("文档关闭: {}", uri);

    session.document_store_mut().close(&uri);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use lsp_types::{
        TextDocumentIdentifier, TextDocumentItem, VersionedTextDocumentIdentifier,
        TextDocumentContentChangeEvent, Uri,
    };

    fn test_uri(name: &str) -> Uri {
        Uri::from_str(&format!("file:///test/{}", name)).unwrap()
    }

    #[test]
    fn test_did_open() {
        let mut session = Session::new();
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: test_uri("main.yx"),
                language_id: "yaoxiang".to_string(),
                version: 1,
                text: "x = 42".to_string(),
            },
        };

        handle_did_open(&mut session, params);
        assert!(session.document_store().is_open("file:///test/main.yx"));
    }

    #[test]
    fn test_did_change() {
        let mut session = Session::new();

        // 先打开
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            "x = 42".to_string(),
            1,
        );

        // 然后变更
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: test_uri("main.yx"),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "x = 43".to_string(),
            }],
        };

        handle_did_change(&mut session, params);
        let doc = session
            .document_store()
            .get("file:///test/main.yx")
            .unwrap();
        assert_eq!(doc.content(), "x = 43");
        assert_eq!(doc.version(), 2);
    }

    #[test]
    fn test_did_close() {
        let mut session = Session::new();
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            "x = 42".to_string(),
            1,
        );

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: test_uri("main.yx"),
            },
        };

        handle_did_close(&mut session, params);
        assert!(!session.document_store().is_open("file:///test/main.yx"));
    }
}
