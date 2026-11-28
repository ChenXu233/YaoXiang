# YaoXiang Quick Start

> This guide helps you get started quickly with the YaoXiang programming language.
>
> **Note**: The code examples in this document are written according to the YaoXiang language
> specification. If you encounter syntax differences when running them in practice, see the
> [Language Specification](../reference/language-spec/index.md).

## Installation

### Building from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# Build (debug version, for development and testing)
cargo build

# Build (release version, recommended for production)
cargo build --release

# Run the tests
cargo test

# View the version
./target/debug/yaoxiang-rs --version
# Or
./target/release/yaoxiang-rs --version
```

**Verify the installation**:

```bash
./target/debug/yaoxiang-rs --version
# Should output something like: yaoxiang-rs 0.8.2
```

## Your First Program

Create a file named `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Function definition: name: (param: Type, ...) -> return_type = { return ... }  # Code blocks must explicitly return
// Expression form: name: (param: Type, ...) -> return_type = expr           # The expression returns the value directly
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang-rs run hello.yx
# Or use the release version
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
x = 42  // Inferred as Int
name = "YaoXiang"  // Inferred as String
pi = 3.14159  // Inferred as Float
is_valid = true  // Inferred as Bool

// Explicit type annotations (a centralized type convention is recommended)
count: Int = 100

// Immutable by default (a safety feature)
x = 10
x = 20  // ❌ Compile error! Immutable

// Mutable variables (an explicit declaration is required)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Functions

```yaoxiang
// Function definition syntax
// Expression form: directly returns a value; no return needed
add: (a: Int, b: Int) -> Int = a + b

// Block form: return is required to return a value
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

// Type definition (using braces)
Point: Type = { x: Float, y: Float }

// Use the type
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record Types

<!-- docs-example: skip -->

```yaoxiang
// Struct type
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Use
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Interface Definitions

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

Define type methods using the `Type.method: (Type, ...) -> Return = ...` syntax:

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

// Method call (syntax sugar)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### No Implicit Binding

Methods are not automatically bound to types, and the `pub` keyword has also been removed
(RFC-029g)—to add a method, explicitly write it using the `Type.method: (self: T, …) -> R = …`
syntax from the previous section, and use `.` at the call site.

#### Enum Types

```yaoxiang
// Simple enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum with data
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Use generics—the variant constructor must be qualified as "type.variant"
// Writing ok(42) / err("not found") without qualification raises E1001
success: Result(Int, String) = Result(Int, String).ok(42)
failure: Result(Int, String) = Result(Int, String).err("not found")
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

// Add an element: lists have no instance methods; call the function form from std.list
// Note: do not name a variable list—that would shadow the module
mut items = [1, 2, 3]
items = list.push(items, 4)  // [1, 2, 3, 4]
```

### Pattern Matching

```yaoxiang
// match expression
// Note: the Result constructor requires use std.result first; writing ok(42) without qualification raises E1001
use std.result
result: Result(Int, String) = Result(Int, String).ok(42)

message = match result {
    ok(value) -> "Success: " + value.to_string()
    err(error) -> "Error: " + error
}
```

## Spawn Programming (Concurrency)

YaoXiang's concurrency model is built around the `spawn <expr>` primitive—it is the only entry point
for parallelism.

<!-- docs-example: skip -->

```yaoxiang
// spawn decorates any expression and executes it in parallel automatically
main: () -> Void = {
    user = spawn fetch_user(1)   // Runs in the background
    posts = spawn fetch_posts()  // Another parallel step

    // Automatically blocks and waits when a result is needed
    print(user.name)
    print(posts.length)
}
```

**Core rule**: An expression decorated by `spawn` runs in the background, while the outer expression
synchronously blocks and waits for the result. Tasks without dependencies run in parallel
automatically and are scheduled by the runtime's GMP model.

## Module System

```yaoxiang
// Import the standard library
use std.io
use std.math

// Use an imported function
result = math.sqrt(16.0)  // 4.0
print("Hello!")
```

## Frequently Asked Questions

### Q: How do I modify a variable when variables are immutable by default?

```yaoxiang
// Declare a mutable variable with the mut keyword
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
// Record and enum type variants are written as "field name: (payload) -> type"
// Note: | cannot be used in a type body—parsing { ok(T) | err(E) } raises E0010
// In projects, use the Result type built into the standard library
use std.result

r = Result(Int, String).ok(42)

// Handle with pattern matching
match r {
    ok(value) -> print("Success: {value}")
    err(e) -> print("Error: " + e)
}
```

## Next Steps

- 📚 Read the [Language Specification](../reference/language-spec/index.md) to learn the complete
  syntax
- 🏗️ Browse the [Design Documents](../explanation/) for implementation details
- 💡 Read the [Design Manifesto](../explanation/manifesto.md) to learn the core principles

## Related Resources

- [GitHub Repository](https://github.com/ChenXu233/YaoXiang)
- [Issue Tracker](https://github.com/ChenXu233/YaoXiang/issues)
- [Contributing Guide](../dev/contributing.md)
