---
title: 'std.option'
description: 'Option optional value and sum type'
---

# std.option

`Option(T)`—the optional value type for "having a value (`some`) or having no value (`none`)". It is
the **record-style sum type** exported by this module (RFC-010), sharing the same mechanism as
user-defined sum types: variant construction, match variant deconstruction, exhaustiveness checking.
`use std.option` before use.

```yaoxiang
use std.option
```

> **This page is handwritten.** This module is implemented purely in YaoXiang source code
> (`src/std/option.yx`), and is not within the generation range of `StdModule::exports()`, so there
> is no `<!-- stdlib:... -->` generation marker. The signatures in the table below are taken
> verbatim from the export declarations in that file.

## Exports

| Export   | Signature           | Description                                 |
| -------- | ------------------- | ------------------------------------------- |
| `Option` | `(T: Type) -> Type` | Sum type, with two variants `some` / `none` |

The module **only exports the single binding `Option`**. The four methods of `Try` are declared
inside the type body (see below) and **are not module-level exports**—`option.is_failure(...)`
reports `E1042 field 'is_failure' not found in struct 'option'`. They are accessed with the **method
call** syntax `o.is_failure()`.

## Type Body

```
pub Option: (T: Type) -> Type = {
    some: (T) -> Option(T),
    none: () -> Option(T),
    Try(Option(T), T, Void),
}
```

- `some` — the value variant carrying a `T`
- `none` — the no-value variant carrying no payload
- `Try(Option(T), T, Void)` — on failure the residual type is `Void` (corresponding to the `NoneT`
  semantics of Rust's `Try` experiment)

Variant construction at expression position **requires type qualification**: `Option(Int).some(5)`,
`Option(Int).none()`; a bare name `some(5)` is not a constructor call (see
[Syntax Specification §1.4.2](../language-spec/syntax.md)).

## Functions

The `Try` methods defined inside the type body (`src/std/option.yx:23-45`):

| Method                | Signature                    | on `some`                     | on `none`                     |
| --------------------- | ---------------------------- | ----------------------------- | ----------------------------- |
| `is_failure()`        | `(self: &Option(T)) -> Bool` | `false`                       | `true`                        |
| `success()`           | `(self: &Option(T)) -> T`    | returns payload               | `E6005` (dead end)            |
| `residual()`          | `(self: &Option(T)) -> Void` | `E6005` (dead end)            | returns `void`                |
| `from_error(v: Void)` | `(v: Void) -> Option(T)`     | see [Known Gaps](#known-gaps) | see [Known Gaps](#known-gaps) |

The "dead end" in the latter three is the `assert(false)` written explicitly inside the type body
(`src/std/option.yx:33,40`); `Never <: T` makes these branches legal at the type level and
terminates with `E6005` at runtime.

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

## match Variant Deconstruction

Deconstructing `Option` requires the variant set to be in scope, i.e. `use std.option` first (see
[Syntax Specification §2.8](../language-spec/syntax.md)). The exhaustiveness check goes over the
full variant set—both `some` and `none` arms must be written; a catch-all arm `_` may be exempt.

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

The following three "declared but currently not runnable" items have each been confirmed through
actual testing with `yaoxiang-rs run`:

1. **`?` propagation does not apply to `Option`.** The type body instantiates
   `Try(Option(T), T, Void)`, but the type checker only recognizes `Result`: `o?` reports
   `E1081 ? is only allowed in functions returning a type that implements Try`. `?` on `Result`
   works normally and can serve as a reference.
2. **`from_error` cannot be called.** Written as a method `o.from_error()` it compiles but at
   runtime reports `E6006 Function not found: from_error`; written as `Option(T).from_error(...)` it
   reports `E1042` at compile time. It is the internal bridge for `?` propagation, and since `?` is
   unavailable, this bridge has no call sites.
3. **No `is_some` / `is_none` / `unwrap` / `unwrap_or` / `map`.** These names have zero hits in the
   standard library; to check failure use `is_failure()`, to take the payload use `success()`. The
   corresponding utilities on the `Result` side are in [`std.result`](./result).

## See Also

- [`std.result`](./result) — `Result(T, E)`; `?` propagation is currently available **only** on it
- [`std.assert`](./assert) — source of `assert(false)` in dead-end branches
- [Syntax Specification §2.8](../language-spec/syntax.md) — variant deconstruction and
  exhaustiveness checking
