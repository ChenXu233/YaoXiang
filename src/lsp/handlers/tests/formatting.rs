//! LSP 格式化处理器测试
//!
//! 测试覆盖：
//! - 文档格式化
//! - 范围格式化

use lsp_types::{DocumentFormattingParams, DocumentRangeFormattingParams, TextEdit, Range, Position};
use lsp_types::{FormattingOptions, TextDocumentIdentifier, Uri, WorkDoneProgressParams};
use std::str::FromStr;

use crate::formatter::{format_source, FormatOptions};
use crate::lsp::handlers::formatting::{handle_formatting, handle_range_formatting};
use crate::lsp::session::Session;

fn test_session_with_doc(
    uri: &str,
    content: &str,
) -> Session {
    let mut session = Session::new();
    session
        .document_store_mut()
        .open(uri.to_string(), content.to_string(), 1);
    session
}

#[test]
fn test_handle_formatting() {
    let doc_uri = "file:///test.yx";
    let content = "fn main() {x=1}";
    let session = test_session_with_doc(doc_uri, content);

    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str(doc_uri).unwrap(),
        },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: Default::default(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    // Handle formatting - just verify it doesn't panic
    let _edits = handle_formatting(&session, params);
}

#[test]
fn test_handle_range_formatting() {
    let doc_uri = "file:///test.yx";
    let content = "fn main() {x=1}";
    let session = test_session_with_doc(doc_uri, content);

    let params = DocumentRangeFormattingParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str(doc_uri).unwrap(),
        },
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 15,
            },
        },
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: Default::default(),
            trim_trailing_whitespace: None,
            insert_final_newline: None,
            trim_final_newlines: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    // Handle range formatting - just verify it doesn't panic
    let _edits = handle_range_formatting(&session, params);
}
