# YaoXiang Quick Start

> This guide helps you get started with the YaoXiang programming language quickly.
>
> **Note**: The code examples in this document are based on the YaoXiang language specification. If
> you encounter syntax differences when running them, please refer to
> [the language specification](../reference/language-spec/index.md).

## Installation

### Build from source (recommended)

```bash
# Clone the repository
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# Build (debug version, for development testing)
cargo build

# Build (release version, recommended for production)
cargo build --release

# Run tests
cargo test

# Check version
./target/debug/yaoxiang-rs --version
# or
./target/release/yaoxiang-rs --version
```

**Verify the installation succeeded**:

```bash
./target/debug/yaoxiang-rs --version
# Should output something like: yaoxiang-rs 0.8.2
```

## Your first program

Create the file `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Function definition: name: (param: Type, ...) -> return_type = { return ... }  # Code block must explicitly use return
// Expression form: name: (param: Type, ...) -> return_type = expr           # Expression returns the value directly
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run:

```bash
./target/debug/yaoxiang-rs run hello.yx
# Or use the release version
./target/release/yaoxiang-rs run hello.yx
```

Output:

```
Hello, YaoXiang!
```

## Basic concepts

### Variables and types

```yaoxiang
// Automatic type inference
x = 42  // Inferred as Int
name = "YaoXiang"  // Inferred as String
pi = 3.14159  // Inferred as Float
is_valid = true  // Inferred as Bool

// Explicit type annotation (recommended to use centralized type conventions)
count: Int = 100

// Immutable by default (safety feature)
x = 10
x = 20  // ❌ Compile error! Immutable

// Mutable variables (must be explicitly declared)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Functions

```yaoxiang
// Function definition syntax
// Expression form: returns the value directly, no need for return
add: (a: Int, b: Int) -> Int = a + b

// Code block form: must use return to return a value
// add: (a: Int, b: Int) -> Int = { return a + b }

// Call
result = add(1, 2)  // result = 3

// Single-parameter function (expression form)
inc: (x: Int) -> Int = x + 1
```

### Type definitions

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

#### Record types

<!-- docs-example: skip -->

```yaoxiang
// Struct types
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Usage
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface definitions

An interface is a record type whose fields are all function types:

<!-- docs-example: skip -->

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

#### Type methods

Use the `Type.method: (Type, ...) -> Return = ...` syntax to define type methods:

<!-- docs-example: skip -->

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

#### Automatic binding

Functions declared with the `pub` keyword are automatically bound to types defined in the same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub declarations are automatically bound to Point
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

#### Enum types

```yaoxiang
// Simple enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum with data
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Using generics — variant constructors must be qualified with "Type.variant"
// Writing bare ok(42) / err("not found") reports E1001
success: Result(Int, String) = Result(Int, String).ok(42)
failure: Result(Int, String) = Result(Int, String).err("not found")
```

#### Generic types

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

### Control flow

```yaoxiang
// Conditional expression
x = 42

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

### Lists and dictionaries

```yaoxiang
use std.list

// List
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// Dictionary
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// Adding elements: lists have no instance methods, call std.list as functions
// Note: do not name a variable `list` — that would shadow the module
mut items = [1, 2, 3]
items = list.push(items, 4)  // [1, 2, 3, 4]
```

### Pattern matching

```yaoxiang
// match expression
// Note: Result constructors need `use std.result` first; writing bare `ok(42)` reports E1001
use std.result
result: Result(Int, String) = Result(Int, String).ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Spawn programming (concurrency)

YaoXiang's concurrency model is built around the `spawn <expr>` primitive — it is the only entry
point for parallelism.

<!-- docs-example: skip -->

```yaoxiang
// spawn modifies any expression, executing it in parallel automatically
main: () -> Void = {
    user = spawn fetch_user(1)   // runs in the background
    posts = spawn fetch_posts()  // another parallel step

    // automatically blocks and waits when the result is needed
    print(user.name)
    print(posts.length)
}
```

**Core rule**: an expression modified by `spawn` runs in the background, while the outer scope
synchronously blocks and waits for the result. Tasks with no dependencies run in parallel
automatically, scheduled by the runtime GMP model.

## Module system

```yaoxiang
// Import the standard library
use std.io
use std.math

// Use the imported functions
result = math.sqrt(16.0)  // 4.0
print("Hello!")
```

## FAQ

### Q: Variables are immutable by default. How do I change a variable?

```yaoxiang
// Use the mut keyword to declare a mutable variable
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
// Record-style and enum variants are written as "field_name: (payload) -> type"
// Note: `|` cannot be used inside type bodies — `{ ok(T) | err(E) }` fails to parse with E0010
// In real projects, use the built-in Result from the standard library directly
use std.result

r = Result(Int, String).ok(42)

// Handle with pattern matching
match r {
    ok(value) => print("Success: {value}")
    err(e) => print("Error: " + e)
}
```

## Next steps

- 📚 See the [language specification](../reference/language-spec/index.md) for the complete syntax
- 🏗️ Browse the [design documents](../design/) for implementation details
- 💡 Read the [design manifesto](../design/manifesto.md) for the core philosophy

## Related resources

- [GitHub repository](https://github.com/ChenXu233/YaoXiang)
- [Issue feedback](https://github.com/ChenXu233/YaoXiang/issues)
- [Contributing guide](../dev/contributing.md)
