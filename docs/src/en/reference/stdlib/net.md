---
title: 'std.net'
description: 'HTTP Requests and URL Percent-Encoding'
---

# std.net

Networking module.

```yaoxiang
use std.net
```

> This module depends on the operating system's networking capabilities and is **not exported** for
> the `wasm32` target.

The HTTP functions are based on a synchronous blocking client (rustls TLS) and follow redirects by
default (up to 5 times). Blocking only affects the execution thread where the call is made, which is
consistent with the explicit concurrency model of [`spawn`](../language-spec/concurrency).

## Function Overview

<!-- stdlib:table:net start -->

| Function     | Signature                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------- |
| `http_get`   | `(url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)`                |
| `http_post`  | `(url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)` |
| `url_encode` | `(s: &String) -> String`                                                                                  |
| `url_decode` | `(s: &String) -> String`                                                                                  |

<!-- stdlib:table:net end -->

## Functions

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_get end -->

Initiates an HTTP GET request and returns a structured response dictionary:

| Key       | Type                   | Meaning                                                                                          |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------ |
| `status`  | `Int`                  | HTTP status code (e.g., `200`, `404`)                                                            |
| `headers` | `Dict(String, String)` | Response headers with keys lowercased; multiple values with the same name are joined with `", "` |
| `body`    | `String`               | Response body (decoded as UTF-8)                                                                 |

- `url` — Request URL (read-only borrow)
- `headers` — Optional request header dictionary; sends `{}` when omitted
- `timeout_secs` — Optional overall timeout in seconds; defaults to `30`

**4xx/5xx are normal responses** (the `status` field carries the status code), not errors; only
transport-layer failures (DNS resolution failure, connection refused, timeout, etc.) throw `E6007`.

```yaoxiang
use std.net

// Response shape illustration (requires network, not a runnable example):
// resp = net.http_get("https://httpbin.org/get")
// status = resp["status"]        // 200
// body   = resp["body"]          // Response body text
// ctype  = resp["headers"]["content-type"]
```

With request headers and a custom timeout:

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

Initiates an HTTP POST request; the request body is sent as UTF-8 encoded, and `Content-Length` is
set automatically. The response dictionary structure is the same as [`http_get`](#http_get).

- `url` — Request URL (read-only borrow)
- `body` — Request body (read-only borrow)
- `headers` — Optional request header dictionary; sends `{}` when omitted
- `timeout_secs` — Optional overall timeout in seconds; defaults to `30`

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

Percent-encoding.

- `s` — String to encode (read-only borrow)

Returns: the encoded string. Spaces are encoded as `%20` (not `+`); reserved characters are escaped
per RFC 3986; unreserved characters are left as-is.

Errors: throws `E6007` if the argument is missing; throws a type error if the argument is not a
`String`.

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

Percent-decoding; the inverse of `url_encode`.

- `s` — Already-encoded string (read-only borrow)

Returns: the decoded string. Illegal escape sequences are left as-is.

Errors: throws `E6007` if the argument is missing; throws a type error if the argument is not a
`String`.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // Round-trip consistency
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## Related

- [Error Code Reference](../error-code/) — `E6007` General Runtime Error
