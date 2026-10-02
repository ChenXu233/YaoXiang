---
title: 'std.weak'
description: 'Arc / Weak weak reference'
---

# std.weak

A weak reference module, used together with `Arc` to break reference cycles.

```yaoxiang
use std.weak
```

> This module depends on atomic reference counting and is **not exported** for the `wasm32` target.

## Function Overview

<!-- stdlib:table:weak start -->

| Function  | Signature                                    |
| --------- | -------------------------------------------- |
| `new`     | `(T: Type)(arc: Arc(T)) -> Weak(T)`          |
| `upgrade` | `(T: Type)(weak: Weak(T)) -> Option(Arc(T))` |

<!-- stdlib:table:weak end -->

## Functions

### new

<!-- stdlib:sig:weak.new start -->

```yaoxiang
new: (T: Type)(arc: Arc(T)) -> Weak(T)
```

<!-- stdlib:sig:weak.new end -->

Creates the corresponding weak reference from `Arc`.

- `arc` — strong reference value; passed by value, **moved** after the call

Returns: a weak reference handle.

> **Known gap**: The current implementation of `weak.new` is
> `RuntimeValue::Weak(Arc::downgrade(&Arc::new(arc.clone())))` (`src/std/weak.rs:33-39`) — it first
> clones the **value** of the argument `Arc` into a **newly created** `Arc`, then takes `downgrade`
> on this temporary `Arc`. Therefore:
>
> 1. The weak reference does **not point** to the original `Arc`'s allocation, but to a brand-new
>    allocation with the same content;
> 2. The temporary `Arc` is destructed when the function returns, the strong reference count goes to
>    zero → empirically `weak.upgrade(weak.new(p))` is always `Option.none()`, immediately invalid.
>
> In other words, "does not increase the strong reference count" holds true, but "can be upgraded
> back to the original value" is currently **not** valid. For cross-scope sharing use
> [`ref`](../language-spec/concurrency.md), do not rely on `Weak`'s upgrade-back.

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    // ref creates Arc[Int]
    p = ref 42

    // Arc → Weak registration
    w = weak.new(p)
    assert(true)
}
```

### upgrade

<!-- stdlib:sig:weak.upgrade start -->

```yaoxiang
upgrade: (T: Type)(weak: Weak(T)) -> Option(Arc(T))
```

<!-- stdlib:sig:weak.upgrade end -->

Attempts to upgrade a weak reference to a strong reference.

- `weak` — weak reference handle

Returns: `Option.some(Arc)` when the allocation block is still alive, `Option.none()` when it has
been released. **Does not error** — use `Option` to express "whether the target is still present."

> Per the current implementation of `weak.new` (see "Known gap" in the previous section), a freshly
> created weak reference **always** returns `none()`. The example below demonstrates the
> destructuring syntax; it actually goes through the `none` branch.

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade: target alive returns some(v), released returns none()
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **Variant destructuring prerequisite**: Destructuring `Option` variants requires the variant set
> to be present — after `use std.option`, you can `match some(v)` / `none()` (see Language
> Specification §2.8 match).

## Semantics

A weak reference **does not hold** ownership: the presence of `Weak` does not prevent the target
from being released. A typical use is to break circular references — the parent node holds an `Arc`
pointing to child nodes, and child nodes only hold `Weak` pointing back to the parent; the cycle is
thus broken.

> The above is the **design intent**. Per the current implementation (see "Known gap" in
> [`new`](#new)), `Weak` cannot truly fulfill this role in this version yet.

## Related

- [Language Specification: Type System](../language-spec/type-system.md) — Ownership semantics of
  `Arc` / `Weak`
