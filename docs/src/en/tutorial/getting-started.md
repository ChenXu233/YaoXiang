# YaoXiang Quick Start

> This guide helps you get started with YaoXiang programming language quickly.
>
> **Note**: Code examples in this document are written based on YaoXiang language specification. If you encounter syntax differences in actual execution, please refer to [Language Specification](../design/language-spec.md).

## Installation

### Building from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Build (debug build, for development and testing)
cargo build

# Build (release build, recommended for production)
cargo build --release

# Run tests
cargo test

# Check version
./target/debug/yaoxiang --version
# or
./target/release/yaoxiang --version
```

**Verify Installation Success**:
```bash
./target/debug/yaoxiang --version
# should output something like: yaoxiang x.y.z
```

## First Program

Create file `hello.yx`:

```yaoxiang
# hello.yx
use std.io

# Function definition: name: (param: Type, ...) -> return_type = { ... }
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

Run:

```bash
./target/debug/yaoxiang hello.yx
# or use release build
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

# Explicit type annotation (recommended to use centralized type conventions)
count: Int = 100

# Immutable by default (safety feature)
x = 10
x = 20                    # ❌ Compile error! Immutable

# Mutable variable (requires explicit declaration)
mut counter = 0
counter = counter + 1     # ✅ OK
```

### Functions

```yaoxiang
# Function definition syntax
add: (a: Int, b: Int) -> Int = a + b

# Call
result = add(1, 2)        # result = 3

# Single-argument function
inc: (x: Int) -> Int = x + 1
```

### Type Definition

YaoXiang uses a unified `name: type = value` syntax model:

```yaoxiang
# Variable declaration
x: Int = 42
name: String = "YaoXiang"

# Function definition
add: (a: Int, b: Int) -> Int = a + b

# Type definition (using curly braces)
type Point = { x: Float, y: Float }

# Using the type
p: Point = Point(x: 1.0, y: 2.0)
p.x  # 1.0
p.y  # 2.0
```

#### Record Types

```yaoxiang
# Struct type
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# Usage
p = Point(x: 3.0, y: 4.0)
r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
```

#### Interface Definition

An interface is a record type where all fields are function types:

```yaoxiang
# Define interface
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

Define type methods using `Type.method: (Type, ...) -> Return = ...` syntax:

```yaoxiang
# Type definition
type Point = { x: Float, y: Float }

# Type method definition
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point(${self.x}, ${self.y})"
}

# Using methods (syntactic sugar)
p = Point(x: 1.0, y: 2.0)
p.draw(screen)           # → Point.draw(p, screen)
str = p.serialize()      # → Point.serialize(p)
```

#### Auto Binding

Functions declared with the `pub` keyword are automatically bound to types defined in the same file:

```yaoxiang
type Point = { x: Float, y: Float }

# pub declaration auto-binding to Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Usage
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

# Functional call
d = distance(p1, p2)           # 3.606...

# OOP syntactic sugar (auto-bound to Point.distance)
d2 = p1.distance(p2)           # → distance(p1, p2)
```

#### Enum Types

```yaoxiang
# Simple enum
type Color = red | green | blue

# Enum with data
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Using generics
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Generic Types

```yaoxiang
# Generic type definition
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

# Concrete instantiation
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

# Add element
mut list = [1, 2, 3]
list.append(4)
```

### Pattern Matching

```yaoxiang
# match expression
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Concurrency Programming (Async)

YaoXiang's unique feature: functions marked with `spawn` automatically gain async capabilities.

```yaoxiang
# Define spawn function (automatically async execution)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# Call spawn function (automatically parallel, no await needed)
main: () -> Void = {
    # Both calls execute in parallel automatically
    user = fetch_user(1)     # auto parallel
    posts = fetch_posts()    # auto parallel

    # Auto wait when results are needed
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

### Q: Variables are immutable by default, how do I modify a variable?

```yaoxiang
# Use mut keyword to declare mutable variable
mut x = 10
x = 20                       # ✅ OK
```

### Q: How do I define a function?

```yaoxiang
# Full form (recommended)
add: (a: Int, b: Int) -> Int = a + b

# Short form (type inference)
add = (a, b) => a + b
```

### Q: How do I handle errors?

```yaoxiang
# Use Result type
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Handle with pattern matching
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Next Steps

- 📖 Read [YaoXiang Guide](../YaoXiang-book.md) to understand core features
- 📚 Check [Language Specification](../YaoXiang-language-specification.md) for complete syntax
- 🏗️ Explore [Architecture Documentation](../architecture/) for implementation details
- 💡 Review [Design Manifesto](../YaoXiang-design-manifesto.md) for core philosophy

## Related Resources

- [GitHub Repository](https://github.com/yourusername/yaoxiang)
- [Issue Feedback](https://github.com/yourusername/yaoxiang/issues)
- [Contribution Guide](../guides/dev/)