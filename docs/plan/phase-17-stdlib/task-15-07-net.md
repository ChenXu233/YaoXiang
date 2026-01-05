# Task 15.7: 网络

> **优先级**: P2
> **状态**: ⏳ 待实现

## 功能描述

提供 TCP、UDP、HTTP 等网络编程支持。

## 网络 API

```yaoxiang
use std::net

# TCP 服务器
listener = net::tcp_listen("127.0.0.1:8080")
conn = listener.accept()
reader = conn.reader()
writer = conn.writer()

# TCP 客户端
client = net::tcp_connect("127.0.0.1:8080")
client.write("Hello")
response = client.read()

# UDP
socket = net::udp_socket()
socket.send_to("message", "127.0.0.1:8080")
(data, addr) = socket.recv_from()

# HTTP 客户端
response = net::http_get("https://example.com")
status = response.status
body = response.body()

# HTTP 服务器
server = net::http_server("0.0.0.0:8080", |req| {
    Response::ok("Hello!")
})
```

## 验收测试

```yaoxiang
# test_net.yx

use std::net

# TCP 连接测试
client = net::tcp_connect("httpbin.org", 80)
request = "GET /get HTTP/1.1\r\nHost: httpbin.org\r\n\r\n"
client.write(request)
response = client.read()
assert(response.contains("HTTP/1.1"))

print("Net tests passed!")
```

## 相关文件

- **net/mod.rs**
- **net/tcp.rs**
- **net/http.rs**
