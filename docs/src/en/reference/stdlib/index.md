---
title: 'Standard Library Overview'
description: 'Overview of YaoXiang standard library modules and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. Each module is imported with `use`
and then invoked as `module.function(...)`. This directory contains API reference documentation
split by module.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                                        |
| -------------------------------- | ------- | ---------------------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion from arbitrary values to String                                         |
| [`std.dict`](./dict)             | 11      | Dictionary access, key-value views, and merging                                    |
| [`std.fs`](./fs)                 | 22      | File, directory, and path operations                                               |
| [`std.io`](./io)                 | 4       | Standard output, standard input, and formatting                                    |
| [`std.math`](./math)             | 18      | Integer, floating-point, and trigonometric functions, including PI/E/TAU constants |
| [`std.string`](./string)         | 21      | String searching, splitting, formatting, and parsing                               |
| [`std.time`](./time)             | 14      | Timestamps, formatting, and date component access                                  |
| [`std.result`](./result)         | 8       | Construction and unpacking of Result and Error                                     |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                                     |
| [`std.assert`](./assert)         | 1       | Assertions                                                                         |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent encoding/decoding                                    |
| [`std.concurrent`](./concurrent) | 3       | Sleeping, yielding the scheduler, and thread identification                        |
| [`std.os`](./os)                 | 12      | File handles, environment variables, and working directory                         |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak references                                                         |

<!-- stdlib:index:modules end -->

### Modules Implemented in Pure YaoXiang

The following modules are implemented in `.yx` source code and embedded in the binary
(`src/std/yx_sources.rs`). They are **not** native `StdModule`s, so they are outside the generation
scope of the table above (`modules_for_docs()` at `src/std/gen_docs.rs:43` explicitly excludes
them). Their pages are handwritten, and their signatures are taken verbatim from the `.yx` source
files.

| Module                 | Source              | Description                                                                             |
| ---------------------- | ------------------- | --------------------------------------------------------------------------------------- |
| [`std.list`](list)     | `src/std/list.yx`   | List insertion and deletion, slicing, higher-order functions, and the iterator protocol |
| [`std.json`](json)     | `src/std/json.yx`   | JSON parsing and serialization (RFC 8259)                                               |
| [`std.option`](option) | `src/std/option.yx` | `Option(T)` optional values and sum types                                               |
| [`std.test`](test)     | `src/std/test.yx`   | Test assertion library (value semantics, RFC-036 §3)                                    |

> `std.result` is a **dual-implementation** module: the native utility family (`src/std/result.rs`)
> and the pure yx `Result` type (`src/std/result.yx`) are combined into a single export
> surface—`Result` variant construction goes through yx, while utilities such as `is_ok` / `unwrap`
> / `code` go through native. See [`std.result`](result).

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

A `&` in a signature denotes a **read-only automatic borrow** (RFC-009 §2.8): variables passed by
the caller are not moved and remain usable after the call. This is the default form for many
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

A parameter without `&` is **passed by value**. Consequently, most "modifying" functions follow a
**functional form that consumes the source value and returns a new value**, rather than modifying it
in place:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

The “Semantic Categories” section of each module page lists which functions in that module borrow,
consume, or modify values in place. Pay particular attention to **value retrieval and bounds**:
`list.get` / `list.first` / `list.last` / `list.slice` all use `[]`'s bounds checks and report
`E6003` when out of bounds or when the list is empty; they **do not return sentinel values**.

## Error Model

The standard library has two forms of failure, labeled “Error” and “Return …” in the individual
function entries:

| Form                    | Behavior                                                   | Typical use cases                                                                                   |
| ----------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Throw a runtime error   | Terminates the current execution with an `E6xxx` code      | Missing dictionary key `E6008`, out-of-bounds index `E6003`, assertion failure `E6005`              |
| Return a sentinel value | Does not interrupt execution; returns `Void` / `-1` / `""` | `list.find_index` finds no match, `string.index_of` finds no match, environment variable is missing |

`list` is a **pure YaoXiang implementation** (`src/std/list.yx`): value-access functions use `[]`
indexing directly without clamping bounds, so an out-of-bounds access immediately results in `E6003`
(`src/std/list.yx:77-79`); retrieving the first or last element of an empty list behaves the same
way (`src/std/list.yx:82-99`).

Common runtime error codes:

| Error code | Meaning                                                   | Trigger example                            |
| ---------- | --------------------------------------------------------- | ------------------------------------------ |
| `E6003`    | Index out of bounds                                       | `list.set(l, 99, v)`                       |
| `E6005`    | Assertion failure                                         | `assert(false)`                            |
| `E6007`    | Generic runtime error                                     | File does not exist, `result.unwrap` fails |
| `E6008`    | Missing key                                               | `dict.get(d, "nope")`                      |
| `E6010`    | Integer parsing failure (returned as an Err value)        | `string.parse_int("abc")`                  |
| `E6011`    | Floating-point parsing failure (returned as an Err value) | `string.parse_float("abc")`                |

