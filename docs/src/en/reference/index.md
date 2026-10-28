# YaoXiang Reference Documentation

> YaoXiang is currently in the **experimental validation phase** (for the current version, see
> [Language Specification Overview](language-spec/index.md)), and the standard library and APIs are
> being gradually improved.
>
> The standard library's **authoritative module-by-module reference** is available in the
> [Standard Library Reference](stdlib/index.md); this page serves only as an entry-point index.

## Reference Entry Points

- [Error Code Reference](error-code/index.md) - Overview of diagnostic codes and category lookup
- [Warning Codes](warning-code/warning-codes.md)
- [Package Management](package/index.md) - Manifests, lockfiles, and commands
- [Tool Commands](check-command.md) / [format](format-command.md) / [test](test-command.md)

## Language Specification

- [Language Specification Overview](language-spec/index.md)
- [Syntax Specification](language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](language-spec/type-system.md) - Basic types, composite types, generics, traits
- [Module System](language-spec/modules.md) - Module definitions, imports and exports, scopes
- [Concurrency Model](language-spec/concurrency.md) - Asynchronous programming, concurrency
  primitives, memory model
- [FFI](language-spec/ffi.md) - Foreign function interface
- [Standard Library](language-spec/stdlib.md) - Standard library overview

## Current Status

| Module           | Status         | Description                        | Reference                          |
| ---------------- | -------------- | ---------------------------------- | ---------------------------------- |
| `std.io`         | ✅ Implemented | Input/output                       | [io](stdlib/io.md)                 |
| `std.string`     | ✅ Implemented | String operations                  | [string](stdlib/string.md)         |
| `std.list`       | ✅ Implemented | List operations                    | [list](stdlib/list.md)             |
| `std.dict`       | ✅ Implemented | Dictionary operations              | [dict](stdlib/dict.md)             |
| `std.range`      | ✅ Implemented | Ranges and iterators (#302)        | [range](stdlib/range.md)           |
| `std.math`       | ✅ Implemented | Mathematical functions             | [math](stdlib/math.md)             |
| `std.net`        | ✅ Implemented | Network operations (ureq + rustls) | [net](stdlib/net.md)               |
| `std.concurrent` | ✅ Implemented | Concurrency primitives             | [concurrent](stdlib/concurrent.md) |
| `std.os`         | ✅ Implemented | Operating system interface         | [os](stdlib/os.md)                 |
| `std.fs`         | ✅ Implemented | File system                        | [fs](stdlib/fs.md)                 |
| `std.time`       | ✅ Implemented | Time and dates                     | [time](stdlib/time.md)             |
| `std.convert`    | ✅ Implemented | Type conversion                    | [convert](stdlib/convert.md)       |
| `std.result`     | ✅ Implemented | Result type                        | [result](stdlib/result.md)         |
| `std.assert`     | ✅ Implemented | Assertions                         | [assert](stdlib/assert.md)         |
| `std.weak`       | ✅ Implemented | Weak references                    | [weak](stdlib/weak.md)             |

> `std.net` was initially a “placeholder implementation that made no requests,” but has now been
> changed to a real implementation (#56).

## Built-in Types

### Primitive Types

| Type      | Description                         | Example         |
| --------- | ----------------------------------- | --------------- |
| `Never`   | Zero constructors (divergent)       | —               |
| `Void`    | Void/no return value                | `()`            |
| `Bool`    | Boolean value                       | `true`, `false` |
| `Int`     | Integer (= `Int64`)                 | `42`, `-10`     |
| `Int8`    | 8-bit integer                       | `1`             |
| `Int16`   | 16-bit integer                      | `1`             |
| `Int32`   | 32-bit integer                      | `1`             |
| `Int64`   | 64-bit integer                      | `1`             |
| `Float`   | Floating-point number (= `Float64`) | `3.14`, `-0.5`  |
| `Float32` | 32-bit floating-point number        | `3.14`          |
| `Float64` | 64-bit floating-point number        | `3.14`          |
| `Char`    | Character                           | `'a'`, `'中'`   |
| `String`  | String                              | `"hello"`       |
| `Bytes`   | Raw bytes                           | —               |

> These are all 15 built-in type names (`()` is the type syntax for `Void`). For the authoritative
> table and spelling rules, see [Language Specification §2.1](language-spec/type-system.md): **only
> the capitalized spelling is valid**, and spellings such as `int` / `i64` / `str` / `bool` were
> removed in WBS 6.1.2. `DateTime` is not a built-in name—it is a temporary compatibility alias in
> the std layer (the `now` / `parse_time` signatures of `std.time`); its migration is covered in P10
> S11 (issue #513).

### Composite Types

| Type                 | Description                 | Example        |
| -------------------- | --------------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Heterogeneous-element tuple | `(1, "hello")` |
| `(Args) -> Ret`      | Function type               | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Vec(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in
> primitives; they are generic type constructors and receive the same treatment as user-defined
> generics, handled through a unified generic instantiation path. Literal syntax (`[...]` / `{...}`)
> is retained in the core, with its target determined by contextual annotations. Set has been
> removed (#300); see the [Language Specification](language-spec/syntax.md) for details.
>
> The three container concepts are distinguished by where their length information resides:
> `Array(T, N)` stores its length in the type (fixed-length), `Vec(T)` has a runtime length value (a
> raw buffer primitive), and `List(T)` is a standard library type (`{ data: Vec(T), length: Int }`),
> with all strategy handled by the library.

### User-Defined Types

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
print(value)           // Print without a newline
println(value)         // Print with a newline
```

### Conversion

```yaoxiang
to_string(value)       // Convert to a string
to_int(value)          // Convert to an integer
to_float(value)        // Convert to a floating-point number
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
| `spawn {}`                | Spawn block           |
| `if` / `else if` / `else` | Conditional branches  |
| `match`                   | Pattern matching      |
| `while` / `for`           | Loops                 |
| `return`                  | Return value          |
| `ref`                     | Create a reference    |
| `mut`                     | Mutable marker        |

## Syntax Quick Reference

### Variable Declarations

```yaoxiang
// Immutable variable (default)
x: Int = 42
y = 42                 // Type inference

// Mutable variable
mut count: Int = 0
count = count + 1
```

### Function Definitions

```yaoxiang
// Regular function
add: (a: Int, b: Int) -> Int = a + b

// Spawn function (automatically concurrent)
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// Generic function
identity: [T](x: T) -> T = x
```

### Control Flow

```yaoxiang
// Condition
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
// The ? operator propagates errors
data = fetch_file(path)?
```

## Operator Precedence

| Precedence | Operator                                                                                                                                                                                            |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Highest    | `( )` Function call                                                                                                                                                                                 |
|            | `.` Field access                                                                                                                                                                                    |
|            | `[ ]` Index                                                                                                                                                                                         |
|            | `unary -` Unary minus                                                                                                                                                                               |
|            | `* / %` Multiplication, division, and modulo                                                                                                                                                        |
|            | (Operator precedence and associativity are fixed by the language; the semantics of `+ - * / %`, `== !=`, and `[]` may be overloaded by types that implement the corresponding interface (RFC-011b)) |
|            | `+ -` Addition and subtraction                                                                                                                                                                      |
|            | `== != < > <= >=` Comparison                                                                                                                                                                        |
|            | `and or` Logical operations                                                                                                                                                                         |
| Lowest     | `=` Assignment                                                                                                                                                                                      |

## Standard Library Usage Examples

```yaoxiang
// Import the standard library
use std.io.{print, println}

// List operations
use std.list.{list_push, list_pop, list_len}

// Math functions
use std.math.{sqrt, sin, cos, PI}

// Usage
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## Command-Line Tools

```bash
# Run a script
yx run hello.yx

# Build bytecode
yx build hello.yx -o hello.42

# Execute with the interpreter
yx eval 'println("Hello")'

# View help
yaoxiang --help
```

## Complete Example

```yaoxiang
use std.convert
use std.io

// Compute the Fibonacci sequence
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// Main function
main: () -> Void = {
    io.println("Fibonacci(10) = " + convert.to_string(fib(10)))
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn YaoXiang
- [Design Documents](../explanation/) - Language design decisions
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## Contribution Guide

The standard library is under construction, and contributions are welcome!

1. Choose a module (such as `std.io`, `std.net`)
2. Implement functions in `src/std/`
3. Add documentation comments
4. Submit a PR
