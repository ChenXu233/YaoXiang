# YaoXiang Reference Documentation

> YaoXiang is currently in the **experimental validation stage** (for the current version, see
> [Language Specification Overview](./language-spec/index.md)); the standard library and APIs are
> being progressively improved.
>
> The **authoritative per-module reference** for the standard library is in
> [Standard Library Reference](./stdlib/index.md); this page only serves as an entry index.

## Reference Entries

- [Error Code Reference](./error-code/index.md) - Overview of diagnostic codes and categorized
  lookup
- [Warning Codes](./warning-code/warning-codes.md)
- [Package Management](./package/index.md) - Manifest, lock file, and commands
- [Tool Commands](./check-command.md) / [format](./format-command.md) / [test](./test-command.md)

## Language Specification

- [Language Specification Overview](./language-spec/index.md)
- [Syntax Specification](./language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](./language-spec/type-system.md) - Primitive types, composite types, generics, trait
- [Module System](./language-spec/modules.md) - Module definitions, imports/exports, scope
- [Concurrency Model](./language-spec/concurrency.md) - Asynchronous programming, concurrency
  primitives, memory model
- [FFI](./language-spec/ffi.md) - Foreign function interface
- [Standard Library](./language-spec/stdlib.md) - Standard library overview

## Current Status

| Module           | Status         | Description                 | Reference                            |
| ---------------- | -------------- | --------------------------- | ------------------------------------ |
| `std.io`         | ✅ Implemented | Input/output                | [io](./stdlib/io.md)                 |
| `std.string`     | ✅ Implemented | String operations           | [string](./stdlib/string.md)         |
| `std.list`       | ✅ Implemented | List operations             | [list](./stdlib/list.md)             |
| `std.dict`       | ✅ Implemented | Dictionary operations       | [dict](./stdlib/dict.md)             |
| `std.range`      | ✅ Implemented | Ranges and iterators (#302) | [range](./stdlib/range.md)           |
| `std.math`       | ✅ Implemented | Math functions              | [math](./stdlib/math.md)             |
| `std.net`        | ✅ Implemented | Networking (ureq + rustls)  | [net](./stdlib/net.md)               |
| `std.concurrent` | ✅ Implemented | Concurrency primitives      | [concurrent](./stdlib/concurrent.md) |
| `std.os`         | ✅ Implemented | Operating system interface  | [os](./stdlib/os.md)                 |
| `std.fs`         | ✅ Implemented | File system                 | [fs](./stdlib/fs.md)                 |
| `std.time`       | ✅ Implemented | Time and date               | [time](./stdlib/time.md)             |
| `std.convert`    | ✅ Implemented | Type conversion             | [convert](./stdlib/convert.md)       |
| `std.result`     | ✅ Implemented | Result type                 | [result](./stdlib/result.md)         |
| `std.assert`     | ✅ Implemented | Assertions                  | [assert](./stdlib/assert.md)         |
| `std.weak`       | ✅ Implemented | Weak references             | [weak](./stdlib/weak.md)             |

> `std.net` was originally a "placeholder implementation that did not send requests"; it has now
> been replaced with a real implementation (#56).

## Built-in Types

### Primitive Types

| Type     | Description      | Example         |
| -------- | ---------------- | --------------- |
| `Void`   | Void / no return | `()`            |
| `Bool`   | Boolean value    | `true`, `false` |
| `Int`    | Integer          | `42`, `-10`     |
| `Float`  | Floating-point   | `3.14`, `-0.5`  |
| `Char`   | Character        | `'a'`, `'中'`   |
| `String` | String           | `"hello"`       |

### Composite Types

| Type                 | Description         | Example        |
| -------------------- | ------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Heterogeneous tuple | `(1, "hello")` |
| `(Args) -> Ret`      | Function type       | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Vec(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in
> primitives — they are generic type constructors, treated the same as user-defined generics, and
> handled through a unified generic instantiation path. Literal syntax (`[...]` / `{...}`) remains
> in the core, and the resolution is determined by context annotations. `Set` has been removed
> (#300); see [Language Specification](language-spec/syntax.md) for details.
>
> The three container concepts are distinguished by where the length information lives:
> `Array(T, N)` has the length in the type (fixed-size), `Vec(T)` has the length as a runtime value
> (raw buffer primitive), and `List(T)` is a standard library type (`{ data: Vec(T), length: Int }`,
> with the strategy fully in the library).

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
print(value)           // print, no newline
println(value)         // print, with newline
```

### Conversion

```yaoxiang
to_string(value)       // convert to string
to_int(value)          // convert to integer
to_float(value)        // convert to float
```

### Type Checking

```yaoxiang
typeof(value)         // returns the type name
is_type(value, type)  // checks the type
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
| `ref`                     | Create a reference     |
| `mut`                     | Mutable marker         |

## Syntax Quick Reference

### Variable Declarations

```yaoxiang
// Immutable variable (default)
x: Int = 42
y = 42                 // type inference

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
// Conditionals
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

// Loops
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

| Precedence | Operators                                                                                                                                                                                     |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Highest    | `( )` function call                                                                                                                                                                           |
|            | `.` field access                                                                                                                                                                              |
|            | `[ ]` indexing                                                                                                                                                                                |
|            | `unary -` unary negation                                                                                                                                                                      |
|            | `* / %` multiplication, division, modulus                                                                                                                                                     |
|            | (Operator precedence and associativity are fixed by the language; the semantics of `+ - * / %` `== !=` `[]` can be overloaded by types that implement the corresponding interface (RFC-011b)) |
|            | `+ -` addition, subtraction                                                                                                                                                                   |
|            | `== != < > <= >=` comparison                                                                                                                                                                  |
|            | `and or` logical operators                                                                                                                                                                    |
| Lowest     | `=` assignment                                                                                                                                                                                |

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

- [Tutorials](../tutorial/) - Learn YaoXiang
- [Design Documents](../design/) - Language design decisions
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## Contribution Guide

The standard library is under construction — contributions are welcome!

1. Pick a module (e.g. `std.io`, `std.net`)
2. Implement the functions in `src/std/`
3. Add documentation comments
4. Submit a PR
