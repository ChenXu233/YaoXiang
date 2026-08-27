# Standard Library Specification

This document defines the standard library specification for the YaoXiang programming language,
including the core library, IO library, and math library.

---

## Chapter 1: Core Library

### 1.1 Basic Types

The standard library provides implementations for the following basic types:

| Type           | Module           | Description         |
| -------------- | ---------------- | ------------------- |
| `Option(T)`    | `std.option`     | Optional value type |
| `Result(T, E)` | `std.result`     | Error handling type |
| `List(T)`      | `std.collection` | Dynamic array       |
| `Map(K, V)`    | `std.collection` | Hash map            |
| `String`       | `std.string`     | String type         |
| `Array(T, N)`  | `std.array`      | Fixed-size array    |

### 1.2 Option Type

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Variant Constructors**:

| Variant       | Syntax               | Description |
| ------------- | -------------------- | ----------- |
| `Option.some` | `Option.some(value)` | Has value   |
| `Option.none` | `Option.none()`      | No value    |

**Common Methods**:

```yaoxiang
// Check whether a value is present
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Get value (may panic)
unwrap: (self: Option(T)) -> T

// Get value or default
unwrap_or: (self: Option(T), default: T) -> T

// Map value
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Result Type

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Variant Constructors**:

| Variant      | Syntax              | Description   |
| ------------ | ------------------- | ------------- |
| `Result.ok`  | `Result.ok(value)`  | Success value |
| `Result.err` | `Result.err(error)` | Error value   |

**Common Methods**:

```yaoxiang
// Check whether it succeeded
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Get value (may panic)
unwrap: (self: Result(T, E)) -> T

// Get value or default
unwrap_or: (self: Result(T, E), default: T) -> T

// Map success value
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Map error value
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates the error of a `Result` type:

```
// Returns the value on success, propagates err upward on failure
data = fetch_data()?

// Equivalent to
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Assertion (`std.assert`)

The `std.assert` module provides a unified assertion mechanism — the runtime `assert` and the
compile-time refinement type `Assert` are two sides of the same primitive.

```yaoxiang
// IsTrue: the bridge function from value to type
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, program continues
    false => Never,    // ⊥, diverges
}

// Assert: the compile-time refinement type primitive
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: runtime assertion (value introducer of Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Result overload
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Dispatch**:

| Condition                                              | Behavior                                                         |
| ------------------------------------------------------ | ---------------------------------------------------------------- |
| All free variables of `cond` are known at compile time | Compiler evaluates; `true` → erase, `false` → compile error      |
| A runtime free variable is present                     | Insert a runtime check, inject a flow-sensitive assumption set Γ |

`assert(false, "msg")` is equivalent to `raise` — no separate `throw`/`raise` keyword is needed.

---

## Chapter 2: IO Library

### 2.1 Standard Input/Output

```yaoxiang
// Standard output
print: (msg: String) -> Void
println: (msg: String) -> Void

// Standard input
read_line: () -> String
read_char: () -> Char
```

### 2.2 File Operations

```yaoxiang
// File type
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// File operations
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 Directory Operations

```yaoxiang
// Directory type
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// Directory operations
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## Chapter 3: Math Library

### 3.1 Basic Math Functions

```yaoxiang
// Absolute value
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// Max and min
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// Power
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// Logarithm
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 Trigonometric Functions

```yaoxiang
// Trigonometric functions
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// Inverse trigonometric functions
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 Constants

```yaoxiang
// Math constants
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Chapter 4: String Library

### 4.1 String Operations

```yaoxiang
// String length
length: (s: String) -> Int

// String concatenation
concat: (a: String, b: String) -> String

// String splitting
split: (s: String, delimiter: String) -> List(String)

// String search
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// String replacement
replace: (s: String, old: String, new: String) -> String

// String trimming
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 String Conversion

```yaoxiang
// Type conversion
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// Parsing
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## Chapter 5: Collection Library

### 5.1 List Type

```yaoxiang
// List type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T: Type) -> ((self: List(T), item: T) -> Void),
    pop: (T: Type) -> ((self: List(T)) -> Option(T)),
    get: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    set: (T: Type) -> ((self: List(T), index: Int, value: T) -> Void),
    insert: (T: Type) -> ((self: List(T), index: Int, item: T) -> Void),
    remove: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    clear: (T: Type) -> ((self: List(T)) -> Void),
    contains: (T: Type) -> ((self: List(T), item: T) -> Bool),
    sort: (T: Type) -> ((self: List(T)) -> List(T)),
    reverse: (T: Type) -> ((self: List(T)) -> List(T)),
    map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)),
    reduce: (T: Type, R: Type) -> ((self: List(T), initial: R, f: (R, T) -> R) -> R)
}
```

### 5.2 Map Type

```yaoxiang
// Map type
Map: (K: Type, V: Type) -> Type = {
    data: Array((K, V)),
    length: Int,
    insert: (K: Type, V: Type) -> ((self: Map(K, V), key: K, value: V) -> Void),
    get: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    remove: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    contains_key: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Bool),
    keys: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(K)),
    values: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(V)),
    clear: (K: Type, V: Type) -> ((self: Map(K, V)) -> Void)
}
```

---

## Chapter 6: Iterator Library

### 6.1 Iterator Trait

```yaoxiang
// Iterator trait
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

### 6.2 Iterator Adapters

```yaoxiang
// Range iterator (#300 Item I: Range is a first-class value, step is the third component)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Usage
for i in 0..10 {
    print(i)
}

// Step form (double dot, no new keyword)
for i in 0..10..2 {
    print(i)
}
```

> **#300 Item I**: Range is a first-class value — `r = 1..10` is legal, `x in r` membership check
> and `for i in r` iteration are both desugared according to the static type. A literal `step=0` is
> rejected at compile time; a dynamic `step=0` raises a runtime error (once the future error system
> lands, it will be promoted to `Result`, #301).

---

## Appendix: Standard Library Module Index

| Module           | Description                                                                    |
| ---------------- | ------------------------------------------------------------------------------ |
| `std.assert`     | Assertion mechanism — runtime `assert` + compile-time `Assert` refinement type |
| `std.option`     | `Option` type                                                                  |
| `std.result`     | `Result` type                                                                  |
| `std.collection` | Collection types such as `List`, `Map`                                         |
| `std.string`     | String operations                                                              |
| `std.array`      | Array operations                                                               |
| `std.iterator`   | Iterator                                                                       |

### A.2 IO Modules

| Module     | Description           |
| ---------- | --------------------- |
| `std.io`   | Standard input/output |
| `std.file` | File operations       |
| `std.dir`  | Directory operations  |

### A.3 Math Modules

| Module          | Description             |
| --------------- | ----------------------- |
| `std.math`      | Math functions          |
| `std.math.trig` | Trigonometric functions |
| `std.math.log`  | Logarithm functions     |

### A.4 Utility Modules

| Module       | Description                                                            |
| ------------ | ---------------------------------------------------------------------- |
| `std.random` | Random number generation                                               |
| `std.time`   | Time and date                                                          |
| `std.assert` | Unified compile-time `Assert(C)` and runtime `assert(x > 0)` (RFC-030) |
| `std.regex`  | Regular expression                                                     |
