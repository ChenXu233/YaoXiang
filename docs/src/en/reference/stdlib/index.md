---
title: 'Standard Library Overview'
description: 'Overview of YaoXiang standard library modules and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. Each module is imported with `use`
and then invoked as `module.function(...)`. This directory contains API reference documentation
organized by module.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Number of Exports | Description                                                                        |
| -------------------------------- | ----------------- | ---------------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11                | Conversion of arbitrary values to `String`                                         |
| [`std.dict`](./dict)             | 11                | Dictionary reads and writes, key-value views, and merging                          |
| [`std.fs`](./fs)                 | 22                | File, directory, and path operations                                               |
| [`std.io`](./io)                 | 4                 | Standard output, standard input, and formatting                                    |
| [`std.math`](./math)             | 18                | Integer, floating-point, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 21                | String searching, splitting, formatting, and parsing                               |
| [`std.time`](./time)             | 14                | Timestamps, formatting, and date component access                                  |
| [`std.result`](./result)         | 8                 | Construction and unpacking of `Result` and `Error`                                 |
| [`std.range`](./range)           | 10                | Range iteration, predicates, and lazy adapters                                     |
| [`std.assert`](./assert)         | 1                 | Assertions                                                                         |
| [`std.net`](./net)               | 4                 | HTTP requests and URL percent encoding/decoding                                    |
| [`std.concurrent`](./concurrent) | 3                 | Sleeping, scheduler yielding, and thread identification                            |
| [`std.os`](./os)                 | 12                | File handles, environment variables, and working directory                         |
| [`std.weak`](./weak)             | 2                 | Arc / Weak weak references                                                         |

<!-- stdlib:index:modules end -->

### Modules Implemented in Pure YaoXiang

The following modules are implemented in pure YaoXiang and embedded in the binary
(`src/std/yx_sources.rs`). They are **not** native `StdModule` implementations and therefore fall
outside the generated range in the table above (`modules_for_docs()` at `src/std/gen_docs.rs:43`
explicitly excludes them). Their pages are handwritten, and their signatures are copied verbatim
from the `.yx` source files.

| Module                 | Source              | Description                                                                             |
| ---------------------- | ------------------- | --------------------------------------------------------------------------------------- |
| [`std.list`](list)     | `src/std/list.yx`   | List insertion and deletion, slicing, higher-order functions, and the iterator protocol |
| [`std.json`](json)     | `src/std/json.yx`   | JSON parsing and serialization (RFC 8259)                                               |
| [`std.option`](option) | `src/std/option.yx` | `Option(T)` optional values and sum types                                               |
| [`std.test`](test)     | `src/std/test.yx`   | Test assertion library (value semantics, RFC-036 §3)                                    |

> `std.result` is a **dual-implementation** module: the native utility family (`src/std/result.rs`)
> and the pure yx `Result` type (`src/std/result.yx`) are merged into a single export
> surface—`Result` variant construction uses yx, while utilities such as `is_ok` / `unwrap` / `code`
> use native. See [`std.result`](result).

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

You can also import individual names from a module, including constants:

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## Parameter Borrowing Conventions

The `&` in a signature denotes **automatic read-only borrowing** (RFC-009 §2.8): variables supplied
by the caller are not moved and remain usable after the call. This is the default form for many
read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // All three calls borrow nums read-only, so it remains usable afterward
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

A parameter without `&` is **passed by value**. Consequently, most “modifying” functions follow a
functional pattern that **consumes the source value and returns a new value**, rather than modifying
it in place:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

The “Semantic Classification” section of each module page lists which functions borrow, consume, or
modify values in place. Pay particular attention to **value access and boundaries**: `list.get` /
`list.first` / `list.last` / `list.slice` with an out-of-range index or on an empty list all use
`[]` bounds checks and report `E6003`; they **do not return sentinel values**.

## Error Model

The standard library has two types of failure. Entries for individual functions indicate them
separately as “Error” and “Returns …”:

| Form                    | Behavior                                                   | Typical Scenario                                                                             |
| ----------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Runtime error thrown    | Terminates the current execution with an `E6xxx` code      | Missing dictionary key `E6008`, index out of bounds `E6003`, assertion failure `E6005`       |
| Sentinel value returned | Does not interrupt execution; returns `Void` / `-1` / `""` | No match for `list.find_index`, no match for `string.index_of`, missing environment variable |

`list` is a **pure YaoXiang implementation** (`src/std/list.yx`): value-access functions index
directly with `[]` and do not clamp bounds; an out-of-range access immediately reports `E6003`
(`src/std/list.yx:77-79`). Getting the first or last element of an empty list behaves the same way
(`src/std/list.yx:82-99`).

Common runtime error codes:

| Error Code | Meaning                                            | Example Trigger                            |
| ---------- | -------------------------------------------------- | ------------------------------------------ |
| `E6003`    | Index out of bounds                                | `list.set(l, 99, v)`                       |
| `E6005`    | Assertion failure                                  | `assert(false)`                            |
| `E6007`    | Generic runtime error                              | File does not exist, `result.unwrap` fails |
| `E6008`    | Missing key                                        | `dict.get(d, "nope")`                      |
| `E6010`    | Integer parsing failure (as an `Err` value)        | `string.parse_int("abc")`                  |
| `E6011`    | Floating-point parsing failure (as an `Err` value) | `string.parse_float("abc")`                |

