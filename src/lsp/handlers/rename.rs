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
/// 1. 查找符号的所有引用位置
/// 2. 为每个引用位置生成 TextEdit
/// 3. 返回 WorkspaceEdit
pub fn handle_rename(
    session: &Session,
    _world: &World,
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

    // 按文件分组引用
    let mut refs_by_file: HashMap<String, Vec<Range>> = HashMap::new();

    // 1. 在所有已打开的文档中查找引用（包含定义位置）
    for (doc_uri, doc) in session.document_store().all_documents() {
        let doc_uri_string = doc_uri.to_string();
        let occurrences =
            crate::lsp::locate::find_all_identifier_occurrences(doc.content(), &ident.name);
        for span in occurrences {
            let range = span_to_range(&span);
            refs_by_file
                .entry(doc_uri_string.clone())
                .or_default()
                .push(range);
        }
    }

    // 构建 changes HashMap
    #[allow(clippy::mutable_key_type)]
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    // 为每个文件生成 TextEdit
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
    use crate::lsp::session::Session;
    use crate::lsp::world::World;
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

    #[allow(clippy::mutable_key_type)]
    #[test]
    fn test_rename_single_file() {
        let mut session = Session::new();
        let world = World::new();

        let content = "x = 1\ny = x + x\n";
        session.document_store_mut().open(
            "file:///test/main.yx".to_string(),
            content.to_string(),
            1,
        );

        // 重命名 'x' 为 'new_x'
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
        assert_eq!(edits.len(), 3, "x 应出现 3 次");
    }

    #[test]
    fn test_rename_cross_file() {
        let mut session = Session::new();
        let world = World::new();

        // 文件 A 定义了 foo
        let content_a = "foo = 1\n";
        session.document_store_mut().open(
            "file:///test/a.yx".to_string(),
            content_a.to_string(),
            1,
        );

        // 文件 B 引用 foo
        let content_b = "result = foo + foo\n";
        session.document_store_mut().open(
            "file:///test/b.yx".to_string(),
            content_b.to_string(),
            1,
        );

        // 重命名 'foo' 为 'bar'
        let params = make_rename_params("file:///test/a.yx", 0, 0, "bar");
        let result = handle_rename(&session, &world, params);
        assert!(result.is_some());

        #[allow(clippy::mutable_key_type)]
        let changes = result.unwrap().changes.unwrap();
        assert_eq!(changes.len(), 2, "应修改 2 个文件");
    }
}
