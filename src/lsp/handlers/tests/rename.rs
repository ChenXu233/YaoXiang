//! LSP 符号重命名处理器测试
//!
//! 测试覆盖：
//! - 单文件重命名
//! - 非标识符位置
//! - 新名称保留

use lsp_types::{Range, TextEdit, Uri, WorkspaceEdit};
use std::collections::HashMap;

use crate::frontend::core::typecheck::semantic_db::{
    DefId, DefinitionInfo, DefinitionKind, ReferenceInfo,
};
use crate::lsp::handlers::rename::handle_rename;
use crate::lsp::session::Session;
use crate::lsp::world::World;
use crate::util::span::{Position, Span};

use lsp_types::{
    RenameParams, TextDocumentIdentifier, TextDocumentPositionParams,
    WorkDoneProgressParams,
};
use std::str::FromStr;

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

fn setup() -> (Session, World) {
    let mut session = Session::new();
    let mut world = World::new();

    let content = "x = 1\ny = x + x\n";
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        content.to_string(),
        1,
    );

    let uri = "file:///test/main.yx";
    let x_def_span = Span {
        start: Position {
            line: 1,
            column: 1,
            offset: 0,
        },
        end: Position {
            line: 1,
            column: 2,
            offset: 1,
        },
    };

    // x 的定义
    world.semantic_db_mut().add_definition(
        uri,
        DefinitionInfo {
            def_id: DefId {
                file_path: uri.to_string(),
                span: x_def_span,
            },
            name: "x".to_string(),
            kind: DefinitionKind::Variable,
            span: x_def_span,
            file_path: uri.to_string(),
            type_info: Some("Int".to_string()),
            signature: None,
        },
    );

    // x 的引用 1
    world.semantic_db_mut().add_reference(
        uri,
        ReferenceInfo {
            name: "x".to_string(),
            span: Span {
                start: Position {
                    line: 2,
                    column: 5,
                    offset: 10,
                },
                end: Position {
                    line: 2,
                    column: 6,
                    offset: 11,
                },
            },
            file_path: uri.to_string(),
            resolves_to: DefId {
                file_path: uri.to_string(),
                span: x_def_span,
            },
        },
    );

    // x 的引用 2
    world.semantic_db_mut().add_reference(
        uri,
        ReferenceInfo {
            name: "x".to_string(),
            span: Span {
                start: Position {
                    line: 2,
                    column: 9,
                    offset: 14,
                },
                end: Position {
                    line: 2,
                    column: 10,
                    offset: 15,
                },
            },
            file_path: uri.to_string(),
            resolves_to: DefId {
                file_path: uri.to_string(),
                span: x_def_span,
            },
        },
    );

    (session, world)
}

#[test]
#[allow(clippy::mutable_key_type)]
fn test_rename_single_file() {
    let (session, world) = setup();

    // 重命名 'x' 为 'new_x'（光标在定义位置）
    let params = make_rename_params("file:///test/main.yx", 0, 0, "new_x");
    let result = handle_rename(&session, &world, params);
    assert!(result.is_some());

    let changes = result.unwrap().changes;
    assert!(changes.is_some());

    let changes = changes.unwrap();
    assert_eq!(changes.len(), 1, "应修改 1 个文件");

    let edits = changes
        .get(&Uri::from_str("file:///test/main.yx").unwrap())
        .unwrap();
    // 定义(1) + 引用(2) = 3 个编辑
    assert_eq!(edits.len(), 3, "x 应出现 3 次（含定义）");
}

#[test]
fn test_rename_not_on_ident() {
    let (session, world) = setup();

    let params = make_rename_params("file:///test/main.yx", 0, 2, "new_x");
    let result = handle_rename(&session, &world, params);
    assert!(result.is_none());
}

#[test]
#[allow(clippy::mutable_key_type)]
fn test_rename_preserves_new_name() {
    let (session, world) = setup();

    let params = make_rename_params("file:///test/main.yx", 0, 0, "renamed_var");
    let result = handle_rename(&session, &world, params).unwrap();
    let changes = result.changes.unwrap();
    let edits = changes
        .get(&Uri::from_str("file:///test/main.yx").unwrap())
        .unwrap();
    for edit in edits {
        assert_eq!(edit.new_text, "renamed_var");
    }
}
