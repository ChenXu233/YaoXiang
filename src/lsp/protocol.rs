//! LSP 协议辅助工具
//!
//! JSON-RPC 消息构建、错误码定义和协议类型转换。

use lsp_server::{ErrorCode, Response, ResponseError};
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
    Response {
        id,
        result: Some(serde_json::to_value(result).unwrap()),
        error: None,
    }
}

/// 构建错误响应
pub fn error_response(
    id: lsp_server::RequestId,
    code: ErrorCode,
    message: String,
) -> Response {
    Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message,
            data: None,
        }),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_response() {
        let resp = ok_response(1.into(), "hello");
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_error_response() {
        let resp = error_response(1.into(), ErrorCode::MethodNotFound, "not found".to_string());
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, ErrorCode::MethodNotFound as i32);
        assert_eq!(err.message, "not found");
    }

    #[test]
    fn test_method_not_found() {
        let resp = method_not_found(1.into(), "textDocument/foobar");
        let err = resp.error.unwrap();
        assert_eq!(err.code, ErrorCode::MethodNotFound as i32);
        assert!(err.message.contains("textDocument/foobar"));
    }

    #[test]
    fn test_internal_error() {
        let resp = internal_error(1.into(), "boom".to_string());
        let err = resp.error.unwrap();
        assert_eq!(err.code, ErrorCode::InternalError as i32);
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_server_info() {
        assert_eq!(SERVER_NAME, "yaoxiang-lsp");
        assert!(!SERVER_VERSION.is_empty());
    }
}
