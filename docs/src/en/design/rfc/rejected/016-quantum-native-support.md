---
title: 'RFC 016: Quantum-Native Support and Multi-Backend Integration'
status: 'Rejected'
author: 'Chen Xu'
created: '2026-02-13'
updated: '2026-06-05'
---

# RFC 016: Quantum-Native Support and Multi-Backend Integration

> **Rejection Reason**: Insufficient prerequisites. The Primitive::Extension mechanism has not been
> implemented, the language compiler is incomplete, and there is no actual user demand. Quantum
> support should be reconsidered after the language matures, as a consumer of the Extension
> mechanism.

> **Dependencies**:
>
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)

## Abstract

This document defines the **quantum-native support** and **multi-backend integration** scheme for
the YaoXiang language. Core idea: **YaoXiang's existing design (default Move, ownership flow-back,
opaque types, DAG scheduler, generic constant parameters) naturally constitutes a complete
foundation for quantum programming language, without introducing any new quantum-specific syntax**.
We implement quantum-native semantics, maximize quantum utilization through automatic parallelism,
hybrid classical programming, and multi-backend support by adding a few built-in types (`Qubit`,
`Complex`, `Topology`) and built-in functions (quantum gates, measurements, topology constraints),
utilizing existing language mechanisms.

## Motivation

### Why Quantum-Native Support?

The current quantum programming ecosystem suffers from severe fragmentation:

- **Low-level languages (QCIS, OpenQASM)**: Directly manipulate physical quantum gates, but lack
  type systems and abstraction mechanisms, making it difficult to write complex algorithms.
