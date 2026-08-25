# YaoXiang Reference Documentation

> This document is under construction...

YaoXiang is currently in the **experimental validation stage**, with the standard library and API
being gradually improved.

## Language Specification

- [Language Specification Overview](./language-spec/index.md)
- [Syntax Specification](./language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](./language-spec/type-system.md) - Basic types, composite types, generics, trait
- [Module System](./language-spec/modules.md) - Module definition, import/export, scope
- [Concurrency Model](./language-spec/concurrency.md) - Asynchronous programming, concurrency
  primitives, memory model
- [Standard Library](./language-spec/stdlib.md) - Core library, IO library, math library

## Current Status

| Module           | Status         | Description            |
| ---------------- | -------------- | ---------------------- |
| `std.io`         | 🔨 In progress | Input/Output           |
| `std.string`     | 🔨 In progress | String operations      |
| `std.list`       | 🔨 In progress | List operations        |
| `std.dict`       | ✅ Implemented | Dictionary operations  |
| `std.math`       | 🔨 In progress | Math functions         |
| `std.net`        | 📋 Planned     | Network operations     |
| `std.concurrent` | 📋 Planned     | Concurrency primitives |

## Built-in Types

### Primitive Types

| Type     | Description            | Example         |
| -------- | ---------------------- | --------------- |
| `Void`   | Void / no return value | `()`            |
| `Bool`   | Boolean value          | `true`, `false` |
| `Int`    | Integer                | `42`, `-10`     |
| `Float`  | Floating-point number  | `3.14`, `-0.5`  |
| `Char`   | Character              | `'a'`, `'中'`   |
| `String` | String                 | `"hello"`       |

### Composite Types

| Type                 | Description         | Example        |
| -------------------- | ------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Heterogeneous tuple | `(1, "hello")` |
| `(Args) -> Ret`      | Function type       | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Array(T, N)` / `Dict(K, V)` / `Set(T)`) are NOT built-in
> primitives—they are generic type constructors, treated the same as user-defined generics, and
> handled through the unified generic instantiation path. Literal syntax (`[...]` / `{...}`) is
> retained in the core, with the resolution target determined by context annotations. See the
> [Language Specification](language-spec/syntax.md) for details.

### User-defined Types

```yaoxiang
// Record type (struct)
Point: Type = { x: Float, y: Float }

// Enum type
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Interface type (all fields are functions)
Callable: Type = { call: (String) -> Void }
```

## Built-in Functions

### Output

```yaoxiang
print(value)           // Print, no newline
println(value)         // Print, with newline
```

### Conversion

```yaoxiang
to_string(value)       // Convert to string
to_int(value)          // Convert to integer
to_float(value)        // Convert to float
```

### Type Checking

```yaoxiang
typeof(value)         // Return the type name
is_type(value, type)  // Check the type
```

## Keywords

| Keyword                   | Description           |
| ------------------------- | --------------------- |
| `Type`                    | Meta type             |
| `spawn`                   | Mark a spawn function |
| `spawn for`               | Parallel loop         |
| `spawn {}`                | spawn block           |
| `if` / `else if` / `else` | Conditional branches  |
| `match`                   | Pattern matching      |
| `while` / `for`           | Loops                 |
| `return`                  | Return a value        |
| `ref`                     | Create a reference    |
| `mut`                     | Mutability marker     |

## Syntax Cheatsheet

### Variable Declaration

```yaoxiang
// Immutable variable (default)
x: Int = 42
y = 42                 // Type inference

// Mutable variable
mut count: Int = 0
count = count + 1
```

### Function Definition

```yaoxiang
// Ordinary function
add: (a: Int, b: Int) -> Int = a + b

// spawn function (automatically concurrent)
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// Generic function
identity: [T](x: T) -> T = x
```

### Control Flow

```yaoxiang
// Conditional
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// Pattern matching
match result {
    ok(value) => print("success: " + value),
    err(error) => print("error: " + error),
}

// Loop
for i in 0..10 {
    print(i)
}
```

### Error Handling

```yaoxiang
// ? operator propagates errors
data = fetch_file(path)?
```

## Operator Precedence

| Precedence | Operator                       |
| ---------- | ------------------------------ |
| Highest    | `( )` function call            |
|            | `.` field access               |
|            | `[ ]` index                    |
|            | `unary -` unary minus          |
|            | `* / %` multiply/divide/modulo |
|            | `+ -` add/subtract             |
|            | `== != < > <= >=` comparison   |
|            | `and or` logical operations    |
| Lowest     | `=` assignment                 |

## Standard Library Usage Examples

```yaoxiang
// Import standard library
use std.io.{print, println}

// List operations
use std.list.{list_push, list_pop, list_len}

// Math functions
use std.math.{sqrt, sin, cos, PI}

// Usage
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## Command Line Tool

```bash
# Run a script
yaoxiang run hello.yx

# Build bytecode
yaoxiang build hello.yx -o hello.42

# Interpret execution
yaoxiang eval 'println("Hello")'

# View help
yaoxiang --help
```

## Complete Example

```yaoxiang
// Compute the Fibonacci sequence
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// Main function
main: () -> Void = {
    print("Fibonacci(10) = " + fib(10).to_string())
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn YaoXiang
- [Design Documents](../design/) - Language design decisions
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## Contribution Guide

The standard library is under construction—contributions are welcome!

1. Choose a module (e.g., `std.io`, `std.net`)
2. Implement the functions in `src/std/`
3. Add documentation comments
4. Submit a PR
