---
title: 'std.weak'
description: 'Arc / Weak weak reference'
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

Create the corresponding weak reference from an `Arc`.

- `arc` — the strong reference value; passed by value and **moved** after the call.

Returns: a `Weak` handle pointing to the same allocation block, **without** incrementing the strong
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

Attempt to upgrade a weak reference to a strong reference.

- `weak` — the weak reference handle

Returns: `Option.some(Arc)` when the allocation block is still alive, `Option.none()` when it has
already been released. **No error is reported** — `Option` is used to express whether the target
still exists.

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // target alive: get some variant
    u = weak.upgrade(w)
    assert(true)
}
```

> **Syntax Limitation**: the variant destructuring syntax for `Option` (e.g. `match some(v)`) has
> not yet landed, so for now only the call success can be verified, and `some` / `none` cannot be
> branched on in source code. See the notes in `src/std/tests/weak_ops.yx`.

## Semantic Notes

Weak references **do not hold** ownership: the existence of a `Weak` does not prevent the target
from being released. The typical use case is breaking circular references — the parent node holds an
`Arc` pointing to the child node, while the child only holds a `Weak` pointing back to the parent;
this breaks the cycle.

## Related

- [Language Spec: type system](../language-spec/type-system.md) — ownership semantics of `Arc` /
  `Weak`
