---
title: 'RFC-026: FFI Core Mechanism'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-07-03'
updated: '2026-07-05'
issue: '#93'
---

# RFC-026: FFI Core Mechanism

> **References**:
>
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-024: Concurrency Model Based on Spawn Blocks](./024-concurrency-model.md)

> **Deprecated**:
>
> - [RFC-020: Dynamic Modules and FFI Integration](../deprecated/020-dynamic-modules-ffi.md) —
>   Content merged into this document
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../deprecated/021-library-driven-ffi-extension.md)
>   — Content merged into this document

> **Sub RFCs**:
>
> - [RFC-026a: Extensible FFI Mechanism System](../review/026a-extensible-ffi-system.md) — Multi-ABI
>   mechanism plugins, `FfiMechanism` abstraction, dynamic loading
> - [RFC-026b: yx-bindgen Toolchain](../draft/026b-yx-bindgen.md) — C header → `.yx` binding code
>   generation

## Summary

This document defines YaoXiang's FFI (Foreign Function Interface) core mechanism. Core idea:
**External libraries are first-class values linked at compile-time, the memory layout of
cross-boundary data is pinned at type definition, and YaoXiang heap objects are structurally
isolated from external code.**

1. **External library as value**: `Native.c("libsqlite3")` links the library at compile-time and
   returns a resolver, with currying carrying library information
2. **External symbol as value**: Resolver applied with symbol name yields external reference, bound
   as type or function via `name: type = value` (RFC-007/010)
3. **Type dichotomy**: Opaque handles (layout belongs to external) / transparent types (layout
   belongs to YaoXiang), no third option
4. **Marshaling isolation**: Cross-boundary data copies to call temporary area by default, YaoXiang
   heap objects isolated from external code
5. **Ownership safety**: Handle unique ownership (Move) + RAII, structurally preventing double-free
   and use-after-free
6. **Escape hatch**: `*T` raw pointer + `unsafe {}`, user explicitly accepts direct memory access
   risk with zero-copy

**Core boundary — five inviolable contracts**:

```
1. Library linked at compile-time, symbol existence verified at compile-time
2. Type layout ownership determined at definition: opaque handles belong to external, transparent types belong to YaoXiang
3. Default marshaling copies through temporary area, YaoXiang heap objects isolated from external code
4. Handle unique ownership + Move, structurally preventing double-free/ dangling
5. External code always reads/writes memory with clear layout and clear ownership, no ambiguity
```

---

## Motivation

### Current State and Goals

The current codebase's `native("symbol")` is merely YaoXiang bytecode's dispatch mechanism for
calling Rust std functions (`FfiRegistry` = `HashMap<String, RustFnPtr>`), **with no real cross-ABI
boundary** — no dlopen, no C ABI marshaling, no memory ownership crossing.

A true FFI must solve four problems:

| Problem               | This RFC's Answer                                                                                           |
| --------------------- | ----------------------------------------------------------------------------------------------------------- |
| **Symbol resolution** | Library is a first-class value linked at compile-time (`Native.c("lib")`), symbols verified at compile-time |
| **Value marshaling**  | Signature-driven, compiler determines conversion rules for each parameter position at compile-time          |
| **Memory ownership**  | Type dichotomy determines ownership; default copy isolation                                                 |
| **Lifetime safety**   | Move + RAII + borrow scoped to single call                                                                  |

RFC-020 and RFC-021 defined different aspects of FFI, with overlap. This document integrates them
into a unified specification.

### Design Goals

1. **Zero raw pointer leakage to user code**: In normal FFI usage, `.yx` source code contains no raw
   pointers
2. **Explicit layout ownership**: User decides at type definition who owns this memory, no runtime
   inference
3. **Structural safety**: No leaks, no double-free, no use-after-free guaranteed by type system, not
   by convention
4. **Honest trust boundary**: C cannot provide compile-time verifiable type contracts, trust
   localized at binding declarations
5. **Bootstrap compatibility**: No overly specialized abstractions unique to the host language

### Out of Scope

- **Multi-ABI mechanism plugin system** (Wasm/Python/custom ABI): See RFC-026a
- **yx-bindgen toolchain**: See RFC-026b
- **YaoXiang exporting functions to C calls (reverse FFI)**: Future RFC, this document only declares
  principles
- **Inline assembly, SIMD intrinsics**: Not in scope for this RFC

---

## Proposal

### 1. External Libraries and Symbols: Curried First-Class Values

The information gap in FFI — "which library, which symbol" — is filled by making libraries
first-class values, without introducing any new keywords.

