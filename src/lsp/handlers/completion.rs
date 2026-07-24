//! 代码补全处理
//!
//! **状态**：阶段 3 (v0.8) 实现
//!
//! 支持：
//! - 关键字补全（17 个关键字）
//! - 保留字补全（7 个保留字）
//! - 标识符补全（基于符号索引 + 当前文档 AST）

use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse};
use tracing::debug;

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::{Module, StmtKind};
use crate::frontend::core::typecheck::semantic_db::DefinitionKind;
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

/// 从 SemanticDB 获取可见符号的补全项
fn semantic_db_items(
    world: &World,
    file: &str,
    line: usize,
    col: usize,
) -> Vec<CompletionItem> {
    let db = world.semantic_db();
    let defs = db.visible_definitions(file, line, col);

    defs.iter()
        .map(|def| {
            let kind = match def.kind {
                DefinitionKind::Function | DefinitionKind::Method => CompletionItemKind::FUNCTION,
                DefinitionKind::Type | DefinitionKind::Interface => CompletionItemKind::CLASS,
                DefinitionKind::Variable | DefinitionKind::Parameter => {
                    CompletionItemKind::VARIABLE
                }
                DefinitionKind::GenericParameter => CompletionItemKind::TYPE_PARAMETER,
            };
            let detail = match def.kind {
                DefinitionKind::Function | DefinitionKind::Method => {
                    def.signature.as_ref().or(def.type_info.as_ref()).cloned()
                }
                DefinitionKind::Type | DefinitionKind::Interface => Some("(类型)".to_string()),
                DefinitionKind::Variable | DefinitionKind::Parameter => {
                    def.type_info.as_ref().map(|ty| format!(": {}", ty))
                }
                _ => None,
            };
            CompletionItem {
                label: def.name.clone(),
                kind: Some(kind),
                detail,
                sort_text: Some(format!("3_{}", def.name)),
                ..CompletionItem::default()
            }
        })
        .collect()
}

/// 从当前文档 AST 提取局部符号补全项
///
/// 补充符号索引未覆盖的当前文件符号（如局部变量）。
fn document_symbol_items(content: &str) -> Vec<CompletionItem> {
    let tokens = match tokenize(content) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let parse_result = parse(&tokens);
    extract_symbols_from_module(&parse_result.module)
}

/// 从 Module AST 中提取符号为补全项
fn extract_symbols_from_module(module: &Module) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for stmt in &module.items {
        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: _,
                value,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let (name, type_name) = match target.as_ref() {
                    Expr::Var(n, _) => (n.clone(), None),
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (field.clone(), Some(tn.clone()))
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => continue,
                };
                if let Some(tn) = type_name {
                    items.push(CompletionItem {
                        label: format!("{}.{}", tn, name),
                        kind: Some(CompletionItemKind::METHOD),
                        detail: Some("方法绑定".to_string()),
                        sort_text: Some(format!("4_{}.{}", tn, name)),
                        ..CompletionItem::default()
                    });
                } else if let Some(v) = value {
                    if let Expr::Lambda { params, .. } = v.as_ref() {
                        if !params.is_empty() {
                            items.push(CompletionItem {
                                label: name.clone(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                detail: Some(format!("函数 (参数: {})", params.len())),
                                sort_text: Some(format!("4_{}", name)),
                                ..CompletionItem::default()
                            });
                        }
                    }
                } else {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("变量".to_string()),
                        sort_text: Some(format!("4_{}", name)),
                        ..CompletionItem::default()
                    });
                }
            }
            StmtKind::TypeDefinition { name, .. } => {
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("类型".to_string()),
                    sort_text: Some(format!("4_{}", name)),
                    ..CompletionItem::default()
                });
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

    // 1. 关键字 + 保留字
    items.extend(keyword_items());
    items.extend(reserved_word_items());

    // 2. 从 SemanticDB 获取可见符号
    let line = params.text_document_position.position.line as usize + 1;
    let col = params.text_document_position.position.character as usize + 1;
    items.extend(semantic_db_items(world, &uri, line, col));

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
