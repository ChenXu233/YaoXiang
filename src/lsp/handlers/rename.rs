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
