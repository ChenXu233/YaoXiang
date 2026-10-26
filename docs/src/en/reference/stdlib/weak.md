---
title: 'std.weak'
description: 'Arc / Weak weak reference'
---

# std.weak

Weak reference module, used together with `Arc` to break reference cycles.

```yaoxiang
use std.weak
```

> This module depends on atomic reference counting and is **not exported** on the `wasm32` target.

## Function overview

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

Create a corresponding weak reference from `Arc`.

- `arc` — strong reference value; passed by value, **moved** after the call

Returns: a weak reference handle.

> **Known limitation**: the current implementation of `weak.new` is
> `RuntimeValue::Weak(Arc::downgrade(&Arc::new(arc.clone())))` (`src/std/weak.rs:33-39`) — it first
> clones the **value** of the argument `Arc` into a **newly created** `Arc`, then calls `downgrade`
> on this temporary `Arc`. Therefore:
>
> 1. The weak reference **does not point to** the original `Arc`'s allocation, but to a brand new
>    allocation with identical content;
> 2. The temporary `Arc` is destructed when the function returns, and the strong reference count
>    drops to zero → empirically, `weak.upgrade(weak.new(p))` is always `Option.none()`, immediately
>    invalid.
>
> In other words, "does not increase the strong reference count" holds, but "upgradable back to the
> original value" currently **does not hold**. When you need to share across scopes, use
> [`ref`](../language-spec/concurrency.md) to express it — do not rely on `Weak` to upgrade back.

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

Attempt to upgrade a weak reference to a strong reference.

- `weak` — weak reference handle

Returns: `Option.some(Arc)` when the allocation block is still alive, `Option.none()` when it has
been released. **Does not report an error** — `Option` is used to express whether the target is
still there.

> According to the current `weak.new` implementation (see the "known limitation" section above), a
> newly created weak reference **always** returns `none()`. The example below demonstrates the
> destructuring syntax, but actually goes through the `none` branch.

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

    // Variant set present: the same some/none can also be constructed directly
    probe = Option(Int).some(7)
}
```

> **Variant destructuring prerequisite**: `Option`'s variant destructuring requires the variant set
> to be present — after importing with `use std.option`, you can `match some(v)` / `none()` (see
> language spec §2.8 match).

## Semantics

A weak reference **does not hold** ownership: the existence of `Weak` does not prevent the target
from being released. A typical use is to break circular references — a parent node holds an `Arc`
pointing to a child node, and the child node only holds a `Weak` pointing back to the parent,
breaking the cycle.

> The above is the **design intent**. According to the current implementation (see the "known
> limitation" of [`new`](#new)), `Weak` cannot truly fulfill this role in this version.

## Related

- [Language Specification: Type System](../language-spec/type-system.md) — ownership semantics of
  `Arc` / `Weak`