- **High-level frameworks (Qiskit, Cirq, Q#)**: Extend based on classical languages (Python, C#),
  with quantum semantics implemented through libraries, leading to:
  - No-cloning theorem for quantum states requires manual compliance by users (or reliance on linear
    type systems retrofitted).
  - Quantum gate operations are syntactically disjoint from classical code, creating a high learning
    curve.
- **Hybrid computing**: Quantum and classical parts require explicit separation, lacking a unified
  dataflow model.

### Current Issues

YaoXiang's existing design provides exactly the complete foundation to address these issues:

| Quantum Computing Requirement            | YaoXiang Existing Design        | Description                                                                                                                    |
| ---------------------------------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| No-cloning of quantum states             | **Default Move semantics**      | Assignment moves ownership; no implicit copying, naturally compliant with no-cloning theorem                                   |
| Quantum gates as unitary transformations | **Ownership flow-back**         | `q = H(q)` consumes the original qubit, returns a new qubit, precisely matching gate semantics                                 |
| Entangled states                         | **Opaque types**                | `BellPair` can only be operated as a whole; compiler tracks lifetime, preventing improper decomposition                        |
| Physical topology constraints            | **Generic constant parameters** | `Qubit(Topology, N)` performs compile-time adjacency checking                                                                  |
| Measurement collapse                     | **Empty state reuse**           | After measurement, qubit becomes empty and can be re-initialized, simulating quantum state collapse                            |
| Automatic quantum circuit parallelism    | **DAG scheduler**               | Statements within a function are automatically parallelized based on data dependencies; independent gates execute concurrently |
| Hybrid classical-quantum control flow    | **Unified syntax**              | Quantum and classical operations use the same `name: type = value` form                                                        |

**YaoXiang is not "adding quantum support", but discovering that its design is already
quantum-native.**

> **Semantic Note**: This document uses YaoXiang's **ownership semantics** to express quantum
> operations. The compiler guarantees no-cloning (ownership safety) at the compiler level. At the
> language level, "consume-create" is a **syntactic expression of ownership transfer**—consume =
> acquire ownership, return = transfer ownership. The underlying implementation can be true
> reversible quantum gates (in-place quantum state modification), rather than truly "creating new
> quantum states".

### Design Goals

1. **Zero new syntax**: No introduction of keywords like `quantum` or `circuit`; all quantum
   features are expressed through existing language mechanisms.
2. **Type safety**: The compiler guarantees quantum states are not copied or used illegally.
3. **Topology constraint compile-time checking**: Through generic constant parameters `Qubit(T, N)`,
   verify whether two-qubit gate operations comply with physical topology at compile time.
4. **Transparent multi-backend support**: The same quantum code can be compiled to QIR (general
   ecosystem) or QCIS (domestic quantum instruction set), switched via command-line arguments.
5. **Seamless hybrid classical**: Quantum computing can freely call classical functions; classical
   code can also manipulate quantum data (via `ref` sharing, but constrained by ownership).

## Proposal

### Core Design

#### 1. Quantum Type System Mapping

**Base types**:

```yaoxiang
Qubit: Type0 = primitive_qubit
Complex: Type0 = { re: Float, im: Float }
```

- `Qubit` is a first-class type, following ownership rules (Move, RAII).
- `Complex` is used to represent amplitudes; the compiler can perform inline optimization.

**Quantum gates as functions**:

```yaoxiang
# Builtin function signatures
H: (Qubit) -> Qubit = builtin_hadamard
X: (Qubit) -> Qubit = builtin_pauli_x
Y: (Qubit) -> Qubit = builtin_pauli_y
Z: (Qubit) -> Qubit = builtin_pauli_z
CNOT: (control: Qubit, target: Qubit) -> { Qubit, Qubit } = builtin_cnot
```

- All gates consume input qubits, returning new qubits (or entangled pairs). The ownership flow-back
  syntax `q = H(q)` directly corresponds to mathematical semantics.
- Multi-qubit gates return structs, with results obtained through pattern matching or field access.

**Measurement**:

```yaoxiang
measure: (Qubit) -> Int = builtin_measure   # Consumes qubit, returns classical bit
measure_all: (List(Qubit)) -> List(Int) = builtin_measure_all
```

- After measurement, the qubit is consumed (becomes empty); users can re-initialize through empty
  state reuse.

**Initialization**:

```yaoxiang
qubit: (Int) -> Qubit = builtin_qubit   # 0 or 1 initializes basis state
```

#### 2. Entanglement and Opaque Type Encapsulation

Encapsulate entangled pairs as opaque types, providing only composite operations and prohibiting
decomposition:

```yaoxiang
# Builtin opaque type
BellPair: Type0 = primitive_bell_pair

# Builtin functions - can only operate on the whole
CNOT: (Qubit, Qubit) -> BellPair
measure_bell: (BellPair) -> { Int, Int }
split_bell: (BellPair) -> { Qubit, Qubit }  # Split entangled pair (use with caution)
apply_cnot_to_bell: (BellPair, Qubit) -> BellPair
```

**Key design**:

- No field accessors are provided; operations are only allowed on the whole through builtin
  functions.
- `measure_bell(bp)` consumes the entire entangled pair at once, returning classical bits.
- The compiler can track the complete lifecycle of entangled pairs.

**Comparison with Python/Qiskit**:

```
Python (Qiskit): Runtime circuit construction; errors may only be discovered after submission
YaoXiang:       Compile-time capture of most logical errors
```

**Remaining 10%** (such as physical decoherence, gate errors) are hardware issues, not resolvable by
the language.

#### 3. Physical Topology Constraints

A quantum chip is a constrained topology graph, **not any two qubits can perform two-qubit
gates**—they must be adjacent. YaoXiang uses **generic constant parameters** to guarantee topology
constraints at compile time.

**Topology type definition**:

```yaoxiang
# Topology as a type, containing adjacency matrix
Topology: Type0 = primitive_topology

# Builtin topology constants
Linear8: Topology = topology(8)          # Linear 8-qubit: 0-1-2-3-4-5-6-7
Grid3x3: Topology = topology(3, 3)        # 3x3 grid
Ring16: Topology = topology(16, ring)    # Ring 16-qubit
```

**Qubit binds topology and position**:

```yaoxiang
# Qubit(T, N) - T is the topology type, N is a constant position parameter
q0: Qubit(Grid3x3, 0)   # Grid3x3 topology, position (0,0)
q1: Qubit(Grid3x3, 1)   # Grid3x3 topology, position (0,1)
q2: Qubit(Grid3x3, 2)   # Grid3x3 topology, position (0,2)
q3: Qubit(Grid3x3, 3)   # Grid3x3 topology, position (1,0)
```

**Gate operations auto-constrained**:

```yaoxiang
# CNOT type signature with topology constraint
CNOT: (T: Topology, I: Int, J: Int) -> (
    (Qubit(T, I), Qubit(T, J)) -> { Qubit(T, I), Qubit(T, J) }
) when adjacent(T, I, J)

# Compile-time checking
CNOT(q0, q1)  # ✅ (0,0) is adjacent to (0,1) in Grid3x3
CNOT(q0, q2)  # ❌ Compile error: (0,0) is not adjacent to (0,2)
```

**`adjacent` compile-time constraint**:

- `adjacent` is a compile-time function, using the topology's adjacency matrix for static checking.
- With constant indices, 100% compile-time verification.
- With dynamic indices, generates runtime checking code.

**Virtual-to-physical mapping**:

```yaoxiang
# Don't know specific physical position at compile time? Use type inference
q = qubit(Grid3x3)  # Auto-assign position 0, subsequent derivation
```

#### 4. Ownership and Linear Flow of Quantum States

All quantum operations follow Move semantics, ensuring qubits are not copied:

```yaoxiang
q = qubit(0)
q2 = q          # ❌ Compile error: q has been moved, cannot be used again
q = H(q)        # ✅ Consume q, return new q
measure(q)      # ✅ Consume q, after which q becomes empty
q = qubit(0)    # ✅ Empty state reuse
```

#### 4. Automatic Parallelism and DAG Scheduling

Under Standard or Full Runtime, the DAG scheduler automatically analyzes quantum programs:

```yaoxiang
apply_two_qubit_gates: () -> {Qubit, Qubit} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    # The two lines above have no data dependency; DAG automatically parallelizes execution
    CNOT(q1, q2)   # Depends on q1 and q2, automatically waits
}
```

- The scheduler uses `num_workers` configuration (number of physical quantum processors) to achieve
  true parallelism.
- Users don't need to manually arrange gate order; they just describe the dataflow.

#### 5. Hybrid Classical Computing

Classical and quantum code are fully integrated:

```yaoxiang
grover_search: (target: Int) -> Int = () => {
    n = 4
    qubits = List(Qubit)()
    for i in 0..n {
        qubits.append(H(qubit(0)))
    }
    # Classical loop mixed with quantum operations
    oracle(qubits, target)   # oracle is a quantum gate sequence
    qubits = diffusion(qubits)
    results = measure_all(qubits)
    return decode_result(results)   # Classical post-processing
}
```

- Quantum gates and classical control flow can be freely mixed within the same function.
- The ownership system ensures quantum variables are not incorrectly copied in classical branches.

#### 6. Multi-Backend Support Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   YaoXiang      │     │   Type Check    │     │   DAG IR        │
│   Source        │     │   + Ownership   │     │   (Dataflow     │
│   (Unified      │────▶│   Analysis      │────▶│   Graph)        │
│   Syntax)       │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                          ┌─────────────────────────────────────────────┐
                          │           Code Generation Backends         │
                          │              (Pluggable)                  │
                          ├─────────────────┬───────────────────────────┤
                          │  QIR Backend   │  QCIS Backend             │
                          │  (General      │  (Domestic Quantum        │
                          │   Ecosystem)   │   Instruction Set)        │
                          ├─────────────────┼───────────────────────────┤
                          │  - Output .ll  │  - Output .qcis text      │
                          │  - Adapt to    │  - Adapt to CAS/          │
                          │    multiple    │    Qudyn hardware         │
                          │    QPUs        │                           │
                          └─────────────────┴───────────────────────────┘
```

- **Compilation flow**: Unified frontend → DAG construction → Backend selection → Target code
  generation.
- **QIR Backend**: Maps DAG nodes to QIR's quantum gate intrinsics, generates LLVM bitcode, enabling
  further LLVM optimization.
- **QCIS Backend**: Serializes DAG to QCIS instructions (e.g., `H q0`), supporting direct submission
  to quantum chip consoles.

### Examples

#### Bell State Preparation and Measurement

```yaoxiang
bell_measure: () -> {Int, Int} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    bell = CNOT(q1, q2)  # Returns BellPair opaque type
    result = measure_bell(bell)  # Measure two bits at once
    return result
}
```

#### Quantum Teleportation (Simplified)

```yaoxiang
teleport: (msg: Qubit, bell: BellPair) -> Qubit = (msg, bell) => {
    # Split entangled pair to get two independent qubits
    (alice_qubit, bob_qubit) = split_bell(bell)

    # Alice's operations
    (msg, alice_qubit) = CNOT(msg, alice_qubit)
    msg = H(msg)
    a1 = measure(msg)
    a2 = measure(alice_qubit)

    # Classical information transmission (automatically handled by scheduler for dependencies)
    # Bob's operations
    if a2 == 1 { bob_qubit = X(bob_qubit) }
    if a1 == 1 { bob_qubit = Z(bob_qubit) }
    return bob_qubit
}
```

## Detailed Design

### Builtin Types and Function Definitions

Add in the `compiler/builtins` module:

```rust
builtins.insert("Qubit", Ty::Primitive(Primitive::Qubit));
builtins.insert("Complex", Ty::Record(vec![
    ("re", Ty::Primitive(Primitive::Float)),
    ("im", Ty::Primitive(Primitive::Float)),
]));

