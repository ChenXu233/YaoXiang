//! LSP 符号重命名处理器
//!
//! 实现 `textDocument/rename` 功能。

use lsp_types::{Range, TextEdit, Uri, WorkspaceEdit};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::debug;

use crate::lsp::locate::{find_identifier_at_position, span_to_range};
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 处理 `textDocument/rename` 请求
///
/// 实现完整的符号重命名功能：
/// 1. 通过 SemanticDB 查找符号的所有引用位置
/// 2. 为每个引用位置生成 TextEdit
/// 3. 返回 WorkspaceEdit
pub fn handle_rename(
    session: &Session,
    world: &World,
    params: lsp_types::RenameParams,
) -> Option<WorkspaceEdit> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    let uri_str = uri.to_string();

    debug!(
        "重命名请求: {} @ {}:{} -> {}",
        uri_str, position.line, position.character, params.new_name
    );

    // 获取文档内容
    let doc = session.document_store().get(&uri_str)?;
    let content = doc.content();

    // 查找光标处的标识符
    let ident = find_identifier_at_position(content, &position)?;
    debug!("重命名符号: {}", ident.name);

    let new_name = params.new_name;
    let db = world.semantic_db();
    let line = position.line as usize + 1;
    let col = position.character as usize + 1;

    // 先找到光标位置的定义
    let def = if let Some(def) = db.resolve_reference(&uri_str, line, col) {
        def
    } else {
        let defs = db.get_definitions(&uri_str);
        defs.iter().find(|d| {
            d.span.start.line == line && d.span.start.column <= col && d.span.end.column > col
        })?
    };

    let mut refs_by_file: HashMap<String, Vec<Range>> = HashMap::new();

    // 添加定义位置
    refs_by_file
        .entry(def.file_path.clone())
        .or_default()
        .push(span_to_range(&def.span));

    // 添加所有引用位置
    let refs = db.find_all_references_to(&def.file_path, &def.span);
    for r in refs {
        refs_by_file
            .entry(r.file_path.clone())
            .or_default()
            .push(span_to_range(&r.span));
    }

    // 构建 changes HashMap
    #[allow(clippy::mutable_key_type)]
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    for (file_uri, ranges) in refs_by_file {
        let uri = Uri::from_str(&file_uri).ok()?;
        let edits: Vec<TextEdit> = ranges
            .iter()
            .map(|range| TextEdit {
                range: *range,
                new_text: new_name.clone(),
            })
            .collect();

        if edits.is_empty() {
            continue;
        }

        changes.insert(uri, edits);
    }

    if changes.is_empty() {
        return None;
    }

    debug!("重命名将修改 {} 个文件", changes.len());

    Some(WorkspaceEdit {
        document_changes: None,
        changes: Some(changes),
        change_annotations: None,
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
        RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, Uri,
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
}
