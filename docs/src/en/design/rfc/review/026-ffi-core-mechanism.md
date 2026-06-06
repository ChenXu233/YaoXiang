```markdown
---
title: "RFC-026: FFI Core Mechanism"
status: "Under Review"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-05"
---

# RFC-026: FFI Core Mechanism

> **Reference**:
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](./024-concurrency-model.md)

> **Superseded**:
> - [RFC-020: Dynamic Modules and FFI Integration](./020-dynamic-modules-ffi.md) — Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./021-library-driven-ffi-extension.md) — Content merged into this document

## Summary

This document defines YaoXiang's FFI (Foreign Function Interface) core mechanism, including:

1. **FFI Type Definition**: Define opaque types using `unsafe {}` blocks, returning them to the outer scope via `return`
2. **FFI Function Declaration**: Declare external functions using `native("symbol")` syntax
3. **Method Binding**: Use `[0]` syntax to specify self parameter position
4. **Opaque Type Handling**: Compiler automatically determines opaque types and void types
5. **FFI Behavior in spawn Blocks**: Resource types are automatically serialized, non-resource types can be parallelized

**Core Design — One Principle, Unified Semantics**:

```
All `return` in `{}` blocks returns content to the outer scope
No return defaults to returning Void
```

---

## Motivation

### Why is this design needed?

RFC-020 and RFC-021 define different aspects of FFI separately:
- RFC-020: Dynamic modules and FFI integration
- RFC-021: Library-driven FFI extension

Both have overlaps and need to be integrated into a unified FFI specification.

### Design Goals

1. **Unification**: Consistent `return` semantics for all `{}` blocks
2. **Safety**: Field access for opaque types requires unsafe permission
3. **Simplicity**: No new keywords or special markers needed
4. **Practicality**: yx-bindgen auto-generates bindings

---

## Proposal

### 1. FFI Type Definition

#### 1.1 unsafe Block + return Semantics

Define opaque types within unsafe blocks, returning them to the outer scope via return:

```yaoxiang
// Define opaque type in unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")

// ❌ Compile error: handle field requires unsafe permission
handle = db.handle

// ✅ Via method call
db.close()
```

#### 1.2 Transparent Types

Transparent types are defined directly without unsafe blocks:

```yaoxiang
// Transparent type
Point: Type = {
    x: Int32,
    y: Int32
}

// Users can create directly
p: Point = Point { x: 1, y: 2 }
```

#### 1.3 Opaque Type Determination

The compiler automatically determines opaque types and void types:

```yaoxiang
// Opaque type (referenced by native function)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb is referenced by native function → Opaque type

// Void type (not referenced by native function)
MyType: Type = {}
// → MyType is not referenced by native function → Void type
```

**Determination Rules**:
- If a type is referenced by a `native` function → Opaque type
- Otherwise → Void type

---

### 2. FFI Function Declaration

#### 2.1 native Syntax

Declare external functions using `native("symbol")` syntax:

```yaoxiang
// FFI function declaration
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

#### 2.2 Parameter Type Mapping

FFI function parameter types directly use YaoXiang types, and the compiler automatically handles C type mapping:

| C Type | YaoXiang Type |
|--------|---------------|
| `int` | `Int32` |
| `long` | `Int64` |
| `float` | `Float32` |
| `double` | `Float64` |
| `char` | `Char` |
| `char*` | `String` |
| `bool` | `Bool` |
| `size_t` | `Uint` |
| `void*` | `*Void` |
| `struct T*` | `T` (transparent type) |
| `typedef struct T T` | `T` (opaque type) |

#### 2.3 Return Type

FFI function return types directly use YaoXiang types:

```yaoxiang
// Return opaque type
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Return transparent type
get_point: () -> Point = native("get_point")

// Return primitive type
get_value: () -> Int32 = native("get_value")
```

---

### 3. Method Binding

#### 3.1 [0] Syntax

Use `[0]` syntax to specify the position of the self parameter in the function parameter tuple:

```yaoxiang
// FFI function
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Method binding (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Invocation Style**:
```yaoxiang
db = sqlite3_open("test.db")

// Method call
db.close()  // Equivalent to sqlite3_close(db)
db.exec("SELECT * FROM users")  // Equivalent to sqlite3_exec(db, "SELECT * FROM users")
```

#### 3.2 Constructor Binding

Constructors do not have `[0]`, bound as regular functions:

```yaoxiang
// FFI function
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Constructor binding (regular function)
SqliteDb.open = sqlite3_open
```

**Invocation Style**:
```yaoxiang
// Create via constructor
db = SqliteDb.open("test.db")
```

#### 3.3 Binding Location

Method bindings can be at any location because types are data containers:

```yaoxiang
// Bind after type definition
SqliteDb.close = sqlite3_close[0]

