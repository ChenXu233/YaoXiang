---
title: "RFC-026: FFI Core Mechanisms"
status: "Accepted"
author: "Chenxu"
created: "2026-07-03"
updated: "2026-07-05"
issue: "#93"
---

# RFC-026: FFI Core Mechanisms

> **References**:
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](./024-concurrency-model.md)

> **Deprecated**:
> - [RFC-020: Dynamic Modules and FFI Integration](../deprecated/020-dynamic-modules-ffi.md) — Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../deprecated/021-library-driven-ffi-extension.md) — Content merged into this document

> **Sub-RFCs**:
> - [RFC-026a: Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md) — Multi-ABI mechanism plugins, `FfiMechanism` abstraction, dynamic loading
> - [RFC-026b: yx-bindgen Toolchain](../draft/026b-yx-bindgen.md) — C header → `.yx` binding code generation

## Summary

This document defines the core FFI (Foreign Function Interface) mechanisms of YaoXiang. The core idea: **External libraries are first-class values linked at compile-time, the memory layout ownership of cross-boundary data is pinned at type definition, and YaoXiang's heap objects are structurally isolated from external code.**

1. **External library as value**: `Native.c("libsqlite3")` links the library at compile-time and returns a resolver, carrying library information via currying
2. **External symbol as value**: Applying a symbol name to the resolver yields an external reference, bound as a type or function via `name: type = value` (RFC-007/010)
3. **Type dichotomy**: Opaque handle (layout owned by external) / Transparent type (layout owned by YaoXiang), no third category
4. **Marshalling isolation**: Cross-boundary data is copied to a call temporary region by default; YaoXiang heap objects are isolated from external code
5. **Ownership safety**: Unique ownership of handles (Move) + RAII, structurally preventing double-free and use-after-free
6. **Escape hatch**: `*T` raw pointer + `unsafe {}`, where users explicitly accept the risk of zero-copy direct memory access

**Core boundaries — five inviolable contracts**:

```
1. Libraries are linked at compile-time, symbol existence is verified at compile-time
2. Type layout ownership is determined at definition: opaque handles belong to external, transparent types belong to YaoXiang
3. Default marshalling uses temporary region copying; YaoXiang heap objects are isolated from external code
4. Unique ownership of handles + Move, structurally preventing double-free/dangling pointers
5. External code always reads/writes memory with "explicit layout, explicit ownership"; no ambiguous zone exists
```

---

## Motivation

### Current Status and Goals

The codebase's current `native("symbol")` is merely a dispatch mechanism for YaoXiang bytecode calling Rust std functions (`FfiRegistry` = `HashMap<String, RustFnPtr>`), **with no real cross-ABI boundary** — no dlopen, no C ABI marshalling, no memory ownership crossing boundaries.

A real FFI must solve four problems:

| Problem | This RFC's Answer |
|------|--------------|
| **Symbol resolution** | Library is a first-class value linked at compile-time (`Native.c("lib")`), symbol verified at compile-time |
| **Value marshalling** | Signature-driven, conversion rules determined at compile-time for each parameter position |
| **Memory ownership** | Type dichotomy determines ownership; default copy isolation |
| **Lifecycle safety** | Move + RAII + borrowing limited to single call |

RFC-020 and RFC-021 each define different aspects of FFI, with overlap. This document unifies them into a single specification.

### Design Goals

1. **Zero raw pointer leakage into user code**: In routine FFI usage, no raw pointers appear in `.yx` source code
2. **Explicit layout ownership**: When users define a type, they decide who owns that memory, not inferred at runtime
3. **Structural safety**: No leaks, no double-free, no use-after-free guaranteed by the type system, not by convention
4. **Honest trust boundary**: C cannot provide compile-time verifiable type contracts; trust is localized at the binding declaration
5. **Self-hosting compatible**: No excessive abstraction unique to the host language

### Out of Scope

- **Multi-ABI mechanism plugin system** (Wasm/Python/custom ABI): see RFC-026a
- **yx-bindgen toolchain**: see RFC-026b
- **YaoXiang exporting functions for C to call (reverse FFI)**: follow-up RFC; this document only states principles
- **Inline assembly, SIMD intrinsics**: not in this RFC's scope

---

## Proposal

### 1. External Libraries and Symbols: First-Class Values via Currying

The information gap in FFI — "which library, which symbol to link" — is filled by making the library a first-class value, introducing no new keywords.

#### 1.1 Library as Value