#### 1.1 Library as Value

```yaoxiang
// Native.c applied with library name → compile-time link, returns a symbol resolver
sqlite3 = Native.c("libsqlite3")
```

`Native.c("libsqlite3")` is a **compile-time action + runtime value**:

- **Compile-time**: Linker `-lsqlite3`, library enters symbol table, symbol existence verifiable
- **Value**: `sqlite3` is a resolver, applied with symbol name yields external reference

`.c` is the ABI mechanism tag (C ABI). Core only includes `.c` built-in; other mechanisms (`.wasm`,
etc.) see RFC-026a.

#### 1.2 Symbol as Value, Binding as `name: type = value`

Resolver applied with symbol name yields external reference, bound using RFC-007/010 unified syntax.
**The type annotation on LHS determines whether this reference is a type or function**:

```yaoxiang
sqlite3 = Native.c("libsqlite3")

// LHS is Type → bound as opaque type
SqliteDb: Type = sqlite3("sqlite3")

// LHS is function signature → bound as function
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// .drop is ordinary method binding (RFC-009 RAII convention)
SqliteDb.drop = SqliteDb.close
```

Compile-time verification: `sqlite3("sqlite3_open")` must have `sqlite3_open` in `libsqlite3` symbol
table, otherwise compile error.

#### 1.3 Method Binding and self Position

In `Type.method: (...) -> ...` syntax, `self` is implicitly at position zero — when calling
`db.exec("SELECT")`, `db` becomes the 0th argument of C function `sqlite3_exec`.

To bind an already declared standalone function as a method, use `[N]` syntax to specify self
position (RFC-004 curried multi-position binding):

```yaoxiang
// Standalone function
sqlite3_close_v2: (db: SqliteDb) -> Int32 = sqlite3("sqlite3_close_v2")

// Bound as method, [0] means db is self
SqliteDb.soft_close = sqlite3_close_v2[0]
```

`Native.c(...)` direct method binding and `[N]` manual binding both use `name: type = value`, both
put a function value to the right of `=`, no two mechanisms.

#### 1.4 User Usage: Zero unsafe, Zero Raw Pointers

```yaoxiang
import sqlite3_bindings

db = SqliteDb.open("test.db")
db.exec("SELECT * FROM users")
// ← Scope ends, RAII automatically calls SqliteDb.drop → sqlite3_close(db)
```

---

### 2. Type Dichotomy: Layout Ownership Pinned at Definition

When external data enters YaoXiang, only one question matters: **who decides the layout of this
memory?**

```
├─ Layout is external's black box (sqlite3, FILE*, socket fd)
│   → Opaque handle  =  lib("symbol")
│   → YaoXiang only holds pointer, never dereferences, passes only between library functions
│   → External code reads its own memory, YaoXiang doesn't touch
│
└─ Layout is YaoXiang's definition (timespec, point, struct with fields to read)
    → Transparent type  =  { field: Type, ... }
    → YaoXiang owns memory, defines layout, reads/writes fields
    → External code writes/reads into YaoXiang-defined layout memory
```

**No third option.** The previous design's "three-layer memory mode (copy/take/system-level)" was
patch thinking — the truth is binary layout ownership.

#### 2.1 Opaque Handles: Layout Belongs to External

```yaoxiang
SqliteDb: Type = sqlite3("sqlite3")
```

