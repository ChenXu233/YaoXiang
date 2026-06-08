//! 跳转定义处理
//!
//! **状态**：阶段 4 (v0.8) 实现
//!
//! 支持：
//! - 函数定义跳转
//! - 变量定义跳转
//! - 类型定义跳转
//! - 跨文件跳转（基于全局符号索引）
//! - SemanticDB 精确匹配（优先使用作用域和上下文信息）
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
/// 查找光标位置处标识符的定义位置，按优先级：
/// 1. SemanticDB 精确匹配（利用作用域和上下文信息）
/// 2. 全局符号索引回退
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

    // 策略 1：尝试 SemanticDB 精确匹配
    if let Some(response) = try_semantic_db_lookup(world, &uri_str, &ident.name, position) {
        debug!("SemanticDB 精确匹配成功: {}", ident.name);
        return Some(response);
    }

    // 策略 2：回退到全局符号索引
    let symbols = world.symbol_index().find_by_name(&ident.name);

    if symbols.is_empty() {
        debug!("未找到符号定义: {}", ident.name);
        return None;
    }

    // 如果只有一个同名定义，直接返回
    if symbols.len() == 1 {
        let sym = &symbols[0];
        let uri = Uri::from_str(&sym.location.file_path).ok()?;
        let range = span_to_range(&sym.location.span);
        debug!("SymbolIndex 唯一匹配: {}", ident.name);
        return Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)));
    }

    // 多个同名定义：尝试用当前文件优先匹配
    let mut same_file: Vec<Location> = Vec::new();
    let mut other_files: Vec<Location> = Vec::new();

    for sym in symbols {
        if let Ok(uri) = Uri::from_str(&sym.location.file_path) {
            let range = span_to_range(&sym.location.span);
            let loc = Location::new(uri, range);
            if sym.location.file_path == uri_str {
                same_file.push(loc);
            } else {
                other_files.push(loc);
            }
        }
    }

    // 优先返回同文件的定义
    let locations = if !same_file.is_empty() {
        same_file
    } else {
        other_files
    };

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

