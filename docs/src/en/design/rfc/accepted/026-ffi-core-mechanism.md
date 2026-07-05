---
title: "RFC-026: FFI Core Mechanism"
status: "Accepted"
updated: "2026-07-03"
---

# RFC-026: FFI Core Mechanism

> **References**:
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](./024-concurrency-model.md)

> **Deprecated**:
> - [RFC-020: Dynamic Module and FFI Integration](../deprecated/020-dynamic-modules-ffi.md) — Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../deprecated/021-library-driven-ffi-extension.md) — Content merged into this document

> **Sub-RFCs**:
> - [RFC-026a: Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md) — Multi-ABI mechanism plugins, `FfiMechanism` abstraction, dynamic loading
> - [RFC-026b: yx-bindgen Toolchain](../draft/026b-yx-bindgen.md) — C header file → `.yx` binding code generation

## Summary

This document defines the FFI (Foreign Function Interface) core mechanism of YaoXiang. The core idea: **external libraries are first-class values linked at compile-time, the memory layout ownership of cross-boundary data is nailed down at type definition time, and YaoXiang's heap objects are structurally isolated from external code.**

1. **External libraries are values**: `Native.c("libsqlite3")` links the library at compile-time and returns a parser, carrying library information through currying
2. **External symbols are values**: applying a symbol name to the parser yields an external reference, bound as a type or function through `name: type = value` (RFC-007/010)
3. **Type dichotomy**: opaque handle (layout owned by external) / transparent type (layout owned by YaoXiang) — no third option
4. **Marshalling isolation**: cross-boundary data is by default copied to a call scratch area; YaoXiang heap objects are isolated from external code
5. **Ownership safety**: unique ownership of handles (Move) + RAII, structurally preventing double-free and use-after-free
6. **Escape hatch**: `*T` raw pointer + `unsafe {}`, where the user explicitly accepts the risk of zero-copy direct memory access

**Core boundary — five inviolable contracts**:

```
1. Libraries are linked at compile-time, symbol existence is verified at compile-time
2. Type layout ownership is determined at definition time: opaque handle belongs to external, transparent type belongs to YaoXiang
3. By default, marshalling goes through scratch area copying; YaoXiang heap objects are isolated from external code
4. Unique handle ownership + Move, structurally preventing double-free/dangling
5. External code always reads/writes memory with "clear layout and clear ownership" — no gray area
```

---

## Motivation

### Current State and Goals

The current codebase's `native("symbol")` is merely a dispatch mechanism for YaoXiang bytecode to call Rust std functions (`FfiRegistry` = `HashMap<String, RustFnPtr>`), **with no real cross-ABI boundary** — no dlopen, no C ABI marshalling, no memory ownership crossing.

True FFI must solve four problems:

| Problem | Answer in this RFC |
|------|--------------|
| **Symbol resolution** | Libraries are first-class values linked at compile-time (`Native.c("lib")`), symbol verified at compile-time |
| **Value marshalling** | Signature-driven, compile-time determines conversion rules for each parameter position |
| **Memory ownership** | Type dichotomy determines ownership; default copying provides isolation |
| **Lifecycle safety** | Move + RAII + borrow restricted to single call |

RFC-020 and RFC-021 define different aspects of FFI with some overlap; this document consolidates them into a unified specification.

### Design Goals

1. **Zero raw pointer leakage to user code**: in normal FFI usage, no raw pointers appear in `.yx` source code
2. **Explicit layout ownership**: when users define a type, they decide who owns this memory — not inferred at runtime
3. **Structural safety**: no leak, no double-free, no use-after-free guaranteed by the type system, not by convention
4. **Honest trust boundary**: C cannot provide compile-time verifiable type contracts, so trust is localized at the binding declaration
5. **Self-hosting compatible**: no host-language-specific over-abstraction

### Out of Scope

- **Multi-ABI mechanism plugin system** (Wasm/Python/custom ABI): see RFC-026a
- **yx-bindgen toolchain**: see RFC-026b
- **YaoXiang exporting functions for C to call (reverse FFI)**: follow-up RFC, this document only declares principles
- **Inline assembly, SIMD intrinsics**: not within the scope of this RFC

---

## Proposal

### 1. External Libraries and Symbols: Curried First-Class Values

