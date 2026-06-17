---
title: Lists and Dicts
---

# Lists and Dicts

Data structures are the backbone of any program. YaoXiang provides three built-in collection types: lists, dicts, and sets.

## Lists

A list is an **ordered** sequence of values, all of the same type. Create one with `[]`:

```yaoxiang
# Create lists
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       # Empty lists need a type annotation
```

### Index Access

Access elements by position with `[]`, starting from 0:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    # 95
second = scores[1]   # 87
last = scores[3]     # 91
```

### Common Operations

```yaoxiang
mut items = [1, 2, 3]

# Append
items.append(4)       # [1, 2, 3, 4]

# Length
count = items.len()   # 4

# Slice
slice = items[0..2]   # [1, 2]
```

### List Comprehensions

List comprehensions are a powerful way to create lists — generate new lists from existing ones:

```yaoxiang
# Basic comprehension
squares = [x * x for x in [1, 2, 3, 4, 5]]
println(squares)  # [1, 4, 9, 16, 25]

# With filter condition
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
println(evens)  # [2, 4, 6]

# Type transformation
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
println(lengths)  # [5, 3, 7]
```

Syntax: `[expression for variable in list if condition]` — the `if condition` part is optional.

## Dicts

A dict is a collection of **key-value pairs**. Keys are strings, values can be any type. Create one with `{}`:

```yaoxiang
# Create dicts
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          # Empty dicts need a type annotation
```

### Key Access

Access values by key with `[]`:

```yaoxiang
scores = {"Alice": 90, "Bob": 85}

alice = scores["Alice"]   # 90
bob = scores["Bob"]       # 85
```

### Modifying Dicts

```yaoxiang
mut data = {"name": "Alice"}

# Add/update key-value pairs
data["age"] = 25
data["name"] = "Bob"

println(data)  # {"name": "Bob", "age": 25}
```

### Membership Test

Use `in` to check if a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    # true
has_user = "user" in config    # false
```

## Sets

A set is an **unordered** collection of **unique** values. Create one with `{}` (no colons, unlike dicts):

```yaoxiang
# Create sets
colors = {"red", "green", "blue"}
numbers = {1, 2, 3, 3, 2, 1}   # Duplicates auto-removed

println(numbers)  # {1, 2, 3}
```

### Set Operations

```yaoxiang
mut tags = {"rust", "compiler"}

# Insert
tags.insert("language")

# Membership test
has_rust = "rust" in tags      # true
has_python = "python" in tags  # false
```

## Summary

| Type | Syntax | Ordered? | Duplicates? | Key Type |
|------|--------|----------|-------------|----------|
| List | `[1, 2, 3]` | ✅ | ✅ | Integer index |
| Dict | `{"a": 1}` | ✅ | Keys unique | String |
| Set | `{1, 2, 3}` | ❌ | ❌ | None |

Lists are your workhorse container, dicts excel at key-value lookups, and sets shine for deduplication and membership tests.
