//! LSP 服务端能力声明
//!
//! 定义 YaoXiang 语言服务器支持的 LSP 功能。

use lsp_types::{
    CompletionOptions, HoverProviderCapability, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, SaveOptions,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities,
};

use crate::frontend::typecheck::semantic_db::{SemanticTokenModifier, SemanticTokenType};

/// 构建服务端能力声明
///
/// 当前阶段（v0.8）支持的能力：
/// - 文档同步（Full sync）
/// - 代码补全（关键字、保留字、注解、标识符）
/// - 跳转定义
/// - 查找引用
/// - 悬停提示
///
/// 后续阶段将逐步添加：
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

        // 代码补全（v0.8）
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".to_string(), "@".to_string()]),
            ..CompletionOptions::default()
        }),

        // 跳转定义（v0.8）
        definition_provider: Some(OneOf::Left(true)),

        // 查找引用（v0.8）
        references_provider: Some(OneOf::Left(true)),

        // 悬停提示（v0.8）
        hover_provider: Some(HoverProviderCapability::Simple(true)),

        // 语义 tokens（v0.10）
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: SemanticTokenType::legend()
                        .into_iter()
                        .map(lsp_types::SemanticTokenType::new)
                        .collect(),
                    token_modifiers: SemanticTokenModifier::legend()
                        .into_iter()
                        .map(lsp_types::SemanticTokenModifier::new)
                        .collect(),
                },
                full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                range: None,
                ..SemanticTokensOptions::default()
            },
        )),

        // 以下能力将在后续阶段启用
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
    fn test_completion_capability() {
        let caps = server_capabilities();
        assert!(caps.completion_provider.is_some(), "补全能力应开启");

        let comp = caps.completion_provider.unwrap();
        assert_eq!(comp.resolve_provider, Some(false));
        let triggers = comp.trigger_characters.unwrap();
        assert!(triggers.contains(&".".to_string()));
        assert!(triggers.contains(&"@".to_string()));
    }

    #[test]
    fn test_definition_capability() {
        let caps = server_capabilities();
        assert!(caps.definition_provider.is_some(), "跳转定义能力应开启");
    }

    #[test]
    fn test_references_capability() {
        let caps = server_capabilities();
        assert!(caps.references_provider.is_some(), "查找引用能力应开启");
    }

    #[test]
    fn test_hover_capability() {
        let caps = server_capabilities();
        assert!(caps.hover_provider.is_some(), "悬停提示能力应开启");
    }

    #[test]
    fn test_capabilities_serializable() {
        let caps = server_capabilities();
        // 确保能力声明能序列化为 JSON（LSP 协议需要）
        let json = serde_json::to_string(&caps);
        assert!(json.is_ok());
    }
}
