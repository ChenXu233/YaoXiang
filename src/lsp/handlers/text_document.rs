//! 文档同步处理
//!
//! 处理 `textDocument/didOpen`、`textDocument/didChange`、`textDocument/didClose`。
//!
//! **状态**：阶段 2 实现

use lsp_types::{DidOpenTextDocumentParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams};
use tracing::info;

use crate::lsp::session::Session;

/// 处理 `textDocument/didOpen`
///
/// 返回文档 URI，供调用方触发诊断。
pub fn handle_did_open(
    session: &mut Session,
    params: DidOpenTextDocumentParams,
) -> String {
    let uri = params.text_document.uri.as_str().to_string();
    let version = params.text_document.version as u32;
    let content = params.text_document.text;

    info!("文档打开: {} (v{})", uri, version);

    session
        .document_store_mut()
        .open(uri.clone(), content, version);
    uri
}

/// 处理 `textDocument/didChange`
///
/// 内容发生变更时返回 `Some(uri)`，用于触发诊断更新。
pub fn handle_did_change(
    session: &mut Session,
    params: DidChangeTextDocumentParams,
) -> Option<String> {
    let uri = params.text_document.uri.as_str().to_string();
    let version = params.text_document.version as u32;

    // Full sync 模式：取最后一个变更（即全量内容）
    if let Some(change) = params.content_changes.into_iter().last() {
        let changed = session
            .document_store_mut()
            .update(&uri, change.text, version);

        if changed {
            info!("文档更新: {} (v{})", uri, version);
            return Some(uri);
        }
    }
    None
}

/// 处理 `textDocument/didClose`
///
/// 返回关闭文档的 URI，供调用方清除诊断。
pub fn handle_did_close(
    session: &mut Session,
    params: DidCloseTextDocumentParams,
) -> String {
    let uri = params.text_document.uri.as_str().to_string();
    info!("文档关闭: {}", uri);

    session.document_store_mut().close(&uri);
    uri
}
