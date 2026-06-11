---
title: "RFC-025: Extensible Primitive Type Mechanism"
status: "Under Review"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-06-05"
---

# RFC-025: Extensible Primitive Type Mechanism

## Abstract

This document defines the **Extensible Primitive Type Mechanism** (`Primitive::Extension`) of the YaoXiang compiler. It allows external code to register custom primitive types with the compiler, enabling support for domain-specific types (qubits, GPU buffers, SIMD vectors, hardware registers, etc.) without hardcoding them into the compiler.

## Motivation

### Why is this mechanism needed?

The current compiler hardcodes all primitive types: `Int`, `Float`, `String`, `Bool`, `Unit`. Adding a new primitive type requires modifying multiple locations in the compiler source code — the type checker, code generator, ownership analyzer, and optimizer.

This violates the open-closed principle: open for extension, closed for modification.

### Design Boundary

```
Hardcoded core types (language foundation):  Int, Float, String, Bool, Unit
Dynamically extended types (domain plugins):  Registered via Primitive::Extension
```

The core types are hardcoded because the compiler deeply depends on their semantics (arithmetic operations, conditional branches, hashing, comparison). Extended types are **opaque values** to the compiler — the compiler only knows their size, alignment, and ownership properties, not their internal semantics.

This is not "unify all types into dynamic loading." Core types and extended types are two different things.

## Proposal

### Core Design

#### 1. Extension Type Properties

Each registered extended primitive type must declare the following properties:

```rust
pub struct PrimitiveExtension {
    /// Type name, e.g., "Qubit", "Buffer", "Vec128"
    pub name: String,

    /// Byte size (fixed-size type)
    /// None indicates the size is unknown at compile-time (resolved at runtime)
    pub size: Option<usize>,

    /// Alignment requirement
    pub align: Option<usize>,

    /// Whether implicit copying is allowed
    /// false = Move semantics (assignment moves, e.g., Qubit)
    /// true = Copy semantics (assignment copies, e.g., Vec128)
    pub is_copy: bool,

    /// Whether zero-sized values are allowed (zero-sized type)
    pub allow_zst: bool,
}
```

#### 2. Registration Interface

```rust
// Compiler internal API
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".to_string(),
    size: Some(0),           // logical size is 0; physical state lives on the QPU
    align: Some(1),
    is_copy: false,          // Move semantics, consistent with no-cloning
    allow_zst: true,
});
```

After registration, `Qubit` becomes a legal primitive type in the type system, usable for variable declarations, function parameters, and struct fields.

#### 3. Type Checker Behavior

Extended primitive types follow these rules during type checking:

| Scenario | Behavior |
|------|------|
| Variable declaration `q: Qubit = ...` | ✅ Legal |
| Function parameter `fn(q: Qubit)` | ✅ Legal |
| Struct field `{ q: Qubit }` | ✅ Legal |
| Assignment when `is_copy == false` | Move semantics; the original variable is invalidated |
| Assignment when `is_copy == true` | Copy semantics; the original variable remains valid |
| Implicit copying (used in multiple places in a function) | Depends on `is_copy` |
| Comparison `==`, `!=` | ❌ Compile error (no built-in comparison) |
| Arithmetic `+`, `-` | ❌ Compile error (no built-in operations) |
| Generic constraint `T: Copy` | Satisfied only when `is_copy == true` |

#### 4. Code Generator Behavior

Extended primitive types are treated as **opaque values** during code generation:

- LLVM IR: emitted as `{size} x i8` or a struct of corresponding size
- No special instructions are generated — semantics are handled by the backend or library
- If the backend requires special handling (e.g., QIR quantum gates), this is achieved through the backend registration mechanism (out of scope for this RFC)

### Examples

#### Registering a Move-Semantics Type

```rust
// Qubit: non-copyable, size 0 (physical state lives on the QPU)
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".into(),
    size: Some(0),
    align: Some(1),
    is_copy: false,
    allow_zst: true,
});
```

