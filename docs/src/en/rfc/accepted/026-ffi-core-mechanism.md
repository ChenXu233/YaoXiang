---
title: 'RFC-026: FFI Core Mechanism'
status: 'Accepted'
author: '晨煦'
created: '2026-07-03'
updated: '2026-07-05'
issue: '#93'
---

# RFC-026: FFI Core Mechanism

> **References**:
>
> - [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md)
> - [RFC-009: Ownership Model Design](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)

> **Deprecated**:
>
> - [RFC-020: Dynamic Modules and FFI Integration](../deprecated/020-dynamic-modules-ffi.md) —
>   Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../deprecated/021-library-driven-ffi-extension.md)
>   — Content merged into this document

> **Sub-RFCs**:
>
> - [RFC-026a: Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md) — Multi-ABI
>   mechanism plugins, `FfiMechanism` abstraction, dynamic loading
> - [RFC-026b: yx-bindgen Toolchain](../draft/026b-yx-bindgen.md) — C header → `.yx` binding code
>   generation

## Summary

This document defines the FFI (Foreign Function Interface) core mechanism of YaoXiang. The core
idea: **external libraries are first-class values linked at compile-time, the memory layout
ownership of cross-boundary data is pinned at type definition, and YaoXiang's heap objects are
structurally isolated from external code.**

1. **External libraries as values**: `Native.c("libsqlite3")` links the library at compile-time and
   returns a resolver, using currying to carry the library information
2. **External symbols as values**: The resolver, when applied with a symbol name, yields an external
   reference, which is bound as a type or function via `name: type = value` (RFC-007/010)
3. **Type dichotomy**: Opaque handles (layout owned by external) / Transparent types (layout owned
   by YaoXiang), no third kind
4. **Marshaling isolation**: Cross-boundary data is by default copied into a call temporary area;
   YaoXiang heap objects are isolated from external code
5. **Ownership safety**: Handle unique ownership (Move) + RAII, structurally preventing double-free
   and use-after-free
6. **Escape hatch**: `*T` raw pointers + `unsafe {}`, the user explicitly accepts the risk of
   zero-copy direct memory access

**Core boundary — five inviolable contracts**:

```
1. Libraries are linked at compile-time; symbol existence is verified at compile-time
2. Type layout ownership is determined at definition: opaque handles belong to external, transparent types belong to YaoXiang
3. Marshaling goes through temporary area copying by default; YaoXiang heap objects are isolated from external code
4. Handle unique ownership + Move, structurally preventing double-free/dangling
5. External code always reads/writes "layout-explicit, ownership-explicit" memory; no ambiguous zone exists
```

---

## Motivation

### Current State and Goals

The current codebase's `native("symbol")` is only a dispatch mechanism for YaoXiang bytecode to call
Rust std functions (`FfiRegistry` = `HashMap<String, RustFnPtr>`), **with no real cross-ABI
boundary** — no dlopen, no C ABI marshaling, no memory ownership crossing.

True FFI must solve four problems:

| Problem               | This RFC's Answer                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------------------------- |
| **Symbol resolution** | Libraries are first-class values linked at compile-time (`Native.c("lib")`), symbols verified at compile-time |
| **Value marshaling**  | Signature-driven; conversion rules determined for each parameter position at compile-time                     |
| **Memory ownership**  | Type dichotomy decides ownership; default copy isolation                                                      |
| **Lifetime safety**   | Move + RAII + borrow limited to a single call                                                                 |

RFC-020 and RFC-021 define different aspects of FFI respectively, with overlaps between them; this
document integrates them into a unified specification.

### Design Goals

1. **Zero raw pointer leakage into user code**: In normal FFI usage, raw pointers do not appear in
   `.yx` source code
2. **Explicit layout ownership**: When users define a type, they decide who owns this block of
   memory, no runtime inference needed
3. **Structural safety**: No leaks, no double-free, no use-after-free is guaranteed by the type
   system, not by convention
4. **Honest trust boundary**: C cannot provide compile-time verifiable type contracts; trust is
   localized at the binding declaration
