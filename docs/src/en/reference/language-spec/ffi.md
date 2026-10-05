# FFI Specification

This document defines the FFI (Foreign Function Interface) specification of the YaoXiang programming
language, including type definitions, function declarations, method bindings, and handling of opaque
types.

> **Detailed design**: For the complete FFI design, motivation, and trade-offs, see
> [RFC-026: FFI Core Mechanism](../../rfc/accepted/026-ffi-core-mechanism.md).

---

## Chapter 1: Overview

### 1.1 Core Principles of FFI

```
All `return`s in `{}` return their content to the enclosing scope. The default with no `return` is to return Void.
```

### 1.2 FFI Components

| Component            | Description                              | Syntax                 |
| -------------------- | ---------------------------------------- | ---------------------- |
| Type Definition      | Define FFI types (opaque or transparent) | `unsafe {}` + `return` |
| Function Declaration | Declare external function                | `native("symbol")`     |
| Method Binding       | Bind methods to types                    | `[0]` syntax           |

---

## Chapter 2: FFI Type Definition

### 2.1 Opaque Types

Opaque types are defined inside an `unsafe {}` block and returned to the enclosing scope via
`return`:

```yaoxiang
// Define an opaque type in an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

// ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

// ✅ Via a method call
db.close()
```

### 2.2 Transparent Types

Transparent types are defined directly, without an `unsafe {}` block:

```yaoxiang
// Transparent type
Point: Type = {
    x: Int32,
    y: Int32
}

// Users can create them directly
p: Point = Point { x: 1, y: 2 }
```

### 2.3 Determining Opaque Types

The compiler automatically distinguishes opaque types from vacuous types:

```yaoxiang
// Opaque type (referenced by a native function)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb is referenced by a native function → opaque type

// Vacuous type (not referenced by any native function)
MyType: Type = {}
// → MyType is not referenced by any native function → vacuous type
```

**Determination rules**:

- If the type is referenced by a `native` function → opaque type
- Otherwise → vacuous type

---

## Chapter 3: FFI Function Declaration

### 3.1 `native` Syntax

Use the `native("symbol")` syntax to declare external functions:

```yaoxiang
// FFI function declaration
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 Parameter Type Mapping

FFI function parameter types use YaoXiang types directly; the compiler handles the C type mapping
automatically:

| C type               | YaoXiang type                     |
| -------------------- | --------------------------------- |
| `int`                | `Int32`                           |
| `long`               | `Int64`                           |
| `float`              | `Float32`                         |
| `double`             | `Float64`                         |
| `char`               | `Char`                            |
| `char*`              | `String`                          |
| `bool`               | `Bool`                            |
| `size_t`             | **no direct mapping** (see below) |
| `void*`              | `*Void`                           |
| `struct T*`          | `T` (transparent type)            |
| `typedef struct T T` | `T` (opaque type)                 |

> **`size_t` has no corresponding YaoXiang type**: The type system has only **signed** integer
> families (`src/frontend/core/types/mono.rs:618-632`: `Int` / `Int8` / `Int16` / `Int32` / `Int64`,
> and `Float` / `Float32` / `Float64`), **with no `Uint` or any unsigned type**. For size / length
> parameters, use `Int64` and bound the upper limit on your side (the actual values of `size_t` will
> not exceed `i64::MAX`, but the upstream convention of returning negative numbers is something you
> have to defend against yourself).
>
> (The name `Uint` does in fact appear in the repo, but only in two **non-type** contexts: the
> completion-candidate table at `src/lsp/world.rs:176`, and the `sizeof` fallback branch at
> `src/frontend/core/types/eval/const_eval.rs:503`. It is not a usable type name.)

### 3.3 Return Type

FFI function return types use YaoXiang types directly:

```yaoxiang
// Returns an opaque type
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Returns a transparent type
get_point: () -> Point = native("get_point")

