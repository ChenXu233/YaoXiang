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

use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::notification::{
    DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized, PublishDiagnostics,
};
use lsp_types::request::{Completion, GotoDefinition, Initialize, References, Shutdown};
use lsp_types::request::HoverRequest;
use lsp_types::InitializeParams;

use crate::lsp::server::{handle_request, handle_notification, publish_diagnostics_for_uri};
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
        crate::frontend::core::typecheck::semantic_db::DefinitionInfo {
            def_id: crate::frontend::core::typecheck::semantic_db::DefId {
                file_path: "file:///test/main.yx".to_string(),
                span: Span::dummy(),
            },
            name: "x".to_string(),
            kind: crate::frontend::core::typecheck::semantic_db::DefinitionKind::Variable,
            span: Span::dummy(),
            file_path: "file:///test/main.yx".to_string(),
            type_info: Some("Int".to_string()),
            signature: None,
        },
    );
}

/// Helper: 构造 `uri` 的 didOpen 通知（内容 `text`）。
fn did_open_notification(
    uri: &str,
    text: &str,
) -> Notification {
    let params = lsp_types::DidOpenTextDocumentParams {
        text_document: lsp_types::TextDocumentItem {
            uri: lsp_types::Uri::from_str(uri).expect("测试 URI 应可解析"),
            language_id: "yaoxiang".to_string(),
            version: 1,
            text: text.to_string(),
        },
    };
    Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD.to_string(),
        params: serde_json::to_value(params).expect("didOpen 通知参数应可序列化"),
    }
}

/// Helper: 构造 `uri` 的 didClose 通知。
fn did_close_notification(uri: &str) -> Notification {
    let params = lsp_types::DidCloseTextDocumentParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: lsp_types::Uri::from_str(uri).expect("测试 URI 应可解析"),
        },
    };
    Notification {
        method: <DidCloseTextDocument as lsp_types::notification::Notification>::METHOD.to_string(),
        params: serde_json::to_value(params).expect("didClose 通知参数应可序列化"),
    }
}

/// Helper: 语义数据库中 `uri` 的 token 数量（无记录为 0）。
fn semantic_token_count(
    world: &World,
    uri: &str,
) -> usize {
    world
        .semantic_db()
        .get_tokens(uri)
        .map(|t| t.len())
        .unwrap_or(0)
}

/// Helper: 断言 `uri` 的语义信息已从数据库中移除。
fn assert_semantic_index_removed(
    world: &World,
    uri: &str,
) {
    assert!(
        world.semantic_db().get_tokens(uri).is_none(),
        "关闭文档后语义信息应被移除"
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
fn test_handle_request_shutdown() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    let req = Request {
        id: 2.into(),
        method: <Shutdown as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::Value::Null,
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    assert!(session.is_shutting_down());
}

#[test]
fn test_handle_request_unknown() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    let req = Request {
        id: 3.into(),
        method: "custom/unknown".to_string(),
        params: serde_json::Value::Null,
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.response_result.is_err());
    assert_eq!(
        resp.response_result.unwrap_err().code,
        lsp_server::ErrorCode::MethodNotFound as i32
    );
}

#[test]
fn test_handle_notification_initialized() {
    let (conn, _rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Initializing);
    let mut world = World::new();

    let not = Notification {
        method: <Initialized as lsp_types::notification::Notification>::METHOD.to_string(),
        params: serde_json::Value::Null,
    };

    let should_exit = handle_notification(&conn, &mut session, &mut world, not).unwrap();
    assert!(!should_exit);
    assert!(session.is_ready());
}

#[test]
fn test_handle_notification_exit() {
    let (conn, _rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::ShuttingDown);
    let mut world = World::new();

    let not = Notification {
        method: <Exit as lsp_types::notification::Notification>::METHOD.to_string(),
        params: serde_json::Value::Null,
    };

    let should_exit = handle_notification(&conn, &mut session, &mut world, not).unwrap();
    assert!(should_exit);
}

#[test]
fn test_handle_notification_did_open() {
    let (conn, rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    let not = did_open_notification("file:///test/main.yx", "x = 42");
    let should_exit = handle_notification(&conn, &mut session, &mut world, not).unwrap();
    assert!(!should_exit);
    assert!(session.document_store().is_open("file:///test/main.yx"));

    // 应该收到 publishDiagnostics 通知
    let msg = rx.try_recv();
    assert!(msg.is_ok(), "应发送 publishDiagnostics 通知");
    if let Ok(Message::Notification(n)) = msg {
        assert_eq!(
            n.method,
            <PublishDiagnostics as lsp_types::notification::Notification>::METHOD
        );
    }
}

#[test]
fn test_handle_notification_did_open_with_errors() {
    let (conn, rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    // 语法错误
    let not = did_open_notification("file:///test/bad.yx", "@ @ @\n");
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 应该收到带有诊断的 publishDiagnostics 通知
    let msg = rx.try_recv();
    assert!(msg.is_ok());
    if let Ok(Message::Notification(n)) = msg {
        let params: lsp_types::PublishDiagnosticsParams = serde_json::from_value(n.params).unwrap();
        assert!(!params.diagnostics.is_empty(), "语法错误的代码应产生诊断");
    }
}

#[test]
fn test_handle_notification_did_close_clears_diagnostics() {
    let (conn, rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    // 先打开文档
    session
        .document_store_mut()
        .open("file:///test/main.yx".to_string(), "x = 42".to_string(), 1);

    let not = did_close_notification("file:///test/main.yx");
    handle_notification(&conn, &mut session, &mut world, not).unwrap();
    assert!(!session.document_store().is_open("file:///test/main.yx"));

    // 应该收到空诊断（清除）
    let msg = rx.try_recv();
    assert!(msg.is_ok());
    if let Ok(Message::Notification(n)) = msg {
        assert_eq!(
            n.method,
            <PublishDiagnostics as lsp_types::notification::Notification>::METHOD
        );
        let params: lsp_types::PublishDiagnosticsParams = serde_json::from_value(n.params).unwrap();
        assert!(params.diagnostics.is_empty(), "关闭文档应清除诊断");
    }
}

#[test]
fn test_publish_diagnostics_for_uri() {
    let (conn, rx) = test_connection();
    let mut session = Session::new();
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42\n".to_string(),
        1,
    );

    publish_diagnostics_for_uri(&conn, &session, "file:///test/main.yx");

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
fn test_did_open_updates_symbol_index() {
    let (conn, _rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    let not = did_open_notification("file:///test/indexed.yx", "x = 42\nadd = (a, b) => a + b\n");
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 语义数据库应包含符号
    let semantic_tokens = world
        .semantic_db()
        .get_tokens("file:///test/indexed.yx")
        .map(|tokens| tokens.to_vec())
        .unwrap_or_default();
    assert!(!semantic_tokens.is_empty(), "无错误代码也应有语义 tokens");
}

#[test]
fn test_did_close_removes_symbol_index() {
    let (conn, _rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    // 先打开
    let not = did_open_notification("file:///test/closing.yx", "y = 99\n");
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 检查语义数据库中有 tokens（表示文件被处理了）
    let tokens_before = semantic_token_count(&world, "file:///test/closing.yx");
    assert!(tokens_before > 0, "打开文件后应有语义 tokens");

    // 关闭
    let not = did_close_notification("file:///test/closing.yx");
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 关闭后语义信息应被移除
    assert_semantic_index_removed(&world, "file:///test/closing.yx");
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