```yaoxiang
// Native.c applies library name → links library at compile-time, returns a symbol resolver
sqlite3 = Native.c("libsqlite3")
```

`Native.c("libsqlite3")` is a **compile-time action + runtime value**:

- **Compile-time**: Linker `-lsqlite3`, library enters the symbol table, symbol existence can be verified
- **Value**: `sqlite3` is a resolver; applying a symbol name yields that library's external reference

`.c` is the ABI mechanism tag (C ABI). The core only built-in supports `.c`; other mechanisms (`.wasm` etc.) are covered in RFC-026a.

#### 1.2 Symbol as Value, Binding as `name: type = value`

Applying a symbol name to the resolver yields an external reference, bound via the unified syntax from RFC-007/010. **The type annotation on the LHS determines whether this reference is a type or a function**:

```yaoxiang
sqlite3 = Native.c("libsqlite3")

// LHS is Type → bound as opaque type
SqliteDb: Type = sqlite3("sqlite3")

// LHS is function signature → bound as function
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// .drop is a normal method binding (RFC-009 RAII convention)
SqliteDb.drop = SqliteDb.close
```

Compile-time verification: `sqlite3` in `sqlite3("sqlite3_open")` must exist in the `libsqlite3` symbol table, otherwise a compile error occurs.

#### 1.3 Method Binding and self Position

In the `Type.method: (...) -> ...` syntax, `self` is implicitly at the first position — when `db.exec("SELECT")` is called, `db` is passed as the 0th parameter to C function `sqlite3_exec`.

When you need to bind a declared standalone function as a method, use `[N]` syntax to specify the self position (RFC-004 currying multi-position binding):

```yaoxiang
// Standalone function
sqlite3_close_v2: (db: SqliteDb) -> Int32 = sqlite3("sqlite3_close_v2")

// Bind as method, [0] means db is self
SqliteDb.soft_close = sqlite3_close_v2[0]
```

`Native.c(...)` direct method binding and `[N]` manual binding are both `name: type = value`, both place a function value on the right side of `=`, with no two sets of mechanisms.

#### 1.4 User Experience: Zero unsafe, Zero Raw Pointers

```yaoxiang
import sqlite3_bindings

db = SqliteDb.open("test.db")
db.exec("SELECT * FROM users")
// ← End of scope, RAII automatically calls SqliteDb.drop → sqlite3_close(db)
```

---

### 2. Type Dichotomy: Layout Ownership Pinned at Definition

When external data enters YaoXiang, only one question is asked: **Whose definition governs this memory's layout?**

```
├─ Layout is an external black box (sqlite3, FILE*, socket fd)
│   → Opaque handle  =  lib("symbol")
│   → YaoXiang only holds a pointer, never dereferences, only passes between library functions
│   → External code reads its own memory; YaoXiang does not touch
│
└─ Layout is defined by YaoXiang (timespec, point, struct with readable fields)
    → Transparent type  =  { field: Type, ... }
    → YaoXiang owns the memory, defines the layout, reads/writes fields
    → External code fills/reads memory whose layout YaoXiang has defined
```

**There is no third category.** The "three-tier memory mode (copy/takeover/system-level)" in previous designs is patch-thinking — the truth is the dichotomy of layout ownership.

#### 2.1 Opaque Handle: Layout Owned by External

```yaoxiang
SqliteDb: Type = sqlite3("sqlite3")
```

