# YaoXiang Reference Documentation

> This document is under construction...

YaoXiang is currently in the **experimental validation stage**, with the standard library and API
being gradually improved.

## Language Specification

- [Language Specification Overview](./language-spec/index.md)
- [Syntax Specification](./language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](./language-spec/type-system.md) - primitive types, composite types, generics, trait
- [Module System](./language-spec/modules.md) - module definition, import/export, scope
- [Concurrency Model](./language-spec/concurrency.md) - asynchronous programming, concurrency
  primitives, memory model
- [Standard Library](./language-spec/stdlib.md) - core library, IO library, math library

## Current Status

| Module           | Status         | Description                 |
| ---------------- | -------------- | --------------------------- |
| `std.io`         | 🔨 In progress | Input/Output                |
| `std.string`     | 🔨 In progress | String operations           |
| `std.list`       | 🔨 In progress | List operations             |
| `std.dict`       | ✅ Implemented | Dictionary operations       |
| `std.range`      | ✅ Implemented | Ranges and iterators (#302) |
| `std.math`       | 🔨 In progress | Math functions              |
| `std.net`        | 📋 Planned     | Network operations          |
| `std.concurrent` | 📋 Planned     | Concurrency primitives      |

## Built-in Types

### Primitive Types

| Type     | Description            | Example         |
| -------- | ---------------------- | --------------- |
| `Void`   | void / no return value | `()`            |
| `Bool`   | Boolean                | `true`, `false` |
| `Int`    | Integer                | `42`, `-10`     |
| `Float`  | Float                  | `3.14`, `-0.5`  |
| `Char`   | Character              | `'a'`, `'中'`   |
| `String` | String                 | `"hello"`       |

### Composite Types

| Type                 | Description                 | Example        |
| -------------------- | --------------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Heterogeneous element tuple | `(1, "hello")` |
| `(Args) -> Ret`      | Function type               | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Vec(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in
> primitives — they are generic type constructors, treated the same as user-defined generics, and
> handled through the unified generics instantiation path. The literal syntax (`[...]` / `{...}`)
> remains in the core, with placement decided by context annotation. Set has been removed (#300);
> see the [Language Specification](language-spec/syntax.md) for details.
>
> The three container concepts are distinguished by where the length information lives:
> `Array(T, N)` — length in the type (fixed-length), `Vec(T)` — length is a runtime value (raw
> buffer primitive), `List(T)` — a standard library type (`{ data: Vec(T), length: Int }`, with all
> strategy in the library).

### User-Defined Types

```yaoxiang
// record type (struct)
Point: Type = { x: Float, y: Float }

// enum type
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// interface type (all fields are functions)
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

### Type Check

```yaoxiang
typeof(value)         // return type name
is_type(value, type)  // check type
```

## Keywords

| Keyword                   | Description         |
| ------------------------- | ------------------- |
| `Type`                    | meta type           |
| `spawn`                   | mark spawn function |
| `spawn for`               | parallel loop       |
| `spawn {}`                | spawn block         |
| `if` / `else if` / `else` | conditional branch  |
| `match`                   | pattern matching    |
| `while` / `for`           | loop                |
| `return`                  | return value        |
| `ref`                     | create reference    |
| `mut`                     | mutable marker      |

## Syntax Quick Reference

### Variable Declaration

```yaoxiang
// immutable variable (default)
x: Int = 42
y = 42                 // type inference

// mutable variable
mut count: Int = 0
count = count + 1
```

### Function Definition

```yaoxiang
// regular function
add: (a: Int, b: Int) -> Int = a + b

// spawn function (automatic concurrency)
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// generic function
identity: [T](x: T) -> T = x
```

### Control Flow

```yaoxiang
// conditional
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// pattern matching
match result {
    ok(value) => print("success: " + value),
    err(error) => print("error: " + error),
}

// loop
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

| Precedence | Operator                                 |
| ---------- | ---------------------------------------- |
| Highest    | `( )` function call                      |
|            | `.` field access                         |
|            | `[ ]` indexing                           |
|            | `unary -` unary minus                    |
|            | `* / %` multiplication, division, modulo |
|            | `+ -` addition, subtraction              |
|            | `== != < > <= >=` comparison             |
|            | `and or` logical operations              |
| Lowest     | `=` assignment                           |

## Standard Library Usage Examples

```yaoxiang
// import standard library
use std.io.{print, println}

// list operations
use std.list.{list_push, list_pop, list_len}

// math functions
use std.math.{sqrt, sin, cos, PI}

// usage
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## Command-Line Tool

```bash
# run script
yx run hello.yx

# build bytecode
yx build hello.yx -o hello.42

# interpret and execute
yx eval 'println("Hello")'

# view help
yaoxiang --help
```

## Complete Example

```yaoxiang
use std.convert
use std.io

// compute Fibonacci sequence
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// main function
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

1. Choose a module (e.g. `std.io`, `std.net`)
2. Implement functions in `src/std/`
3. Add documentation comments
4. Submit a PR
