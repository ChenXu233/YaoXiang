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

use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/references` 请求
///
/// 查找光标位置处标识符的所有引用位置：
/// 1. 解析光标处的标识符名称
/// 2. 通过 SemanticDB 的引用链查找所有引用
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

    let db = world.semantic_db();
    let line = position.line as usize + 1;
    let col = position.character as usize + 1;

    // 先尝试通过引用找到定义
    let def = if let Some(def) = db.resolve_reference(&uri_str, line, col) {
        def
    } else {
        // 光标可能在定义位置本身，查找 definitions
        let defs = db.get_definitions(&uri_str);
        defs.iter().find(|d| {
            d.span.start.line == line && d.span.start.column <= col && d.span.end.column > col
        })?
    };

    let mut locations: Vec<Location> = Vec::new();

    // 如果 include_declaration 为 true，包含定义位置
    if include_declaration {
        if let Ok(uri) = Uri::from_str(&def.file_path) {
            let range = span_to_range(&def.span);
            locations.push(Location::new(uri, range));
        }
    }

    // 查找所有引用到该定义的位置
    let refs = db.find_all_references_to(&def.file_path, &def.span);
    for r in refs {
        let range = span_to_range(&r.span);
        if let Ok(uri) = Uri::from_str(&r.file_path) {
            let loc = Location::new(uri, range);
            // 去重
            if !locations
                .iter()
                .any(|l| l.uri == loc.uri && l.range == loc.range)
            {
                locations.push(loc);
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
    use crate::frontend::core::typecheck::semantic_db::{
        DefId, DefinitionInfo, DefinitionKind, ReferenceInfo,
    };
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
}
