---
title: 'Lists and Dictionaries'
---

# Lists and Dictionaries

Data structures are the backbone of a program. YaoXiang's collection types are layered: `Vec(T)` is
a runtime-length raw buffer, `List(T)` is a standard library type (`{ data: Vec(T), length: Int }`),
and dictionaries are built on the same layering mechanism. Users can also define their own container
types — after implementing the `Index` interface (RFC-011b), they likewise support `[]` subscript
access.

## List

A list is an **ordered** sequence of values, where all elements have the same type. Create one with
`[]`:

```yaoxiang
// Create a list
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]

// Note: `empty: List(Int) = []` raises E1002 — the `List(T)` annotation on an empty list is incompatible with the `Vec` inferred from the literal. For empty lists, please use a function like list.push to construct them.
```

### Index Access

Use `[]` to access an element by position, with indices starting from 0:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### Common Operations

```yaoxiang
use std.list

main = () => {
    mut items = [1, 2, 3]

    // Add an element
    items = list.push(items, 4)   // [1, 2, 3, 4]

    // Length
    count = list.len(items)   // 4

    // Slice
    // Note: range slicing items[0..2] currently raises E6007 at runtime (verified), please use std.list.slice
    sl = list.slice(items, 0, 2)   // [1, 2]
    print(items)
    print(count)
    print(sl)
}
```

### List Comprehensions

List comprehensions are a powerful tool for creating lists — generating new lists from existing
ones:

```yaoxiang
use std.string

main = () => {
    // Basic comprehension
    squares = [x * x for x in [1, 2, 3, 4, 5]]
    print(squares)  // [1, 4, 9, 16, 25]

    // Comprehension with filter
    evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
    print(evens)  // [2, 4, 6]

    // Transform types
    names = ["Alice", "Bob", "Charlie"]
    lengths = [string.len(n) for n in names]
    print(lengths)  // [5, 3, 7]
}
```

Syntax: `[expression for variable in list if condition]` — the `if condition` part is optional.

## Dictionary

A dictionary is a collection of **key-value pairs**, where keys are strings and values can be of any
type. Create one with `{}`:

```yaoxiang
use std.dict

main = () => {
    // Create a dictionary
    scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
    // Note: `{}` is parsed as an empty block (E1108), so empty dictionaries must use dict.new()
    empty = dict.new()
    print(scores)
    print(empty)
}
```

### Key Access

Use `[]` to access a value by its key:

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   // 90
bob = scores["Bob"]       // 85
```

### Modifying a Dictionary

```yaoxiang
use std.dict

main = () => {
    // Add/update a key-value pair
    // Dictionary subscript assignment works (verified on 0.8.2: `data["k"] = v` can be read back after writing),
    // but please use dict.set — it returns a new dictionary that needs to be rebound.
    data = dict.set({"name": "Alice"}, "age", 25)

    print(data["name"])  // Alice
    print(data["age"])   // 25
}
```

### Membership Check

Use `in` to check whether a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## Summary

| Type       | Syntax      | Ordered? | Duplicates?       | Key Type      |
| ---------- | ----------- | -------- | ----------------- | ------------- |
| List       | `[1, 2, 3]` | ✅       | ✅                | Integer index |
| Dictionary | `{"a": 1}`  | ✅       | No duplicate keys | String        |

Lists are your workhorse container, and dictionaries are suited for key-value lookups.
