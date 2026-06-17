---
title: Ownership Model
---

# Ownership Model

YaoXiang does not use garbage collection (GC), nor does it use lifetime annotations. Its memory safety is built upon **five concepts, one gradient**.

## Five Concepts, One Gradient

```
Look/In-place modify   Take away    Shared hold      Copy one      System-level
        │                │              │              │              │
       &T              Move           ref          clone()        unsafe
      &mut T          zero-copy      compiler-auto  explicit deep   *T
    zero-size token   default      picks Rc/Arc    copy           user responsible
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
p = Point(1.0, 2.0)
p2 = p              # Move! p's ownership transfers to p2
                    # p can no longer be read after this

# Want to modify p2? Use mut
mut p3 = Point(3.0, 4.0)
shift(p3, 1.0, 1.0)    # In-place modification
```

Function arguments and returns are also Move:

```yaoxiang
# Parameter: Move in
process: (p: Point) -> Point = {
    p.transform()
    p                  # Move return — zero-copy
}

# Invocation
p = Point(1.0, 2.0)
result = process(p)    # p is moved away
```

## &T / &mut T: Borrowing Tokens

If you don't want to take away ownership, just temporarily "take a look" (`&T`) or "modify in place" (`&mut T`), the compiler automatically generates a **zero-size borrowing token**:

```yaoxiang
data = [1, 2, 3, 4, 5]

# Compiler automatically passes a &List(Int) token — does not take ownership
println(data.len())    # 5
println(data)          # ✅ data is still here, just took a look
```

`&T` and `&mut T` are **zero-size types** — they exist at compile time and disappear at runtime. You don't need to write `&` manually; the compiler decides automatically based on usage context:

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
print(p)                # Pass in &Point
shift(p, 1.0, 1.0)      # Pass in &mut Point
```

**Key distinction**: `&T` is copyable (shared read-only), `&mut T` is not copyable (exclusive mutable). This is not a special rule — it follows from two type properties.

## ref: Cross-Scope Sharing

When you need to **hold a value simultaneously** in multiple places, use `ref`:

```yaoxiang
data = [1, 2, 3, 4, 5]

# ref creates shared ownership
shared = ref data

# Compiler automatically selects the reference counter:
# - Not crossing tasks → Rc (single-threaded reference counting)
# - Crossing tasks → Arc (atomic reference counting)
spawn {
    use(shared)    # Crossing tasks! Compiler automatically uses Arc
}

# You don't need to know the difference between Rc and Arc
# The compiler picks for you
```

## clone(): Explicit Deep Copy

When you need an independent copy, explicitly call `clone()`:

```yaoxiang
original = [1, 2, 3]
backup = original.clone()   # Deep copy — owns an independent copy

# Each is independent
original[0] = 10
println(backup[0])    # 1 — unaffected
```

`clone()` is explicit — you clearly ask for a copy, unlike some languages that copy by default.

## No Lifetimes

YaoXiang has no lifetimes `'a`. This design choice comes from a key observation:

> The borrow conflict problem is essentially equivalent to Hoare proposition verification. Handing it to the type checker's proof pipeline for a unified solution eliminates the need for an additional borrow checking framework.

You don't need to annotate `'a`, you don't need to understand lifetimes — the compiler automatically verifies ownership safety during the type checking phase.

## No GC

The entire ownership model has no garbage collection. The release timing of all memory is determined at compile time:

- **After Move** → the original variable is unusable, RAII automatically releases
- **After ref** → released when reference count drops to zero
- **Scope ends** → stack variables are automatically released

Zero GC pauses, zero runtime overhead.

## Summary

| Operation | Keyword/Syntax | Copy? | When to use |
|------|------------|--------|--------|
| Take ownership | Default behavior | Zero-copy | Function parameters, assignment |
| Take a look | Auto `&T` | Zero-size token | Read-only access |
| Modify in place | Auto `&mut T` | Zero-size token | Mutable modification |
| Shared hold | `ref` | Reference counting | Cross-scope / cross-task |
| Explicit copy | `.clone()` | Deep copy | Need independent copy |
| Raw pointer | `unsafe` + `*T` | Manual | System-level operations |

**Remember**: Move is the default, ref is for sharing, clone is the exception. Three rules, goodbye GC forever.