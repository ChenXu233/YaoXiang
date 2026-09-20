---
title: 'std.range'
description: 'Range iteration, predicates and lazy adapters'
---

# std.range

Range (`Range`) iteration and adapters.

```yaoxiang
use std.range
```

## Range literals

| Syntax    | Meaning                   |
| --------- | ------------------------- |
| `a..b`    | From `a` to `b`, step `1` |
| `a..b..s` | From `a` to `b`, step `s` |

A range is **exclusive of the end value** (half-open, left-closed right-open). The step can be
negative to indicate a decreasing range.

## Iterator protocol

[`iter`](#iter) returns a `Result`—when the step is `0` it is the error path (`E6009`), so you must
`unwrap` or handle it explicitly:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

> **Move semantics**: The signatures of `has_next` and `next` do not take `&`, so they **move** the
> iterator. Therefore, you must create a new iterator on every access, or just iterate with
> `for ... in`. This matches the iterator in [`std.list`](./list).

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // create a new iterator each time
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

For everyday iteration, just use `for ... in` directly:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    mut sum = 0
    for x in nums {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## Function overview

<!-- stdlib:table:range start -->

| Function             | Signature                                                     |
| -------------------- | ------------------------------------------------------------- |
| `iter`               | `(r: Range(Int)) -> Result(Iterator(Any), Error)`             |
| `has_next`           | `(it: Iterator(Any)) -> Bool`                                 |
| `next`               | `(it: &Iterator(Any)) -> Any`                                 |
| `contains`           | `(r: Range(Int), x: Int) -> Result(Bool, Error)`              |
| `abort_invalid_step` | `(r: Range(Int)) -> Any`                                      |
| `map`                | `(it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)`       |
| `filter`             | `(it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)`      |
| `collect`            | `(it: Iterator(Any)) -> Vec(Any)`                             |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->

## Iterator protocol

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

Creates an iterator from a range.

- `r` — the range, e.g. `1..6` or `3..0..-1`

Returns: `Result.ok(iterator)` on success; `Result.err` with `code` `E6009` when the step is `0`.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### has_next

<!-- stdlib:sig:range.has_next start -->

```yaoxiang
has_next: (it: Iterator(Any)) -> Bool
```

<!-- stdlib:sig:range.has_next end -->

Whether there are still unconsumed elements.

> **Moves** the iterator.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### next

<!-- stdlib:sig:range.next start -->

```yaoxiang
next: (it: &Iterator(Any)) -> Any
```

<!-- stdlib:sig:range.next end -->

Takes the current element and advances the internal cursor by one position.

Returns: the current element; `Void` when iteration ends.

> **Moves** the iterator.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.next(it) == 1)
}
```

Decreasing ranges are supported as well:

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    desc = result.unwrap(range.iter(3..0..-1))
    assert(range.next(desc) == 3)
}
```

### contains

<!-- stdlib:sig:range.contains start -->

```yaoxiang
contains: (r: Range(Int), x: Int) -> Result(Bool, Error)
```

<!-- stdlib:sig:range.contains end -->

Checks whether `x` falls within the range.

- `r` — the range
- `x` — the value to test

Returns: `Result.ok(Bool)`. The end value is **excluded** (open); when a step is given, only
elements aligned with the step are matched.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // end value excluded

    assert(result.unwrap(range.contains(0..10..2, 4)))    // aligned with step
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // not aligned
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

The abort hook for an invalid step, invoked when `for ... in` consumes a range whose step is `0`.

**Always** raises `E6007` with the message `Range step must be non-zero (for/in consumption)`.
Normal code does not need to call this directly.

```yaoxiang
use std.range

main: () -> Void = {
    // using iter directly gives you an Err, so you do not need this hook
    r = range.iter(1..3)
}
```

## Adapters

`map` and `filter` return **lazy** adapters—they do not compute immediately, and only produce
results once consumed by [`collect`](#collect) / [`reduce`](#reduce) / [`for_each`](#for_each) /
`for ... in`.

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

Maps `f` over every element and returns a new lazy iterator.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.len(doubled) == 3)
    assert(doubled[0] == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

Keeps the elements for which `p` is true, and returns a new lazy iterator.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(list.len(big) == 2)
    assert(big[0] == 4)
}
```

Adapters can be chained:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    r = 1..6
    chained = range.collect(range.map(range.filter(result.unwrap(range.iter(r)), x => x % 2 == 0), x => x * 10))
    // value semantics: `chained` is consumed by indexed reads, so bind each value to a local
    first = chained[0]
    second = chained[1]
    assert(first == 20)
    assert(second == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> Vec(Any)
```

<!-- stdlib:sig:range.collect end -->

Consumes the iterator and collects all elements into a `List`.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    xs = range.collect(result.unwrap(range.iter(1..4)))
    assert(list.len(xs) == 3)
}
```

### reduce

<!-- stdlib:sig:range.reduce start -->

```yaoxiang
reduce: (it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any
```

<!-- stdlib:sig:range.reduce end -->

Consumes the iterator and folds it.

- `it` — the iterator
- `init` — the initial accumulator value
- `f` — the reduction function `(accumulator, element) -> new accumulator`

> Note that the argument order differs from [`std.list.reduce`](./list#reduce): in this module it is
> `(iterator, init, function)`, while `std.list` is `(list, function, init)`.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    total = range.reduce(result.unwrap(range.iter(1..6)), 0, (acc, x) => acc + x)
    assert(total == 15)
}
```

### for_each

<!-- stdlib:sig:range.for_each start -->

```yaoxiang
for_each: (it: Iterator(Any), f: (Any) -> Void) -> Void
```

<!-- stdlib:sig:range.for_each end -->

Runs `f` on every element, intended for side effects.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // prints 1, 2, 3
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> Closures currently **cannot capture and mutate** an outer `mut` variable, so accumulating with
> `for_each` is not possible (it raises `E1001`)—use [`reduce`](#reduce) for accumulation.

## Related

- [`std.list`](./list) — lists and their iterators
- [`std.result`](./result) — unwrapping the return value of `iter` / `contains`
- [Error code reference](../error-code/) — `E6009` invalid step
