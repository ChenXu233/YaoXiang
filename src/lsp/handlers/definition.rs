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
