//! Initialize / Shutdown / Exit 处理
//!
//! LSP 生命周期方法实现。

use lsp_server::Response;
use lsp_types::{InitializeParams, InitializeResult, ServerInfo};
use tracing::info;

use crate::lsp::capabilities::server_capabilities;
use crate::lsp::protocol::{self, SERVER_NAME, SERVER_VERSION};
use crate::lsp::session::{Session, SessionState};
use crate::lsp::world::World;

/// 处理 `initialize` 请求
///
/// 返回服务端能力声明和服务器信息。
/// 调用后会话进入 `Initializing` 状态。
pub fn handle_initialize(
    session: &mut Session,
    world: &mut World,
    id: lsp_server::RequestId,
    params: InitializeParams,
) -> Response {
    info!(
        "收到 initialize 请求, 客户端: {:?}",
        params.client_info.as_ref().map(|c| &c.name)
    );

    // 提取工作区根路径
    #[allow(deprecated)]
    let root_path = params
        .root_uri
        .as_ref()
        .map(|uri| uri.as_str().to_string())
        .or_else(|| {
            #[allow(deprecated)]
            params.root_path.clone()
        });

    // initialize 时主动清理一次运行时缓存，避免异常重连时残留旧状态。
    session.document_store_mut().clear();
    world.reset_for_new_session();

    session.set_root_path(root_path.clone());
    session.set_state(SessionState::Initializing);

    info!("工作区根路径: {:?}", root_path);

    let result = InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(ServerInfo {
            name: SERVER_NAME.to_string(),
            version: Some(SERVER_VERSION.to_string()),
        }),
    };

    protocol::ok_response(id, result)
}

/// 处理 `initialized` 通知
///
/// 客户端确认初始化完成，会话进入 `Running` 状态。
pub fn handle_initialized(session: &mut Session) {
    info!("收到 initialized 通知，LSP 服务器就绪");
    session.set_state(SessionState::Running);
}

/// 处理 `shutdown` 请求
///
/// 准备关闭：清理资源，会话进入 `ShuttingDown` 状态。
/// 返回 null 结果（LSP 规范要求）。
pub fn handle_shutdown(
    session: &mut Session,
    id: lsp_server::RequestId,
) -> Response {
    info!("收到 shutdown 请求，准备关闭");
    session.set_state(SessionState::ShuttingDown);

    // 清理文档缓存
    session.document_store_mut().clear();

    // shutdown 请求返回 null
    protocol::ok_response(id, serde_json::Value::Null)
}
