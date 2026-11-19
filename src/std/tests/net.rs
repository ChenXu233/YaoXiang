//! `std.net` 模块的单元测试
//!
//! 覆盖范围与依据：
//! - HTTP 请求为**真实实现**（#56）：基于 `ureq` 3（同步阻塞 + rustls TLS；#489 从 2.x 迁移）；
//! - 响应为结构化字典 `{status: Int, headers: Dict<String,String>, body: String}`；
//! - **4xx/5xx 是合法响应而非传输错误**（携带状态码与响应体）；
//! - 响应头名统一小写（HTTP/1.1 语义本就不区分大小写；ureq 3 经 http crate 归一，
//!   2.x 原样保留头名大小写——#489 对账差异，语义等价）；请求头值按传入原样发送；
//!   POST 体按原样发送；
//! - 「非正数 timeout_secs」报运行时错误、参数类型不符报类型错误；
//! - **环境代理变量被显式禁用**（HTTP_PROXY 注入假代理地址时 http_get 仍直连本地目标）。
//!   代理变量是进程级的，ureq agent 每次请求都重新构造——进程内 set_var 会污染同进程
//!   并行运行的其它测试，这两个用例走「重新拉起本测试可执行文件、只在子进程注入环境」
//!   的探针形态（先例：`src/package/source/tests/git.rs` 的 GIT_DIR 重拉形态）。
//!
//! 规范来源：`std.net` 无独立 RFC（`docs/src/rfc/` 内无 std.net 专章），
//! 故断言对象为上述实现契约，其出处是 `src/std/net.rs` 模块头注释（#56 真实实现 +
//! 响应字典结构）与 `native_http_get` / `response_to_value` 的文档注释；用户可见契约
//! 的文档条款在 `docs/src/reference/stdlib/net.md`（「4xx/5xx 是正常响应」）；
//! 错误码段位依据 RFC-013（`013-error-code-specification.md:560`「E7xxx 为 std.io /
//! std.net 错误值预留段位」）；阻塞式调用只阻塞当前 worker 的并发前提见 RFC-008
//! （`008-runtime-concurrency-model.md`）。
//!
//! 迁移自 `src/std/net.rs` 的内联 `mod tests`（违反 `docs/src/dev/test-specification.md` 规则 1.4），
//! 现按规则 1.1「单文件模块 → 父级 tests/」落位于此，由 `src/std/mod.rs` 的 `#[cfg(test)] mod tests;` 声明。

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

/// 探针环境变量：标记子进程为代理探针形态。
const PROXY_PROBE_ENV: &str = "YX_NET_PROXY_PROBE";

/// 环境代理变量全集：ureq `Proxy::try_from_env` 的读取面（ALL_PROXY/HTTPS_PROXY/
/// HTTP_PROXY 及小写形态）+ NO_PROXY 旁路面。
const PROXY_ENV_VARS: [&str; 8] = [
    "ALL_PROXY",
    "all_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// 重新拉起本测试可执行文件（只跑 `probe_test` 一个用例），先清掉全部代理变量，
/// 再按 `http_proxy` 决定是否注入 HTTP_PROXY。
fn relaunch_self_with_proxy_env(
    http_proxy: Option<&str>,
    probe_test: &str,
) -> std::process::Output {
    let exe = std::env::current_exe().expect("测试可执行文件路径");
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--exact").arg(probe_test).env(PROXY_PROBE_ENV, "1");
    for var in PROXY_ENV_VARS {
        cmd.env_remove(var);
    }
    if let Some(value) = http_proxy {
        cmd.env("HTTP_PROXY", value);
    }
    cmd.output().expect("重新拉起测试进程")
}

/// Assert: 探针必须真的跑到了本用例并通过，否则探针形同虚设。
fn assert_probe_child_passed(output: &std::process::Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "探针进程应通过其 1 个用例；stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_http_get_returns_status_headers_body() {
    // Arrange：本地单次请求服务器返回 200 + 自定义头 + 5 字节体
    let (url, server) = serve_once(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Tag: hello\r\nContent-Length: 5\r\n\r\nhello"
            .to_string(),
    );

    // Act：GET 该 URL
    let resp = call_get(&url, None).expect("http_get should succeed");

    // Assert：状态码 / 响应体 / 响应头（名字大小写不敏感）
    assert_eq!(int_of(dict_entry(&resp, "status")), 200);
    assert_eq!(string_of(dict_entry(&resp, "body")), "hello");
    assert_eq!(read_header(&resp, "content-type"), "text/plain");
    assert_eq!(read_header(&resp, "x-tag"), "hello");
    server.join().expect("server thread");
}

#[test]
fn test_http_post_sends_body() {
    // Arrange：本地服务器 + 独立 Heap/NativeContext 与实参表
    let (url, server) = serve_once("HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n".to_string());
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);
    let args = vec![
        RuntimeValue::String(url.into()),
        RuntimeValue::String("payload=1".into()),
    ];

    // Act：POST 带体请求
    let resp = native_http_post(&args, &mut ctx).expect("http_post should succeed");

    // Assert：201 是合法响应
    assert_eq!(int_of(dict_entry(&resp, "status")), 201);

    // Act + Assert：服务器收到的原始请求文本（请求行 + 请求体）
    let request = server.join().expect("server thread");
    assert!(
        request.starts_with("POST /data HTTP/1.1"),
        "request line: {request}"
    );
    assert!(request.ends_with("payload=1"), "request body: {request}");
}

