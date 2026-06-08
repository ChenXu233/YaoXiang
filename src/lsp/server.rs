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
    PublishDiagnostics,
};
use lsp_types::request::{Completion, GotoDefinition, Initialize, References, Rename, Shutdown};
use lsp_types::request::HoverRequest;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::request::SemanticTokensFullDeltaRequest;
use lsp_types::request::SemanticTokensRefresh;
use lsp_types::request::Formatting;
use lsp_types::request::RangeFormatting;
use lsp_types::request::InlayHintRequest;
use lsp_types::InitializeParams;
use tracing::{debug, info, warn};

use std::sync::atomic::{AtomicI32, Ordering};

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

    // 加载标准库符号到索引
    world.load_std_library_symbols(None);
    // 加载内置类型到索引
    world.load_builtin_types();

    // 主消息循环
    main_loop(&connection, &mut session, &mut world)?;

    // 等待 IO 线程结束
    io_threads.join()?;

    info!("LSP 服务器已退出");
    Ok(())
}

/// 主消息循环
static OUTGOING_REQUEST_ID: AtomicI32 = AtomicI32::new(1);

fn request_semantic_tokens_refresh(connection: &Connection) {
    let id = OUTGOING_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let req = lsp_server::Request {
        id: id.into(),
        method: <SemanticTokensRefresh as lsp_types::request::Request>::METHOD.to_string(),
        params: serde_json::Value::Null,
    };
    if let Err(e) = connection.sender.send(Message::Request(req)) {
        warn!("failed to request semanticTokens refresh: {}", e);
    }
}

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
                if handle_notification(connection, session, world, not)? {
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
    world: &mut World,
    req: Request,
) -> Option<lsp_server::Response> {
    let method = req.method.as_str();
    info!("← 请求: {} (id={})", method, req.id);

    match method {
        // initialize
        m if m == <Initialize as lsp_types::request::Request>::METHOD => {
            let params: InitializeParams = serde_json::from_value(req.params).unwrap_or_default();
            Some(handlers::initialize::handle_initialize(
                session, world, req.id, params,
            ))
        }

        // shutdown
        m if m == <Shutdown as lsp_types::request::Request>::METHOD => {
            Some(handlers::initialize::handle_shutdown(session, req.id))
        }

        // textDocument/completion
        m if m == <Completion as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value::<lsp_types::CompletionParams>(req.params) {
                Ok(params) => {
                    let result = handlers::completion::handle_completion(session, world, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("补全请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/definition
        m if m == <GotoDefinition as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::definition::handle_definition(session, world, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("跳转定义请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/references
        m if m == <References as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::references::handle_references(session, world, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("查找引用请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/rename
        m if m == <Rename as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::rename::handle_rename(session, world, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("重命名请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/codeAction
        "textDocument/codeAction" => {
            match serde_json::from_value::<lsp_types::CodeActionParams>(req.params) {
                Ok(params) => {
                    // 获取文档内容
                    let uri = params.text_document.uri.as_str();
                    let content = session
                        .document_store()
                        .get(uri)
                        .map(|d| d.content())
                        .unwrap_or_default();
                    let result = handlers::code_action::handle_code_action(params, content);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("Code action 请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/hover
        m if m == <HoverRequest as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::hover::handle_hover(session, world, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("悬停提示请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/semanticTokens/full
        m if m == <SemanticTokensFullRequest as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value::<lsp_types::SemanticTokensParams>(req.params) {
                Ok(params) => {
                    let uri = params.text_document.uri.as_str();
                    let document_text = session.document_store().get(uri).map(|d| d.content());
                    let (db, cache) = world.semantic_db_and_cache();
                    let result = handlers::semantic_tokens::handle_semantic_tokens_full(
                        db,
                        cache,
                        document_text,
                        params,
                    );
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("语义 tokens 请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/semanticTokens/full/delta
        m if m == <SemanticTokensFullDeltaRequest as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value::<lsp_types::SemanticTokensDeltaParams>(req.params) {
                Ok(params) => {
                    let uri = params.text_document.uri.as_str();
                    let document_text = session.document_store().get(uri).map(|d| d.content());
                    let (db, cache) = world.semantic_db_and_cache();
                    let result = handlers::semantic_tokens::handle_semantic_tokens_full_delta(
                        db,
                        cache,
                        document_text,
                        params,
                    );
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("语义 tokens delta 请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/formatting
        m if m == <Formatting as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::formatting::handle_formatting(session, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("格式化请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/rangeFormatting
        m if m == <RangeFormatting as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::formatting::handle_range_formatting(session, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("范围格式化请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // textDocument/inlayHint
        m if m == <InlayHintRequest as lsp_types::request::Request>::METHOD => {
            match serde_json::from_value(req.params) {
                Ok(params) => {
                    let result = handlers::inlay_hint::handle_inlay_hint(session, params);
                    Some(protocol::ok_response(req.id, result))
                }
                Err(e) => {
                    warn!("InlayHint请求参数解析失败: {}", e);
                    Some(protocol::internal_error(
                        req.id,
                        format!("参数解析失败: {}", e),
                    ))
                }
            }
        }

        // workspace/symbol
        "workspace/symbol" => match serde_json::from_value(req.params) {
            Ok(params) => {
                let result = handlers::workspace_symbol::handle_workspace_symbol(world, params);
                Some(protocol::ok_response(req.id, result))
            }
            Err(e) => {
                warn!("工作区符号搜索请求参数解析失败: {}", e);
                Some(protocol::internal_error(
                    req.id,
                    format!("参数解析失败: {}", e),
                ))
            }
        },

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
    connection: &Connection,
    session: &mut Session,
    world: &mut World,
    not: Notification,
) -> Result<bool> {
    let method = not.method.as_str();

    match method {
        // initialized
        m if m == <Initialized as lsp_types::notification::Notification>::METHOD => {
            info!("← 通知: initialized");
            handlers::initialize::handle_initialized(session);
            request_semantic_tokens_refresh(connection);
        }

        // exit
        m if m == <Exit as lsp_types::notification::Notification>::METHOD => {
            info!("← 通知: exit");
            return Ok(true);
        }

        // textDocument/didOpen
        m if m == <DidOpenTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                let uri = handlers::text_document::handle_did_open(session, params);
                update_symbol_index(session, world, &uri);
                request_semantic_tokens_refresh(connection);
                publish_diagnostics_for_uri(connection, session, &uri);
            }
        }

        // textDocument/didChange
        m if m == <DidChangeTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                if let Some(uri) = handlers::text_document::handle_did_change(session, params) {
                    update_symbol_index(session, world, &uri);
                    request_semantic_tokens_refresh(connection);
                    publish_diagnostics_for_uri(connection, session, &uri);
                }
            }
        }

        // textDocument/didClose
        m if m == <DidCloseTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                let uri = handlers::text_document::handle_did_close(session, params);
                // 移除关闭文档的符号索引
                world.remove_file_symbols(&uri);
                // 清除关闭文档的诊断
                let clear_params = handlers::diagnostics::clear_diagnostics(&uri);
                let not = protocol::notification::<PublishDiagnostics>(clear_params);
                if let Err(e) = connection.sender.send(Message::Notification(not)) {
                    warn!("发送清除诊断失败: {}", e);
                }
                debug!("已清除诊断: {}", uri);
            }
        }

        // 忽略未知通知（LSP 规范允许）
        _ => {
            info!("← 通知(忽略): {}", method);
        }
    }

    Ok(false)
}

/// 更新指定文件的符号索引和语义数据库
///
/// 从 DocumentStore 获取文件内容，解析后更新 World 的符号索引。
/// 同时运行 typecheck 收集语义 token 并存入 SemanticDB。
fn update_symbol_index(
    session: &Session,
    world: &mut World,
    uri: &str,
) {
    if let Some(doc) = session.document_store().get(uri) {
        let tokens = match crate::frontend::core::lexer::tokenize(doc.content()) {
            Ok(t) => t,
            Err(_) => {
                // 词法错误时移除旧索引
                world.remove_file_symbols(uri);
                return;
            }
        };

        let parse_result = crate::frontend::core::parser::parse_with_recovery(&tokens);
        world.update_index_from_ast(uri, &parse_result.module);

        // 运行 typecheck 收集语义 tokens（使用 collect_all 模式以收集更多信息）
        let mut tc = crate::frontend::core::typecheck::TypeChecker::new(uri);
        let result = tc.check_module_collect_all(&parse_result.module);
        world.update_semantic_db(result.semantic_db);

        debug!(
            "已更新符号索引和语义数据库: {} ({} 个符号)",
            uri,
            world.symbol_count()
        );
    }
}

/// 对指定 URI 的文档运行诊断并发送 publishDiagnostics 通知
fn publish_diagnostics_for_uri(
    connection: &Connection,
    session: &Session,
    uri: &str,
) {
    if let Some(doc) = session.document_store().get(uri) {
        let params = handlers::diagnostics::run_diagnostics(uri, doc.content());
        let diag_count = params.diagnostics.len();

        let not = protocol::notification::<PublishDiagnostics>(params);
        if let Err(e) = connection.sender.send(Message::Notification(not)) {
            warn!("发送诊断失败: {}", e);
        }

        debug!("已发布 {} 条诊断: {}", diag_count, uri);
    } else {
        warn!("文档未找到，跳过诊断: {}", uri);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::session::SessionState;
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

        // 符号索引应包含 x 和 add
        assert!(
            world.symbol_count() >= 2,
            "应至少有 2 个符号，实际: {}",
            world.symbol_count()
        );

        // 无语法错误时也应收集到 semantic tokens
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

        let count_before = world.symbol_count();
        assert!(count_before > 0);

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

        assert_eq!(world.symbol_count(), 0, "关闭文档后符号应被移除");
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

        // 注册符号
        use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
        use crate::util::span::{Position, Span};
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Var {
                    name: "x".to_string(),
                    name_span: Span {
                        start: Position {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        end: Position {
                            line: 1,
                            column: 2,
                            offset: 1,
                        },
                    },
                    type_annotation: None,
                    initializer: None,
                    is_mut: false,
                },
                span: Span {
                    start: Position {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    end: Position {
                        line: 1,
                        column: 7,
                        offset: 6,
                    },
                },
            }],
            span: Span::dummy(),
        };
        world.update_index_from_ast("file:///test/main.yx", &module);

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

        // 注册符号以获得悬停信息
        use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind};
        use crate::util::span::{Position, Span};
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Var {
                    name: "x".to_string(),
                    name_span: Span {
                        start: Position {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                        end: Position {
                            line: 1,
                            column: 2,
                            offset: 1,
                        },
                    },
                    type_annotation: None,
                    initializer: None,
                    is_mut: false,
                },
                span: Span {
                    start: Position {
                        line: 1,
                        column: 1,
                        offset: 0,
                    },
                    end: Position {
                        line: 1,
                        column: 7,
                        offset: 6,
                    },
                },
            }],
            span: Span::dummy(),
        };
        world.update_index_from_ast("file:///test/main.yx", &module);

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
}
