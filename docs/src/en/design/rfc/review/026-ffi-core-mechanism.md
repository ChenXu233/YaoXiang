---
title: "RFC-026: FFI Core Mechanisms"
status: "Under Review"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-06-10"
---

# RFC-026: FFI Core Mechanisms

> **References**:
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-024: spawn Block-Based Concurrency Model](./024-concurrency-model.md)

> **Deprecated**:
> - [RFC-020: Dynamic Modules and FFI Integration](./020-dynamic-modules-ffi.md) — Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Invocation Support](./021-library-driven-ffi-extension.md) — Content merged into this document

## Abstract

This document defines YaoXiang's FFI (Foreign Function Interface) core mechanisms, including:

1. **FFI Type Definition**: Use `unsafe {}` blocks to define opaque types, returning them to the enclosing scope via `return`
2. **FFI Function Declaration**: Use the `native("symbol")` syntax to declare external functions
3. **Method Binding**: Use the `[0]` syntax to specify the position of the self parameter
4. **Opaque Type Handling**: `unsafe {}` blocks explicitly define opaque types; the empty body `Type = {}` denotes a vacuum type
5. **Opaque Type Lifetime**: `.drop` binding for destructors, RAII-based automatic release, Null safety handling
6. **FFI Behavior in spawn Blocks**: Resource types are automatically serialized; non-resource types may execute in parallel

**Core Design—One Principle, Unified Semantics**:

```
All return statements within {} return their content to the enclosing scope
The default absence of a return statement returns Void
```

---

## Motivation

### Why is this design needed?

RFC-020 and RFC-021 each define different aspects of FFI:
- RFC-020: Dynamic Modules and FFI Integration
- RFC-021: Library-Driven FFI Extension

The two overlap and need to be consolidated into a unified FFI specification.

### Design Goals

1. **Unified**: Consistent return semantics across all `{}` blocks
2. **Safe**: Field access on opaque types requires unsafe permission
3. **Concise**: No new keywords or special markers required
4. **Practical**: yx-bindgen automatically generates bindings

---

## Proposal

### 1. FFI Type Definition

#### 1.1 unsafe Block + return Semantics

Define opaque types within an unsafe block and return them to the enclosing scope via `return`:

```yaoxiang
// Define an opaque type within an unsafe block
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

// ✅ Access through method calls
db.close()
```

#### 1.2 Transparent Types

Transparent types are defined directly, without an unsafe block:

```yaoxiang
// Transparent type
Point: Type = {
    x: Int32,
    y: Int32
}

// Users can create instances directly
p: Point = Point { x: 1, y: 2 }
```

#### 1.3 Opaque Types, Transparent Types, and Vacuum Types

The distinction between these three types is determined at **definition time**, without requiring the compiler to perform cross-file inference:

```yaoxiang
// Transparent type: has fields
Point: Type = { x: Int32, y: Int32 }
// Users can create instances and access fields
p: Point = Point { x: 1, y: 2 }

// Vacuum type: empty body, not within an unsafe block
MyMarker: Type = {}
// Zero-sized type, freely creatable
x: MyMarker = {}

// Opaque type: returned from an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
// SqliteDb is an opaque type; cannot be created directly, fields cannot be accessed
```

**Rules**:
- **Has fields** → Transparent type
- **Empty body + not within an unsafe block** → Vacuum type (zero-sized)
- **Returned from an unsafe block** → Opaque type

`native` functions can only reference explicitly defined types and do not change a type's properties. Type properties are determined at definition time, not inferred at use site.

---

### 2. FFI Function Declaration

#### 2.1 native Syntax

Use the `native("symbol")` syntax to declare external functions:

```yaoxiang
// FFI function declarations
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

#### 2.2 Parameter Type Mapping

FFI function parameters use YaoXiang types directly; the compiler automatically handles C type mapping:

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

#### 2.3 Return Types

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

### 3. Method Binding

#### 3.1 [0] Syntax

Use the `[0]` syntax to specify the position of the self parameter in the function's parameter tuple:

```yaoxiang
// FFI functions
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
db.close()  // equivalent to sqlite3_close(db)
db.exec("SELECT * FROM users")  // equivalent to sqlite3_exec(db, "SELECT * FROM users")
```

#### 3.2 Constructor Binding

Constructors do not use `[0]`; they are bound as regular functions:

```yaoxiang
// FFI function
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Constructor binding (regular function)
SqliteDb.open = sqlite3_open
```

**Invocation**:
```yaoxiang
// Create through constructor
db = SqliteDb.open("test.db")
```

#### 3.3 Binding Location

Method bindings can occur anywhere, because types are data containers:

```yaoxiang
// Bind after type definition
SqliteDb.close = sqlite3_close[0]

