---
title: 'std.list'
description: 'List add/remove, slicing, higher-order functions, and iterator protocol'
---

# std.list

List operations module. **Pay special attention to move semantics**: there are two categories of
functions — those that read-only borrow the source list, and those that consume (move) the source
list. The automatic borrow rules for `&` parameters are described in RFC-009 §2.8: when the argument
is still used after the call, the compiler automatically creates a read-only token.

```yaoxiang
use std.list
```

## Semantic Categories

Parameters with `&` in the signature are read-only borrows; the source value is still usable after
the call. Parameters without `&` are passed by value, and the source value is **moved** after the
call — using it again reports `E2014`.

| Category                | Function                                                                                                         | Behavior                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| **Consume source list** | `push` `append` `prepend` `set` `pop` `remove_at` `iter`                                                         | Source list is moved and can no longer be used                                               |
| Read-only borrow        | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | Source list can be reused; `item` in `contains`/`find_index` is also passed by borrow (`&A`) |
| Iterator protocol       | `has_next` (`&Iter(T)`) `next` (`&mut Iter(T)`)                                                                  | Both **borrow** the iterator and do not consume it                                           |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // Read-only borrow: nums can be reused
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // Consumed: base cannot be used after this
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## Function Overview

<!-- stdlib:table:list start -->

| Function     | Signature                                                                                  |
| ------------ | ------------------------------------------------------------------------------------------ |
| `push`       | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `pop`        | `(A: Type) -> (list: Vec(A)) -> Vec(A)`                                                    |
| `append`     | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `prepend`    | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `remove_at`  | `(A: Type) -> (list: Vec(A), index: Int) -> Vec(A)`                                        |
| `reverse`    | `(A: Type) -> (list: &Vec(A)) -> Vec(A)`                                                   |
| `concat`     | `(A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)`                                          |
| `map`        | `(T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)`                       |
| `filter`     | `(T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)`                          |
| `reduce`     | `(T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc` |
| `len`        | `(A: Type) -> (list: &Vec(A)) -> Int`                                                      |
| `is_empty`   | `(A: Type) -> (list: &Vec(A)) -> Bool`                                                     |
| `get`        | `(A: Type) -> (list: &Vec(A), index: Int) -> A`                                            |
| `set`        | `(A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)`                              |
| `first`      | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `last`       | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `slice`      | `(A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)`                             |
| `contains`   | `(A: Type) -> (list: &Vec(A), item: &A) -> Bool`                                           |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: &A) -> Int`                                            |
| `Iter`       | `(T: Type) -> Type`                                                                        |
| `iter`       | `(T: Type) -> (list: Vec(T)) -> Iter(T)`                                                   |
| `next`       | `(T: Type) -> (it: &mut Iter(T)) -> T`                                                     |
| `has_next`   | `(T: Type) -> (it: &Iter(T)) -> Bool`                                                      |
| `empty`      | `(T: Type) -> Vec(T)`                                                                      |
| `of`         | `(T: Type) -> (data: Vec(T)) -> Vec(T)`                                                    |

<!-- stdlib:table:list end -->## Functions

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.push end -->

Returns a **new list** with `item` appended to the end of `list`. `list` is passed by value and is
**moved** after the call — it can no longer be used.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

### append

<!-- stdlib:sig:list.append start -->

```yaoxiang
append: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.append end -->

Alias for `push`; behavior is identical.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    extended = list.append([1, 2], 3)
    assert(list.len(extended) == 3)
}
```

### prepend

<!-- stdlib:sig:list.prepend start -->

```yaoxiang
prepend: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.prepend end -->

Returns a new list with `item` inserted at the head. `list` is passed by value and is **moved**
after the call.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.prepend([2, 3], 1)
    assert(list.first(l) == 1)
}
```

### pop

<!-- stdlib:sig:list.pop start -->

```yaoxiang
pop: (A: Type) -> (list: Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.pop end -->

Removes the last element and returns the **shortened list** (value semantics). The source list is
consumed, and is not the native exception case of "signature carries `&` yet mutates the source in
place".

Returns: a new list with the last element removed; returns the list as-is when empty. To read the
removed element, take its value with `last` before calling.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l is consumed, rest is the shortened new list
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // The last element 3 has been removed

    // To read the removed element, take it with last first then pop
    l2 = [1, 2, 3]
    removed = list.last(l2)
    assert(removed == 3)

    empty = list.empty(Int)
    assert(list.is_empty(list.pop(empty)))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)
