---
title: 'std.dict'
description: 'Dictionary read/write, key-value views, and merging'
---

# std.dict

Dictionary (`Dict(K, V)`) operations module.

```yaoxiang
use std.dict
```

## Semantic Categories

| Category            | Functions                                                      | Behavior                             |
| ------------------- | -------------------------------------------------------------- | ------------------------------------ |
| Read-only borrow    | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | Source dict can be reused            |
| **Consumes source** | `set` `delete`                                                 | Source dict is moved, unusable after |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)

    // Read-only borrow: d can be reused
    assert(dict.get(d, "a") == 1)
    assert(dict.len(d) == 1)
    assert(dict.has(d, "a"))
}
```

## Function List

<!-- stdlib:table:dict start -->

| Function   | Signature                                                                  |
| ---------- | -------------------------------------------------------------------------- |
| `get`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any`                   |
| `set`      | `(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)` |
| `has`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool`                  |
| `values`   | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `keys`     | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `entries`  | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `delete`   | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)`             |
| `len`      | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int`                             |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool`                            |
| `merge`    | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)`         |

<!-- stdlib:table:dict end -->

## Functions

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

Returns a **new dictionary** with `key` → `value` written into it. `dict` is passed by value and is
**moved** after the call.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d1 = dict.set({}, "a", 1)
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

Gets a value by key (read-only borrow, `dict` is reusable).

Returns: the value corresponding to the key. Error: **throws `E6008` (missing key) when the key does
not exist**. You can use [`has`](#has) to check first before retrieving.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

Existence check before retrieval:

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

Error: throws `E6007` when the first argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

Returns a **new dictionary** with `key` removed. `dict` is passed by value and is **moved** after
the call.

Returns: a new dictionary. Deleting a non-existent key does not raise an error; the dictionary
remains unchanged.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.keys end -->

Returns a list of all keys (read-only borrow).

> The return order depends on the hash implementation and is **not guaranteed to be stable**. Sort
> manually if you need ordered output.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.values end -->

Returns a list of all values (read-only borrow). The order is not guaranteed to be stable.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.entries end -->

Returns a list of key-value pairs, where each item is a `(key, value)` tuple. The order is not
guaranteed to be stable.

```yaoxiang
use std.assert
use std.dict
use std.list

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

Number of entries. Read-only borrow, `dict` can be reused.

Error: throws `E6007` when the argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set({}, "a", 1)
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

Error: throws `E6007` when the argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    assert(dict.is_empty({}))
    d = dict.set({}, "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

Merges two dictionaries and returns a new one. Both source dictionaries are read-only borrows and
remain unchanged.

On key conflicts, **`b`'s value overrides `a`**.

Error: throws `E6007` when either argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set({}, "x", 10), dict.set({}, "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## Related

- [`std.list`](./list) — works with the return values of `keys` / `values` / `entries`
- [Error code reference](../error-code/) — `E6008` missing key