5. **Self-hosting compatibility**: No excessive abstractions unique to the host language

### Out of Scope

- **Multi-ABI mechanism plugin system** (Wasm/Python/custom ABI): See RFC-026a
- **yx-bindgen toolchain**: See RFC-026b
- **YaoXiang exporting functions for C to call (reverse FFI)**: Future RFC; this document only
  states the principle
- **Inline assembly, SIMD intrinsics**: Not in scope of this RFC

---

## Proposal

### 1. External Libraries and Symbols: Curried First-Class Values

The information gap in FFI — "which library to link, which symbol" — is filled by making the library
a first-class value, without introducing any new keywords.

#### 1.1 Library as Value

```yaoxiang
// Native.c applies a library name → links the library at compile-time, returns a symbol resolver
sqlite3 = Native.c("libsqlite3")
```

`Native.c("libsqlite3")` is a **compile-time action + runtime value**:

- **Compile-time**: Linker `-lsqlite3`, library enters the symbol table, symbol existence is
  verifiable
- **Value**: `sqlite3` is a resolver; applying a symbol name yields an external reference to that
  library

`.c` is the ABI mechanism tag (C ABI). The core only has `.c` built-in; other mechanisms (`.wasm`,
etc.) are described in RFC-026a.

#### 1.2 Symbol as Value, Binding as `name: type = value`

The resolver applies a symbol name, and the resulting external reference is bound via the unified
syntax from RFC-007/010. **The type annotation on the LHS determines whether this reference is a
type or a function**:

```yaoxiang
sqlite3 = Native.c("libsqlite3")

// LHS is Type → bound as an opaque type
SqliteDb: Type = sqlite3("sqlite3")

// LHS is a function signature → bound as a function
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// .drop is a normal method binding (RFC-009 RAII convention)
SqliteDb.drop = SqliteDb.close
```

Compile-time verification: `sqlite3` in `sqlite3("sqlite3_open")` must exist in the `libsqlite3`
symbol table, otherwise it is a compile error.

#### 1.3 Method Binding and self Position

In the `Type.method: (...) -> ...` form, `self` is implicitly in the first position — when
`db.exec("SELECT")` is called, `db` is passed as the 0th argument to the C function `sqlite3_exec`.

If you need to bind an already-declared standalone function as a method, use the `[N]` syntax to
specify the self position (RFC-004 curried multi-position binding):

```yaoxiang
// Standalone function
sqlite3_close_v2: (db: SqliteDb) -> Int32 = sqlite3("sqlite3_close_v2")

// Bind as a method, [0] means db is self
SqliteDb.soft_close = sqlite3_close_v2[0]
```

`Native.c(...)` direct method binding and `[N]` manual binding are both `name: type = value`, both
put a function value on the right of `=`, with no two parallel mechanisms.

#### 1.4 User Experience: Zero unsafe, Zero Raw Pointers

```yaoxiang
import sqlite3_bindings

db = SqliteDb.open("test.db")
db.exec("SELECT * FROM users")
// ← End of scope, RAII automatically calls SqliteDb.drop → sqlite3_close(db)
```

---

### 2. Type Dichotomy: Layout Ownership Pinned at Definition

When external data enters YaoXiang, only one question is asked: **who decides the layout of this
block of memory?**

```
├─ The layout is an external black box (sqlite3, FILE*, socket fd)
│   → Opaque handle  =  lib("symbol")
│   → YaoXiang only holds a pointer, never dereferences, only passes between library functions
│   → External code reads its own memory; YaoXiang does not touch it
│
└─ The layout is defined by YaoXiang (timespec, point, struct whose fields need to be read)
    → Transparent type  =  { field: Type, ... }
    → YaoXiang owns the memory, defines the layout, reads/writes fields
    → External code writes/reads into the memory whose layout is defined by YaoXiang
```

**There is no third kind.** The "three-layer memory model (copy/takeover/system-level)" in the
previous design was patch thinking — the truth is the dichotomy of layout ownership.

