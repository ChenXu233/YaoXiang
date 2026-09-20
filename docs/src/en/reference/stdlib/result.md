---
title: 'std.result'
description: 'Construction and unpacking of Result and Error'
---

# std.result

Construction and unpacking of `Result(T, E)`, and field access for the `Error` carrier.

```yaoxiang
use std.result
```

## Runtime Representation

| Value               | Representation                        |
| ------------------- | ------------------------------------- |
| `Result.ok(value)`  | enum variant, carrying `value`        |
| `Result.err(error)` | enum variant, carrying `error`        |
| `Error`             | struct, with fields `(code, message)` |

`Error.code` is a registered code from the `E6xxx` / `E7xxx` range per RFC-013 (a stable contract
across versions), and `Error.message` is a human-readable description.

## Function Summary

<!-- stdlib:table:result start -->

| Function     | Signature                                                  |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `ok`         | `(T: Type, E: Type)(value: T) -> Result(T, E)`             |
| `err`        | `(T: Type, E: Type)(error: E) -> Result(T, E)`             |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |

<!-- stdlib:table:result end -->

## Construction

### ok

<!-- stdlib:sig:result.ok start -->

```yaoxiang
ok: (T: Type, E: Type)(value: T) -> Result(T, E)
```

<!-- stdlib:sig:result.ok end -->

Wraps a success value.

The `Ok` value unpacked by `?` must be re-wrapped before it can continue to propagate along a
`Result` return type; `ok` is that wrapper.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(42)
    assert(result.is_ok(r))
}
```

### err

<!-- stdlib:sig:result.err start -->

```yaoxiang
err: (T: Type, E: Type)(error: E) -> Result(T, E)
```

<!-- stdlib:sig:result.err end -->

Wraps an error value.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("boom")
    assert(result.is_err(r))
}
```

## Predicates

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

Whether this is the success variant. Read-only borrow; `self` can be used repeatedly.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // reusable
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

Whether this is the error variant. Read-only borrow.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("e")
    assert(result.is_err(r))
}
```

## Extracting Values

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Extracts the success value.

Returns: the value carried by the `Ok` variant. Error: calling on an `Err` value throws `E6007`,
with the message **including the original error code and description**, in a form like
`unwrap called on Err value (E6010: parse_int: ...)`, so the failure reason is visible without first
calling `unwrap_err`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("42")
    assert(result.unwrap(r) == 42)
}
```

### unwrap_or

<!-- stdlib:sig:result.unwrap_or start -->

```yaoxiang
unwrap_or: (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
```

<!-- stdlib:sig:result.unwrap_or end -->

Extracts the success value, or returns `default` on `Err`.

- `default` —— fallback value when `Err`

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    good = string.parse_int("42")
    assert(result.unwrap_or(good, 0) == 42)

    bad = string.parse_int("abc")
    assert(result.unwrap_or(bad, 0) == 0)
}
```

### unwrap_err

<!-- stdlib:sig:result.unwrap_err start -->

```yaoxiang
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
```

<!-- stdlib:sig:result.unwrap_err end -->

Extracts the error value.

Returns: the value carried by the `Err` variant. Error: calling on an `Ok` value throws `E6007`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.is_err(r))
}
```

## Error Fields

### code

<!-- stdlib:sig:result.code start -->

```yaoxiang
code: (self: &Error) -> String
```

<!-- stdlib:sig:result.code end -->

Reads the error code string, such as `"E6010"`.

> The signature type is `Error`, but the runtime error carrier is a struct with fields
> `(code, message)`. Call directly on an `Error` value.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.code(e) == "E6010")
}
```

### message

<!-- stdlib:sig:result.message start -->

```yaoxiang
message: (self: &Error) -> String
```

<!-- stdlib:sig:result.message end -->

Reads the error description text.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(string.len(result.message(e)) > 0)
}
```

## Related

- [`std.string`](./string#parse_int) —— parsing function that produces a `Result`
- [Error code reference](../error-code/) —— runtime error value codes such as `E6010` / `E6011`
