---
title: 'Standard Library Overview'
description: 'YaoXiang standard library module overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. After importing a module via `use`,
you call its functions as `module.function_name(...)`. This section provides per-module API
reference.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                         |
| -------------------------------- | ------- | ------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion of any value to String                                   |
| [`std.dict`](./dict)             | 10      | Dictionary read/write, key-value views, merging                     |
| [`std.io`](./io)                 | 7       | Standard output, standard input, whole-file I/O                     |
| [`std.list`](./list)             | 22      | List add/remove, slicing, higher-order, iterator                    |
| [`std.math`](./math)             | 18      | Integer, float, and trig functions, with PI/E/TAU constants         |
| [`std.string`](./string)         | 19      | String search, splitting, formatting, parsing                       |
| [`std.time`](./time)             | 14      | Timestamps, formatting, DateTime field access                       |
| [`std.result`](./result)         | 9       | Construction and unpacking of Result and Error                      |
| [`std.range`](./range)           | 10      | Range iteration, predicates, lazy adapters                          |
| [`std.assert`](./assert)         | 1       | Assertions                                                          |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent-encoding/decoding                     |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, thread identifier                          |
| [`std.os`](./os)                 | 22      | File handles, directories, environment variables, working directory |
| [`std.weak`](./weak)             | 2       | Arc / Weak references                                               |

<!-- stdlib:index:modules end -->

## Import Conventions

Import an entire module:

```yaoxiang
use std.list
use std.string

main = {
    parts = string.split("a,b,c", ",")
    println(list.len(parts))
}
```

You can also import by name from a module, including constants:

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main = {
    assert(PI > 3.14)
}
```

## Parameter Borrowing Conventions

The `&` in a signature denotes a **read-only automatic borrow** (RFC-009 §2.8): the caller's
variable is not moved and remains usable after the call. This is the default form for the many
read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main = {
    nums = [1, 2, 3]

    // All three calls borrow nums read-only; it remains usable afterward
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

A parameter without `&` means **pass-by-value**. Most "mutating" functions are therefore **consume
the source value, return a new value** — a functional shape rather than in-place mutation:

```yaoxiang
use std.assert
use std.list

main = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

Each module page's "Semantics" section lists which functions borrow, which consume, and which mutate
in place. Two cases deserve special attention:

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) are signed as `&List(A)` but
  **mutate the list in place**.
- Handles returned by [`os.open`](./os#open) are **single-use**; the `open` → `write` → `close`
  sequence does not compile.

## Error Model

The standard library has two failure shapes, marked in each function entry as "Errors" and "Returns
..." respectively:

| Shape                  | Behavior                                          | Typical Cases                                                             |
| ---------------------- | ------------------------------------------------- | ------------------------------------------------------------------------- |
| Throws runtime error   | Terminates current execution with an `E6xxx` code | Missing dict key `E6008`, index out of bounds `E6003`, assertion failure  |
| Returns sentinel value | Does not abort; returns `Void` / `-1` / `""`      | List out-of-bounds read, head of empty list, missing environment variable |

Common runtime error codes:

| Code    | Meaning                        | Example Trigger                             |
| ------- | ------------------------------ | ------------------------------------------- |
| `E6003` | Index out of bounds            | `list.set(l, 99, v)`                        |
| `E6005` | Assertion failed               | `assert(false)`                             |
| `E6007` | Generic runtime error          | File does not exist, `result.unwrap` failed |
| `E6008` | Missing key                    | `dict.get(d, "nope")`                       |
| `E6010` | Integer parse failure (as Err) | `string.parse_int("abc")`                   |
| `E6011` | Float parse failure (as Err)   | `string.parse_float("abc")`                 |

See the full table in the [Error Code Reference](../error-code/).

`string.parse_int` / `string.parse_float` fall into a third shape: **no throw** — failures are
wrapped as `Err` values of a `Result`, which can be unwrapped with [`std.result`](./result) or
propagated with `?`.

```yaoxiang
use std.assert
use std.result
use std.string

main = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

## Iterator Protocol

`std.list` and `std.range` share the same iterator protocol. An iterator itself is a `Tuple`-shaped
state carrier.

> **Move semantics**: `next` and `has_next` both **move** the iterator (the signature has no `&`),
> so each use must recreate it, or you can use `for ... in` directly.

```yaoxiang
use std.assert
use std.list

main = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next moved it; recreate it before taking an element
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

For everyday traversal, just use `for ... in`:

```yaoxiang
use std.assert

main = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

[`range.map`](./range#map) / [`range.filter`](./range#filter) return **lazy** adapters that only
produce results once consumed by `collect` / `reduce` / `for_each` / `for ... in`:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.get(doubled, 0) == 2)
    assert(list.len(doubled) == 3)
}
```

## Platform Availability

The following depend on OS capabilities and are **not exported** on the `wasm32` target:

| Scope                                                           | Requires      |
| --------------------------------------------------------------- | ------------- |
| All of `std.os`, all of `std.net`, all of `std.weak`            | Files/network |
| All of `std.concurrent`                                         | Threads       |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | Standard I/O  |
| `std.time.sleep`                                                | Thread sleep  |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`, plus `std.io.print` / `println` / `format_fallback`, are available on all targets.

## Known Gaps

Each of the following was confirmed by actually running examples while writing this documentation.
All have open issues for tracking; once fixed, the corresponding page body should be updated
accordingly (the generation gate will auto-prompt on signature changes):

| Location                                                       | Problem                                                                   | Tracking |
| -------------------------------------------------------------- | ------------------------------------------------------------------------- | -------- |
| [`net.http_get`](./net#http_get) / `http_post`                 | Stub implementation; does not send requests, returns a description string | #56      |
| [`os.open`](./os#open)                                         | Handle is single-use; `open`→`write`→`close` does not compile             | #337     |
| [`time.DateTime::*`](./time#datetime-field-access-unavailable) | 8 accessors unreachable from source (export name contains `::`)           | #338     |
| [`time.parse_time`](./time#parse_time)                         | `fmt` parameter ignored; return value unusable                            | #340     |
| [`math.clamp`](./math#clamp)                                   | `min > max` panics the interpreter rather than returning an error         | #339     |

## Documentation Maintenance

This directory uses a **generated + hand-written** hybrid structure:

- **Generated regions** (between `<!-- stdlib:KEY start/end -->` markers): function summary tables
  and signature blocks, derived from `StdModule::exports()`. Signatures come byte-for-byte from
  `NativeExport::signature` and cannot drift from the implementation.
- **Hand-written regions** (outside the markers): module overview, borrow/move semantics, error
  model, known gaps, and examples.

Gates (run alongside `cargo test --lib` in CI):

| Gate              | Test                                          | Effect                                              |
| ----------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated regions must match `exports()`            |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed the generator's output |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Examples runnable | `test_stdlib_docs_examples_run`               | Every ```yaoxiang` example must actually run        |

After `exports()` changes, regenerate the generated regions with the healing tool:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic with `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code tables).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — language-level design conventions
  for the standard library
- [FFI Specification](../language-spec/ffi.md) — user-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — full table of `E6xxx` runtime error codes
