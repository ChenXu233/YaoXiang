---
title: 'std.weak'
description: 'Arc / Weak weak references'
---

# std.weak

The weak reference module, used together with `Arc` to break reference cycles.

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

Creates the corresponding weak reference from an `Arc`.

- `arc` — strong reference value; passed by value and therefore **moved** on call

Returns: a weak reference handle.

> **Known gap**: The current implementation of `weak.new` is
> `RuntimeValue::Weak(Arc::downgrade(&Arc::new(arc.clone())))` (`src/std/weak.rs:33-39`)—it first
> clones the **value** of the `Arc` argument into a **newly created** `Arc`, then calls `downgrade`
> on this temporary `Arc`. Therefore:
>
> 1. The weak reference does **not** point to the original `Arc`'s allocation, but to a brand-new
>    allocation with identical contents;
> 2. The temporary `Arc` is destroyed when the function returns, bringing the strong reference count
>    to zero → empirically, `weak.upgrade(weak.new(p))` always evaluates to `Option.none()` and
>    expires immediately.
>
> In other words, “does not increment the strong reference count” holds, but “can be upgraded back
> to the original value” currently **does not hold**. When sharing across scopes, use
> [`ref`](../language-spec/concurrency.md); do not rely on upgrading `Weak`.

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    // ref creates an Arc[Int]
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

Returns: `Option.some(Arc)` if the allocation is still alive, or `Option.none()` if it has been
released. **No error is raised**—use `Option` to indicate “whether the target still exists.”

> With the current implementation of `weak.new` (see “Known Gap” above), a newly created weak
> reference **always** returns `none()`. The following example demonstrates destructuring syntax,
> but actually takes the `none` branch.

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade: returns some(v) while the target is alive, and none() after it is released
    u = weak.upgrade(w)
    match u {
        some(v) -> println("alive"),
        none() -> println("dropped"),
    }
}
```

> **Variant destructuring prerequisite**: Variant destructuring for `Option` requires the variant
> set to be in scope—after importing with `use std.option`, you can `match some(v)` / `none()` (see
> Language Specification §2.8, match).

## Semantic Notes

A weak reference does **not** hold ownership: the existence of a `Weak` does not prevent the target
from being released. A typical use is to break reference cycles—a parent node holds an `Arc`
pointing to a child node, while the child node holds only a `Weak` pointing back to the parent,
breaking the cycle.

> The above is the **design intent**. With the current implementation (see “Known Gap” in
> [`new`](#new)), `Weak` cannot yet truly fulfill this role in this version.

## Related

- [Language Specification: Type System](../language-spec/type-system.md) — Ownership semantics of
  `Arc` / `Weak`
