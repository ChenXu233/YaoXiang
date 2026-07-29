---
title: Ownership Model
---

# Ownership Model

YaoXiang does not use garbage collection (GC), nor does it use lifetime annotations. Its memory
safety is built on **five concepts, one gradient**.

## Five Concepts, One Gradient

```
Glance/Modify in place    Take away          Shared ownership        Copy a piece       System-level
     │                      │                    │                     │                  │
    &T                    Move               ref                   clone()           unsafe
   &mut T                 Zero-copy         Compiler auto-         Explicit           *T
   Zero-size token         Default           selects Rc/Arc        deep copy         User responsible
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
p = Point(1.0, 2.0)
p2 = p              // Move! Ownership of p transfers to p2
                    // After this, p cannot be read anymore

// Want to modify p2? Use mut
mut p3 = Point(3.0, 4.0)
shift(p3, 1.0, 1.0)    // Modify in place
```

Function parameters and returns are also Move:

```yaoxiang
// Parameter: Move in
process: (p: Point) -> Point = {
    p.transform()
    p                  // Move return—zero-copy
}

// Call
p = Point(1.0, 2.0)
result = process(p)    // p is moved away
```

## &T / &mut T: Borrow Tokens

If you don't want to take ownership, just temporarily "glance" (`&T`) or "modify in place"
(`&mut T`), the compiler automatically generates **zero-size borrow tokens**:

```yaoxiang
data = [1, 2, 3, 4, 5]

// Compiler automatically passes &List(Int) token—doesn't take ownership
print(data.len())    // 5
print(data)          // ✅ data is still there, just glanced at
```

`&T` and `&mut T` are **zero-size types**—they exist at compile time and disappear at runtime. You
don't need to manually write `&`; the compiler decides automatically based on usage:

```yaoxiang
// Read-only access → automatically &T
print: (point: &Point) -> Void = {
    print("({point.x}, {point.y})")
}

// Mutable modification → automatically &mut T
shift: (point: &mut Point, dx: Float, dy: Float) -> Void = {
    point.x = point.x + dx
    point.y = point.y + dy
}

mut p = Point(1.0, 2.0)
print(p)                // Pass &Point
shift(p, 1.0, 1.0)      // Pass &mut Point
```

**Key distinction**: `&T` is copyable (shared read-only), `&mut T` is not copyable (exclusive
mutable). This is not a special rule—it is two type properties.

## ref: Cross-scope Sharing

When you need to **simultaneously hold** a value in multiple places, use `ref`:

```yaoxiang
data = [1, 2, 3, 4, 5]

// ref creates shared ownership
shared = ref data

// Compiler automatically selects reference counter:
// - No cross-task usage → Rc (single-thread reference counting)
// - Cross-task usage → Arc (atomic reference counting)
spawn {
    use(shared)    // Cross-task! Compiler automatically uses Arc
}

// You don't need to know the difference between Rc and Arc
// Compiler picks for you automatically
```

## clone(): Explicit Deep Copy

When you need an independent copy, explicitly call `clone()`:

```yaoxiang
original = [1, 2, 3]
backup = original.clone()   // Deep copy—has independent copy

// Independent from each other
original[0] = 10
print(backup[0])    // 1—unaffected
```

`clone()` is explicit—you clearly intend to copy, unlike some languages that copy by default.

## No Lifetimes

YaoXiang has no lifetimes `'a`. This design choice comes from a key observation:

> Borrow conflict problems are essentially equivalent to Hoare logic verification. Letting the type
> checker's proof pipeline solve them unified eliminates the need for an additional borrow checker
> framework.

You don't need to annotate `'a`, don't need to understand lifetimes—the compiler automatically
verifies ownership safety during type checking.

## No GC

The entire ownership model has no garbage collection. All memory deallocation timing is determined
at compile time:

- **After Move** → Original variable unavailable, RAII auto-releases
- **After ref** → Released when reference count reaches zero
- **Scope ends** → Stack variables auto-release

Zero GC pauses, zero runtime overhead.

## Summary

| Operation        | Keyword/Syntax     | Copy?           | When to use               |
| ---------------- | ------------------ | --------------- | ------------------------- |
| Take ownership   | Default behavior   | Zero-copy       | Function args, assignment |
| Glance           | Automatic `&T`     | Zero-size token | Read-only access          |
| Modify in place  | Automatic `&mut T` | Zero-size token | Mutable modification      |
| Shared ownership | `ref`              | Reference count | Cross-scope/cross-task    |
| Explicit copy    | `.clone()`         | Deep copy       | Need independent copy     |
| Raw pointer      | `unsafe` + `*T`    | Manual          | System-level operations   |

**Remember**: Move is default, ref is sharing, clone is the exception. Three rules, goodbye GC.
