# YaoXiang Reference Documentation

> This document is under construction...

YaoXiang is currently in the **experimental validation phase**, and the standard library and APIs are being gradually improved.

## Current Status

| Module | Status | Description |
|--------|--------|-------------|
| `std.io` | 🔨 WIP | Input/Output |
| `std.string` | 🔨 WIP | String operations |
| `std.list` | 🔨 WIP | List operations |
| `std.dict` | 📋 Planned | Dictionary operations |
| `std.math` | 🔨 WIP | Math functions |
| `std.net` | 📋 Planned | Network operations |
| `std.concurrent` | 📋 Planned | Concurrency primitives |

## Built-in Types

### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `Void` | Empty value/no return | `()` |
| `Bool` | Boolean | `true`, `false` |
| `Int` | Integer | `42`, `-10` |
| `Float` | Floating point | `3.14`, `-0.5` |
| `Char` | Character | `'a'`, `'中'` |
| `String` | String | `"hello"` |

### Composite Types

| Type | Description | Example |
|------|-------------|---------|
| `List[T]` | List of same-type elements | `[1, 2, 3]` |
| `Tuple(T1, T2, ...)` | Tuple of different-type elements | `(1, "hello")` |
| `Dict[K, V]` | Key-value mapping | `{"a": 1}` |
| `Fn(Args) -> Ret` | Function type | `(Int) -> Int` |

### User-Defined Types

```yaoxiang
# Record types (structs)
type Point = { x: Float, y: Float }

# Enum types
type Result[T, E] = ok(T) | err(E)

# Interface types (all fields are functions)
type Callable = { call: (String) -> Void }
```

## Built-in Functions

### Output

```yaoxiang
print(value)           # Print, no newline
println(value)         # Print, with newline
```

### Conversion

```yaoxiang
to_string(value)       # Convert to string
to_int(value)         # Convert to integer
to_float(value)       # Convert to float
```

### Type Checking

```yaoxiang
typeof(value)         # Returns type name
is_type(value, type)  # Check type
```

## Keywords

| Keyword | Description |
|---------|-------------|
| `type` | Define types |
| `spawn` | Mark bingzuo functions |
| `spawn for` | Parallel loops |
| `spawn {}` | Bingzuo blocks |
| `if` / `elif` / `else` | Conditional branches |
| `match` | Pattern matching |
| `while` / `for` | Loops |
| `return` | Return value |
| `ref` | Create reference |
| `mut` | Mutability marker |

## Syntax Quick Reference

### Variable Declarations

```yaoxiang
# Immutable variables (default)
x: Int = 42
y = 42                 # Type inference

# Mutable variables
mut count: Int = 0
count = count + 1
```

### Function Definitions

```yaoxiang
# Regular functions
add: (a: Int, b: Int) -> Int = a + b

# Bingzuo functions (automatic concurrency)
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

# Generic functions
identity: [T](x: T) -> T = x
```

### Control Flow

```yaoxiang
# Conditionals
if x > 0 {
    println("positive")
} elif x < 0 {
    println("negative")
} else {
    println("zero")
}

# Pattern matching
match result {
    ok(value) => println("success: " + value),
    err(error) => println("error: " + error),
}

# Loops
for i in 0..10 {
    print(i)
}
```

### Error Handling

```yaoxiang
# ? operator propagates errors
data = fetch_file(path)?
```

## Operator Precedence

| Precedence | Operators |
|------------|-----------|
| Highest | `( )` Function call |
| | `.` Field access |
| | `[ ]` Index |
| | `unary -` Unary minus |
| | `* / %` Multiply/divide/modulo |
| | `+ -` Add/subtract |
| | `== != < > <= >=` Comparison |
| | `and or` Logical operations |
| Lowest | `=` Assignment |

## Standard Library Usage Examples

```yaoxiang
# Import standard library
from std.io import print, println

# List operations
from std.list import list_push, list_pop, list_len

# Math functions
from std.math import sqrt, sin, cos, PI

# Usage
println("Hello, YaoXiang!")
result = sqrt(16.0)  # 4.0
```

## Command Line Tools

```bash
# Run scripts
yaoxiang run hello.yx

# Build bytecode
yaoxiang build hello.yx -o hello.42

# Interpret and execute
yaoxiang eval 'println("Hello")'

# View help
yaoxiang --help
```

## Complete Example

```yaoxiang
# Calculate Fibonacci sequence
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

# Main function
main: () -> Void = {
    println("Fibonacci(10) = " + fib(10).to_string())
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn YaoXiang
- [Design Documents](../design/) - Language design decisions
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## Contributing Guide

The standard library is under construction. Contributions are welcome!

1. Choose a module (e.g., `std.io`, `std.net`)
2. Implement functions in `src/std/`
3. Add documentation comments
4. Submit a PR
