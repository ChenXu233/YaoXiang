//! 代码补全处理器测试
//!
//! 测试覆盖：
//! - 关键字补全项
//! - 保留字补全项
//! - 注解补全项
//! - 文档符号补全项
//! - 补全请求处理
//! - 排序文本验证

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, Documentation,
    MarkupContent, MarkupKind,
};

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_recovery;
use crate::frontend::core::parser::ast::{Module, StmtKind};
use crate::frontend::core::typecheck::semantic_db::DefinitionKind;
use crate::lsp::handlers::completion::{
    handle_completion, keyword_items, reserved_word_items, annotation_items, document_symbol_items,
    KEYWORDS, RESERVED_WORDS, ANNOTATIONS,
};
use crate::lsp::session::Session;
use crate::lsp::world::World;

use std::str::FromStr;

#[test]
fn test_keyword_items_count() {
    let items = keyword_items();
    assert_eq!(items.len(), 17, "应有 17 个关键字");
}

#[test]
fn test_all_keywords_present() {
    let items = keyword_items();
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for (kw, _) in KEYWORDS {
        assert!(labels.contains(kw), "缺少关键字: {}", kw);
    }
}

#[test]
fn test_keyword_items_kind() {
    let items = keyword_items();
    for item in &items {
        assert_eq!(item.kind, Some(CompletionItemKind::KEYWORD));
    }
}

#[test]
fn test_reserved_word_items_count() {
    let items = reserved_word_items();
    assert_eq!(items.len(), 7, "应有 7 个保留字");
}

#[test]
fn test_all_reserved_words_present() {
    let items = reserved_word_items();
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for (word, _) in RESERVED_WORDS {
        assert!(labels.contains(word), "缺少保留字: {}", word);
    }
}

#[test]
fn test_reserved_word_items_kind() {
    let items = reserved_word_items();
    for item in &items {
        assert_eq!(item.kind, Some(CompletionItemKind::CONSTANT));
    }
}

#[test]
fn test_annotation_items_count() {
    let items = annotation_items();
    assert_eq!(items.len(), 2, "应有 2 个注解");
}

#[test]
fn test_annotation_items_present() {
    let items = annotation_items();
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"@block"));
    assert!(labels.contains(&"@eager"));
}

#[test]
fn test_annotation_items_have_docs() {
    let items = annotation_items();
    for item in &items {
        assert!(item.documentation.is_some(), "{} 应有文档", item.label);
    }
}

#[test]
fn test_document_symbol_items_valid_code() {
    let content = "x = 42\nadd = (a, b) => a + b\n";
    let items = document_symbol_items(content);
    // 至少应有 x 和 add
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"x"), "应包含变量 x，实际: {:?}", labels);
    assert!(
        labels.contains(&"add"),
        "应包含函数 add，实际: {:?}",
        labels
    );
}

#[test]
fn test_document_symbol_items_invalid_code() {
    let content = "@ @ @\n";
    // 不应 panic
    let items = document_symbol_items(content);
    let _ = items;
}

#[test]
fn test_handle_completion_basic() {
    let session = Session::new();
    let world = World::new();

    let params = CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///test.yx").unwrap(),
            },
            position: lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let response = handle_completion(&session, &world, params);
    if let CompletionResponse::Array(items) = response {
        // 至少应有 17 关键字 + 7 保留字 + 2 注解 = 26
        assert!(
            items.len() >= 26,
            "至少应有 26 个补全项，实际: {}",
            items.len()
        );

        // 检查 sort_text 排序正确
        let kw_items: Vec<_> = items
            .iter()
            .filter(|i| i.kind == Some(CompletionItemKind::KEYWORD))
            .collect();
        assert_eq!(kw_items.len(), 17);
    } else {
        panic!("expected CompletionResponse::Array");
    }
}

#[test]
fn test_sort_text_ordering() {
    let items = keyword_items();
    let reserved = reserved_word_items();
    let annotations = annotation_items();

    // 关键字 sort_text 前缀 "0_"
    for item in &items {
        assert!(
            item.sort_text.as_ref().unwrap().starts_with("0_"),
            "关键字 sort_text 应以 0_ 开头"
        );
    }
    // 保留字 sort_text 前缀 "1_"
    for item in &reserved {
        assert!(
            item.sort_text.as_ref().unwrap().starts_with("1_"),
            "保留字 sort_text 应以 1_ 开头"
        );
    }
    // 注解 sort_text 前缀 "2_"
    for item in &annotations {
        assert!(
            item.sort_text.as_ref().unwrap().starts_with("2_"),
            "注解 sort_text 应以 2_ 开头"
        );
    }
}