```yaoxiang
# User code
q: Qubit = qubit(0)
q2 = q          # ❌ Compile error: Qubit is a Move type; q is invalidated
q = H(q)        # ✅ Consumes q, returns a new q
```

#### Registering a Copy-Semantics Type

```rust
// SIMD vector: copyable, fixed size
compiler.register_primitive(PrimitiveExtension {
    name: "Vec128".into(),
    size: Some(16),
    align: Some(16),
    is_copy: true,
    allow_zst: false,
});
```

```yaoxiang
# User code
a: Vec128 = load_vec128(data)
b = a           # ✅ Copy semantics; a remains valid
c = add_vec128(a, b)  # ✅ Both a and b are usable
```

## Detailed Design

### Compiler Changes

| Component | Change |
|------|------|
| Type system | Add a `Ty::Extension` variant storing `PrimitiveExtension` metadata |
| Type checker | Extended types do not participate in built-in operation resolution and do not satisfy built-in trait constraints (unless explicitly implemented) |
| Ownership analyzer | Determines Move or Copy semantics based on `is_copy` |
| Code generator | Emits opaque values per `size`/`align`; no special instructions |
| Error messages | Error messages for extended types reference the `name` from registration |

### Relationship with FFI

`Primitive::Extension` is orthogonal to RFC-021 (FFI):

| | Primitive::Extension | FFI |
|---|---|---|
| Purpose | Register a new **type** | Call external **functions** |
| Layer | Type system | Runtime |
| Example | `Qubit` is a type | `native("sin")` is a function call |

A domain may require both: the `Qubit` type is registered via Extension, and quantum gate functions are registered via FFI.

### Backward Compatibility

- ✅ Fully backward compatible
- The semantics of no existing type is modified
- Extended types are an additive capability and do not affect existing code

## Trade-offs

### Advantages

- ✅ The compiler does not need source-code changes for each new domain
- ✅ Domain experts can register types themselves without depending on the compiler team
- ✅ Core types remain hardcoded, so the compiler's deep optimizations for basic types are not sacrificed
- ✅ Simple interface — a single struct defines all properties

### Disadvantages

- ⚠️ Extended types do not support built-in operations — additional functions or backend mechanisms are required to implement semantics
- ⚠️ Extended types appear as opaque values during debugging, which is less intuitive than core types

## Alternatives

| Alternative | Why not chosen |
|------|--------------|
| Dynamic loading of all types | Core types (Int/Float/Bool) require deep compiler optimization; dynamic loading would lose these capabilities |
| Hardcoding for every domain | The compiler would need modification for each new domain — not extensible |
| Pure library approach (no type registration) | Cannot guarantee semantics (such as no-cloning) at the type-system level; only runtime checks are possible |

## Implementation Strategy

### Phase 1: Core Interface

- [ ] Add the `Ty::Extension` variant to the type system
- [ ] Implement the `register_primitive` API
- [ ] Extend the type checker to handle `is_copy` semantics
- [ ] Extend the code generator to handle opaque values
- [ ] Unit tests

### Phase 2: Registration Timing

- [ ] Support batch registration during compiler initialization (configuration file or builder API)
- [ ] Support standard library pre-registration (the `std.primitive` module exports extended type definitions)

### Dependencies

- No hard dependencies. Can be implemented independently of other RFCs.

## Open Questions

- [ ] Should extended types support trait implementation (e.g., having `Qubit` implement a custom `QuantumGate` trait)?
- [ ] Are lifetime hooks (such as `on_drop`) needed to support RAII-semantic extended types?
- [ ] Configuration file format: TOML, YAML, or YaoXiang's own configuration syntax (see RFC-015)?

---

## Design Decision Records

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| Core types are not dynamically loaded | Keep hardcoded | The compiler deeply depends on the semantics of core types; dynamic loading yields no benefit | 2026-06-05 |
| Extended types are opaque values | Do not inject semantics | Semantics are handled by the backend/library; the compiler only guarantees type safety and ownership | 2026-06-05 |
| Orthogonal to FFI | Do not merge | Type registration and function calls are different abstraction layers | 2026-06-05 |