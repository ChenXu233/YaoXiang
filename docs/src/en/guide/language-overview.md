---
title: Syntax Cheatsheet
---

# Syntax Cheatsheet

Learn YaoXiang core syntax in 5 minutes. For in-depth learning, visit the [Tutorial](/tutorial/).

## Variables

```yaoxiang
x = 42                    # Immutable (default)
mut y = 0                 # Mutable

name: String = "hello"    # Explicit type
count: Int = 100          # Type annotation
```

## Functions

```yaoxiang
# Expression form (returns the value directly)
add: (a: Int, b: Int) -> Int = a + b

# Block form (explicit return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

## Types

```yaoxiang
# Record type
type Point = { x: Float, y: Float }
p = Point(x: 1.0, y: 2.0)

# Enum
type Result(T, E) = ok(T) | err(E)
type Color = red | green | blue

# Interface
type Drawable = { draw: (Surface) -> Void }

# Generics
List: (T: Type) -> Type = { data: Array(T), length: Int }
```

## Control Flow

```yaoxiang
# if is an expression
grade = if score >= 90 { "A" } elif score >= 60 { "B" } else { "C" }

# match
result = match value {
    ok(v) => "success: ${v}",
    err(e) => "error: ${e}",
}

# Loops
for i in 0..5 { println(i) }

mut n = 0
while n < 5 { println(n); n = n + 1 }
```

## Data Structures

```yaoxiang
# List
nums = [1, 2, 3, 4, 5]
first = nums[0]           # 1

# Dictionary
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       # 90

# Set
colors = {"red", "green", "blue"}

# List comprehension
evens = [x for x in nums if x % 2 == 0]
```

## Pattern Matching

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

# Struct pattern
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "(${x}, ${y})",
}

# Guard expression
match age {
    adult(n) if n >= 18 => true,
    _ => false,
}
```

## Lambda

```yaoxiang
double = (x) => x * 2
add = (a, b) => a + b
apply = (list, op) => [op(x) for x in list]
```

## F-string

```yaoxiang
name = "YaoXiang"
println(f"Hello {name}")          # Hello YaoXiang
println(f"Sum: {10 + 20}")        # Sum: 30
println(f"Pi: {pi:.2f}")          # Pi: 3.14
```

## Modules

```yaoxiang
use std.io
use std.math

println("hello")
result = math.sqrt(16)    # 4.0
```

## Ownership

```yaoxiang
# Move: ownership transfer by default
p1 = Point(1.0, 2.0)
p2 = p1                   # p1 is moved

# ref: shared hold
shared = ref data         # Compiler auto-selects Rc/Arc

# clone: explicit deep copy
backup = data.clone()
```

## Concurrency

```yaoxiang
# Functions marked with spawn are async automatically
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# Automatic parallelism, no await needed
user = fetch_user(1)
posts = fetch_posts()
```