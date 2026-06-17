---
title: Ownership Model
---

# Ownership Model

YaoXiang uses neither garbage collection nor lifetime annotations. Its memory safety is built on **five concepts on one gradient**.

## Five Concepts, One Gradient

```
Read/Mutate       Take           Share          Copy           System
    │               │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T        Zero-copy      Compiler       Explicit      *T
  Zero-size      Default       picks Rc/Arc   Deep copy     Your problem
  tokens
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default, with zero copying:

```yaoxiang
p = Point(1.0, 2.0)
p2 = p              # Move! p's ownership transferred to p2
                    # p can no longer be read

# Want to modify p2? Use mut
mut p3 = Point(3.0, 4.0)
shift(p3, 1.0, 1.0)    # Modify in place
```

Function parameters and returns are also Move:

```yaoxiang
# Parameter: Move in
process: (p: Point) -> Point = {
    p.transform()
    p                  # Move return — zero copy
}

# Call
p = Point(1.0, 2.0)
result = process(p)    # p was moved away
```

## &T / &mut T: Borrow Tokens

If you don't want to take ownership — just "take a look" (`&T`) or "modify in place" (`&mut T`) — the compiler auto-generates **zero-size borrow tokens**:

```yaoxiang
data = [1, 2, 3, 4, 5]

# Compiler auto-passes &List(Int) token — doesn't take ownership
println(data.len())    # 5
println(data)          # ✅ data is still here, you just looked
```

`&T` and `&mut T` are **zero-size types** — they exist at compile time and vanish at runtime. You don't manually write `&`; the compiler decides based on usage:

```yaoxiang
# Read-only access → auto &T
print: (point: &Point) -> Void = {
    println("(${point.x}, ${point.y})")
}

# Mutable modification → auto &mut T
shift: (point: &mut Point, dx: Float, dy: Float) -> Void = {
    point.x = point.x + dx
    point.y = point.y + dy
}

mut p = Point(1.0, 2.0)
print(p)                # Passes &Point
shift(p, 1.0, 1.0)      # Passes &mut Point
```

**Key difference**: `&T` is copyable (shared, read-only), `&mut T` is not copyable (exclusive, mutable). These aren't special rules — they're two type properties.

## ref: Cross-Scope Sharing

When you need to hold a value in **multiple places simultaneously**, use `ref`:

```yaoxiang
data = [1, 2, 3, 4, 5]

# ref creates shared ownership
shared = ref data

# Compiler auto-selects the reference counter:
# - Same task → Rc (single-threaded reference count)
# - Cross-task → Arc (atomic reference count)
spawn {
    use(shared)    # Cross-task! Compiler auto-uses Arc
}

# You don't need to know the difference between Rc and Arc
# The compiler picks for you
```

## clone(): Explicit Deep Copy

When you need an independent copy, explicitly call `clone()`:

```yaoxiang
original = [1, 2, 3]
backup = original.clone()   # Deep copy — independent copy

# Each is independent
original[0] = 10
println(backup[0])    # 1 — unaffected
```

`clone()` is explicit — you're asking for a copy, unlike languages that copy by default.

## No Lifetimes

YaoXiang has no lifetime `'a`. This design choice comes from a key insight:

> Borrow conflict problems are equivalent to Hoare triple verification. Hand them to the type checker's proof pipeline for unified resolution — no separate borrow checker needed.

You don't annotate `'a`, you don't need to understand lifetimes — the compiler verifies ownership safety during type checking.

## No GC

The entire ownership model has no garbage collection. All memory release timing is determined at compile time:

- **After Move** → original variable unusable, RAII auto-releases
- **After ref** → released when reference count reaches zero
- **End of scope** → stack variables auto-released

Zero GC pauses, zero runtime overhead.

## Summary

| Operation | Keyword/Syntax | Copy? | When to Use |
|-----------|---------------|-------|-------------|
| Take ownership | Default | Zero-copy | Function args, assignment |
| Take a look | Auto `&T` | Zero-size token | Read-only access |
| Modify in place | Auto `&mut T` | Zero-size token | Mutable modification |
| Share ownership | `ref` | Reference counting | Cross-scope / cross-task |
| Explicit copy | `.clone()` | Deep copy | Need independent copy |
| Raw pointer | `unsafe` + `*T` | Manual | System-level ops |

**Remember**: Move is default, ref is share, clone is exception. Three rules, goodbye GC forever.
