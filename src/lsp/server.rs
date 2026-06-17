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

    // 加载标准库符号到语义数据库
    world.load_std_symbols_to_semantic_db();
    // 加载内置类型到语义数据库
    world.load_builtin_types_to_semantic_db();

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
                update_semantic_db(session, world, &uri);
                request_semantic_tokens_refresh(connection);
                publish_diagnostics_for_uri(connection, session, &uri);
            }
        }

        // textDocument/didChange
        m if m == <DidChangeTextDocument as lsp_types::notification::Notification>::METHOD => {
            if let Ok(params) = serde_json::from_value(not.params) {
                if let Some(uri) = handlers::text_document::handle_did_change(session, params) {
                    update_semantic_db(session, world, &uri);
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
/// 从 DocumentStore 获取文件内容，解析后更新 World 的语义数据库。
fn update_semantic_db(
    session: &Session,
    world: &mut World,
    uri: &str,
) {
    if let Some(doc) = session.document_store().get(uri) {
        let tokens = match crate::frontend::core::lexer::tokenize(doc.content()) {
            Ok(t) => t,
            Err(_) => {
                // 词法错误时移除旧语义信息
                world.remove_file_symbols(uri);
                return;
            }
        };

        let parse_result = crate::frontend::core::parser::parse(&tokens);

        // 运行 typecheck 收集语义 tokens（使用 collect_all 模式以收集更多信息）
        let mut tc = crate::frontend::core::typecheck::TypeChecker::new(uri);
        let result = tc.check_module_collect_all(&parse_result.module);
        world.update_semantic_db(result.semantic_db);

        debug!("已更新语义数据库: {}", uri);
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
