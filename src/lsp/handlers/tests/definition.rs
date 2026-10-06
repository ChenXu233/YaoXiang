//! 跳转定义处理器测试
//!
//! 语义数据由真实 typecheck 管线产出（见 support 模块），遵守静默通道判定
//! （RFC-039 D55 / coding-rules 第六部分）：禁止手工构造 definitions/references。
//!
//! 测试覆盖：
//! - 引用点跳转到绑定定义
//! - 调用名跳转到函数绑定
//! - 定义点自身无引用记录（precise-only）
//! - 非标识符位置
//! - 未打开的文档

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Uri};
use std::str::FromStr;

use super::support::{SAMPLE_URI, open_and_check};
use crate::lsp::handlers::definition::handle_definition;
use crate::lsp::session::Session;
use crate::lsp::world::World;

use lsp_types::{
    PartialResultParams, TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};

fn make_params(
    uri: &str,
    line: u32,
    character: u32,
) -> GotoDefinitionParams {
    GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    }
}

/// 样例：第二行 `y = x + x` 对 x 有两个引用点，定义在第一行
fn setup() -> (Session, World) {
    open_and_check("x = 1\ny = x + x\n")
}

#[test]
fn test_definition_resolves_reference_to_binding() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在第二行第一个 x 引用上（0-indexed line=1, character=4）
    let params = make_params(SAMPLE_URI, 1, 4);
    let result = handle_definition(&session, &world, params);
    // Assert：跳回第一行 x 定义
    let response = result.expect("x 引用点必须跳转到定义（#433 数据链闭合）");
    match response {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.uri.to_string(), SAMPLE_URI, "定义在同一文件");
            assert_eq!(loc.range.start.line, 0, "x 定义在第一行（0-indexed）");
            assert_eq!(loc.range.start.character, 0, "x 定义在第 1 列");
        }
        _ => panic!("单一定义应返回 Scalar"),
    }
}

#[test]
fn test_definition_resolves_callee_to_function_binding() {
    // Arrange：调用点与函数绑定
    let (session, world) = open_and_check("add = (a, b) => a + b\ny = add(1, 2)\n");
    // Act：光标在第二行 add 调用名上（0-indexed line=1, character=4）
    let params = make_params(SAMPLE_URI, 1, 4);
    let result = handle_definition(&session, &world, params);
    // Assert：跳回第一行 add 绑定
    let response = result.expect("add 调用名必须跳转到其绑定");
    match response {
        GotoDefinitionResponse::Scalar(loc) => {
            assert_eq!(loc.range.start.line, 0, "add 绑定在第一行（0-indexed）");
        }
        _ => panic!("单一定义应返回 Scalar"),
    }
}

#[test]
fn test_definition_on_binding_site_returns_none_precise_only() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在第一行 x 定义名上——引用事件只登记使用点，
    // 定义点自身没有 reference 记录，precise-only（D55）不造按名兜底
    let params = make_params(SAMPLE_URI, 0, 0);
    let result = handle_definition(&session, &world, params);
    // Assert
    assert!(
        result.is_none(),
        "定义点自身无引用记录，precise-only 返回 None"
    );
}

#[test]
fn test_definition_not_on_identifier_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在 '=' 号上（0-indexed line=0, character=2）
    let params = make_params(SAMPLE_URI, 0, 2);
    let result = handle_definition(&session, &world, params);
    // Assert
    assert!(result.is_none(), "非标识符位置应返回 None");
}

#[test]
fn test_definition_doc_not_open_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：请求未打开的文档
    let params = make_params("file:///test/other.yx", 0, 0);
    let result = handle_definition(&session, &world, params);
    // Assert
    assert!(result.is_none(), "未打开的文档应返回 None");
}
