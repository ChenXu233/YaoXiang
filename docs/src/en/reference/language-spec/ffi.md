# FFI Specification

This document defines the FFI (Foreign Function Interface) specification for the YaoXiang programming language, including type definitions, function declarations, method bindings, and opaque type handling.

> **Detailed Design**: The complete design, motivation, and trade-offs of FFI are documented in
> [RFC-026: FFI Core Mechanism](../design/rfc/accepted/026-ffi-core-mechanism.md).

---

## Chapter 1: Overview

### 1.1 Core Principles of FFI

```
All returns inside {} return content to the upper scope
By default, no return means returning Void
```

### 1.2 Components of FFI

| Component      | Description                           | Syntax                   |
| -------------- | ------------------------------------- | ------------------------ |
| Type Definition | Define FFI types (opaque or transparent) | `unsafe {}` + `return` |
| Function Declaration | Declare external functions        | `native("symbol")`       |
| Method Binding  | Bind methods to types                 | `[0]` syntax             |

---

## Chapter 2: FFI Type Definitions

### 2.1 Opaque Types

Opaque types are defined inside `unsafe {}` blocks and returned to the upper scope via `return`:

```yaoxiang
// Define opaque type in unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

// ❌ Compile error: handle field requires unsafe permission
handle = db.handle

// ✅ Through method calls
db.close()
```

### 2.2 Transparent Types

Transparent types are defined directly without `unsafe {}` blocks:

```yaoxiang
// Transparent type
Point: Type = {
    x: Int32,
    y: Int32
}

// Users can create directly
p: Point = Point { x: 1, y: 2 }
```

### 2.3 Opaque Type Determination

The compiler automatically determines opaque and transparent types:

```yaoxiang
// Opaque type (referenced by native functions)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb is referenced by native function → opaque type

// Transparent type (not referenced by native functions)
MyType: Type = {}
// → MyType is not referenced by native function → transparent type
```

**Determination Rules**:

- If a type is referenced by `native` functions → opaque type
- Otherwise → transparent type

---

## Chapter 3: FFI Function Declarations

### 3.1 native Syntax

Use `native("symbol")` syntax to declare external functions:

```yaoxiang
// FFI function declarations
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 Parameter Type Mapping

FFI function parameter types use YaoXiang types directly, and the compiler handles C type mapping automatically:

| C Type               | YaoXiang Type    |
| -------------------- | ----------------- |
| `int`                | `Int32`           |
| `long`               | `Int64`           |
| `float`              | `Float32`         |
| `double`             | `Float64`         |
| `char`               | `Char`            |
| `char*`              | `String`          |
| `bool`               | `Bool`            |
| `size_t`             | `Uint`            |
| `void*`              | `*Void`           |
| `struct T*`          | `T` (transparent type)   |
| `typedef struct T T` | `T` (opaque type) |

### 3.3 Return Types

FFI function return types use YaoXiang types directly:

```yaoxiang
// Return opaque type
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Return transparent type
get_point: () -> Point = native("get_point")

// Return primitive type
get_value: () -> Int32 = native("get_value")
```

---

## Chapter 4: Method Bindings

### 4.1 [0] Syntax

Use `[0]` syntax to specify the position of the self parameter in the function parameter tuple:

```yaoxiang
// FFI functions
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Method bindings (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Calling Methods**:

```yaoxiang
db = sqlite3_open("test.db")

// Method calls
db.close()  // equivalent to sqlite3_close(db)
db.exec("SELECT * FROM users")  // equivalent to sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Constructor Bindings

Constructors do not use `[0]` and are bound as regular functions:

```yaoxiang
// FFI function
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Constructor binding (regular function)
SqliteDb.open = sqlite3_open
```

**Calling Constructors**:

```yaoxiang
// Create through constructor
db = SqliteDb.open("test.db")
```

### 4.3 Binding Locations

Method bindings can be at any location because types are data containers:

```yaoxiang
// Bind after type definition
SqliteDb.close = sqlite3_close[0]

// Bind in other files
SqliteDb.exec = sqlite3_exec[0]

// The compiler will check them all in the end
```

---

## Chapter 5: FFI Behavior in spawn Blocks

### 5.1 Resource Types Automatically Serialized

If an FFI type is a resource type, it is automatically serialized in spawn blocks:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // different instances, can be parallel
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // same SqliteDb
    result2 = db.exec("INSERT ...")   // automatically serialized
}
```

### 5.2 Non-Resource Types Can Be Parallel

If an FFI type is not a resource type, it can be parallel in spawn blocks:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // can be parallel
    result2 = cos(1.0)   // can be parallel
}
```

---

## Chapter 6: yx-bindgen Toolchain

### 6.1 Generated Content

yx-bindgen generates the following:

- FFI type definitions (unsafe blocks + return)
- FFI function declarations (native syntax)
- Method bindings ([0] syntax)

### 6.2 Generation Example

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

Generated output:

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

// Constructors (regular functions)
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
// FFI function declarations
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 Method Bindings

```yaoxiang
// Constructor (regular function)
SqliteDb.open = sqlite3_open

// Method (self at position 0)
SqliteDb.close = sqlite3_close[0]
```

### A.4 Calling Methods

```yaoxiang
// Create through constructor
db = SqliteDb.open("test.db")

// Call through methods
db.close()
db.exec("SELECT * FROM users")
```
