---
title: 'std.list'
description: 'List add/remove, slicing, higher-order functions, and iterator protocol'
---

# std.list

List operations module. **Pay special attention to move semantics**: some functions only read-borrow
the source list, others consume (move) it, and two more, although marked as borrow, will modify the
list in place.

```yaoxiang
use std.list
```

## Semantic Classification

Parameters with `&` in the signature are read-only borrows; after the call the source value is still
usable. Parameters without `&` are passed by value; after the call the source value has been
**moved**, and using it again raises `E2014`.

| Category                         | Function                                                                                                                | Behavior                                          |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| **Consumes source list**         | `push` `append` `prepend` `set`                                                                                         | Source list is moved and cannot be used afterward |
| Read-only borrow                 | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce` `iter` | Source list can be reused repeatedly              |
| Borrow but **modifies in place** | `pop` `remove_at`                                                                                                       | Source list contents are changed                  |
| Consumes iterator                | `next` `has_next`                                                                                                       | Iterator tuple is moved                           |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // Read-only borrow: nums can be reused
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // Consumption: base cannot be used after this
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## Function Overview

<!-- stdlib:table:list start -->

| Function     | Signature                                                                     |
| ------------ | ----------------------------------------------------------------------------- |
| `push`       | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `pop`        | `(A: Type)(list: &List(A)) -> Any`                                            |
| `append`     | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `prepend`    | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `remove_at`  | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `reverse`    | `(A: Type)(list: &List(A)) -> List(A)`                                        |
| `concat`     | `(A: Type)(a: &List(A), b: &List(A)) -> List(A)`                              |
| `map`        | `(T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)`                    |
| `filter`     | `(T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)`                 |
| `reduce`     | `(T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any` |
| `len`        | `(A: Type)(list: &List(A)) -> Int`                                            |
| `is_empty`   | `(A: Type)(list: &List(A)) -> Bool`                                           |
| `get`        | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `set`        | `(A: Type)(list: List(A), index: Int, value: A) -> List(A)`                   |
| `first`      | `(A: Type)(list: &List(A)) -> Any`                                            |
| `last`       | `(A: Type)(list: &List(A)) -> Any`                                            |
| `slice`      | `(A: Type)(list: &List(A), start: Int, end: Int) -> List(A)`                  |
| `contains`   | `(A: Type)(list: &List(A), item: Any) -> Bool`                                |
| `find_index` | `(A: Type)(list: &List(A), item: Any) -> Int`                                 |
| `iter`       | `(A: Type)(list: &List(A)) -> Tuple`                                          |
| `next`       | `(iterator: Tuple) -> Any`                                                    |
| `has_next`   | `(iterator: Tuple) -> Bool`                                                   |

<!-- stdlib:table:list end -->## Functions

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.push end -->

Returns a **new list** with `item` appended to the end of `list`. `list` is passed by value, so it
is **moved** after the call and cannot be used again.

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
append: (A: Type)(list: List(A), item: A) -> List(A)
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
prepend: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.prepend end -->

Returns a new list with `item` inserted at the head of `list`. `list` is passed by value, so it is
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
pop: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.pop end -->

Removes and returns the last element. **Modifies `list` in place** — this is an exception where the
signature carries `&` but the source value is still changed.

Returns: the removed element; returns `Void` when the list is empty, and the list remains empty.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [1, 2, 3]
    gone = list.pop(l)
    assert(list.len(l) == 2)         // shortened in place
    assert(gone == 3)

    mut empty = []
    v = list.pop(empty)
    assert(list.is_empty(empty))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.remove_at end -->

Removes and returns the element at index `index`. **Modifies `list` in place**.

- `index` — character/element index; defaults to `0`

Returns: the removed element. Error: raises `E6003` (index out of bounds) when the index is negative
or ≥ length, and the list is left unchanged.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [10, 20, 30]
    x = list.remove_at(l, 1)
    assert(x == 20)
    assert(list.len(l) == 2)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type)(list: List(A), index: Int, value: A) -> List(A)
```

<!-- stdlib:sig:list.set end -->

Returns a new list with index `index` rewritten to `value`. `list` is passed by value, so it is
**moved** after the call.

- `index` — index; defaults to `0`
- `value` — new value; defaults to `Void`

Error: raises `E6003` when the index is negative or ≥ length (out-of-bounds writes are no longer
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
get: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.get end -->

Reads the element at index `index` (read-only borrow; `list` can be reused).

- `index` — index; defaults to `0`

Returns: the element value; **returns `Void` on out-of-bounds** (no error raised). Error: raises
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
first: (A: Type)(list: &List(A)) -> Any
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
last: (A: Type)(list: &List(A)) -> Any
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
slice: (A: Type)(list: &List(A), start: Int, end: Int) -> List(A)
```

<!-- stdlib:sig:list.slice end -->

Takes the sub-list in the `[start, end)` range.

- `start` — starting index; defaults to `0`
- `end` — ending index (exclusive); defaults to the end of the list

Returns: a new list. Boundaries are **clamped** to the valid range; no error is raised. Error:
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
reverse: (A: Type)(list: &List(A)) -> List(A)
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
concat: (A: Type)(a: &List(A), b: &List(A)) -> List(A)
```

<!-- stdlib:sig:list.concat end -->

Concatenates two lists and returns a new list. Both source lists remain unchanged.

Error: raises `E6007` when the second argument is not a list.

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
len: (A: Type)(list: &List(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

Number of elements. Read-only borrow; `list` can be reused repeatedly.

Error: raises `E6007` when the argument is not a list.

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
is_empty: (A: Type)(list: &List(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

Whether the list is empty.

Error: raises `E6007` when the argument is not a list.

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
contains: (A: Type)(list: &List(A), item: Any) -> Bool
```

<!-- stdlib:sig:list.contains end -->

Whether `item` is in the list (compared by value equality).

Returns: `true` if present; `false` if the argument is not a list.

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
find_index: (A: Type)(list: &List(A), item: Any) -> Int
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
map: (T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)
```

<!-- stdlib:sig:list.map end -->

Calls `fn` on each element and returns a new list of the results. Passing the function value uses a
**curried** form: `list.map(nums, x => x * 2)`. The source list is unchanged.

Error: raises `E6007` when the second argument is not a function.

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
filter: (T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)
```

<!-- stdlib:sig:list.filter end -->

Keeps the elements for which `fn` is true. The source list is unchanged.

Error: raises `E6007` when the second argument is not a function.

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
reduce: (T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any
```

<!-- stdlib:sig:list.reduce end -->

Folds from left to right: starting with `init` as the initial value, calls `fn(acc, item)` in turn.

- `fn` — reduction function `(accumulator, item) -> new accumulator`
- `init` — initial accumulator value

Returns: the final accumulator. Returns `init` when the list is empty.

Error: raises `E6007` when the second argument is not a function.

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
iter: (A: Type)(list: &List(A)) -> Tuple
```

<!-- stdlib:sig:list.iter end -->

Creates an iterator. An iterator is a `(list, index)` tuple state carrier; once created, it is
**consumed sequentially** in `next`. The source list is a read-only borrow, and remains usable
during iteration.

Returns: an iterator tuple, to be passed to `next` / `has_next`.

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
next: (iterator: Tuple) -> Any
```

<!-- stdlib:sig:list.next end -->

Takes the current element and advances the internal index by one.

Returns: the current element; returns `Void` when the iteration ends.

> Both `next` and `has_next` **move** the iterator (the signature has no `&`), so each access
> requires creating a new iterator, or just iterating directly with `for ... in`. This differs from
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
has_next: (iterator: Tuple) -> Bool
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

- [`std.range`](./range) — range iteration and lazy adapters
- [`std.assert`](./assert) — assertion utility used in the examples
