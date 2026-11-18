//! Standard Network library (YaoXiang)
//!
//! This module provides network-related functionality for YaoXiang programs.
//!
//! HTTP 请求为真实实现（#56）：基于 `ureq` 3（同步阻塞、rustls TLS；#489 从 2.x 迁移，
//! feature "rustls" = rustls(ring) + webpki-roots 1.x 打包根证书），与运行时的
//! 显式并发模型（RFC-008/024 worker 线程池）自洽——阻塞调用只阻塞当前 worker。
//! 响应为结构化字典 `{status: Int, headers: Dict<String, String>, body: String}`。
//!
//! ureq 3 下的行为契约（与 2.x 迁移对账，#489）：
//! - Agent 以 `http_status_as_error(false)` 构建——4xx/5xx 作为正常 `Ok` 响应返回
//!   （携带状态码与响应体），不是传输错误；
//! - 超时用 `timeout_global`（DNS→读体全段一个预算，对位 2.x 的 `Request::timeout`）；
//! - 重定向上限显式设 5（对位 2.x 默认 `redirects: 5`；3.x 默认是 10）；
//! - 自定义 ConfigBuilder 链不读环境代理变量（对位 2.x 无 proxy-from-env feature 的默认）；
//! - 请求头名经 http crate 归一为小写写入报文（HTTP 语义大小写不敏感；2.x 原样保留），
//!   非法头名/头值在 `call()` 时报错（2.x 是延迟校验，同为 call 时报错）。

use std::collections::HashMap;

use crate::backends::common::{HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::{NativeContext, NativeExport, StdModule};

// NetModule - StdModule Implementation

/// Net module implementation.
#[derive(Default)]
pub struct NetModule;

impl StdModule for NetModule {
    fn module_path(&self) -> &str {
        "std.net"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            #[cfg(not(target_arch = "wasm32"))]
            export!(
                "http_get",
                "std.net.http_get",
                "(url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)",
                native_http_get
            ),
            #[cfg(not(target_arch = "wasm32"))]
            export!(
                "http_post",
                "std.net.http_post",
                "(url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)",
                native_http_post
            ),
            export!(
                "url_encode",
                "std.net.url_encode",
                "(s: &String) -> String",
                native_url_encode
            ),
            export!(
                "url_decode",
                "std.net.url_decode",
                "(s: &String) -> String",
                native_url_decode
            ),
        ]
    }
}

/// Singleton instance for std.net module.
pub const NET_MODULE: NetModule = NetModule;

// HTTP Functions

/// 默认整体请求超时（秒）。未传 `timeout_secs` 时生效。
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 重定向上限。ureq 2 默认 `redirects: 5`；ureq 3 默认 10——显式对齐 2.x 行为（#489）。
#[cfg(not(target_arch = "wasm32"))]
const UREQ_MAX_REDIRECTS: u32 = 5;

/// 按调用级超时与重定向上限构建 ureq Agent。
///
/// `http_status_as_error(false)`：4xx/5xx 作为正常响应返回（保留状态码与响应体），
/// 传输层故障仍走 `Err`。自定义 ConfigBuilder 链不拾取环境代理变量——与 ureq 2
/// （无 proxy-from-env feature）一致，不要改用 `Agent::new_with_defaults()`。
#[cfg(not(target_arch = "wasm32"))]
fn ureq_agent(timeout_secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(UREQ_MAX_REDIRECTS)
        .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
        .build()
        .into()
}

/// Native implementation: http_get
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_http_get(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "http_get expects 1..3 arguments (url: String, headers: Dict = {}, timeout_secs: Int = 30)"
                .to_string(),
        ));
    }

    let url = expect_string(&args[0], "http_get", "url")?;
    let headers = extract_request_headers(args.get(1), "http_get")?;
    let timeout_secs = extract_timeout_secs(args.get(2), "http_get")?;

    let mut request = ureq_agent(timeout_secs).get(&url);
    for (name, value) in &headers {
        request = request.header(name, value);
    }

    match request.call() {
        Ok(resp) => response_to_value(resp, ctx, "http_get", &url),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "http_get failed for '{}': {}",
            url, e
        ))),
    }
}

