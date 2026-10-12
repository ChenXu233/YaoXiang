//! LSP 符号重命名处理器测试
//!
//! 语义数据由真实 typecheck 管线产出（见 support 模块），遵守静默通道判定
//! （RFC-039 D55 / coding-rules 第六部分）：禁止手工构造 definitions/references。
//!
//! 测试覆盖：
//! - 单文件重命名覆盖定义与全部使用点
//! - 非标识符位置
//! - 新名称保留

use lsp_types::Uri;
use std::str::FromStr;

use super::support::{SAMPLE_URI, open_and_check};
use crate::lsp::handlers::rename::handle_rename;
use crate::lsp::session::Session;
use crate::lsp::world::World;

use lsp_types::{
    RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};

fn make_rename_params(
    uri: &str,
    line: u32,
    character: u32,
    new_name: &str,
) -> RenameParams {
    RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        new_name: new_name.to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    }
}

/// 样例：第二行 `y = x + x` 有两个 x 引用点，定义在第一行
fn setup() -> (Session, World) {
    open_and_check("x = 1\ny = x + x\n")
}

#[test]
fn test_rename_rewrites_definition_and_all_usages() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在第二行 x 引用上重命名
    let params = make_rename_params(SAMPLE_URI, 1, 4, "renamed");
    let result = handle_rename(&session, &world, params);
    // Assert：同一文件的 3 处编辑（定义 + 两个使用点），新名一致
    let edit = result.expect("重命名应返回 WorkspaceEdit");
    let changes = edit.changes.expect("应包含 changes 表");
    assert_eq!(changes.len(), 1, "单文件样例只应有一个文件被改");
    let edits = changes
        .get(&Uri::from_str(SAMPLE_URI).unwrap())
        .expect("编辑应落在样例文档上");
    assert_eq!(edits.len(), 3, "定义加两个使用点共 3 处编辑");
    assert!(
        edits.iter().all(|e| e.new_text == "renamed"),
        "所有编辑的新名都应为 renamed"
    );
}

#[test]
fn test_rename_not_on_ident_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在 '=' 号上（0-indexed line=0, character=2）
    let params = make_rename_params(SAMPLE_URI, 0, 2, "renamed");
    let result = handle_rename(&session, &world, params);
    // Assert
    assert!(result.is_none(), "非标识符位置应返回 None");
}
