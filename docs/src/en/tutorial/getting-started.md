# YaoXiang Quick Start

> This guide helps you get up and running with the YaoXiang programming language quickly.
>
> **Note**: The code examples in this document are written based on the YaoXiang language
> specification. If you encounter syntax differences during actual execution, please refer to the
> [Language Specification](../reference/language-spec/index.md).

## Installation

### Build from Source (Recommended)

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

**Verify Successful Installation**:

```bash
./target/debug/yaoxiang-rs --version
# Should output something like: yaoxiang-rs 0.8.2
```

## Your First Program

Create the file `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Function definition: name: (param: Type, ...) -> return_type = { return ... }  # Code blocks must explicitly use return
// Expression form:      name: (param: Type, ...) -> return_type = expr            # The expression returns its value directly
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang-rs run hello.yx
# or use the release version
./target/release/yaoxiang-rs run hello.yx
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

// Explicit type annotation (using the centralized type convention is recommended)
count: Int = 100

// Immutable by default (safety feature)
x = 10
x = 20  // ❌ Compile error! Immutable.

// Mutable variable (requires explicit declaration)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Functions

```yaoxiang
// Function definition syntax
// Expression form: returns the value directly, no return needed
add: (a: Int, b: Int) -> Int = a + b

// Code block form: must use return to return a value
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

#### Record Type

<!-- docs-example: skip -->

```yaoxiang
// Struct type
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Usage
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface Definition

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

#### Type Methods

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

// Using the method (syntactic sugar)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### No Implicit Binding

Methods are not automatically attached to types, and the `pub` keyword has also been removed
(RFC-029g) — to add a method, write it explicitly using `Type.method: (self: T, …) -> R = …` as in
the previous section, and use `.` at the call site.

#### Enum Type

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

#### Generic Type

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

### Lists and Dictionaries

```yaoxiang
use std.list

// List
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// Dictionary
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// Adding elements: lists have no instance methods, call std.list functions
// Note: don't name a variable `list` — it would shadow the module
mut items = [1, 2, 3]
items = list.push(items, 4)  // [1, 2, 3, 4]
```

### Pattern Matching

```yaoxiang
// match expression
// Note: Result constructors need std.result imported first; bare ok(42) reports E1001
use std.result
result: Result(Int, String) = Result(Int, String).ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Spawn Programming (Concurrency)

YaoXiang's concurrency model is built around the `spawn <expr>` primitive — it is the only entry
point for parallelism.

<!-- docs-example: skip -->

```yaoxiang
// spawn decorates any expression and runs it in parallel automatically
main: () -> Void = {
    user = spawn fetch_user(1)   // runs in the background
    posts = spawn fetch_posts()  // another parallel step

    // When the result is needed, it automatically blocks and waits
    print(user.name)
    print(posts.length)
}
```

**Core rule**: An expression decorated with `spawn` runs in the background, and the outer
synchronous code blocks and waits for its result. Tasks without dependencies run in parallel
automatically, scheduled by the runtime's GMP model.

## Module System

```yaoxiang
// Import the standard library
use std.io
use std.math

// Use imported functions
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
// Record-style enum variants are written as "field: (payload) -> Type"
// Note: `|` cannot be used inside a type body — `{ ok(T) | err(E) }` fails to parse with E0010
// In practice, just use the built-in Result from the standard library
use std.result

r = Result(Int, String).ok(42)

// Handle with pattern matching
match r {
    ok(value) => print("Success: {value}")
    err(e) => print("Error: " + e)
}
```

## Next Steps

- 📚 Check the [Language Specification](../reference/language-spec/index.md) for the complete syntax
- 🏗️ Browse the [Design Documents](../explanation/) for implementation details
- 💡 Read the [Design Manifesto](../explanation/manifesto.md) to learn the core ideas

## Related Resources

- [GitHub Repository](https://github.com/ChenXu233/YaoXiang)
- [Issue Tracker](https://github.com/ChenXu233/YaoXiang/issues)
- [Contributing Guide](../dev/contributing.md)