/// Native implementation: http_post
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn native_http_post(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.len() < 2 {
        return Err(ExecutorError::runtime_only(
            "http_post expects 2..4 arguments (url: String, body: String, headers: Dict = {}, timeout_secs: Int = 30)"
                .to_string(),
        ));
    }

    let url = expect_string(&args[0], "http_post", "url")?;
    let body = expect_string(&args[1], "http_post", "body")?;
    let headers = extract_request_headers(args.get(2), "http_post")?;
    let timeout_secs = extract_timeout_secs(args.get(3), "http_post")?;

    let mut request = ureq_agent(timeout_secs).post(&url);
    for (name, value) in &headers {
        request = request.header(name, value);
    }

    match request.send(body.as_str()) {
        Ok(resp) => response_to_value(resp, ctx, "http_post", &url),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "http_post failed for '{}': {}",
            url, e
        ))),
    }
}

/// 把 ureq 响应转成 yx 字典：`{status: Int, headers: Dict, body: String}`。
///
/// 响应头键统一小写（HTTP/1.1 语义本就不区分大小写；http crate 的头名本就是小写形态）；
/// 同名多值以 ", " 合并。
#[cfg(not(target_arch = "wasm32"))]
fn response_to_value(
    mut resp: ureq::http::Response<ureq::Body>,
    ctx: &mut NativeContext<'_>,
    fname: &str,
    url: &str,
) -> Result<RuntimeValue, ExecutorError> {
    let status = RuntimeValue::Int(resp.status().as_u16() as i64);
    // 先取头再读体：读体走 `body_mut()` 可变借用，与取头的共享借用分段。
    let headers = response_headers_to_map(&resp);
    let body = read_response_body(&mut resp, fname, url)?;

    let mut map: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
    map.insert(RuntimeValue::String("status".into()), status);
    map.insert(
        RuntimeValue::String("headers".into()),
        RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(headers))),
    );
    map.insert(RuntimeValue::String("body".into()), body);

    Ok(RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(map))))
}

/// 把响应头收集为 `{头名: 值}` 映射：头名按 HTTP/1.1 语义统一小写，同名多值以 `", "` 合并。
#[cfg(not(target_arch = "wasm32"))]
fn response_headers_to_map(
    resp: &ureq::http::Response<ureq::Body>
) -> HashMap<RuntimeValue, RuntimeValue> {
    let mut headers: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
    for name in resp.headers().keys() {
        // `keys()` 对同名多值头会重复给出名字，重算同一份合并结果无副作用。
        let joined = resp
            .headers()
            .get_all(name)
            .iter()
            .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
            .collect::<Vec<_>>()
            .join(", ");
        headers.insert(
            RuntimeValue::String(name.as_str().into()),
            RuntimeValue::String(joined.into()),
        );
    }
    headers
}

/// 读取响应体并包成 String 值；失败时报带 `fname`/`url` 上下文的运行时错误。
///
/// `read_to_string` 默认上限 10 MB、非法 UTF-8 按 lossy 替换——对位 ureq 2
/// `into_string` 的 10 MB 上限与 lossy 转换（#489）。
#[cfg(not(target_arch = "wasm32"))]
fn read_response_body(
    resp: &mut ureq::http::Response<ureq::Body>,
    fname: &str,
    url: &str,
) -> Result<RuntimeValue, ExecutorError> {
    match resp.body_mut().read_to_string() {
        Ok(text) => Ok(RuntimeValue::String(text.into())),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "{}: failed reading response body of '{}': {}",
            fname, url, e
        ))),
    }
}

/// 提取可选的请求头字典（Dict<String, String>）。
/// 缺省位置（None）与显式 Void 都视为空；其他非字典类型报类型错误。
#[cfg(not(target_arch = "wasm32"))]
fn extract_request_headers(
    arg: Option<&RuntimeValue>,
    fname: &str,
) -> Result<Vec<(String, String)>, ExecutorError> {
    let Some(value) = arg else {
        return Ok(Vec::new());
    };
    if matches!(value, RuntimeValue::Void) {
        return Ok(Vec::new());
    }
    headers_from_dict(value, fname)
}

