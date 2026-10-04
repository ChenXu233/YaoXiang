---
title: 'std.net'
description: 'HTTP 请求与 URL 百分号编解码'
---

# std.net

网络模块。

```yaoxiang
use std.net
```

> 本模块依赖操作系统网络能力，在 `wasm32` 目标上**不导出**。

HTTP 函数基于同步阻塞客户端实现（rustls TLS），重定向默认跟随（最多 5 次）。
阻塞只作用于调用所在的执行线程，与 [`spawn`](../language-spec/concurrency) 的
显式并发模型自洽。

## 函数一览

<!-- stdlib:table:net start -->

| 函数 | 签名 |
| ---- | ---- |
| `http_get` | `(url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)` |
| `http_post` | `(url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)` |
| `url_encode` | `(s: &String) -> String` |
| `url_decode` | `(s: &String) -> String` |

<!-- stdlib:table:net end -->

## 函数

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_get end -->

发起 HTTP GET 请求，返回结构化响应字典：

| 键 | 类型 | 含义 |
| --- | --- | --- |
| `status` | `Int` | HTTP 状态码（如 `200`、`404`） |
| `headers` | `Dict(String, String)` | 响应头，键统一小写；同名多值以 `", "` 合并 |
| `body` | `String` | 响应体（按 UTF-8 解码） |

- `url` —— 请求地址（只读借用）
- `headers` —— 可选请求头字典；省略时发送 `{}`
- `timeout_secs` —— 可选整体超时秒数；省略时为 `30`

**4xx/5xx 是正常响应**（`status` 字段携带状态码），不是错误；只有传输层故障
（DNS 解析失败、连接被拒、超时等）才抛出 `E6007`。

```yaoxiang
use std.net

// 响应形态示意（需要网络，不作为可运行示例）：
// resp = net.http_get("https://httpbin.org/get")
// status = resp["status"]        // 200
// body   = resp["body"]          // 响应体文本
// ctype  = resp["headers"]["content-type"]
```

携带请求头与自定义超时：

```yaoxiang
use std.net

// net.http_get(
//     "https://api.example.com/v1/data",
//     { "Authorization": "Bearer token123" },
//     10,
// )
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_post end -->

发起 HTTP POST 请求，请求体按 UTF-8 编码发送，`Content-Length` 自动设置。
响应字典结构同 [`http_get`](#http_get)。

- `url` —— 请求地址（只读借用）
- `body` —— 请求体（只读借用）
- `headers` —— 可选请求头字典；省略时发送 `{}`
- `timeout_secs` —— 可选整体超时秒数；省略时为 `30`

```yaoxiang
use std.net

// net.http_post(
//     "https://httpbin.org/post",
//     "payload=1&name=yx",
//     { "Content-Type": "application/x-www-form-urlencoded" },
// )
```

### url_encode

<!-- stdlib:sig:net.url_encode start -->

```yaoxiang
url_encode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_encode end -->

百分号编码（percent-encoding）。

- `s` —— 待编码字符串（只读借用）

返回：编码后的字符串。空格编码为 `%20`（非 `+`）；保留字符按 RFC 3986 转义；非保留字符原样保留。

错误：参数缺失时抛出 `E6007`；参数非 `String` 时抛出类型错误。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_encode("a b") == "a%20b")
    assert(net.url_encode("a&b=c?d") == "a%26b%3Dc%3Fd")
}
```

### url_decode

<!-- stdlib:sig:net.url_decode start -->

```yaoxiang
url_decode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_decode end -->

百分号解码，与 `url_encode` 互逆。

- `s` —— 已编码字符串（只读借用）

返回：解码后的字符串。非法转义序列按原样保留。

错误：参数缺失时抛出 `E6007`；参数非 `String` 时抛出类型错误。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // 往返一致
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## 相关

- [错误码参考](../error-code/) —— `E6007` 通用运行时错误
