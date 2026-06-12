//! LSP 服务端能力声明
//!
//! 定义 YaoXiang 语言服务器支持的 LSP 功能。

use lsp_types::{
    CodeActionOptions, CompletionOptions, HoverProviderCapability, OneOf, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, SaveOptions,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities,
};

use crate::frontend::core::typecheck::semantic_db::{SemanticTokenModifier, SemanticTokenType};

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

        // 重命名（v0.9）
        rename_provider: Some(OneOf::Left(true)),

        // 代码操作（v0.9）
        code_action_provider: Some(CodeActionOptions::default().into()),

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

        // 工作区符号搜索（v0.9）
        workspace_symbol_provider: Some(OneOf::Left(true)),

        // 文档格式化（v0.9）
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),

        // 幽灵提示（Inlay Hints）
        inlay_hint_provider: Some(OneOf::Left(true)),

        ..ServerCapabilities::default()
    }
}
