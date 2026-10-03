//! `std.net` 模块的单元测试
//!
//! 覆盖范围与依据：
//! - HTTP 请求为**真实实现**（#56）：基于 `ureq`（同步阻塞 + rustls TLS）；
//! - 响应为结构化字典 `{status: Int, headers: Dict<String,String>, body: String}`；
//! - **4xx/5xx 是合法响应而非传输错误**（携带状态码与响应体）；
//! - 响应头名大小写不敏感；请求头按传入原样发送；POST 体按原样发送；
//! - 「非正数 timeout_secs」报运行时错误、参数类型不符报类型错误。
//!
//! 规范来源：`std.net` 无独立 RFC（`docs/src/design/rfc/` 内无 std.net 专章），
//! 故断言对象为上述实现契约，其出处是 `src/std/net.rs` 模块头注释（#56 真实实现 +
//! 响应字典结构）与 `native_http_get` / `response_to_value` 的文档注释；错误码段位依据
//! RFC-013（`013-error-code-specification.md:560`「E7xxx 为 std.io / std.net 错误值预留段位」）；
//! 阻塞式调用只阻塞当前 worker 的并发前提见 RFC-008（`008-runtime-concurrency-model.md`）。
//!
//! 迁移自 `src/std/net.rs` 的内联 `mod tests`（违反 `docs/src/dev/test-specification.md` 规则 1.4），
//! 现按规则 1.1「单文件模块 → 父级 tests/」落位于此，由 `src/std/mod.rs` 的 `#[cfg(test)] mod tests;` 声明。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;

use crate::backends::common::{Heap, HeapValue, RuntimeValue};
use crate::backends::ExecutorError;
use crate::std::net::{native_http_get, native_http_post};
use crate::std::NativeContext;

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

/// 用 args 调 http_get，并**要求**它被拒绝；返回错误以便断言具体变体。
fn get_rejected(
    args: &[RuntimeValue],
    ctx: &mut NativeContext<'_>,
) -> ExecutorError {
    match native_http_get(args, ctx) {
        Ok(v) => panic!("http_get should reject the given arguments, got {v:?}"),
        Err(e) => e,
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
    let (url, server) = serve_once("HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n".to_string());
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
    let url = RuntimeValue::String("http://127.0.0.1:1/".into());

    let err = get_rejected(&[RuntimeValue::Int(1)], &mut ctx);
    assert!(matches!(err, ExecutorError::Type(_, _)), "url type error");

    let err = get_rejected(&[url.clone(), RuntimeValue::Int(5)], &mut ctx);
    assert!(
        matches!(err, ExecutorError::Type(_, _)),
        "headers type error"
    );

    let err = get_rejected(&[url, RuntimeValue::Void, RuntimeValue::Int(0)], &mut ctx);
    assert!(
        matches!(err, ExecutorError::Runtime(_, _)),
        "timeout > 0 guard"
    );
}
