# YaoXiang Reference Documentation

> YaoXiang is currently in the **experimental validation stage** (for the current version, see
> [Language Specification Overview](language-spec/index.md)), the standard library and API are being
> gradually improved.
>
> The **per-module authoritative reference** of the standard library is at
> [Standard Library Reference](stdlib/index.md); this page serves only as an entry index.

## Reference Entries

- [Error Code Reference](error-code/index.md) - Diagnostic codes overview and category lookup
- [Warning Codes](warning-code/warning-codes.md)
- [Package Management](package/index.md) - Manifest, lockfile and commands
- [Tool Commands](check-command.md) / [format](format-command.md) / [test](test-command.md)

## Language Specification

- [Language Specification Overview](language-spec/index.md)
- [Syntax Specification](language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](language-spec/type-system.md) - Basic types, composite types, generics, trait
- [Module System](language-spec/modules.md) - Module definition, import/export, scope
- [Concurrency Model](language-spec/concurrency.md) - Async programming, concurrency primitives,
  memory model
- [FFI](language-spec/ffi.md) - Foreign function interface
- [Standard Library](language-spec/stdlib.md) - Standard library overview

## Current Status

| Module           | Status         | Description                        | Reference                          |
| ---------------- | -------------- | ---------------------------------- | ---------------------------------- |
| `std.io`         | ✅ Implemented | Input/output                       | [io](stdlib/io.md)                 |
| `std.string`     | ✅ Implemented | String operations                  | [string](stdlib/string.md)         |
| `std.list`       | ✅ Implemented | List operations                    | [list](stdlib/list.md)             |
| `std.dict`       | ✅ Implemented | Dictionary operations              | [dict](stdlib/dict.md)             |
| `std.range`      | ✅ Implemented | Range and iterators (#302)         | [range](stdlib/range.md)           |
| `std.math`       | ✅ Implemented | Math functions                     | [math](stdlib/math.md)             |
| `std.net`        | ✅ Implemented | Network operations (ureq + rustls) | [net](stdlib/net.md)               |
| `std.concurrent` | ✅ Implemented | Concurrency primitives             | [concurrent](stdlib/concurrent.md) |
| `std.os`         | ✅ Implemented | Operating system interface         | [os](stdlib/os.md)                 |
| `std.fs`         | ✅ Implemented | File system                        | [fs](stdlib/fs.md)                 |
| `std.time`       | ✅ Implemented | Time and date                      | [time](stdlib/time.md)             |
| `std.convert`    | ✅ Implemented | Type conversion                    | [convert](stdlib/convert.md)       |
| `std.result`     | ✅ Implemented | Result type                        | [result](stdlib/result.md)         |
| `std.assert`     | ✅ Implemented | Assertions                         | [assert](stdlib/assert.md)         |
| `std.weak`       | ✅ Implemented | Weak references                    | [weak](stdlib/weak.md)             |

> `std.net` was previously a "placeholder implementation that did not send requests"; it has now
> been changed to a real implementation (#56).

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

| Type                 | Description                  | Example        |
| -------------------- | ---------------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Tuple of heterogeneous items | `(1, "hello")` |
| `(Args) -> Ret`      | Function type                | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Vec(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in
> primitives — they are generic type constructors, treated the same as user-defined generics and
> processed through a unified generic instantiation path. Literal syntax (`[...]` / `{...}`) is
> retained in the core, with the landing point determined by contextual annotations. Set has been
> removed (#300); see [Language Specification](language-spec/syntax.md) for details.
>
> The three container concepts are distinguished by where the length information lives:
> `Array(T, N)` has its length in the type (fixed length), `Vec(T)` has its length as a runtime
> value (raw buffer primitive), and `List(T)` is a standard-library type
> (`{ data: Vec(T), length: Int }`, with all policy in the library).

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

### Type Check

```yaoxiang
typeof(value)         // Return the type name
is_type(value, type)  // Check the type
```

## Keywords

| Keyword                   | Description            |
| ------------------------- | ---------------------- |
| `Type`                    | Meta type              |
| `spawn`                   | Mark as spawn function |
| `spawn for`               | Parallel loop          |
| `spawn {}`                | Spawn block            |
| `if` / `else if` / `else` | Conditional branch     |
| `match`                   | Pattern matching       |
| `while` / `for`           | Loop                   |
| `return`                  | Return value           |
| `ref`                     | Create reference       |
| `mut`                     | Mutable marker         |

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
// Regular function
add: (a: Int, b: Int) -> Int = a + b

// Spawn function (automatic concurrency)
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
// ? operator propagates errors
data = fetch_file(path)?
```

## Operator Precedence

| Precedence | Operator                                                                                                                                                                                      |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Highest    | `( )` function call                                                                                                                                                                           |
|            | `.` field access                                                                                                                                                                              |
|            | `[ ]` index                                                                                                                                                                                   |
|            | `unary -` unary minus                                                                                                                                                                         |
|            | `* / %` multiplication / division / modulo                                                                                                                                                    |
|            | (Operator precedence and associativity are fixed by the language; the semantics of `+ - * / %`, `== !=`, `[]` can be overloaded by types implementing the corresponding interface (RFC-011b)) |
|            | `+ -` addition / subtraction                                                                                                                                                                  |
|            | `== != < > <= >=` comparison                                                                                                                                                                  |
|            | `and or` logical operations                                                                                                                                                                   |
| Lowest     | `=` assignment                                                                                                                                                                                |

## Standard Library Usage Examples

```yaoxiang
// Import the standard library
use std.io.{print, println}

// List operations
use std.list.{list_push, list_pop, list_len}

// Math functions
use std.math.{sqrt, sin, cos, PI}

// Use
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## Command-line Tools

```bash
# Run a script
yx run hello.yx

# Build bytecode
yx build hello.yx -o hello.42

# Interpret and execute
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

The standard library is under construction — contributions are welcome!

1. Choose a module (e.g. `std.io`, `std.net`)
2. Implement the functions in `src/std/`
3. Add documentation comments
4. Submit a PR
