---
title: 'std.option'
description: 'Option optional values and sum types'
---

# std.option

`Option(T)`—an optional-value type that is either “present (`some`)” or “absent (`none`)”. It is the
**record-style sum type** exported by this module (RFC-010), using the same mechanism as
user-defined sum types: variant construction, `match`-based variant destructuring, and
exhaustiveness checking. Use `use std.option` before using it.

```yaoxiang
use std.option
```

> **This page is manually written.** This module is implemented in pure YaoXiang source
> (`src/std/option.yx`) and is not within the generation scope of `StdModule::exports()`, so there
> is no `<!-- stdlib:... -->` generated-section marker. The signatures in the table below are copied
> verbatim from that file's export declarations.

## Exports

| Export   | Signature           | Description                                    |
| -------- | ------------------- | ---------------------------------------------- |
| `Option` | `(T: Type) -> Type` | Sum type with the two variants `some` / `none` |

The module **exports only the single binding `Option`**. The four methods of `Try` are declared in
the type body (see below), but are **not module-level exports**—`o.is_failure(...)` reports
`E1042 field 'is_failure' not found in struct 'option'`. They are accessed using **method-call**
syntax: `o.is_failure()`.

## Type Body

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

- `some` — the value-carrying variant that holds one `T`
- `none` — the no-value variant that carries no payload
- `Try(Option(T), T, Void)` — the residual type on failure is `Void` (corresponding to the `NoneT`
  semantics of Rust's experimental `Try`)

Variant construction in expression position **must be type-qualified**: `Option(Int).some(5)`,
`Option(Int).none()`; the bare name `some(5)` is not a constructor call (see
[Syntax Specification §1.4.2](../language-spec/syntax.md)).

## Functions

The `Try` methods defined in the type body (`src/std/option.yx:23-45`):

| Method                | Signature                    | On `some`                     | On `none`                     |
| --------------------- | ---------------------------- | ----------------------------- | ----------------------------- |
| `is_failure()`        | `(self: &Option(T)) -> Bool` | `false`                       | `true`                        |
| `success()`           | `(self: &Option(T)) -> T`    | Returns payload               | `E6005` (dead end)            |
| `residual()`          | `(self: &Option(T)) -> Void` | `E6005` (dead end)            | Returns `void`                |
| `from_error(v: Void)` | `(v: Void) -> Option(T)`     | See [Known Gaps](#known-gaps) | See [Known Gaps](#known-gaps) |

The dead-end branches shown above are explicitly written as `assert(false)` in the type body
(`src/std/option.yx:33,40`). `Never <: T` makes these branches valid at the type level while
terminating them at runtime with `E6005`.

```yaoxiang
use std.option

main: () -> Void = {
    s = Option(Int).some(7)
    println(s.is_failure())   // false
    println(s.success())      // 7
    // s.residual() would diverge: the some arm is a dead end (E6005) and cannot be called

    n = Option(Int).none()
    println(n.is_failure())   // true
    println(n.residual())     // void
    // n.success() likewise diverges: the none arm is a dead end (E6005)
}
```

## match Variant Destructuring

Destructuring `Option` requires the variant set to be in scope, so first `use std.option` (see
[Syntax Specification §2.8](../language-spec/syntax.md)). Exhaustiveness checking considers the
complete variant set—both the `some` and `none` arms are required, although the catch-all `_` arm
may be omitted.

```yaoxiang
use std.option

name_of: (Int) -> Option(Int) = (x) => {
    if x > 0 {
        return Option(Int).some(x)
    }
    return Option(Int).none()
}

main: () -> Void = {
    for x in [1, -1] {
        v = name_of(x)
        match v {
            some(n) -> println(n),
            none() -> println("no value"),
        }
    }
}
```

## Known Gaps

The following three items are “declared but currently do not work”; each was verified with
`yaoxiang-rs run`:

1. **`?` propagation does not apply to `Option`.** The type body instantiates
   `Try(Option(T), T, Void)`, but the type checker recognizes only `Result`: `o?` reports
   `E1081 ? is only allowed in functions returning a type that implements Try`. `?` works normally
   on `Result`, which can be used for comparison.
2. **`from_error` cannot be called.** When written as the method `o.from_error()`, it compiles but
   reports `E6006 Function not found: from_error` at runtime; when written as
   `Option(T).from_error(...)`, it reports `E1042` at compile time. It is the internal bridge for
   `?` propagation; because `?` is unavailable, this bridge has no call site.
3. **There is no `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`.** These names have zero
   occurrences in the standard library; use `is_failure()` to test for failure and `success()` to
   obtain the payload. The corresponding utilities for `Result` are in [`std.result`](result).

## Related

- [`std.result`](result) — `Result(T, E)`; `?` propagation currently works **only** on this type
- [`std.assert`](assert) — source of `assert(false)` in dead-end branches
- [Syntax Specification §2.8](../language-spec/syntax.md) — variant destructuring and exhaustiveness
  checking
