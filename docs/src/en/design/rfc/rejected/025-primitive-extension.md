---
title: "RFC-025: Extensible Primitive Type Mechanism"
status: "Rejected"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-07-03"
rejected: "2026-07-03"
reason: "Industry practice has shown that languages do not need a generic primitive type registration mechanism. Domain types are handled through standard library struct wrappers, FFI opaque handles (RFC-026 §2.1), or compiler backend branches. This RFC attempts to solve a non-existent generic problem, and the is_copy field oversimplifies the ownership semantics of RFC-009."
---

# RFC-025: Extensible Primitive Type Mechanism

## Abstract

This document defines YaoXiang compiler's **Extensible Primitive Type Mechanism** (`Primitive::Extension`). It allows external code to register custom primitive types with the compiler, enabling the compiler to support domain-specific types (qubits, GPU buffers, SIMD vectors, hardware registers, etc.) without hardcoding them.

## Motivation

### Why is this mechanism needed?

The current compiler hardcodes all primitive types: `Int`, `Float`, `String`, `Bool`, `Unit`. Each new primitive type added requires modifying multiple locations in the compiler source code — the type checker, code generator, ownership analyzer, and optimizer.

This violates the open-closed principle: open to extension, closed to modification.

### Design Boundary

```
Hardcoded core types (language foundation):  Int, Float, String, Bool, Unit
Dynamically extended types (domain plugins): Registered via Primitive::Extension
```

Core types are hardcoded because the compiler relies deeply on their semantics (arithmetic operations, conditional branching, hashing, comparison). Extended types are **opaque values** to the compiler — the compiler only knows their size, alignment, and ownership properties, not their internal semantics.

This is not "unifying all types into dynamically loaded ones." Core types and extended types are two different things.

## Proposal

### Core Design

#### 1. Extension Type Attributes

Each extended primitive type registration must declare the following attributes:

```rust
pub struct PrimitiveExtension {
    /// Type name, e.g. "Qubit", "Buffer", "Vec128"
    pub name: String,

    /// Byte size (fixed-size type)
    /// None means the size is unknown at compile-time (determined at runtime)
    pub size: Option<usize>,

    /// Alignment requirement
    pub align: Option<usize>,

    /// Whether implicit copying is allowed
    /// false = Move semantics (assignment moves, e.g. Qubit)
    /// true = Copy semantics (assignment copies, e.g. Vec128)
    pub is_copy: bool,

    /// Whether void values are allowed (zero-sized type)
    pub allow_zst: bool,
}
```

#### 2. Registration Interface

```rust
// Compiler internal API
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".to_string(),
    size: Some(0),           // Logical size is 0, physical state resides on the quantum processor
    align: Some(1),
    is_copy: false,          // Move semantics, consistent with no-cloning
    allow_zst: true,
});
```

After registration, `Qubit` becomes a legal primitive type in the type system, usable for variable declarations, function parameters, and struct fields.

#### 3. Type Checker Behavior

Extended primitive types follow these rules in type checking:

| Scenario | Behavior |
|------|------|
| Variable declaration `q: Qubit = ...` | ✅ Legal |
| Function parameter `fn(q: Qubit)` | ✅ Legal |
| Struct field `{ q: Qubit }` | ✅ Legal |
| Assignment when `is_copy == false` | Move semantics, original variable invalidated |
| Assignment when `is_copy == true` | Copy semantics, original variable preserved |
| Implicit copying (used in multiple places in a function) | Depends on `is_copy` |
| Comparison `==`, `!=` | ❌ Compile error (no built-in comparison) |
| Arithmetic `+`, `-` | ❌ Compile error (no built-in operation) |
| Generic constraint `T: Copy` | Satisfied only when `is_copy == true` |

#### 4. Code Generator Behavior

Extended primitive types are handled as **opaque values** during code generation:

- LLVM IR: generated as `{size} x i8` or a struct of corresponding size
- No special instructions are generated — semantics are handled by the backend or library
- If the backend requires special handling (e.g., QIR quantum gates), it is implemented through the backend registration mechanism (out of scope for this RFC)

