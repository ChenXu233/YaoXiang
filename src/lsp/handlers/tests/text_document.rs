//! 文档同步处理器测试
//!
//! 测试覆盖：
//! - 文档打开
//! - 文档变更
//! - 文档关闭
//! - 变更返回 URI

use lsp_types::{DidOpenTextDocumentParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams};

use crate::lsp::handlers::text_document::{handle_did_open, handle_did_change, handle_did_close};
use crate::lsp::session::Session;

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

    let uri = handle_did_open(&mut session, params);
    assert_eq!(uri, "file:///test/main.yx");
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
fn test_did_change_returns_uri() {
    let mut session = Session::new();
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42".to_string(),
        1,
    );

    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: test_uri("main.yx"),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "x = 99".to_string(),
        }],
    };

    let result = handle_did_change(&mut session, params);
    assert_eq!(result, Some("file:///test/main.yx".to_string()));
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

    let uri = handle_did_close(&mut session, params);
    assert_eq!(uri, "file:///test/main.yx");
    assert!(!session.document_store().is_open("file:///test/main.yx"));
}
