//! Initialize / Shutdown / Exit 处理器测试
//!
//! 测试覆盖：
//! - 初始化请求处理
//! - 无根路径初始化
//! - 初始化完成通知
//! - 关闭请求处理

use lsp_server::Response;
use lsp_types::{InitializeParams, InitializeResult, ServerInfo};

use crate::lsp::capabilities::server_capabilities;
use crate::lsp::handlers::initialize::{handle_initialize, handle_initialized, handle_shutdown};
use crate::lsp::protocol::{self, SERVER_NAME, SERVER_VERSION};
use crate::lsp::session::{Session, SessionState};
use crate::lsp::world::World;

use std::str::FromStr;

#[allow(deprecated)]
fn make_init_params(root: Option<&str>) -> InitializeParams {
    InitializeParams {
        root_uri: root.map(|r| lsp_types::Uri::from_str(r).unwrap()),
        capabilities: lsp_types::ClientCapabilities::default(),
        ..InitializeParams::default()
    }
}

#[test]
fn test_handle_initialize() {
    let mut session = Session::new();
    let mut world = World::new();
    let params = make_init_params(Some("file:///workspace/project"));

    let resp = handle_initialize(&mut session, &mut world, 1.into(), params);

    // 响应成功
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());

    // 会话进入 Initializing 状态
    assert_eq!(session.state(), SessionState::Initializing);

    // 工作区路径已设置
    assert_eq!(session.root_path(), Some("file:///workspace/project"));

    // 验证返回了服务器信息
    let result: InitializeResult = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert_eq!(result.server_info.as_ref().unwrap().name, SERVER_NAME);
    assert!(result.server_info.unwrap().version.is_some());
}

#[test]
fn test_handle_initialize_no_root() {
    let mut session = Session::new();
    let mut world = World::new();
    let params = make_init_params(None);

    let resp = handle_initialize(&mut session, &mut world, 1.into(), params);
    assert!(resp.error.is_none());
    assert!(session.root_path().is_none());
}

#[test]
fn test_handle_initialized() {
    let mut session = Session::new();
    session.set_state(SessionState::Initializing);

    handle_initialized(&mut session);

    assert_eq!(session.state(), SessionState::Running);
    assert!(session.is_ready());
}

#[test]
fn test_handle_shutdown() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);

    // 添加一些文档
    session
        .document_store_mut()
        .open("test.yx".to_string(), "x = 42".to_string(), 1);
    assert_eq!(session.document_store().document_count(), 1);

    let resp = handle_shutdown(&mut session, 1.into());

    // 响应成功
    assert!(resp.error.is_none());

    // 会话进入 ShuttingDown 状态
    assert!(session.is_shutting_down());

    // 文档缓存已清理
    assert_eq!(session.document_store().document_count(), 0);
}
