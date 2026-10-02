---
title: 'Ownership Model'
---

# Ownership Model

YaoXiang doesn't use garbage collection (GC), nor does it use lifetime annotations. Its memory
safety is built on **five concepts, one gradient**.

## Five Concepts, One Gradient

```
peek/modify-in-place    take-away      shared-holding    make-a-copy      system-level
       │                   │                │                │                │
      &T                 Move             ref            concat()         unsafe
     &mut T            zero-copy      compiler-auto  explicit deep copy    *T
  zero-size-token       default       choose Rc/Arc                user-responsible
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
Point: Type = { x: Float, y: Float }

main: () -> Void = {
    p = Point(x=1.0, y=2.0)
    p2 = p              // Move! p's ownership transfers to p2
                        // p cannot be read after this

    // Want to modify p2? Re-bind with mut
    mut p3 = Point(x=3.0, y=4.0)
    p3 = Point(x= p3.x + 1.0, y= p3.y + 1.0)
    print(p2)
    print(p3)
}
```

Function arguments and returns are also Move:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Parameter: Move in
process: (p: Point) -> Point = p

main: () -> Void = {
    p = Point(x=1.0, y=2.0)
    result = process(p)    // p is moved away
    print(result)
}
```

## &T / &mut T: Borrowing Tokens

If you don't want to take ownership, just temporarily "take a look" (`&T`) or "modify in place"
(`&mut T`), the compiler automatically generates **zero-size borrowing tokens**:

```yaoxiang
use std.list

main: () -> Void = {
    data = [1, 2, 3, 4, 5]

    // The compiler automatically passes an &List(Int) token — doesn't take ownership
    print(list.len(data))    // 5
    print(data)              // ✅ data is still here, just took a look
}
```

`&T` and `&mut T` are **zero-size types** — they exist at compile time, and disappear at runtime.
You don't need to manually write `&`; the compiler automatically decides based on usage context:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Read-only access → automatic &T
// Note: don't name a function print, that would shadow the built-in print function
show: (point: &Point) -> Void = {
    print("({point.x}, {point.y})")
}

// Mutable modification → automatic &mut T
shift: (point: &mut Point, dx: Float, dy: Float) -> Void = {
    point.x = point.x + dx
    point.y = point.y + dy
}

main: () -> Void = {
    mut p = Point(x=1.0, y=2.0)
    show(p)             // Pass &Point
    shift(p, 1.0, 1.0)  // Pass &mut Point
    show(p)
}
```

**Key difference**: `&T` is copyable (shared read-only), `&mut T` is not copyable (exclusive
mutable). This isn't a special rule — these are two type properties.

## ref: Cross-Scope Sharing

When you need to **hold** a value in multiple places simultaneously, use `ref`:

```yaoxiang
main: () -> Void = {
    data = [1, 2, 3, 4, 5]

    // ref creates shared holding
    shared = ref data

    // Cross-task sharing: the spawn block's receiving binding `(binding) = spawn { block }` obtains the shared value,
    // the compiler automatically chooses the reference counter:
    // - Not crossing tasks → Rc (single-threaded reference counting)
    // - Crossing tasks → Arc (atomic reference counting)
    (shared) = spawn {
        print(shared)
    }

    // You don't need to know the difference between Rc and Arc — the compiler chooses for you automatically
}
```

## Copy: Explicitly Make a New Value

When you need an independent copy, explicitly make a new value (YaoXiang has no implicit copy):

```yaoxiang
use std.list

main: () -> Void = {
    original = [1, 2, 3]
    // Note: lists have neither clone() nor copy() (both tested E1042).
    // To get an independent copy, use concat to create a new list:
    backup = list.concat(original, [])   // Independent of original

    // Each independent: modifying original won't affect backup
    print(original)   // [1, 2, 3]
    print(backup)     // [1, 2, 3]
}
```

Explicit copying is explicit — you explicitly ask to copy, unlike some languages that copy by
default. ⚠️ 0.8.2's `std.list` doesn't yet provide `clone()` / `copy()` (`list.clone` reports
`E1042`); currently you can only use `list.concat(xs, [])` to make a copy.

## No Lifetimes

YaoXiang has no lifetime `'a`. This design choice comes from a key observation:

> The borrow conflict problem is essentially equivalent to Hoare proposition verification. Hand it
> over to the type checker's proof pipeline to solve uniformly — no need for an additional borrow
> checking framework.

You don't need to annotate `'a`, you don't need to understand lifetimes — the compiler automatically
verifies ownership safety during the type checking phase.

## No GC

The entire ownership model has no garbage collection. The release timing of all memory is determined
at compile time:

- **After Move** → Original variable unavailable, RAII auto-release
- **After ref** → Released when reference count reaches zero
- **Scope ends** → Stack variables auto-release

Zero GC pauses, zero runtime overhead.

## Summary

| Operation       | Keyword/Syntax           | Copy?              | When to use               |
| --------------- | ------------------------ | ------------------ | ------------------------- |
| Take ownership  | Default behavior         | Zero-copy          | Function args, assignment |
| Take a look     | Automatic `&T`           | Zero-size token    | Read-only access          |
| Modify in place | Automatic `&mut T`       | Zero-size token    | Mutable modification      |
| Shared holding  | `ref`                    | Reference counting | Cross-scope / cross-task  |
| Explicit copy   | No `clone()` (0.8.2 gap) | New value          | Need independent copy     |
| Raw pointer     | `unsafe` + `*T`          | Manual             | System-level operations   |

**Remember**: Move is the default, ref is sharing, copy is explicit. Three rules, goodbye GC
forever.
