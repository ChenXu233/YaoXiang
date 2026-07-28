---
title: 'RFC-026b: yx-bindgen Toolchain'
status: 'Draft'
author: 'Chen Xu'
created: '2026-06-05'
updated: '2026-07-03'
group: 'rfc-026'
---

# RFC-026b: yx-bindgen Toolchain

> **Parent RFC**: [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md)
>
> **Dependency**: The implementation of this RFC depends on RFC-026's core FFI mechanism being landed first.

## Summary

yx-bindgen mechanically converts C header files into `.yx` FFI binding files, generating library bindings, type dichotomy (opaque handles / transparent types), function declarations, and method bindings.

**Two Core Principles**:

1. **Mechanical output draft, don't guess ownership** — Ownership is determined by YaoXiang types; users confirm according to C documentation
2. **Platform-correct layout guarantee** — Field sizes and alignment for transparent types are calculated for the target platform, ensuring binary compatibility with C structs

## Motivation

Manually writing FFI bindings is tedious and error-prone — C libraries have dozens or hundreds of functions. More dangerous is the **layout of transparent types**: when manually writing
`Timespec: Type = { tv_sec: Int64, tv_nsec: Int64 }`,
field sizes, alignment, and padding must precisely match the target platform's C `struct timespec`, otherwise C reading/writing with the wrong layout will cause out-of-bounds access (the trust guarantee in RFC-026 §2.2). yx-bindgen mechanically computes layouts from `.h` files, eliminating this manual guarantee risk.

## Proposal

### 1. Usage

```bash
yx-bindgen --header /usr/include/sqlite3.h --lib sqlite3 --output sqlite3_bindings.yx
```

`--lib` specifies the library name to link, generating `Native.c("libsqlite3")` header.

### 2. Generated Content

yx-bindgen generates four types of content:

- **Library binding header**: `lib = Native.c("libxxx")`
- **Type dichotomy**: Black-box pointers → opaque handles; data structures → transparent types (with layout)
- **Function declarations**: `lib("symbol")` bindings
- **Method bindings**: `Type.method` or `[N]`

### 3. Automatic Type Dichotomy Determination

yx-bindgen determines classification based on how C types are used (RFC-026 §2):

| C Type                                         | Classification     | yx-bindgen Output             |
| ---------------------------------------------- | ------------------ | ----------------------------- |
| `typedef struct T T;` (incomplete type/pointer-only use) | Black-box → opaque handle | `T: Type = lib("T")`          |
| `struct T { fields };` (fields visible, read/written) | Data → transparent type   | `T: Type = { ...layout }` |
| `int/long/float/double`                        | Value              | `Int32/Int64/Float32/Float64` |
| `char*` (parameter/return)                     | Value (copy)       | `String`                      |
| `void*`                                        | Untyped            | `*Void` (system-level)        |

Incomplete types (`typedef struct sqlite3 sqlite3;` without definition) → must be black-box handles. Fully defined structs → transparent types, field layout mechanically calculated.

### 4. Layout Calculation (Key to Transparent Types)

For fully defined structs, yx-bindgen calculates each field's offset, size, and alignment according to the target platform's ABI:

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
// Total size 16, alignment 8 — binary consistent with C struct
```

**Platform differences**: `time_t`, `long`, `size_t` sizes vary by platform. yx-bindgen selects the correct mapping based on `--target`, ensuring generated transparent types byte-match the target platform's C struct. This is the core value for eliminating the manual layout guarantee risk in RFC-026 §2.2.

### 5. Generation Example

Input (`sqlite3.h` abstraction):

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
// Library Bindings
// ============================================================================
sqlite3 = Native.c("libsqlite3")

// ============================================================================
// Types (incomplete types → opaque handles)
// ============================================================================
SqliteDb: Type = sqlite3("sqlite3")
SqliteStmt: Type = sqlite3("sqlite3_stmt")

// ============================================================================
// Function + Method Bindings
// ============================================================================
SqliteDb.open: (filename: String) -> ?SqliteDb = sqlite3("sqlite3_open")
SqliteDb.exec: (sql: String) -> Int32 = sqlite3("sqlite3_exec")
SqliteDb.close: () -> Int32 = sqlite3("sqlite3_close")

// ============================================================================
// Destructor (optional, user confirms)
// ============================================================================
SqliteDb.drop = SqliteDb.close
```

### 6. User Adjustments

Generated bindings are drafts; users confirm ownership semantics according to C library documentation (yx-bindgen doesn't guess):

```yaoxiang
// Generated: default String (copy) — correct for most cases
SqliteDb.errmsg: () -> String = sqlite3("sqlite3_errmsg")

// getenv returns static storage, should neither copy nor take ownership — user changes to raw pointer
getenv: (name: String) -> *const U8 = Native.c("libc")("getenv")

// Library-owned handles, user confirms whether to bind .drop
SqliteDb.drop = SqliteDb.close   // generated suggestion, user confirms
```

**Key point**: Layout is mechanically guaranteed by yx-bindgen (platform-correct), but ownership semantics (whether to `.drop`, whether `char*` copies or is a raw pointer) are confirmed by users according to C documentation. The tool handles mechanical correctness; users handle semantic correctness.

---

## Trade-offs

### Advantages

1. **Layout guarantee automation**: Transparent type layout is mechanically calculated per platform, eliminating manual padding/alignment errors
2. **Automatic type dichotomy determination**: Incomplete types → handles, complete structs → transparent types
3. **Auditable**: Output is plain `.yx`, readable by users, editable, and commit-able to version control

### Disadvantages

1. **Ownership still requires human confirmation**: yx-bindgen doesn't guess `.drop` or `char*` semantics
2. **C header parsing dependency**: Requires libclang or tree-sitter-c
3. **Platform-specific**: Different `--target` generates different layouts; cross-platform packages need multiple copies or runtime selection

---

## Implementation Strategy

- [ ] C header file parsing (libclang)
- [ ] Type dichotomy determination (incomplete types vs complete structs)
- [ ] Platform ABI layout calculation (offset/size/align, per --target)
- [ ] Code generation (library bindings + types + functions + methods)
- [ ] Integration tests (sqlite3, libcurl, multi-platform layout verification)

---

## Relationship with Other RFCs

- **RFC-026** (parent): FFI core mechanism — generated bindings use its `Native.c("lib")("sym")` syntax and type dichotomy
- **RFC-026a**: Extensible FFI mechanism — future extension could generate bindings for other mechanisms like `Native.wasm`

---

## Design Decision Record

| Decision         | Decision                                      | Reason                                             | Date       |
| ---------------- | --------------------------------------------- | -------------------------------------------------- | ---------- |
| **Positioning**  | Mechanical output draft, don't guess ownership | Ownership is determined by YaoXiang types, users confirm via C docs | 2026-07-03 |
| **Layout guarantee** | Mechanical calculation of offset/size/align per --target | Eliminate out-of-bounds risk from manual layout (RFC-026 §2.2) | 2026-07-03 |
| **Type determination** | Incomplete types → handles, complete structs → transparent types | Align with RFC-026 type dichotomy | 2026-07-03 |

---

## Lifecycle and Destination

| Status       | Location                        | Description                      |
| ------------ | ------------------------------- | -------------------------------- |
| **Draft**    | `docs/design/rfc/draft/`        | Depends on RFC-026 being landed  |
| **Accepted** | `docs/design/rfc/accepted/`     | Official design document         |
