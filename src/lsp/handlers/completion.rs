//! 代码补全处理
//!
//! **状态**：阶段 3 (v0.8) 实现
//!
//! 支持：
//! - 关键字补全（17 个关键字）
//! - 保留字补全（7 个保留字）
//! - 函数注解补全（@block, @eager）
//! - 标识符补全（基于符号索引 + 当前文档 AST）

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, Documentation,
    MarkupContent, MarkupKind,
};
use tracing::debug;

use crate::frontend::core::lexer::symbols::{SymbolKind, SymbolIndex};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_recovery;
use crate::frontend::core::parser::ast::{Module, StmtKind};
use crate::lsp::session::Session;
use crate::lsp::world::World;

// ─── 关键字定义 ─────────────────────────────────────

/// YaoXiang 关键字（language-spec.md 第 2.3 节，共 17 个）
const KEYWORDS: &[(&str, &str)] = &[
    ("pub", "公开声明"),
    ("use", "模块导入"),
    ("spawn", "并作函数标记"),
    ("ref", "Arc 引用计数共享"),
    ("mut", "可变绑定"),
    ("if", "条件分支"),
    ("elif", "否则如果"),
    ("else", "否则分支"),
    ("match", "模式匹配"),
    ("while", "条件循环"),
    ("for", "迭代循环"),
    ("return", "函数返回"),
    ("break", "循环跳出"),
    ("continue", "循环继续"),
    ("as", "类型转换"),
    ("in", "for 循环迭代"),
    ("unsafe", "不安全代码块"),
];

/// YaoXiang 保留字（language-spec.md 第 2.4 节，共 7 个）
const RESERVED_WORDS: &[(&str, &str)] = &[
    ("Type", "元类型（用于类型定义）"),
    ("true", "Bool 真值"),
    ("false", "Bool 假值"),
    ("void", "Void 空值"),
    ("some", "Option 值变体构造 some(T)"),
    ("ok", "Result 成功变体构造 ok(T)"),
    ("err", "Result 错误变体构造 err(E)"),
];

/// 函数注解（language-spec.md 第 6.9.1 节）
const ANNOTATIONS: &[(&str, &str)] = &[("@block", "禁用并发优化"), ("@eager", "强制急切求值")];

// ─── 补全项构建 ─────────────────────────────────────

/// 构建关键字补全项
fn keyword_items() -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .enumerate()
        .map(|(i, (kw, desc))| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(desc.to_string()),
            sort_text: Some(format!("0_{:02}", i)),
            ..CompletionItem::default()
        })
        .collect()
}

/// 构建保留字补全项
fn reserved_word_items() -> Vec<CompletionItem> {
    RESERVED_WORDS
        .iter()
        .enumerate()
        .map(|(i, (word, desc))| CompletionItem {
            label: word.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(desc.to_string()),
            sort_text: Some(format!("1_{:02}", i)),
            ..CompletionItem::default()
        })
        .collect()
}

/// 构建注解补全项
fn annotation_items() -> Vec<CompletionItem> {
    ANNOTATIONS
        .iter()
        .enumerate()
        .map(|(i, (ann, desc))| CompletionItem {
            label: ann.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(desc.to_string()),
            sort_text: Some(format!("2_{:02}", i)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{}**\n\n{}", ann, desc),
            })),
            ..CompletionItem::default()
        })
        .collect()
}

/// 将 SymbolKind 转换为 LSP CompletionItemKind
fn symbol_kind_to_completion_kind(kind: &SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Variable => CompletionItemKind::VARIABLE,
        SymbolKind::Function | SymbolKind::GenericFunction => CompletionItemKind::FUNCTION,
        SymbolKind::Type | SymbolKind::GenericType => CompletionItemKind::CLASS,
        SymbolKind::TypeClass | SymbolKind::Trait => CompletionItemKind::INTERFACE,
        SymbolKind::ConstGeneric => CompletionItemKind::CONSTANT,
        SymbolKind::HigherKindedType | SymbolKind::TypeFamily => CompletionItemKind::CLASS,
        SymbolKind::Binding | SymbolKind::PositionBinding => CompletionItemKind::VALUE,
    }
}

/// 从符号索引构建标识符补全项
fn symbol_index_items(index: &SymbolIndex) -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();

    for name in index.all_names() {
        if seen.contains(name) {
            continue;
        }
        seen.insert(name);

        let symbols = index.find_by_name(name);
        if let Some(first) = symbols.first() {
            let kind = symbol_kind_to_completion_kind(&first.kind);
            let detail = match &first.kind {
                SymbolKind::Function | SymbolKind::GenericFunction => {
                    let arity = first.arity.unwrap_or(0);
                    Some(format!("函数 (参数: {})", arity))
                }
                SymbolKind::Variable => Some("变量".to_string()),
                SymbolKind::Type | SymbolKind::GenericType => Some("类型".to_string()),
                _ => None,
            };

            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(kind),
                detail,
                sort_text: Some(format!("3_{}", name)),
                ..CompletionItem::default()
            });
        }
    }

    items
}

/// 从当前文档 AST 提取局部符号补全项
///
/// 补充符号索引未覆盖的当前文件符号（如局部变量）。
fn document_symbol_items(content: &str) -> Vec<CompletionItem> {
    let tokens = match tokenize(content) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let parse_result = parse_with_recovery(&tokens);
    extract_symbols_from_module(&parse_result.module)
}

