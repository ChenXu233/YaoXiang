---
title: 'std.weak'
description: 'Arc / Weak weak references'
---

# std.weak

The weak reference module, used with `Arc` to break reference cycles.

```yaoxiang
use std.weak
```

> This module relies on atomic reference counting and is **not exported** on `wasm32` targets.

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

Creates a weak reference corresponding to an `Arc`.

- `arc` — a strong reference value; it is passed by value and is therefore **moved** on invocation.

Returns: a weak reference handle.

> **Known limitation**: The current implementation of `weak.new` is
> `RuntimeValue::Weak(Arc::downgrade(&Arc::new(arc.clone())))` (`src/std/weak.rs:33-39`)—it first
> clones the **value** of the argument `Arc` into a **newly created** `Arc`, and then calls
> `downgrade` on this temporary `Arc`. Therefore:
>
> 1. The weak reference does **not** point to the original `Arc`'s allocation, but to a brand-new
>    allocation with the same contents;
> 2. The temporary `Arc` is destroyed when the function returns, dropping the strong reference count
>    to zero → in practice, `weak.upgrade(weak.new(p))` always yields `Option.none()` and expires
>    immediately.
>
> In other words, “does not increment the strong reference count” holds, but “can be upgraded back
> to the original value” currently **does not**. When sharing across scopes, use
> [`ref`](../language-spec/concurrency.md) to express this; do not rely on `Weak` upgrades.

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

- `weak` — a weak reference handle

Returns: `Option.some(Arc)` while the allocation is still alive, or `Option.none()` once it has been
freed. It **does not report an error**—the `Option` indicates whether the “target” is still present.

> With the current implementation of `weak.new` (see the “Known limitation” above), a newly created
> weak reference **always** returns `none()`. The example below demonstrates destructuring; in
> practice, it takes the `none` branch.

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade: returns some(v) while the target is alive, or none() once it has been freed
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **Prerequisite for variant destructuring**: Destructuring `Option` variants requires the variants
> to be in scope—after importing them with `use std.option`, you can use `match some(v)` / `none()`
> (see the Language Specification, §2.8, match).

## Semantic Description

A weak reference **does not hold ownership**: the existence of a `Weak` does not prevent the target
from being freed. A typical use is to break reference cycles—a parent node holds an `Arc` pointing
to a child node, while the child node holds only a `Weak` pointing back to the parent, thus breaking
the cycle.

> The above is the **design intent**. With the current implementation (see the “Known limitation” in
> [`new`](#new)), `Weak` cannot yet truly serve this role in this version.

## Related

- [Language Specification: Type System](../language-spec/type-system.md) — Ownership semantics for
  `Arc` / `Weak`
