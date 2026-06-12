//! LSP 代码操作处理器测试
//!
//! 测试覆盖：
//! - 无选区时的代码操作
//! - 有选区时的代码操作
//! - 诊断相关的快速修复

use lsp_types::{CodeAction, CodeActionKind, CodeActionParams, Range};
use lsp_types::{CodeActionContext, Position, TextDocumentIdentifier, Uri};
use std::str::FromStr;

use crate::lsp::handlers::code_action::handle_code_action;

fn make_params(uri: &str) -> CodeActionParams {
    CodeActionParams {
        text_document: TextDocumentIdentifier {
            uri: Uri::from_str(uri).unwrap(),
        },
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        context: CodeActionContext {
            diagnostics: vec![],
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    }
}

fn make_params_with_range(
    uri: &str,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
) -> CodeActionParams {
    let mut params = make_params(uri);
    params.range = Range {
        start: Position {
            line: start_line,
            character: start_char,
        },
        end: Position {
            line: end_line,
            character: end_char,
        },
    };
    params
}

#[test]
fn test_code_action_no_selection() {
    let params = make_params("file:///test.yx");
    let result = handle_code_action(params, "x = 1");
    assert!(result.is_some());

    let actions = result.unwrap();
    // At least inline variable
    assert!(actions.iter().any(|a| a.title == "内联变量"));
}

#[test]
fn test_code_action_with_selection() {
    let params = make_params_with_range("file:///test.yx", 0, 0, 0, 5);
    let result = handle_code_action(params, "x = 1 + 2");
    assert!(result.is_some());

    let actions = result.unwrap();
    assert!(actions.iter().any(|a| a.title == "提取为变量"));
    assert!(actions.iter().any(|a| a.title == "内联变量"));
}
