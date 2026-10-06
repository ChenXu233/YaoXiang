//! 悬停提示处理器测试
//!
//! 语义数据由真实 typecheck 管线产出（见 support 模块），遵守静默通道判定
//! （RFC-039 D55 / coding-rules 第六部分）：禁止手工构造 definitions/references。
//!
//! 测试覆盖：
//! - 变量悬停显示推断类型
//! - 非标识符位置
//! - 引用点悬停（数据链经 resolve_reference 闭合）
//! - 文件来源信息
//! - 未打开文档

use lsp_types::{HoverContents, HoverParams, Uri};
use std::str::FromStr;

use super::support::{SAMPLE_URI, open_and_check};
use crate::lsp::handlers::hover::handle_hover;
use crate::lsp::session::Session;
use crate::lsp::world::World;

use lsp_types::{TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams};

fn make_params(
    uri: &str,
    line: u32,
    character: u32,
) -> HoverParams {
    HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    }
}

/// 样例：第二行 `y = x + x` 对 x 有两个引用点
fn setup() -> (Session, World) {
    open_and_check("x = 1\ny = x + x\n")
}

#[test]
fn test_hover_on_reference_shows_binding_type() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在第二行 x 引用上（0-indexed line=1, character=4）
    let params = make_params(SAMPLE_URI, 1, 4);
    let result = handle_hover(&session, &world, params);
    // Assert：悬停内容包含变量名与绑定类型
    let hover = result.expect("x 引用点必须给出悬停");
    let HoverContents::Markup(markup) = &hover.contents else {
        panic!("应返回 Markup 内容");
    };
    assert!(markup.value.contains("x"), "应包含变量名");
    // 类型文本取诊断层 MonoType Display 的既定形态（int64）
    assert!(markup.value.contains("int64"), "应显示 x 绑定的类型 int64");
}

#[test]
fn test_hover_includes_range() {
    // Arrange
    let (session, world) = setup();
    // Act
    let params = make_params(SAMPLE_URI, 1, 4);
    let result = handle_hover(&session, &world, params);
    // Assert
    let hover = result.expect("引用点应给出悬停");
    assert!(hover.range.is_some(), "应包含高亮 range");
}

#[test]
fn test_hover_shows_definition_source_file() {
    // Arrange
    let (session, world) = setup();
    // Act
    let params = make_params(SAMPLE_URI, 1, 4);
    let result = handle_hover(&session, &world, params);
    // Assert
    let hover = result.expect("引用点应给出悬停");
    let HoverContents::Markup(markup) = &hover.contents else {
        panic!("应返回 Markup 内容");
    };
    assert!(markup.value.contains("定义于"), "应包含文件来源信息");
}

#[test]
fn test_hover_not_on_identifier_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在 '=' 号上（0-indexed line=0, character=2）
    let params = make_params(SAMPLE_URI, 0, 2);
    let result = handle_hover(&session, &world, params);
    // Assert
    assert!(result.is_none(), "非标识符位置应返回 None");
}

#[test]
fn test_hover_doc_not_open_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：请求未打开的文档
    let params = make_params("file:///test/nope.yx", 0, 0);
    let result = handle_hover(&session, &world, params);
    // Assert
    assert!(result.is_none(), "未打开的文档应返回 None");
}