#### 2.1 Opaque Handle: Layout Owned by External

```yaoxiang
SqliteDb: Type = sqlite3("sqlite3")
```

- Internally YaoXiang only holds a pointer-sized handle
- The user cannot construct it (`SqliteDb {}` → compile error), cannot access fields (no fields to
  access)
- The only source: external functions that return `SqliteDb`
- When calling a method, the handle is borrowed back to the library, which reads **its own** memory
  (the `sqlite3` struct lives on the library's heap)

When external code "reads internals" it reads the structure it allocated itself; YaoXiang just
transports the handle. No memory conflict.

#### 2.2 Transparent Type: Layout Owned by YaoXiang

```yaoxiang
// Fields are meaningful and need to be read/written → transparent type, layout declared by YaoXiang
Timespec: Type = {
    tv_sec: Int64,
    tv_nsec: Int64
}
clock_gettime: (clk: Int32, ts: *Timespec) -> Int32 = Native.c("librt")("clock_gettime")

ts = clock_gettime(CLOCK_REALTIME)   // See §3, marshaling goes through the temporary area
print(ts.tv_sec)                      // YaoXiang reads according to its own field definition
```

External code reads/writes into a block of memory **whose layout is defined by YaoXiang and owned by
YaoXiang**. The layout is YaoXiang's contract, not the external's.

#### 2.3 Decision Rule

The user only needs to decide one thing: **do I need to read the fields of this type?**

| Decision                                                          | Type                         | Layout Ownership |
| ----------------------------------------------------------------- | ---------------------------- | ---------------- |
| Don't read fields, only pass the handle between library functions | Opaque handle `= lib("sym")` | External         |
| Need to read/write fields                                         | Transparent type `{ ... }`   | YaoXiang         |

---

### 3. Marshaling: Signature-Driven, Temporary Area Isolation

Cross-boundary data conversion is **signature-driven**, with conversion rules determined for each
parameter position at compile-time. **The core safety guarantee: external code reads/writes the
marshaling temporary area, not YaoXiang's heap objects.**

#### 3.1 Default Temporary Area Copy

```
YaoXiang → C (input parameter):
    Copy data to the call temporary area → pass temporary area pointer to C
    → C out-of-bounds/writing damage only harms the temporary area; YaoXiang heap objects are isolated

C → YaoXiang (return/output parameter):
    C writes to the temporary area → YaoXiang memcpy's back to its own object
    → C cannot reach YaoXiang's final object
```

**External code always reads/writes the marshaling temporary area, fully isolated from YaoXiang heap
objects.** Wrong layout declaration, C storing a dangling pointer, C out-of-bounds — all only harm
the temporary area, while YaoXiang objects remain intact. The cost is one memcpy.

#### 3.2 Marshaling Rules Table

**Input direction (YaoXiang → C)**:

| YaoXiang Type       | C Representation  | Marshaling Action                                     | Ownership                                  |
| ------------------- | ----------------- | ----------------------------------------------------- | ------------------------------------------ |
| `Int32/Int64/Float` | `int/long/double` | Put directly in registers, zero conversion            | Value semantics                            |
| `String`            | `const char*`     | Lend a read-only view (temporary, valid for the call) | YaoXiang retains, C read-only              |
| Transparent type    | `struct T*`       | Copy to temporary area, pass temporary area pointer   | YaoXiang owns the object, C reads the copy |
| Opaque handle       | `void*`           | Extract the internal handle pointer                   | YaoXiang holds, borrows to C               |
| `*T`                | `T*`              | Pass raw pointer directly (unsafe)                    | User is responsible                        |

**Return direction (C → YaoXiang)**:

| C Return                            | YaoXiang Type    | Marshaling Action                                    | Ownership                                         |
| ----------------------------------- | ---------------- | ---------------------------------------------------- | ------------------------------------------------- |
| `int/double`                        | `Int32/Float`    | Read directly from registers                         | Value semantics                                   |
| `char*`                             | `String`         | strlen + memcpy to YaoXiang String                   | YaoXiang owns the copy, original memory untouched |
| `struct T*` (newly created handle)  | Opaque handle    | Handle stored in YaoXiang object                     | YaoXiang takes over                               |
| `struct T` (value/output parameter) | Transparent type | C writes to temporary area → memcpy back to YaoXiang | YaoXiang owns                                     |
| `char*` (static area)               | `*const U8`      | Store raw pointer, no copy (unsafe read)             | Not taken over, user is responsible               |

#### 3.3 Borrow Lifetime: Strictly Limited to a Single Call

Pointers YaoXiang lends to external code (String read-only view, transparent type temporary area,
handle) **have lifetimes strictly limited to within a single call**:

- During the call: the pointer is valid, external code can read/write
- After the call returns: the borrow is immediately invalidated

If external code stores the pointer and uses it after the call, it is external code violating the
FFI standard contract (equivalent to a library bug), and YaoXiang is not responsible for this. This
is consistent with the C FFI contract of all languages (Rust's `&T` passed to C has the same
constraint).

#### 3.4 String Never Gives Out a Persistent Pointer

`String` is the key to "C does not interfere with YaoXiang memory":

- Into C: Lend a **temporary read-only view**, valid for the call
- Out of C: strlen + memcpy into a **copy** owned by YaoXiang

C never gets a persistent pointer to YaoXiang's String, and YaoXiang never holds a long-term
reference to C's `char*`. Structurally isolated.

---

### 4. Ownership and Lifetime: Move + RAII

Opaque handles follow the ownership model from RFC-009, with zero new concepts.

#### 4.1 Core Principles

- **Move semantics**: Opaque handles default to Move; assignment/parameter passing/return =
  ownership transfer, non-copyable
- **Handle unique ownership**: At any moment, a handle has only one owner → structurally prevents
  double-free
- **RAII release**: When the scope ends, if `.drop` is bound, it is called automatically
- **Consumption tracking**: After explicit destruct or Move, the variable is consumed and cannot be
  used again → prevents use-after-free

#### 4.2 `.drop` is an Optional External Side Effect

```yaoxiang
SqliteDb.drop = SqliteDb.close     // Scope end calls sqlite3_close
```

**`.drop` is not a mechanism to prevent YaoXiang leaks** — the handle storage on the YaoXiang side
(a pointer-sized value) is automatically reclaimed, unrelated to `.drop`. `.drop` is an **optional
side effect of calling an external function when the scope ends**:

- If `.drop` is bound → call it when the scope ends (clean up external resources)
- If `.drop` is not bound → do nothing, **no error, no warning**

Whether external resources need cleanup is a matter of the external library's specification
(`getenv` returns a static area and should not be freed; a global singleton should not be freed),
and YaoXiang does not overreach to enforce it. Leak prevention relies on Move + unique ownership
(unconditional, structural), not on `.drop`.

#### 4.3 Automatic Destruct and Order

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    // ← End of scope, reverse-order automatic destruct (only called if .drop is bound):
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

Destruct order: reverse of definition order, consistent with RAII.

#### 4.4 Move and Consumption

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transfer
db.exec("...")          // ❌ Compile error: db has been Moved, cannot be read after consumption

process_db: (db: SqliteDb) -> Void = {
    db.exec("...")
    // ← Function end, db is destructed here
}
process_db(some_db)     // Move into the function
// some_db is invalid here
```

#### 4.5 Null Handling

```yaoxiang
// May return null → ?T, user must handle it
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => db.exec("SELECT 1"),
    None => print("open failed")
}

