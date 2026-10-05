---
title: 'std.option'
description: 'Option: optional value and sum type'
---

# std.option

`Option(T)` — an optional value type representing "has a value (`some`) or no value (`none`)". It is
the **record-style sum type** exported by this module (RFC-010), using the same mechanism as
user-defined sum types: variant construction, match variant destructuring, and exhaustiveness
checking. `use std.option` before use.

```yaoxiang
use std.option
```

> **This page is hand-written.** This module is implemented in pure YaoXiang source code
> (`src/std/option.yx`), outside the generation scope of `StdModule::exports()`, so it has no
> `<!-- stdlib:... -->` generation block marker. The signatures in the table below are taken
> verbatim from the export declarations of that file.

## Exports

| Export   | Signature           | Description                                |
| -------- | ------------------- | ------------------------------------------ |
| `Option` | `(T: Type) -> Type` | sum type with two variants `some` / `none` |

The module **exports only this single binding `Option`**. The four methods of `Try` are declared in
the type body (see below), **not as module-level exports** — `option.is_failure(...)` reports
`E1042 field 'is_failure' not found in struct 'option'`. They are accessed using the **method call**
syntax `o.is_failure()`.

## Type body

```
pub Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

- `some` — the value variant carrying one `T`
- `none` — the value-less variant carrying no payload
- `Try(Option(T), T, Void)` — the residual type on failure is `Void` (corresponding to the `NoneT`
  semantics of the Rust `Try` experiment)

Variant construction at expression position **must be type-qualified**: `Option(Int).some(5)`,
`Option(Int).none()`; the bare name `some(5)` is not a constructor call (see
[Syntax Specification §1.4.2](../language-spec/syntax.md)).

## Functions

The `Try` methods defined in the type body (`src/std/option.yx:23-45`):

| Method                | Signature                    | On `some`                   | On `none`                   |
| --------------------- | ---------------------------- | --------------------------- | --------------------------- |
| `is_failure()`        | `(self: &Option(T)) -> Bool` | `false`                     | `true`                      |
| `success()`           | `(self: &Option(T)) -> T`    | returns payload             | `E6005` (dead end)          |
| `residual()`          | `(self: &Option(T)) -> Void` | `E6005` (dead end)          | returns `void`              |
| `from_error(v: Void)` | `(v: Void) -> Option(T)`     | see [Known Gaps](#已知缺口) | see [Known Gaps](#已知缺口) |

The "dead ends" in the latter three are explicit `assert(false)` written in the type body
(`src/std/option.yx:33,40`); `Never <: T` makes these branches legal at the type level, terminating
with `E6005` at runtime.

```yaoxiang
use std.option

main: () -> Void = {
    s = Option(Int).some(7)
    println(s.is_failure())   // false
    println(s.success())      // 7
    // s.residual() will diverge: the some arm is a dead end (E6005), cannot be called

    n = Option(Int).none()
    println(n.is_failure())   // true
    println(n.residual())     // void
    // n.success() also diverges: the none arm is a dead end (E6005)
}
```

## Match variant destructuring

Destructuring `Option` requires the variant set to be in scope, i.e., `use std.option` first (see
[Syntax Specification §2.8](../language-spec/syntax.md)). Exhaustiveness checking examines the full
variant set — both `some` and `none` arms must be written; the catch-all arm `_` is exempt.

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
            some(n) => println(n),
            none() => println("no value"),
        }
    }
}
```

## Known gaps

The following three "are declared but currently don't work"; each one verified by `yaoxiang-rs run`
testing:

1. **`?` propagation does not apply to `Option`.** The type body instantiates
   `Try(Option(T), T, Void)`, but the type checker only recognizes `Result`: `o?` reports
   `E1081 ? is only allowed in functions returning a type that implements Try`. `?` works normally
   on `Result`, which can serve as a control.
2. **`from_error` cannot be called.** Written as a method `o.from_error()` it compiles but reports
   `E6006 Function not found: from_error` at runtime; written as `Option(T).from_error(...)` it
   reports `E1042` at compile time. It is the internal bridge for `?` propagation; since `?` is
   unavailable, this bridge has no call site.
3. **No `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`.** These names have 0 hits in the
   standard library; use `is_failure()` to check for failure, use `success()` to retrieve the
   payload. The corresponding tools on the `Result` side are in [`std.result`](result).

## Related

- [`std.result`](result) — `Result(T, E)`; `?` propagation is currently **only** available on it
- [`std.assert`](assert) — the source of `assert(false)` for dead-end branches
- [Syntax Specification §2.8](../language-spec/syntax.md) — variant destructuring and exhaustiveness
  checking
