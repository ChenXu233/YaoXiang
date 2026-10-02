---
title: 'Ownership Model'
---

# Ownership Model

YaoXiang doesn't use garbage collection (GC), and it doesn't use lifetime annotations either. Its
memory safety is built on **five concepts and one gradient**.

## Five Concepts, One Gradient

```
看一眼/原地改     拿走           共享持有         复制一份        系统级
    │              │              │              │              │
   &T            Move           ref          concat()      unsafe
  &mut T         零拷贝        编译器自动      显式深拷贝      *T
  零大小令牌       默认          选Rc/Arc                   用户负责
```

## Move: Default Ownership Transfer

In YaoXiang, **assignment = ownership transfer**. This is the default behavior, zero-copy:

```yaoxiang
Point: Type = { x: Float, y: Float }

main: () -> Void = {
    p = Point(x=1.0, y=2.0)
    p2 = p              // Move! p 的所有权转给 p2
                        // 此后不能再读 p

    // 想要修改 p2？用 mut 重新绑定
    mut p3 = Point(x=3.0, y=4.0)
    p3 = Point(x= p3.x + 1.0, y= p3.y + 1.0)
    print(p2)
    print(p3)
}
```

Function parameters and returns are also Move:

```yaoxiang
Point: Type = { x: Float, y: Float }

// 参数：Move 传入
process: (p: Point) -> Point = p

main: () -> Void = {
    p = Point(x=1.0, y=2.0)
    result = process(p)    // p 被 Move 走了
    print(result)
}
```

## &T / &mut T: Borrow Tokens

If you don't want to take ownership, just temporarily "take a look" (`&T`) or "modify in place"
(`&mut T`), the compiler automatically generates **zero-sized borrow tokens**:

```yaoxiang
use std.list

main: () -> Void = {
    data = [1, 2, 3, 4, 5]

    // 编译器自动传 &List(Int) 令牌——不拿走所有权
    print(list.len(data))    // 5
    print(data)              // ✅ data 还在，只是看了一眼
}
```

`&T` and `&mut T` are **zero-sized types** — they exist at compile-time and disappear at runtime.
You don't need to write `&` manually; the compiler decides automatically based on the usage context:

```yaoxiang
Point: Type = { x: Float, y: Float }

// 只读访问 → 自动 &T
// 注意：不要把函数命名为 print，那会遮蔽内建打印函数
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
    show(p)             // 传入 &Point
    shift(p, 1.0, 1.0)  // 传入 &mut Point
    show(p)
}
```

**Key difference**: `&T` is copyable (shared, read-only), `&mut T` is not copyable (exclusive,
mutable). This isn't a special rule — it's just two type properties.

## ref: Sharing Across Scopes

When you need to **hold** a value in multiple places simultaneously, use `ref`:

```yaoxiang
main: () -> Void = {
    data = [1, 2, 3, 4, 5]

    // ref 创建共享持有
    shared = ref data

    // 跨任务共享：spawn 块的接收式绑定 `(绑定) = spawn { 块 }` 拿到共享值，
    // 编译器自动选择引用计数器：
    // - 不跨任务 → Rc（单线程引用计数）
    // - 跨任务 → Arc（原子引用计数）
    (shared) = spawn {
        print(shared)
    }

    // 你不需要知道 Rc 和 Arc 的区别——编译器自动帮你选
}
```

## clone(): Explicit Deep Copy

When you need an independent copy, explicitly call `clone()`:

```yaoxiang
use std.list

main: () -> Void = {
    original = [1, 2, 3]
    // 注意：列表既无 clone() 也无 copy()（实测均 E1042）。
    // 要一份独立副本，用 concat 造新列表：
    backup = list.concat(original, [])   // 与 original 互不影响

    // 各自独立：改动 original 不会波及 backup
    print(original)   // [1, 2, 3]
    print(backup)     // [1, 2, 3]
}
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
