# YaoXiang Quick Start

> This guide helps you get started with YaoXiang programming language.
>
> **Note**: Code examples in this document are written based on YaoXiang language specification. If
> you encounter syntax differences in actual execution, please refer to
> [Language Specification](../reference/language-spec/index.md).

## Installation

### Build from Source (Recommended)

```bash
# Clone repository
git clone https://github.com/ChenXu233/YaoXiang.git
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
# Should output something like: yaoxiang x.y.z
```

## Your First Program

Create file `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Function definition: name: (param: Type, ...) -> return_type = { return ... }  # code block must explicitly return
// Expression form: name: (param: Type, ...) -> return_type = expr           # expression returns value directly
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run:

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
// Automatic type inference
x = 42  // inferred as Int
name = "YaoXiang"  // inferred as String
pi = 3.14159  // inferred as Float
is_valid = true  // inferred as Bool

// Explicit type annotation (recommended to use type-centric convention)
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
// Expression form: return value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b

// Code block form: must use return to return value
// add: (a: Int, b: Int) -> Int = { return a + b }

// Call
result = add(1, 2)  // result = 3

// Single parameter function (expression form)
inc: (x: Int) -> Int = x + 1
```

### Type Definitions

YaoXiang uses unified `name: type = value` syntax model:

```yaoxiang
// Variable declaration
x: Int = 42
name: String = "YaoXiang"

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Type definition (using braces)
Point: Type = { x: Float, y: Float }

// Use type
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record Types

```yaoxiang
// Struct type
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Usage
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface Definitions

Interfaces are record types with all function fields:

```yaoxiang
// Define interface
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

Use `Type.method: (Type, ...) -> Return = ...` syntax to define type methods:

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

// Use methods (syntactic sugar)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Automatic Binding

Functions declared with `pub` keyword automatically bind to types defined in the same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub declaration automatically binds to Point
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

// OOP syntactic sugar (auto-binds to Point.distance)
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Enum Types

```yaoxiang
// Simple enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum with data
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

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
} else if x == 0 {
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

// Add elements
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

YaoXiang's concurrency model builds around the `spawn <expr>` primitive — the sole entry point for
parallelism.

```yaoxiang
// spawn can decorate any expression, executed in parallel automatically
main: () -> Void = {
    user = spawn fetch_user(1)   // executes in background
    posts = spawn fetch_posts()  // another parallel step

    // automatically blocks waiting for result when needed
    print(user.name)
    print(posts.length)
}
```

**Core rule**: Expressions decorated with `spawn` execute in background, and the outer scope
synchronously blocks waiting for results. Independent tasks automatically run in parallel, scheduled
by the runtime's GMP model.

## Module System

```yaoxiang
// Import standard library
use std.io
use std.math

// Use imported functions
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## FAQ

### Q: Variables are immutable by default, how do I modify a variable?

```yaoxiang
// Use mut keyword to declare mutable variable
mut x = 10
x = 20  // ✅ OK
```

### Q: How do I define a function?

```yaoxiang
// Full form (recommended)
add: (a: Int, b: Int) -> Int = a + b

// Short form (type inference)
add = (a, b) => a + b
```

### Q: How do I handle errors?

```yaoxiang
// Use Result type
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Handle with pattern matching
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Next Steps

- 📚 Check out [Language Specification](../YaoXiang-language-specification.md) for complete syntax
- 🏗️ Browse [Architecture Documentation](../architecture/) for implementation details
- 💡 Read [Design Manifesto](../YaoXiang-design-manifesto.md) for core philosophy

## Related Resources

- [GitHub Repository](https://github.com/yourusername/yaoxiang)
- [Issue Tracker](https://github.com/yourusername/yaoxiang/issues)
- [Contribution Guide](../guides/dev/)
