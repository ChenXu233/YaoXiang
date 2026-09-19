---
title: 'Standard Library Overview'
description: 'YaoXiang standard library modules overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized by modules; each module is imported with `use`
and then invoked as `Module.function(...)`. This directory contains per-module API reference
documentation.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                               |
| -------------------------------- | ------- | ------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion of any value to String                                         |
| [`std.dict`](./dict)             | 11      | Dictionary read/write, key/value views, and merging                       |
| [`std.io`](./io)                 | 7       | Standard output, standard input, and whole-file read/write                |
| [`std.math`](./math)             | 18      | Integer, float, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 19      | String search, split, formatting, and parsing                             |
| [`std.time`](./time)             | 14      | Timestamps, formatting, and DateTime field access                         |
| [`std.result`](./result)         | 9       | Construction and unwrapping of Result and Error                           |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                            |
| [`std.assert`](./assert)         | 1       | Assertions                                                                |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent-encoding/decoding                           |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identity                              |
| [`std.os`](./os)                 | 22      | File handles, directories, environment variables, and working directory   |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak references                                                |

<!-- stdlib:index:modules end -->

## Import Conventions

Importing a whole module:

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

A `&` in a signature indicates **read-only auto-borrow** (RFC-009 §2.8): the variable passed by the
caller is not moved and can still be used after the call. This is the default shape for most
read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // All three calls only read-borrow nums; it remains usable afterward
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

A parameter without `&` means **passed by value**. Because of this, most "modifying" functions take
the form of **consuming the source value and returning a new one**—the functional style—rather than
in-place mutation:

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
which mutate in place. One point deserves special attention:

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) are marked as `&List(A)` in the
  signature, but **mutate the list in place**

## Error Model

The standard library has two shapes of failure, each labeled in individual function entries as
either "Error" or "Returns …":

| Shape                  | Behavior                                              | Typical scenarios                                                        |
| ---------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------ |
| Throws runtime error   | Terminates the current execution with an `E6xxx` code | Missing dict key `E6008`, index out of bounds `E6003`, assertion failure |
| Returns sentinel value | Does not interrupt; returns `Void` / `-1` / `""`      | List out-of-bounds read, head of empty list, missing env var             |

Common runtime error codes:

| Code    | Meaning                              | Trigger example                              |
| ------- | ------------------------------------ | -------------------------------------------- |
| `E6003` | Index out of bounds                  | `list.set(l, 99, v)`                         |
| `E6005` | Assertion failed                     | `assert(false)`                              |
| `E6007` | Generic runtime error                | File does not exist, `result.unwrap` failure |
| `E6008` | Missing key                          | `dict.get(d, "nope")`                        |
| `E6010` | Integer parse failure (as Err value) | `string.parse_int("abc")`                    |
| `E6011` | Float parse failure (as Err value)   | `string.parse_float("abc")`                  |

See the [Error Code Reference](../error-code/) for the full table.

`string.parse_int` / `string.parse_float` fall into a third shape: **they do not throw**, but wrap
the failure as the `Err` value of a `Result`, which can be unwrapped with [`std.result`](./result)
or propagated with `?`.

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

`std.list` and `std.range` provide the same iterator protocol. The iterator itself is a `Tuple`
carrying the state.

> **Move semantics**: both `next` and `has_next` **move** the iterator (no `&` in the signature), so
> each access requires recreating it, or simply use `for ... in`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next has moved it; recreate it before taking the element
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

For everyday traversal, just use `for ... in`:

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
produce results once consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

The following content depends on operating system capabilities and is **not exported** for the
`wasm32` target:

| Scope                                                           | Requires        |
| --------------------------------------------------------------- | --------------- |
| All of `std.os`, all of `std.net`, all of `std.weak`            | Files / network |
| All of `std.concurrent`                                         | Threads         |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | Standard I/O    |
| `std.time.sleep`                                                | Thread sleep    |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`, and `std.io.print` / `println` / `format_fallback` are available on all targets.

## Known Gaps

The following issues were each confirmed by actually running the examples when the documentation was
written, and are all tracked with issues.

**Fixed (2026-09-19)**: #337 / #338 / #339 / #340 are all fixed, and the corresponding pages have
been rewritten as normal usage:

| Location                                          | Original issue                                                           | Fix                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------ |
| [`os.open`](./os#open)                            | Handle is one-shot; `open`→`write`→`close` would not compile             | Handle now passed by reference ✅          |
| [`time.datetime_*`](./time#datetime-field-access) | 8 accessors could not be called from source (export name contained `::`) | Flattened to names like `datetime_year` ✅ |
| [`time.parse_time`](./time#parse_time)            | `fmt` parameter was ignored; return value could not be used further      | Step-by-step parsing according to fmt ✅   |
| [`math.clamp`](./math#clamp)                      | `min > max` would panic the interpreter instead of returning an error    | Returns `E6007` ✅                         |

**Still open**:

| Location                                       | Issue                                                                             | Tracking |
| ---------------------------------------------- | --------------------------------------------------------------------------------- | -------- |
| [`net.http_get`](./net#http_get) / `http_post` | Placeholder implementation; does not send a request, returns a description string | #56      |

## Documentation Maintenance

This directory follows a **generated + handwritten** hybrid structure:

- **Generated region** (between the `<!-- stdlib:KEY start/end -->` markers): function overview
  tables and signature blocks, derived from `StdModule::exports()`. Signatures come byte-for-byte
  from `NativeExport::signature` and cannot drift from the implementation.
- **Handwritten region** (outside the markers): module overviews, borrow/move semantics, the error
  model, known gaps, and examples.

Gates (run in CI alongside `cargo test --lib`):

| Gate                | Test                                          | Purpose                                             |
| ------------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection     | `test_stdlib_docs_match_generation`           | Generated region must match `exports()`             |
| Orphan detection    | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output       |
| Coverage            | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Example runnability | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run         |

After `exports()` changes, use the healing tool to rewrite the generated region:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code table).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — Language-level design conventions
  for the standard library
- [FFI Specification](../language-spec/ffi.md) — User-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — Full table of `E6xxx` runtime error codes
