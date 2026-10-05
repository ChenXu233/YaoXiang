---
title: 'Standard Library Overview'
description: 'YaoXiang Standard Library Module Overview and Usage Conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized as modules. Each module is imported via `use` and
then called as `module_name.function_name(...)`. This directory contains the API reference
documentation, split by module.

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
| [`std.assert`](./assert)         | 1       | Assertions                                                                |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent encoding/decoding                           |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identifier                            |
| [`std.os`](./os)                 | 12      | File handles, environment variables, and working directory                |
| [`std.weak`](./weak)             | 2       | Arc / Weak references                                                     |

<!-- stdlib:index:modules end -->

### Modules Implemented in Pure YaoXiang

The following modules are implemented in `.yx` source files and embedded with the binary
(`src/std/yx_sources.rs`). They are **not** native `StdModule`, and therefore fall outside the
generated scope of the table above (`modules_for_docs()` in `src/std/gen_docs.rs:43` explicitly
excludes them). Their pages are hand-written, with signatures taken verbatim from the `.yx` source
files.

| Module                 | Source              | Description                                                             |
| ---------------------- | ------------------- | ----------------------------------------------------------------------- |
| [`std.list`](list)     | `src/std/list.yx`   | List add/remove, slicing, higher-order functions, and iterator protocol |
| [`std.json`](json)     | `src/std/json.yx`   | JSON parsing and serialization (RFC 8259)                               |
| [`std.option`](option) | `src/std/option.yx` | `Option(T)` optional value and sum type                                 |
| [`std.test`](test)     | `src/std/test.yx`   | Test assertion library (value semantics, RFC-036 §3)                    |

> `std.result` is a **dual-implementation** module: the native tool family (`src/std/result.rs`) and
> the pure-yx `Result` type (`src/std/result.yx`) are merged into a single export surface—`Result`
> variant construction goes through yx, while tools like `is_ok` / `unwrap` / `code` go through
> native. See [`std.result`](result) for details.

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

## Parameter Borrowing Conventions

The `&` in a signature indicates **read-only automatic borrow** (RFC-009 §2.8): the variable passed
by the caller is not moved and remains usable after the call. This is the default form for the many
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

Parameters without `&` are **passed by value**. Most "modifying" functions are therefore functional
in form, **consuming the source value and returning a new one**, rather than mutating in place:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

Each module's "Semantic categories" section lists which functions in that module borrow, which
consume, and which mutate in place. Pay special attention to **value retrieval and bounds**:
out-of-bounds or empty-list access in `list.get` / `list.first` / `list.last` / `list.slice` all
goes through `[]`'s bounds check and reports `E6003`—**they do not return a sentinel value**.

## Error Model

The standard library has two kinds of failure forms, labeled in each function entry as "Error" and
"Returns..." respectively:

| Form                     | Behavior                                          | Typical scenario                                                                       |
| ------------------------ | ------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Throws a runtime error   | Terminates current execution with an `E6xxx` code | Missing dictionary key `E6008`, index out of bounds `E6003`, assertion failure `E6005` |
| Returns a sentinel value | Does not interrupt; returns `Void` / `-1` / `""`  | `list.find_index` miss, `string.index_of` miss, missing environment variable           |

`list` is **implemented in pure YaoXiang** (`src/std/list.yx`): value-retrieval functions use `[]`
subscripting directly, without bounds clamping—an out-of-bounds access immediately reports `E6003`
(`src/std/list.yx:77-79`); the same applies to retrieving the first/last element of an empty list
(`src/std/list.yx:82-99`).

Common runtime error codes:

| Error code | Meaning                              | Trigger example                              |
| ---------- | ------------------------------------ | -------------------------------------------- |
| `E6003`    | Index out of bounds                  | `list.set(l, 99, v)`                         |
| `E6005`    | Assertion failure                    | `assert(false)`                              |
| `E6007`    | Generic runtime error                | File does not exist, `result.unwrap` failure |
| `E6008`    | Missing key                          | `dict.get(d, "nope")`                        |
| `E6010`    | Integer parse failure (as Err value) | `string.parse_int("abc")`                    |
| `E6011`    | Float parse failure (as Err value)   | `string.parse_float("abc")`                  |

See the [Error Code Reference](../error-code/) for the complete table.

`string.parse_int` / `string.parse_float` represent a third form: **they do not throw**, but wrap
the failure as the `Err` value of a `Result`, which can be unwrapped via [`std.result`](result) or
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

Both `std.list` and `std.range` provide `iter` / `has_next` / `next`, but **the parameter forms on
the two sides are opposite**—copying the usage from one to the other will not match:

