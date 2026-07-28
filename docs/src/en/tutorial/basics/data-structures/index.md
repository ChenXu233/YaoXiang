---
title: Lists and Dictionaries
---

# Lists and Dictionaries

Data structures are the skeleton of programs. YaoXiang provides two built-in collection types: lists and dictionaries.

## Lists

A list is an **ordered** sequence of values, where all elements have the same type. Create them with `[]`:

```yaoxiang
// Creating a list
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       // Empty lists need type annotations
```

### Index Access

Use `[]` to access elements by position, with indices starting from 0:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### Common Operations

```yaoxiang
mut items = [1, 2, 3]

// Adding elements
items.append(4)       // [1, 2, 3, 4]

// Length
count = items.len()   // 4

// Slice
slice = items[0..2]   // [1, 2]
```

### List Comprehensions

List comprehensions are a powerful tool for creating lists—generating new lists from existing ones:

```yaoxiang
// Basic comprehension
squares = [x * x for x in [1, 2, 3, 4, 5]]
print(squares)  // [1, 4, 9, 16, 25]

// Comprehension with filter condition
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
print(evens)  // [2, 4, 6]

// Type conversion
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
print(lengths)  // [5, 3, 7]
```

Syntax: `[expression for variable in list if condition]`—the `if condition` part is optional.

## Dictionaries

A dictionary is a collection of **key-value pairs**, where keys are strings and values can be of any type. Create them with `{}`:

```yaoxiang
// Creating a dictionary
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          // Empty dictionaries need type annotations
```

### Key Access

Use `[]` to access values by key:

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   // 90
bob = scores["Bob"]       // 85
```

### Modifying Dictionaries

```yaoxiang
mut data = {"name": "Alice"}

// Adding/updating key-value pairs
data["age"] = 25
data["name"] = "Bob"

print(data)  // {"name": "Bob", "age": 25}
```

### Membership Testing

Use `in` to check if a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## Summary

| Type       | Syntax         | Ordered? | Allows Duplicates? | Key Type     |
| ---------- | -------------- | -------- | ------------------ | ------------ |
| List       | `[1, 2, 3]`    | ✅        | ✅                  | Integer index |
| Dictionary | `{"a": 1}`     | ✅        | Keys are unique    | String       |

Lists are your go-to containers, and dictionaries are well-suited for key-value lookups.
