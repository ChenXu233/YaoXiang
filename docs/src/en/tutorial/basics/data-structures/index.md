---
title: 'Lists and Dictionaries'
---

# Lists and Dictionaries

Data structures are the backbone of a program. YaoXiang's collection types are built in layers:
`Vec(T)` is a primitive buffer with a runtime length, `List(T)` is a standard-library type
(`{ data: Vec(T), length: Int }`), and dictionaries are built on the same layered mechanism. Users
can also define their own container types — once they implement the `Index` interface (RFC-011b),
they likewise support `[]` subscript access.

## Lists

A list is an **ordered** sequence of values where all elements share the same type. Create one with
`[]`:

```yaoxiang
// Create lists
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]

// Note: `empty: List(Int) = []` reports E1002 — the `List(T)` annotation on an empty list is
// incompatible with the `Vec` inferred from the literal. Construct empty lists via functions like list.push.
```

### Index access

Use `[]` to access elements by position; indices start at 0:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### Common operations

```yaoxiang
use std.list

main = () => {
    mut items = [1, 2, 3]

    // Add element
    items = list.push(items, 4)   // [1, 2, 3, 4]

    // Length
    count = list.len(items)   // 4

    // Slice
    // Note: range slice items[0..2] currently reports E6007 at runtime (tested); use std.list.slice instead
    sl = list.slice(items, 0, 2)   // [1, 2]
    print(items)
    print(count)
    print(sl)
}
```

### List comprehensions

List comprehensions are a powerful way to build lists — generating a new list from an existing one:

```yaoxiang
use std.string

main = () => {
    // Basic comprehension
    squares = [x * x for x in [1, 2, 3, 4, 5]]
    print(squares)  // [1, 4, 9, 16, 25]

    // Transform types
    names = ["Alice", "Bob", "Charlie"]
    // Note: list comprehension currently depends on std.list.iter, and may report E6006 at runtime
    lengths = [string.len(n) for n in names]
    print(lengths)  // [5, 3, 7]
}
```

Syntax: `[expression for variable in list]`.

⚠️ The 0.8.2 comprehension **does not support an `if condition` suffix** —
`[x for x in xs if x % 2 == 0]` reports the parse error「Expected RBracket, found KwIf」(`E0010`).
For filtering, use `std.list.filter` instead:

```yaoxiang
use std.list

main = () => {
    evens = list.filter([1, 2, 3, 4, 5, 6], (x) => x % 2 == 0)
    print(evens)  // [2, 4, 6]
}
```

## Dictionaries

A dictionary is a collection of **key-value pairs**; keys are strings, and values may be of any
type. Create one with `{}`:

```yaoxiang
use std.dict

main = () => {
    // Create dictionary
    scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
    // Note: `{}` is parsed as an empty block (E1108); empty dictionaries must be created with dict.new()
    empty = dict.new()
    print(scores)
    print(empty)
}
```

### Key access

Use `[]` to access a value by key:

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   // 90
bob = scores["Bob"]       // 85
```

### Modifying a dictionary

```yaoxiang
use std.dict

main = () => {
    // Add/update key-value
    // Dictionary subscript assignment works (tested on 0.8.2: `data["k"] = v` reads back correctly),
    // but please use dict.set — it returns a new dictionary, so rebinding is required.
    data = dict.set({"name": "Alice"}, "age", 25)

    print(data["name"])  // Alice
    print(data["age"])   // 25
}
```

### Membership test

Use `in` to check whether a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## Summary

| Type | Syntax      | Ordered? | Duplicates? | Key type  |
| ---- | ----------- | -------- | ----------- | --------- |
| List | `[1, 2, 3]` | ✅       | ✅          | Int index |
| Dict | `{"a": 1}`  | ✅       | Keys unique | String    |

Lists are your workhorse container; dictionaries fit key-value lookup.
