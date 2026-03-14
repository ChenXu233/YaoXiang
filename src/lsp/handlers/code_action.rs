//! LSP 代码操作处理器
//!
//! 实现 `textDocument/codeAction` 功能，提供：
//! - 基于诊断的快速修复
//! - 代码重构建议

use lsp_types::{CodeAction, CodeActionKind, CodeActionParams, Range};
use tracing::debug;

use crate::lsp::handlers::diagnostics;

/// 处理 `textDocument/codeAction` 请求
///
/// 返回基于当前上下文的代码操作列表：
/// 1. 诊断相关的快速修复
/// 2. 代码重构建议
pub fn handle_code_action(
    params: CodeActionParams,
    content: &str,
) -> Option<Vec<CodeAction>> {
    let uri = params.text_document.uri.as_str();

    // 运行诊断获取当前文件的诊断信息
    let diag_result = diagnostics::run_diagnostics(uri, content);
    let diagnostic_list = diag_result.diagnostics;

    let mut actions: Vec<CodeAction> = Vec::new();

    // 为每个诊断生成快速修复
    for diagnostic in &diagnostic_list {
        if let Some(fix) = generate_fix_for_diagnostic(diagnostic) {
            actions.push(fix);
        }
    }

    // 添加通用重构建议
    actions.extend(generate_refactor_actions(&params.range));

    if actions.is_empty() {
        None
    } else {
        debug!("生成 {} 个 code action", actions.len());
        Some(actions)
    }
}

/// 根据诊断信息生成快速修复
fn generate_fix_for_diagnostic(diagnostic: &lsp_types::Diagnostic) -> Option<CodeAction> {
    let message = diagnostic.message.as_str();

    // 根据错误消息生成快速修复
    let (title, _kind) = if message.contains("未定义的符号")
        || message.contains("undefined")
        || message.contains("未找到")
    {
        // 建议创建变量或导入
        ("创建变量".to_string(), "create_variable".to_string())
    } else if message.contains("类型不匹配") || message.contains("type mismatch") {
        // 建议类型转换
        ("添加类型转换".to_string(), "add_cast".to_string())
    } else if message.contains("未使用的变量")
        || message.contains("unused")
        || message.contains("未使用的")
    {
        // 建议删除或使用下划线前缀
        ("删除变量".to_string(), "remove_variable".to_string())
    } else {
        return None;
    };

    Some(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic.clone()]),
        edit: None,
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

/// 生成通用重构建议
fn generate_refactor_actions(range: &Range) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // 检查选区是否有效（开始和结束位置不同表示有选区）
    let has_selection = range.start != range.end;

    if has_selection {
        // 提取符号重构
        actions.push(CodeAction {
            title: "提取为变量".to_string(),
            kind: Some(CodeActionKind::REFACTOR_EXTRACT),
            diagnostics: None,
            edit: None,
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        });
    }

    // 内联重构（不依赖选区）
    actions.push(CodeAction {
        title: "内联变量".to_string(),
        kind: Some(CodeActionKind::REFACTOR_INLINE),
        diagnostics: None,
        edit: None,
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    });

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{CodeActionContext, Position, Range, TextDocumentIdentifier, Uri};
    use std::str::FromStr;

    fn make_params(uri: &str) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }
    }

    fn make_params_with_range(
        uri: &str,
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
    ) -> CodeActionParams {
        let mut params = make_params(uri);
        params.range = Range {
            start: Position {
                line: start_line,
                character: start_char,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        };
        params
    }

    #[test]
    fn test_code_action_no_selection() {
        let params = make_params("file:///test.yx");
        let result = handle_code_action(params, "x = 1");
        assert!(result.is_some());

        let actions = result.unwrap();
        // At least inline variable
        assert!(actions.iter().any(|a| a.title == "内联变量"));
    }

    #[test]
    fn test_code_action_with_selection() {
        let params = make_params_with_range("file:///test.yx", 0, 0, 0, 5);
        let result = handle_code_action(params, "x = 1 + 2");
        assert!(result.is_some());

        let actions = result.unwrap();
        assert!(actions.iter().any(|a| a.title == "提取为变量"));
        assert!(actions.iter().any(|a| a.title == "内联变量"));
    }
}