// Convention does not return null → not marked, panic on null to expose
```

C returning null is either handled by the user (`?T`), or panics to expose. There is no third
"silently ignore" option.

#### 4.6 Destruct Failure Handling

The return type of the function bound to `.drop` determines the behavior:

| `.drop` Return Type  | Behavior                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------ |
| `Void`               | No failure                                                                                 |
| `Int32` (error code) | Panic on non-zero — destruct failure means abnormal state, exposure is better than silence |
| `?Error`             | Panic on non-`None` — same as above                                                        |

Destruct failure cannot be silenced. When ignoring specific errors is needed, handle them explicitly
in the wrapper function bound to `.drop`.

---

### 5. FFI Behavior in spawn Blocks

Resource type determination is decided by the `.drop` binding (RFC-024), with zero additional
markers:

| Determination                       | Behavior                                                                      |
| ----------------------------------- | ----------------------------------------------------------------------------- |
| Opaque handle with `.drop` bound    | Resource type — same-instance operations in spawn blocks auto-serialize       |
| Opaque handle without `.drop` bound | Non-resource type — parallelizable (pure data handle, no release side effect) |
| Transparent type / value type       | Non-resource type — parallelizable                                            |

```yaoxiang
SqliteDb.drop = SqliteDb.close   // → Resource type

