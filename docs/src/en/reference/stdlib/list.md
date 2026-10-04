---
title: 'std.list'
description: 'List add/remove, slice, higher-order functions, and iterator protocol'
---

# std.list

List operations module. **Pay special attention to move semantics**: there are two categories of
functions—those that only read-borrow the source list, and those that consume (move) the source
list. The auto-borrowing rules for `&` parameters are described in RFC-009 §2.8: when the argument
is still used after the call, the compiler automatically creates a read-only token.

```yaoxiang
use std.list
```

## Semantic Categories

Parameters marked with `&` in the signature are read-only borrows; the source value remains usable
after the call. Parameters without `&` are passed by value; after the call the source value **has
been moved**, and using it again will raise `E2014`.

| Category                 | Functions                                                                                                        | Behavior                                                                                              |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **Consumes source list** | `push` `append` `prepend` `set` `pop` `remove_at` `iter`                                                         | The source list is moved and cannot be used again                                                     |
| Read-only borrow         | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | The source list can be used repeatedly; the `item` of `contains`/`find_index` is also borrowed (`&A`) |
| Iterator protocol        | `has_next` (`&Iter(T)`) `next` (`&mut Iter(T)`)                                                                  | Both **borrow** the iterator and do not consume it                                                    |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // Read-only borrow: nums can be used repeatedly
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // Consumes: base cannot be used after this point
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

<!-- stdlib:table:list end -->

## Functions

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.push end -->

Returns a **new list** with `item` appended to the end of `list`. `list` is passed by value and is
**moved** upon call, so it cannot be used again.

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

An alias for `push`; behavior is exactly the same.

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

Returns a new list with `item` inserted at the head of `list`. `list` is passed by value and is
**moved** upon call.

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
consumed and is no longer an exception to the "signatures with `&` still mutate the source in place"
rule from the native version.

Returns: a new list with the last element removed; if the list is empty, returns it unchanged. To
read the removed element, use `last` to fetch its value before the call.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l is consumed; rest is the new shortened list
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // the last element 3 has been removed

    // To read the removed element, use last to fetch it before popping
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

- `index` —— element index

Returns: a new list with that element removed. Error: when `index` is **negative**, raises `E6003`
(the `list[i + 1]` in `src/std/list.yx:156-158` goes out of bounds).

> **Out-of-bounds does not raise**: when `index` is ≥ the length, the copy loop never executes even
> once; it only decrements `length` by 1 and silently truncates——`list.remove_at([1, 2, 3], 5)`
> returns a two-element list and **does not raise**. Please check bounds yourself before calling.

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

Returns a new list with the value at index `index` overwritten to `value`. `list` is passed by value
and is **moved** upon call.

- `index` —— index
- `value` —— new value

Error: when `index` is negative or ≥ the length, raises `E6003` (out-of-bounds writes are no longer
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

- `index` —— index

Returns: the element value.

Error: when `index` is negative or ≥ the length, raises `E6003` (`src/std/list.yx:78` directly uses
`list[index]`; out-of-bounds is caught by the `[]` bounds check, **not returning `Void`**).

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

Error: **an empty list will raise `E6003`**——the fallback branch in `src/std/list.yx:82-89` reads
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

Error: **an empty list will raise `E6003`** (same as [`first`](#first); `src/std/list.yx:92-99`).

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

Takes the sub-list over the interval `[start, end)`.

- `start` —— starting index
- `end` —— ending index (exclusive)

Returns: a new list. When `start >= end`, returns an empty list.

Error: **no bounds clamping**——`src/std/list.yx:196-205` reads `list[i]` one by one; an
out-of-bounds index (including negative `start` / `end`) directly raises `E6003`.
`list.slice([1, 2, 3], 1, 99)` will raise rather than returning a two-element list.

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

Returns a new list with elements in reverse order; the source list is unchanged.

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

Concatenates two lists and returns a new list. Both source lists remain unchanged.

Type checking is performed at compile time: `concat` is statically generic (`src/std/list.yx:179`);
when the second argument is not a list, it reports a type error, **with no runtime `E6007` path**.

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

Number of elements. Read-only borrow; `list` can be used repeatedly.

`len` is statically generic (`src/std/list.yx:67`); a non-list argument is a compile-time type
error, not a runtime exception.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)      // reusable
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type) -> (list: &Vec(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

Whether the list is empty.

Like `len`, statically generic (`src/std/list.yx:72`); a non-list argument is a compile-time type
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

Whether `item` is in the list (compared by value equality; element types must support
`==`——primitive types support it natively, and record types are provided by auto-derivation or
explicit instance of `Equal` per RFC-011b). `item` is passed as a read-only borrow (the call site
automatically creates an `&A` token), and the argument remains usable after the call.

Returns: `true` if present; `false` if not (linear scan, does not raise).

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

The index of the first occurrence of `item`. `item` is passed as a read-only borrow (the call site
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

Calls `fn` on each element and returns a new list composed of the results. Passing a function value
uses the **curried** form: `list.map(nums, x => x * 2)`. The source list is unchanged.

`map` is statically generic (`src/std/list.yx:210`); when the second argument is not a function, it
reports a compile-time type error.

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

Keeps elements for which `fn` returns true. The source list is unchanged.

`filter` is statically generic (`src/std/list.yx:222`); when the second argument is not a function,
it reports a compile-time type error.

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

Folds from left to right: with `init` as the initial value, calls `fn(acc, item)` in turn.

- `fn` —— reduction function `(accumulator, element) -> new accumulator`
- `init` —— initial accumulator value

Returns: the final accumulator. When the list is empty, returns `init`.

`reduce` is statically generic (`src/std/list.yx:241`); when the second argument is not a function,
it reports a compile-time type error. Note that `f` receives the accumulator and element by
value——each round hands the current accumulator to `f` and uses the return value as the next round's
accumulator.

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

Creates an iterator. The iterator is the record type `Iter(T)` exported by this module, with two
named fields——a buffer and a cursor (`src/std/list.yx:259-262`):

```
Iter: (T: Type) -> Type = {
    buf: Vec(T),
    pos: Int,
}
```

The source list is passed by value and is **moved** upon call.

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

Takes out the current element and advances the internal cursor `pos` **in place** by one.

Returns: the current element.

> **Borrow semantics**: `next` takes `&mut Iter(T)` and `has_next` takes `&Iter(T)`; **neither
> moves** the iterator, so the same iterator can be used to fetch elements in succession. Please
> check [`has_next`](#has_next) before calling: fetching again after the iterator is exhausted will
> be caught by `Vec`'s bounds check and reported as `E6003`, **not returning `Void`**
> (`src/std/list.yx:277-285`).
>
> This is the opposite of [`std.range.next`](./range#next)——the signature of `range.has_next` has no
> `&`, and consumes the iterator by value.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])

    // The same iterator fetches in succession—because next borrows rather than consumes
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

Whether there are still unconsumed elements. Read-only borrows `&Iter(T)`, not consuming the
iterator.

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

### for ... in Traversal

Lists can be traversed directly with `for ... in`, without manually calling `next`:

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

- [`std.range`](./range) —— Range iteration and lazy adapters
- [`std.assert`](./assert) —— Assertion utilities used in the examples