See the [Error Code Reference](../error-code/) for the complete table of error codes.

`string.parse_int` / `string.parse_float` belong to a third form: they **do not throw**, but wrap
failures into the `Err` value of a `Result`. You can unpack them with [`std.result`](result) or
propagate them with `?`.

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

Both `std.list` and `std.range` provide `iter` / `has_next` / `next`, but the **parameter forms on
the two sides are opposite**. Using the form from either side with the other will result in a
mismatch:

| Function   | `std.list` (`src/std/list.yx`) | `std.range` (`src/std/range.rs`) |
| ---------- | ------------------------------ | -------------------------------- |
| `has_next` | `(it: &Iter(T)) -> Bool`       | `(it: Iterator(Any)) -> Bool`    |
| `next`     | `(it: &mut Iter(T)) -> T`      | `(it: &Iterator(Any)) -> Any`    |

In other words, the `list` side **borrows** the iterator (`next` takes `&mut`, while `has_next`
takes `&`), so the same iterator can yield elements repeatedly. On the `range` side, `has_next`
**consumes the iterator by value** (its signature has no `&`), so it must be recreated for every
check; `next` borrows it. For routine traversal, use `for ... in` directly and neither form needs to
be handled manually.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // The list side borrows: the same iterator can retrieve elements repeatedly
    assert(list.next(it) == 1)
    assert(list.next(it) == 2)
}
```

For routine traversal, use `for ... in` directly:

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

[`range.map`](range#map) / [`range.filter`](range#filter) return **lazy** adapters. A result is
produced only after they are consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

| Scope                                                | Requirement  |
| ---------------------------------------------------- | ------------ |
| All of `std.os`, all of `std.net`, all of `std.weak` | File/network |
| All of `std.concurrent`                              | Thread       |
| `std.io.read_line`                                   | Standard I/O |
| All of `std.fs`                                      | File system  |
| `std.time.sleep`                                     | Thread sleep |

`std.string` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` / `std.assert`,
the pure yx `std.list` / `std.json` / `std.option` / `std.test`, and `std.io.print` / `println` /
`format_fallback` are available on all targets.

## Known Gaps

Each of the following entries was individually confirmed by actually running its examples while the
documentation was being written:

| Location                                                      | Original Issue                                                                         | Current Status                                                                      |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [`os.open`](os#open)                                          | The handle was one-shot, so `open`→`write`→`close` could not compile                   | Handles are now passed by reference ✅                                              |
| [`time.datetime_*`](time#日期分量访问)                        | The eight accessors could not be called from source code (export names contained `::`) | Changed to flat names such as `datetime_year`, etc. ✅                              |
| [`time.parse_time`](time#parse_time)                          | The `fmt` argument was ignored, and the return value could not be used further         | Now parsed step by step according to `fmt`; returns `Int` ✅                        |
| [`math.clamp`](math#clamp)                                    | `min > max` would panic the interpreter instead of returning an error                  | Returns `E6007` ✅                                                                  |
| [`net.http_get`](net#http_get) / [`http_post`](net#http_post) | Used to be placeholder implementations that did not send requests                      | Now real implementations (`ureq` synchronous blocking + `rustls` TLS, issue #56) ✅ |

The following issues remain open:

| Location                        | Issue                                                                                                                                               | Tracking                                 |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| [`std.option`](option#已知缺口) | The `Try(Option(T), T, Void)` instantiation is not recognized by the type checker, and `?` reports `E1081`; `from_error` reports `E6006` at runtime | See “Known Gaps” on that page            |
| [`list.slice`](list#slice)      | Out-of-range indices are not clamped and result directly in `E6003`                                                                                 | See the corresponding entry on that page |

## Documentation Maintenance

This directory uses a hybrid **generated + handwritten** structure:

- **Generated regions** (between `<!-- stdlib:KEY start/end -->` markers): function overview tables
  and signature blocks, derived from `StdModule::exports()`. Signatures are taken byte-for-byte from
  `NativeExport::signature` and cannot drift from the implementation.
- **Handwritten regions** (outside the markers): module overviews, borrowing/moving semantics, error
  model, known gaps, and examples.

The gate (run with `cargo test --lib` in CI) consists of the following:

| Gate              | Test                                          | Purpose                                                           |
| ----------------- | --------------------------------------------- | ----------------------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated regions must match `exports()`                          |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | The number of module pages must not exceed the generator's output |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | The documented module set must cover the interface view           |
| Runnable examples | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run                       |

> The generator covers only native `StdModule`; pages for pure yx modules (`list` / `json` /
> `option` / `test`) contain no generated regions. Their validity is jointly ensured by the
> runnable-examples gate and orphan detection (page names are included in the allowlist in
> `src/std/yx_sources.rs`).

After changing `exports()`, rewrite the generated regions using the healing tool:

```bash
cargo run --example gen-stdlib-docs
```

This is isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code table).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — Language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — User-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — Complete table of `E6xxx` runtime error codes
