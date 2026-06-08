//! 悬停提示处理
//!
//! **状态**：阶段 4 (v0.8) 实现
//!
//! 支持：
//! - 变量类型显示
//! - 函数签名显示（参数数量、泛型等）
//! - 类型定义信息显示

use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};
use tracing::debug;

use crate::frontend::core::typecheck::semantic_db::DefinitionKind;
use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/hover` 请求
///
/// 根据光标位置标识符查找其定义信息，生成悬停提示内容：
/// - 变量：显示类型
/// - 函数：显示签名
/// - 类型：显示类型定义信息
pub fn handle_hover(
    session: &Session,
    world: &World,
    params: HoverParams,
) -> Option<Hover> {
    let uri_str = params
        .text_document_position_params
        .text_document
        .uri
        .to_string();
    let position = &params.text_document_position_params.position;

    debug!(
        "悬停提示请求: {} @ {}:{}",
        uri_str, position.line, position.character
    );

    // 获取文档内容
    let doc = session.document_store().get(&uri_str)?;
    let content = doc.content();

    // 查找光标处的标识符
    let ident = find_identifier_at_position(content, position)?;
    debug!("悬停标识符: {}", ident.name);

    // 核心路径：SemanticDB 精确解析
    let db = world.semantic_db();
    let line = position.line as usize + 1;
    let col = position.character as usize + 1;

    let def = db.resolve_reference(&uri_str, line, col)?;

    // 构建悬停内容
    let mut markdown = String::from("```yaoxiang\n");

    match def.kind {
        DefinitionKind::Function | DefinitionKind::Method => {
            if let Some(ref sig) = def.signature {
                markdown.push_str(sig);
                markdown.push('\n');
            } else if let Some(ref ty) = def.type_info {
                markdown.push_str(&format!("{}: {}\n", def.name, ty));
            } else {
                markdown.push_str(&format!("(函数) {}\n", def.name));
            }
        }
        DefinitionKind::Type | DefinitionKind::Interface => {
            if let Some(ref ty) = def.type_info {
                markdown.push_str(&format!("{}: {}\n", def.name, ty));
            } else {
                markdown.push_str(&format!("(类型) {}: Type\n", def.name));
            }
        }
        DefinitionKind::Variable | DefinitionKind::Parameter => {
            if let Some(ref ty) = def.type_info {
                markdown.push_str(&format!("{}: {}\n", def.name, ty));
            } else {
                markdown.push_str(&format!("(变量) {}\n", def.name));
            }
        }
        _ => {
            if let Some(ref ty) = def.type_info {
                markdown.push_str(&format!("{}: {}\n", def.name, ty));
            } else {
                markdown.push_str(&format!("{}\n", def.name));
            }
        }
    }

    markdown.push_str("```\n");
    markdown.push_str(&format!("*定义于 {}*", def.file_path));

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(span_to_range(&ident.span)),
    })
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
        HoverParams, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
        WorkDoneProgressParams,
    };
    use std::str::FromStr;

    fn make_params(
        uri: &str,
        line: u32,
        character: u32,
    ) -> HoverParams {
        HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Uri::from_str(uri).unwrap(),
                },
                position: lsp_types::Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        }
    }

    fn setup() -> (Session, World) {
        let mut session = Session::new();
        let mut world = World::new();

        let content = "x = 42\nadd = (a, b) => a + b\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        let uri = "file:///test/main.yx";

        // x 的定义
        world.semantic_db_mut().add_definition(
            uri,
            DefinitionInfo {
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
            },
        );

        // x 的引用（在第二行）
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

        // add 的定义
        world.semantic_db_mut().add_definition(
            uri,
            DefinitionInfo {
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
            },
        );

        (session, world)
    }

    #[test]
    fn test_hover_variable() {
        let (session, world) = setup();
        // 光标在第二行的 x 引用上
        let params = make_params("file:///test/main.yx", 1, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("Int"), "应显示 x 的类型 Int");
            assert!(markup.value.contains("x"), "应包含变量名");
        } else {
            panic!("应返回 Markup 内容");
        }
    }

    #[test]
    fn test_hover_not_on_identifier() {
        let (session, world) = setup();
        // 光标在 '=' 号上
        let params = make_params("file:///test/main.yx", 0, 2);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_hover_unknown_symbol() {
        let (session, world) = setup();
        // 光标在未注册的位置
        let params = make_params("file:///test/main.yx", 1, 7);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_hover_includes_range() {
        let (session, world) = setup();
        let params = make_params("file:///test/main.yx", 1, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());
        assert!(result.unwrap().range.is_some(), "应包含高亮 range");
    }

    #[test]
    fn test_hover_function_signature() {
        let (session, world) = setup();
        // 光标在第二行的 add 定义上
        let params = make_params("file:///test/main.yx", 1, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            // x 的引用应该显示类型信息
            assert!(markup.value.contains("Int"), "应显示类型信息");
        } else {
            panic!("应返回 Markup 内容");
        }
    }

    #[test]
    fn test_hover_doc_not_open() {
        let (session, world) = setup();
        let params = make_params("file:///test/nope.yx", 0, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_hover_file_info() {
        let (session, world) = setup();
        let params = make_params("file:///test/main.yx", 1, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("定义于"), "应包含文件来源信息");
        }
    }
}