// Bind in other files
SqliteDb.exec = sqlite3_exec[0]

// Compiler will check all in the end
```

---

### 4. Opaque Type Handling

#### 4.1 Compiler Automatic Handling

The compiler automatically determines opaque types and internally handles C pointer storage:

```
Compiler analysis:
    SqliteDb: Type = {}
    sqlite3_open: ... -> SqliteDb = native("sqlite3_open")

Inference:
    SqliteDb is an opaque type
    Internally automatically adds @internal handle: *Void
    Users prohibited from creating directly
```

#### 4.2 User Code

```yaoxiang
import sqlite3_bindings

// ✅ Create via constructor
db = SqliteDb.open("test.db")

// ❌ Compile error: Cannot directly create opaque type
db: SqliteDb = {}

// ✅ Via method call
result = db.exec("SELECT * FROM users")
db.close()
```

---

### 5. FFI Behavior in spawn Blocks

#### 5.1 Resource Types Automatically Serialized

If an FFI type is a resource type, it is automatically serialized in spawn blocks:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // Different instances, can be parallelized
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // Same SqliteDb
    result2 = db.exec("INSERT ...")   // Automatically serialized
}
```

#### 5.2 Non-Resource Types Can Be Parallelized

If an FFI type is not a resource type, it can be parallelized in spawn blocks:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // Can be parallelized
    result2 = cos(1.0)   // Can be parallelized
}
```

---

### 6. yx-bindgen Toolchain

#### 6.1 Generated Content

yx-bindgen generates the following:
- FFI type definitions (unsafe block + return)
- FFI function declarations (native syntax)
- Method bindings ([0] syntax)

#### 6.2 Generation Example

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

// Constructor (regular function)
SqliteDb.open = sqlite3_open

// Method (self at position 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// SqliteStmt methods
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## Tradeoffs

### Advantages

1. **Unified Semantics**: Consistent `return` semantics for all `{}` blocks
2. **No New Keywords**: Uses existing unsafe and return
3. **No Special Markers**: Compiler automatically determines opaque types
4. **Safety**: Field access for opaque types requires unsafe permission
5. **Practicality**: yx-bindgen auto-generates bindings

### Disadvantages

1. **unsafe Block Scope**: Need to modify `return` semantics of `{}` blocks
2. **Compiler Complexity**: Need to automatically determine opaque types
3. **yx-bindgen Maintenance**: Need ongoing updates to support new C libraries

---

## Implementation Strategy

### Phase 1: Core Mechanism (v0.8)

- [ ] Implement unsafe block `return` semantics
- [ ] Implement FFI type definition
- [ ] Implement FFI function declaration
- [ ] Implement method binding

### Phase 2: Opaque Types (v0.9)

- [ ] Implement compiler automatic opaque type determination
- [ ] Implement internal handle storage
- [ ] Implement direct opaque type creation prohibition

### Phase 3: Toolchain (v1.0)

- [ ] Implement yx-bindgen
- [ ] Support Linux/macOS/Windows
- [ ] Integration testing

---

## Relationship with Other RFCs

- **RFC-008**: Runtime concurrency model, FFI calls as independent tasks
- **RFC-009**: Ownership model, semantics of unsafe blocks
- **RFC-010**: Unified type syntax, `{}` block `return` semantics
- **RFC-024**: Concurrency model, FFI behavior in spawn blocks

---

## Design Decision Records

| Decision | Decision Made | Reason | Date |
|----------|---------------|--------|------|
| FFI Type Definition | unsafe block + return | Unified semantics, no new keywords | 2026-06-05 |
| Opaque Type Determination | Compiler automatic | No special markers needed | 2026-06-05 |
| Method Binding | [0] syntax | Explicit self position | 2026-06-05 |
| Constructor | Regular function binding | No special syntax needed | 2026-06-05 |
| spawn Block Behavior | Resource types automatically serialized | Safety, aligns with concurrency model | 2026-06-05 |

---

## References

### YaoXiang Official Documentation

- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-024 Concurrency Model](./024-concurrency-model.md)

### External References

- [Rust FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading](https://docs.rs/libloading/latest/libloading/)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Under Review** | `docs/design/rfc/` | Open for community discussion |
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |
```