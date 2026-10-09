---
title: 'std.option'
description: 'Option optional value and sum type'
---

# std.option

`Option(T)` — the optional value type representing 'has a value (`some`)' or 'has no value
(`none`)'. It is the **record-style sum type** (RFC-010) exported by this module, sharing the same
mechanism as user-defined sum types: variant construction, match variant destructuring,
exhaustiveness checking. `use std.option` before use.

```yaoxiang
use std.option
```

> **This page is hand-written.** This module is implemented in pure YaoXiang source
> (`src/std/option.yx`), outside the generation range of `StdModule::exports()`, so there is no
> `<!-- stdlib:... -->` generation-region marker. The signatures in the table below are taken
> verbatim from that file's export declarations.

## Exports

| Export   | Signature           | Description                                 |
| -------- | ------------------- | ------------------------------------------- |
| `Option` | `(T: Type) -> Type` | Sum type, with two variants `some` / `none` |

The module **only exports the single binding `Option`**. The four `Try` methods are declared in the
type body (see below), **not as module-level exports** — `option.is_failure(...)` reports
`E1042 field 'is_failure' not found in struct 'option'`. They are accessed via the **method call**
syntax `o.is_failure()`.

## Type Body

```
Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

- `some` — the value-carrying variant with a `T`
- `none` — the no-value variant carrying no payload
- `Try(Option(T), T, Void)` — the residual type on failure is `Void` (corresponding to the `NoneT`
  semantics of the Rust `Try` experiment)

Variant construction in expression position **requires type qualification**: `Option(Int).some(5)`,
`Option(Int).none()`; the bare name `some(5)` is not a constructor call (see
[Syntax Specification §1.4.2](../language-spec/syntax.md)).

## Functions

The `Try` methods defined in the type body (`src/std/option.yx:23-45`):

| Method                | Signature                    | On `some`                   | On `none`                   |
| --------------------- | ---------------------------- | --------------------------- | --------------------------- |
| `is_failure()`        | `(self: &Option(T)) -> Bool` | `false`                     | `true`                      |
| `success()`           | `(self: &Option(T)) -> T`    | Returns payload             | `E6005` (dead end)          |
| `residual()`          | `(self: &Option(T)) -> Void` | `E6005` (dead end)          | Returns `void`              |
| `from_error(v: Void)` | `(v: Void) -> Option(T)`     | See [Known Gaps](#已知缺口) | See [Known Gaps](#已知缺口) |

The "dead ends" in the latter three are the explicit `assert(false)` written in the type body
(`src/std/option.yx:33,40`); `Never <: T` makes these branches valid at the type level and terminate
at runtime with `E6005`.

```yaoxiang
use std.option

main: () -> Void = {
    s = Option(Int).some(7)
    println(s.is_failure())   // false
    println(s.success())      // 7
    // s.residual() will diverge: the some arm is a dead end (E6005), not callable

    n = Option(Int).none()
    println(n.is_failure())   // true
    println(n.residual())     // void
    // n.success() likewise diverges: the none arm is a dead end (E6005)
}
```

## match Variant Destructuring

Destructuring `Option` requires the variant set to be in scope, i.e. first `use std.option` (see
[Syntax Specification §2.8](../language-spec/syntax.md)). Exhaustiveness checks the full variant set
— both `some` / `none` arms must be written; the catch-all arm `_` is exempt.

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

## Known Gaps

The following three items are "declared but currently cannot be run", each confirmed by
`yaoxiang-rs run` testing:

1. **`?` propagation does not apply to `Option`.** The type body instantiates
   `Try(Option(T), T, Void)`, but the type checker only recognizes `Result`: `o?` reports
   `E1081 ? is only allowed in functions returning a type that implements Try`. The `?` operator on
   `Result` works normally and can serve as a comparison.
2. **`from_error` cannot be called.** Written as a method `o.from_error()` it compiles but reports
   `E6006 Function not found: from_error` at runtime; written as `Option(T).from_error(...)` it is
   reported at compile time as `E1042`. It is the internal bridge of `?` propagation, and since `?`
   is unavailable, this bridge has no call site.
3. **No `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`.** These names have 0 hits in the
   standard library; check failure with `is_failure()`, retrieve the payload with `success()`. The
   corresponding tools on the `Result` side are in [`std.result`](result).

## Related

- [`std.result`](result) — `Result(T, E)`; `?` propagation is currently available **only** on it
- [`std.assert`](assert) — source of the `assert(false)` for dead-end branches
- [Syntax Specification §2.8](../language-spec/syntax.md) — variant destructuring and exhaustiveness
  checking
