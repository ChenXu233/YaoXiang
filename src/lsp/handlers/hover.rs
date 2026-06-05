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

use crate::frontend::core::lexer::symbols::SymbolKind;
use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/hover` 请求
///
/// 根据光标位置标识符查找其定义信息，生成悬停提示内容：
/// - 变量：显示类型为变量
/// - 函数：显示参数数量和泛型信息
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

    // 在全局符号索引中查找
    let symbols = world.symbol_index().find_by_name(&ident.name);

    if symbols.is_empty() {
        debug!("未找到符号: {}", ident.name);
        return None;
    }

    // 用第一个匹配的符号构建悬停信息
    // 如果有多个同名符号，全部展示
    let mut parts: Vec<String> = Vec::new();

    for sym in symbols {
        let info = match sym.kind {
            SymbolKind::Variable => {
                format!("```yaoxiang\n(变量) {}\n```", sym.name)
            }
            SymbolKind::Function => {
                if let Some(arity) = sym.arity {
                    format!("```yaoxiang\n(函数) {}({} 个参数)\n```", sym.name, arity)
                } else {
                    format!("```yaoxiang\n(函数) {}\n```", sym.name)
                }
            }
            SymbolKind::GenericFunction => {
                if let Some(arity) = sym.arity {
                    format!(
                        "```yaoxiang\n(泛型函数) {}(T: Type, ...{} 个参数)\n```",
                        sym.name, arity
                    )
                } else {
                    format!("```yaoxiang\n(泛型函数) {}(T: Type)\n```", sym.name)
                }
            }
            SymbolKind::Type => {
                format!("```yaoxiang\n(类型) {}: Type\n```", sym.name)
            }
            SymbolKind::GenericType => {
                format!("```yaoxiang\n(泛型类型) {}(T): Type\n```", sym.name)
            }
            SymbolKind::TypeClass => {
                format!("```yaoxiang\n(类型类) {}\n```", sym.name)
            }
            SymbolKind::Trait => {
                format!("```yaoxiang\n(特征) {}\n```", sym.name)
            }
            _ => {
                format!("```yaoxiang\n{}\n```", sym.name)
            }
        };

        // 添加来源文件信息
        let file_info = format!("*定义于 {}*", sym.location.file_path);
        parts.push(format!("{}\n\n{}", info, file_info));
    }

    let markdown = parts.join("\n\n---\n\n");

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
    use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
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

        let module = Module {
            items: vec![
                Stmt {
                    kind: StmtKind::Var {
                        name: "x".to_string(),
                        name_span: Span {
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
                    kind: StmtKind::Binding {
                        name: "add".to_string(),
                        type_name: None,
                        method_type: None,
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
    fn test_hover_variable() {
        let (session, world) = setup();
        let params = make_params("file:///test/main.yx", 0, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("变量"));
            assert!(markup.value.contains("x"));
        } else {
            panic!("应返回 Markup 内容");
        }
    }

    #[test]
    fn test_hover_function() {
        let (session, world) = setup();
        // 'add' 在第二行
        let params = make_params("file:///test/main.yx", 1, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("函数"));
            assert!(markup.value.contains("add"));
        } else {
            panic!("应返回 Markup 内容");
        }
    }

    #[test]
    fn test_hover_not_on_identifier() {
        let (session, world) = setup();
        let params = make_params("file:///test/main.yx", 0, 2);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_hover_unknown_symbol() {
        let (session, world) = setup();
        // 'a' 是参数，不在顶层索引中
        let params = make_params("file:///test/main.yx", 1, 7);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_hover_includes_range() {
        let (session, world) = setup();
        let params = make_params("file:///test/main.yx", 0, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());
        assert!(result.unwrap().range.is_some(), "应包含高亮 range");
    }

    #[test]
    fn test_hover_type() {
        let mut session = Session::new();
        let mut world = World::new();

        let content = "Point = 1\n";
        session.document_store_mut().open(
            "file:///test/types.yx".to_string(),
            content.to_string(),
            1,
        );

        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Binding {
                    name: "Point".to_string(),
                    type_name: None,
                    method_type: None,
                    generic_params: vec![],
                    type_annotation: Some(crate::frontend::core::parser::ast::Type::Void),

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
                        column: 10,
                        offset: 9,
                    },
                },
            }],
            span: Span::dummy(),
        };
        world.update_index_from_ast("file:///test/types.yx", &module);

        let params = make_params("file:///test/types.yx", 0, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("类型"));
            assert!(markup.value.contains("Point"));
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
        let params = make_params("file:///test/main.yx", 0, 0);
        let result = handle_hover(&session, &world, params);
        assert!(result.is_some());

        let hover = result.unwrap();
        if let HoverContents::Markup(markup) = &hover.contents {
            assert!(markup.value.contains("定义于"), "应包含文件来源信息");
        }
    }
}
