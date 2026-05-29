# Standard Library Specification

This document defines the standard library specification for the YaoXiang programming language, including the core library, IO library, and math library.

---

## Chapter 1: Core Library

### 1.1 Basic Types

The standard library provides implementations for the following basic types:

| Type | Module | Description |
|------|--------|-------------|
| `Option[T]` | `std.option` | Optional value type |
| `Result[T, E]` | `std.result` | Error handling type |
| `List[T]` | `std.collection` | Dynamic array |
| `Map[K, V]` | `std.collection` | Hash map |
| `String` | `std.string` | String type |
| `Array[T, N]` | `std.array` | Fixed-size array |

### 1.2 Option Type

```
Option: Type[T] = some(T) | none
```

**Value variant constructors**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `some(T)` | `some(value)` | Has a value |
| `none` | `none` | No value |

**Common methods**:

```yaoxiang
// Check if has value
is_some: (self: Option[T]) -> Bool
is_none: (self: Option[T]) -> Bool

// Get value (may panic)
unwrap: (self: Option[T]) -> T

// Get value or default
unwrap_or: (self: Option[T], default: T) -> T

// Map value
map: [R](self: Option[T], f: Fn(T) -> R) -> Option[R]
```

### 1.3 Result Type

```
Result: Type[T, E] = ok(T) | err(E)
```

**Value variant constructors**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `ok(T)` | `ok(value)` | Success value |
| `err(E)` | `err(error)` | Error value |

**Common methods**:

```yaoxiang
// Check if success
is_ok: (self: Result[T, E]) -> Bool
is_err: (self: Result[T, E]) -> Bool

// Get value (may panic)
unwrap: (self: Result[T, E]) -> T

// Get value or default
unwrap_or: (self: Result[T, E], default: T) -> T

// Map success value
map: [R](self: Result[T, E], f: Fn(T) -> R) -> Result[R, E]

// Map error value
map_err: [F](self: Result[T, E], f: Fn(E) -> F) -> Result[T, F]
```

### 1.4 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates Result type errors:

```
// Returns value on success, returns err upward on failure
data = fetch_data()?

// Equivalent to
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

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
    read: (self: File) -> Result[String, Error],
    write: (self: File, content: String) -> Result[Void, Error],
    append: (self: File, content: String) -> Result[Void, Error],
    close: (self: File) -> Void
}

// File operations
open: (path: String) -> Result[File, Error]
create: (path: String) -> Result[File, Error]
delete: (path: String) -> Result[Void, Error]
```

### 2.3 Directory Operations

```yaoxiang
// Directory type
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result[List[String], Error],
    create: (self: Dir) -> Result[Void, Error],
    delete: (self: Dir) -> Result[Void, Error]
}

// Directory operations
read_dir: (path: String) -> Result[Dir, Error]
create_dir: (path: String) -> Result[Void, Error]
delete_dir: (path: String) -> Result[Void, Error]
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

// Power operations
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// Logarithms
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
split: (s: String, delimiter: String) -> List[String]

// String searching
find: (s: String, pattern: String) -> Option[Int]
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
parse_int: (s: String) -> Result[Int, Error]
parse_float: (s: String) -> Result[Float, Error]
```

---

## Chapter 5: Collection Library

### 5.1 List Type

```yaoxiang
// List type
List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    pop: [T](self: List[T]) -> Option[T],
    get: [T](self: List[T], index: Int) -> Option[T],
    set: [T](self: List[T], index: Int, value: T) -> Void,
    insert: [T](self: List[T], index: Int, item: T) -> Void,
    remove: [T](self: List[T], index: Int) -> Option[T],
    clear: [T](self: List[T]) -> Void,
    contains: [T](self: List[T], item: T) -> Bool,
    sort: [T](self: List[T]) -> List[T],
    reverse: [T](self: List[T]) -> List[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], predicate: Fn(T) -> Bool) -> List[T],
    reduce: [T, R](self: List[T], initial: R, f: Fn(R, T) -> R) -> R
}
```

### 5.2 Map Type

```yaoxiang
// Map type
Map: Type[K, V] = {
    data: Array[(K, V)],
    length: Int,
    insert: [K, V](self: Map[K, V], key: K, value: V) -> Void,
    get: [K, V](self: Map[K, V], key: K) -> Option[V],
    remove: [K, V](self: Map[K, V], key: K) -> Option[V],
    contains_key: [K, V](self: Map[K, V], key: K) -> Bool,
    keys: [K, V](self: Map[K, V]) -> List[K],
    values: [K, V](self: Map[K, V]) -> List[V],
    clear: [K, V](self: Map[K, V]) -> Void
}
```

---

## Chapter 6: Iterator Library

### 6.1 Iterator trait

```yaoxiang
// Iterator trait
Iterator: Type[T] = {
    Item: T,
    next: (self: Self) -> Option[T],
    has_next: (self: Self) -> Bool,
    map: [R](self: Self, f: Fn(T) -> R) -> Iterator[R],
    filter: (self: Self, predicate: Fn(T) -> Bool) -> Iterator[T],
    collect: (self: Self) -> List[T],
    reduce: [R](self: Self, initial: R, f: Fn(R, T) -> R) -> R,
    for_each: (self: Self, f: Fn(T) -> Void) -> Void
}
```

### 6.2 Iterator Adapters

```yaoxiang
// Range iterator
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator[Int]
}

// Usage
for i in 0..10 {
    print(i)
}

for i in 0..10 step 2 {
    print(i)
}
```

---

## Appendix: Standard Library Module Index

### A.1 Core Modules

| Module | Description |
|--------|-------------|
| `std.option` | Option type |
| `std.result` | Result type |
| `std.collection` | Collection types like List, Map |
| `std.string` | String operations |
| `std.array` | Array operations |
| `std.iterator` | Iterator |

### A.2 IO Modules

| Module | Description |
|--------|-------------|
| `std.io` | Standard input/output |
| `std.file` | File operations |
| `std.dir` | Directory operations |

### A.3 Math Modules

| Module | Description |
|--------|-------------|
| `std.math` | Math functions |
| `std.math.trig` | Trigonometric functions |
| `std.math.log` | Logarithmic functions |

### A.4 Utility Modules

| Module | Description |
|--------|-------------|
| `std.random` | Random number generation |
| `std.time` | Date and time |
| `std.regex` | Regular expressions |