For the complete error-code table, see the [Error Code Reference](../error-code/).

`string.parse_int` / `string.parse_float` use a third form: they **do not throw an error**; instead,
they wrap failures in and return an `Err` value of `Result`, which can be unpacked with
[`std.result`](result) or propagated with `?`.

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
the two sides are opposite**. Copying the syntax from either side to the other will result in a
mismatch:

| Function   | `std.list` (`src/std/list.yx`) | `std.range` (`src/std/range.rs`) |
| ---------- | ------------------------------ | -------------------------------- |
| `has_next` | `(it: &Iter(T)) -> Bool`       | `(it: Iterator(Any)) -> Bool`    |
| `next`     | `(it: &mut Iter(T)) -> T`      | `(it: &Iterator(Any)) -> Any`    |

In other words, on the `list` side, the iterator is **borrowed** (`next` takes `&mut`, and
`has_next` takes `&`), so the same iterator can be used to retrieve elements successively. On the
`range` side, `has_next` **consumes the iterator by value** (its signature has no `&`), so it must
be recreated for every check; only `next` borrows it. For everyday iteration, use `for ... in`
directly and you need not worry about either form.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // On the list side, it is borrowed: the same iterator can retrieve elements repeatedly
    assert(list.next(it) == 1)
    assert(list.next(it) == 2)
}
```

For everyday iteration, use `for ... in` directly:

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

[`range.map`](range#map) / [`range.filter`](range#filter) return **lazy** adapters, which produce
results only after being consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

The following depend on operating-system capabilities and are **not exported** for `wasm32` targets:

| Scope                                                    | Requires      |
| -------------------------------------------------------- | ------------- |
| All of `std.os`, all of `std.net`, and all of `std.weak` | Files/network |
| All of `std.concurrent`                                  | Threads       |
| `std.io.read_line`                                       | Standard I/O  |
| All of `std.fs`                                          | File system   |
| `std.time.sleep`                                         | Thread sleep  |

`std.string` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` / `std.assert`,
the pure yx `std.list` / `std.json` / `std.option` / `std.test`, and `std.io.print` / `println` /
`format_fallback` are available on all targets.

## Known Gaps

The following items were each verified by actually running the examples while writing this
documentation:

| Location                                                      | Original issue                                                                              | Current status                                                                     |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`os.open`](os#open)                                          | The handle is single-use, so `open`→`write`→`close` cannot compile                          | Handles are now passed by reference ✅                                             |
| [`time.datetime_*`](time#date-component-access)               | Eight accessors cannot be called from source code because their exported names contain `::` | Flattened names such as `datetime_year` are now used ✅                            |
| [`time.parse_time`](time#parse_time)                          | The `fmt` parameter is ignored; the return value cannot be used further                     | Parsing now advances according to fmt and returns an `Int` ✅                      |
| [`math.clamp`](math#clamp)                                    | `min > max` panics the interpreter instead of returning an error                            | Now returns `E6007` ✅                                                             |
| [`net.http_get`](net#http_get) / [`http_post`](net#http_post) | It was a placeholder implementation and did not issue requests                              | Now a real implementation (`ureq` synchronous blocking + rustls TLS, issue #56) ✅ |

Open issues:

| Location                        | Issue                                                                                                                                            | Tracking                                 |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------- |
| [`std.option`](option#已知缺口) | The instantiation of `Try(Option(T), T, Void)` is not accepted by the type checker; `?` reports `E1081`; `from_error` reports `E6006` at runtime | See “Known Gaps” on that page            |
| [`list.slice`](list#slice)      | Out-of-bounds indices are not clamped and directly result in `E6003`                                                                             | See the corresponding entry on that page |

## Documentation Maintenance

This directory uses a **generated + handwritten** hybrid structure:

- **Generated sections** (between `<!-- stdlib:KEY start/end -->` markers): function overview tables
  and signature blocks derived from `StdModule::exports()`. Signatures are copied byte-for-byte from
  `NativeExport::signature`, so they cannot drift from the implementation.
- **Handwritten sections** (outside the markers): module overviews, borrowing/movement semantics,
  the error model, known gaps, and examples.

CI gates (run alongside `cargo test --lib`):

| Gate              | Test                                          | Purpose                                                 |
| ----------------- | --------------------------------------------- | ------------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated sections must match `exports()`               |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not outnumber the generator's output  |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | The documented module set must cover the interface view |
| Examples runnable | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run             |

> The generator covers only native `StdModule`s; pages for pure yx modules (`list` / `json` /
> `option` / `test`) contain no generated sections. Their validity is jointly ensured by the
> runnable-examples gate and orphan detection (the page names are on the allowlist in
> `src/std/yx_sources.rs`).

After changing `exports()`, use the repair tool to rewrite the generated sections:

```bash
cargo run --example gen-stdlib-docs
```

This is analogous to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code table).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — Language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — User-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — Complete table of `E6xxx` runtime error codes
