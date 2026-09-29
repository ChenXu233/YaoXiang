---
title: 'std.weak'
description: 'Arc / Weak reference'
---

# std.weak

The weak reference module, used together with `Arc` to break reference cycles.

```yaoxiang
use std.weak
```

> This module depends on atomic reference counting and is **not exported** on the `wasm32` target.

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

- `arc` —— strong reference value; passed by value, **moved** after the call

Returns: a `Weak` handle pointing to the same allocation block, **without incrementing** the strong
reference count.

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

- `weak` —— weak reference handle

Returns: `Option.some(Arc)` when the allocation block is still alive, `Option.none()` when it has
been released. **Does not error** — uses `Option` to express whether the target still exists.

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade: returns some(v) if target is alive, none() if released
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **Prerequisite for variant destructuring**: Destructuring `Option` variants requires the variant
> set to be in scope — after `use std.option`, you can `match some(v)` / `none()` (see language spec
> §2.8 match).

## Semantic Notes

Weak references **do not hold** ownership: the existence of a `Weak` does not prevent the target
from being released. A typical use is to break cyclic references — a parent node holds an `Arc`
pointing to a child node, and the child node only holds a `Weak` pointing back to the parent,
breaking the cycle.

## Related

- [Language Spec: Type System](../language-spec/type-system.md) —— ownership semantics of `Arc` /
  `Weak`
