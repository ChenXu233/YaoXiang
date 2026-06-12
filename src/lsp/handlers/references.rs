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
