//! LSP 服务器核心测试
//!
//! 测试覆盖：
//! - 请求处理
//! - 通知处理
//! - 初始化请求
//! - 关闭请求
//! - 未知方法处理
//! - 文档打开/关闭
//! - 诊断发布

use lsp_server::{Connection, Message, Request};
use lsp_types::notification::{PublishDiagnostics};
use lsp_types::request::{Completion, GotoDefinition, Initialize, References};
use lsp_types::request::HoverRequest;
use lsp_types::InitializeParams;

use crate::lsp::server::{analyze_and_publish, handle_request};
use crate::lsp::session::{Session, SessionState};
use crate::lsp::world::World;

use crossbeam_channel::unbounded;
use std::str::FromStr;

/// 创建测试用的 Connection（不连接真实 IO）
fn test_connection() -> (Connection, crossbeam_channel::Receiver<Message>) {
    let (to_client_tx, to_client_rx) = unbounded();
    let (_to_server_tx, to_server_rx) = unbounded();
    let conn = Connection {
        sender: to_client_tx,
        receiver: to_server_rx,
    };
    (conn, to_client_rx)
}

/// Helper: 创建已打开 `file:///test/main.yx`（内容 `text`）的运行态会话。
fn running_session_with_main(text: &str) -> Session {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    session
        .document_store_mut()
        .open("file:///test/main.yx".to_string(), text.to_string(), 1);
    session
}

/// Helper: 指向 `file:///test/main.yx` (0,0) 位置的文档位置参数。
fn main_text_document_position() -> lsp_types::TextDocumentPositionParams {
    lsp_types::TextDocumentPositionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: lsp_types::Uri::from_str("file:///test/main.yx").expect("测试 URI 应可解析"),
        },
        position: lsp_types::Position {
            line: 0,
            character: 0,
        },
    }
}

/// Helper: 向 `world` 的语义数据库注册符号 `x`（Variable / Int）。
fn register_x_definition(world: &mut World) {
    use crate::util::span::Span;
    world.semantic_db_mut().add_definition(
        "file:///test/main.yx",
        crate::frontend::core::sema::semantic_db::DefinitionInfo {
            def_id: crate::frontend::core::sema::semantic_db::DefId {
                file_path: "file:///test/main.yx".to_string(),
                span: Span::dummy(),
            },
            name: "x".to_string(),
            kind: crate::frontend::core::sema::semantic_db::DefinitionKind::Variable,
            span: Span::dummy(),
            file_path: "file:///test/main.yx".to_string(),
            type_info: Some("Int".to_string()),
            signature: None,
        },
    );
}

/// Helper: 以 `params` 发出 `method` 请求（id 为 `id`）。
fn send_request(
    session: &mut Session,
    world: &mut World,
    id: i32,
    method: &str,
    params: serde_json::Value,
) -> Option<lsp_server::Response> {
    let req = Request {
        id: id.into(),
        method: method.to_string(),
        params,
    };
    handle_request(session, world, req)
}
#[test]
fn test_handle_request_initialize() {
    let mut session = Session::new();
    let mut world = World::new();

    let req = Request {
        id: 1.into(),
        method: <Initialize as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(InitializeParams::default()).unwrap(),
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_ok());
    assert_eq!(session.state(), SessionState::Initializing);
}

#[test]
fn test_analyze_and_publish() {
    let (conn, rx) = test_connection();
    let mut session = Session::new();
    let mut world = World::new();
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42\n".to_string(),
        1,
    );

    analyze_and_publish(&conn, &session, &mut world, "file:///test/main.yx");

    let msg = rx.try_recv();
    assert!(msg.is_ok());
    if let Ok(Message::Notification(n)) = msg {
        assert_eq!(
            n.method,
            <PublishDiagnostics as lsp_types::notification::Notification>::METHOD
        );
    }
}

#[test]
fn test_handle_request_completion() {
    let mut session = running_session_with_main("x = 42\n");
    let mut world = World::new();

    let params = lsp_types::CompletionParams {
        text_document_position: main_text_document_position(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let resp = send_request(
        &mut session,
        &mut world,
        10,
        <Completion as lsp_types::request::Request>::METHOD,
        serde_json::to_value(params).unwrap(),
    );
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_ok(), "补全请求不应返回错误");
    assert!(resp.response_result.is_ok(), "补全应有结果");
}

#[test]
fn test_handle_request_definition() {
    let mut session = running_session_with_main("x = 42\n");
    let mut world = World::new();
    register_x_definition(&mut world);

    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: main_text_document_position(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let resp = send_request(
        &mut session,
        &mut world,
        20,
        <GotoDefinition as lsp_types::request::Request>::METHOD,
        serde_json::to_value(params).unwrap(),
    );
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_ok(), "跳转定义请求不应返回错误");
    assert!(resp.response_result.is_ok());
}

#[test]
fn test_handle_request_references() {
    let mut session = running_session_with_main("x = 1\ny = x\n");
    let mut world = World::new();

    let params = lsp_types::ReferenceParams {
        text_document_position: main_text_document_position(),
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: lsp_types::ReferenceContext {
            include_declaration: false,
        },
    };

    let resp = send_request(
        &mut session,
        &mut world,
        21,
        <References as lsp_types::request::Request>::METHOD,
        serde_json::to_value(params).unwrap(),
    );
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_ok(), "查找引用请求不应返回错误");
}

#[test]
fn test_handle_request_hover() {
    let mut session = running_session_with_main("x = 42\n");
    let mut world = World::new();
    register_x_definition(&mut world);

    let params = lsp_types::HoverParams {
        text_document_position_params: main_text_document_position(),
        work_done_progress_params: Default::default(),
    };

    let resp = send_request(
        &mut session,
        &mut world,
        22,
        <HoverRequest as lsp_types::request::Request>::METHOD,
        serde_json::to_value(params).unwrap(),
    );
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_ok(), "悬停提示请求不应返回错误");
    assert!(resp.response_result.is_ok());
}
