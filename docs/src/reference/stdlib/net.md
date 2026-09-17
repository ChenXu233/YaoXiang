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

> **实现状态警告（#56）**：本模块 4 个函数中，只有 `url_encode` / `url_decode`
> 是真实实现。`http_get` / `http_post`
> **是占位实现——它们不发任何网络请求**，只把参数拼成字符串返回。详见下文各条。

## 函数一览

<!-- stdlib:table:net start -->

| 函数         | 签名                                      |
| ------------ | ----------------------------------------- |
| `http_get`   | `(url: &String) -> String`                |
| `http_post`  | `(url: &String, body: &String) -> String` |
| `url_encode` | `(s: &String) -> String`                  |
| `url_decode` | `(s: &String) -> String`                  |

<!-- stdlib:table:net end -->

## 函数

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String) -> String
```

<!-- stdlib:sig:net.http_get end -->

> **占位实现，未接入 HTTP 客户端（#56）。** 当前行为是把入参拼成 `"GET: {url}"`
> 字符串返回，**不发起任何网络请求**，也不返回响应体。依赖它做真实的 HTTP 调用会静默失败——拿到的是一个描述性字符串，而非响应内容。

- `url` —— 请求地址（只读借用）

返回：形如 `"GET: http://example.com"` 的字符串。错误：参数缺失时抛出 `E6007`；参数非 `String`
时抛出类型错误。

```yaoxiang
use std.assert
use std.net

main = {
    // 当前实现返回描述字符串，而非响应体
    r = net.http_get("http://example.com")
    assert(r == "GET: http://example.com")
}
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String) -> String
```

<!-- stdlib:sig:net.http_post end -->

> **占位实现，未接入 HTTP 客户端（#56）。** 当前行为是拼接 `"POST {url}: {body}"`
> 字符串返回，**不发起任何网络请求**。

- `url` —— 请求地址（只读借用）
- `body` —— 请求体（只读借用）

返回：形如 `"POST http://example.com: hello"` 的字符串。错误：参数不足时抛出
`E6007`；参数类型不符时抛出类型错误。

```yaoxiang
use std.assert
use std.net

main = {
    r = net.http_post("http://example.com", "hello")
    assert(r == "POST http://example.com: hello")
}
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

main = {
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

main = {
    assert(net.url_decode("a%20b") == "a b")

    // 往返一致
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## 相关

- [错误码参考](../error-code/) —— `E6007` 通用运行时错误
