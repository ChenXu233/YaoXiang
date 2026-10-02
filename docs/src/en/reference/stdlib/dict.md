---
title: 'std.dict'
description: 'Dictionary read/write, key-value views, and merging'
---

# std.dict

Operations module for dictionaries (`Dict(K, V)`).

```yaoxiang
use std.dict
```

## Semantic Categories

| Category                       | Functions                                                      | Behavior                                                |
| ------------------------------ | -------------------------------------------------------------- | ------------------------------------------------------- |
| Read-only borrow               | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | source dictionary can be reused                         |
| No arguments                   | `new`                                                          | construct an empty dictionary                           |
| **Consumes source dictionary** | `set` `delete`                                                 | source dictionary is moved and cannot be used afterward |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)

    // Read-only borrow: d can be reused
    assert(dict.get(d, "a") == 1)
    assert(dict.len(d) == 1)
    assert(dict.has(d, "a"))
}
```

## Function Overview

<!-- stdlib:table:dict start -->

| Function   | Signature                                                                  |
| ---------- | -------------------------------------------------------------------------- |
| `get`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any`                   |
| `set`      | `(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)` |
| `has`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool`                  |
| `values`   | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `keys`     | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `entries`  | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)`                 |
| `delete`   | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)`             |
| `len`      | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int`                             |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool`                            |
| `merge`    | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)`         |
| `new`      | `(K: Type, V: Type)() -> Dict(K, V)`                                       |

<!-- stdlib:table:dict end -->

## Functions

### new

<!-- stdlib:sig:dict.new start -->

```yaoxiang
new: (K: Type, V: Type)() -> Dict(K, V)
```

<!-- stdlib:sig:dict.new end -->

Create an empty dictionary. `K` / `V` are inferred from context.

> **You must use it; do not use `{}`**: `{}` is an **empty block** (value is `Void`), not an empty
> dictionary (`src/std/dict.rs:97-100`). The only construction channel for empty dictionaries is
> `dict.new()`. Dictionary literals `{ "k": v, … }` are still available, see
> [Syntax Specification §1.6.4](../language-spec/syntax.md).

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.new()
    assert(dict.is_empty(d))
    assert(dict.len(d) == 0)

    d2 = dict.set(d, "a", 1)
    assert(!dict.is_empty(d2))
}
```

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

Returns a **new dictionary** with `key` → `value` written. `dict` is passed by value, and is
**moved** after the call.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d1 = dict.set(dict.new(), "a", 1)
    d2 = dict.set(d1, "b", 2)
    assert(dict.len(d2) == 2)
}
```

### get

<!-- stdlib:sig:dict.get start -->

```yaoxiang
get: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any
```

<!-- stdlib:sig:dict.get end -->

Retrieve a value by key (read-only borrow, `dict` is reusable).

Returns: the value corresponding to the key. Error: **throws `E6008` if the key does not exist**
(missing key). Use [`has`](#has) to check before retrieving.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

Check existence before retrieving:

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.has(d, "nope"))
    if dict.has(d, "a") {
        assert(dict.get(d, "a") == 1)
    }
}
```

### has

<!-- stdlib:sig:dict.has start -->

```yaoxiang
has: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
```

<!-- stdlib:sig:dict.has end -->

Whether `key` exists in the dictionary.

Error: throws `E6007` if the first argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.has(d, "a"))
    assert(!dict.has(d, "zzz"))
}
```

### delete

<!-- stdlib:sig:dict.delete start -->

```yaoxiang
delete: (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.delete end -->

Returns a **new dictionary** with `key` removed. `dict` is passed by value, and is **moved** after
the call.

Returns: the new dictionary. Deleting a non-existent key does not error, and the dictionary remains
unchanged.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.keys end -->

Returns a list of all keys (read-only borrow).

> The return order depends on the hash implementation, **stability is not guaranteed**. Sort
> yourself if ordered output is needed.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.values end -->

Returns a list of all values (read-only borrow). Order is not guaranteed to be stable.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> Vec(C)
```

<!-- stdlib:sig:dict.entries end -->

Returns a list of key-value pairs, each item being a `(key, value)` tuple. Order is not guaranteed
to be stable.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    es = dict.entries(d)
    assert(list.len(es) == 1)
}
```

### len

<!-- stdlib:sig:dict.len start -->

```yaoxiang
len: (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
```

<!-- stdlib:sig:dict.len end -->

Number of entries. Read-only borrow, `dict` is reusable.

Error: throws `E6007` if the argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // reusable
}
```

### is_empty

<!-- stdlib:sig:dict.is_empty start -->

```yaoxiang
is_empty: (K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
```

<!-- stdlib:sig:dict.is_empty end -->

Whether the dictionary is empty.

Error: throws `E6007` if the argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    assert(dict.is_empty(dict.new()))
    d = dict.set(dict.new(), "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

Merges two dictionaries and returns a new dictionary. Both source dictionaries are read-only
borrows; neither is modified.

On key conflict, **the value from `b` overwrites `a`**.

Error: throws `E6007` if any argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set(dict.new(), "x", 10), dict.set(dict.new(), "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## See Also

- [`std.list`](./list) — for processing the return values of `keys` / `values` / `entries`
- [Error Code Reference](../error-code/) — `E6008` missing key