```

<!-- stdlib:sig:list.remove_at end -->

Removes the element at index `index` and returns the **shortened new list** (value semantics). The
source list is consumed.

- `index` — element index

Returns: a new list with the element removed. Error: throws `E6003` when the index is **negative**
(`list[i + 1]` out of bounds in `src/std/list.yx:156-158`).

> **Out-of-bounds does not error**: when index ≥ length, the copy loop never runs even once — only
> `length` is decremented by 1 and the result is silently truncated. `list.remove_at([1, 2, 3], 5)`
> returns a two-element list and **does not error**. Please check bounds yourself before calling.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [10, 20, 30]
    got = list.remove_at(l, 1)
    assert(list.len(got) == 2)
    assert(list.get(got, 0) == 10)
    assert(list.get(got, 1) == 30)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
```

<!-- stdlib:sig:list.set end -->

Returns a new list with index `index` overwritten by `value`. `list` is passed by value and is
**moved** after the call.

- `index` — index
- `value` — new value

Error: throws `E6003` when the index is negative or ≥ length (out-of-bounds writes are no longer
silently discarded).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.set([1, 2, 3], 1, 99)
    assert(list.get(l, 1) == 99)
}
```

### get

<!-- stdlib:sig:list.get start -->

```yaoxiang
get: (A: Type) -> (list: &Vec(A), index: Int) -> A
```

<!-- stdlib:sig:list.get end -->

Reads the element at index `index` (read-only borrow; `list` is reusable).

- `index` — index

Returns: the element value.

Error: throws `E6003` when the index is negative or ≥ length (`src/std/list.yx:78` uses
`list[index]` directly; out-of-bounds is caught by `[]`'s boundary check and **does not return
`Void`**).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.get(nums, 1) == 2)
}
```

### first

<!-- stdlib:sig:list.first start -->

```yaoxiang
first: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.first end -->

Returns the first element.

Error: **throws `E6003` on an empty list** — the fallback branch in `src/std/list.yx:82-89` reads
`zero[0]`, and index 0 of an empty `Vec` is out of bounds. Please check [`is_empty`](#is_empty)
first.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.first([1, 2, 3]) == 1)
}
```

### last

<!-- stdlib:sig:list.last start -->

```yaoxiang
last: (A: Type) -> (list: &Vec(A)) -> A
```

<!-- stdlib:sig:list.last end -->

Returns the last element.

