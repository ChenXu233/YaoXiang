//! LSP 协议辅助工具
//!
//! JSON-RPC 消息构建、错误码定义和协议类型转换。

use lsp_server::{ErrorCode, Response};
use lsp_types::notification::Notification;
use serde::Serialize;

/// YaoXiang LSP 服务器基本信息
pub const SERVER_NAME: &str = "yaoxiang-lsp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 构建成功响应
pub fn ok_response<T: Serialize>(
    id: lsp_server::RequestId,
    result: T,
) -> Response {
    Response::new_ok(id, result)
}

/// 构建错误响应
pub fn error_response(
    id: lsp_server::RequestId,
    code: ErrorCode,
    message: String,
) -> Response {
    Response::new_err(id, code as i32, message)
}
/// 构建方法未找到的错误响应
pub fn method_not_found(
    id: lsp_server::RequestId,
    method: &str,
) -> Response {
    error_response(
        id,
        ErrorCode::MethodNotFound,
        format!("方法未实现: {}", method),
    )
}

/// 构建内部错误响应
pub fn internal_error(
    id: lsp_server::RequestId,
    message: String,
) -> Response {
    error_response(id, ErrorCode::InternalError, message)
}

/// 构建 LSP 通知消息
pub fn notification<N: Notification>(params: N::Params) -> lsp_server::Notification
where
    N::Params: Serialize,
{
    lsp_server::Notification {
        method: N::METHOD.to_string(),
        params: serde_json::to_value(params).unwrap(),
    }
}
