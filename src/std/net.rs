//! Standard Network library (YaoXiang)
//!
//! This module provides network-related functionality for YaoXiang programs.
//!
//! HTTP 请求为真实实现（#56）：基于 `ureq`（同步阻塞、rustls TLS），与运行时的
//! 显式并发模型（RFC-008/024 worker 线程池）自洽——阻塞调用只阻塞当前 worker。
//! 响应为结构化字典 `{status: Int, headers: Dict<String, String>, body: String}`。

use std::collections::HashMap;

use crate::backends::common::{RuntimeValue, HeapValue};
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

/// Native implementation: http_get
#[cfg(not(target_arch = "wasm32"))]
fn native_http_get(
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

    let mut request = ureq::get(&url).timeout(std::time::Duration::from_secs(timeout_secs));
    for (name, value) in &headers {
        request = request.set(name, value);
    }

    match request.call() {
        Ok(resp) => response_to_value(resp, ctx, "http_get", &url),
        // 4xx/5xx 是合法响应（携带状态码与响应体），不是传输错误
        Err(ureq::Error::Status(_, resp)) => response_to_value(resp, ctx, "http_get", &url),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "http_get failed for '{}': {}",
            url, e
        ))),
    }
}

/// Native implementation: http_post
#[cfg(not(target_arch = "wasm32"))]
fn native_http_post(
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

    let mut request = ureq::post(&url).timeout(std::time::Duration::from_secs(timeout_secs));
    for (name, value) in &headers {
        request = request.set(name, value);
    }

    match request.send_string(&body) {
        Ok(resp) => response_to_value(resp, ctx, "http_post", &url),
        Err(ureq::Error::Status(_, resp)) => response_to_value(resp, ctx, "http_post", &url),
        Err(e) => Err(ExecutorError::runtime_only(format!(
            "http_post failed for '{}': {}",
            url, e
        ))),
    }
}

