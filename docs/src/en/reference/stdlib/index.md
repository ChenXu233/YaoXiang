---
title: 'Standard Library Overview'
description: 'YaoXiang standard library module overview and usage conventions'
---

# Standard Library Reference

The YaoXiang standard library (`std`) is organized into modules. Each module is imported via `use`
and called as `Module.function_name(...)`. This directory contains the API reference documentation
broken down by module.

## Module Index

<!-- stdlib:index:modules start -->

| Module                           | Exports | Description                                                          |
| -------------------------------- | ------- | -------------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11      | Conversion of any value to String                                    |
| [`std.dict`](./dict)             | 11      | Dictionary read/write, key-value views, and merging                  |
| [`std.fs`](./fs)                 | 22      | File, directory, and path operations                                 |
| [`std.io`](./io)                 | 4       | Standard output, standard input, and formatting                      |
| [`std.math`](./math)             | 18      | Integer, float, and trigonometric functions, with PI/E/TAU constants |
| [`std.string`](./string)         | 21      | String search, splitting, formatting, and parsing                    |
| [`std.time`](./time)             | 14      | Timestamps, formatting, and DateTime field access                    |
| [`std.result`](./result)         | 8       | Construction and unwrapping of Result and Error                      |
| [`std.range`](./range)           | 10      | Range iteration, predicates, and lazy adapters                       |
| [`std.assert`](./assert)         | 1       | Assertion                                                            |
| [`std.net`](./net)               | 4       | HTTP requests and URL percent-encoding/decoding                      |
| [`std.concurrent`](./concurrent) | 3       | Sleep, yield scheduling, and thread identifier                       |
| [`std.os`](./os)                 | 12      | File handles, environment variables, and working directory           |
| [`std.weak`](./weak)             | 2       | Arc / Weak weak references                                           |

<!-- stdlib:index:modules end -->

### Modules Implemented in Pure YaoXiang

The following modules are implemented in `.yx` source and embedded with the binary
(`src/std/yx_sources.rs`). They are **not** native `StdModule`s, so they are not included in the
generated range of the table above (`modules_for_docs()` at `src/std/gen_docs.rs:43` explicitly
excludes them). Their pages are hand-written, and the signatures are taken verbatim from the `.yx`
source files.

| Module                   | Source              | Description                                                             |
| ------------------------ | ------------------- | ----------------------------------------------------------------------- |
| [`std.list`](./list)     | `src/std/list.yx`   | List add/remove, slicing, higher-order functions, and iterator protocol |
| [`std.json`](./json)     | `src/std/json.yx`   | JSON parsing and serialization (RFC 8259)                               |
| [`std.option`](./option) | `src/std/option.yx` | `Option(T)` optional value and sum type                                 |
| [`std.test`](./test)     | `src/std/test.yx`   | Test assertion library (value semantics, RFC-036 §3)                    |

> `std.result` is a **dual-implementation** module: the native utility family (`src/std/result.rs`)
> and the pure yx `Result` type (`src/std/result.yx`) are merged into the same export surface —
> `Result` variant construction goes through yx, while tools like `is_ok` / `unwrap` / `code` go
> through native. See [`std.result`](./result) for details.

## Import Conventions

Whole-module import:

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

The `&` in a signature indicates **automatic read-only borrowing** (RFC-009 §2.8): the variable
passed in by the caller is not moved, and remains usable after the call. This is the default form
for the many read-only functions in the standard library.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // All three calls only borrow nums read-only; it's still usable afterward
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