/// 从 Module AST 中提取符号为补全项
fn extract_symbols_from_module(module: &Module) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for stmt in &module.items {
        match &stmt.kind {
            StmtKind::Var { name, .. } => {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some("变量".to_string()),
                    sort_text: Some(format!("4_{}", name)),
                    ..CompletionItem::default()
                });
            }
            StmtKind::Binding {
                name,
                type_name,
                params,
                ..
            } => {
                if let Some(type_name) = type_name {
                    // 方法绑定
                    items.push(CompletionItem {
                        label: format!("{}.{}", type_name, name),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("方法绑定".to_string()),
                        sort_text: Some(format!("4_{}.{}", type_name, name)),
                        ..CompletionItem::default()
                    });
                } else if !params.is_empty() {
                    // 函数定义
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("函数 (参数: {})", params.len())),
                        sort_text: Some(format!("4_{}", name)),
                        ..CompletionItem::default()
                    });
                } else {
                    // 类型定义
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some("类型".to_string()),
                        sort_text: Some(format!("4_{}", name)),
                        ..CompletionItem::default()
                    });
                }
            }
            _ => {}
        }
    }

    items
}

// ─── 补全请求处理 ─────────────────────────────────────

/// 处理 `textDocument/completion` 请求
///
/// 返回补全列表，包含：
/// 1. 关键字（sort_text 前缀 "0_"）
/// 2. 保留字（sort_text 前缀 "1_"）
/// 3. 注解（sort_text 前缀 "2_"）
/// 4. 全局符号索引（sort_text 前缀 "3_"）
/// 5. 当前文档符号（sort_text 前缀 "4_"）
pub fn handle_completion(
    session: &Session,
    world: &World,
    params: CompletionParams,
) -> CompletionResponse {
    let uri = params
        .text_document_position
        .text_document
        .uri
        .as_str()
        .to_string();

    debug!(
        "补全请求: {} 行={} 列={}",
        uri,
        params.text_document_position.position.line,
        params.text_document_position.position.character,
    );

    let mut items = Vec::new();

    // 1. 关键字 + 保留字 + 注解
    items.extend(keyword_items());
    items.extend(reserved_word_items());
    items.extend(annotation_items());

    // 2. 全局符号索引
    items.extend(symbol_index_items(world.symbol_index()));

    // 3. 当前文档符号
    if let Some(doc) = session.document_store().get(&uri) {
        let doc_items = document_symbol_items(doc.content());
        // 去重：全局索引已有的不再添加
        let existing: std::collections::HashSet<String> =
            items.iter().map(|i| i.label.clone()).collect();
        for item in doc_items {
            if !existing.contains(&item.label) {
                items.push(item);
            }
        }
    }

    debug!("补全项: {} 个", items.len());

    CompletionResponse::Array(items)
}

// ─── 测试 ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use crate::frontend::core::lexer::symbols::SymbolLocation;
    use crate::frontend::core::lexer::symbols::IndexedSymbol;
    use crate::util::span::{Position as YxPosition, Span};

    fn dummy_span() -> Span {
        Span {
            start: YxPosition {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: YxPosition {
                line: 1,
                column: 10,
                offset: 9,
            },
        }
    }

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
    fn test_symbol_kind_to_completion_kind() {
        assert_eq!(
            symbol_kind_to_completion_kind(&SymbolKind::Variable),
            CompletionItemKind::VARIABLE
        );
        assert_eq!(
            symbol_kind_to_completion_kind(&SymbolKind::Function),
            CompletionItemKind::FUNCTION
        );
        assert_eq!(
            symbol_kind_to_completion_kind(&SymbolKind::Type),
            CompletionItemKind::CLASS
        );
        assert_eq!(
            symbol_kind_to_completion_kind(&SymbolKind::Trait),
            CompletionItemKind::INTERFACE
        );
    }

    #[test]
    fn test_symbol_index_items() {
        let mut index = SymbolIndex::new();
        index.add(IndexedSymbol {
            name: "my_func".to_string(),
            kind: SymbolKind::Function,
            arity: Some(2),
            location: SymbolLocation::new("test.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "MyType".to_string(),
            kind: SymbolKind::Type,
            arity: None,
            location: SymbolLocation::new("test.yx".to_string(), dummy_span()),
        });

        let items = symbol_index_items(&index);
        assert_eq!(items.len(), 2);

        let func_item = items.iter().find(|i| i.label == "my_func").unwrap();
        assert_eq!(func_item.kind, Some(CompletionItemKind::FUNCTION));
        assert!(func_item.detail.as_ref().unwrap().contains("2"));

        let type_item = items.iter().find(|i| i.label == "MyType").unwrap();
        assert_eq!(type_item.kind, Some(CompletionItemKind::CLASS));
    }

    #[test]
    fn test_symbol_index_items_dedup() {
        let mut index = SymbolIndex::new();
        // 同名符号在两个文件
        index.add(IndexedSymbol {
            name: "shared".to_string(),
            kind: SymbolKind::Function,
            arity: Some(0),
            location: SymbolLocation::new("a.yx".to_string(), dummy_span()),
        });
        index.add(IndexedSymbol {
            name: "shared".to_string(),
            kind: SymbolKind::Function,
            arity: Some(0),
            location: SymbolLocation::new("b.yx".to_string(), dummy_span()),
        });

        let items = symbol_index_items(&index);
        // 应去重
        assert_eq!(items.len(), 1);
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
}
