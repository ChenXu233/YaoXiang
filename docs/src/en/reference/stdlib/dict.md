---
title: 'std.dict'
description: 'Dictionary read/write, key-value views, and merge'
---

# std.dict

The dictionary (`Dict(K, V)`) operation module.

```yaoxiang
use std.dict
```

## Semantic Categories

| Category            | Functions                                                      | Behavior                                 |
| ------------------- | -------------------------------------------------------------- | ---------------------------------------- |
| Read-only borrow    | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | Source dict is reusable                  |
| **Consumes source** | `set` `delete`                                                 | Source dict is moved, unusable afterward |

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)

    // Read-only borrow: d is reusable
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

Fetches the value by key (read-only borrow, `dict` is reusable).

Returns: the value corresponding to the key. Error: **throws `E6008` when the key is missing**. You
can use [`has`](#has) to check before fetching.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

Check existence before fetching:

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

Error: throws `E6007` when the first argument is not a dictionary.

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

Returns a **new dictionary** with `key` removed. `dict` is passed by value and is **moved** after
the call.

Returns: the new dictionary. Deleting a non-existent key does not raise an error; the dictionary
remains unchanged.

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

> The return order depends on the hash implementation and is **not guaranteed to be stable**. If you
> need ordered output, sort it yourself.

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

Returns a list of all values (read-only borrow). The order is not guaranteed to be stable.

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

Returns a list of key-value pairs, where each item is a `(key, value)` tuple. The order is not
guaranteed to be stable.

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

The number of entries. Read-only borrow, `dict` is reusable.

Error: throws `E6007` when the argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    d = dict.set(dict.new(), "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // Reusable
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

Merges two dictionaries and returns a new dictionary. Both source dictionaries are read-only borrows
and remain unchanged.

On key conflict, **the value from `b` overrides `a`**.

Error: throws `E6007` when either argument is not a dictionary.

```yaoxiang
use std.assert
use std.dict

main: () -> Void = {
    m = dict.merge(dict.set(dict.new(), "x", 10), dict.set(dict.new(), "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## Related

- [`std.list`](./list) — handles the return values of `keys` / `values` / `entries`
- [Error Code Reference](../error-code/) — `E6008` for missing key
