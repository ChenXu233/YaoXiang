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

Create a weak reference from an `Arc`.

- `arc` — strong reference value; passed by value and **moved** after the call.

Returns: a `Weak` handle pointing to the same allocation, which does **not** increase the strong
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

Attempt to promote a weak reference to a strong reference.

- `weak` — weak reference handle.

Returns: `Option.some(Arc)` if the allocation is still alive, `Option.none()` if it has been
released. **No error**—`Option` is used to express "whether the target still exists".

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // target is alive: get the some variant
    u = weak.upgrade(w)
    assert(true)
}
```

> **Syntax limitation**: The variant destructuring syntax for `Option` (e.g. `match some(v)`) is not
> yet implemented, so currently only the successful call can be verified, and `some` / `none` cannot
> be branched in source code. See the notes in `src/std/tests/weak_ops.yx`.

## Semantics

A weak reference **does not hold** ownership: the existence of a `Weak` does not prevent the target
from being released. A typical use is to break cyclic references—a parent node holds an `Arc`
pointing to a child node, while the child node only holds a `Weak` pointing back to the parent, thus
breaking the cycle.

## Related

- [Language Specification: Type System](../language-spec/type-system.md) — ownership semantics of
  `Arc` / `Weak`
