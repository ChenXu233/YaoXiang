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
