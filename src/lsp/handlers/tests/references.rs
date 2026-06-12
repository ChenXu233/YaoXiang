//! 查找引用处理器测试
//!
//! 测试覆盖：
//! - 排除声明的引用查找
//! - 包含声明的引用查找
//! - 非标识符位置
//! - 未打开文档

use lsp_types::{Location, ReferenceParams, Uri};
use std::str::FromStr;

use crate::frontend::core::typecheck::semantic_db::{
    DefId, DefinitionInfo, DefinitionKind, ReferenceInfo,
};
use crate::lsp::handlers::references::handle_references;
use crate::lsp::session::Session;
use crate::lsp::world::World;
use crate::util::span::{Position, Span};

use lsp_types::{
    PartialResultParams, ReferenceContext, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkDoneProgressParams,
};

fn make_params(
    uri: &str,
    line: u32,
    character: u32,
    include_declaration: bool,
) -> ReferenceParams {
    ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration,
        },
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

    // x 的引用 1（第二行 y = x...）
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

    // x 的引用 2（第二行 + x）
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
fn test_references_excluding_declaration() {
    let (session, world) = setup();

    // 光标在第一行的 'x' 定义上
    let params = make_params("file:///test/main.yx", 0, 0, false);
    let result = handle_references(&session, &world, params);
    assert!(result.is_some());

    let locs = result.unwrap();
    // 应该只有 2 个引用（不含定义）
    assert_eq!(locs.len(), 2, "x 应有 2 个引用（不含定义）");
}

#[test]
fn test_references_include_declaration() {
    let (session, world) = setup();

    let params = make_params("file:///test/main.yx", 0, 0, true);
    let result = handle_references(&session, &world, params);
    assert!(result.is_some());

    let locs = result.unwrap();
    // include_declaration=true: 定义(1) + 引用(2) = 3
    assert_eq!(locs.len(), 3, "应有 3 个位置（含定义）");
}

#[test]
fn test_references_not_on_ident() {
    let (session, world) = setup();

    let params = make_params("file:///test/main.yx", 0, 2, false);
    let result = handle_references(&session, &world, params);
    assert!(result.is_none());
}

#[test]
fn test_references_doc_not_open() {
    let session = Session::new();
    let world = World::new();

    let params = make_params("file:///test/nope.yx", 0, 0, false);
    let result = handle_references(&session, &world, params);
    assert!(result.is_none());
}
