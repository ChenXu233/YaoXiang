//! LSP 服务端能力声明
//!
//! 定义 YaoXiang 语言服务器支持的 LSP 功能。

use lsp_types::{
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    SaveOptions,
};

/// 构建服务端能力声明
///
/// 当前阶段（v0.7）支持的能力：
/// - 文档同步（Full sync）
///
/// 后续阶段将逐步添加：
/// - 代码补全 (v0.8)
/// - 跳转定义 (v0.8)
/// - 查找引用 (v0.8)
/// - 悬停提示 (v0.8)
/// - 工作区符号搜索 (v0.9)
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // 文档同步：全量同步模式
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(SaveOptions::default().into()),
            },
        )),

        // 以下能力将在后续阶段启用
        // completion_provider: None,        // v0.8
        // definition_provider: None,        // v0.8
        // references_provider: None,        // v0.8
        // hover_provider: None,             // v0.8
        // workspace_symbol_provider: None,  // v0.9
        // document_formatting_provider: None, // v0.9
        ..ServerCapabilities::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_capabilities() {
        let caps = server_capabilities();

        // 文档同步能力已开启
        assert!(caps.text_document_sync.is_some());

        // 验证同步模式为 Full
        if let Some(TextDocumentSyncCapability::Options(opts)) = &caps.text_document_sync {
            assert_eq!(opts.change, Some(TextDocumentSyncKind::FULL));
            assert_eq!(opts.open_close, Some(true));
        } else {
            panic!("Expected TextDocumentSyncOptions");
        }
    }

    #[test]
    fn test_capabilities_serializable() {
        let caps = server_capabilities();
        // 确保能力声明能序列化为 JSON（LSP 协议需要）
        let json = serde_json::to_string(&caps);
        assert!(json.is_ok());
    }
}
