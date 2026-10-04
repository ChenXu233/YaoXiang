---
title: 'std.result'
description: 'Construction and unpacking of Result and Error'
---

# std.result

Unpacking of `Result(T, E)` and field access of the `Error` carrier. `Result` itself is a
record-style sum type exported by `std.result` (RFC-010): construction uses **variant construction
syntax**, destructuring uses `match` variant patterns, and `?` propagation is driven by the `Try`
interface (see below).

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

> **This module is a 'dual-implementation' merge surface**: the `Result` type and the four `Try`
> methods come from pure yx in `src/std/result.yx`, while the 8 utilities `is_ok` / `is_err` /
> `unwrap` / `unwrap_or` / `unwrap_err` / `code` / `message` / `error` come from the native
> `src/std/result.rs`. Both register to the same export surface
> (`src/frontend/module/registry.rs:315-334` explicitly performs the union merge; on name conflicts,
> yx takes precedence), so a single `use std.result` retrieves all 9 bindings. However,
> **module-level `Try` methods are not visible** — `result.is_failure(...)` reports `E1042`; the
> four `Try` methods can only be accessed via method-call syntax `r.is_failure()`.

## Runtime Representation

| Value                     | Representation                       |
| ------------------------- | ------------------------------------ |
| `Result(T, E).ok(value)`  | Enum variant carrying `value`        |
| `Result(T, E).err(error)` | Enum variant carrying `error`        |
| `Error`                   | Struct with fields `(code, message)` |

`Error.code` is a registered code in the `E6xxx` / `E7xxx` segment of RFC-013 (a stable contract
across versions); `Error.message` is a human-readable description.

## Try Interface (`?` propagation)

`Result` instantiates the four-method `Try(Result(T, E), T, E)` interface within its type body
(`src/std/result.yx:22`), and the `?` operator is driven accordingly. These methods can also be
called explicitly, but **only via method syntax** (module-level `result.is_failure(...)` reports
`E1042`):

| Method                       | Signature                                         |
| ---------------------------- | ------------------------------------------------- |
| `r.is_failure()`             | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool` |
| `r.success()`                | `(T: Type, E: Type)(self: &Result(T, E)) -> T`    |
| `r.residual()`               | `(T: Type, E: Type)(self: &Result(T, E)) -> E`    |
| `Result(T, E).from_error(e)` | `(T: Type, E: Type)(e: E) -> Result(T, E)`        |

Unlike [`std.option`](./option), `?` propagation on `Result` **is available**:

```yaoxiang
use std.result
use std.string

parse_then_add_one: (String) -> Result(Int, Error) = (s) => {
    n = string.parse_int(s)?      // Err is propagated as-is
    return Result(Int, Error).ok(n + 1)
}

main: () -> Void = {
    println(result.unwrap(parse_then_add_one("41")))   // 42
}
```

> **No `map` / `map_err`**: the 8 native exports of this module (`src/std/result.rs:71-126`) do not
> include these two names; `result.map(...)` reports
> `E1042 field 'map' not found in struct 'result'`. To transform the success value, use `match`
> variant destructuring.

## Function Reference

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
| `error`      | `(code: &String, message: &String) -> Error`               |

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
`unwrap called on Err value (E6010: parse_int: ...)`, so the cause of failure can be seen without
first calling `unwrap_err`.

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

Extract the success value, or return `default` on `Err`.

- `default` — the fallback value on `Err`

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

## Construction

### error

<!-- stdlib:sig:result.error start -->

```yaoxiang
error: (code: &String, message: &String) -> Error
```

<!-- stdlib:sig:result.error end -->

Construct an `Error` value. This is the **only channel** for constructing `Error` at the pure yx
layer — the `Error` type family only registers type identity and has no value-space constructor, so
writing `Error("E…", msg)` in yx would report `E3006` at the IR layer (`src/std/result.rs:117-124`).

- `code` — the error code string (a registered code in the `E6xxx` / `E7xxx` segment of RFC-013)
- `message` — human-readable description

Returns: the newly created `Error` value, which can be passed directly to [`code`](#code) /
[`message`](#message) for reading, or used as the payload of `Result.err(...)`.

> **The code table is registry-based**: `RUNTIME_ERROR_CODES` in `src/std/result.rs:26-32` lists
> only 5 registered codes (`E6009` / `E6010` / `E6011` / `E6012` / `E6013`). Self-made codes (such
> as `"E9999"`) can circulate within a program, but `yx explain` will not find documentation for
> them — use only within your own agreed subdomain.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    e = result.error("E6010", "parse_int failed")
    assert(result.code(e) == "E6010")
    assert(result.message(e) == "parse_int failed")

    r = Result(Int, Error).err(e)
    assert(result.is_err(r))
    assert(result.code(result.unwrap_err(r)) == "E6010")
}
```

## Related

- [`std.string`](./string#parse_int) — parsing functions that produce `Result`
- [`std.range`](./range#iter) — returns `Err` with `E6009` when `step=0`
- [`std.option`](./option) — `Option(T)`; its `?` propagation is currently **unavailable**; see
  'Known Gaps' on that page
- [Error Code Reference](../error-code/) — runtime error value codes such as `E6010` / `E6011`