- YaoXiang internally holds only a pointer-sized handle
- User cannot construct (`SqliteDb {}` → compile error), cannot access fields (no fields to access)
- Sole source: external functions returning `SqliteDb`
- When calling methods, borrow handle back to library, library reads **its own** memory (`sqlite3`
  struct on library's heap)

External code "reading internals" reads structs it allocated; YaoXiang merely transports handles. No
memory conflict.

#### 2.2 Transparent Types: Layout Belongs to YaoXiang

```yaoxiang
// Fields matter, need to read/write → transparent type, YaoXiang declares layout
Timespec: Type = {
    tv_sec: Int64,
    tv_nsec: Int64
}
clock_gettime: (clk: Int32, ts: *Timespec) -> Int32 = Native.c("librt")("clock_gettime")

ts = clock_gettime(CLOCK_REALTIME)   // See §3, marshaling goes through temporary area
print(ts.tv_sec)                      // YaoXiang reads according to its own field definition
```

External code reads/writes into memory **YaoXiang defines layout for and owns**. Layout is
YaoXiang's contract, not external's.

#### 2.3 Decision Rules

User only needs to decide one thing: **do I need to read this type's fields?**

| Decision                                                 | Type                         | Layout Ownership |
| -------------------------------------------------------- | ---------------------------- | ---------------- |
| Don't read fields, pass handle between library functions | Opaque handle `= lib("sym")` | External         |
| Need to read/write fields                                | Transparent type `{ ... }`   | YaoXiang         |

---

### 3. Marshaling: Signature-Driven, Temporary Area Isolation

Cross-boundary data conversion is **signature-driven**, compiler determines conversion rules for
each parameter position at compile-time. **Core safety guarantee: external code reads/writes the
marshaling temporary area, not YaoXiang heap objects.**

#### 3.1 Default Copy Through Temporary Area

```
YaoXiang → C (input parameter):
    Copy data to call temporary area → pass temporary area pointer to C
    → C out-of-bounds/corruption only damages temporary area, YaoXiang heap objects isolated

C → YaoXiang (return/output parameter):
    C writes temporary area → YaoXiang memcpy back to its own object
    → C cannot touch YaoXiang's final object
```

**External code always reads/writes the marshaling temporary area, completely isolated from YaoXiang
heap objects.** Wrong layout declaration, C stores dangling pointer, C out-of-bounds — all only
damage temporary area, YaoXiang objects intact. Cost: one memcpy.

#### 3.2 Marshaling Rules Table

**Input direction (YaoXiang → C)**:

| YaoXiang Type       | C Representation  | Marshaling Action                                    | Ownership                          |
| ------------------- | ----------------- | ---------------------------------------------------- | ---------------------------------- |
| `Int32/Int64/Float` | `int/long/double` | Directly placed in register, zero conversion         | Value semantics                    |
| `String`            | `const char*`     | Borrow read-only view (temporary, valid during call) | YaoXiang retains, C read-only      |
| Transparent type    | `struct T*`       | Copy to temporary area, pass temporary area pointer  | YaoXiang owns object, C reads copy |
| Opaque handle       | `void*`           | Extract internal handle pointer                      | YaoXiang holds, borrowed to C      |
| `*T`                | `T*`              | Directly pass raw pointer (unsafe)                   | User responsible                   |

**Return direction (C → YaoXiang)**:

| C Return                     | YaoXiang Type    | Marshaling Action                            | Ownership                              |
| ---------------------------- | ---------------- | -------------------------------------------- | -------------------------------------- |
| `int/double`                 | `Int32/Float`    | Directly read from register                  | Value semantics                        |
| `char*`                      | `String`         | strlen + memcpy to YaoXiang String           | YaoXiang owns copy, original untouched |
| `struct T*` (new handle)     | Opaque handle    | Handle stored in YaoXiang object             | YaoXiang takes over                    |
| `struct T` (value/out param) | Transparent type | C writes temporary area → memcpy to YaoXiang | YaoXiang owns                          |
| `char*` (static region)      | `*const U8`      | Store raw pointer, don't copy (unsafe read)  | Not taken over, user responsible       |

#### 3.3 Borrow Lifetime: Strictly Scoped to Single Call

Pointers YaoXiang borrows to external code (String read-only view, transparent type temporary area,
handle) have **lifetime strictly scoped to single call**:

- During call: pointer valid, external code can read/write
- After call returns: borrow immediately invalid

If external code stores the pointer for use after the call, that's external code violating the FFI
standard contract (equivalent to library bug), YaoXiang not responsible. Consistent with all
languages' C FFI contracts (Rust's `&T` passed to C has same constraint).

#### 3.4 String Never Surrenders Persistent Pointer

`String` is key to "C doesn't touch YaoXiang memory":

- Into C: borrow **temporary read-only view**, valid during call
- Out of C: strlen + memcpy into YaoXiang-owned **copy**

C never gets YaoXiang String's persistent pointer, YaoXiang never holds C `char*`'s long-term
reference. Structurally isolated.

---

### 4. Ownership and Lifetime: Move + RAII

Opaque handles follow RFC-009 ownership model, zero new concepts.

#### 4.1 Core Principles

- **Move semantics**: Opaque handles are Move by default, assignment/parameter passing/return =
  ownership transfer, not copyable
- **Handle unique ownership**: At any moment a handle has exactly one owner → structurally prevents
  double-free
- **RAII release**: At scope end, if `.drop` is bound, automatically called
- **Consume tracking**: After explicit destruction or Move, variable consumed, cannot be used →
  prevents use-after-free

#### 4.2 `.drop` Is Optional External Side Effect

