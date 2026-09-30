//! 本地 mock HTTP 服务器（测试共享工具）
//!
//! 响应按序消费、逐连接关闭（`Connection: close` 保证请求-响应对齐）；
//! 记录每个请求的 path / method / If-None-Match 供断言。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// 一条预编排的 mock 响应
pub(crate) struct MockResp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl MockResp {
    pub fn json(
        status: u16,
        body: &str,
    ) -> Self {
        MockResp {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn bytes(
        status: u16,
        body: Vec<u8>,
    ) -> Self {
        MockResp {
            status,
            headers: Vec::new(),
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RecordedReq {
    pub method: String,
    pub path: String,
    pub if_none_match: Option<String>,
}

pub(crate) struct MockApi {
    pub url: String,
    requests: Arc<Mutex<Vec<RecordedReq>>>,
    pub handle: std::thread::JoinHandle<()>,
}

impl MockApi {
    /// 起本地 mock：每个响应服务一个连接
    pub fn spawn(responses: Vec<MockResp>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let handle = std::thread::spawn(move || {
            for resp in responses {
                let (mut stream, _) = match listener.accept() {
                    Ok(x) => x,
                    Err(_) => return,
                };
                let req_text = read_request(&mut stream);
                let mut lines = req_text.lines();
                let request_line = lines.next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or_default().to_string();
                let if_none_match = req_text.lines().find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("if-none-match")
                        .then(|| v.trim().to_string())
                });
                recorded.lock().unwrap().push(RecordedReq {
                    method,
                    path,
                    if_none_match,
                });

                let reason = match resp.status {
                    200 => "OK",
                    201 => "Created",
                    304 => "Not Modified",
                    403 => "Forbidden",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let mut head = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                    resp.status,
                    reason,
                    resp.body.len()
                );
                for (k, v) in &resp.headers {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&resp.body);
                let _ = stream.flush();
                // drop 关闭连接
            }
        });
        MockApi {
            url: format!("http://127.0.0.1:{port}"),
            requests,
            handle,
        }
    }

    pub fn recorded(&self) -> Vec<RecordedReq> {
        self.requests.lock().unwrap().clone()
    }
}

/// 读取一个 HTTP 请求头（读到头部结束即可；请求体不读，由套接字缓冲承载小体量）
fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}