(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Same instance, auto-serialized
    r2 = db.exec("INSERT ...")    // Wait for r1
}

(x, y) = spawn {
    db1 = SqliteDb.open("a.db"),   // Different instances, parallelizable
    db2 = SqliteDb.open("b.db")
}
```

Types with `.drop` bound auto-serialize same-instance operations in spawn, ensuring destruct is free
of concurrent contention.

---

### 6. Escape Hatch: Raw Pointers + unsafe

The default marshaling goes through temporary area copying, which is safe but has memcpy overhead.
For performance-sensitive scenarios (large structs, high-frequency calls) requiring zero copy, the
user explicitly takes the raw pointer escape hatch:

```yaoxiang
// C reads YaoXiang memory directly, zero copy — user explicitly accepts the risk
ptr: *const U8 = Native.c("libc")("getenv")("HOME")
unsafe {
    value = read_c_string(ptr)   // User guarantees ptr is valid
}
```

**`unsafe` is only used for raw pointer operations, fully orthogonal to opaque handles and
transparent types.** Normal FFI (handle + transparent type) does not require `unsafe`. Writing
`unsafe {}` = the user explicitly signs off on accepting the risk of direct memory access.

**Trust boundary**: C cannot provide compile-time verifiable type contracts (`.h` is not an ABI
contract; the symbol table only has names, not signatures). Therefore, the correctness of C
signatures cannot be automatically verified — the binding author guarantees it when writing
`Native.c(...)` + signature. **Trust is localized at the binding declaration**: the binding author
guarantees, and package users get a safe API. This is consistent with Rust's `extern "C"` (writing
`extern` is a trust action; once the safe wrapper is wrapped, calls are safe).

---

## Trade-offs

### Advantages

1. **Complete information**: Library linked at compile-time, symbol verified at compile-time, no
   runtime ambiguity of "library not found"
2. **Explicit layout ownership**: Type dichotomy, pinned at definition, no runtime inference
3. **Structural safety**: Temporary area isolation + Move + RAII, external code cannot reach
   YaoXiang heap objects
4. **Zero new keywords**: `Native.c` currying + `name: type = value`, fully reuses existing syntax
5. **Honest boundary**: Does not pretend to verify C signatures; trust is localized at the
   declaration

### Disadvantages

1. **memcpy overhead**: Default marshaling copies; large structs with high-frequency calls need to
   explicitly take the escape hatch
2. **Layout guarantees are manual**: Matching transparent type layout with C struct is guaranteed by
   the binding author/yx-bindgen
3. **C signatures are not compile-time verifiable**: Fundamental limitation of FFI; YaoXiang cannot
   eliminate it

---

## Implementation Strategy

### Phase 1: External Libraries and Symbols (v0.8)

- [ ] Implement `Native.c("lib")` compile-time link + return resolver value
- [ ] Implement symbol resolver application (`lib("symbol")`) + compile-time symbol table
      verification
- [ ] Implement type dichotomy (opaque handle / transparent type)
- [ ] Implement method binding (direct binding + `[N]` position binding)

### Phase 2: Marshaling and Safety (v0.8)

- [ ] Implement signature-driven marshaling code generation
- [ ] Implement temporary area copy isolation (input copy, return memcpy)
- [ ] Implement String temporary read-only view + return copy
- [ ] Implement borrow lifetime limited to a single call

### Phase 3: Ownership and Lifetime (v0.9)

- [ ] Implement opaque handle Move + unique ownership
- [ ] Implement `.drop` RAII automatic destruct (optional, no error if missing)
- [ ] Implement consumption tracking (disabled after Move)
- [ ] Implement `?T` integration with null returns
- [ ] Implement spawn resource type serialization

### Future Work

- **Extensible FFI mechanism** (RFC-026a): `FfiMechanism` abstraction, `.wasm`/`.python` plugins,
  dynamic loading
- **yx-bindgen** (RFC-026b): C header → `.yx` binding + platform-correct layout generation

---

## Relationships with Other RFCs

- **RFC-004**: Curried multi-position binding — the origin of the `[N]` method binding syntax
- **RFC-007**: Function definition syntax unification — `Native.c(...)` binding is
  `name: type = value`
- **RFC-009**: Ownership model — Move, RAII, `?T`; handle lifetime is entirely based on this
- **RFC-010**: Unified type syntax — LHS type annotation determines whether the binding is a type or
  function
- **RFC-024**: Concurrency model — resource type determination in spawn is based on `.drop`
- **RFC-020/021** (deprecated): Content merged into this document
- **RFC-026a**: Extensible FFI mechanism system
- **RFC-026b**: yx-bindgen toolchain

---

## Design Decision Record

| Decision                                | Decision                                          | Reason                                                                                                                       | Date       |
| --------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **Library as value**                    | `Native.c("lib")` curried returns resolver        | Library information becomes a first-class value visible at compile-time, filling the "which library" gap, zero new keywords  | 2026-07-03 |
| **Compile-time link**                   | `Native.c("lib")` triggers `-llib`                | Symbol table readable at compile-time, symbol existence verifiable, type is real                                             | 2026-07-03 |
| **Type dichotomy**                      | Opaque handle / transparent type                  | Layout ownership dichotomy covers everything; removes the "three-layer memory model" patch                                   | 2026-07-03 |
| **Marshaling temporary area isolation** | Default copy, heap objects isolated from external | External out-of-bounds/dangling only harms temporary area, YaoXiang objects intact; zero copy requires explicit escape hatch | 2026-07-03 |
| **`.drop` optional**                    | Missing does nothing, no error                    | YaoXiang handle storage auto-reclaimed; external resource cleanup is external spec, not overreached                          | 2026-07-03 |
| **Leak prevention mechanism**           | Move + handle unique ownership (unconditional)    | Structural guarantee, unrelated to `.drop`                                                                                   | 2026-07-03 |
| **Trust boundary**                      | At the `Native.c(...)` declaration                | C signatures are not compile-time verifiable; trust localized; unsafe only for raw pointers                                  | 2026-07-03 |
| **Null handling**                       | `?T` or panic                                     | C's problem not hidden, no "silently ignore" option                                                                          | 2026-07-03 |
| **Destruct failure**                    | `.drop` return type determines, uniformly panic   | Destruct failure cannot be silenced                                                                                          | 2026-07-03 |

---

## References

### YaoXiang Official Documentation

- [RFC-004 Curried Multi-Position Binding](004-curry-multi-position-binding.md)
- [RFC-007 Function Definition Syntax Unification](007-function-syntax-unification.md)
- [RFC-009 Ownership Model](009-ownership-model.md)
- [RFC-010 Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-024 Concurrency Model](024-concurrency-model.md)
- [RFC-026a Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md)
- [RFC-026b yx-bindgen Toolchain](../draft/026b-yx-bindgen.md)

### External References

- [Rust FFI (Nomicon)](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [LuaJIT FFI](https://luajit.org/ext_ffi.html)

---

## Lifecycle and Destination

| Status           | Location                    | Description               |
| ---------------- | --------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/`   | Open community discussion |
| **Accepted**     | `docs/design/rfc/accepted/` | Official design document  |
