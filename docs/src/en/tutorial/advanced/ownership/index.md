---
title: Ownership Model
---

# Ownership Model

YaoXiang does not use garbage collection (GC), nor does it use lifetime annotations. Its memory safety is built on **five concepts, one gradient**.

## Five Concepts, One Gradient

```
peek/mutate-in-place   take away   shared holding   copy one   system-level
        │                │              │              │              │
       &T              Move           ref          clone()        unsafe
      &mut T          zero-copy      auto by         explicit     *T
    zero-sized token   default     compiler/Rc/Arc   deep copy   user responsible
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
p = Point(1.0, 2.0)
p2 = p              // Move! Ownership of p is transferred to p2
                    // After this, p can no longer be read

// Want to modify p2? Use mut
mut p3 = Point(3.0, 4.0)
shift(p3, 1.0, 1.0)    // Modify in place
```

Function parameter passing and returning are also Move:

```yaoxiang
// Parameter: Move in
process: (p: Point) -> Point = {
    p.transform()
    p                  // Move return — zero-copy
}

// Call
p = Point(1.0, 2.0)
result = process(p)    // p has been moved away
```

## &T / &mut T: Borrow Tokens

If you don't want to take away ownership, but only want to "peek" (`&T`) or "modify in place" (`&mut T`), the compiler will automatically generate **zero-sized borrow tokens**:

```yaoxiang
data = [1, 2, 3, 4, 5]

// Compiler automatically passes a &List(Int) token — does not take away ownership
print(data.len())    // 5
print(data)          // ✅ data is still here, just took a peek
```

`&T` and `&mut T` are **zero-sized types** — they exist at compile-time and vanish at runtime. You don't need to write `&` manually; the compiler decides automatically based on the usage context:

```yaoxiang
// Read-only access → automatic &T
print: (point: &Point) -> Void = {
    print("({point.x}, {point.y})")
}

// Mutable modification → automatic &mut T
shift: (point: &mut Point, dx: Float, dy: Float) -> Void = {
    point.x = point.x + dx
    point.y = point.y + dy
}

mut p = Point(1.0, 2.0)
print(p)                // Passes &Point
shift(p, 1.0, 1.0)      // Passes &mut Point
```

**Key difference**: `&T` is copyable (shared read-only), `&mut T` is not copyable (exclusive mutable). This isn't a special rule — it's two type attributes.

## ref: Cross-Scope Sharing

When you need to **hold a value simultaneously in multiple places**, use `ref`:

```yaoxiang
data = [1, 2, 3, 4, 5]

// ref creates shared holding
shared = ref data

// Compiler automatically selects the reference counter:
// - No cross-task → Rc (single-threaded reference counting)
// - Cross-task → Arc (atomic reference counting)
spawn {
    use(shared)    // Cross-task! Compiler automatically uses Arc
}

// You don't need to know the difference between Rc and Arc
// The compiler picks for you automatically
```

## clone(): Explicit Deep Copy

When you need an independent copy, call `clone()` explicitly:

```yaoxiang
original = [1, 2, 3]
backup = original.clone()   // Deep copy — has an independent copy

// Each is independent
original[0] = 10
print(backup[0])    // 1 — unaffected
```

`clone()` is explicit — you clearly want to copy, unlike some languages that copy by default.

## No Lifetimes

YaoXiang has no lifetimes `'a`. This design choice comes from a key observation:

> The borrow conflict problem is essentially equivalent to Hoare proposition verification. Hand it off to the type checker's proof pipeline to solve uniformly — no need for an additional borrow checking framework.

You don't need to annotate `'a`, you don't need to understand lifetimes — the compiler automatically verifies ownership safety during the type checking phase.

## No GC

The entire ownership model has no garbage collection. The release timing for all memory is determined at compile-time:

- **After Move** → the original variable is unusable, RAII releases automatically
- **After ref** → released when the reference count drops to zero
- **Scope ends** → stack variables are released automatically

Zero GC pauses, zero runtime overhead.

## Summary

| Operation | Keyword/Syntax | Copy? | When to Use |
|------|------------|--------|--------|
| Take ownership | Default behavior | Zero-copy | Function parameter, assignment |
| Peek | Automatic `&T` | Zero-sized token | Read-only access |
| Mutate in place | Automatic `&mut T` | Zero-sized token | Mutable modification |
| Shared holding | `ref` | Reference counted | Cross-scope / cross-task |
| Explicit copy | `.clone()` | Deep copy | Need an independent copy |
| Raw pointer | `unsafe` + `*T` | Manual | System-level operations |

**Remember**: Move is the default, ref is for sharing, clone is the exception. Three rules, and farewell to GC.