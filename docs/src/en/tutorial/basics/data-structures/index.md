---
title: 'Lists and Dictionaries'
---

# Lists and Dictionaries

Data structures are the skeleton of programs. YaoXiang's collection types are layered: `Vec(T)` is
the runtime-length raw buffer, `List(T)` is a standard library type
(`{ data: Vec(T), length: Int }`), and dictionaries are built on the same layering. Users can also
define their own container types—after implementing the `Index` interface (RFC-011b), subscript
access with `[]` is also supported.

## Lists

A list is an **ordered** sequence of values, where all elements share the same type. Create one with
`[]`:

```yaoxiang
// 创建列表
numbers = [1, 2, 3, 4, 5]
names = ["Alice", "Bob", "Charlie"]
empty: List(Int) = []       // 空列表需要类型注解
```

### Index Access

Use `[]` to access elements by position, with 0-based indexing:

```yaoxiang
scores = [95, 87, 73, 91]

first = scores[0]    // 95
second = scores[1]   // 87
last = scores[3]     // 91
```

### Common Operations

```yaoxiang
mut items = [1, 2, 3]

// 添加元素
items.append(4)       // [1, 2, 3, 4]

// 长度
count = items.len()   // 4

// 切片
slice = items[0..2]   // [1, 2]
```

### List Comprehensions

List comprehensions are a powerful tool for creating lists—generate a new list from an existing one:

```yaoxiang
// 基本推导式
squares = [x * x for x in [1, 2, 3, 4, 5]]
print(squares)  // [1, 4, 9, 16, 25]

// 带过滤条件的推导式
evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
print(evens)  // [2, 4, 6]

// 转换类型
names = ["Alice", "Bob", "Charlie"]
lengths = [n.len() for n in names]
print(lengths)  // [5, 3, 7]
```

Syntax: `[expression for variable in list if condition]`—the `if condition` part is optional.

## Dictionaries

A dictionary is a collection of **key-value pairs**, where keys are strings and values may be of any
type. Create one with `{}`:

```yaoxiang
// 创建字典
scores = {"Alice": 90, "Bob": 85, "Charlie": 92}
empty: Dict(Int) = {}          // 空字典需要类型注解
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

// 添加/更新键值
data["age"] = 25
data["name"] = "Bob"

print(data)  // {"name": "Bob", "age": 25}
```

### Membership Check

Use `in` to check whether a key exists:

```yaoxiang
config = {"host": "localhost", "port": "8080"}

has_host = "host" in config    // true
has_user = "user" in config    // false
```

## Summary

| Type | Syntax      | Ordered? | Duplicates?       | Key Type      |
| ---- | ----------- | -------- | ----------------- | ------------- |
| List | `[1, 2, 3]` | ✅       | ✅                | Integer Index |
| Dict | `{"a": 1}`  | ✅       | No duplicate keys | String        |

Lists are your workhorse container; dictionaries are best for key-value lookups.
