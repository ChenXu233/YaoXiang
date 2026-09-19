---
title: 'Standard Library Overview'
description: 'YaoXiang standard library module overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized as modules; each module is imported via `use` and
called as `Module.function(...)`. This directory is the API reference documentation, split by
module.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                               |
| -------------------------------- | ------- | ------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Arbitrary value to String conversion                                      |
| [`std.dict`](./dict)             | 11      | Dictionary read/write, key-value views, and merging                       |
| [`std.io`](./io)                 | 7       | Standard output, standard input, and whole-file read/write                |
| [`std.list`](./list)             | 22      | List add/remove, slicing, higher-order functions, and iterator protocol   |
| [`std.math`](./math)             | 18      | Integer, float, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 19      | String search, splitting, formatting, and parsing                         |
| [`std.time`](./time)             | 14      | Timestamps, formatting, and DateTime field access                         |
| [`std.result`](./result)         | 9       | Result and Error construction and unpacking                               |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                            |
| [`std.assert`](./assert)         | 1       | Assertions                                                                |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent encoding/decoding                           |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identifier                            |
| [`std.os`](./os)                 | 22      | File handles, directories, environment variables, and working directory   |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak references                                                |

<!-- stdlib:index:modules end -->

## Import Conventions

Import a whole module:

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

The `&` in a signature indicates **read-only automatic borrowing** (RFC-009 §2.8): the variable
passed by the caller is not moved and can still be used after the call. This is the default form for
most read-only functions in the standard library.

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

A parameter without `&` is **passed by value**. Most "modifying" functions are therefore in a
**functional form that consumes the source value and returns a new value**, rather than mutating in
place:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

The "Semantic Categories" section of each module page lists which functions borrow, which consume,
and which mutate in place. There are two cases worth noting:

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) are signed as `&List(A)`, but they
  **mutate the list in place**
- The handle returned by [`os.open`](./os#open) is **single-use**; `open` → `write` → `close` will
  not compile

## Error Model

The standard library has two forms of failure, each marked in a function's entry as "Error" or
"Returns ...":

| Form                   | Behavior                                          | Typical Scenario                                                                             |
| ---------------------- | ------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Throws runtime error   | Terminates current execution with an `E6xxx` code | Missing dict key `E6008`, index out of bounds `E6003`, assertion failure                     |
| Returns sentinel value | Does not interrupt; returns `Void` / `-1` / `""`  | List out-of-bounds read, taking first element of an empty list, missing environment variable |

Common runtime error codes:

| Code    | Meaning                              | Triggering Example                           |
| ------- | ------------------------------------ | -------------------------------------------- |
| `E6003` | Index out of bounds                  | `list.set(l, 99, v)`                         |
| `E6005` | Assertion failed                     | `assert(false)`                              |
| `E6007` | General runtime error                | File does not exist, `result.unwrap` failure |
| `E6008` | Key missing                          | `dict.get(d, "nope")`                        |
| `E6010` | Integer parse failure (as Err value) | `string.parse_int("abc")`                    |
| `E6011` | Float parse failure (as Err value)   | `string.parse_float("abc")`                  |

For the full error code table, see [Error Code Reference](../error-code/).

`string.parse_int` / `string.parse_float` belong to a third form: they **do not throw**, instead
wrapping the failure as an `Err` value of `Result`, which can be unpacked via
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

`std.list` and `std.range` provide the same iterator protocol. The iterator itself is a
`Tuple`-shaped state carrier.

> **Move semantics**: `next` and `has_next` both **move** the iterator (the signature has no `&`),
> so every use requires re-creating it, or simply use `for ... in`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next moved it; recreate it before taking the element
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
produce a result when consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

The following depend on OS capabilities and are **not exported** for the `wasm32` target:

| Scope                                                           | Requires       |
| --------------------------------------------------------------- | -------------- |
| All of `std.os`, all of `std.net`, all of `std.weak`            | File / network |
| All of `std.concurrent`                                         | Threads        |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | Standard I/O   |
| `std.time.sleep`                                                | Thread sleep   |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` and `std.io.print` / `println` / `format_fallback` are available on all targets.

## Known Gaps

The following issues were each confirmed by actually running the example while writing the
documentation; all have an open issue for tracking. After the fix, the corresponding page body
should be updated in sync (the generation area gate will automatically warn when the signature
changes):

| Location                                             | Issue                                                                             | Tracking |
| ---------------------------------------------------- | --------------------------------------------------------------------------------- | -------- |
| [`net.http_get`](./net#http_get) / `http_post`       | Placeholder implementation, does not send a request, returns a description string | #56      |
| [`os.open`](./os#open)                               | Handle is single-use; `open`→`write`→`close` will not compile                     | #337     |
| [`time.DateTime::*`](./time#datetime-字段访问不可用) | 8 accessors not callable from source (export names contain `::`)                  | #338     |
| [`time.parse_time`](./time#parse_time)               | `fmt` parameter is ignored; the return value cannot be used further               | #340     |
| [`math.clamp`](./math#clamp)                         | `min > max` panics the interpreter instead of returning an error                  | #339     |

## Documentation Maintenance

This directory uses a **generated + hand-written** hybrid structure:

- **Generated areas** (between the `<!-- stdlib:KEY start/end -->` markers): function summary tables
  and signature blocks, derived from `StdModule::exports()`. Signatures come byte-for-byte from
  `NativeExport::signature` and cannot drift from the implementation.
- **Hand-written areas** (outside the markers): module overview, borrow/move semantics, error model,
  known gaps, and examples.

Gates (run in CI alongside `cargo test --lib`):

| Gate              | Test                                          | Purpose                                         |
| ----------------- | --------------------------------------------- | ----------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated area must match `exports()`           |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output   |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover interface view |
| Examples runnable | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run     |

After `exports()` changes, use the healing tool to rewrite the generated areas:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code tables).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — Language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — User-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — Full table of `E6xxx` runtime error codes
