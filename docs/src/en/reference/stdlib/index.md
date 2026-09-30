---
title: 'Standard Library Overview'
description: 'YaoXiang standard library module overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. After importing a module with
`use`, its functions are called as `Module.function_name(...)`. This directory contains the
per-module API reference documentation.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                               |
| -------------------------------- | ------- | ------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion of any value to String                                         |
| [`std.dict`](./dict)             | 11      | Dictionary read/write, key-value views, and merging                       |
| [`std.fs`](./fs)                 | 22      | File, directory, and path operations                                      |
| [`std.io`](./io)                 | 4       | Standard output, standard input, and formatting                           |
| [`std.math`](./math)             | 18      | Integer, float, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 21      | String search, split, formatting, and parsing                             |
| [`std.time`](./time)             | 14      | Timestamps, formatting, and DateTime field access                         |
| [`std.result`](./result)         | 8       | Construction and unwrapping of Result and Error                           |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                            |
| [`std.assert`](./assert)         | 1       | Assertion                                                                 |
| [`std.net`](./net)               | 4       | HTTP request and URL percent encoding/decoding                            |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identifier                            |
| [`std.os`](./os)                 | 12      | File handle, environment variables, and working directory                 |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak reference                                                 |

<!-- stdlib:index:modules end -->

## Import Conventions

Importing the whole module:

```yaoxiang
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    println(list.len(parts))
}
```

You can also import by name from a module, including constants:

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## Parameter Borrowing Conventions

A `&` in a signature indicates **read-only automatic borrow** (RFC-009 §2.8): the variable passed by
the caller is not moved, and remains usable after the call. This is the default form for most
read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // All three calls only borrow nums; it stays usable afterwards
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

A parameter without `&` means **passed by value**. Most "modifying" functions therefore take a
**functional form — consume the source value and return a new one** — rather than mutating in place:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

Each module page's "Semantic Categories" section lists which functions borrow, which consume, and
which mutate in place. One place deserves particular attention:

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) signatures are marked `&List(A)`,
  but they **mutate the list in place**

## Error Model

The standard library has two kinds of failure modes; each function entry labels them with "Error"
and "Returns …" respectively:

| Form                 | Behavior                                         | Typical Scenario                                                                                 |
| -------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| Throws runtime error | Terminates the current execution with `E6xxx`    | Dictionary missing key `E6008`, index out of bounds `E6003`, assertion failure                   |
| Returns sentinel     | Does not interrupt; returns `Void` / `-1` / `""` | List out-of-bounds read, taking the first element of an empty list, missing environment variable |

Common runtime error codes:

| Code    | Meaning                              | Triggering Example                         |
| ------- | ------------------------------------ | ------------------------------------------ |
| `E6003` | Index out of bounds                  | `list.set(l, 99, v)`                       |
| `E6005` | Assertion failed                     | `assert(false)`                            |
| `E6007` | Generic runtime error                | File does not exist, `result.unwrap` fails |
| `E6008` | Missing key                          | `dict.get(d, "nope")`                      |
| `E6010` | Integer parse failure (as Err value) | `string.parse_int("abc")`                  |
| `E6011` | Float parse failure (as Err value)   | `string.parse_float("abc")`                |

For the full error code table, see [Error Code Reference](../error-code/).

`string.parse_int` / `string.parse_float` belong to a third form: **no error is thrown**; failures
are wrapped into a `Result`'s `Err` value, which can be unwrapped with [`std.result`](./result) or
propagated with `?`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

## Iteration Protocol

`std.list` and `std.range` share the same iterator protocol. An iterator itself is a `Tuple` state
carrier.

> **Move semantics**: both `next` and `has_next` **move** the iterator (the signature has no `&`),
> so each consumption requires recreating it, or simply using `for ... in`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next moved it; recreate it before taking the next element
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

For everyday traversal, use `for ... in` directly:

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

[`range.map`](./range#map) / [`range.filter`](./range#filter) return **lazy** adapters, which only
produce a result after being consumed by `collect` / `reduce` / `for_each` / `for ... in`:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.get(doubled, 0) == 2)
    assert(list.len(doubled) == 3)
}
```

## Platform Availability

The following depend on operating system capabilities and are **not exported** for the `wasm32`
target:

| Range                                                           | Requires       |
| --------------------------------------------------------------- | -------------- |
| All of `std.os`, all of `std.net`, all of `std.weak`            | File / network |
| All of `std.concurrent`                                         | Threads        |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | Standard I/O   |
| `std.time.sleep`                                                | Thread sleep   |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`, and `std.io.print` / `println` / `format_fallback` are available on every target.

## Known Gaps

The following issues were confirmed by actually running the examples while writing the
documentation, and each is tracked with an issue.

**Fixed (2026-09-19)**: #337 / #338 / #339 / #340 have all been fixed, and the corresponding page
bodies have been updated to describe normal usage:

| Location                                      | Original Issue                                                              | Fix                                           |
| --------------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------- |
| [`os.open`](./os#open)                        | One-shot handle, `open`→`write`→`close` could not compile                   | Handle changed to pass by reference ✅        |
| [`time.datetime_*`](./time#datetime-字段访问) | 8 accessors could not be called from source (exported names contained `::`) | Changed to flat names like `datetime_year` ✅ |
| [`time.parse_time`](./time#parse_time)        | `fmt` parameter was ignored; return value could not be reused               | Step-by-step parsing per fmt ✅               |
| [`math.clamp`](./math#clamp)                  | `min > max` would panic the interpreter instead of returning an error       | Returns `E6007` ✅                            |

**Still open**:

| Location                                       | Issue                                                                               | Tracking |
| ---------------------------------------------- | ----------------------------------------------------------------------------------- | -------- |
| [`net.http_get`](./net#http_get) / `http_post` | Placeholder implementation; does not send the request, returns a description string | #56      |

## Documentation Maintenance

This directory uses a **generated + hand-written** hybrid structure:

- **Generated regions** (between the `<!-- stdlib:KEY start/end -->` markers): function summary
  tables and signature blocks, derived from `StdModule::exports()`. Signatures come byte-for-byte
  from `NativeExport::signature` and cannot drift from the implementation.
- **Hand-written regions** (outside the markers): module overview, borrow/move semantics, error
  model, known gaps, and examples.

Gates (run alongside `cargo test --lib` in CI):

| Gate              | Test                                          | Purpose                                             |
| ----------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated regions must match `exports()`            |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output       |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Examples runnable | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run         |

After `exports()` changes, rewrite the generated regions with the healing tool:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic with `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code tables).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — user-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — full table of `E6xxx` runtime error codes
