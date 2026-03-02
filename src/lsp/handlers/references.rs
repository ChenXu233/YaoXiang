//! 查找引用处理
//!
//! **状态**：阶段 4 (v0.8) 实现
//!
//! 支持：
//! - 变量引用查找
//! - 函数引用查找
//! - 跨文件引用查找

use lsp_types::{Location, ReferenceParams, Uri};
use std::str::FromStr;
use tracing::debug;

use crate::lsp::locate::{find_all_identifier_occurrences, find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/references` 请求
///
/// 查找光标位置处标识符的所有引用位置：
/// 1. 解析光标处的标识符名称
/// 2. 在所有已打开文档中搜索该标识符的所有出现
/// 3. 可选地包含定义位置
pub fn handle_references(
    session: &Session,
    world: &World,
    params: ReferenceParams,
) -> Option<Vec<Location>> {
    let uri_str = params.text_document_position.text_document.uri.to_string();
    let position = &params.text_document_position.position;
    let include_declaration = params.context.include_declaration;

    debug!(
        "查找引用请求: {} @ {}:{} (include_declaration={})",
        uri_str, position.line, position.character, include_declaration
    );

    // 获取文档内容
    let doc = session.document_store().get(&uri_str)?;
    let content = doc.content();

    // 查找光标处的标识符
    let ident = find_identifier_at_position(content, position)?;
    debug!("查找引用: {}", ident.name);

    let mut locations: Vec<Location> = Vec::new();

    // 1. 如果 include_declaration 为 true，包含定义位置
    if include_declaration {
        let definitions = world.symbol_index().find_by_name(&ident.name);
        for sym in definitions {
            if let Ok(uri) = Uri::from_str(&sym.location.file_path) {
                let range = span_to_range(&sym.location.span);
                locations.push(Location::new(uri, range));
            }
        }
    }

    // 2. 在所有已打开的文档中查找引用
    for (doc_uri, doc) in session.document_store().all_documents() {
        let occurrences = find_all_identifier_occurrences(doc.content(), &ident.name);
        for span in occurrences {
            let range = span_to_range(&span);
            if let Ok(uri) = Uri::from_str(doc_uri) {
                // 如果 include_declaration 为 true，定义位置已经加过了，
                // 检查是否与已有位置重复
                let loc = Location::new(uri, range);
                if !locations
                    .iter()
                    .any(|l| l.uri == loc.uri && l.range == loc.range)
                {
                    locations.push(loc);
                }
            }
        }
    }

    if locations.is_empty() {
        debug!("未找到引用: {}", ident.name);
        return None;
    }

    debug!("找到 {} 个引用", locations.len());
    Some(locations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
    use crate::lsp::session::Session;
    use crate::lsp::world::World;
    use crate::util::span::{Position, Span};
    use lsp_types::{
        PartialResultParams, ReferenceContext, ReferenceParams, TextDocumentIdentifier,
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

    #[test]
    fn test_references_in_single_file() {
        let mut session = Session::new();
        let world = World::new();

        let content = "x = 1\ny = x + x\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        // 查找 'x' 的引用（不含定义）
        let params = make_params("file:///test/main.yx", 0, 0, false);
        let result = handle_references(&session, &world, params);
        assert!(result.is_some());

        let locs = result.unwrap();
        // 'x' 出现 3 次: 定义处 + 两次引用（全部在同一文件，都算引用出现）
        assert_eq!(locs.len(), 3, "x 应出现 3 次（含定义位置的 token）");
    }

    #[test]
    fn test_references_include_declaration() {
        let mut session = Session::new();
        let mut world = World::new();

        let content = "x = 1\ny = x\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        // 在符号索引中注册 x
        let module = Module {
            items: vec![Stmt {
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
                        column: 6,
                        offset: 5,
                    },
                },
            }],
            span: Span::dummy(),
        };
        world.update_index_from_ast("file:///test/main.yx", &module);

        let params = make_params("file:///test/main.yx", 0, 0, true);
        let result = handle_references(&session, &world, params);
        assert!(result.is_some());

        let locs = result.unwrap();
        // include_declaration=true: 定义(1) + 引用(2) = 至少 2
        assert!(locs.len() >= 2);
    }

    #[test]
    fn test_references_not_on_ident() {
        let mut session = Session::new();
        let world = World::new();

        let content = "x = 1\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        let params = make_params("file:///test/main.yx", 0, 2, false);
        let result = handle_references(&session, &world, params);
        assert!(result.is_none());
    }

    #[test]
    fn test_references_cross_file() {
        let mut session = Session::new();
        let mut world = World::new();

        // 文件 A 定义了 foo
        let content_a = "foo = () => 42\n";
        session.document_store_mut().open(
            "file:///test/a.yx".to_string(),
            content_a.to_string(),
            1,
        );

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
                        column: 15,
                        offset: 14,
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

        let params = make_params("file:///test/a.yx", 0, 0, false);
        let result = handle_references(&session, &world, params);
        assert!(result.is_some());

        let locs = result.unwrap();
        // foo 在 a.yx 出现 1 次，在 b.yx 出现 1 次
        assert_eq!(locs.len(), 2, "foo 应在两个文件中各出现 1 次");
        let uris: Vec<String> = locs.iter().map(|l| l.uri.to_string()).collect();
        assert!(uris.contains(&"file:///test/a.yx".to_string()));
        assert!(uris.contains(&"file:///test/b.yx".to_string()));
    }

    #[test]
    fn test_references_doc_not_open() {
        let session = Session::new();
        let world = World::new();

        let params = make_params("file:///test/nope.yx", 0, 0, false);
        let result = handle_references(&session, &world, params);
        assert!(result.is_none());
    }
}
