---
title: RFC 016: Quantum-Native Support and Multi-Backend Integration
---

# RFC 016: Quantum-Native Support and Multi-Backend Integration

> **Status**: Draft
> **Author**: Chen Xu (晨煦)
> **Created**: 2026-02-13
> **Last Updated**: 2026-02-13
> **Target Implementation Timeline**: 10 months

> **Dependencies**:
> - [RFC-001: Concurrency Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)

## Abstract

This document defines **quantum-native support** and **multi-backend integration** for the YaoXiang language. The core insight: **YaoXiang's existing design (default Move semantics, ownership return, structural subtyping, DAG scheduler) naturally forms a complete foundation for quantum programming language, without introducing any quantum-specific syntax**. We implement quantum-native semantics, automatic parallelism for maximum quantum utilization, seamless classical-quantum hybrid programming, and multi-backend support by adding a small set of built-in types (`Qubit`, `Complex`) and built-in functions (quantum gates, measurements), leveraging existing language mechanisms.

## Motivation

### Why Quantum-Native Support?

The current quantum programming ecosystem is severely fragmented:

- **Low-level languages (QCIS, OpenQASM)**: Directly manipulate physical quantum gates, but lack type systems and abstraction mechanisms, making it difficult to write complex algorithms.
- **High-level frameworks (Qiskit, Cirq, Q#)**: Extend classical languages (Python, C#), where quantum semantics are implemented through libraries, leading to:
  - Quantum no-cloning theorem must be manually enforced by users (or patched with linear type systems).
  - Quantum gate operations are syntactically separated from classical code, increasing learning curve.
  - Automatic parallelism optimization depends on external compilers, difficult to integrate with classical control flow.
- **Hybrid computing**: Quantum and classical parts must be explicitly separated, lacking a unified dataflow model.

### Current State

YaoXiang's existing design恰好 provides a complete foundation for solving these problems:

| Quantum Computing Requirement | YaoXiang Existing Design | Description |
|-----------------------------|-------------------------|-------------|
| Quantum no-cloning | **Default Move semantics** | Assignment moves ownership, no implicit copying, naturally符合 no-cloning theorem |
| Quantum gates as unitary transformations | **Ownership return** | `q = H(q)` consumes original qubit, returns new qubit, exactly matching gate semantics |
| Entangled states | **Structural subtyping** | `{ Qubit, Qubit }` naturally represents entangled pairs, no special tensor product syntax needed |
| Measurement collapse | **Empty state reuse** | Qubit becomes empty after measurement, can be reinitialized, simulating quantum state collapse |
| Automatic quantum circuit parallelism | **DAG scheduler** | Statements in a function are automatically parallelized based on data dependency, gates without dependencies naturally concurrent |
| Hybrid classical-quantum control flow | **Unified syntax** | Quantum and classical operations use the same `name: type = value` form |

**YaoXiang is not "adding quantum support", but discovering its design is already quantum-native.**

### Design Goals

1. **Zero new syntax**: No `quantum`, `circuit` keywords introduced; all quantum features expressed through existing language mechanisms.
2. **Type safety**: Compiler guarantees quantum states are not copied or used illegally.
3. **Automatic parallelism for maximum utilization**: DAG scheduler automatically analyzes quantum gate dependencies, implements gate-level parallelism on quantum hardware, no manual annotations required.
4. **Transparent multi-backend**: Same quantum code can compile to QIR (general ecosystem) or QCIS (domestic quantum instruction set), switched via command-line parameters.
5. **Seamless hybrid classical**: Quantum computing can freely call classical functions; classical code can also operate on quantum data (via `ref` sharing, but constrained by ownership).

## Proposal

### Core Design

#### 1. Quantum Type System Mapping

**Base types**:
```yaoxiang
Qubit: Type0 = primitive_qubit
Complex: Type0 = { re: Float, im: Float }
```
- `Qubit` is a first-class type, following ownership rules (Move, RAII).
- `Complex` represents amplitudes, can be inlined and optimized by the compiler.

**Quantum gates as functions**:
```yaoxiang
# Built-in function signatures
H: (Qubit) -> Qubit = builtin_hadamard
X: (Qubit) -> Qubit = builtin_pauli_x
Y: (Qubit) -> Qubit = builtin_pauli_y
Z: (Qubit) -> Qubit = builtin_pauli_z
CNOT: (control: Qubit, target: Qubit) -> { Qubit, Qubit } = builtin_cnot
```
- All gates consume input qubits and return new qubits (or entangled pairs). The ownership return syntax `q = H(q)` directly corresponds to mathematical semantics.
- Multi-qubit gates return structs, accessible via pattern matching or field access.

**Measurement**:
```yaoxiang
measure: (Qubit) -> Int = builtin_measure   # Consumes qubit, returns classical bit
measure_all: (List[Qubit]) -> List[Int] = builtin_measure_all
```
- After measurement, the qubit is consumed (becomes empty), and users can reinitialize via empty state reuse.

**Initialization**:
```yaoxiang
qubit: (Int) -> Qubit = builtin_qubit   # 0 or 1 initializes basis state
```

#### 2. Entanglement and Structural Subtyping

Entangled states are directly represented using structs:
```yaoxiang
bell_pair: { Qubit, Qubit } = CNOT(H(qubit(0)), qubit(0))
```
- Struct field order doesn't matter; compiler can recognize this as a joint state of two qubits.
- Access individual qubits via field access for continued operations.

**Ownership and Quantum Semantics Boundary**

`{Qubit, Qubit}` struct provides syntactic operational capability, but **does not guarantee these two bits remain entangled at the quantum backend level**.

Users must follow the correct quantum computing usage order:
1. Prepare entanglement
2. Perform operations/measurements on individual qubits

Performing intermediate operations on an entangled pair (e.g., `bell.q2 = H(bell.q2)`) will destroy entanglement. This is a quantum knowledge issue, not something the compiler can check.

> **Design Principle**: The compiler only guarantees "won't copy qubits" (ownership safety), not "correct use of quantum semantics".
> This mirrors Rust's design philosophy - Rust guarantees memory safety, but doesn't guarantee multi-threaded code has no data races.

### Quantum Semantic Safety: 90% Caught at Compile Time

YaoXiang's existing type system + DAG scheduler + ownership model can already catch most common quantum errors:

| Error Type | Capture Mechanism |
|-----------|-------------------|
| Qubit copying | Ownership system (Move semantics) |
| Use after measurement | Empty state reuse + dataflow analysis |
| Illegal gate operation order | DAG scheduler ensures correct dependencies |
| Entangled pair misuse | **Opaque type** encapsulation (recommended) |

#### Opaque Type Encapsulation for Entangled Pairs

Encapsulate entangled pairs as opaque types, providing only composite operations, prohibiting disassembly:

```yaoxiang
# Built-in opaque type
BellPair: Type0 = primitive_bell_pair

# Built-in functions - operate only as a whole
CNOT: (Qubit, Qubit) -> BellPair
measure_bell: (BellPair) -> { Int, Int }
apply_cnot_to_bell: (BellPair, Qubit) -> BellPair
```

**Key Design**:
- No field accessors provided; only whole operations via built-in functions
- `measure_bell(bp)` consumes the entire entangled pair at once, returning classical bits
- Compiler can track the complete lifecycle of entangled pairs

**Comparison with Python/Qiskit**:
```
Python (Qiskit): Circuit built at runtime, errors may only be found after submission
YaoXiang: Most logic errors caught at compile time
```

**Remaining 10%** (e.g., physical decoherence, gate errors) are hardware issues, not solvable by the language.

#### 3. Ownership and Linear Quantum State Flow

All quantum operations follow Move semantics, ensuring qubits are not copied:
```yaoxiang
q = qubit(0)
q2 = q          # ❌ Compile error: q has been moved, cannot use again
q = H(q)        # ✅ Consumes q, returns new q
measure(q)      # ✅ Consumes q, then q becomes empty
q = qubit(0)    # ✅ Empty state reuse
```

#### 4. Automatic Parallelism and DAG Scheduling

Under Standard or Full Runtime, the DAG scheduler automatically analyzes quantum programs:
```yaoxiang
apply_two_qubit_gates: () -> {Qubit, Qubit} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    # The above two lines have no data dependency, DAG automatically executes in parallel
    CNOT(q1, q2)   # Depends on q1 and q2, automatically waits
}
```
- Scheduler uses `num_workers` configuration (number of physical quantum processors) for true parallelism.
- Users don't need to manually arrange gate order, only describe dataflow.

#### 5. Hybrid Classical-Quantum Computing

Classical and quantum code are fully fused:
```yaoxiang
grover_search: (target: Int) -> Int = () => {
    n = 4
    qubits = List[Qubit]()
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
- Quantum gates and classical control flow can be arbitrarily mixed within the same function.
- Ownership system ensures quantum variables aren't incorrectly copied in classical branches.

#### 6. Multi-Backend Support Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   YaoXiang      │     │   Type Check    │     │   DAG IR        │
│   Source        │────▶│   + Ownership    │────▶│   (Dataflow)    │
│   (Unified)     │     │   Analysis      │     │                 │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                          ┌─────────────────────────────────────────────┐
                          │         Code Generation Backend (Pluggable)  │
                          ├─────────────────┬───────────────────────────┤
                          │  QIR Backend    │  QCIS Backend             │
                          │  (General       │  (Domestic Quantum         │
                          │   Ecosystem)    │   Instruction Set)         │
                          ├─────────────────┼───────────────────────────┤
                          │  - Output .ll   │  - Output .qcis text      │
                          │    files        │  - Adapt to CAS/GuoDang   │
                          │  - Adapt to     │    hardware               │
                          │    multiple QPU │                          │
                          └─────────────────┴───────────────────────────┘
```

- **Compilation pipeline**: Frontend → DAG Construction → Backend Selection → Target Code Generation.
- **QIR Backend**: Maps DAG nodes to QIR quantum gate intrinsics, generates LLVM bitcode, can further leverage LLVM optimizations.
- **QCIS Backend**: Serializes DAG to QCIS instructions (e.g., `H q0`), supports direct submission to quantum chip consoles.

#### 7. Easter Egg Quantum Mapping

When the compiler encounters `Type: Type = Type`, it outputs special information. In the quantum backend context, an additional calibration sequence can be generated—for example, applying H gates to all available qubits simultaneously and measuring, interpreting binary results as ASCII characters to output philosophical statements. This feature is an optional easter egg and does not affect regular compilation.

### Examples

#### Bell State Preparation and Measurement
```yaoxiang
bell_measure: () -> {Int, Int} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    bell = CNOT(q1, q2)
    m1 = measure(bell.q1)
    m2 = measure(bell.q2)
    return {m1, m2}
}
```

#### Quantum Teleportation (Simplified)
```yaoxiang
teleport: (msg: Qubit, bell: {Qubit, Qubit}) -> Qubit = (msg, bell) => {
    # Alice's operations
    msg, bell.q1 = CNOT(msg, bell.q1)
    msg = H(msg)
    a1 = measure(msg)
    a2 = measure(bell.q1)

    # Classical information transmission (automatically handled by scheduler)
    # Bob's operations
    q3 = bell.q2
    if a2 == 1 { q3 = X(q3) }
    if a1 == 1 { q3 = Z(q3) }
    return q3
}
```

## Detailed Design

### Built-in Type and Function Definitions

Add to `compiler/builtins` module:
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

### Ownership Checker Special Handling for Qubit

- `Qubit` is marked as `!Copy` (default Move), prohibiting implicit copying.
- `measure` function takes `Qubit` by value, consumes ownership.
- Fields in record types returned by multi-qubit gates are all `Qubit`, still following ownership rules.

### DAG Scheduler Optimizations for Quantum Gates

- Quantum gate nodes are treated as pure functions (no side effects), scheduler can freely reorder gates without dependencies.
- When outputting "quantum instruction sequences", data dependencies are preserved, and parallel gates are grouped (suitable for multi-quantum processors).
- Support for `--target-num-qubits` and `--target-topology` configuration for future layout and routing.

### QIR Backend Detailed Mapping

| YaoXiang Operation | QIR Instruction |
|-------------------|-----------------|
| `H(q)` | `call void @__quantum__qis__h__body(%Qubit* %q)` |
| `CNOT(q1, q2)` | `call void @__quantum__qis__cnot__body(%Qubit* %q1, %Qubit* %q2)` |
| `measure(q)` | `%result = call i1 @__quantum__qis__mz__body(%Qubit* %q)` |
| `qubit(0)` | `%q = call %Qubit* @__quantum__rt__qubit_allocate()` |

QIR backend leverages LLVM's `-O2` for further optimization, outputs bitcode compatible with QIR Alliance.

### QCIS Backend Detailed Mapping

| YaoXiang Operation | QCIS Instruction |
|-------------------|-----------------|
| `H(q)` (q corresponds to physical bit 2) | `H 2` |
| `CNOT(q1,q2)` (q1→bit 0, q2→bit 1) | `CNOT 0 1` |
| `measure(q)` (bit 0) | `M 0` |
| `qubit(0)` initialization | Implicit in first usage instruction, no extra instruction needed |

- Must maintain mapping table from virtual qubits (YaoXiang variables) to physical bits.
- Topology constraint checking supported (future implementation).

### Hybrid Classical Code Generation

- Classical parts (loops, conditions, integer operations) generate native code (x86/ARM) normally, interacting with quantum backend via FFI or embedded calls.
- In QIR backend, classical parts can be lowered to LLVM IR, compiled with QIR.

### Type System Impact

- Added `Qubit` and `Complex` primitive types.
- `Qubit` automatically has Move semantics, copying prohibited.
- Quantum gate function signatures must be registered in type system.

### Backward Compatibility

- ✅ Fully backward compatible
- Added built-in types and functions, no impact on existing code
- Quantum features are optional, no extra overhead when not enabled

## Trade-offs

### Advantages

- **No new syntax**: Developers only need to learn a few built-in functions to write quantum programs.
- **Type safety**: Ownership system automatically prevents qubit copying, avoiding common quantum programming errors.
- **Automatic parallelism**: DAG scheduler provides gate-level parallelism for free, no extra compiler optimization needed.
- **Ecosystem compatibility**: QIR backend enables YaoXiang to run on multiple quantum cloud platforms; QCIS backend ensures autonomous control.
- **Hybrid capability**: Classical-quantum fusion is natural, suitable for writing complex quantum algorithms (e.g., classical control in Shor, Grover).

### Disadvantages

- **Static qubit count**: Current design assumes qubit count is known at compile time; dynamic allocation requires `List[Qubit]`, but heap allocation for `List` may introduce overhead (can be mitigated with optimization).
- **Topology constraints**: No built-in coupling graph constraints for quantum chips; early users must manually ensure gate operations comply with physical topology (layout and routing passes can be added in future).
- **Post-measurement reuse**: Empty state reuse allows qubit reinitialization, but physical qubits may have relaxation times, requiring runtime system handling (currently user responsibility).

## Alternative Approaches

| Approach | Why Not Chosen |
|---------|----------------|
| Introduce `quantum` keyword and `circuit` type | Adds new syntax, high learning cost, violates YaoXiang's simplicity principle |
| Implement quantum support only as a library | Cannot leverage compiler for quantum state safety, cannot deeply integrate with DAG scheduler |
| Wait for quantum hardware to mature before support | Misses critical window for quantum programming language design |
| Reuse existing quantum frameworks (e.g., Qiskit) | Quantum semantics implemented via library, cannot obtain type system and ownership system safety guarantees |
| Design separate quantum sub-language | Increases language complexity, high maintenance cost |

## Implementation Strategy

### Phase Breakdown

| Phase | Duration | Content |
|-------|----------|---------|
| Phase 1 | 1 month | Basic quantum types and built-in functions: Add `Qubit`, `Complex` types to compiler, implement type checking for built-in functions, extend ownership checker |
| Phase 2 | 1 month | DAG scheduler recognizes quantum gates: Modify DAG construction logic, mark quantum gates as pure functions, implement parallel gate grouping output |
| Phase 3 | 2 months | QIR backend prototype: Implement DAG to QIR code generator, integrate LLVM, connect QIR simulator for verification |
| Phase 4 | 2 months | QCIS backend prototype: Implement DAG to QCIS instruction translation, design virtual-physical qubit mapping, connect domestic quantum platform for verification |
| Phase 5 | 2 months | Hybrid classical enhancement: Ensure correct code generation when classical control flow intersects with quantum gates, support `List[Qubit]`, add example programs |
| Phase 6 | 2 months | Optimization and documentation: Implement basic layout and routing, write user guide and quantum programming tutorial, release preview version |

### Risks

1. **Quantum hardware availability**: Depends on availability of external quantum simulators and real QPUs.
   - **Mitigation**: Prioritize integration with open-source simulators (QIR runner, Qiskit Aer), real QPUs as long-term goal.

2. **Backend implementation complexity**: QIR and QCIS specifications may change.
   - **Mitigation**: Abstract code generation interface, isolate backend differences, facilitate future adaptation.

3. **Performance uncertainty**: Quantum program performance characteristics differ from classical programs.
   - **Mitigation**: Provide profiling tools, let users understand gate-level parallelism effects.

## Open Questions

- [ ] **Topology constraints**: Should coupling graphs of quantum chips be supported at the language level? Users can manually specify mapping initially, automatic layout passes added in future.
- [ ] **Dynamic quantum registers**: How to map `List[Qubit]` in QCIS backend? Can generate corresponding number of physical bits, but requires runtime allocation mechanism.
- [ ] **Error mitigation**: Provide built-in error mitigation constructs (e.g., dynamic decoupling)? Can be implemented as library initially.
- [ ] **Interoperability with existing quantum SDKs**: Can QASM or QIR modules be imported? FFI can be considered in future.

## References

- [QIR Specification](https://github.com/qir-alliance/qir-spec)
- [QCIS: A Quantum Control Instruction Set](https://arxiv.org/abs/2005.12534) (USTC/GuoDang)
- [Rust Quantum Computing Examples](https://github.com/Rust-GPU/rust-gpu)
- [Qunity: A Unified Language for Quantum and Classical Computing](https://qunity-lang.org) (2025)

---

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Created by author
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  In Review  │  ← Community discussion
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
│   accepted/ │    │     rfc/    │
│  (Official) │    │  (Retain)   │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/draft/` | Author's draft, waiting for submission review |
| **In Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory, status updated |