Parameters without `&` are **passed by value**. For this reason, most "mutating" functions take the
form of **consuming the source value and returning a new value** — a functional style rather than
in-place modification:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // Returns a new list; base has been moved
    assert(list.len(extended) == 3)
}
```

The "Semantic Classification" section on each module's page lists which functions in that module
borrow, which consume, and which modify in place. Of particular note are **value access and
bounds**: `list.get` / `list.first` / `list.last` / `list.slice` all go through the `[]` bounds
check on out-of-bounds access or an empty list, and report `E6003` — they **do not return a sentinel
value**.

## Error Model

The standard library has two forms of failure, each marked in function entries as "Error" and
"Returns..." respectively:

| Form                     | Behavior                                              | Typical scenario                                                                       |
| ------------------------ | ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Throws a runtime error   | Terminates the current execution with an `E6xxx` code | Dictionary missing key `E6008`, index out of bounds `E6003`, assertion failure `E6005` |
| Returns a sentinel value | Does not interrupt; returns `Void` / `-1` / `""`      | `list.find_index` no match, `string.index_of` no match, missing environment variable   |

`list` is implemented in **pure YaoXiang** (`src/std/list.yx`): value-access functions directly use
`[]` subscripting without bounds clamping, so out-of-bounds access reports `E6003`
(`src/std/list.yx:77-79`); the same applies to fetching the first/last element of an empty list
(`src/std/list.yx:82-99`).

Common runtime error codes:

| Code    | Meaning                              | Trigger example                              |
| ------- | ------------------------------------ | -------------------------------------------- |
| `E6003` | Index out of bounds                  | `list.set(l, 99, v)`                         |
| `E6005` | Assertion failure                    | `assert(false)`                              |
| `E6007` | Generic runtime error                | File does not exist, `result.unwrap` failure |
| `E6008` | Missing key                          | `dict.get(d, "nope")`                        |
| `E6010` | Integer parse failure (as Err value) | `string.parse_int("abc")`                    |
| `E6011` | Float parse failure (as Err value)   | `string.parse_float("abc")`                  |

See the [Error Code Reference](../error-code/) for the complete table of error codes.

`string.parse_int` / `string.parse_float` belong to a third form: **they do not throw**, and instead
wrap the failure as the `Err` value of a `Result`, which can be unwrapped via
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

## Iterator Protocol

Both `std.list` and `std.range` provide `iter` / `has_next` / `next`, but **the parameter forms on
the two sides are opposite**: copying the form from either side to the other results in a mismatch:

| Function   | `std.list` (`src/std/list.yx`) | `std.range` (`src/std/range.rs`) |
| ---------- | ------------------------------ | -------------------------------- |
| `has_next` | `(it: &Iter(T)) -> Bool`       | `(it: Iterator(Any)) -> Bool`    |
| `next`     | `(it: &mut Iter(T)) -> T`      | `(it: &Iterator(Any)) -> Any`    |

In other words: the `list` side **borrows** the iterator (`next` takes `&mut`, `has_next` takes
`&`), so the same iterator can be used to take elements consecutively; on the `range` side,
`has_next` **consumes the iterator by value** (the signature has no `&`), so a new one must be
created for each check, and only `next` borrows. For everyday traversal, just use `for ... in` and
you don't have to worry about either form.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // list side is borrowed: the same iterator can be taken from consecutively
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

[`range.map`](./range#map) / [`range.filter`](./range#filter) return **lazy** adapters, which only
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

The following depend on operating system capabilities and are **not exported** on the `wasm32`
target:

| Scope                                       | Required     |
| ------------------------------------------- | ------------ |
| `std.os` all, `std.net` all, `std.weak` all | File/network |
| `std.concurrent` all                        | Threading    |
| `std.io.read_line`                          | Standard I/O |
| `std.fs` all                                | Filesystem   |
| `std.time.sleep`                            | Thread sleep |

`std.string` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` / `std.assert`,
the pure yx `std.list` / `std.json` / `std.option` / `std.test`, and `std.io.print` / `println` /
`format_fallback` are available on all targets.

## Known Gaps

The following items were each individually verified by actually running examples while writing this
documentation:

| Location                                                          | Original issue                                                            | Current status                                                                            |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| [`os.open`](./os#open)                                            | Handle was one-shot; `open`→`write`→`close` could not compile             | Handle changed to pass by reference ✅                                                    |
| [`time.datetime_*`](./time#datetime-字段访问)                     | 8 accessors could not be called from source (export names contained `::`) | Changed to flat names like `datetime_year` ✅                                             |
| [`time.parse_time`](./time#parse_time)                            | `fmt` parameter was ignored; the return value could not be used further   | Step-by-step parsing by fmt, returns `Int` ✅                                             |
| [`math.clamp`](./math#clamp)                                      | `min > max` panicked the interpreter instead of returning an error        | Returns `E6007` ✅                                                                        |
| [`net.http_get`](./net#http_get) / [`http_post`](./net#http_post) | Was a placeholder implementation, did not send requests                   | Changed to a real implementation (`ureq` synchronous blocking + rustls TLS, issue #56) ✅ |

Still open issues:

| Location                          | Issue                                                                                                                                       | Tracking                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| [`std.option`](./option#已知缺口) | `Try(Option(T), T, Void)` instantiation is not recognized by the type checker, `?` reports `E1081`; `from_error` reports `E6006` at runtime | See "Known Gaps" on that page |
| [`list.slice`](./list#slice)      | Out-of-bounds indices are not clamped, directly `E6003`                                                                                     | See entry on that page        |

## Documentation Maintenance

This directory is a **generated + hand-written** hybrid structure:

- **Generated sections** (between `<!-- stdlib:KEY start/end -->` markers): the function summary
  tables and signature blocks, derived from `StdModule::exports()`. The signatures come
  byte-for-byte from `NativeExport::signature`, so they cannot drift from the implementation.
- **Hand-written sections** (outside the markers): module overview, borrow/move semantics, error
  model, known gaps, and examples.

CI gates (run with `cargo test --lib` in CI):

| Gate              | Test                                          | Purpose                                             |
| ----------------- | --------------------------------------------- | --------------------------------------------------- |
| Drift detection   | `test_stdlib_docs_match_generation`           | Generated sections must match `exports()`           |
| Orphan detection  | `test_stdlib_docs_has_no_orphan_module_pages` | Module pages must not exceed generator output       |
| Coverage          | `test_stdlib_docs_covers_interface_modules`   | Documented module set must cover the interface view |
| Examples runnable | `test_stdlib_docs_examples_run`               | Every ```yaoxiang example must actually run         |

> The generator only covers native `StdModule`s; the pages of pure yx modules (`list` / `json` /
> `option` / `test`) contain no generated sections, and their validity is jointly guaranteed by the
> example-runnability gate and orphan detection (the page name is in the whitelist in
> `src/std/yx_sources.rs`).

After `exports()` changes, use the cure tool to rewrite the generated sections:

```bash
cargo run --example gen-stdlib-docs
```

It is isomorphic to `gen-std-interfaces` (RFC-037 interface view) and `tools/code-tables --fix`
(RFC-013 code tables).

## Related Documentation

- [Standard Library Specification](../language-spec/stdlib.md) — language-level standard library
  design conventions
- [FFI Specification](../language-spec/ffi.md) — user-side `native` extension and C ABI bindings
- [Error Code Reference](../error-code/) — complete table of `E6xxx` runtime error codes