// Quantum gates
for (name, sig) in GATES {
    builtins.insert(name, Ty::Function(vec![Ty::Qubit], Ty::Qubit));
}
builtins.insert("CNOT", Ty::Function(
    vec![Ty::Qubit, Ty::Qubit],
    Ty::Record(vec![
        ("q0", Ty::Qubit),
        ("q1", Ty::Qubit)
    ])
));
builtins.insert("measure", Ty::Function(vec![Ty::Qubit], Ty::Primitive(Primitive::Int)));
builtins.insert("qubit", Ty::Function(vec![Ty::Primitive(Primitive::Int)], Ty::Qubit));
```

### Special Handling of Qubit by the Ownership Checker

- `Qubit` is marked as `!Copy` (default Move), prohibiting implicit copying.
- The measurement function `measure` takes `Qubit` as a parameter (pass-by-value), consuming
  ownership.
- Fields in the record type returned by multi-qubit gates are all `Qubit`, still subject to
  ownership rules.

### DAG Scheduler Optimization for Quantum Gates

- Quantum gate nodes are treated as pure functions (no side effects), allowing the scheduler to
  arbitrarily reorder independent gates.
- When the scheduler outputs "quantum instruction sequence", it preserves data dependencies and
  groups parallel gates (applicable to multi-qubit processors).
- Supports configuring `--target-num-qubits` and `--target-topology` for subsequent layout and
  routing (future extension).

### QIR Backend Detailed Mapping

| YaoXiang Operation | QIR Instruction                                                   |
| ------------------ | ----------------------------------------------------------------- |
| `H(q)`             | `call void @__quantum__qis__h__body(%Qubit* %q)`                  |
| `CNOT(q1, q2)`     | `call void @__quantum__qis__cnot__body(%Qubit* %q1, %Qubit* %q2)` |
| `measure(q)`       | `%result = call i1 @__quantum__qis__mz__body(%Qubit* %q)`         |
| `qubit(0)`         | `%q = call %Qubit* @__quantum__rt__qubit_allocate()`              |

The QIR backend uses LLVM's `-O2` for further optimization and outputs bitcode compatible with the
QIR Alliance.

### QCIS Backend Detailed Mapping

| YaoXiang Operation                       | QCIS Instruction                                                 |
| ---------------------------------------- | ---------------------------------------------------------------- |
| `H(q)` (q corresponds to physical bit 2) | `H 2`                                                            |
| `CNOT(q1,q2)` (q1→bit 0, q2→bit 1)       | `CNOT 0 1`                                                       |
| `measure(q)` (bit 0)                     | `M 0`                                                            |
| `qubit(0)` initialization                | Implicit in first usage instruction, no extra instruction needed |

- A mapping table from virtual qubits (YaoXiang variables) to physical bits must be maintained.
- Topology constraint checking supported (future implementation).

### Hybrid Classical Code Generation

- Classical parts (such as loops, conditions, integer operations) generate native code (x86/ARM) as
  usual, interacting with the quantum backend through FFI or embedded calls.
- In the QIR backend, classical parts can be lowered to LLVM IR, compiled together with QIR.

### Type System Impact

- New `Qubit` and `Complex` primitive types added.
- `Qubit` automatically has Move semantics, prohibiting copying.
- Quantum gate function signatures need to be registered in the type system.

### Backward Compatibility

- ✅ Fully backward compatible
- New builtin types and functions do not affect existing code
- Quantum features are optional, introducing no extra overhead when not enabled

## Trade-offs

### Advantages

- **No new syntax**: Developers only need to learn a few builtin functions to write quantum
  programs.
- **Type safety**: The ownership system automatically prevents qubit copying, avoiding common
  quantum programming errors.
- **Automatic parallelism**: The DAG scheduler provides gate-level parallelism for free, without
  additional compiler optimization.
- **Ecosystem compatibility**: The QIR backend enables YaoXiang to run on multiple quantum cloud
  platforms; the QCIS backend ensures autonomy and controllability.
- **Hybrid capability**: Classical-quantum fusion is natural, suitable for writing complex quantum
  algorithms (such as classical control in Shor and Grover).

### Disadvantages

- **Static qubit count**: The current design assumes qubit count is known at compile time; dynamic
  allocation requires `List(Qubit)`, but `List`'s heap allocation may introduce extra overhead (can
  be mitigated through optimization).
- **Post-measurement reuse**: Empty state reuse allows re-initialization of qubits, but physical
  qubits may have relaxation times, requiring runtime system handling (currently user
  responsibility).
- **Dynamic topology mapping**: When physical topology is only known at runtime, compile-time
  checking cannot take effect, requiring runtime checking code generation (current version only
  supports static checking).

## Alternative Approaches

| Approach                                           | Why Not Chosen                                                                                                  |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Introduce `quantum` keyword and `circuit` type     | Adds new syntax, high learning cost, violates YaoXiang's concise design principle                               |
| Implement quantum support only as a library        | Cannot leverage compiler guarantees for quantum state safety; cannot deeply integrate with DAG scheduler        |
| Wait for quantum hardware to mature before support | Miss the critical window period for quantum programming language design                                         |
| Reuse existing quantum frameworks (like Qiskit)    | Quantum semantics implemented through libraries; cannot gain type system and ownership system safety guarantees |
| Design a separate quantum sublanguage              | Increases language complexity; high maintenance cost                                                            |

## Implementation Strategy

### Phase Breakdown

| Phase   | Duration | Content                                                                                                                                                             |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 | 1 month  | Basic quantum types and builtin functions: Add `Qubit`, `Complex` types to compiler; implement type checking for builtin functions; extend ownership checker        |
| Phase 2 | 1 month  | DAG scheduler recognizes quantum gates: Modify DAG construction logic; mark quantum gates as pure functions; implement parallel gate grouping output                |
| Phase 3 | 2 months | QIR backend prototype: Implement DAG to QIR code generator; integrate LLVM; connect QIR simulator for verification                                                  |
| Phase 4 | 2 months | QCIS backend prototype: Implement DAG to QCIS instruction translator; design virtual-physical bit mapping; connect domestic quantum platform for verification       |
| Phase 5 | 2 months | Hybrid classical enhancement: Ensure correct code generation when classical control flow intersects with quantum gates; support `List(Qubit)`; add example programs |
| Phase 6 | 2 months | Optimization and documentation: Implement basic layout and routing; write user guide and quantum programming tutorial; release preview version                      |

### Risks

1. **Quantum hardware availability**: Depends on the availability of external quantum simulators and
   real QPUs.
   - **Mitigation**: Prioritize connecting to open-source simulators (QIR runner, Qiskit Aer); real
     QPUs as a long-term goal.

2. **Backend implementation complexity**: QIR and QCIS specifications may change.
   - **Mitigation**: Abstract code generation interface; isolate backend differences; facilitate
     subsequent adaptation.

3. **Performance uncertainty**: Quantum programs have different performance characteristics than
   classical programs.
   - **Mitigation**: Provide performance profiling tools, allowing users to understand gate-level
     parallelism effects.

## Open Questions

- [x] **Topology constraints**: Implemented through `Qubit(Topology, N)` generic constant parameters
      for compile-time checking.
- [ ] **Dynamic quantum registers**: How should `List(Qubit)` be mapped in the QCIS backend?
      Physical bits of corresponding quantity can be generated, but requires runtime allocation
      mechanism.
- [ ] **Error mitigation**: Should builtin error mitigation constructs (such as dynamic decoupling)
      be provided? Can be implemented as a library first.
- [ ] **Interoperability with existing quantum SDKs**: Can QASM or QIR modules be imported? FFI can
      be considered in the future.
- [ ] **Automatic layout and routing**: When virtual qubit count exceeds physical qubit count, how
      to auto-map?

## References

- [QIR Specification](https://github.com/qir-alliance/qir-spec)
- [QCIS: A Quantum Control Instruction Set](https://arxiv.org/abs/2005.12534) (USTC/Qudyn)
- [Rust Quantum Computing Examples](https://github.com/Rust-GPU/rust-gpu)
- [Qunity: A Unified Language for Quantum and Classical Computing](https://qunity-lang.org) (2025)

---

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Community discussion
│  Review     │
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│  (Official  │    │ (Preserved  │
│   Design)   │    │   In Place) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status           | Location                 | Description                                                   |
| ---------------- | ------------------------ | ------------------------------------------------------------- |
| **Draft**        | `docs/design/rfc/draft/` | Author draft; awaiting submission for review                  |
| **Under Review** | `docs/design/rfc/`       | Open for community discussion and feedback                    |
| **Accepted**     | `docs/design/accepted/`  | Becomes official design document; enters implementation phase |
| **Rejected**     | `docs/design/rfc/`       | Preserved in RFC directory; status updated                    |
