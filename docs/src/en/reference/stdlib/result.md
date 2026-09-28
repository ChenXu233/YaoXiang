---
title: 'std.result'
description: 'Construction and unpacking of Result and Error'
---

# std.result

Unpacking of `Result(T, E)` and field access of the `Error` carrier. `Result` itself is a
record-style sum type exported by `std.result` (RFC-010): construction uses **variant construction
syntax**, deconstruction uses `match` variant patterns, and `?` propagation is driven by the `Try`
interface (see below).

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

## Runtime Representation

| Value                     | Representation                        |
| ------------------------- | ------------------------------------- |
| `Result(T, E).ok(value)`  | Enum variant, carrying `value`        |
| `Result(T, E).err(error)` | Enum variant, carrying `error`        |
| `Error`                   | Struct, with fields `(code, message)` |

`Error.code` is the registered code from the `E6xxx` / `E7xxx` segment of RFC-013 (a stable contract
across versions), and `Error.message` is the human-readable description.

## Try Interface (`?` propagation)

`Result` instantiates the four-method `Try(Result(T, E), T, E)` interface in its type body, and the
`?` operator is driven accordingly: `is_failure` determines failure, `success` retrieves the success
payload, `residual` retrieves the failure payload, and `from_error` reconstructs a `Result` from an
error value. These methods can also be called explicitly.

## Function Overview

<!-- stdlib:table:result start -->

| Function     | Signature                                                  |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |

<!-- stdlib:table:result end -->

## Predicates

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

Whether it is the success variant. Read-only borrow; `self` can be used repeatedly.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).ok(1)
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

Whether it is the error variant. Read-only borrow.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).err("e")
    assert(result.is_err(r))
}
```

## Value Extraction

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Extract the success value.

Returns: the value carried by the `Ok` variant. Error: calling on an `Err` value throws `E6007`,
with the message **including the original error code and description**, in the form
`unwrap called on Err value (E6010: parse_int: ...)`, so there is no need to call `unwrap_err` first
to see the cause of failure.

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

Extract the success value, or return `default` when it is `Err`.

- `default` — the fallback value when `Err`

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

Extract the error value.

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

Read the error code string, such as `"E6010"`.

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

Read the error description text.

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

## See Also

- [`std.string`](./string#parse_int) — parsing function that produces `Result`
- [Error code reference](../error-code/) — runtime error value codes such as `E6010` / `E6011`
