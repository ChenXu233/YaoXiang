---
title: 'Standard Library Overview'
description: 'YaoXiang standard library module overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. Each module is imported via `use`
and then called as `module_name.function_name(...)`. This directory contains the API reference
documentation organized by module.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                               |
| -------------------------------- | ------- | ------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion of any value to String                                         |
| [`std.dict`](./dict)             | 11      | Dictionary read/write, key-value views, and merging                       |
| [`std.io`](./io)                 | 7       | Standard output, standard input, and whole-file read/write                |
| [`std.math`](./math)             | 18      | Integer, float, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 21      | String search, split, formatting, and parsing                             |
| [`std.time`](./time)             | 14      | Timestamp, formatting, and DateTime field access                          |
| [`std.result`](./result)         | 8       | Construction and unpacking of Result and Error                            |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                            |
| [`std.assert`](./assert)         | 1       | Assertion                                                                 |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent-encoding/decoding                           |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identifier                            |
| [`std.os`](./os)                 | 22      | File handles, directories, environment variables, and working directory   |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak references                                                |

<!-- stdlib:index:modules end -->

## Import Conventions

Importing an entire module:

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

## Parameter Borrowing Convention

The `&` in a signature denotes **automatic read-only borrowing** (RFC-009 §2.8): the variable passed
by the caller is not moved and remains usable after the call. This is the default form for the many
read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 三处调用都只读借用 nums，之后仍可用
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

Parameters without `&` mean **passed by value**. Most "modifying" functions therefore take the form
of **consuming the source value and returning a new value** (a functional style), rather than
in-place mutation:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 返回新列表；base 已被移动
    assert(list.len(extended) == 3)
}
```

The "Semantic Categories" section of each module page lists which functions in that module borrow,
which consume, and which mutate in place. One point deserves special attention:

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) signatures are marked as
  `&List(A)`, but they **mutate the list in place**

## Error Model

The standard library has two forms of failure. Each function entry annotates them as "Error" and
"Returns …" respectively:

| Form                     | Behavior                                              | Typical Scenario                                                                           |
| ------------------------ | ----------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Throws a runtime error   | Terminates the current execution with an `E6xxx` code | Missing dictionary key `E6008`, index out of bounds `E6003`, assertion failure             |
| Returns a sentinel value | Does not interrupt; returns `Void` / `-1` / `""`      | List out-of-bounds read, getting first element of empty list, missing environment variable |

Common runtime error codes:

| Error Code | Meaning                                 | Trigger Example                         |
| ---------- | --------------------------------------- | --------------------------------------- |
| `E6003`    | Index out of bounds                     | `list.set(l, 99, v)`                    |
| `E6005`    | Assertion failure                       | `assert(false)`                         |
| `E6007`    | General runtime error                   | File not found, `result.unwrap` failure |
| `E6008`    | Missing key                             | `dict.get(d, "nope")`                   |
| `E6010`    | Integer parse failure (as an Err value) | `string.parse_int("abc")`               |
| `E6011`    | Float parse failure (as an Err value)   | `string.parse_float("abc")`             |

For the complete error code list, see [Error Code Reference](../error-code/).

`string.parse_int` / `string.parse_float` belong to a third form: **they do not throw**, instead
wrapping the failure into the `Err` value of a `Result`, which can be unpacked with
[`std.result`](./result) or propagated with `?`.

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

`std.list` and `std.range` provide the same set of iterator protocols. An iterator itself is a
`Tuple` state carrier.

> **Move semantics**: Both `next` and `has_next` **move** the iterator (the signature has no `&`),
> so each access requires recreating it, or simply use `for ... in`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next 移动了 it，重新创建后再取元素
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

[`range.map`](./range#map) / [`range.filter`](./range#filter) return **lazy** adapters; they only
produce results when consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

The following depend on OS capabilities and are **not exported** on the `wasm32` target:

| Scope                                                           | Required      |
| --------------------------------------------------------------- | ------------- |
| All of `std.os`, all of `std.net`, all of `std.weak`            | Files/Network |
| All of `std.concurrent`                                         | Threads       |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | Standard I/O  |
| `std.time.sleep`                                                | Thread sleep  |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` and `std.io.print` / `println` / `format_fallback` are available on all targets.

## Resolved Gaps

The following issues were confirmed by actually running examples one by one while writing the
documentation, and all have been tracked with issues.

**Fixed (2026-09-19)**: Issues #337 / #338 / #339 / #340 have all been fixed, and the corresponding
page body has been rewritten to describe normal usage:

| Location                                      | Original Issue                                                       | Fix                                           |
| --------------------------------------------- | -------------------------------------------------------------------- | --------------------------------------------- |
| [`os.open`](./os#open)                        | Handle is one-shot; `open`→`write`→`close` cannot compile            | Handle changed to pass by reference ✅        |
| [`time.datetime_*`](./time#datetime-字段访问) | 8 accessors unreachable from source code (export names contain `::`) | Changed to flat names like `datetime_year` ✅ |
| [`time.parse_time`](./time#parse_time)        | `fmt` parameter is ignored; return value is unusable                 | Step-by-step parsing according to fmt ✅      |
| [`math.clamp`](./math#clamp)                  | `min > max` panics the interpreter instead of returning an error     | Returns `E6007` ✅                            |

**Still open**:

| Location                                       | Issue                                                                            | Tracking |
| ---------------------------------------------- | -------------------------------------------------------------------------------- | -------- |
| [`net.http_get`](./net#http_get) / `http_post` | Placeholder implementation; does not send requests; returns a description string | #56      |

## Documentation Maintenance

This directory uses a **generated + handwritten** mixed structure:

- **Generated sections** (between `<!-- stdlib:KEY start/end -->` markers): function summary tables
  and signature blocks, derived from `StdModule::exports()`. The signatures come byte-for-byte from
  `NativeExport::signature`, so they cannot drift from the implementation.
- **Handwritten sections** (outside the markers): module overview, borrowing/move semantics, error
  model, known gaps, and examples.

Gates (run in CI along with `cargo test --lib`):

| Gate              | Test                                          | Purpose                                             |
| ----------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated sections must match `exports()`           |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output       |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Examples must run | `test_stdlib_docs_examples_run`               | Every `yaoxiang` example must actually run          |

After `exports()` changes, rewrite the generated sections with the healing tool:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code tables).

## Related Documents

- [Standard Library Specification](../language-spec/stdlib.md) — language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — user-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — complete list of `E6xxx` runtime error codes