The FFI information gap — "which library to link, which symbol" — is filled by making libraries first-class values, without introducing any new keywords.

#### 1.1 Libraries are Values

```yaoxiang
// Native.c applies library name → links the library at compile-time, returns a symbol parser
sqlite3 = Native.c("libsqlite3")
```

`Native.c("libsqlite3")` is a **compile-time action + runtime value**:

- **Compile-time**: linker `-lsqlite3`, the library enters the symbol table, symbol existence is verifiable
- **Value**: `sqlite3` is a parser; applying a symbol name yields an external reference of that library

`.c` is the ABI mechanism tag (C ABI). The core only has `.c` built-in; other mechanisms (`.wasm` etc.) are in RFC-026a.

#### 1.2 Symbols are Values, Bindings are `name: type = value`

Applying a symbol name to the parser yields an external reference, which is bound through RFC-007/010's unified syntax. **The type annotation on the LHS determines whether this reference is a type or a function**:

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

Compile-time verification: `sqlite3_open` in `sqlite3("sqlite3_open")` must exist in the `libsqlite3` symbol table, otherwise a compile error.

#### 1.3 Method Binding and self Position

In the form `Type.method: (...) -> ...`, `self` is implicitly in the first position — when `db.exec("SELECT")` is called, `db` is passed as the 0th parameter of the C function `sqlite3_exec`.

If you need to bind a declared standalone function as a method, use the `[N]` syntax to specify the self position (RFC-004 curried multi-position binding):

```yaoxiang
// Standalone function
sqlite3_close_v2: (db: SqliteDb) -> Int32 = sqlite3("sqlite3_close_v2")

// Bound as a method, [0] means db is self
SqliteDb.soft_close = sqlite3_close_v2[0]
```

Direct method binding via `Native.c(...)` and manual binding via `[N]` are both `name: type = value`, both putting a function value on the right side of `=` — no two mechanisms.

#### 1.4 User Experience: Zero unsafe, Zero Raw Pointer

```yaoxiang
import sqlite3_bindings

db = SqliteDb.open("test.db")
db.exec("SELECT * FROM users")
// ← Scope ends, RAII automatically calls SqliteDb.drop → sqlite3_close(db)
```

---

### 2. Type Dichotomy: Layout Ownership Nailed Down at Definition Time

When external data enters YaoXiang, only one question is asked: **who decides the layout of this memory?**

```
├─ Layout is an external black box (sqlite3, FILE*, socket fd)
│   → Opaque handle  =  lib("symbol")
│   → YaoXiang only holds a pointer, never dereferences, only passes between library functions
│   → External code reads its own memory; YaoXiang doesn't touch it
│
└─ Layout is defined by YaoXiang (timespec, point, struct whose fields need reading)
    → Transparent type  =  { field: Type, ... }
    → YaoXiang owns the memory, defines the layout, reads/writes fields
    → External code fills/reads into memory whose layout is defined by YaoXiang
```

**There is no third option.** The "three-layer memory model (copy/take-over/system-level)" in the previous design is patch-thinking — the truth is the dichotomy of layout ownership.

#### 2.1 Opaque Handle: Layout Owned by External

```yaoxiang
SqliteDb: Type = sqlite3("sqlite3")
```

