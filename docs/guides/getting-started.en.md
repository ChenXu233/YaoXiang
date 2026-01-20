# YaoXiang Quick Start

> This guide helps you get started with the YaoXiang programming language quickly.
>
> **Note**: Code examples in this document are based on the YaoXiang language specification. If you encounter syntax differences during actual execution, please refer to the [Language Specification](../YaoXiang-book.md).

[English](getting-started.en.md) | [中文](getting-started.md)

## Installation

### Building from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Build (debug version, for development testing)
cargo build

# Build (release version, recommended for production)
cargo build --release

# Run tests
cargo test

# Check version
./target/debug/yaoxiang --version
# or
./target/release/yaoxiang --version
```

**Verify successful installation**:
```bash
./target/debug/yaoxiang --version
# Should output: yaoxiang x.y.z
```

## Your First Program

Create a file `hello.yx`:

```yaoxiang
# hello.yx
use std.io

# Function definition: name: (types) -> return_type = (params) => body
main: () -> Void = () => {
    println("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang hello.yx
# or use release version
./target/release/yaoxiang hello.yx
```

Output:

```
Hello, YaoXiang!
```

## Basic Concepts

### Variables and Types

```yaoxiang
# Automatic type inference
x = 42                    # inferred as Int
name = "YaoXiang"         # inferred as String
pi = 3.14159              # inferred as Float
is_valid = true           # inferred as Bool

# Explicit type annotation (recommended using type centralization convention)
count: Int = 100

# Default immutable (safety feature)
x = 10
x = 20                    # ❌ Compile error! Immutable

# Mutable variables (requires explicit declaration)
mut counter = 0
counter = counter + 1     # ✅ OK
```

### Functions

```yaoxiang
# Function definition syntax
add: (Int, Int) -> Int = (a, b) => a + b

# Call
result = add(1, 2)        # result = 3

# Single parameter function
inc: Int -> Int = x => x + 1
```

### Type Definitions

YaoXiang uses a unified `name: type = value` syntax model:

```yaoxiang
# Variable declaration
x: Int = 42
name: String = "YaoXiang"

# Function definition
add: (Int, Int) -> Int = (a, b) => a + b

# Type definition (using braces)
type Point = { x: Float, y: Float }

# Using the type
p: Point = Point(x: 1.0, y: 2.0)
p.x  # 1.0
p.y  # 2.0
```

#### Record Types

```yaoxiang
# Struct types
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# Usage
p = Point(x: 3.0, y: 4.0)
r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
```

#### Interface Definitions

Interfaces are record types where all fields are function types:

```yaoxiang
# Define interfaces
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# Empty interface
type EmptyInterface = {}
```

#### Type Methods

Use `Type.method: (Type, ...) -> Return = ...` syntax to define type methods:

```yaoxiang
# Type definition
type Point = { x: Float, y: Float }

# Type method definitions
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    "Point(${self.x}, ${self.y})"
}

# Using methods (syntactic sugar)
p = Point(x: 1.0, y: 2.0)
p.draw(screen)           # → Point.draw(p, screen)
str = p.serialize()      # → Point.serialize(p)
```

#### Auto-Binding

Functions declared with `pub` are automatically bound to types defined in the same file:

```yaoxiang
type Point = { x: Float, y: Float }

# pub declarations auto-bind to Point
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Usage
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

# Function-style call
d = distance(p1, p2)           # 3.606...

# OOP syntactic sugar (auto-bound to Point.distance)
d2 = p1.distance(p2)           # → distance(p1, p2)
```

#### Enum Types

```yaoxiang
# Simple enums
type Color = red | green | blue

# Enums with data
type Result[T, E] = ok(T) | err(E)

# Using generics
success: Result[Int, String] = ok(42)
failure: Result[Int, String] = err("not found")
```

#### Generic Types

```yaoxiang
# Generic type definitions
type List[T] = {
    data: Array[T],
    length: Int,
    push: (List[T], T) -> Void
}

# Concrete instantiations
type IntList = List(Int)
type StringList = List(String)
```

### Control Flow

```yaoxiang
# Conditional expression
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# Loop
for i in 0..5 {
    print(i)
}

# while loop
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Lists and Dictionaries

```yaoxiang
# List
numbers = [1, 2, 3, 4, 5]
first = numbers[0]         # 1

# Dictionary
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  # 90

# Add elements
mut list = [1, 2, 3]
list.append(4)
```

### Pattern Matching

```yaoxiang
# match expression
result: Result[Int, String] = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Concurrent Programming (Async)

YaoXiang's unique feature: functions marked with `spawn` automatically gain async capabilities.

```yaoxiang
# Define concurrent function (auto-async execution)
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

# Call concurrent function (auto-parallel, no await needed)
main: () -> Void = () => {
    # Two calls execute automatically in parallel
    user = fetch_user(1)     # Auto-parallel
    posts = fetch_posts()    # Auto-parallel

    # Auto-wait when results are needed
    print(user.name)
    print(posts.length)
}
```

## Module System

```yaoxiang
# Import standard library
use std.io
use std.math

# Use imported functions
result = math.sqrt(16)      # 4.0
println("Hello!")
```

## FAQ

### Q: Variables are immutable by default, how do I modify them?

```yaoxiang
# Use mut keyword to declare mutable variables
mut x = 10
x = 20                       # ✅ OK
```

### Q: How do I define functions?

```yaoxiang
# Full form (recommended)
add: (Int, Int) -> Int = (a, b) => a + b

# Short form (type inference)
add = (a, b) => a + b
```

### Q: How do I handle errors?

```yaoxiang
# Use Result type
type Result[T, E] = ok(T) | err(E)

# Pattern matching for handling
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Next Steps

- 📖 Read [YaoXiang Guide](../YaoXiang-book.md) to understand core features
- 📚 Check [Language Specification](../YaoXiang-book.md) for complete syntax
- 🏗️ Browse [Architecture Documentation](../architecture/) for implementation details
- 💡 Check [Design Manifesto](../design/manifesto.md) for core philosophy

## Related Resources

- [GitHub Repository](https://github.com/yourusername/yaoxiang)
- [Issue Reporting](https://github.com/yourusername/yaoxiang/issues)
- [Contribution Guide](../guides/dev/)