/// 把 ureq 响应转成 yx 字典：`{status: Int, headers: Dict, body: String}`。
///
/// 响应头键统一小写（HTTP/1.1 语义本就不区分大小写）；同名多值以 ", " 合并。
#[cfg(not(target_arch = "wasm32"))]
fn response_to_value(
    resp: ureq::Response,
    ctx: &mut NativeContext<'_>,
    fname: &str,
    url: &str,
) -> Result<RuntimeValue, ExecutorError> {
    let status = RuntimeValue::Int(resp.status() as i64);

    let mut headers: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
    for name in resp.headers_names() {
        let joined = resp.all(&name).join(", ");
        headers.insert(
            RuntimeValue::String(name.into()),
            RuntimeValue::String(joined.into()),
        );
    }

    let body = match resp.into_string() {
        Ok(text) => RuntimeValue::String(text.into()),
        Err(e) => {
            return Err(ExecutorError::runtime_only(format!(
                "{}: failed reading response body of '{}': {}",
                fname, url, e
            )))
        }
    };

    let mut map: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
    map.insert(RuntimeValue::String("status".into()), status);
    map.insert(
        RuntimeValue::String("headers".into()),
        RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(headers))),
    );
    map.insert(RuntimeValue::String("body".into()), body);

    Ok(RuntimeValue::Dict(ctx.heap.allocate(HeapValue::Dict(map))))
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
    let RuntimeValue::Dict(handle) = value else {
        if matches!(value, RuntimeValue::Void) {
            return Ok(Vec::new());
        }
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
        let name = match key {
            RuntimeValue::String(s) => s.to_string(),
            other => {
                return Err(ExecutorError::type_only(format!(
                    "{} expects header names as String, got {:?}",
                    fname,
                    other.value_type(None)
                )))
            }
        };
        let value = match val {
            RuntimeValue::String(s) => s.to_string(),
            other => {
                return Err(ExecutorError::type_only(format!(
                    "{} expects header values as String, got {:?}",
                    fname,
                    other.value_type(None)
                )))
            }
        };
        out.push((name, value));
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::backends::common::heap::Heap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// 起一个只服务单次请求的本地 HTTP 服务器（不依赖外部网络）。
    /// 返回 (url, 捕获到的原始请求文本的 JoinHandle)。
    fn serve_once(response: String) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept one connection");
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = stream.read(&mut chunk).expect("read request");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Ok(text) = std::str::from_utf8(&buf) {
                    if let Some(header_end) = text.find("\r\n\r\n") {
                        let content_length = text
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())?
                            })
                            .unwrap_or(0);
                        if buf.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                if buf.len() > 1 << 20 {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&buf).to_string();
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            request
        });
        (format!("http://127.0.0.1:{port}/data"), handle)
    }

    fn call_get(
        url: &str,
        headers: Option<RuntimeValue>,
    ) -> Result<RuntimeValue, ExecutorError> {
        let mut heap = Heap::new();
        let mut ctx = NativeContext::new(&mut heap);
        let mut args = vec![RuntimeValue::String(url.into())];
        if let Some(h) = headers {
            args.push(h);
        }
        native_http_get(&args, &mut ctx)
    }

    fn dict_entry(
        response: &RuntimeValue,
        key: &str,
    ) -> RuntimeValue {
        let RuntimeValue::Dict(handle) = response else {
            panic!("response should be a Dict");
        };
        let guard = handle.lock();
        let map = match &*guard {
            HeapValue::Dict(map) => map,
            other => panic!("response heap value should be Dict, got {:?}", other),
        };
        map.get(&RuntimeValue::String(key.into()))
            .cloned()
            .unwrap_or_else(|| panic!("response missing key '{}'", key))
    }

    fn read_header(
        response: &RuntimeValue,
        name: &str,
    ) -> String {
        let headers = dict_entry(response, "headers");
        let RuntimeValue::Dict(handle) = headers else {
            panic!("headers should be a Dict");
        };
        let guard = handle.lock();
        match &*guard {
            HeapValue::Dict(map) => map
                .get(&RuntimeValue::String(name.into()))
                .cloned()
                .map(|v| match v {
                    RuntimeValue::String(s) => s.to_string(),
                    other => panic!("header value should be String, got {:?}", other),
                })
                .unwrap_or_else(|| panic!("missing response header '{}'", name)),
            other => panic!("headers heap value should be Dict, got {:?}", other),
        }
    }

    fn string_of(value: RuntimeValue) -> String {
        match value {
            RuntimeValue::String(s) => s.to_string(),
            other => panic!("expected String, got {:?}", other),
        }
    }

    fn int_of(value: RuntimeValue) -> i64 {
        match value {
            RuntimeValue::Int(n) => n,
            other => panic!("expected Int, got {:?}", other),
        }
    }

    fn alloc_dict(entries: Vec<(&str, &str)>) -> RuntimeValue {
        let mut heap = Heap::new();
        let mut map: HashMap<RuntimeValue, RuntimeValue> = HashMap::new();
        for (k, v) in entries {
            map.insert(
                RuntimeValue::String(k.into()),
                RuntimeValue::String(v.into()),
            );
        }
        RuntimeValue::Dict(heap.allocate(HeapValue::Dict(map)))
    }

    #[test]
    fn http_get_returns_status_headers_body() {
        let (url, server) = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Tag: hello\r\nContent-Length: 5\r\n\r\nhello"
                .to_string(),
        );
        let resp = call_get(&url, None).expect("http_get should succeed");
        assert_eq!(int_of(dict_entry(&resp, "status")), 200);
        assert_eq!(string_of(dict_entry(&resp, "body")), "hello");
        assert_eq!(read_header(&resp, "content-type"), "text/plain");
        assert_eq!(read_header(&resp, "x-tag"), "hello");
        server.join().expect("server thread");
    }

    #[test]
    fn http_get_sends_request_headers() {
        let (url, server) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_string());
        let headers = alloc_dict(vec![("Authorization", "Bearer token123")]);
        call_get(&url, Some(headers)).expect("http_get with headers");
        let request = server.join().expect("server thread");
        assert!(
            request.starts_with("GET /data HTTP/1.1"),
            "request line: {request}"
        );
        assert!(
            request.contains("Authorization: Bearer token123"),
            "request headers: {request}"
        );
    }

    #[test]
    fn http_post_sends_body() {
        let (url, server) =
            serve_once("HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n".to_string());
        let mut heap = Heap::new();
        let mut ctx = NativeContext::new(&mut heap);
        let args = vec![
            RuntimeValue::String(url.into()),
            RuntimeValue::String("payload=1".into()),
        ];
        let resp = native_http_post(&args, &mut ctx).expect("http_post should succeed");
        assert_eq!(int_of(dict_entry(&resp, "status")), 201);
        let request = server.join().expect("server thread");
        assert!(
            request.starts_with("POST /data HTTP/1.1"),
            "request line: {request}"
        );
        assert!(request.ends_with("payload=1"), "request body: {request}");
    }

    #[test]
    fn http_error_status_is_a_response_not_an_error() {
        let (url, server) = serve_once(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\r\nnope"
                .to_string(),
        );
        let resp = call_get(&url, None).expect("404 must return a response, not an error");
        assert_eq!(int_of(dict_entry(&resp, "status")), 404);
        assert_eq!(string_of(dict_entry(&resp, "body")), "nope");
        server.join().expect("server thread");
    }

    #[test]
    fn http_get_timeout_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        });
        let mut heap = Heap::new();
        let mut ctx = NativeContext::new(&mut heap);
        let args = vec![
            RuntimeValue::String(format!("http://127.0.0.1:{port}/slow").into()),
            RuntimeValue::Void,
            RuntimeValue::Int(1),
        ];
        let result = native_http_get(&args, &mut ctx);
        assert!(result.is_err(), "timeout must produce an error");
        server.join().expect("server thread");
    }

    #[test]
    fn http_get_rejects_bad_argument_types() {
        let mut heap = Heap::new();
        let mut ctx = NativeContext::new(&mut heap);

        let err = native_http_get(&[RuntimeValue::Int(1)], &mut ctx).unwrap_err();
        assert!(matches!(err, ExecutorError::Type(_, _)), "url type error");

        let err = native_http_get(
            &[
                RuntimeValue::String("http://127.0.0.1:1/".into()),
                RuntimeValue::Int(5),
            ],
            &mut ctx,
        )
        .unwrap_err();
        assert!(
            matches!(err, ExecutorError::Type(_, _)),
            "headers type error"
        );

        let err = native_http_get(
            &[
                RuntimeValue::String("http://127.0.0.1:1/".into()),
                RuntimeValue::Void,
                RuntimeValue::Int(0),
            ],
            &mut ctx,
        )
        .unwrap_err();
        assert!(
            matches!(err, ExecutorError::Runtime(_, _)),
            "timeout > 0 guard"
        );
    }
}
