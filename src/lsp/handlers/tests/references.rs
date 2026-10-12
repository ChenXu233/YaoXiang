//! 查找引用处理器测试
//!
//! 语义数据由真实 typecheck 管线产出（见 support 模块），遵守静默通道判定
//! （RFC-039 D55 / coding-rules 第六部分）：禁止手工构造 definitions/references。
//!
//! 测试覆盖：
//! - 排除声明的引用查找
//! - 包含声明的引用查找
//! - 非标识符位置
//! - 未打开文档

use lsp_types::{ReferenceParams};
use std::str::FromStr;

use super::support::{SAMPLE_URI, open_and_check};
use crate::lsp::handlers::references::handle_references;
use crate::lsp::session::Session;
use crate::lsp::world::World;

use lsp_types::{
    PartialResultParams, ReferenceContext, TextDocumentIdentifier, TextDocumentPositionParams,
    WorkDoneProgressParams, Uri,
};

fn make_params(
    uri: &str,
    line: u32,
    character: u32,
    include_declaration: bool,
) -> ReferenceParams {
    ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: Uri::from_str(uri).unwrap(),
            },
            position: lsp_types::Position { line, character },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext {
            include_declaration,
        },
    }
}

/// 样例：第二行 `y = x + x` 有两个 x 引用点，定义在第一行
fn setup() -> (Session, World) {
    open_and_check("x = 1\ny = x + x\n")
}

#[test]
fn test_references_excluding_declaration_finds_all_usages() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在第二行第一个 x 引用上，不含声明
    let params = make_params(SAMPLE_URI, 1, 4, false);
    let result = handle_references(&session, &world, params);
    // Assert：`y = x + x` 的两个使用点都在列，定义点不在列
    let locations = result.expect("x 引用点必须给出引用列表");
    assert_eq!(locations.len(), 2, "两个使用点都应被找到");
    assert!(
        locations.iter().all(|l| l.range.start.line == 1),
        "排除声明时只应包含第二行的使用点"
    );
}

#[test]
fn test_references_including_declaration_adds_definition_site() {
    // Arrange
    let (session, world) = setup();
    // Act：同上但包含声明
    let params = make_params(SAMPLE_URI, 1, 4, true);
    let result = handle_references(&session, &world, params);
    // Assert：两个使用点 + 一个定义点
    let locations = result.expect("x 引用点必须给出引用列表");
    assert_eq!(locations.len(), 3, "两个使用点加一个定义点");
    assert!(
        locations.iter().any(|l| l.range.start.line == 0),
        "定义点（第一行）应在列"
    );
}

#[test]
fn test_references_not_on_ident_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：光标在 '+' 号上（0-indexed line=1, character=6）
    let params = make_params(SAMPLE_URI, 1, 6, false);
    let result = handle_references(&session, &world, params);
    // Assert
    assert!(result.is_none(), "非标识符位置应返回 None");
}

#[test]
fn test_references_doc_not_open_returns_none() {
    // Arrange
    let (session, world) = setup();
    // Act：请求未打开的文档
    let params = make_params("file:///test/other.yx", 0, 0, false);
    let result = handle_references(&session, &world, params);
    // Assert
    assert!(result.is_none(), "未打开的文档应返回 None");
}
