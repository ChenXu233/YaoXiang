//! LSP 协议辅助工具测试
//!
//! 测试覆盖：
//! - 成功响应构建
//! - 错误响应构建
//! - 方法未找到响应
//! - 内部错误响应
//! - 服务器信息常量

use lsp_server::{ErrorCode, Response, ResponseError};
use lsp_types::notification::Notification;
use serde::Serialize;

use crate::lsp::protocol::{ok_response, error_response, method_not_found, internal_error, SERVER_NAME, SERVER_VERSION};

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