| Function   | `std.list` (`src/std/list.yx`) | `std.range` (`src/std/range.rs`) |
| ---------- | ------------------------------ | -------------------------------- |
| `has_next` | `(it: &Iter(T)) -> Bool`       | `(it: Iterator(Any)) -> Bool`    |
| `next`     | `(it: &mut Iter(T)) -> T`      | `(it: &Iterator(Any)) -> Any`    |

In other words: on the `list` side the iterator is **borrowed** (`next` takes `&mut`, `has_next`
takes `&`), so the same iterator can yield elements consecutively; on the `range` side `has_next`
**consumes the iterator by value** (the signature has no `&`), so a fresh iterator is needed for
each check—only `next` borrows. For everyday traversal, just use `for ... in`—neither form requires
manual handling.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // On the list side it's a borrow: the same iterator can be consumed consecutively
    assert(list.next(it) == 1)
    assert(list.next(it) == 2)
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

[`range.map`](range#map) / [`range.filter`](range#filter) return **lazy** adapters, which only
produce results after being consumed by `collect` / `reduce` / `for_each` / `for ... in`:

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

| Scope                                       | Required     |
| ------------------------------------------- | ------------ |
| `std.os` all, `std.net` all, `std.weak` all | File/network |
| `std.concurrent` all                        | Threads      |
| `std.io.read_line`                          | Standard I/O |
| `std.fs` all                                | File system  |
| `std.time.sleep`                            | Thread sleep |

`std.string` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` / `std.assert`,
the pure-yx `std.list` / `std.json` / `std.option` / `std.test`, along with `std.io.print` /
`println` / `format_fallback`, are available on all targets.

## Known Gaps

The entries below were verified by actually running the examples one by one while writing the
documentation:

| Location                                                      | Original issue                                                   | Current status                                                                     |
| ------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`os.open`](os#open)                                          | Handle is one-shot; `open`→`write`→`close` does not compile      | Handle changed to pass by reference ✅                                             |
| [`time.datetime_*`](time#datetime-field-access)               | 8 accessors not callable from source (export name contains `::`) | Changed to flat names like `datetime_year`, etc. ✅                                |
| [`time.parse_time`](time#parse_time)                          | `fmt` parameter ignored; return value cannot be used further     | Parse step-by-step by fmt, returns `Int` ✅                                        |
| [`math.clamp`](math#clamp)                                    | `min > max` panics the interpreter instead of returning an error | Returns `E6007` ✅                                                                 |
| [`net.http_get`](net#http_get) / [`http_post`](net#http_post) | Was a placeholder implementation, did not send requests          | Changed to a real implementation (`ureq` sync blocking + rustls TLS, issue #56) ✅ |

Still open issues:

| Location                          | Issue                                                                                                                                    | Tracking                      |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| [`std.option`](option#known-gaps) | `Try(Option(T), T, Void)` instantiation not recognized by the type checker, `?` reports `E1081`; `from_error` reports `E6006` at runtime | See "Known Gaps" on that page |
| [`list.slice`](list#slice)        | Out-of-bounds indices are not clamped; reports `E6003` directly                                                                          | See the entry on that page    |

## Documentation Maintenance

This directory is a **generated + hand-written** hybrid structure:

- **Generated regions** (between `<!-- stdlib:KEY start/end -->` markers): function overviews and
  signature blocks, derived from `StdModule::exports()`. Signatures come byte-for-byte from
  `NativeExport::signature`; they cannot drift from the implementation.
- **Hand-written regions** (outside the markers): module overview, borrow/move semantics, error
  model, known gaps, and examples.

Gates (run in CI along with `cargo test --lib`):

| Gate              | Test                                          | Purpose                                             |
| ----------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated regions must match `exports()`            |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output       |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Examples must run | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run         |

> The generator only covers native `StdModule`; pages for pure-yx modules (`list` / `json` /
> `option` / `test`) contain no generated regions. Their validity is jointly guaranteed by the
> runnable-examples gate and orphan detection (page names must be in the whitelist in
> `src/std/yx_sources.rs`).

After `exports()` changes, use the remedy tool to rewrite the generated regions:

```bash
cargo run --example gen-stdlib-docs
```

Isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix` (RFC-013
code tables).

## Related Documents

- [Standard Library Specification](../language-spec/stdlib.md) — Standard library design conventions
  at the language level
- [FFI Specification](../language-spec/ffi.md) — User-side `native` extensions and C ABI bindings
- [Error Code Reference](../error-code/) — Complete table of `E6xxx` runtime error codes
