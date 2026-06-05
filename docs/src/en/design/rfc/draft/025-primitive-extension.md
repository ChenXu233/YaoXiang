---
title: "RFC-025: Extensible Primitive Type Mechanism"
---

# RFC-025: Extensible Primitive Type Mechanism

> **Status**: Draft
> **Author**: Chen Xu
> **Created**: 2026-06-05
> **Last Updated**: 2026-06-05

## Summary

This document defines the **Extensible Primitive Type Mechanism** (`Primitive::Extension`) for the YaoXiang compiler. It allows external code to register custom primitive types with the compiler, enabling support for domain-specific types (qubits, GPU buffers, SIMD vectors, hardware registers, etc.) without hardcoding.

## Motivation

### Why is this mechanism needed?

Currently, the compiler hardcodes all primitive types: `Int`, `Float`, `String`, `Bool`, `Unit`. Adding a new primitive type requires modifying multiple locations in the compiler source code—type checker, code generator, ownership analyzer, optimizer.

This violates the Open-Closed Principle: open for extension, closed for modification.

### Design Boundary

```
Hardcoded Core Types (language foundation): Int, Float, String, Bool, Unit
Dynamically Extended Types (domain plugins):  Registered via Primitive::Extension
```

Core types are hardcoded because the compiler deeply depends on their semantics (arithmetic operations, conditional branches, hashing, comparison). Extension types are **opaque values** to the compiler—the compiler only knows their size, alignment, and ownership properties, not their internal semantics.

This is not "unifying all types into dynamic loading." Core types and extension types are two different things.

## Proposal

### Core Design

#### 1. Extension Type Attributes

Each extended primitive type must declare the following attributes upon registration:

```rust
pub struct PrimitiveExtension {
    /// Type name, e.g., "Qubit", "Buffer", "Vec128"
    pub name: String,

    /// Byte size (fixed-size types)
    /// None indicates size is unknown at compile-time (requires runtime determination)
    pub size: Option<usize>,

    /// Alignment requirement
    pub align: Option<usize>,

    /// Whether implicit copying is allowed
    /// false = Move semantics (assignment moves, e.g., Qubit)
    /// true = Copy semantics (assignment copies, e.g., Vec128)
    pub is_copy: bool,

    /// Whether zero-sized types are allowed
    pub allow_zst: bool,
}
```

#### 2. Registration Interface

```rust
// Compiler internal API
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".to_string(),
    size: Some(0),           // Logical size is 0, physical state is on quantum processor
    align: Some(1),
    is_copy: false,          // Move semantics, consistent with no-cloning
    allow_zst: true,
});
```

After registration, `Qubit` becomes a valid primitive type in the type system, usable for variable declarations, function parameters, and struct fields.

#### 3. Type Checker Behavior

Extended primitive types follow these rules in type checking:

| Scenario | Behavior |
|----------|----------|
| Variable declaration `q: Qubit = ...` | ✅ Valid |
| Function parameter `fn(q: Qubit)` | ✅ Valid |
| Struct field `{ q: Qubit }` | ✅ Valid |
| Assignment when `is_copy == false` | Move semantics, original variable invalidated |
| Assignment when `is_copy == true` | Copy semantics, original variable retained |
| Implicit copying (multiple uses in function) | Depends on `is_copy` |
| Comparison `==`, `!=` | ❌ Compile error (no built-in comparison) |
| Arithmetic `+`, `-` | ❌ Compile error (no built-in operations) |
| Generic constraint `T: Copy` | Only satisfied when `is_copy == true` |

#### 4. Code Generator Behavior

Extended primitive types are treated as **opaque values** in code generation:

- LLVM IR: generated as `{size} x i8` or a struct of corresponding size
- No special instructions generated—semantics are handled by backend or library
- If a backend requires special handling (e.g., QIR quantum gates), this is implemented via the backend registration mechanism (out of scope for this RFC)

### Examples

#### Registering a Move Semantics Type

```rust
// Qubit: non-copyable, size is 0 (physical state is on QPU)
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
q2 = q          # ❌ Compile error: Qubit is a Move type, q is invalidated
q = H(q)        # ✅ Consumes q, returns new q
```

#### Registering a Copy Semantics Type

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
b = a           # ✅ Copy semantics, a is still valid
c = add_vec128(a, b)  # ✅ Both a and b are usable
```

## Detailed Design

### Compiler Changes

| Component | Changes |
|-----------|---------|
| Type System | New `Ty::Extension` variant, storing `PrimitiveExtension` metadata |
| Type Checker | Extended types do not participate in built-in operation resolution, do not satisfy built-in trait constraints (unless explicitly implemented) |
| Ownership Analyzer | Determines Move or Copy semantics based on `is_copy` |
| Code Generator | Generates opaque values based on `size`/`align`, no special instructions |
| Error Messages | Error messages for extended types reference the registered `name` |

### Relationship with FFI

`Primitive::Extension` is orthogonal to RFC-021 (FFI):

| | Primitive::Extension | FFI |
|---|---|---|
| Purpose | Register new **type** | Call external **function** |
| Layer | Type system | Runtime |
| Example | `Qubit` is a type | `native("sin")` is a function call |

A domain may need both: `Qubit` type registered via Extension, quantum gate functions registered via FFI.

### Backward Compatibility

- ✅ Fully backward compatible
- Does not modify semantics of any existing types
- Extension types are a new capability, do not affect existing code

## Trade-offs

### Advantages

- ✅ Compiler does not need to modify source code for each new domain
- ✅ Domain experts can register types independently, without relying on compiler team
- ✅ Core types remain hardcoded, without sacrificing compiler's deep optimization of basic types
- ✅ Simple interface: one struct defines all attributes

### Disadvantages

- ⚠️ Extension types do not support built-in operations—additional functions or backend mechanisms are needed to implement semantics
- ⚠️ During debugging, extension types display as opaque values, less intuitive than core types

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| All types dynamically loaded | Core types (Int/Float/Bool) require deep compiler optimization; dynamic loading would lose these capabilities |
| Hardcode each domain | Modifying compiler for each domain is not extensible |
| Pure library solution (no type registration) | Cannot guarantee semantics at the type system level (e.g., no-cloning), only runtime checking |

## Implementation Strategy

### Phase 1: Core Interface

- [ ] Add `Ty::Extension` variant to type system
- [ ] Implement `register_primitive` API
- [ ] Extend type checker to handle `is_copy` semantics
- [ ] Extend code generator to handle opaque values
- [ ] Unit tests

### Phase 2: Registration Timing

- [ ] Support bulk registration during compiler initialization (config file or builder API)
- [ ] Support standard library pre-registration (`std.primitive` module exports extension type definitions)

### Dependencies

- No hard dependencies. Can be implemented independently of other RFCs.

## Open Questions

- [ ] Should extension types support trait implementation (e.g., let `Qubit` implement a custom `QuantumGate` trait)?
- [ ] Are lifetime hooks needed (e.g., `on_drop`) to support RAII-semantic extension types?
- [ ] Config file format: TOML, YAML, or YaoXiang's own config syntax (refer to RFC-015)?

---

## Design Decision Record

| Decision | Decision | Rationale | Date |
|----------|----------|-----------|------|
| Core types not dynamically loaded | Keep hardcoded | Compiler deeply depends on core type semantics; dynamicization yields zero benefit | 2026-06-05 |
| Extension types are opaque values | Do not inject semantics | Semantics handled by backend/library; compiler only guarantees type safety and ownership | 2026-06-05 |
| Orthogonal to FFI | Do not merge | Type registration and function calls are different abstraction layers | 2026-06-05 |