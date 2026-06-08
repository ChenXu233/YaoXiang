//! 跳转定义处理
//!
//! **状态**：阶段 4 (v0.8) 实现
//!
//! 支持：
//! - 函数定义跳转
//! - 变量定义跳转
//! - 类型定义跳转
//! - 跨文件跳转（基于 SemanticDB 精确解析）
//! - 局部变量和函数参数跳转
//! - Use 导入符号跳转

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Uri};
use std::str::FromStr;
use tracing::debug;

use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/definition` 请求
///
/// 查找光标位置处标识符的定义位置，通过 SemanticDB 精确解析。
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

    // 核心路径：SemanticDB 精确解析（按文件+位置）
    let db = world.semantic_db();
    let line = position.line as usize + 1;
    let col = position.character as usize + 1;

    if let Some(def) = db.resolve_reference(&uri_str, line, col) {
        let uri = Uri::from_str(&def.file_path).ok()?;
        let range = span_to_range(&def.span);
        debug!(
            "SemanticDB 精确匹配: {} → {}:{}",
            ident.name, def.file_path, def.span.start.line
        );
        return Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)));
    }

    debug!("未找到符号定义: {}", ident.name);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::typecheck::semantic_db::{
        DefId, DefinitionInfo, DefinitionKind, ReferenceInfo,
    };
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
}