- YaoXiang internally only holds a pointer-sized handle
- Users cannot construct ( `SqliteDb {}` → compile error), cannot access fields (no fields to access)
- The only source: external functions that return `SqliteDb`
- When calling a method, the handle is borrowed back to the library, which reads **its own** memory (the `sqlite3` struct lives on the library's heap)

External code "reading inside" reads structures it allocated; YaoXiang only carries the handle. No memory conflict.

#### 2.2 Transparent Type: Layout Owned by YaoXiang

```yaoxiang
// Fields are meaningful, need read/write → transparent type, layout declared by YaoXiang
Timespec: Type = {
    tv_sec: Int64,
    tv_nsec: Int64
}
clock_gettime: (clk: Int32, ts: *Timespec) -> Int32 = Native.c("librt")("clock_gettime")

ts = clock_gettime(CLOCK_REALTIME)   // See §3, marshalling through scratch area
print(ts.tv_sec)                      // YaoXiang reads by its own field definition
```

External code reads/writes into memory whose **layout is defined and owned by YaoXiang**. The layout is a YaoXiang contract, not an external one.

#### 2.3 Decision Rule

Users only need to decide one thing: **do I need to read the fields of this type?**

| Decision | Type | Layout Ownership |
|------|------|---------|
| Don't read fields, just pass the handle between library functions | Opaque handle `= lib("sym")` | External |
| Need to read/write fields | Transparent type `{ ... }` | YaoXiang |

---

### 3. Marshalling: Signature-Driven, Scratch Area Isolation

Cross-boundary data conversion is **signature-driven**; the compile-time determines conversion rules for each parameter position. **Core safety guarantee: external code reads/writes the marshalling scratch area, not YaoXiang's heap objects.**

#### 3.1 Default Goes Through Scratch Area Copy

```
YaoXiang → C (input parameters):
    Copy data to call scratch area → pass scratch area pointer to C
    → C out-of-bounds/overwrite only damages scratch area, YaoXiang heap objects isolated

C → YaoXiang (return/output parameters):
    C writes to scratch area → YaoXiang memcpy back to its own object
    → C never touches YaoXiang's final object
```

**External code always reads/writes the marshalling scratch area, completely isolated from YaoXiang heap objects.** Wrong layout declaration, C storing dangling pointer, C out-of-bounds — all only damage the scratch area, YaoXiang's objects remain intact. The cost is one memcpy.

#### 3.2 Marshalling Rules Table

**Input direction (YaoXiang → C)**:

| YaoXiang Type | C Representation | Marshalling Action | Ownership |
|--------------|--------|---------|--------|
| `Int32/Int64/Float` | `int/long/double` | Direct register placement, zero conversion | Value semantics |
| `String` | `const char*` | Lend out read-only view (temporary, valid during call) | YaoXiang retains, C read-only |
| Transparent type | `struct T*` | Copy to scratch area, pass scratch area pointer | YaoXiang owns object, C reads copy |
| Opaque handle | `void*` | Extract internal handle pointer | YaoXiang holds, lends to C |
| `*T` | `T*` | Direct raw pointer passing (unsafe) | User responsibility |

**Return direction (C → YaoXiang)**:

| C Returns | YaoXiang Type | Marshalling Action | Ownership |
|--------|--------------|---------|--------|
| `int/double` | `Int32/Float` | Direct register read | Value semantics |
| `char*` | `String` | strlen + memcpy to YaoXiang String | YaoXiang owns copy, original memory untouched |
| `struct T*` (new handle) | Opaque handle | Handle stored in YaoXiang object | YaoXiang takes over |
| `struct T` (value/output parameter) | Transparent type | C writes to scratch area → memcpy to YaoXiang | YaoXiang owns |
| `char*` (static area) | `*const U8` | Store raw pointer, no copy (unsafe read) | No take-over, user responsibility |

#### 3.3 Borrow Lifecycle: Strictly Limited to Single Call

Pointers that YaoXiang lends to external code (String read-only view, transparent type scratch area, handle) have **lifecycle strictly limited to a single call**:

- During call: pointer valid, external code can read/write
- After call returns: borrow immediately invalidated

If external code stores the pointer for use after the call, it is external code violating the FFI standard contract (equivalent to a library bug); YaoXiang is not responsible for this. This is consistent with C FFI contracts in all languages (Rust's `&T` passed to C has the same constraint).

#### 3.4 String Never Gives Up Persistent Pointer

`String` is the key to "C doesn't touch YaoXiang memory":

- Into C: lend out a **temporary read-only view**, valid during the call
- Out of C: strlen + memcpy into a **copy** owned by YaoXiang

C never gets a persistent pointer to YaoXiang String; YaoXiang never holds a long-term reference to a C `char*`. Structurally isolated.

---

### 4. Ownership and Lifecycle: Move + RAII

Opaque handles follow RFC-009's ownership model, with zero new concepts.

#### 4.1 Core Principles

- **Move semantics**: opaque handles default to Move; assignment/parameter passing/return = ownership transfer, non-copyable
- **Unique handle ownership**: a handle has only one owner at any time → structurally prevents double-free
- **RAII release**: when scope ends, if `.drop` is bound, automatically called
- **Consumption tracking**: after explicit destruction or Move, the variable is consumed and cannot be used again → prevents use-after-free

#### 4.2 `.drop` is an Optional External Side Effect

```yaoxiang
SqliteDb.drop = SqliteDb.close     // Call sqlite3_close when scope ends
```

**`.drop` is not a mechanism to prevent YaoXiang leaks** — the handle storage on the YaoXiang side (a pointer-sized value) is automatically reclaimed, unrelated to `.drop`. `.drop` is an optional side effect of **calling an external function at the end of the scope**:

- `.drop` bound → call it at scope end (clean up external resources)
- `.drop` not bound → do nothing, **no error, no warning**

Whether external resources need cleanup is a matter of external library specification (`getenv` returns a static area that shouldn't be freed, global singletons shouldn't be freed); YaoXiang doesn't overreach to enforce. Leak prevention relies on Move + unique ownership (unconditional, structural), not on `.drop`.

#### 4.3 Automatic Destruction and Order

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    // ← Scope ends, reverse-order automatic destruction (only calls if .drop is bound):
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
    // ← Function ends, db destructed here
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
    None => print("open failed")
}

// Convention does not return null → not marked, panic to expose when null
```

C returning null: either the user handles it (`?T`), or it panics to expose. There is no third "silently ignore" option.

#### 4.6 Destruction Failure Handling

The return value type of the function bound to `.drop` determines behavior:

| `.drop` Return Type | Behavior |
|----------------|------|
| `Void` | No failure |
| `Int32` (error code) | Non-zero panics — destruction failure means abnormal state, expose is better than silent |
| `?Error` | Non-None panics — same as above |

Destruction failure cannot be silent. To ignore specific errors, explicitly handle in the wrapper function bound to `.drop`.

---

### 5. FFI Behavior in spawn Blocks

Resource type determination is decided by the `.drop` binding (RFC-024), with zero additional markers:

| Determination | Behavior |
|------|------|
| Opaque handle with `.drop` bound | Resource type — same instance operations in spawn block automatically serialized |
| Opaque handle without `.drop` bound | Non-resource type — can run in parallel (pure data handle, no release side effect) |
| Transparent type / value type | Non-resource type — can run in parallel |

```yaoxiang
SqliteDb.drop = SqliteDb.close   // → resource type

(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Same instance, automatically serialized
    r2 = db.exec("INSERT ...")    // Wait for r1
}

(x, y) = spawn {
    db1 = SqliteDb.open("a.db"),   // Different instances, can run in parallel
    db2 = SqliteDb.open("b.db")
}
```

Types with `.drop` automatically serialize same-instance operations in spawn, ensuring destruction has no concurrent contention.

---

### 6. Escape Hatch: Raw Pointer + unsafe

Default marshalling goes through scratch area copying — safe but with memcpy overhead. For performance-sensitive scenarios (large structures, high-frequency calls) requiring zero-copy, users explicitly use the raw pointer escape hatch:

```yaoxiang
// C directly reads YaoXiang memory, zero-copy — user explicitly accepts risk
ptr: *const U8 = Native.c("libc")("getenv")("HOME")
unsafe {
    value = read_c_string(ptr)   // User guarantees ptr is valid
}
```

**`unsafe` is only used for raw pointer operations, completely orthogonal to opaque handles and transparent types.** Normal FFI (handles + transparent types) doesn't need unsafe. Writing `unsafe {}` = user explicitly signs off on the risk of direct memory access.

**Trust boundary**: C cannot provide compile-time verifiable type contracts (`.h` is not an ABI contract; the symbol table has names but no signatures). So the correctness of C signatures cannot be automatically verified — it is guaranteed by the binding author when writing `Native.c(...)` + signatures. **Trust is localized at the binding declaration**: the binding author guarantees it, package users get a safe API. This is consistent with Rust's `extern "C"` (writing extern is a trust act, and a safe wrapper wraps it for safe calls).

---

## Trade-offs

### Advantages

1. **Complete information**: libraries linked at compile-time, symbols verified at compile-time, no runtime "library not found" ambiguity
2. **Explicit layout ownership**: type dichotomy, nailed down at definition time, no runtime inference
3. **Structural safety**: scratch area isolation + Move + RAII, external code cannot touch YaoXiang heap objects
4. **Zero new keywords**: `Native.c` currying + `name: type = value`, all reusing existing syntax
5. **Honest boundary**: doesn't pretend to verify C signatures, localizes trust at the declaration

### Disadvantages

1. **memcpy overhead**: default marshalling copies, large structures with high-frequency calls need to explicitly use the escape hatch
2. **Layout guarantee is manual**: transparent type layout matching C struct is guaranteed by binding author/yx-bindgen
3. **C signatures cannot be verified at compile-time**: fundamental limitation of FFI, YaoXiang cannot eliminate this

---

## Implementation Strategy

### Phase 1: External Libraries and Symbols (v0.8)

- [ ] Implement `Native.c("lib")` compile-time link + return parser value
- [ ] Implement symbol parser application (`lib("symbol")`) + compile-time symbol table verification
- [ ] Implement type dichotomy (opaque handle / transparent type)
- [ ] Implement method binding (direct binding + `[N]` position binding)

### Phase 2: Marshalling and Safety (v0.8)

- [ ] Implement signature-driven marshalling code generation
- [ ] Implement scratch area copy isolation (input copy, return memcpy)
- [ ] Implement String temporary read-only view + return copy
- [ ] Implement borrow lifecycle limited to single call

### Phase 3: Ownership and Lifecycle (v0.9)

- [ ] Implement opaque handle Move + unique ownership
- [ ] Implement `.drop` RAII automatic destruction (optional, no error if missing)
- [ ] Implement consumption tracking (disabled after Move)
- [ ] Implement `?T` and null return integration
- [ ] Implement spawn resource type serialization

### Follow-up Work

- **Extensible FFI Mechanism** (RFC-026a): `FfiMechanism` abstraction, `.wasm`/`.python` etc. plugins, dynamic loading
- **yx-bindgen** (RFC-026b): C header file → `.yx` binding + platform-correct layout generation

---

## Relationship with Other RFCs

- **RFC-004**: Curried multi-position binding — source of the `[N]` method binding syntax
- **RFC-007**: Unified function definition syntax — `Native.c(...)` binding is `name: type = value`
- **RFC-009**: Ownership model — Move, RAII, `?T`, handle lifecycle entirely based on this
- **RFC-010**: Unified type syntax — LHS type annotation determines whether binding is a type or function
- **RFC-024**: Concurrency model — resource type determination in spawn is based on `.drop`
- **RFC-020/021** (deprecated): content merged into this document
- **RFC-026a**: Extensible FFI mechanism system
- **RFC-026b**: yx-bindgen toolchain

---

## Design Decision Record

| Decision | Decision | Reason | Date |
|------|------|------|------|
| **Libraries are values** | `Native.c("lib")` curried returns parser | Library information becomes a compile-time visible first-class value, fills the "which library to link" gap, zero new keywords | 2026-07-03 |
| **Compile-time link** | `Native.c("lib")` triggers `-llib` | Symbol table readable at compile-time, symbol existence verifiable, types are real | 2026-07-03 |
| **Type dichotomy** | Opaque handle / transparent type | Layout ownership dichotomy covers everything; remove "three-layer memory model" patch | 2026-07-03 |
| **Scratch area marshalling isolation** | Default copy, heap objects isolated from external | External out-of-bounds/dangling only damages scratch area, YaoXiang objects intact; zero-copy needs explicit escape hatch | 2026-07-03 |
| **`.drop` is optional** | Missing does nothing, no error | YaoXiang handle storage automatically reclaimed; external resource cleanup is external spec, no overreach | 2026-07-03 |
| **Leak prevention mechanism** | Move + unique handle ownership (unconditional) | Structural guarantee, unrelated to `.drop` | 2026-07-03 |
| **Trust boundary** | At `Native.c(...)` declaration | C signatures cannot be verified at compile-time, trust localized, unsafe only for raw pointers | 2026-07-03 |
| **Null handling** | `?T` or panic | C's problem not hidden, no "silently ignore" option | 2026-07-03 |
| **Destruction failure** | `.drop` return type determines, uniform panic | Destruction failure cannot be silent | 2026-07-03 |

---

## References

### YaoXiang Official Documentation

- [RFC-004 Curried Multi-Position Binding](./004-curry-multi-position-binding.md)
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