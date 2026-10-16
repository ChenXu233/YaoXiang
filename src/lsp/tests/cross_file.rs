//! 跨文件跳转钉测试（WBS 4.6.1：LSP 语义数据管线统一）。
//!
//! 判据（09 验收「项目内跨文件跳转命中定义」）：项目内 `use lib.{add_one}`
//! 后调用点的 `add_one` 必须经 SemanticDB `resolves_to` 命中 lib.yx 的
//! 真实定义位置。旧手工管线（server.rs 的 lex→parse→单文件 typecheck）没有
//! 模块注册表，`use` 解析失败、引用事件不产生，跳转静默落空（D55 登记的
//! B 类静默失败——本测试在修复前必须红）。

use lsp_server::{Connection, Notification, Request};
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::request::GotoDefinition;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::lsp::server::{handle_notification, handle_request};
use crate::lsp::session::{Session, SessionState};
use crate::lsp::world::World;

/// 落一个带 manifest 的双文件项目：lib.yx 定义 `add_one`，main.yx 导入并调用。
/// 返回 (dir 守卫, main.yx 路径, lib.yx 路径)。
fn make_cross_file_project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\n",
    )
    .expect("write manifest failed");
    let lib = dir.path().join("lib.yx");
    std::fs::write(
        &lib,
        "add_one: (x: Int) -> Int = (x) => {\n    return x + 1\n}\n",
    )
    .expect("write lib.yx failed");
    let main = dir.path().join("main.yx");
    std::fs::write(&main, "use lib.{add_one}\n\ny = add_one(1)\n").expect("write main.yx failed");
    (dir, main, lib)
}

/// 磁盘路径转 file URI（与客户端发送形式一致：正斜杠、file:/// 前缀）。
fn file_uri(path: &Path) -> String {
    format!("file:///{}", path.display().to_string().replace('\\', "/"))
}

/// 创建测试用 Connection（不连接真实 IO）。
fn test_connection() -> Connection {
    let (to_client_tx, _to_client_rx) = crossbeam_channel::unbounded();
    let (_to_server_tx, to_server_rx) = crossbeam_channel::unbounded();
    Connection {
        sender: to_client_tx,
        receiver: to_server_rx,
    }
}

#[test]
fn test_definition_cross_file_import_hits_real_definition() {
    // Arrange - 磁盘项目 + didOpen main.yx（缓冲区与磁盘一致）
    let (_dir, main_path, lib_path) = make_cross_file_project();
    let main_uri = file_uri(&main_path);
    let main_src = std::fs::read_to_string(&main_path).expect("read main.yx");
    let conn = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();
    let open = Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD.to_string(),
        params: serde_json::to_value(lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: lsp_types::Uri::from_str(&main_uri).expect("main URI 应可解析"),
                language_id: "yaoxiang".to_string(),
                version: 1,
                text: main_src,
            },
        })
        .expect("didOpen 参数应可序列化"),
    };
    handle_notification(&conn, &mut session, &mut world, open).expect("didOpen 通知处理不应报错");

    // Act - 光标落在 `y = add_one(1)` 的 `add_one` 起点（0-indexed 行 2、列 4）：
    // resolve_reference 按引用**起点**精确匹配（D55 precise-only 现状口径，
    // 光标在标识符中部的容差不属于本任务）
    let req = Request {
        id: 1.into(),
        method: <GotoDefinition as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(lsp_types::GotoDefinitionParams {
            text_document_position_params: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: lsp_types::Uri::from_str(&main_uri).expect("main URI 应可解析"),
                },
                position: lsp_types::Position {
                    line: 2,
                    character: 4,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .expect("definition 参数应可序列化"),
    };
    let resp = handle_request(&mut session, &mut world, req).expect("definition 请求应有响应");
    let value = resp.response_result.expect("definition 请求不应返回错误");
    // handle_definition 的 None 经 Response::new_ok 序列化为 null
    let locations: Option<lsp_types::GotoDefinitionResponse> =
        serde_json::from_value(value).expect("definition 载荷应为 GotoDefinitionResponse 或 null");

    // Assert - 命中 lib.yx 的真实定义（行 0），不是落空、也不是 main.yx 的 use 行
    let response =
        locations.expect("跨文件导入符号的跳转不得落空（4.6.1 前为 None：旧手工管线无注册表）");
    let lsp_types::GotoDefinitionResponse::Scalar(location) = response else {
        panic!("跳转结果应为单个 Location，实际 {response:?}");
    };
    assert!(
        location.uri.as_str().ends_with("lib.yx"),
        "跨文件跳转必须命中定义所在文件 lib.yx，实际 {}",
        location.uri.as_str()
    );
    assert_eq!(
        location.range.start.line, 0,
        "add_one 定义在 lib.yx 第 0 行，实际 {:?}",
        location.range
    );
    // 钉板：跳转不得落在 use 行（main.yx 行 0 是 `use lib.{add_one}`）
    let _ = lib_path;
}
