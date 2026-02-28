//! Initialize / Shutdown / Exit 处理
//!
//! LSP 生命周期方法实现。

use lsp_server::Response;
use lsp_types::{InitializeParams, InitializeResult, ServerInfo};
use tracing::info;

use crate::lsp::capabilities::server_capabilities;
use crate::lsp::protocol::{self, SERVER_NAME, SERVER_VERSION};
use crate::lsp::session::{Session, SessionState};

/// 处理 `initialize` 请求
///
/// 返回服务端能力声明和服务器信息。
/// 调用后会话进入 `Initializing` 状态。
pub fn handle_initialize(
    session: &mut Session,
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

#[cfg(test)]
mod tests {
    use super::*;
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
        let params = make_init_params(Some("file:///workspace/project"));

        let resp = handle_initialize(&mut session, 1.into(), params);

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
        let params = make_init_params(None);

        let resp = handle_initialize(&mut session, 1.into(), params);
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
}