Error: **throws `E6003` on an empty list** (same as [`first`](#first), `src/std/list.yx:92-99`).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.last([1, 2, 3]) == 3)
}
```

### slice

<!-- stdlib:sig:list.slice start -->

```yaoxiang
slice: (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
```

<!-- stdlib:sig:list.slice end -->

Takes the sub-list over the range `[start, end)`.

- `start` — start index
- `end` — end index (exclusive)

Returns: a new list. Returns an empty list when `start >= end`.

Error: **boundaries are not clamped** — `src/std/list.yx:196-205` reads `list[i]` one by one, and
out-of-bounds indices (including negative `start` / `end`) directly throw `E6003`.
`list.slice([1, 2, 3], 1, 99)` will error rather than return a two-element list.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    sub = list.slice([1, 2, 3, 4], 1, 3)
    assert(list.len(sub) == 2)
    assert(list.first(sub) == 2)
}
```

### reverse

<!-- stdlib:sig:list.reverse start -->

```yaoxiang
reverse: (A: Type) -> (list: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.reverse end -->

Returns a new list with elements in reversed order; the source list is unchanged.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    rev = list.reverse([1, 2, 3])
    assert(list.first(rev) == 3)
}
```

### concat

<!-- stdlib:sig:list.concat start -->

```yaoxiang
concat: (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.concat end -->

Concatenates two lists and returns a new list. Neither source list is changed.

Type checking is done at compile-time: `concat` is statically generic (`src/std/list.yx:179`), a
type error is reported when the second argument is not a list, and **there is no runtime `E6007`
path**.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    joined = list.concat([1, 2], [3, 4])
    assert(list.len(joined) == 4)
}
```

### len

<!-- stdlib:sig:list.len start -->

```yaoxiang
len: (A: Type) -> (list: &Vec(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

Number of elements. Read-only borrow; `list` can be reused repeatedly.

`len` is statically generic (`src/std/list.yx:67`); a non-list argument is a compile-time type
error, not a runtime exception.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)      // Reusable
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type) -> (list: &Vec(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

Whether the list is empty.

Same as `len`, statically generic (`src/std/list.yx:72`); a non-list argument is a compile-time type
error.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.is_empty([]))
    assert(!list.is_empty([1]))
}
```

### contains

<!-- stdlib:sig:list.contains start -->

```yaoxiang
contains: (A: Type) -> (list: &Vec(A), item: &A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

Whether `item` is in the list (compared by value equality; the element type must support `==` —
primitive types support it natively, while record types are provided by RFC-011b's `Equal` automatic
derivation or explicit instantiation). `item` is passed by read-only borrow (the call site
automatically creates an `&A` token), and the argument remains usable after the call.

Returns: `true` if present; `false` if not (linear scan, no error is thrown).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.contains(nums, 3))
    assert(!list.contains(nums, 99))
}
```

### find_index

<!-- stdlib:sig:list.find_index start -->

```yaoxiang
find_index: (A: Type) -> (list: &Vec(A), item: &A) -> Int
```

<!-- stdlib:sig:list.find_index end -->

The index of the first occurrence of `item`. `item` is passed by read-only borrow (the call site
automatically creates an `&A` token), and the argument remains usable after the call.

Returns: the index if found; `-1` if not found.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.find_index([1, 2, 3, 4], 3) == 2)
    assert(list.find_index([1, 2], 99) == -1)
}
```

### map

<!-- stdlib:sig:list.map start -->

```yaoxiang
map: (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
```

<!-- stdlib:sig:list.map end -->

Calls `fn` on each element and returns a new list of the results. The function value is passed in
**curried** form: `list.map(nums, x => x * 2)`. The source list is unchanged.

`map` is statically generic (`src/std/list.yx:210`); a compile-time type error is reported when the
second argument is not a function.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    doubled = list.map([1, 2, 3], x => x * 2)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:list.filter start -->

```yaoxiang
filter: (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
```

<!-- stdlib:sig:list.filter end -->

Retains elements for which `fn` is true. The source list is unchanged.

`filter` is statically generic (`src/std/list.yx:222`); a compile-time type error is reported when
the second argument is not a function.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    evens = list.filter([1, 2, 3, 4], x => x % 2 == 0)
    assert(list.len(evens) == 2)
}
```

### reduce

<!-- stdlib:sig:list.reduce start -->

```yaoxiang
reduce: (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc
```

<!-- stdlib:sig:list.reduce end -->

Folds from left to right: starting with `init`, calls `fn(acc, item)` in turn.

- `fn` — reduction function `(accumulator, element) -> new accumulator`
- `init` — initial accumulator

Returns: the final accumulator. Returns `init` when the list is empty.

`reduce` is statically generic (`src/std/list.yx:241`); a compile-time type error is reported when
the second argument is not a function. Note that `f` receives the accumulator and element by value —
each round passes the current accumulator to `f`, and uses the return value as the accumulator for
the next round.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    total = list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0)
    assert(total == 10)
}
```

### iter

<!-- stdlib:sig:list.iter start -->

```yaoxiang
iter: (T: Type) -> (list: Vec(T)) -> Iter(T)
```

<!-- stdlib:sig:list.iter end -->

Creates an iterator. The iterator is a record type `Iter(T)` exported by this module, containing two
named fields — buffer and cursor (`src/std/list.yx:259-262`):

```
Iter: (T: Type) -> Type = {
    buf: Vec(T),
    pos: Int,
}
```

The source list is passed by value and is **moved** after the call.

Returns: an `Iter(T)` value, to be used with [`next`](#next) / [`has_next`](#has_next).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))
}
```

### next

<!-- stdlib:sig:list.next start -->

```yaoxiang
next: (T: Type) -> (it: &mut Iter(T)) -> T
```

<!-- stdlib:sig:list.next end -->

Takes the current element and moves the internal cursor `pos` **in place** forward by one.

Returns: the current element.

> **Borrow semantics**: `next` takes `&mut Iter(T)` and `has_next` takes `&Iter(T)` — **neither
> moves** the iterator, so the same iterator can be used to fetch elements consecutively. Please
> check [`has_next`](#has_next) before calling: after iteration ends, taking again will be caught by
> `Vec`'s boundary check and report `E6003`, **does not return `Void`** (`src/std/list.yx:277-285`).
>
> This is the opposite of [`std.range.next`](range#next) — `range.has_next`'s signature has no `&`,
> so it consumes the iterator by value.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])

    // Same iterator taken consecutively — because next borrows rather than consumes
    assert(list.next(it) == 7)
    assert(list.next(it) == 8)
    assert(!list.has_next(it))
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

Whether there are unconsumed elements. Read-only borrows `&Iter(T)`; does not consume the iterator.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
    assert(list.next(it) == 1)
    assert(!list.has_next(it))
}
```

### for ... in Iteration

Lists can be iterated directly with `for ... in`, with no need to manually call `next`:

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## Related

- [`std.range`](range) — range iteration and lazy adapters
- [`std.assert`](assert) — assertion utility used in examples