### Examples

#### Registering a Move-Semantics Type

```rust
// Qubit: non-copyable, size 0 (physical state resides on QPU)
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
q2 = q          # ❌ Compile error: Qubit is a Move type, q has been invalidated
q = H(q)        # ✅ Consume q, return new q
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
b = a           # ✅ Copy semantics, a remains valid
c = add_vec128(a, b)  # ✅ Both a and b are usable
```

## Detailed Design

### Compiler Changes

| Component | Change |
|------|------|
| Type system | Add new `Ty::Extension` value variant, storing `PrimitiveExtension` metadata |
| Type checker | Extended types do not participate in built-in operation resolution, do not satisfy built-in trait constraints (unless explicitly implemented) |
| Ownership analyzer | Determine Move or Copy semantics based on `is_copy` |
| Code generator | Generate opaque values based on `size`/`align`, no special instructions |
| Error messages | Error messages for extended types reference the registered `name` |

### Relationship with FFI

`Primitive::Extension` is orthogonal to RFC-021 (FFI):

| | Primitive::Extension | FFI |
|---|---|---|
| Purpose | Register new **types** | Call external **functions** |
| Layer | Type system | Runtime |
| Example | `Qubit` is a type | `native("sin")` is a function call |

A domain may need both simultaneously: the `Qubit` type is registered via Extension, and quantum gate functions are registered via FFI.

### Backward Compatibility

- ✅ Fully backward compatible
- Does not modify the semantics of any existing types
- Extended types are an added capability and do not affect existing code

## Trade-offs

### Advantages

- ✅ The compiler does not need to modify source code for each new domain
- ✅ Domain experts can register types on their own, without depending on the compiler team
- ✅ Core types remain hardcoded, without sacrificing the compiler's deep optimization of fundamental types
- ✅ Simple interface — a single struct defines all attributes

### Disadvantages

- ⚠️ Extended types do not support built-in operations — additional functions or backend mechanisms are required to implement semantics
- ⚠️ During debugging, extended types display as opaque values, less intuitive than core types

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| All types dynamically loaded | Core types (Int/Float/Bool) require deep compiler optimization; dynamic loading would forfeit these capabilities |
| Per-domain hardcoding | Each new domain requires compiler modifications — not extensible |
| Pure library approach (no type registration) | Cannot enforce semantics at the type system level (e.g., no-cloning); can only be checked at runtime |

## Implementation Strategy

### Phase 1: Core Interface

- [ ] Add the `Ty::Extension` value variant in the type system
- [ ] Implement the `register_primitive` API
- [ ] Extend the type checker to handle `is_copy` semantics
- [ ] Extend the code generator to handle opaque values
- [ ] Unit tests

### Phase 2: Registration Timing

- [ ] Support batch registration during compiler initialization (config file or builder API)
- [ ] Support standard library pre-registration (the `std.primitive` module exports extended type definitions)

### Dependencies

- No hard dependencies. Can be implemented independently of other RFCs.

## Open Questions

- [ ] Should extended types support trait implementation (e.g., letting `Qubit` implement a custom `QuantumGate` trait)?
- [ ] Are lifecycle hooks (e.g., `on_drop`) needed to support RAII-semantic extended types?
- [ ] Configuration file format: TOML, YAML, or YaoXiang's own configuration syntax (referencing RFC-015)?

---

## Design Decision Records

| Decision | Determination | Reason | Date |
|------|------|------|------|
| Core types not dynamically loaded | Keep hardcoded | The compiler deeply depends on core type semantics; dynamic loading yields zero benefit | 2026-06-05 |
| Extended types as opaque values | Do not inject semantics | Semantics are handled by backend/library; the compiler only guarantees type safety and ownership | 2026-06-05 |
| Orthogonal to FFI | Do not merge | Type registration and function calls are different abstraction layers | 2026-06-05 |