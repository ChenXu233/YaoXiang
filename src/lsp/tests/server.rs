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
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, GotoDefinition, Initialize, References, Rename, Shutdown};
use lsp_types::request::HoverRequest;
use lsp_types::InitializeParams;

use crate::lsp::handlers;
use crate::lsp::protocol;
use crate::lsp::server::{handle_request, handle_notification, publish_diagnostics_for_uri};
use crate::lsp::session::{Session, SessionState};
use crate::lsp::world::World;

use crossbeam::channel::unbounded;
use std::str::FromStr;

/// 创建测试用的 Connection（不连接真实 IO）
fn test_connection() -> (Connection, crossbeam::channel::Receiver<Message>) {
    let (to_client_tx, to_client_rx) = unbounded();
    let (_to_server_tx, to_server_rx) = unbounded();
    let conn = Connection {
        sender: to_client_tx,
        receiver: to_server_rx,
    };
    (conn, to_client_rx)
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
    assert!(resp.error.is_none());
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
    assert!(resp.error.is_some());
    assert_eq!(
        resp.error.unwrap().code,
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

    let params = lsp_types::DidOpenTextDocumentParams {
        text_document: lsp_types::TextDocumentItem {
            uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
            language_id: "yaoxiang".to_string(),
            version: 1,
            text: "x = 42".to_string(),
        },
    };

    let not = Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

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

    let params = lsp_types::DidOpenTextDocumentParams {
        text_document: lsp_types::TextDocumentItem {
            uri: lsp_types::Uri::from_str("file:///test/bad.yx").unwrap(),
            language_id: "yaoxiang".to_string(),
            version: 1,
            text: "@ @ @\n".to_string(), // 语法错误
        },
    };

    let not = Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 应该收到带有诊断的 publishDiagnostics 通知
    let msg = rx.try_recv();
    assert!(msg.is_ok());
    if let Ok(Message::Notification(n)) = msg {
        let params: lsp_types::PublishDiagnosticsParams =
            serde_json::from_value(n.params).unwrap();
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
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42".to_string(),
        1,
    );

    let params = lsp_types::DidCloseTextDocumentParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
        },
    };

    let not = Notification {
        method: <DidCloseTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

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
        let params: lsp_types::PublishDiagnosticsParams =
            serde_json::from_value(n.params).unwrap();
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
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42\n".to_string(),
        1,
    );
    let mut world = World::new();

    let params = lsp_types::CompletionParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
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

    let req = Request {
        id: 10.into(),
        method: <Completion as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.error.is_none(), "补全请求不应返回错误");
    assert!(resp.result.is_some(), "补全应有结果");
}

#[test]
fn test_did_open_updates_symbol_index() {
    let (conn, _rx) = test_connection();
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    let mut world = World::new();

    let params = lsp_types::DidOpenTextDocumentParams {
        text_document: lsp_types::TextDocumentItem {
            uri: lsp_types::Uri::from_str("file:///test/indexed.yx").unwrap(),
            language_id: "yaoxiang".to_string(),
            version: 1,
            text: "x = 42\nadd = (a, b) => a + b\n".to_string(),
        },
    };

    let not = Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

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
    let open_params = lsp_types::DidOpenTextDocumentParams {
        text_document: lsp_types::TextDocumentItem {
            uri: lsp_types::Uri::from_str("file:///test/closing.yx").unwrap(),
            language_id: "yaoxiang".to_string(),
            version: 1,
            text: "y = 99\n".to_string(),
        },
    };

    let not = Notification {
        method: <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(open_params).unwrap(),
    };
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 检查语义数据库中有 tokens（表示文件被处理了）
    let tokens_before = world
        .semantic_db()
        .get_tokens("file:///test/closing.yx")
        .map(|t| t.len())
        .unwrap_or(0);
    assert!(tokens_before > 0, "打开文件后应有语义 tokens");

    // 关闭
    let close_params = lsp_types::DidCloseTextDocumentParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: lsp_types::Uri::from_str("file:///test/closing.yx").unwrap(),
        },
    };

    let not = Notification {
        method: <DidCloseTextDocument as lsp_types::notification::Notification>::METHOD
            .to_string(),
        params: serde_json::to_value(close_params).unwrap(),
    };
    handle_notification(&conn, &mut session, &mut world, not).unwrap();

    // 关闭后语义信息应被移除
    assert!(
        world
            .semantic_db()
            .get_tokens("file:///test/closing.yx")
            .is_none(),
        "关闭文档后语义信息应被移除"
    );
}

#[test]
fn test_handle_request_definition() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42\n".to_string(),
        1,
    );
    let mut world = World::new();

    // 注册符号到语义数据库
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

    let params = lsp_types::GotoDefinitionParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
            },
            position: lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let req = Request {
        id: 20.into(),
        method: <GotoDefinition as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.error.is_none(), "跳转定义请求不应返回错误");
    assert!(resp.result.is_some());
}

#[test]
fn test_handle_request_references() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 1\ny = x\n".to_string(),
        1,
    );
    let mut world = World::new();

    let params = lsp_types::ReferenceParams {
        text_document_position: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
            },
            position: lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: lsp_types::ReferenceContext {
            include_declaration: false,
        },
    };

    let req = Request {
        id: 21.into(),
        method: <References as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.error.is_none(), "查找引用请求不应返回错误");
}

#[test]
fn test_handle_request_hover() {
    let mut session = Session::new();
    session.set_state(SessionState::Running);
    session.document_store_mut().open(
        "file:///test/main.yx".to_string(),
        "x = 42\n".to_string(),
        1,
    );
    let mut world = World::new();

    // 注册符号到语义数据库
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

    let params = lsp_types::HoverParams {
        text_document_position_params: lsp_types::TextDocumentPositionParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: lsp_types::Uri::from_str("file:///test/main.yx").unwrap(),
            },
            position: lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let req = Request {
        id: 22.into(),
        method: <HoverRequest as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::to_value(params).unwrap(),
    };

    let resp = handle_request(&mut session, &mut world, req);
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert!(resp.error.is_none(), "悬停提示请求不应返回错误");
    assert!(resp.result.is_some());
}