```yaoxiang
SqliteDb.drop = SqliteDb.close     // At scope end calls sqlite3_close
```

**`.drop` is not a mechanism to prevent YaoXiang leaks** — YaoXiang-side handle storage (a
pointer-sized value) is automatically reclaimed, unrelated to `.drop`. `.drop` is an **optional side
effect of calling an external function at scope end**:

- `.drop` bound → called at scope end (clean up external resources)
- `.drop` not bound → does nothing, **no error, no warning**

Whether external resources need cleanup is a matter of external library specification (`getenv`
returns static region, shouldn't free; global singletons shouldn't free), YaoXiang doesn't overstep.
Leak prevention relies on Move + unique ownership (unconditional, structural), not `.drop`.

#### 4.3 Automatic Destruction and Order

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    // ← Scope ends, reverse order automatic destruction (only calls if .drop bound):
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

Destruction order: reverse of definition order, consistent with RAII.

#### 4.4 Move and Consume

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred
db.exec("...")          // ❌ Compile error: db already moved, cannot read after consume

process_db: (db: SqliteDb) -> Void = {
    db.exec("...")
    // ← Function ends, db destroyed here
}
process_db(some_db)     // Move into function
// some_db invalid here
```

#### 4.5 Null Handling

```yaoxiang
// May return null → ?T, user must handle
SqliteDb.open: (file: String) -> ?SqliteDb = sqlite3("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => db.exec("SELECT 1"),
    None => print("Open failed")
}

// Convention of never returning null → no marking, null causes panic
```

C returns null either user handles (`?T`) or panic reveals. No third "silently ignore" option.

#### 4.6 Destruction Failure Handling

Return value of `.drop` bound function determines behavior:

| `.drop` Return Type  | Behavior                                                                                     |
| -------------------- | -------------------------------------------------------------------------------------------- |
| `Void`               | No failure                                                                                   |
| `Int32` (error code) | Non-zero causes panic — destruction failure means abnormal state, expose better than silence |
| `?Error`             | Non-None causes panic — same as above                                                        |

Destruction failure cannot be silent. To ignore specific errors when needed, explicitly handle in
wrapper function bound to `.drop`.

---

### 5. FFI Behavior in Spawn Blocks

Resource type determination decided by `.drop` binding (RFC-024), zero extra marking:

| Determination                       | Behavior                                                                         |
| ----------------------------------- | -------------------------------------------------------------------------------- |
| Opaque handle with `.drop` bound    | Resource type — same instance operations in spawn block automatically serialized |
| Opaque handle without `.drop` bound | Non-resource type — can parallelize (pure data handle, no release side effects)  |
| Transparent type / value type       | Non-resource type — can parallelize                                              |

```yaoxiang
SqliteDb.drop = SqliteDb.close   // → resource type

(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Same instance, automatically serialized
    r2 = db.exec("INSERT ...")    // Waits for r1
}

