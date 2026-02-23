//! LSP 服务器核心
//!
//! 实现 JSON-RPC 消息循环和请求分发。
//!
//! 架构：
//! ```text
//! stdin → Connection → main_loop → dispatch → handlers
//!                          ↓
//!                       Session (状态)
//!                       World   (编译)
//! ```

use anyhow::Result;
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized,
};
use lsp_types::request::{Initialize, Shutdown};
use lsp_types::InitializeParams;
use tracing::{info, warn};

use crate::lsp::handlers;
use crate::lsp::protocol;
use crate::lsp::session::Session;
use crate::lsp::world::World;

/// 启动 LSP 服务器
///
/// 通过 stdin/stdout 建立连接，处理 LSP 生命周期：
/// 1. 等待 `initialize` 请求
/// 2. 进入主消息循环
/// 3. 收到 `shutdown` 后等待 `exit`
pub fn run_lsp_server() -> Result<()> {
    info!("启动 YaoXiang LSP 服务器 v{}", protocol::SERVER_VERSION);

    // 创建 stdio 连接
    let (connection, io_threads) = Connection::stdio();

    // 创建会话和编译世界
    let mut session = Session::new();
    let mut world = World::new();

    // 主消息循环
    main_loop(&connection, &mut session, &mut world)?;

    // 等待 IO 线程结束
    io_threads.join()?;

    info!("LSP 服务器已退出");
    Ok(())
}

/// 主消息循环
fn main_loop(
    connection: &Connection,
    session: &mut Session,
    world: &mut World,
) -> Result<()> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                // 检查是否是 exit 之后的请求（不应该处理）
                if session.is_shutting_down() {
                    // shutdown 后只接受 exit 通知，忽略其他请求
                    warn!("shutdown 后收到请求，忽略: {}", req.method);
                    let resp = protocol::error_response(
                        req.id,
                        lsp_server::ErrorCode::InvalidRequest,
                        "服务器正在关闭".to_string(),
                    );
                    connection.sender.send(Message::Response(resp))?;
                    continue;
                }

                if let Some(resp) = handle_request(session, world, req) {
                    connection.sender.send(Message::Response(resp))?;
                }
            }
            Message::Notification(not) => {
                if handle_notification(session, world, not)? {
                    // exit 通知 → 退出循环
                    return Ok(());
                }
            }
            Message::Response(resp) => {
                // 我们目前不发送请求给客户端，忽略响应
                warn!("收到意外的响应: {:?}", resp.id);
            }
        }
    }

    Ok(())
}

/// 处理请求
///
/// 返回 `Some(Response)` 表示需要发送响应，`None` 表示已处理。
fn handle_request(
    session: &mut Session,
    _world: &mut World,
    req: Request,
) -> Option<lsp_server::Response> {
    let method = req.method.as_str();
    info!("← 请求: {} (id={})", method, req.id);

    match method {
        // initialize
        m if m == <Initialize as lsp_types::request::Request>::METHOD => {
            let params: InitializeParams = serde_json::from_value(req.params).unwrap_or_default();
            Some(handlers::initialize::handle_initialize(
                session, req.id, params,
            ))
        }

        // shutdown
        m if m == <Shutdown as lsp_types::request::Request>::METHOD => {
            Some(handlers::initialize::handle_shutdown(session, req.id))
        }

        // 未实现的方法
        _ => {
            warn!("未处理的请求方法: {}", method);
            Some(protocol::method_not_found(req.id, method))
        }
    }
}

/// 处理通知
///
/// 返回 `true` 表示应该退出服务器（收到 `exit` 通知）。
fn handle_notification(
    session: &mut Session,
    _world: &mut World,
    not: Notification,
) -> Result<bool> {
    let method = not.method.as_str();

    match method {
        // initialized
        m if m == <Initialized as lsp_types::notification::Notification>::METHOD => {
            info!("← 通知: initialized");
            handlers::initialize::handle_initialized(session);
        }

        // exit
        m if m == <Exit as lsp_types::notification::Notification>::METHOD => {
            info!("← 通知: exit");
            return Ok(true);
        }

        // textDocument/didOpen
        m if m == <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                handlers::text_document::handle_did_open(session, params);
            }
        }

        // textDocument/didChange
        m if m == <DidChangeTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                handlers::text_document::handle_did_change(session, params);
            }
        }

        // textDocument/didClose
        m if m == <DidCloseTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                handlers::text_document::handle_did_close(session, params);
            }
        }

        // 忽略未知通知（LSP 规范允许）
        _ => {
            info!("← 通知(忽略): {}", method);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::session::SessionState;
    use std::str::FromStr;

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
        let mut session = Session::new();
        session.set_state(SessionState::Initializing);
        let mut world = World::new();

        let not = Notification {
            method: <Initialized as lsp_types::notification::Notification>::METHOD.to_string(),
            params: serde_json::Value::Null,
        };

        let should_exit = handle_notification(&mut session, &mut world, not).unwrap();
        assert!(!should_exit);
        assert!(session.is_ready());
    }

    #[test]
    fn test_handle_notification_exit() {
        let mut session = Session::new();
        session.set_state(SessionState::ShuttingDown);
        let mut world = World::new();

        let not = Notification {
            method: <Exit as lsp_types::notification::Notification>::METHOD.to_string(),
            params: serde_json::Value::Null,
        };

        let should_exit = handle_notification(&mut session, &mut world, not).unwrap();
        assert!(should_exit);
    }

    #[test]
    fn test_handle_notification_did_open() {
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

        let should_exit = handle_notification(&mut session, &mut world, not).unwrap();
        assert!(!should_exit);
        assert!(session.document_store().is_open("file:///test/main.yx"));
    }
}