/// 尝试使用 SemanticDB 进行精确符号查找
///
/// 利用 SemanticDB 中的作用域信息和定义位置进行更精确的匹配：
/// 1. 查找光标所在的最内层作用域
/// 2. 检查该作用域及其父作用域中是否有该符号的定义
/// 3. 优先返回作用域最近的定义
fn try_semantic_db_lookup(
    world: &World,
    file_path: &str,
    name: &str,
    position: &lsp_types::Position,
) -> Option<GotoDefinitionResponse> {
    let db = world.semantic_db();

    // 先从 SemanticDB 的 symbol_defs 查找定义位置
    let defs = db.get_symbol_defs(name)?;

    if defs.is_empty() {
        return None;
    }

    // 如果只有一个定义，直接返回
    if defs.len() == 1 {
        let def = &defs[0];
        let uri = Uri::from_str(&def.file_path).ok()?;
        let range = span_to_range(&def.span);
        return Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)));
    }

    // 多个定义：利用作用域信息过滤
    // LSP 0-indexed → 1-indexed
    let cursor_line = position.line as usize + 1;
    let cursor_col = position.character as usize + 1;

    // 查找光标所在的最内层作用域
    if let Some(scope) = db.find_innermost_scope(file_path, cursor_line, cursor_col) {
        // 如果该作用域包含该符号，优先匹配同文件的定义
        if scope.symbols.contains(&name.to_string()) {
            let same_file_defs: Vec<_> = defs.iter().filter(|d| d.file_path == file_path).collect();

            if same_file_defs.len() == 1 {
                let def = same_file_defs[0];
                let uri = Uri::from_str(&def.file_path).ok()?;
                let range = span_to_range(&def.span);
                return Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)));
            }
        }
    }

    // 如果无法通过作用域精确匹配，优先同文件定义
    let same_file_defs: Vec<_> = defs.iter().filter(|d| d.file_path == file_path).collect();
    if same_file_defs.len() == 1 {
        let def = same_file_defs[0];
        let uri = Uri::from_str(&def.file_path).ok()?;
        let range = span_to_range(&def.span);
        return Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)));
    }

    // 所有定义都返回，让客户端选择
    let locations: Vec<Location> = defs
        .iter()
        .filter_map(|def| {
            let uri = Uri::from_str(&def.file_path).ok()?;
            let range = span_to_range(&def.span);
            Some(Location::new(uri, range))
        })
        .collect();

    if locations.is_empty() {
        return None;
    }

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
    use crate::frontend::core::typecheck::semantic_db::{
        FileSemanticInfo, SemanticToken, SemanticTokenModifier, SemanticTokenType, ScopeInfo,
        ScopeKind,
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
                kind: StmtKind::Binding {
                    name: "foo".to_string(),
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
                kind: StmtKind::Binding {
                    name: "helper".to_string(),
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

    #[test]
    fn test_definition_via_semantic_db() {
        let mut session = Session::new();
        let mut world = World::new();

        // 打开文档
        let content = "add = (a, b) => a + b\nresult = add(1, 2)\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        // 在 SemanticDB 中注册 add 的定义
        let info = FileSemanticInfo {
            file_path: "file:///test/main.yx".to_string(),
            tokens: vec![
                SemanticToken {
                    name: "add".to_string(),
                    token_type: SemanticTokenType::Function,
                    modifiers: vec![SemanticTokenModifier::Declaration],
                    span: Span {
                        start: Position {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        end: Position {
                            line: 1,
                            column: 4,
                            offset: 3,
                        },
                    },
                },
                SemanticToken {
                    name: "add".to_string(),
                    token_type: SemanticTokenType::Function,
                    modifiers: vec![],
                    span: Span {
                        start: Position {
                            line: 2,
                            column: 10,
                            offset: 31,
                        },
                        end: Position {
                            line: 2,
                            column: 13,
                            offset: 34,
                        },
                    },
                },
            ],
            scopes: vec![ScopeInfo {
                span: Span {
                    start: Position {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    end: Position {
                        line: 2,
                        column: 20,
                        offset: 40,
                    },
                },
                parent: None,
                symbols: vec!["add".to_string()],
                kind: ScopeKind::Global,
            }],
            ..Default::default()
        };

        world
            .semantic_db_mut()
            .set_file_info("file:///test/main.yx".to_string(), info);

        // 光标在第二行的 'add' 引用上
        let params = make_params("file:///test/main.yx", 1, 9);
        let result = handle_definition(&session, &world, params);
        assert!(result.is_some(), "SemanticDB 应找到 add 的定义");

        match result.unwrap() {
            GotoDefinitionResponse::Scalar(loc) => {
                assert_eq!(loc.uri.to_string(), "file:///test/main.yx");
                assert_eq!(loc.range.start.line, 0, "应跳转到第 1 行（0-indexed）");
            }
            _ => panic!("SemanticDB 精确匹配应返回 Scalar"),
        }
    }

    #[test]
    fn test_definition_semantic_db_disambiguates() {
        let mut session = Session::new();
        let mut world = World::new();

        let content = "helper()\n";
        session
            .document_store_mut()
            .open("file:///test/c.yx".to_string(), content.to_string(), 1);

        // 两个文件都定义了 helper
        let info_a = FileSemanticInfo {
            file_path: "file:///test/a.yx".to_string(),
            tokens: vec![SemanticToken {
                name: "helper".to_string(),
                token_type: SemanticTokenType::Function,
                modifiers: vec![SemanticTokenModifier::Declaration],
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
            }],
            scopes: vec![],
            ..Default::default()
        };
        let info_b = FileSemanticInfo {
            file_path: "file:///test/b.yx".to_string(),
            tokens: vec![SemanticToken {
                name: "helper".to_string(),
                token_type: SemanticTokenType::Function,
                modifiers: vec![SemanticTokenModifier::Declaration],
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
            }],
            scopes: vec![],
            ..Default::default()
        };

        world
            .semantic_db_mut()
            .set_file_info("file:///test/a.yx".to_string(), info_a);
        world
            .semantic_db_mut()
            .set_file_info("file:///test/b.yx".to_string(), info_b);

        let params = make_params("file:///test/c.yx", 0, 0);
        let result = handle_definition(&session, &world, params);
        assert!(result.is_some(), "应从 SemanticDB 找到 helper 定义");

        // 两个定义，应返回 Array
        match result.unwrap() {
            GotoDefinitionResponse::Array(locs) => {
                assert_eq!(locs.len(), 2, "两个 SemanticDB 定义应返回 Array");
            }
            _ => panic!("多个 SemanticDB 定义应返回 Array"),
        }
    }
}
