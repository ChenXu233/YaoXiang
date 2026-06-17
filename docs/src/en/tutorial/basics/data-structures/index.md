---
title: Lists and Dictionaries
---

# Lists and Dictionaries

Data structures are the backbone of programs. YaoXiang provides three built-in collection types: lists, dictionaries, and sets.

## Lists

A list is an **ordered** sequence of values, where all elements have the same type. Create one with `[]`:

```yaoxiang
# Create a list
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       # Empty list requires a type annotation
```

### Index Access

Use `[]` to access elements by position; indices start from 0:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    # 95
second = scores[1]   # 87
last = scores[3]     # 91
```

### Common Operations

```yaoxiang
mut items = [1, 2, 3]

# Add an element
items.append(4)       # [1, 2, 3, 4]

# Length
count = items.len()   # 4

# Slice
slice = items[0..2]   # [1, 2]
```

### List Comprehensions

List comprehensions are a powerful tool for creating lists—generate a new list from an existing one:

```yaoxiang
# Basic comprehension
squares = [x * x for x in [1, 2, 3, 4, 5]]
println(squares)  # [1, 4, 9, 16, 25]

# Comprehension with filter
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
println(evens)  # [2, 4, 6]

# Transform types
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
println(lengths)  # [5, 3, 7]
```

Syntax: `[expression for variable in list if condition]`—the `if condition` part is optional.

## Dictionaries

A dictionary is a collection of **key-value pairs**, where keys are strings and values can be of any type. Create one with `{}`:

```yaoxiang
# Create a dictionary
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          # Empty dictionary requires a type annotation
```

### Key Access

Use `[]` to access values by key:

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   # 90
bob = scores["Bob"]       # 85
```

### Modifying a Dictionary

```yaoxiang
mut data = {"name": "Alice"}

# Add/update a key-value pair
data["age"] = 25
data["name"] = "Bob"

println(data)  # {"name": "Bob", "age": 25}
```

### Membership Check

Use `in` to check if a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    # true
has_user = "user" in config    # false
```

## Sets

A set is a collection of **unordered, unique** values. Create one with `{}` (the difference from a dictionary: no colon):

```yaoxiang
# Create a set
colors = {"red", "green", "blue"}
numbers = {1, 2, 3, 3, 2, 1}   # Duplicates are automatically removed

println(numbers)  # {1, 2, 3}
```

### Set Operations

```yaoxiang
mut tags = {"rust", "compiler"}

# Add
tags.insert("language")

# Membership check
has_rust = "rust" in tags      # true
has_python = "python" in tags  # false
```

## Summary

| Type | Syntax | Ordered? | Duplicates Allowed? | Key Type |
|------|------|--------|----------|--------|
| List | `[1, 2, 3]` | ✅ | ✅ | Integer index |
| Dictionary | `{"a": 1}` | ✅ | Keys unique | String |
| Set | `{1, 2, 3}` | ❌ | ❌ | None |

Lists are your workhorse container, dictionaries are great for key-value lookups, and sets are suitable for deduplication and membership checks.