---
title: "RFC-026b: yx-bindgen Toolchain"
status: "Draft"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-07-03"
group: "rfc-026"
---

# RFC-026b: yx-bindgen Toolchain

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> **Dependency**: The implementation of this RFC depends on the core FFI mechanism from RFC-026 being landed first.

## Summary

yx-bindgen mechanically converts C header files into `.yx` FFI binding files, producing library bindings, type dichotomies (opaque handles / transparent types), function declarations, and method bindings.

**Two core principles**:
1. **Mechanical output as a draft, no guessing of ownership** — ownership is conferred by the YaoXiang type system, and the user confirms based on the C documentation.
2. **Platform-correct layout guarantee** — field sizes and alignment of transparent types are calculated according to the target platform, ensuring binary compatibility with the C struct.

## Motivation

Writing FFI bindings by hand is tedious and error-prone — C libraries have dozens or hundreds of functions. Even more dangerous is the **layout of transparent types**: when manually writing `Timespec: Type = { tv_sec: Int64, tv_nsec: Int64 }`, the field sizes, alignment, and padding must exactly match the C `struct timespec` on the target platform, otherwise C reads and writes following the wrong layout will go out of bounds (the trust guarantee in RFC-026 §2.2). yx-bindgen mechanically computes layouts from `.h` files, eliminating the risk of this manual guarantee.

## Proposal

### 1. Usage

```bash
yx-bindgen --header /usr/include/sqlite3.h --lib sqlite3 --output sqlite3_bindings.yx
```

`--lib` specifies the linked library name, and generates a `Native.c("libsqlite3")` header.

### 2. Generated Content

yx-bindgen generates four categories of content:
- **Library binding header**: `lib = Native.c("libxxx")`
- **Type dichotomy**: black-box pointer → opaque handle; data structure → transparent type (with layout)
- **Function declarations**: `lib("symbol")` bindings
- **Method bindings**: `Type.method` or `[N]`

### 3. Automatic Determination of Type Dichotomy

yx-bindgen determines categorization based on how the C type is used (RFC-026 §2):

| C Type | Determination | yx-bindgen Output |
|--------|---------------|-------------------|
| `typedef struct T T;` (incomplete type / only used by pointer) | Black box → opaque handle | `T: Type = lib("T")` |
| `struct T { fields };` (fields visible, read/written) | Data → transparent type | `T: Type = { ...layout }` |
| `int/long/float/double` | Value | `Int32/Int64/Float32/Float64` |
| `char*` (parameter/return) | Value (copy) | `String` |
| `void*` | Untyped | `*Void` (system-level) |

Incomplete types (e.g., `typedef struct sqlite3 sqlite3;` without a defining body) → necessarily black-box handles. Fully defined structs → transparent types, with field layout computed mechanically.

### 4. Layout Computation (Key to Transparent Types)

For fully defined structs, yx-bindgen computes each field's offset, size, and alignment based on the target platform's ABI:

```c
struct timespec {
    time_t tv_sec;    // platform-dependent: Linux x86_64 = 8 bytes
    long   tv_nsec;   // 8 bytes
};
```

```yaoxiang
// Target platform Linux x86_64
Timespec: Type = {
    tv_sec: Int64,    // offset 0, size 8
    tv_nsec: Int64    // offset 8, size 8
}
// Total size 16, alignment 8 — binary-identical to C struct
```

**Platform differences**: the sizes of `time_t`, `long`, `size_t` vary by platform. yx-bindgen selects the correct mapping according to `--target`, ensuring the generated transparent type matches the C struct byte-for-byte on the target platform. This is the core value that eliminates the manual layout guarantee risk in RFC-026 §2.2.

### 5. Generation Example

Input (abstraction of `sqlite3.h`):

```c
typedef struct sqlite3 sqlite3;          // incomplete → black-box handle
typedef struct sqlite3_stmt sqlite3_stmt;

int sqlite3_open(const char *filename, sqlite3 **ppDb);
int sqlite3_close(sqlite3 *db);
int sqlite3_exec(sqlite3 *db, const char *sql, ...);
```

Output (`sqlite3_bindings.yx`):

```yaoxiang
// sqlite3_bindings.yx —— auto-generated, do not edit manually

// ============================================================================
// Library Binding
// ============================================================================
sqlite3 = Native.c("libsqlite3")

// ============================================================================
// Types (incomplete types → opaque handles)
// ============================================================================
SqliteDb: Type = sqlite3("sqlite3")
SqliteStmt: Type = sqlite3("sqlite3_stmt")

// ============================================================================
// Functions + Method Bindings
// ============================================================================
SqliteDb.open: (filename: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// ============================================================================
// Destructor (optional, user confirmation)
// ============================================================================
SqliteDb.drop = SqliteDb.close
```

### 6. User Adjustments

The generated bindings are a draft; the user confirms the ownership semantics based on the C library documentation (yx-bindgen does not guess):

```yaoxiang
// Generated: default String (copy) — correct in most cases
SqliteDb.errmsg: () -> String = sqlite3("sqlite3_errmsg")

// getenv returns from static storage; should not be copied or taken over — user changes to raw pointer
getenv: (name: String) -> *const U8 = Native.c("libc")("getenv")

// Library-owned handle, user confirms whether to bind .drop
SqliteDb.drop = SqliteDb.close   // generated suggestion, user confirms
```

**Key point**: the layout is mechanically guaranteed by yx-bindgen (platform-correct), but ownership semantics (whether to `.drop`, whether `char*` is a copy or a raw pointer) are confirmed by the user based on the C documentation. The tool is responsible for mechanical correctness; the user is responsible for semantic correctness.

---

## Trade-offs

### Advantages

1. **Automated layout guarantee**: transparent type layouts are computed mechanically per platform, eliminating manual padding/alignment errors.
2. **Automatic type dichotomy determination**: incomplete types → handles, complete structs → transparent types.
3. **Auditable**: the output is plain `.yx`, which the user can read, modify, and commit to version control.

### Disadvantages

1. **Ownership still requires manual confirmation**: yx-bindgen does not guess `.drop`, nor does it guess `char*` semantics.
2. **C header parsing dependency**: requires libclang or tree-sitter-c.
3. **Platform specialization**: different `--target` produces different layouts; cross-platform packages need multiple copies or runtime selection.

---

## Implementation Strategy

- [ ] C header file parsing (libclang)
- [ ] Type dichotomy determination (incomplete type vs. complete struct)
- [ ] Platform ABI layout computation (offset/size/align, by `--target`)
- [ ] Code generation (library binding + types + functions + methods)
- [ ] Integration tests (sqlite3, libcurl; multi-platform layout validation)

---

## Relationship to Other RFCs

- **RFC-026** (parent): FFI core mechanism — the generated bindings use its `Native.c("lib")("sym")` syntax and type dichotomy.
- **RFC-026a**: Extensible FFI Mechanism — in the future, bindings for other mechanisms such as `Native.wasm` can be extended.

---

## Design Decision Record

| Decision | Resolution | Reason | Date |
|----------|------------|--------|------|
| **Position** | Mechanical output as a draft, no guessing of ownership | Ownership is conferred by the YaoXiang type system, user confirms based on C documentation | 2026-07-03 |
| **Layout guarantee** | Mechanically compute offset/size/align by `--target` | Eliminates the risk of manual layout out-of-bounds (RFC-026 §2.2) | 2026-07-03 |
| **Type determination** | Incomplete type → handle, complete struct → transparent type | Aligns with the type dichotomy in RFC-026 | 2026-07-03 |

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/draft/` | Depends on RFC-026 being landed first |
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |