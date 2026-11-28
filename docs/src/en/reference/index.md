# YaoXiang Reference Documentation

> YaoXiang is currently at the **experimental validation stage** (for the current version, see
> [Language Specification Overview](language-spec/index.md)). The standard library and APIs are
> being gradually improved.
>
> The standard library's **authoritative module-by-module reference** is available in the
> [Standard Library Reference](stdlib/index.md). This page serves only as an entry index.

## Reference Entry Points

- [Error Code Reference](error-code/index.md) - Overview of diagnostic codes and categorized lookup
- [Warning Codes](warning-code/warning-codes.md)
- [Package Management](package/index.md) - Manifests, lockfiles, and commands
- [Tool Commands](check-command.md) / [format](format-command.md) / [test](test-command.md)

## Language Specification

- [Language Specification Overview](language-spec/index.md)
- [Syntax Specification](language-spec/syntax.md) - Lexical structure, grammar rules, and operator
  precedence
- [Type System](language-spec/type-system.md) - Primitive types, composite types, generics, and
  traits
- [Module System](language-spec/modules.md) - Module definitions, imports and exports, and scopes
- [Concurrency Model](language-spec/concurrency.md) - Asynchronous programming, concurrency
  primitives, and the memory model
- [FFI](language-spec/ffi.md) - Foreign function interface
- [Standard Library](language-spec/stdlib.md) - Standard library overview

## Current Status

| Module           | Status         | Description                        | Reference                          |
| ---------------- | -------------- | ---------------------------------- | ---------------------------------- |
| `std.io`         | ✅ Implemented | Input and output                   | [io](stdlib/io.md)                 |
| `std.string`     | ✅ Implemented | String operations                  | [string](stdlib/string.md)         |
| `std.list`       | ✅ Implemented | List operations                    | [list](stdlib/list.md)             |
| `std.dict`       | ✅ Implemented | Dictionary operations              | [dict](stdlib/dict.md)             |
| `std.range`      | ✅ Implemented | Ranges and iterators (#302)        | [range](stdlib/range.md)           |
| `std.math`       | ✅ Implemented | Mathematical functions             | [math](stdlib/math.md)             |
| `std.net`        | ✅ Implemented | Network operations (ureq + rustls) | [net](stdlib/net.md)               |
| `std.concurrent` | ✅ Implemented | Concurrency primitives             | [concurrent](stdlib/concurrent.md) |
| `std.os`         | ✅ Implemented | Operating system interface         | [os](stdlib/os.md)                 |
| `std.fs`         | ✅ Implemented | File system                        | [fs](stdlib/fs.md)                 |
| `std.time`       | ✅ Implemented | Time and date                      | [time](stdlib/time.md)             |
| `std.convert`    | ✅ Implemented | Type conversion                    | [convert](stdlib/convert.md)       |
| `std.result`     | ✅ Implemented | Result type                        | [result](stdlib/result.md)         |
| `std.assert`     | ✅ Implemented | Assertions                         | [assert](stdlib/assert.md)         |
| `std.weak`       | ✅ Implemented | Weak references                    | [weak](stdlib/weak.md)             |

> In its early stages, `std.net` was a “placeholder implementation that sent no requests.” It has
> now been changed to a real implementation (#56).

## Built-in Types

### Primitive Types

| Type      | Description                         | Example         |
| --------- | ----------------------------------- | --------------- |
| `Never`   | Zero constructors (diverges)        | —               |
| `Void`    | Null value/no return value          | `()`            |
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

> These are all 15 built-in type names (`()` is the type notation for `Void`). See
> [Language Spec §2.1](language-spec/type-system.md) for the authoritative table and spelling rules:
> **only this uppercase spelling is valid**. Aliases such as `int` / `i64` / `str` / `bool` were
> removed in WBS 6.1.2. `DateTime` is not a built-in name—the `now` / `parse_time` functions in
> `std.time` return an `Int` timestamp (the temporary compatibility alias was removed in P10 S11 /
> issue #513: using a bare `DateTime` in an annotation produces an unknown-type error).

### Composite Types

| Type                 | Description                     | Example        |
| -------------------- | ------------------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Tuple of heterogeneous elements | `(1, "hello")` |
| `(Args) -> Ret`      | Function type                   | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Vec(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in
> primitives. They are generic type constructors, treated the same as user-defined generics and
> processed through a unified generic instantiation path. Literal syntax (`[...]` / `{...}`) is
> retained in the core, with its interpretation determined by contextual inference. Set has been
> removed (#300). See the [Language Specification](language-spec/syntax.md).
>
> The three container concepts are distinguished by where their length information resides:
> `Array(T, N)` stores its length in the type (fixed-length), `Vec(T)` stores its length as a
> runtime value (raw buffer primitive), and `List(T)` is a standard library type
> (`{ data: Vec(T), length: Int }`; all strategy resides in the library).

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
print(value)           // Prints without a newline
println(value)         // Prints with a newline
```

### Conversions

```yaoxiang
to_string(value)       // Converts to a string
to_int(value)          // Converts to an integer
to_float(value)        // Converts to a floating-point number
```

### Type Checking

```yaoxiang
typeof(value)         // Returns the type name
is_type(value, type)  // Checks the type
```

## Keywords

| Keyword                   | Description            |
| ------------------------- | ---------------------- |
| `Type`                    | Meta type              |
| `spawn`                   | Marks a spawn function |
| `spawn for`               | Parallel loop          |
| `spawn {}`                | Spawn block            |
| `if` / `else if` / `else` | Conditional branches   |
| `match`                   | Pattern matching       |
| `while` / `for`           | Loops                  |
| `return`                  | Return value           |
| `ref`                     | Creates a reference    |
| `mut`                     | Mutable marker         |

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
    ok(value) -> print("success: " + value),
    err(error) -> print("error: " + error),
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

| Precedence | Operator                                                                                                                                                                                                 |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Highest    | `( )` Function call                                                                                                                                                                                      |
|            | `.` Field access                                                                                                                                                                                         |
|            | `[ ]` Indexing                                                                                                                                                                                           |
|            | `unary -` Unary minus                                                                                                                                                                                    |
|            | `* / %` Multiplication, division, modulo                                                                                                                                                                 |
|            | (Operator precedence and associativity are fixed by the language; the semantics of `+ - * / %`, `== !=`, and `[]` may be type-overloaded through the corresponding implementation interface (RFC-011b).) |
|            | `+ -` Addition and subtraction                                                                                                                                                                           |
|            | `== != < > <= >=` Comparison                                                                                                                                                                             |
|            | `and or` Logical operations                                                                                                                                                                              |
| Lowest     | `=` Assignment                                                                                                                                                                                           |

## Standard Library Usage Examples

```yaoxiang
// Import the standard library
use std.io.{print, println}

// List operations
use std.list.{list_push, list_pop, list_len}

// Mathematical functions
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

# Execute in the interpreter
yx eval 'println("Hello")'

# View help
yaoxiang --help
```

## Complete Example

```yaoxiang
use std.convert
use std.io

// Calculate the Fibonacci sequence
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

## Contribution Guidelines

The standard library is under construction, and contributions are welcome!

1. Choose a module (such as `std.io`, `std.net`)
2. Implement functions in `src/std/`
3. Add documentation comments
4. Submit a PR
