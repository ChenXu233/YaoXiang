//! Standard Network library (YaoXiang)
//!
//! This module provides network-related functionality for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// ============================================================================
// NetModule - StdModule Implementation
// ============================================================================

/// Net module implementation.
pub struct NetModule;

impl Default for NetModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for NetModule {
    fn module_path(&self) -> &str {
        "std.net"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "http_get",
                "std.net.http_get",
                "(url: String) -> String",
                native_http_get,
            ),
            NativeExport::new(
                "http_post",
                "std.net.http_post",
                "(url: String, body: String) -> String",
                native_http_post,
            ),
            NativeExport::new(
                "url_encode",
                "std.net.url_encode",
                "(s: String) -> String",
                native_url_encode,
            ),
            NativeExport::new(
                "url_decode",
                "std.net.url_decode",
                "(s: String) -> String",
                native_url_decode,
            ),
        ]
    }
}

/// Singleton instance for std.net module.
pub const NET_MODULE: NetModule = NetModule;

// ============================================================================
// Network Functions
// ============================================================================

/// Native implementation: http_get
fn native_http_get(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "http_get expects 1 argument (url: String)".to_string(),
        ));
    }

    let url = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "http_get expects String argument, got {:?}",
                other
            )))
        }
    };

    Ok(RuntimeValue::String(format!("GET: {}", url).into()))
}

/// Native implementation: http_post
fn native_http_post(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::Runtime(
            "http_post expects 2 arguments (url: String, body: String)".to_string(),
        ));
    }

    let url = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "http_post expects String argument for url, got {:?}",
                other
            )))
        }
    };

    let body = match &args[1] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "http_post expects String argument for body, got {:?}",
                other
            )))
        }
    };

    Ok(RuntimeValue::String(
        format!("POST {}: {}", url, body).into(),
    ))
}

/// Native implementation: url_encode
fn native_url_encode(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "url_encode expects 1 argument (s: String)".to_string(),
        ));
    }

    let s = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "url_encode expects String argument, got {:?}",
                other
            )))
        }
    };

    let encoded = urlencoding::encode(&s).to_string();
    Ok(RuntimeValue::String(encoded.into()))
}

/// Native implementation: url_decode
fn native_url_decode(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::Runtime(
            "url_decode expects 1 argument (s: String)".to_string(),
        ));
    }

    let s = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::Type(format!(
                "url_decode expects String argument, got {:?}",
                other
            )))
        }
    };

    match urlencoding::decode(&s) {
        Ok(decoded) => Ok(RuntimeValue::String(decoded.to_string().into())),
        Err(e) => Err(ExecutorError::Runtime(format!("url_decode failed: {}", e))),
    }
}
