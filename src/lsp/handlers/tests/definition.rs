//! 跳转定义处理器测试
//!
//! 测试覆盖：
//! - 定义查找成功
//! - 定义查找失败（无符号）
//! - 非标识符位置
//! - 未打开的文档

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Uri};
use std::str::FromStr;

use crate::frontend::core::typecheck::semantic_db::{
    DefId, DefinitionInfo, DefinitionKind, ReferenceInfo,
};
use crate::lsp::handlers::definition::handle_definition;
use crate::lsp::session::Session;
use crate::lsp::world::World;
use crate::util::span::{Position, Span};

use lsp_types::{
    PartialResultParams, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkDoneProgressParams,
};

fn make_params(
    uri: &str,
    line: u32,
    character: u32,
) -> GotoDefinitionParams {
    GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    }
}

fn setup_session_and_world() -> (Session, World) {
    let mut session = Session::new();
    let mut world = World::new();

    // 打开一个文档并创建语义信息
    let content = "x = 42\nadd = (a, b) => a + b\n";
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        content.to_string(),
        1,
    );

    let uri = "file:///test/main.yx";

    // 添加 x 的定义
    let x_def = DefinitionInfo {
        def_id: DefId {
            file_path: uri.to_string(),
            span: Span {
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
            },
        },
        name: "x".to_string(),
        kind: DefinitionKind::Variable,
        span: Span {
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
        },
        file_path: uri.to_string(),
        type_info: Some("Int".to_string()),
        signature: None,
    };
    world.semantic_db_mut().add_definition(uri, x_def);

    // 添加 x 的引用（出现在第二行的表达式中）
    world.semantic_db_mut().add_reference(
        uri,
        ReferenceInfo {
            name: "x".to_string(),
            span: Span {
                start: Position {
                    line: 2,
                    column: 1,
                    offset: 7,
                },
                end: Position {
                    line: 2,
                    column: 2,
                    offset: 8,
                },
            },
            file_path: uri.to_string(),
            resolves_to: DefId {
                file_path: uri.to_string(),
                span: Span {
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
                },
            },
        },
    );

    // 添加 add 的定义
    let add_def = DefinitionInfo {
        def_id: DefId {
            file_path: uri.to_string(),
            span: Span {
                start: Position {
                    line: 2,
                    column: 1,
                    offset: 7,
                },
                end: Position {
                    line: 2,
                    column: 4,
                    offset: 10,
                },
            },
        },
        name: "add".to_string(),
        kind: DefinitionKind::Function,
        span: Span {
            start: Position {
                line: 2,
                column: 1,
                offset: 7,
            },
            end: Position {
                line: 2,
                column: 4,
                offset: 10,
            },
        },
        file_path: uri.to_string(),
        type_info: Some("(Int, Int) -> Int".to_string()),
        signature: Some("add: (a: Int, b: Int) -> Int".to_string()),
    };
    world.semantic_db_mut().add_definition(uri, add_def);

    (session, world)
}

#[test]
fn test_definition_found() {
    let (session, world) = setup_session_and_world();

    // 光标在第二行的 'x' 引用上（1-indexed: line=2, col=1-2）
    // 0-indexed: line=1, character=0
    let params = make_params("file:///test/main.yx", 1, 0);
    let result = handle_definition(&session, &world, params);
    assert!(result.is_some(), "应找到 x 的定义");

    match result.unwrap() {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri.to_string(), "file:///test/main.yx");
            // 定义在第一行 (1-indexed line=1 → 0-indexed line=0)
            assert_eq!(loc.range.start.line, 0);
        }
        _ => panic!("单个定义应返回 Scalar"),
    }
}

#[test]
fn test_definition_not_found_no_symbol() {
    let (session, world) = setup_session_and_world();

    // 在第二行找 'a' (参数，未注册引用)
    let params = make_params("file:///test/main.yx", 1, 7);
    let result = handle_definition(&session, &world, params);
    assert!(result.is_none());
}

#[test]
fn test_definition_not_on_identifier() {
    let (session, world) = setup_session_and_world();

    // 光标在 '=' 号上
    let params = make_params("file:///test/main.yx", 0, 2);
    let result = handle_definition(&session, &world, params);
    assert!(result.is_none(), "非标识符位置应返回 None");
}

#[test]
fn test_definition_doc_not_open() {
    let (session, world) = setup_session_and_world();

    // 请求未打开的文档
    let params = make_params("file:///test/other.yx", 0, 0);
    let result = handle_definition(&session, &world, params);
    assert!(result.is_none(), "未打开的文档应返回 None");
}
