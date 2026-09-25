---
title: FFI Specification
---

# FFI Specification

This document defines the FFI (Foreign Function Interface) specification for the YaoXiang
programming language, including type definitions, function declarations, method bindings, and opaque
type handling.

> **Detailed Design**: For the complete FFI design, motivation, and trade-offs, see
> [RFC-026: FFI Core Mechanism](../../design/rfc/accepted/026-ffi-core-mechanism.md).

---

## Chapter 1: Overview

### 1.1 Core Principles of FFI

```
All returns inside {} return content to the parent scope
Default no return returns Void
```

### 1.2 Components of FFI

| Component            | Description                              | Syntax                 |
| -------------------- | ---------------------------------------- | ---------------------- |
| Type Definition      | Define FFI types (opaque or transparent) | `unsafe {}` + `return` |
| Function Declaration | Declare external functions               | `native("symbol")`     |
| Method Binding       | Bind methods to types                    | `[0]` syntax           |

---

## Chapter 2: FFI Type Definition

### 2.1 Opaque Types

Opaque types are defined inside `unsafe {}` blocks and returned to the parent scope via `return`:

```yaoxiang
// In an unsafe block, define an opaque type
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is accessible outside unsafe block
db = sqlite3_open("test.db")

// ❌ Compile error: the `handle` field requires unsafe permission
handle = db.handle

// ✅ Via method call
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

### 2.3 Opaque Type Detection

The compiler automatically distinguishes opaque types and vacuum types:

```yaoxiang
// Opaque type (referenced by native function)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb is referenced by native function → Opaque type

// Vacuum type (not referenced by native function)
MyType: Type = {}
// → MyType is not referenced by native function → Vacuum type
```

**Detection Rules**:

- If a type is referenced by a `native` function → Opaque type
- Otherwise → Vacuum type

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

FFI function parameter types directly use YaoXiang types, and the compiler automatically handles C
type mapping:

| C Type               | YaoXiang Type          |
| -------------------- | ---------------------- |
| `int`                | `Int32`                |
| `long`               | `Int64`                |
| `float`              | `Float32`              |
| `double`             | `Float64`              |
| `char`               | `Char`                 |
| `char*`              | `String`               |
| `bool`               | `Bool`                 |
| `size_t`             | `Uint`                 |
| `void*`              | `*Void`                |
| `struct T*`          | `T` (transparent type) |
| `typedef struct T T` | `T` (opaque type)      |

### 3.3 Return Types

FFI function return types directly use YaoXiang types:

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

Use the `[0]` syntax to specify the position of the `self` parameter within the function's parameter
tuple:

```yaoxiang
// FFI functions
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Method binding (self is at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Invocation**:

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

// Constructor binding (ordinary function)
SqliteDb.open = sqlite3_open
```

**Invocation**:

```yaoxiang
// Create via constructor
db = SqliteDb.open("test.db")
```

### 4.3 Binding Location

Method bindings can appear anywhere, because types are data containers:

```yaoxiang
// Bound after the type definition
SqliteDb.close = sqlite3_close[0]

// Bound in another file
SqliteDb.exec = sqlite3_exec[0]

// Compiler will ultimately check both
```

---

## Chapter 5: FFI Behavior in `spawn` Blocks

### 5.1 Automatic Serialization of Resource Types

If an FFI type is a resource type, it is automatically serialized within a `spawn` block:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // different instances, can parallelize
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // same SqliteDb
    result2 = db.exec("INSERT ...")   // automatically serialized
}
```

### 5.2 Non-Resource Types Can Parallelize

If an FFI type is not a resource type, it can parallelize in spawn blocks:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // can parallelize
    result2 = cos(1.0)   // can parallelize
}
```

---

## Chapter 6: The `yx-bindgen` Toolchain

### 6.1 Generated Content

`yx-bindgen` generates the following:

- FFI type definitions (`unsafe` block + `return`)
- FFI function declarations (`native` syntax)
- Method bindings (`[0]` syntax)

### 6.2 Generation Example

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

Generated result:

```yaoxiang
// sqlite3_bindings.yx
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

// Constructor (ordinary function)
SqliteDb.open = sqlite3_open

// Method (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// Methods on SqliteStmt
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## Appendix: FFI Syntax Cheat Sheet

### A.1 Type Definitions

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

### A.2 Function Declarations

```yaoxiang
// FFI function declaration
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 Method Bindings

```yaoxiang
// Constructor (ordinary function)
SqliteDb.open = sqlite3_open

// Method (self at position 0)
SqliteDb.close = sqlite3_close[0]
```

### A.4 Invocation

```yaoxiang
// Create via constructor
db = SqliteDb.open("test.db")

// Call via method
db.close()
db.exec("SELECT * FROM users")
```