(x, y) = spawn {
    db1 = SqliteDb.open("a.db"),   // Different instances, can parallelize
    db2 = SqliteDb.open("b.db")
}
```

Types with `.drop` automatically serialize same-instance operations in spawn, ensuring destruction
has no concurrent race.

---

### 6. Escape Hatch: Raw Pointers + unsafe

Default marshaling copies through temporary area, safe but has memcpy overhead. For
performance-sensitive scenarios (large structures, high-frequency calls) requiring zero-copy, user
explicitly takes the raw pointer escape hatch:

```yaoxiang
// C directly reads YaoXiang memory, zero-copy — user explicitly accepts risk
ptr: *const U8 = Native.c("libc")("getenv")("HOME")
unsafe {
    value = read_c_string(ptr)   // User guarantees ptr valid
}
```

**`unsafe` only used for raw pointer operations, completely orthogonal to opaque handles and
transparent types.** Normal FFI (handles + transparent types) doesn't need unsafe. Writing
`unsafe {}` = user explicitly signs accepting direct memory access risk.

**Trust boundary**: C cannot provide compile-time verifiable type contracts (`.h` is not ABI
contract, symbol table only has names not signatures). So C signature correctness cannot be
automatically verified — guaranteed by binding author when writing `Native.c(...)` + signature.
**Trust localized at binding declarations**: binding author guarantees, package users get safe API.
Consistent with Rust's `extern "C"` (writing extern is trust behavior, safe wrapper packages it up,
then calls safe).

---

## Trade-offs

### Advantages

1. **Complete information**: Library linked at compile-time, symbols verified at compile-time, no
   runtime "library not found" ambiguity
2. **Explicit layout ownership**: Type dichotomy, pinned at definition, no runtime inference
3. **Structural safety**: Temporary area isolation + Move + RAII, external code cannot touch
   YaoXiang heap objects
4. **Zero new keywords**: `Native.c` currying + `name: type = value`, all reuse existing syntax
5. **Honest boundary**: Doesn't pretend to verify C signatures, trust localized at declarations

### Disadvantages

1. **memcpy overhead**: Default marshaling copies, large structures high-frequency calls need
   explicit escape hatch
2. **Layout guarantee is manual**: Transparent type layout matching C struct guaranteed by binding
   author/yx-bindgen
3. **C signatures not compile-time verifiable**: Fundamental FFI limitation, YaoXiang cannot
   eliminate from C

---

## Implementation Strategy

### Phase 1: External Libraries and Symbols (v0.8)

- [ ] Implement `Native.c("lib")` compile-time link + return resolver value
- [ ] Implement symbol resolver application (`lib("symbol")`) + compile-time symbol table
      verification
- [ ] Implement type dichotomy (opaque handles / transparent types)
- [ ] Implement method binding (direct binding + `[N]` position binding)

### Phase 2: Marshaling and Safety (v0.8)

- [ ] Implement signature-driven marshaling code generation
- [ ] Implement temporary area copy isolation (input copy, return memcpy)
- [ ] Implement String temporary read-only view + return copy
- [ ] Implement borrow lifetime scoped to single call

### Phase 3: Ownership and Lifetime (v0.9)

- [ ] Implement opaque handle Move + unique ownership
- [ ] Implement `.drop` RAII automatic destruction (optional, absence not an error)
- [ ] Implement consume tracking (disabled after Move)
- [ ] Implement `?T` integration with null returns
- [ ] Implement spawn resource type serialization

### Future Work

- **Extensible FFI mechanism** (RFC-026a): `FfiMechanism` abstraction, `.wasm`/`.python` etc
  plugins, dynamic loading
- **yx-bindgen** (RFC-026b): C header → `.yx` bindings + platform-correct layout generation

---

## Relationship to Other RFCs

- **RFC-004**: Curried multi-position binding — source of `[N]` method binding syntax
- **RFC-007**: Unified function definition syntax — `Native.c(...)` binding is `name: type = value`
- **RFC-009**: Ownership model — Move, RAII, `?T`, handle lifetime entirely based on this
- **RFC-010**: Unified type syntax — LHS type annotation determines binding as type or function
- **RFC-024**: Concurrency model — spawn resource type determination based on `.drop`
- **RFC-020/021** (deprecated): Content merged into this document
- **RFC-026a**: Extensible FFI mechanism system
- **RFC-026b**: yx-bindgen toolchain

---

## Design Decision Log

| Decision                                | Decision                                          | Reason                                                                                                                         | Date       |
| --------------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ---------- |
| **Library as value**                    | `Native.c("lib")` currying returns resolver       | Library info becomes compile-time visible first-class value, fills "which library" gap, zero new keywords                      | 2026-07-03 |
| **Compile-time link**                   | `Native.c("lib")` triggers `-llib`                | Symbol table readable at compile-time, symbol existence verifiable, type is real                                               | 2026-07-03 |
| **Type dichotomy**                      | Opaque handle / transparent type                  | Binary layout ownership covers all cases; removes "three-layer memory mode" patch                                              | 2026-07-03 |
| **Marshaling temporary area isolation** | Default copy, heap objects isolated from external | External out-of-bounds/dangling only damages temporary area, YaoXiang objects intact; zero-copy requires explicit escape hatch | 2026-07-03 |
| **`.drop` optional**                    | Absence does nothing, no error                    | YaoXiang handle storage automatically reclaimed; external resource cleanup is external specification, no overstep              | 2026-07-03 |
| **Leak prevention mechanism**           | Move + handle unique ownership (unconditional)    | Structural guarantee, unrelated to `.drop`                                                                                     | 2026-07-03 |
| **Trust boundary**                      | At `Native.c(...)` declaration                    | C signatures not compile-time verifiable, trust localized, unsafe only for raw pointers                                        | 2026-07-03 |
| **Null handling**                       | `?T` or panic                                     | C's problem not hidden, no "silently ignore" option                                                                            | 2026-07-03 |
| **Destruction failure**                 | `.drop` return type determines, uniform panic     | Destruction failure cannot be silent                                                                                           | 2026-07-03 |

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

| Status       | Location                    | Description            |
| ------------ | --------------------------- | ---------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |
