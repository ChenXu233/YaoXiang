---
title: 'std.net'
description: 'HTTP Requests and URL Percent-Encoding'
---

# std.net

The networking module.

```yaoxiang
use std.net
```

> This module depends on the operating system's networking capabilities and is **not exported** on
> the `wasm32` target.

> **Implementation Status Warning (#56)**: Of the 4 functions in this module, only `url_encode` /
> `url_decode` are real implementations. `http_get` / `http_post` **are placeholder
> implementations—they do not send any network request** and only concatenate parameters into a
> string and return it. See the entries below for details.

## Function Overview

<!-- stdlib:table:net start -->

| Function     | Signature                                 |
| ------------ | ----------------------------------------- |
| `http_get`   | `(url: &String) -> String`                |
| `http_post`  | `(url: &String, body: &String) -> String` |
| `url_encode` | `(s: &String) -> String`                  |
| `url_decode` | `(s: &String) -> String`                  |

<!-- stdlib:table:net end -->

## Functions

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String) -> String
```

<!-- stdlib:sig:net.http_get end -->

> **Placeholder implementation, not wired to an HTTP client (#56).** The current behavior
> concatenates the input into a string like `"GET: {url}"` and returns it, **does not initiate any
> network request**, and does not return a response body. Relying on it for real HTTP calls will
> fail silently—you will get a descriptive string, not the response content.

- `url` —— The request URL (read-only borrow)

Returns: a string like `"GET: http://example.com"`. Errors: throws `E6007` when an argument is
missing; throws a type error when an argument is not a `String`.

```yaoxiang
use std.assert
use std.net

main = {
    // Current implementation returns a descriptive string, not the response body
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

> **Placeholder implementation, not wired to an HTTP client (#56).** The current behavior
> concatenates into a string like `"POST {url}: {body}"` and returns it, **does not initiate any
> network request**.

- `url` —— The request URL (read-only borrow)
- `body` —— The request body (read-only borrow)

Returns: a string like `"POST http://example.com: hello"`. Errors: throws `E6007` when arguments are
insufficient; throws a type error when argument types do not match.

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

Percent-encoding.

- `s` —— The string to encode (read-only borrow)

Returns: the encoded string. Spaces are encoded as `%20` (not `+`); reserved characters are escaped
according to RFC 3986; unreserved characters are preserved as-is.

Errors: throws `E6007` when an argument is missing; throws a type error when an argument is not a
`String`.

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

Percent-decoding, inverse of `url_encode`.

- `s` —— The encoded string (read-only borrow)

Returns: the decoded string. Illegal escape sequences are preserved as-is.

Errors: throws `E6007` when an argument is missing; throws a type error when an argument is not a
`String`.

```yaoxiang
use std.assert
use std.net

main = {
    assert(net.url_decode("a%20b") == "a b")

    // Round-trip is consistent
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## Related

- [Error Code Reference](../error-code/) —— `E6007` generic runtime error