// Returns a primitive type
get_value: () -> Int32 = native("get_value")
```

---

## Chapter 4: Method Binding

### 4.1 The `[0]` Syntax

Use the `[0]` syntax to specify the position of the `self` parameter in the function's parameter
tuple:

```yaoxiang
// FFI functions
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Method bindings (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Calling method**:

```yaoxiang
db = sqlite3_open("test.db")

// Method call
db.close()  // equivalent to sqlite3_close(db)
db.exec("SELECT * FROM users")  // equivalent to sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Constructor Binding

Constructors do not use `[0]`; they are bound as ordinary functions:

```yaoxiang
// FFI function
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Constructor binding (regular function)
SqliteDb.open = sqlite3_open
```

**Calling method**:

```yaoxiang
// Created via the constructor
db = SqliteDb.open("test.db")
```

### 4.3 Binding Location

Method bindings can appear anywhere, because types are data containers:

```yaoxiang
// Bind after the type definition
SqliteDb.close = sqlite3_close[0]

// Bind in another file
SqliteDb.exec = sqlite3_exec[0]

// The compiler will check everything in the end
```

---

## Chapter 5: FFI Behavior in `spawn` Blocks

### 5.1 Resource Types Are Automatically Serialized

If an FFI type is a resource type, accesses inside a `spawn` block are automatically serialized:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // Different instances, can run in parallel
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // Same SqliteDb
    result2 = db.exec("INSERT ...")   // Automatically serialized
}
```

### 5.2 Non-Resource Types Can Run in Parallel

If an FFI type is not a resource type, calls inside a `spawn` block can run in parallel:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // can run in parallel
    result2 = cos(1.0)   // can run in parallel
}
```

---

## Chapter 6: yx-bindgen Toolchain (Planned, Not Implemented)

> **Status: not implemented.** Auto-generating bindings from C headers is the goal of
> [RFC-026b](../../rfc/accepted/026-ffi-core-mechanism.md) (the automation sub-proposal of this
> chapter; there is no standalone document for it in the repo yet). At present there is **no
> executable entry point**:
>
> - `Cargo.toml` declares only a single binary, `[[bin]] yaoxiang-rs`; there is no `yx-bindgen`.
> - Writing `[build].headers` in the package manifest will **report a direct error**——
>   `[build].headers requires yx-bindgen (RFC-026b, not yet implemented); remove headers or switch to [binaries] for precompiled distribution`
>   (`src/package/build/mod.rs:152-159`).
>
> Therefore this section only records the **design intent** (the shape that should be generated once
> RFC-026b lands), not a currently usable CLI. To do the same thing today, just handwrite the
> content of Chapters 2 through 4 — the `unsafe {}` type definitions, `native("symbol")`
> declarations, and `[0]` method bindings described there are all already implemented.

### 6.1 Planned Generated Content

- FFI type definitions (`unsafe` block + `return`)
- FFI function declarations (`native` syntax)
- Method bindings (`[0]` syntax)

### 6.2 Planned Call Form and Handwritten Equivalent

```bash
# Planned, not yet implemented:
# yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

The generated output is equivalent to the following handwritten form:

```
// sqlite3_bindings.yx (currently must be handwritten)
// Auto-generated; do not edit by hand

// ============================================================================
// Type definitions
// ============================================================================

SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

SqliteStmt = unsafe {
    SqliteStmt: Type = {
        handle: *Void
    }
    return SqliteStmt
}

// ============================================================================
// FFI function declarations
// ============================================================================

sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
sqlite3_prepare_v2: (db: SqliteDb, sql: String) -> SqliteStmt = native("sqlite3_prepare_v2")
sqlite3_step: (stmt: SqliteStmt) -> Int32 = native("sqlite3_step")
sqlite3_finalize: (stmt: SqliteStmt) -> Int32 = native("sqlite3_finalize")

// ============================================================================
// Method bindings
// ============================================================================

// Constructor (regular function)
SqliteDb.open = sqlite3_open

// Methods (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// SqliteStmt methods
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## Appendix: FFI Syntax Quick Reference

### A.1 Type Definition

```yaoxiang
// Opaque type
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

// Transparent type
Point: Type = {
    x: Int32,
    y: Int32
}
```

### A.2 Function Declaration

```yaoxiang
// FFI function declaration
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 Method Binding

```yaoxiang
// Constructor (regular function)
SqliteDb.open = sqlite3_open

// Method (self at position 0)
SqliteDb.close = sqlite3_close[0]
```

### A.4 Calling Method

```yaoxiang
// Created via the constructor
db = SqliteDb.open("test.db")

// Via a method call
db.close()
db.exec("SELECT * FROM users")
```
