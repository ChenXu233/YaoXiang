---
title: 'std.list'
description: 'List add/remove, slicing, higher-order functions and iterator protocol'
---

# std.list

List operations module. **Pay special attention to move semantics**: there are two kinds of
functions—those that only borrow the source list, and those that consume (move) it. The auto-borrow
rule for `&` parameters is described in RFC-009 §2.8: when the argument is used after the call, the
compiler automatically creates a read-only token.

```yaoxiang
use std.list
```

## Semantic Categories

Parameters with `&` in their signature are read-only borrows—the source value remains usable after
the call. Parameters without `&` are passed by value, and the source value is **moved** after the
call; using it again will raise `E2014`.

| Category                | Functions                                                                                                        | Behavior                                  |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **Consume source list** | `push` `append` `prepend` `set` `pop` `remove_at`                                                                | Source list is moved, unusable afterwards |
| Read-only borrow        | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` | Source list can be used repeatedly        |
| Iterator protocol       | `iter` (consumes source list, returns iterator) `has_next` `next` (borrows / mutably borrows iterator)           | See explanation below                     |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // Read-only borrow: nums can be used repeatedly
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // Consume: base cannot be used after this point
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
| `contains`   | `(A: Type) -> (list: &Vec(A), item: A) -> Bool`                                            |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: A) -> Int`                                             |
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
**moved** after the call—it cannot be used again.

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

An alias for `push`, with identical behavior.

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
**moved** after the call.

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
consumed; this is not the native "signature has `&` yet mutates the source in place" exception.

Returns: a new list with the last element removed; returns the original unchanged when the list is
empty. To read the removed element, use `last` to retrieve it before calling.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l is consumed, rest is the new shortened list
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // the last element 3 has been removed

    // To read the removed element, use last before pop
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

Returns: a new list with that element removed. Errors: raises `E6003` (index out of bounds) when the
index is negative or ≥ the length.

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

Returns a new list with the element at index `index` overwritten by `value`. `list` is passed by
value and is **moved** after the call.

- `index` —— index; defaults to `0`
- `value` —— new value; defaults to `Void`

Errors: raises `E6003` when the index is negative or ≥ the length (out-of-bounds writes are no
longer silently discarded).

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

Reads the element at index `index` (read-only borrow, `list` can be reused).

- `index` —— index; defaults to `0`

Returns: the element value; **out-of-bounds returns `Void`** (does not throw). Errors: raises
`E6007` when the index is negative.

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

Returns the first element; returns `Void` for an empty list.

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

Returns the last element; returns `Void` for an empty list.

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

Takes the sublist of the interval `[start, end)`.

- `start` —— starting index; defaults to `0`
- `end` —— ending index (exclusive); defaults to the end of the list

Returns: a new list. Boundaries are **clamped** to the valid range without raising an error. Errors:
raises `E6007` when `start` or `end` is negative.

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

Returns a new list with the element order reversed; the source list is unchanged.

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

Errors: raises `E6007` when the second argument is not a list.

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

Errors: raises `E6007` when the argument is not a list.

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

Errors: raises `E6007` when the argument is not a list.

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
contains: (A: Type) -> (list: &Vec(A), item: A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

Whether `item` is in the list (compared by value equality).

Returns: `true` if present; returns `false` when the argument is not a list.

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
find_index: (A: Type) -> (list: &Vec(A), item: A) -> Int
```

<!-- stdlib:sig:list.find_index end -->

The index of the first occurrence of `item`.

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

Calls `fn` on each element and returns a new list composed of the results. The function value is
passed in **curried** form: `list.map(nums, x => x * 2)`. The source list is unchanged.

Errors: raises `E6007` when the second argument is not a function.

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

Keeps the elements for which `fn` returns true. The source list is unchanged.

Errors: raises `E6007` when the second argument is not a function.

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

Folds from left to right: starting with `init`, calls `fn(acc, item)` in order.

- `fn` —— reduction function `(accumulator, element) -> new accumulator`
- `init` —— initial accumulator

Returns: the final accumulator. Returns `init` for an empty list.

Errors: raises `E6007` when the second argument is not a function.

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

Creates an iterator. The iterator is a `(list, index)` tuple state carrier; once created, it is
**consumed in order** within `next`. The source list is read-only borrowed and remains usable during
iteration.

Returns: an iterator tuple, to be used with `next` / `has_next`.

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

Takes out the current element and advances the internal index by one.

Returns: the current element; returns `Void` when iteration ends.

> Both `next` and `has_next` **move** the iterator (no `&` in the signature), so each access
> requires recreating the iterator, or you may use a `for ... in` loop directly. This differs from
> the borrow form of [`std.range.next`](./range#next).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])
    assert(list.next(it) == 7)
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

Whether there are still unconsumed elements.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
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

- [`std.range`](./range) —— range iteration and lazy adapters
- [`std.assert`](./assert) —— assertion utility used in the examples
