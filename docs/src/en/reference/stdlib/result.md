---
title: 'std.result'
description: 'Result and Error construction and unwrapping'
---

# std.result

Unwrapping of `Result(T, E)` and field access of the `Error` carrier. `Result` itself is a
record-style sum type exported by `std.result` (RFC-010): use **variant construction syntax** to
build, `match` variant patterns to destructure, and the `?` operator for propagation, driven by the
`Try` interface (see below).

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

> **This module is a "dual-implementation" merge surface**: the `Result` type and the four `Try`
> methods come from the pure-yx `src/std/result.yx`, while the 8 utilities `is_ok` / `is_err` /
> `unwrap` / `unwrap_or` / `unwrap_err` / `code` / `message` / `error` come from the native
> `src/std/result.rs`. Both register on the same export surface
> (`src/frontend/module/registry.rs:315-334` explicitly does a union merge, with yx taking
> precedence on name conflicts), so a single `use std.result` gives you all 9 bindings. However,
> **module-level `Try` methods are not visible** — `result.is_failure(...)` reports `E1042`; the
> four `Try` methods can only be accessed through method-call syntax `r.is_failure()`.

## Runtime representation

| Value                     | Representation                        |
| ------------------------- | ------------------------------------- |
| `Result(T, E).ok(value)`  | Enum variant, carrying `value`        |
| `Result(T, E).err(error)` | Enum variant, carrying `error`        |
| `Error`                   | Struct, with fields `(code, message)` |

`Error.code` is the `E6xxx` / `E7xxx` range registered code from RFC-013 (a cross-version stable
contract); `Error.message` is the human-readable description.

## Try interface (`?` propagation)

`Result` instantiates the `Try(Result(T, E), T, E)` four-method interface inside its type body
(`src/std/result.yx:22`), and the `?` operator is driven by it. These methods can also be called
explicitly, but **only via method syntax** (module-level `result.is_failure(...)` reports `E1042`):

| Method                       | Signature                                         |
| ---------------------------- | ------------------------------------------------- |
| `r.is_failure()`             | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool` |
| `r.success()`                | `(T: Type, E: Type)(self: &Result(T, E)) -> T`    |
| `r.residual()`               | `(T: Type, E: Type)(self: &Result(T, E)) -> E`    |
| `Result(T, E).from_error(e)` | `(T: Type, E: Type)(e: E) -> Result(T, E)`        |

Unlike [`std.option`](option), `?` propagation on `Result` **is available**:

```yaoxiang
use std.result
use std.string

parse_then_add_one: (String) -> Result(Int, Error) = (s) => {
    n = string.parse_int(s)?      // Err propagates as-is
    return Result(Int, Error).ok(n + 1)
}

main: () -> Void = {
    println(result.unwrap(parse_then_add_one("41")))   // 42
}
```

> **No `map` / `map_err`**: the 8 native exports in this module (`src/std/result.rs:71-126`) do not
> include these two names; `result.map(...)` reports
> `E1042 field 'map' not found in struct 'result'`. To transform a success value, use `match`
> variant destructuring.

## Function overview

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

Whether it is the success variant. Read-only borrow; `self` can be reused repeatedly.

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

## Value extraction

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Extract the success value.

Returns: the value carried by the `Ok` variant. Error: calling on an `Err` value throws `E6007`,
with the message **including the original error code and description**, in the form
`unwrap called on Err value (E6010: parse_int: ...)`, so you can see the failure reason without
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

- `default` — fallback value when `Err`

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

## Error fields

### code

<!-- stdlib:sig:result.code start -->

```yaoxiang
code: (self: &Error) -> String
```

<!-- stdlib:sig:result.code end -->

Reads the error code string, e.g. `"E6010"`.

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

## Construction

### error

<!-- stdlib:sig:result.error start -->

```yaoxiang
error: (code: &String, message: &String) -> Error
```

<!-- stdlib:sig:result.error end -->

Constructs an `Error` value. This is the **only channel** for constructing `Error` at the pure-yx
level — the `Error` type family only registers a type identity and has no value-space constructor,
so writing `Error("E…", msg)` in yx will report `E3006` at the IR layer
(`src/std/result.rs:117-124`).

- `code` — error code string (the `E6xxx` / `E7xxx` range registered codes from RFC-013)
- `message` — human-readable description

Returns: a newly created `Error` value, which can be read directly by [`code`](#code) /
[`message`](#message), or used as the payload of `Result.err(...)`.

> **The code table is registration-based**: the `RUNTIME_ERROR_CODES` in `src/std/result.rs:26-32`
> only lists 5 registered codes (`E6009` / `E6010` / `E6011` / `E6012` / `E6013`). Self-made codes
> (e.g. `"E9999"`) can flow within a program, but `yx explain` will not find documentation for them
> — use them only within your own defined subdomain.

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

- [`std.string`](string#parse_int) — parsing functions that produce `Result`
- [`std.range`](range#iter) — returns `Err` with `E6009` when `step=0`
- [`std.option`](option) — `Option(T)`; its `?` propagation is currently **unavailable**, see the
  "Known Gaps" section on that page
- [Error code reference](../error-code/) — runtime error value codes like `E6010` / `E6011`
