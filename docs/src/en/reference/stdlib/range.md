---
title: 'std.range'
description: 'Range iteration, predicates, and lazy adapters'
---

# std.range

Range (`Range`) iteration and adapters.

```yaoxiang
use std.range
```

## Range Literals

| Syntax    | Meaning                   |
| --------- | ------------------------- |
| `a..b`    | From `a` to `b`, step `1` |
| `a..b..s` | From `a` to `b`, step `s` |

The range **does not include the end value** (half-open on the right). The step may be negative,
indicating a descending range.

## Iterator Protocol

[`iter`](#iter) returns a `Result`—when the step is `0` it is the error path (`E6009`), so you must
`unwrap` it or handle it explicitly:

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

> **The two sides have opposite shapes**: `has_next` has **no `&`** in its signature
> (`src/std/range.rs:40`), consuming the iterator by value, so you need to call `iter` again each
> time; `next` **has `&`** in its signature (`src/std/range.rs:46`), borrowing. [`std.list`](list)
> is the opposite—both of its methods borrow (`&Iter(T)` / `&mut Iter(T)`), so the same iterator can
> be queried repeatedly. Copying the writing style from either side to the other will not match.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // has_next has no &: create a new iterator each time
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

For everyday traversal, just use `for ... in`:

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

## Function List

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

<!-- stdlib:table:range end -->## Iterator Protocol

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

Creates an iterator from a range.

- `r` — A range, such as `1..6` or `3..0..-1`

Returns: on success, `Result.ok(iterator)`; when the step is `0`, returns `Result.err` with `code`
`E6009`.

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

> **Consumes** the iterator by value (the signature has no `&`)—after one check the original
> iterator is invalidated, and you must call [`iter`](#iter) again next time.

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

Takes the current element and advances the internal cursor by one.

Returns: the current element; when iteration ends, returns `Void`.

> **Borrows** the iterator (the signature has `&`, `src/std/range.rs:46`), without consuming it.
> This is the opposite of [`has_next`](#has_next), and also the opposite of [`std.list`](list)'s
> "both methods borrow" style.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.next(it) == 1)
}
```

Descending ranges are also supported:

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

Determines whether `x` falls within the range.

- `r` — The range
- `x` — The value to check

Returns: `Result.ok(Bool)`. The end value is **an open boundary** (not included); when a step is
given, only elements aligned with the step match.

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

The abort hook for invalid steps, called by `for ... in` when consuming a range whose step is `0`.

**Always** throws `E6007`, with the message `Range step must be non-zero (for/in consumption)`.
Normal code does not need to call this directly.

```yaoxiang
use std.range

main: () -> Void = {
    // Direct use of iter gives Err; no need to go through this hook
    r = range.iter(1..3)
}
```

## Adapters

`map` and `filter` return **lazy** adapters—they do not compute immediately, and must be consumed by
[`collect`](#collect) / [`reduce`](#reduce) / [`for_each`](#for_each) / `for ... in` to produce a
result.

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

Maps `f` over each element, returning a new lazy iterator.

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

Keeps elements for which `p` is true, returning a new lazy iterator.

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
    // Value semantics: `chained` will be consumed by indexed reads, binding each one to a local
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

Consumes the iterator, collecting all elements into a `List`.

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

- `it` — The iterator
- `init` — The initial accumulator value
- `f` — The reduction function `(accumulator, element) -> new accumulator`

> Note that the parameter order differs from [`std.list.reduce`](list#reduce): in this module it is
> `(iterator, initial, function)`, while in `std.list` it is `(list, function, initial)`.

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

Executes `f` for each element, used for side effects.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // Outputs 1, 2, 3
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> Closures currently **cannot capture and mutate** outer `mut` variables, so using `for_each` to
> accumulate will not work (it reports `E1001`)—for accumulation, use [`reduce`](#reduce).

## Related

- [`std.list`](list) — Lists and their iterators
- [`std.result`](result) — Unwrapping the return values of `iter` / `contains`
- [Error Code Reference](../error-code/) — `E6009` invalid step