/// 请求头名的取值：非 String 报类型错误（错误措辞含 `fname`）。
#[cfg(not(target_arch = "wasm32"))]
fn header_name(
    key: &RuntimeValue,
    fname: &str,
) -> Result<String, ExecutorError> {
    match key {
        RuntimeValue::String(s) => Ok(s.to_string()),
        other => Err(ExecutorError::type_only(format!(
            "{} expects header names as String, got {:?}",
            fname,
            other.value_type(None)
        ))),
    }
}

/// 请求头值的取值：非 String 报类型错误（错误措辞含 `fname`）。
#[cfg(not(target_arch = "wasm32"))]
fn header_value(
    val: &RuntimeValue,
    fname: &str,
) -> Result<String, ExecutorError> {
    match val {
        RuntimeValue::String(s) => Ok(s.to_string()),
        other => Err(ExecutorError::type_only(format!(
            "{} expects header values as String, got {:?}",
            fname,
            other.value_type(None)
        ))),
    }
}

/// 把 headers 字典展开为 `(名, 值)` 列表；任一项非 String 报类型错误，
/// 句柄悬垂报内部运行时错误。
#[cfg(not(target_arch = "wasm32"))]
fn headers_from_dict(
    value: &RuntimeValue,
    fname: &str,
) -> Result<Vec<(String, String)>, ExecutorError> {
    let RuntimeValue::Dict(handle) = value else {
        return Err(ExecutorError::type_only(format!(
            "{} expects headers as Dict<String, String>, got {:?}",
            fname,
            value.value_type(None)
        )));
    };

    let guard = handle.lock();
    let map = match &*guard {
        HeapValue::Dict(map) => map,
        _ => {
            return Err(ExecutorError::runtime_only(format!(
                "internal: dangling dict handle in {}.headers",
                fname
            )))
        }
    };

    let mut out = Vec::with_capacity(map.len());
    for (key, val) in map {
        out.push((header_name(key, fname)?, header_value(val, fname)?));
    }
    Ok(out)
}

/// 提取可选的整秒超时；缺省/显式 Void 用默认值，非正数报运行时错误。
#[cfg(not(target_arch = "wasm32"))]
fn extract_timeout_secs(
    arg: Option<&RuntimeValue>,
    fname: &str,
) -> Result<u64, ExecutorError> {
    let Some(value) = arg else {
        return Ok(DEFAULT_TIMEOUT_SECS);
    };
    match value {
        RuntimeValue::Void => Ok(DEFAULT_TIMEOUT_SECS),
        RuntimeValue::Int(secs) if *secs > 0 => Ok(*secs as u64),
        RuntimeValue::Int(secs) => Err(ExecutorError::runtime_only(format!(
            "{} expects timeout_secs > 0, got {}",
            fname, secs
        ))),
        other => Err(ExecutorError::type_only(format!(
            "{} expects timeout_secs as Int, got {:?}",
            fname,
            other.value_type(None)
        ))),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn expect_string(
    value: &RuntimeValue,
    fname: &str,
    param: &str,
) -> Result<String, ExecutorError> {
    match value {
        RuntimeValue::String(s) => Ok(s.to_string()),
        other => Err(ExecutorError::type_only(format!(
            "{} expects {} as String, got {:?}",
            fname,
            param,
            other.value_type(None)
        ))),
    }
}

// URL Functions

/// Native implementation: url_encode
fn native_url_encode(
    args: &[RuntimeValue],
    _ctx: &mut NativeContext<'_>,
) -> Result<RuntimeValue, ExecutorError> {
    if args.is_empty() {
        return Err(ExecutorError::runtime_only(
            "url_encode expects 1 argument (s: String)".to_string(),
        ));
    }

    let s = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
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
        return Err(ExecutorError::runtime_only(
            "url_decode expects 1 argument (s: String)".to_string(),
        ));
    }

    let s = match &args[0] {
        RuntimeValue::String(s) => s.to_string(),
        other => {
            return Err(ExecutorError::type_only(format!(
                "url_decode expects String argument, got {:?}",
                other
            )))
        }
    };

    match urlencoding::decode(&s) {
        Ok(decoded) => Ok(RuntimeValue::String(decoded.to_string().into())),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "url_decode failed: {}",
            e
        ))),
    }
}