// Bind in another file
SqliteDb.exec = sqlite3_exec[0]

// The compiler will validate them in the end
```

---

### 4. Opaque Type Handling

#### 4.1 Internal Storage of Opaque Types

The `handle: *Void` field defined within the `unsafe {}` block is managed automatically by the compiler:

```text
Compiler processing:
    SqliteDb = unsafe {
        SqliteDb: Type = {
            handle: *Void         // ← compiler internally stores the C pointer
        }
        return SqliteDb
    }

Result:
    SqliteDb is an opaque type (explicitly defined by the unsafe block)
    Field access requires unsafe permission
    Users cannot create instances directly (must use a native constructor)
```

No reverse inference by the compiler is required—whether a type is opaque is determined by its definition, making the behavior clear and predictable.

#### 4.2 User Code

```yaoxiang
import sqlite3_bindings

// ✅ Create through constructor
db = SqliteDb.open("test.db")

// ❌ Compile error: cannot create an opaque type directly
db: SqliteDb = {}

// ✅ Access through method calls
result = db.exec("SELECT * FROM users")
db.close()
```

---

### 5. FFI Behavior in spawn Blocks

#### 5.1 Resource Types Are Automatically Serialized

If the FFI type is a resource type, spawn blocks automatically serialize it:

```yaoxiang
// SqliteDb is a resource type
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb resource
    db2 = SqliteDb.open("db2.sqlite")   // different instances, can run in parallel
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // same SqliteDb
    result2 = db.exec("INSERT ...")   // automatically serialized
}
```

#### 5.2 Non-Resource Types Can Run in Parallel

If the FFI type is not a resource type, the spawn block can execute in parallel:

```yaoxiang
// Float is not a resource type
(a, b) = spawn {
    result1 = sin(1.0),  // can run in parallel
    result2 = cos(1.0)   // can run in parallel
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
// Auto-generated; do not edit manually

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

// ============================================================================
// Destructors
// ============================================================================

SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

---

### 7. Opaque Type Lifetime Management

Opaque types follow the ownership model from RFC-009, with zero new concepts.

#### 7.1 Core Principles

- **Move Semantics**: Opaque types are Move by default—assignment/parameter passing/return transfers ownership; they are not copyable
- **RAII Release**: Destructors are called automatically when the scope ends
- **Consumption Tracking**: After explicit destruction, the variable is consumed and cannot be used again

#### 7.2 Destructor Binding

Use the `.drop` convention to bind destructors; the syntax is identical to ordinary method binding:

```yaoxiang
// Destructor binding (self at position 0)
SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

The compiler recognizes `.drop` bindings and invokes them automatically when the scope ends. **No new keywords are introduced, and no trait system is needed**—this is simply method binding + RAII, the semantics already promised by RFC-009.

#### 7.3 Automatic Destruction

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    stmt.step()
    // ← Scope ends, automatic destruction in reverse order:
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

**Destruction order**: Reverse of definition order (later-defined is destroyed first), consistent with RAII semantics.

#### 7.4 Explicit Destruction

```yaoxiang
db = SqliteDb.open("test.db")
db.close()              // Explicit destruction. close is drop—whatever name is bound, that name is used
db.exec("...")          // ❌ Compile error: db has been consumed (cannot be read after Move)
```

There is no separate "close vs drop" concept. After `SqliteDb.drop = sqlite3_close[0]`, `db.close()` and `db.drop()` refer to the same function.

#### 7.5 Destruction and Move

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transfers to db2
// db is invalid here
// ← Scope ends, db2.drop() is called automatically

// Function consumption
process_db: (db: SqliteDb) -> Void = {
    result = db.exec("...")
    // ← Function ends, db is destroyed here
}

db = SqliteDb.open("test.db")
process_db(db)          // Move into the function, destroyed at function end
// db is invalid here
```

#### 7.6 Null Handling

```yaoxiang
// May return null → mark with ? for an optional type; user must handle it
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => {
        db.exec("SELECT * FROM users")
        // ← Scope ends, db.drop() is called automatically
    }
    None => print("Failed to open")
}

// Cannot return null → do not mark; panic on null
// Used when the C function is contractually guaranteed never to return null
sqlite3_get_global: () -> SqliteDb = native("sqlite3_get_global")
```

**Design principle**: When C returns null, either the user must handle it (`?`) or it panics to expose the issue. There is no third "silently ignore" option.

#### 7.7 Destructor Failure Handling

```yaoxiang
// Destructor may return an error code
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
SqliteDb.drop = sqlite3_close[0]

// Compiler behavior:
//   debug mode: destructor return value != 0 → panic (expose the problem)
//   release mode: ignore the return value (C standard permits close to fail without affecting memory safety)
```

#### 7.8 Destruction in spawn Blocks

```yaoxiang
// Resource types are automatically serialized in spawn blocks; destruction is inherently safe
{
    db = SqliteDb.open("test.db")
    result = db.exec("...")
}  // ← Serialization guarantee: drop occurs after exec completes, no concurrent races

// Move across spawn boundary
db = SqliteDb.open("test.db")
spawn {
    use(db)             // Move into spawn
    // ← spawn ends, automatic destruction
}
```

#### 7.9 Types Without Destructors

Opaque types are not required to bind `.drop`. Types without a bound destructor do nothing when the scope ends—this applies to scenarios such as wrapping static data or global handles that do not require cleanup.

In debug mode, the compiler emits a lint (warn by default) for opaque types without a `.drop` binding to remind users to confirm.

#### 7.10 Lifetime Rules Summary

| Scenario | Behavior | Source |
|------|------|------|
| Opaque type assignment | Move (non-copyable) | RFC-009 |
| `.drop` binding | Method binding syntax `[0]` | This document §3 |
| Scope end | Reverse-order automatic call to `.drop()` | RFC-009 RAII |
| Explicit `.close()` | Consumes the variable; unusable afterward | RFC-009 Move Semantics |
| Null return | `?T` optional / direct panic | This document §7.6 |
| In spawn blocks | Automatically serialized, destruction is safe | RFC-024 |

---

## Trade-offs

### Advantages

1. **Unified semantics**: Consistent return semantics across all `{}` blocks
2. **No new keywords**: Uses existing `unsafe` and `return`
3. **Explicit definition**: Type properties are determined at definition time—unsafe block return → opaque, empty body → vacuum—no inference required
4. **Zero new concepts for lifetime**: `.drop` = method binding + RAII, no trait system, no new keywords
5. **Safety**: Field access on opaque types requires unsafe permission; variables are unusable after destruction
6. **Practical**: yx-bindgen automatically generates bindings (including destructors)

### Disadvantages

1. **Scope of unsafe blocks**: Requires modifying the return semantics of `{}` blocks
2. **yx-bindgen maintenance**: Requires ongoing updates to support new C libraries

---

## Implementation Strategy

### Phase 1: Core Mechanisms (v0.8)

- [ ] Implement the return semantics of `{}` blocks
- [ ] Implement FFI type definition
- [ ] Implement FFI function declaration
- [ ] Implement method binding

### Phase 2: Lifetime Management (v0.9)

- [ ] Implement `.drop` destructor binding
- [ ] Implement automatic destruction at scope end
- [ ] Implement consumption checks after destruction
- [ ] Implement integration of `?T` with FFI null returns
- [ ] Implement internal handle storage
- [ ] Implement the prohibition on directly creating opaque types

### Phase 3: Toolchain (v1.0)

- [ ] Implement yx-bindgen
- [ ] Support Linux/macOS/Windows
- [ ] Integration testing

---

## Relationship with Other RFCs

- **RFC-008**: Runtime concurrency model; FFI calls execute as independent tasks
- **RFC-009**: Ownership model—Move semantics, RAII, `?` optional types; opaque type lifetime management is entirely built on this
- **RFC-010**: Unified type syntax; `{}` block return semantics
- **RFC-024**: Concurrency model; FFI behavior and destruction safety in spawn blocks

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| FFI type definition | unsafe block + return | Unified semantics, no new keywords | 2026-06-05 |
| Opaque type determination | Explicit definition via unsafe block | Type properties determined at definition time, not relying on external inference | 2026-06-05 |
| Method binding | `[0]` syntax | Explicitly mark self's position | 2026-06-05 |
| Constructor | Regular function binding | No special syntax required | 2026-06-05 |
| spawn block behavior | Resource types automatically serialized | Safety, consistent with concurrency model | 2026-06-05 |
| Destructor | `.drop = native_fn[0]` | Method binding + RAII, zero new concepts | 2026-06-10 |
| Null handling | `?T` optional / direct panic | Do not hide C's problems | 2026-06-10 |

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

## Lifetime and Destination

| Status | Location | Description |
|------|------|------|
| **Under Review** | `docs/design/rfc/` | Open for community discussion |
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |