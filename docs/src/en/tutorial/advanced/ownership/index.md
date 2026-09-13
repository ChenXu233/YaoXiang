---
title: 'Ownership Model'
---

# Ownership Model

YaoXiang doesn't use garbage collection (GC), and it doesn't use lifetime annotations either. Its
memory safety is built on **five concepts and one gradient**.

## Five Concepts, One Gradient

```
Take a look / Modify in place   Take away   Shared holding   Clone a copy   System-level
        │                          │              │              │              │
       &T                        Move           ref          clone()        unsafe
      &mut T                     zero-copy    compiler auto   explicit deep copy   *T
    zero-sized token             default      choose Rc/Arc               user responsible
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
p = Point(1.0, 2.0)
p2 = p              // Move! p's ownership is transferred to p2
                    // After this, p can no longer be read

// Want to modify p2? Use mut
mut p3 = Point(3.0, 4.0)
shift(p3, 1.0, 1.0)    // Modify in place
```

Function parameters and returns are also Move:

```yaoxiang
// Parameter: passed in by Move
process: (p: Point) -> Point = {
    p.transform()
    p                  // Returned by Move — zero-copy
}

// Call
p = Point(1.0, 2.0)
result = process(p)    // p is moved away
```

## &T / &mut T: Borrow Tokens

If you don't want to take ownership, just temporarily "take a look" (`&T`) or "modify in place"
(`&mut T`), the compiler automatically generates **zero-sized borrow tokens**:

```yaoxiang
data = [1, 2, 3, 4, 5]

// Compiler automatically passes a &List(Int) token — doesn't take ownership
print(data.len())    // 5
print(data)          // ✅ data is still here, we just took a look
```

`&T` and `&mut T` are **zero-sized types** — they exist at compile-time and disappear at runtime.
You don't need to write `&` manually; the compiler decides automatically based on the usage context:

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
print(p)                // Pass in &Point
shift(p, 1.0, 1.0)      // Pass in &mut Point
```

**Key difference**: `&T` is copyable (shared, read-only), `&mut T` is not copyable (exclusive,
mutable). This isn't a special rule — it's just two type properties.

## ref: Sharing Across Scopes

When you need to **hold** a value in multiple places simultaneously, use `ref`:

```yaoxiang
data = [1, 2, 3, 4, 5]

// ref creates shared holding
shared = ref data

// Compiler automatically chooses the reference counter:
// - Doesn't cross tasks → Rc (single-threaded reference counting)
// - Crosses tasks → Arc (atomic reference counting)
spawn {
    use(shared)    // Crosses tasks! Compiler automatically uses Arc
}

// You don't need to know the difference between Rc and Arc
// The compiler chooses for you automatically
```

## clone(): Explicit Deep Copy

When you need an independent copy, explicitly call `clone()`:

```yaoxiang
original = [1, 2, 3]
backup = original.clone()   // Deep copy — owns an independent copy

// Each is independent
original[0] = 10
print(backup[0])    // 1 — unaffected
```

`clone()` is explicit — you make it clear you want to copy, unlike some languages that copy by
default.

## No Lifetimes

YaoXiang has no lifetime `'a`. This design choice comes from a key observation:

> The borrow conflict problem is essentially equivalent to Hoare proposition verification. Handing
> it off to the type checker's proof pipeline for a unified solution eliminates the need for an
> additional borrow checking framework.

You don't need to annotate `'a`, you don't need to understand lifetimes — the compiler automatically
verifies ownership safety during the type checking phase.

## No GC

The entire ownership model has no garbage collection. The release timing of all memory is determined
at compile-time:

- **After Move** → the original variable becomes unusable, RAII automatically releases it
- **After ref** → released when the reference count drops to zero
- **End of scope** → stack variables are automatically released

Zero GC pauses, zero runtime overhead.

## Summary

| Operation       | Keyword/Syntax     | Copy?              | When to use             |
| --------------- | ------------------ | ------------------ | ----------------------- |
| Take ownership  | Default behavior   | Zero-copy          | Function args, assign   |
| Take a look     | Automatic `&T`     | Zero-sized token   | Read-only access        |
| Modify in place | Automatic `&mut T` | Zero-sized token   | Mutable modification    |
| Shared holding  | `ref`              | Reference counting | Cross-scope/cross-task  |
| Explicit copy   | `.clone()`         | Deep copy          | Need independent copy   |
| Raw pointer     | `unsafe` + `*T`    | Manual             | System-level operations |

**Remember**: Move is the default, ref is for sharing, clone is the exception. Three rules, goodbye
GC forever.
