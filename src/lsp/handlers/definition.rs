//! 跳转定义处理
//!
//! **状态**：阶段 4 (v0.8) 实现
//!
//! 支持：
//! - 函数定义跳转
//! - 变量定义跳转
//! - 类型定义跳转
//! - 跨文件跳转（基于全局符号索引）

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Uri};
use std::str::FromStr;
use tracing::debug;

use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/definition` 请求
///
/// 查找光标位置处标识符的定义位置：
/// 1. 解析光标处的标识符
/// 2. 在全局符号索引中查找定义位置
/// 3. 返回定义位置列表
pub fn handle_definition(
    session: &Session,
    world: &World,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri_str = params
        .text_document_position_params
        .text_document
        .uri
        .to_string();
    let position = &params.text_document_position_params.position;

    debug!(
        "跳转定义请求: {} @ {}:{}",
        uri_str, position.line, position.character
    );

    // 获取文档内容
    let doc = session.document_store().get(&uri_str)?;
    let content = doc.content();

    // 查找光标处的标识符
    let ident = find_identifier_at_position(content, position)?;
    debug!("光标处标识符: {}", ident.name);

    // 在全局符号索引中查找定义
    let symbols = world.symbol_index().find_by_name(&ident.name);

    if symbols.is_empty() {
        debug!("未找到符号定义: {}", ident.name);
        return None;
    }

    // 转换为 LSP Location 列表
    let locations: Vec<Location> = symbols
        .iter()
        .filter_map(|sym| {
            let uri = Uri::from_str(&sym.location.file_path).ok()?;
            let range = span_to_range(&sym.location.span);
            Some(Location::new(uri, range))
        })
        .collect();

    if locations.is_empty() {
        return None;
    }

    debug!("找到 {} 个定义位置", locations.len());

    if locations.len() == 1 {
        Some(GotoDefinitionResponse::Scalar(
            locations.into_iter().next().unwrap(),
        ))
    } else {
        Some(GotoDefinitionResponse::Array(locations))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
    use crate::lsp::session::Session;
    use crate::lsp::world::World;
    use crate::util::span::{Position, Span};
    use lsp_types::{
        GotoDefinitionParams, PartialResultParams, TextDocumentIdentifier,
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

        // 打开一个文档并创建符号索引
        let content = "x = 42\nadd = (a, b) => a + b\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        let module = Module {
            items: vec![
                Stmt {
                    kind: StmtKind::Var {
                        name: "x".to_string(),
                        type_annotation: None,
                        initializer: None,
                        is_mut: false,
                    },
                    span: Span {
                        start: Position {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        end: Position {
                            line: 1,
                            column: 7,
                            offset: 6,
                        },
                    },
                },
                Stmt {
                    kind: StmtKind::Fn {
                        name: "add".to_string(),
                        generic_params: vec![],
                        type_annotation: None,
                        params: vec![],
                        body: (vec![], None),
                        is_pub: false,
                    },
                    span: Span {
                        start: Position {
                            line: 2,
                            column: 1,
                            offset: 7,
                        },
                        end: Position {
                            line: 2,
                            column: 28,
                            offset: 34,
                        },
                    },
                },
            ],
            span: Span::dummy(),
        };

        world.update_index_from_ast("file:///test/main.yx", &module);

        (session, world)
    }

    #[test]
    fn test_definition_found() {
        let (session, world) = setup_session_and_world();

        // 光标在第一行的 'x' 上
        let params = make_params("file:///test/main.yx", 0, 0);
        let result = handle_definition(&session, &world, params);
        assert!(result.is_some(), "应找到 x 的定义");

        match result.unwrap() {
            GotoDefinitionResponse::Scalar(loc) => {
                assert_eq!(loc.uri.to_string(), "file:///test/main.yx");
                assert_eq!(loc.range.start.line, 0); // 0-indexed
            }
            _ => panic!("单个定义应返回 Scalar"),
        }
    }

    #[test]
    fn test_definition_not_found_no_symbol() {
        let (session, world) = setup_session_and_world();

        // 在第二行找 'a' (参数，不在顶层符号索引中)
        let params = make_params("file:///test/main.yx", 1, 7);
        let result = handle_definition(&session, &world, params);
        // 'a' 不在顶层符号索引中，返回 None
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

    #[test]
    fn test_definition_cross_file() {
        let mut session = Session::new();
        let mut world = World::new();

        // 文件 A 定义了 foo
        let module_a = Module {
            items: vec![Stmt {
                kind: StmtKind::Fn {
                    name: "foo".to_string(),
                    generic_params: vec![],
                    type_annotation: None,
                    params: vec![],
                    body: (vec![], None),
                    is_pub: false,
                },
                span: Span {
                    start: Position {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    end: Position {
                        line: 1,
                        column: 20,
                        offset: 19,
                    },
                },
            }],
            span: Span::dummy(),
        };
        world.update_index_from_ast("file:///test/a.yx", &module_a);

        // 文件 B 引用 foo
        let content_b = "result = foo()\n";
        session.document_store_mut().open(
            "file:///test/b.yx".to_string(),
            content_b.to_string(),
            1,
        );

        // 光标在 'foo' 上（character=9 是 'f'）
        let params = make_params("file:///test/b.yx", 0, 9);
        let result = handle_definition(&session, &world, params);
        assert!(result.is_some(), "跨文件应找到 foo 的定义");

        match result.unwrap() {
            GotoDefinitionResponse::Scalar(loc) => {
                assert_eq!(loc.uri.to_string(), "file:///test/a.yx", "应跳转到文件 A");
            }
            _ => panic!("单个定义应返回 Scalar"),
        }
    }

    #[test]
    fn test_definition_multiple_defs() {
        let mut session = Session::new();
        let mut world = World::new();

        // 两个文件都定义了同名函数 'helper'
        let mk_module = |span_start: usize| Module {
            items: vec![Stmt {
                kind: StmtKind::Fn {
                    name: "helper".to_string(),
                    generic_params: vec![],
                    type_annotation: None,
                    params: vec![],
                    body: (vec![], None),
                    is_pub: false,
                },
                span: Span {
                    start: Position {
                        line: 1,
                        column: 1,
                        offset: span_start,
                    },
                    end: Position {
                        line: 1,
                        column: 20,
                        offset: span_start + 19,
                    },
                },
            }],
            span: Span::dummy(),
        };

        world.update_index_from_ast("file:///test/a.yx", &mk_module(0));
        world.update_index_from_ast("file:///test/b.yx", &mk_module(100));

        // 文件 C 引用 helper
        let content_c = "helper()\n";
        session.document_store_mut().open(
            "file:///test/c.yx".to_string(),
            content_c.to_string(),
            1,
        );

        let params = make_params("file:///test/c.yx", 0, 0);
        let result = handle_definition(&session, &world, params);
        assert!(result.is_some());

        match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => {
                assert_eq!(locs.len(), 2, "同名两个定义应返回 Array");
            }
            _ => panic!("多个定义应返回 Array"),
        }
    }
}
