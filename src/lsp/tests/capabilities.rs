//! LSP 能力声明测试
//!
//! 测试覆盖：
//! - 服务器能力声明构建
//! - 文档同步能力
//! - 代码补全能力
//! - 跳转定义能力
//! - 查找引用能力
//! - 悬停提示能力
//! - 能力声明序列化

use lsp_types::{
    CodeActionOptions, CompletionOptions, HoverProviderCapability, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, SaveOptions,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities,
};

use crate::frontend::core::typecheck::semantic_db::{SemanticTokenModifier, SemanticTokenType};
use crate::lsp::capabilities::server_capabilities;

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