#[test]
fn test_http_get_returns_error_status_with_body() {
    // Arrange：本地单次请求服务器返回 404 + 响应头 + 响应体
    // （契约出处：docs/src/reference/stdlib/net.md「4xx/5xx 是正常响应，不是错误」；
    //   #489 迁移后由 ureq_agent 的 http_status_as_error(false) 兑现——检查准入按
    //   coding-rules 7.2 条款门，防「错误状态被当传输错误吞掉」回归）
    let (url, server) = serve_once(
        "HTTP/1.1 404 Not Found\r\nX-Reason: gone\r\nContent-Length: 7\r\n\r\nmissing".to_string(),
    );

    // Act：GET 该 URL
    let resp = call_get(&url, None).expect("4xx/5xx must return a response dict, not an error");

    // Assert：状态码、响应体、响应头按结构化字典携带（而非 Err 吞掉响应）
    assert_eq!(int_of(dict_entry(&resp, "status")), 404);
    assert_eq!(string_of(dict_entry(&resp, "body")), "missing");
    assert_eq!(read_header(&resp, "x-reason"), "gone");
    server.join().expect("server thread");
}

#[test]
fn test_http_get_timeout_errors() {
    // Arrange：服务器故意 3 秒后才响应，客户端超时设为 1 秒
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

    // Act：带 1 秒超时调用 GET
    let result = native_http_get(&args, &mut ctx);

    // Assert：超时必须报错而不是挂死
    assert!(result.is_err(), "timeout must produce an error");
    server.join().expect("server thread");
}

#[test]
fn test_http_get_rejects_bad_argument_types() {
    // Arrange：独立上下文 + 一个合法 url 值
    let mut heap = Heap::new();
    let mut ctx = NativeContext::new(&mut heap);
    let url = RuntimeValue::String("http://127.0.0.1:1/".into());

    // Act + Assert：url 类型不符 → Type
    let err = get_rejected(&[RuntimeValue::Int(1)], &mut ctx);
    assert!(matches!(err, ExecutorError::Type(_, _)), "url type error");

    // Act + Assert：headers 类型不符 → Type
    let err = get_rejected(&[url.clone(), RuntimeValue::Int(5)], &mut ctx);
    assert!(
        matches!(err, ExecutorError::Type(_, _)),
        "headers type error"
    );

    // Act + Assert：timeout 非正数 → Runtime
    let err = get_rejected(&[url, RuntimeValue::Void, RuntimeValue::Int(0)], &mut ctx);
    assert!(
        matches!(err, ExecutorError::Runtime(_, _)),
        "timeout > 0 guard"
    );
}

#[test]
fn test_http_get_ignores_http_proxy_env_var() {
    // Arrange：HTTP_PROXY 注入必失败的假代理地址（127.0.0.1:1 无监听）。
    // 契约：std.net 的 agent 显式禁用环境代理（对位 ureq 2 未启用 proxy-from-env
    // 的不读默认，#489 迁移对账）——若读了环境代理，请求会改道假代理而必败。
    if std::env::var_os(PROXY_PROBE_ENV).is_none() {
        // Act：重新拉起自身，只在子进程注入假代理
        let output = relaunch_self_with_proxy_env(
            Some("http://127.0.0.1:1"),
            "std::tests::net::test_http_get_ignores_http_proxy_env_var",
        );

        // Assert
        assert_probe_child_passed(&output);
        return;
    }

    // 子进程形态：本地真目标已在监听，http_get 必须直连它成功
    let (url, server) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_string());

    // Act
    let resp = call_get(&url, None)
        .expect("HTTP_PROXY 指向无监听地址时 http_get 仍应直连本地目标（不读环境代理）");

    // Assert
    assert_eq!(int_of(dict_entry(&resp, "status")), 200);
    server.join().expect("server thread");
}

#[test]
fn test_http_get_direct_connection_without_proxy_env() {
    // Arrange：对照形态——清空全部代理变量后，直连行为不变。
    if std::env::var_os(PROXY_PROBE_ENV).is_none() {
        // Act
        let output = relaunch_self_with_proxy_env(
            None,
            "std::tests::net::test_http_get_direct_connection_without_proxy_env",
        );

        // Assert
        assert_probe_child_passed(&output);
        return;
    }

    let (url, server) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_string());

    // Act
    let resp = call_get(&url, None).expect("清空代理环境变量后 http_get 应直连本地目标");

    // Assert
    assert_eq!(int_of(dict_entry(&resp, "status")), 200);
    server.join().expect("server thread");
}
