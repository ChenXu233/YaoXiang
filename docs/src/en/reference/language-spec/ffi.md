# FFI Specification

This document defines the FFI (Foreign Function Interface) specification for the YaoXiang
programming language, including type definitions, function declarations, method bindings, and opaque
type handling.

> **Detailed design**: The complete FFI design, motivations, and trade-offs are detailed in
> [RFC-026: FFI Core Mechanism](../../rfc/accepted/026-ffi-core-mechanism.md).

---

## Chapter 1: Overview

### 1.1 Core Principles of FFI

```
All return statements inside {} return their content to the outer scope
By default, if there is no return, the return type is Void
```

### 1.2 Components of FFI

| Component            | Description                              | Syntax                 |
| -------------------- | ---------------------------------------- | ---------------------- |
| Type definition      | Define FFI types (opaque or transparent) | `unsafe {}` + `return` |
| Function declaration | Declare external functions               | `native("symbol")`     |
| Method binding       | Bind methods to types                    | `[0]` syntax           |

---

## Chapter 2: FFI Type Definitions

### 2.1 Opaque Types

Opaque types are defined in `unsafe {}` blocks and returned to the outer scope via `return`:

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

// ❌ Compile error: the handle field requires unsafe permission
handle = db.handle

// ✅ Through method calls
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

// Users can create instances directly
p: Point = Point { x: 1, y: 2 }
```

### 2.3 Determining Opaque Types

The compiler automatically determines opaque types and vacuous types:

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

- If a type is referenced by a `native` function → opaque type
- Otherwise → vacuous type

---

## Chapter 3: FFI Function Declarations

### 3.1 native Syntax

Use the `native("symbol")` syntax to declare external functions:

```yaoxiang
// FFI function declaration
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 Parameter Type Mapping

The parameter types of FFI functions use YaoXiang types directly, and the compiler automatically
handles C type mapping:

| C type               | YaoXiang type                     |
| -------------------- | --------------------------------- |
| `int`                | `Int32`                           |
| `long`               | `Int64`                           |
| `float`              | `Float32`                         |
| `double`             | `Float64`                         |
| `char`               | `Char`                            |
| `char*`              | `String`                          |
| `bool`               | `Bool`                            |
| `size_t`             | **No direct mapping** (see below) |
| `void*`              | `*Void`                           |
| `struct T*`          | `T` (transparent type)            |
| `typedef struct T T` | `T` (opaque type)                 |

> **`size_t` has no corresponding YaoXiang type**: the type system only has **signed** integer
> families (`src/frontend/core/types/mono.rs:618-632`: `Int` / `Int8` / `Int16` / `Int32` / `Int64`
> and `Float` / `Float32` / `Float64`), **with no `Uint` or any unsigned type**. For size/length
> parameters, use `Int64` and handle the upper bound on your own side (in practice, `size_t` values
> will not exceed `i64::MAX`, but upstream conventions that return negative values need to be
> guarded by you).
>
> (The name `Uint` does appear in the repository, but only in two **non-type** contexts: the
> completion candidate table in `src/lsp/world.rs:176`, and the `sizeof` fallback branch in
> `src/frontend/core/types/eval/const_eval.rs:503`. It is not an available type name.)

### 3.3 Return Type

The return type of FFI functions uses YaoXiang types directly:

```yaoxiang
// Return an opaque type
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Return a transparent type
get_point: () -> Point = native("get_point")

// Return a primitive type
get_value: () -> Int32 = native("get_value")
```

---

## Chapter 4: Method Binding

### 4.1 The [0] Syntax

Use the `[0]` syntax to specify the position of the self parameter in the function's parameter
tuple:

```yaoxiang
// FFI function
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Method binding (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Invocation**:

```yaoxiang
db = sqlite3_open("test.db")

// Method call
db.close()  // Equivalent to sqlite3_close(db)
db.exec("SELECT * FROM users")  // Equivalent to sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Constructor Binding

Constructors do not use `[0]`; they are bound as regular functions:

```yaoxiang
// FFI function
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Constructor binding (regular function)
SqliteDb.open = sqlite3_open
```

**Invocation**:

```yaoxiang
// Create via the constructor
db = SqliteDb.open("test.db")
```

### 4.3 Binding Position

Method bindings can be placed anywhere, because types are data containers:

```yaoxiang
// Bind after type definition
SqliteDb.close = sqlite3_close[0]

// Bind in another file
SqliteDb.exec = sqlite3_exec[0]

// The compiler will check all of them eventually
```

---

## Chapter 5: FFI Behavior in spawn Blocks

### 5.1 Resource Types Auto-Serialize

If the FFI type is a resource type, it is automatically serialized within a spawn block:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // Different instances, can run in parallel
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // Same SqliteDb
    result2 = db.exec("INSERT ...")   // Auto-serialized
}
```

### 5.2 Non-Resource Types Can Run in Parallel

If the FFI type is not a resource type, it can run in parallel within a spawn block:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // Can run in parallel
    result2 = cos(1.0)   // Can run in parallel
}
```

---

## Chapter 6: yx-bindgen Toolchain (Planned, Not Implemented)

> **Status: Not implemented.** Automatic binding generation from C headers is the goal of
> [RFC-026b](../../rfc/accepted/026-ffi-core-mechanism.md) (an automation sub-proposal of
> this chapter, with no independent document in the repository yet); currently **there is no
> executable entry point**:
>
> - `Cargo.toml` only declares a single binary `[[bin]] yaoxiang-rs`, with no `yx-bindgen`;
> - Writing `[build].headers` in a package manifest will **fail directly** —
>   `[build].headers requires yx-bindgen (RFC-026b, not yet implemented); remove headers or use [binaries] to distribute prebuilt binaries`
>   (`src/package/build/mod.rs:152-159`).
>
> So this section only records **design intent** (the form that should be generated once RFC-026b
> lands), not a currently usable CLI. To do the same thing today, just hand-write the contents of
> Chapters 2 through 4 — the `unsafe {}` type definitions, `native("symbol")` declarations, and
> `[0]` method bindings described in those three chapters are all already implemented.

### 6.1 Planned Generated Content

- FFI type definitions (unsafe block + return)
- FFI function declarations (native syntax)
- Method bindings ([0] syntax)

### 6.2 Planned Invocation Form and Hand-Written Equivalent

```bash
# Planned, not yet implemented:
# yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

The generated output is equivalent to hand-written:

```
// sqlite3_bindings.yx (currently must be hand-written)
// Auto-generated, do not edit manually

// ============================================================================
// Type Definitions
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
// FFI Function Declarations
// ============================================================================

sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
sqlite3_prepare_v2: (db: SqliteDb, sql: String) -> SqliteStmt = native("sqlite3_prepare_v2")
sqlite3_step: (stmt: SqliteStmt) -> Int32 = native("sqlite3_step")
sqlite3_finalize: (stmt: SqliteStmt) -> Int32 = native("sqlite3_finalize")

// ============================================================================
// Method Bindings
// ============================================================================

// Constructor (regular function)
SqliteDb.open = sqlite3_open

// Methods (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// Methods of SqliteStmt
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

### A.4 Invocation

```yaoxiang
// Create via the constructor
db = SqliteDb.open("test.db")

// Through method calls
db.close()
db.exec("SELECT * FROM users")
```