- Internally, YaoXiang only holds a pointer-sized handle
- Users cannot construct (`SqliteDb {}` → compile error), cannot access fields (no fields to access)
- The only source: external functions returning `SqliteDb`
- When calling a method, the handle is borrowed back to the library, which reads **its own** memory (the `sqlite3` struct on the library's heap)

When external code "reads inside", it reads the structure it allocated; YaoXiang just shuttles the handle. No memory conflicts.

#### 2.2 Transparent Type: Layout Owned by YaoXiang

```yaoxiang
// Fields are meaningful and need to be read/written → transparent type, layout declared by YaoXiang
Timespec: Type = {
    tv_sec: Int64,
    tv_nsec: Int64
}
clock_gettime: (clk: Int32, ts: *Timespec) -> Int32 = Native.c("librt")("clock_gettime")

ts = clock_gettime(CLOCK_REALTIME)   // See §3, marshalling uses temporary region
print(ts.tv_sec)                      // YaoXiang reads according to its own field definition
```

External code reads/writes memory that **YaoXiang defined the layout for and YaoXiang owns**. The layout is YaoXiang's contract, not external's.

#### 2.3 Decision Rule

Users only need to judge one thing: **Do I need to read this type's fields?**

| Judgment | Type | Layout Owner |
|------|------|---------|
| Don't read fields, only pass handle between library functions | Opaque handle `= lib("sym")` | External |
| Need to read/write fields | Transparent type `{ ... }` | YaoXiang |

---

### 3. Marshalling: Signature-Driven, Temporary Region Isolation

Cross-boundary data conversion is **signature-driven**, with conversion rules determined at compile-time for each parameter position. **Core safety guarantee: external code reads/writes the marshalling temporary region, not YaoXiang's heap objects.**

#### 3.1 Default Goes Through Temporary Region Copy

```
YaoXiang → C (input):
    Copy data to call temporary region → pass temporary region pointer to C
    → C out-of-bounds/writes wrong only damage the temporary region; YaoXiang heap objects isolated

C → YaoXiang (return/output parameters):
    C writes temporary region → YaoXiang memcpy back to its own object
    → C cannot touch YaoXiang's final object
```

**External code always reads/writes the marshalling temporary region, completely isolated from YaoXiang's heap objects.** Layout misdeclaration, C storing dangling pointers, C out-of-bounds — all only damage the temporary region, YaoXiang objects are intact. The cost is one memcpy.

#### 3.2 Marshalling Rules Table

**Input direction (YaoXiang → C)**:

| YaoXiang Type | C Representation | Marshalling Action | Ownership |
|--------------|--------|---------|--------|
| `Int32/Int64/Float` | `int/long/double` | Direct register placement, zero conversion | Value semantics |
| `String` | `const char*` | Lend read-only view (temporary, valid during call) | YaoXiang retains, C reads only |
| Transparent type | `struct T*` | Copy to temporary region, pass temporary region pointer | YaoXiang owns object, C reads copy |
| Opaque handle | `void*` | Extract internal handle pointer | YaoXiang holds, lends to C |
| `*T` | `T*` | Pass raw pointer directly (unsafe) | User responsible |

**Return direction (C → YaoXiang)**:

| C Returns | YaoXiang Type | Marshalling Action | Ownership |
|--------|--------------|---------|--------|
| `int/double` | `Int32/Float` | Read register directly | Value semantics |
| `char*` | `String` | strlen + memcpy to YaoXiang String | YaoXiang owns copy, original memory untouched |
| `struct T*` (new handle) | Opaque handle | Store handle into YaoXiang object | YaoXiang takes over |
| `struct T` (value/output parameter) | Transparent type | C writes temporary region → memcpy back to YaoXiang | YaoXiang owns |
| `char*` (static region) | `*const U8` | Store raw pointer, no copy (unsafe read) | No takeover, user responsible |

#### 3.3 Borrowing Lifecycle: Strictly Limited to Single Call

Pointers YaoXiang lends to external code (String read-only view, transparent type temporary region, handles) have a **lifecycle strictly limited to within a single call**:

- During call: Pointer valid, external code can read/write
- After call returns: Borrow immediately invalid

If external code stores the pointer and uses it after the call returns, external code has violated the FFI standard contract (equivalent to a library bug); YaoXiang is not responsible for this. This is consistent with the C FFI contract in all languages (Rust's `&T` passed to C has the same constraint).

#### 3.4 String Never Gives Out Persistent Pointers

`String` is the key to "C doesn't touch YaoXiang memory":

- Going into C: Lend a **temporary read-only view**, valid during call
- Coming out of C: strlen + memcpy into a **copy** owned by YaoXiang

C can never obtain a persistent pointer to a YaoXiang String, and YaoXiang never holds a long-term reference to a C `char*`. Structurally isolated.

---

### 4. Ownership and Lifecycle: Move + RAII

Opaque handles follow RFC-009's ownership model, with zero new concepts.

#### 4.1 Core Principles

- **Move semantics**: Opaque handles are Move by default; assignment/parameter passing/return = ownership transfer, not copyable
- **Unique handle ownership**: At any moment, a handle has only one owner → structurally prevents double-free
- **RAII release**: When scope ends, if `.drop` is bound, automatically called
- **Consumption tracking**: After explicit destruction or Move, variable is consumed and cannot be used again → prevents use-after-free

#### 4.2 `.drop` is an Optional External Side Effect

```yaoxiang
SqliteDb.drop = SqliteDb.close     // Calls sqlite3_close at end of scope
```

**`.drop` is not a mechanism to prevent YaoXiang leaks** — YaoXiang-side handle storage (a pointer-sized value) is automatically reclaimed, independent of `.drop`. `.drop` is an **optional side effect of calling an external function at the end of scope**:

- Bound `.drop` → Called at scope end (cleaning up external resources)
- No `.drop` bound → Does nothing, **no error, no warning**

Whether external resources need cleanup is a matter of the external library's specification (`getenv` returns static region that shouldn't be freed, global singletons shouldn't be freed); YaoXiang does not overstep to enforce. Leak prevention relies on Move + unique ownership (unconditional, structural), not `.drop`.

#### 4.3 Automatic Destruction and Order

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    // ← End of scope, automatic destruction in reverse order (only if .drop is bound):
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

Destruction order: reverse of definition order, consistent with RAII.

#### 4.4 Move and Consumption

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transfer
db.exec("...")          // ❌ Compile error: db has been Moved, cannot read after consumption

process_db: (db: SqliteDb) -> Void = {
    db.exec("...")
    // ← End of function, db destructed here
}
process_db(some_db)     // Move into function
// some_db is invalid here
```

#### 4.5 Null Handling

```yaoxiang
// May return null → ?T, user must handle
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => db.exec("SELECT 1"),
    None => print("Failed to open")
}

// Convention: won't return null → not marked, panic on null to expose issue
```

C returning null is either handled by the user (`?T`), or panics to expose the issue. There is no third "silently ignore" option.

#### 4.6 Destruction Failure Handling

The return value of the function bound to `.drop` determines behavior:

| `.drop` Return Type | Behavior |
|----------------|------|
| `Void` | No failure |
| `Int32` (error code) | Non-zero causes panic — destruction failure means state is abnormal, expose is better than silent |
| `?Error` | Non-None causes panic — same as above |

Destruction failure cannot be silent. To ignore specific errors, handle explicitly in the wrapper function bound to `.drop`.

---

### 5. FFI Behavior in spawn Blocks

Resource type determination is based on whether `.drop` is bound (RFC-024), with zero additional markers:

| Determination | Behavior |
|------|------|
| Opaque handle with `.drop` bound | Resource type — operations on same instance in spawn blocks automatically serialized |
| Opaque handle without `.drop` bound | Non-resource type — can be parallel (pure data handle, no release side effect) |
| Transparent type / value type | Non-resource type — can be parallel |

```yaoxiang
SqliteDb.drop = SqliteDb.close   // → Resource type

(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Same instance, automatically serialized
    r2 = db.exec("INSERT ...")    // Wait for r1
}

(x, y) = spawn {
    db1 = SqliteDb.open("a.db"),   // Different instances, can be parallel
    db2 = SqliteDb.open("b.db")
}
```

Types with `.drop` automatically serialize same-instance operations in spawn, ensuring destruction has no concurrent races.

---

### 6. Escape Hatch: Raw Pointer + unsafe

Default marshalling goes through temporary region copy, safe but with memcpy overhead. For performance-sensitive scenarios (large structures, high-frequency calls) where zero-copy is needed, users explicitly use the raw pointer escape hatch:

```yaoxiang
// C directly reads YaoXiang memory, zero-copy — user explicitly accepts risk
ptr: *const U8 = Native.c("libc")("getenv")("HOME")
unsafe {
    value = read_c_string(ptr)   // User guarantees ptr is valid
}
```

**`unsafe` is only used for raw pointer operations, completely orthogonal to opaque handles and transparent types.** Routine FFI (handles + transparent types) does not require unsafe. Writing `unsafe {}` = user explicitly signs off accepting the risk of direct memory access.

**Trust boundary**: C cannot provide compile-time verifiable type contracts (`.h` is not an ABI contract, symbol tables only have names, no signatures). Therefore, the correctness of C signatures cannot be automatically verified — the binding author guarantees it when writing `Native.c(...)` + signature. **Trust is localized at the binding declaration**: the binding author guarantees, package users get a safe API. This is consistent with Rust's `extern "C"` (writing extern is a trust act; after wrapping in a safe wrapper, calls are safe).

---

## Trade-offs

### Advantages

1. **Complete information**: Library linked at compile-time, symbol verified at compile-time, no runtime "library not found" ambiguity
2. **Explicit layout ownership**: Type dichotomy, pinned at definition, no runtime inference
3. **Structural safety**: Temporary region isolation + Move + RAII, external code cannot touch YaoXiang heap objects
4. **Zero new keywords**: `Native.c` currying + `name: type = value`, fully reuses existing syntax
5. **Honest boundary**: Doesn't pretend to verify C signatures, localizes trust at the declaration

### Disadvantages

1. **memcpy overhead**: Default marshalling copies, large structures with high-frequency calls require explicit escape hatch
2. **Layout guarantee is manual**: Transparent type layout matching C struct is guaranteed by binding author/yx-bindgen
3. **C signature cannot be compile-time verified**: Fundamental limitation of FFI, YaoXiang cannot eliminate

---

## Implementation Strategy

### Phase 1: External Library and Symbol (v0.8)

- [ ] Implement `Native.c("lib")` compile-time link + return resolver value
- [ ] Implement symbol resolver application (`lib("symbol")`) + compile-time symbol table verification
- [ ] Implement type dichotomy (opaque handle / transparent type)
- [ ] Implement method binding (direct binding + `[N]` position binding)

### Phase 2: Marshalling and Safety (v0.8)

- [ ] Implement signature-driven marshalling code generation
- [ ] Implement temporary region copy isolation (input copy, return memcpy)
- [ ] Implement String temporary read-only view + return copy
- [ ] Implement borrowing lifecycle limited to single call

### Phase 3: Ownership and Lifecycle (v0.9)

- [ ] Implement opaque handle Move + unique ownership
- [ ] Implement `.drop` RAII automatic destruction (optional, no error if missing)
- [ ] Implement consumption tracking (disabled after Move)
- [ ] Implement `?T` and null return integration
- [ ] Implement spawn resource type serialization

### Future Work

- **Extensible FFI Mechanism** (RFC-026a): `FfiMechanism` abstraction, `.wasm`/`.python` etc. plugins, dynamic loading
- **yx-bindgen** (RFC-026b): C header → `.yx` bindings + platform-correct layout generation

---

## Relationship to Other RFCs

- **RFC-004**: Currying multi-position binding — source of `[N]` method binding syntax
- **RFC-007**: Unified function definition syntax — `Native.c(...)` binding is `name: type = value`
- **RFC-009**: Ownership model — Move, RAII, `?T`, handle lifecycle is entirely based on this
- **RFC-010**: Unified type syntax — LHS type annotation determines whether binding is type or function
- **RFC-024**: Concurrency model — resource type determination in spawn based on `.drop`
- **RFC-020/021** (deprecated): Content merged into this document
- **RFC-026a**: Extensible FFI mechanism system
- **RFC-026b**: yx-bindgen toolchain

---

## Design Decision Record

| Decision | Determination | Reason | Date |
|------|------|------|------|
| **Library as value** | `Native.c("lib")` currying returns resolver | Library information becomes a compile-time visible first-class value, filling the "which library to link" gap, zero new keywords | 2026-07-03 |
| **Compile-time link** | `Native.c("lib")` triggers `-llib` | Symbol table readable at compile-time, symbol existence verifiable, type is real | 2026-07-03 |
| **Type dichotomy** | Opaque handle / transparent type | Layout ownership dichotomy covers everything; deleted "three-tier memory mode" patch | 2026-07-03 |
| **Marshalling temporary region isolation** | Default copy, heap object isolated from external | External out-of-bounds/dangling only damage temporary region, YaoXiang object intact; zero-copy requires explicit escape hatch | 2026-07-03 |
| **`.drop` optional** | Missing means does nothing, no error | YaoXiang handle storage auto-reclaimed; external resource cleanup is external specification, not overstepping to enforce | 2026-07-03 |
| **Leak prevention mechanism** | Move + handle unique ownership (unconditional) | Structural guarantee, independent of `.drop` | 2026-07-03 |
| **Trust boundary** | At `Native.c(...)` declaration | C signature cannot be compile-time verified, trust localized, unsafe only for raw pointers | 2026-07-03 |
| **Null handling** | `?T` or panic | C's problems are not hidden, no "silently ignore" option | 2026-07-03 |
| **Destruction failure** | `.drop` return type determines, unified panic | Destruction failure cannot be silent | 2026-07-03 |

---

## References

### YaoXiang Official Documentation

- [RFC-004 Currying Multi-Position Binding](./004-curry-multi-position-binding.md)
- [RFC-007 Unified Function Definition Syntax](./007-function-syntax-unification.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-024 Concurrency Model](./024-concurrency-model.md)
- [RFC-026a Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md)
- [RFC-026b yx-bindgen Toolchain](../draft/026b-yx-bindgen.md)

### External References

- [Rust FFI (Nomicon)](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [LuaJIT FFI](https://luajit.org/ext_ffi.html)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |