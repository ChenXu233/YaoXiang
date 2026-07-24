# YaoXiang Quick Start

> This guide helps you get up to speed quickly with the YaoXiang programming language.
>
> **Note**: The code examples in this document are written based on the YaoXiang language specification. If you encounter syntax differences during actual execution, please refer to the [Language Specification](../reference/language-spec/index.md).

## Installation

### Build from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# Build (debug version, for development and testing)
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

**Verify Successful Installation**:
```bash
./target/debug/yaoxiang --version
# Should output something like: yaoxiang x.y.z
```

## Your First Program

Create the file `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Function definition: name: (param: Type, ...) -> return_type = { return ... }  # Code block must explicitly return
// Expression form: name: (param: Type, ...) -> return_type = expr           # Expression directly returns the value
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang hello.yx
# or use the release version
./target/release/yaoxiang hello.yx
```

Output:

```
Hello, YaoXiang!
```

## Basic Concepts

### Variables and Types

```yaoxiang
// Automatic type inference
x = 42  // inferred as Int
name = "YaoXiang"  // inferred as String
pi = 3.14159  // inferred as Float
is_valid = true  // inferred as Bool

// Explicit type annotation (recommended for centralized type conventions)
count: Int = 100

// Immutable by default (safety feature)
x = 10
x = 20  // ❌ Compile error! Immutable

// Mutable variable (requires explicit declaration)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Functions

```yaoxiang
// Function definition syntax
// Expression form: directly returns the value, no `return` needed
add: (a: Int, b: Int) -> Int = a + b

// Code block form: must use `return` to return the value
// add: (a: Int, b: Int) -> Int = { return a + b }

// Call
result = add(1, 2)  // result = 3

// Single-parameter function (expression form)
inc: (x: Int) -> Int = x + 1
```

### Type Definitions

YaoXiang uses a unified `name: type = value` syntax model:

```yaoxiang
// Variable declaration
x: Int = 42
name: String = "YaoXiang"

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Type definition (using curly braces)
Point: Type = { x: Float, y: Float }

// Using the type
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record Types

```yaoxiang
// Struct types
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Usage
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface Definitions

Interfaces are record types whose fields are all function types:

```yaoxiang
// Define an interface
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Empty interface
EmptyInterface: Type = {}
```

#### Type Methods

Use the `Type.method: (Type, ...) -> Return = ...` syntax to define type methods:

```yaoxiang
// Type definition
Point: Type = { x: Float, y: Float }

// Type method definition
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// Using methods (syntactic sugar)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Automatic Binding

Functions declared with the `pub` keyword are automatically bound to types defined in the same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

// `pub` declaration automatically binds to Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Usage
p1 = Point(x=3.0, y=4.0)
p2 = Point(x=1.0, y=2.0)

// Functional call
d = distance(p1, p2)  // 3.606...

// OOP syntactic sugar (automatically bound to Point.distance)
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Enum Types

```yaoxiang
// Simple enum
Color: Type = { red | green | blue }

// Enum with associated data
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Using generics
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Generic Types

```yaoxiang
// Generic type definition
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// Concrete instantiation
IntList: Type = List(Int)
StringList: Type = List(String)
```

### Control Flow

```yaoxiang
// Conditional expression
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

// Loop
for i in 0..5 {
    print(i)
}

// while loop
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Lists and Dictionaries

```yaoxiang
// Lists
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// Dictionaries
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// Adding elements
mut list = [1, 2, 3]
list.append(4)
```

### Pattern Matching

```yaoxiang
// match expression
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Spawn Programming (Concurrency)

YaoXiang's concurrency model is built around the `spawn <expr>` primitive — it is the only entry point for parallelism.

```yaoxiang
// `spawn` modifies any expression, executing it in parallel automatically
main: () -> Void = {
    user = spawn fetch_user(1)   // runs in the background
    posts = spawn fetch_posts()  // another parallel step

    // Automatically blocks to wait for the result when needed
    print(user.name)
    print(posts.length)
}
```

**Core Rule**: An expression modified by `spawn` executes in the background, while the outer synchronous code blocks to wait for the result. Tasks with no dependencies run in parallel automatically, scheduled by the runtime's GMP model.

## Module System

```yaoxiang
// Import the standard library
use std.io
use std.math

// Use imported functions
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## Frequently Asked Questions

### Q: Variables are immutable by default — how do I modify one?

```yaoxiang
// Use the `mut` keyword to declare a mutable variable
mut x = 10
x = 20  // ✅ OK
```

### Q: How do I define a function?

```yaoxiang
// Full form (recommended)
add: (a: Int, b: Int) -> Int = a + b

// Short form (with type inference)
add = (a, b) => a + b
```

### Q: How do I handle errors?

```yaoxiang
// Use the Result type
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Handle with pattern matching
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Next Steps

- 📚 Read the [Language Specification](../YaoXiang-language-specification.md) for the full syntax
- 🏗️ Browse the [Architecture Documentation](../architecture/) for implementation details
- 💡 Check the [Design Manifesto](../YaoXiang-design-manifesto.md) for the core philosophy

## Related Resources

- [GitHub Repository](https://github.com/yourusername/yaoxiang)
- [Issue Tracker](https://github.com/yourusername/yaoxiang/issues)
- [Contributing Guide](../guides/dev/)