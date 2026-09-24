---
title: 'RFC 016: Quantum Native Support and Multi-Backend Integration'
status: 'Rejected'
author: 'Chenxu'
created: '2026-02-13'
updated: '2026-06-05'
---

# RFC 016: Quantum Native Support and Multi-Backend Integration

> **Rejection Reason**: Insufficient prerequisites. The Primitive::Extension mechanism has not yet
> been implemented, the language compiler is incomplete, and there is no real user demand. Quantum
> support should be reassessed as a consumer of the Extension mechanism after the language matures.

> **Dependencies**:
>
> - [RFC-001: Spawn Model and Error Handling System](../deprecated/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics Type System Design](../accepted/011-generic-type-system.md)

## Summary

This document defines **quantum native support** and **multi-backend integration** for the YaoXiang
language. The core idea: **YaoXiang's existing design (default Move, ownership backflow, opaque
types, DAG scheduler, generics constant parameters) naturally forms a complete foundation for a
quantum programming language, without introducing any new quantum-specific syntax**. We implement
quantum native semantics, automatic parallelization that maximizes quantum utilization, hybrid
classical programming, and multi-backend support by adding a small number of builtin types (`Qubit`,
`Complex`, `Topology`) and builtin functions (quantum gates, measurement, topology constraints), and
leveraging existing language mechanisms.

## Motivation

### Why is quantum native support needed?

The current quantum programming ecosystem is severely fragmented:

- **Low-level languages (QCIS, OpenQASM)**: Directly manipulate physical quantum gates, but lack
  type systems and abstraction mechanisms, making it difficult to write complex algorithms.
- **High-level frameworks (Qiskit, Cirq, Q#)**: Extended from classical languages (Python, C#),
  where quantum semantics are implemented through libraries, leading to:
  - The no-cloning rule of quantum states must be manually followed by users (or rely on linear type
    systems added later).
  - Quantum gate operations are syntactically disjoint from classical code, resulting in a high
    learning cost.
- **Hybrid computing**: Quantum and classical parts must be explicitly separated, lacking a unified
  data flow model.

### Current Issues

YaoXiang's existing design provides a complete foundation for addressing these issues:

| Quantum Computing Requirement             | YaoXiang's Existing Design       | Description                                                                                                                                       |
| ----------------------------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| No-cloning of quantum states              | **Default Move Semantics**       | Assignment means ownership transfer, no implicit copy, naturally conforms to the no-cloning theorem                                               |
| Quantum gates as unitary transformations  | **Ownership Backflow**           | `q = H(q)` consumes the original qubit, returns a new qubit, precisely matching gate semantics                                                    |
| Entangled states                          | **Opaque Types**                 | `BellPair` can only be operated on as a whole; the compiler tracks the lifecycle to prevent incorrect decomposition                               |
| Physical topology constraints             | **Generics Constant Parameters** | `Qubit(Topology, N)` performs compile-time adjacency checking                                                                                     |
| Measurement collapse                      | **Void State Reuse**             | After measurement the qubit becomes void and can be reinitialized, simulating quantum state collapse                                              |
| Automatic parallelism of quantum circuits | **DAG Scheduler**                | Statements within a function are automatically parallelized based on data dependencies; gates with no dependencies execute concurrently by nature |
| Hybrid classical-quantum control flow     | **Unified Syntax**               | Quantum and classical operations use the same `name: type = value` form                                                                           |

**YaoXiang is not "adding quantum support", but discovering that its own design is already
quantum-native.**

> **Semantic Note**: This document uses YaoXiang's **ownership semantics** to express quantum
> operations. The compiler guarantees no-cloning (ownership safety). The "consume-create" at the
> language level is the **syntactic expression of ownership transfer**—consume = acquire ownership,
> return = transfer ownership. The underlying implementation can be true reversible quantum gates
> (in-place modification of the quantum state), rather than truly "creating a new quantum state".

### Design Goals

1. **Zero new syntax**: Do not introduce keywords like `quantum` or `circuit`; all quantum features
   are expressed through existing language mechanisms.
2. **Type safety**: The compiler guarantees that quantum states are not copied or used illegally.
3. **Compile-time checking of topology constraints**: Validate two-qubit gate operations against
   physical topology at compile time via the generics constant parameter `Qubit(T, N)`.
4. **Transparent multi-backend**: The same quantum code can be compiled to QIR (general ecosystem)
   or QCIS (domestic quantum instruction set), switched via command-line arguments.
5. **Seamless hybrid classical-quantum**: Quantum computations can freely call classical functions,
   and classical code can also operate on quantum data (via `ref` sharing, but subject to ownership
   constraints).

## Proposal

### Core Design

#### 1. Quantum Type System Mapping

**Basic Types**:

```yaoxiang
Qubit: Type0 = primitive_qubit
Complex: Type0 = { re: Float, im: Float }
```

- `Qubit` is a first-class type, subject to ownership rules (Move, RAII).
- `Complex` is used to represent amplitudes; the compiler can inline-optimize it.

**Quantum Gates as Functions**:

```yaoxiang
# Builtin function signatures
H: (Qubit) -> Qubit = builtin_hadamard
X: (Qubit) -> Qubit = builtin_pauli_x
Y: (Qubit) -> Qubit = builtin_pauli_y
Z: (Qubit) -> Qubit = builtin_pauli_z
CNOT: (control: Qubit, target: Qubit) -> { Qubit, Qubit } = builtin_cnot
```

- All gates consume the input qubit and return a new qubit (or entangled pair). The ownership
  backflow syntax `q = H(q)` directly corresponds to the mathematical semantics.
- Multi-qubit gates return a struct; results are obtained via pattern matching or field access.

**Measurement**:

```yaoxiang
measure: (Qubit) -> Int = builtin_measure   # consume qubit, return classical bit
measure_all: (List(Qubit)) -> List(Int) = builtin_measure_all
```

- After measurement, the qubit is consumed (becomes void); the user can reinitialize it via void
  state reuse.

**Initialization**:

```yaoxiang
qubit: (Int) -> Qubit = builtin_qubit   # initialize basis state with 0 or 1
```

#### 2. Entanglement and Opaque Type Encapsulation

Encapsulate entangled pairs as opaque types; only provide composition operations and prohibit
decomposition:

```yaoxiang
# Builtin opaque type
BellPair: Type0 = primitive_bell_pair

# Builtin functions - can only be operated on as a whole
CNOT: (Qubit, Qubit) -> BellPair
measure_bell: (BellPair) -> { Int, Int }
split_bell: (BellPair) -> { Qubit, Qubit }  # split the entangled pair (use with caution)
apply_cnot_to_bell: (BellPair, Qubit) -> BellPair
```

**Key Design**:

- No field accessors are provided; only operations through builtin functions are allowed
- `measure_bell(bp)` consumes the entire entangled pair at once, returning classical bits
- The compiler can track the complete lifecycle of entangled pairs

**Comparison with Python/Qiskit**:

```
Python (Qiskit): Circuit is built at runtime; errors may only be discovered after submission
YaoXiang:       Most logic errors are caught at compile time
```

**The remaining 10%** (e.g., physical decoherence, gate errors) are hardware issues, not solvable by
the language.

#### 3. Physical Topology Constraints

Quantum chips are constrained topology graphs—**not every pair of qubits can perform a two-qubit
gate; they must be adjacent**. YaoXiang uses **generics constant parameters** to guarantee topology
constraints at compile time.

**Topology Type Definition**:

```yaoxiang
# Topology as a type, containing the adjacency matrix
Topology: Type0 = primitive_topology

# Builtin topology constants
Linear8: Topology = topology(8)          # Linear 8-qubit: 0-1-2-3-4-5-6-7
Grid3x3: Topology = topology(3, 3)        # 3x3 grid
Ring16: Topology = topology(16, ring)    # 16-qubit ring
```

**Qubit Binds Topology and Position**:

```yaoxiang
# Qubit(T, N) - T is the topology type, N is the constant position parameter
q0: Qubit(Grid3x3, 0)   # Grid3x3 topology, position (0,0)
q1: Qubit(Grid3x3, 1)   # Grid3x3 topology, position (0,1)
q2: Qubit(Grid3x3, 2)   # Grid3x3 topology, position (0,2)
q3: Qubit(Grid3x3, 3)   # Grid3x3 topology, position (1,0)
```

**Automatic Gate Operation Constraints**:

```yaoxiang
# CNOT type signature with topology constraint
CNOT: (T: Topology, I: Int, J: Int) -> (
    (Qubit(T, I), Qubit(T, J)) -> { Qubit(T, I), Qubit(T, J) }
) when adjacent(T, I, J)

# Compile-time check
CNOT(q0, q1)  # ✅ (0,0) and (0,1) are adjacent in Grid3x3
CNOT(q0, q2)  # ❌ Compilation error: (0,0) and (0,2) are not adjacent
```

**`adjacent` Compile-time Constraint**:

- `adjacent` is a compile-time function that uses the topology's adjacency matrix for static
  checking
- 100% compile-time verification when indices are constant
- Generates runtime checking code when indices are dynamic

**Virtual-to-Physical Mapping**:

```yaoxiang
# Don't know the specific physical location at compile time? Use type inference
q = qubit(Grid3x3)  # Automatically assign position 0, inferred subsequently
```

#### 4. Linear Flow of Ownership and Quantum States

All quantum operations follow Move semantics, ensuring qubits are not copied:

```yaoxiang
q = qubit(0)
q2 = q          # ❌ Compilation error: q has been moved and cannot be used again
q = H(q)        # ✅ Consume q, return new q
measure(q)      # ✅ Consume q; q becomes void afterwards
q = qubit(0)    # ✅ Void state reuse
```

#### 4. Automatic Parallelism and DAG Scheduling

Under Standard or Full Runtime, the DAG scheduler automatically analyzes quantum programs:

```yaoxiang
apply_two_qubit_gates: () -> {Qubit, Qubit} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    # The two lines above have no data dependencies; DAG automatically parallelizes
    CNOT(q1, q2)   # Depends on q1 and q2; automatically waits
}
```

- The scheduler utilizes the `num_workers` configuration (number of physical quantum processors) to
  achieve true parallelism.
- Users don't need to manually arrange the gate order; they only need to describe the data flow.

#### 5. Hybrid Classical Computing

Classical and quantum code are fully fused:

```yaoxiang
grover_search: (target: Int) -> Int = () => {
    n = 4
    qubits = List(Qubit)()
    for i in 0..n {
        qubits.append(H(qubit(0)))
    }
    # Classical loop mixed with quantum operations
    oracle(qubits, target)   # oracle is a sequence of quantum gates
    qubits = diffusion(qubits)
    results = measure_all(qubits)
    return decode_result(results)   # classical post-processing
}
```

- Classical control flow and quantum gates can be mixed freely within the same function.
- The ownership system ensures quantum variables are not incorrectly copied in classical branches.

#### 6. Multi-Backend Support Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   YaoXiang src   │     │   Type Check    │     │   DAG IR        │
│  (unified syntax)│────▶│ + ownership ana.│────▶│  (dataflow graph)│
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                          ┌─────────────────────────────────────────────┐
                          │        Code generation backend (pluggable)  │
                          ├─────────────────┬───────────────────────────┤
                          │  QIR backend    │  QCIS backend             │
                          │  (general eco.) │  (domestic quantum ISA)   │
                          ├─────────────────┼───────────────────────────┤
                          │  - emit .ll     │  - emit .qcis text        │
                          │  - adapt multi QPU│  - adapt CAS/QuantumCTek│
                          └─────────────────┴───────────────────────────┘
```

- **Compilation flow**: unified frontend → DAG construction → backend selection → target code
  generation.
- **QIR backend**: Map DAG nodes to QIR's quantum gate intrinsics, generate LLVM bitcode, which can
  further leverage LLVM optimizations.
- **QCIS backend**: Serialize DAG into QCIS instructions (e.g., `H q0`), support direct submission
  to quantum chip consoles.

### Examples

#### Bell State Preparation and Measurement

```yaoxiang
bell_measure: () -> {Int, Int} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    bell = CNOT(q1, q2)  # returns BellPair opaque type
    result = measure_bell(bell)  # measure both bits at once
    return result
}
```

#### Quantum Teleportation (Simplified)

```yaoxiang
teleport: (msg: Qubit, bell: BellPair) -> Qubit = (msg, bell) => {
    # Split the entangled pair to obtain two independent qubits
    (alice_qubit, bob_qubit) = split_bell(bell)

    # Alice's operations
    (msg, alice_qubit) = CNOT(msg, alice_qubit)
    msg = H(msg)
    a1 = measure(msg)
    a2 = measure(alice_qubit)

    # Classical information transmission (handled automatically by the scheduler via dependencies)
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
- The parameter of the measurement function `measure` is `Qubit` (passed by value), consuming
  ownership.
- In records returned by multi-qubit gates, the fields are all `Qubit`, which must still obey
  ownership rules.

### DAG Scheduler Optimization for Quantum Gates

- Quantum gate nodes are treated as pure functions (no side effects); the scheduler may freely
  reorder gates with no dependencies.
- When the scheduler outputs the "quantum instruction sequence", it preserves data dependencies and
  groups parallel gates (applicable to multi-quantum-processor setups).
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

| YaoXiang Operation                    | QCIS Instruction                                       |
| ------------------------------------- | ------------------------------------------------------ |
| `H(q)` (q corresponds to phys. bit 2) | `H 2`                                                  |
| `CNOT(q1,q2)` (q1→bit 0, q2→bit 1)    | `CNOT 0 1`                                             |
| `measure(q)` (bit 0)                  | `M 0`                                                  |
| `qubit(0)` initialization             | Implicit in the first use; no extra instruction needed |

- A mapping table from virtual qubits (YaoXiang variables) to physical bits must be maintained.
- Topology constraint checking is supported (future implementation).

### Hybrid Classical Code Generation

- Classical parts (loops, conditions, integer operations) are generated as native code (x86/ARM) as
  usual, interacting with the quantum backend via FFI or inlined calls.
- In the QIR backend, classical parts can be lowered to LLVM IR and mixed-compiled with QIR.

### Type System Impact

- Add `Qubit` and `Complex` primitive types.
- `Qubit` automatically carries Move semantics; copying is forbidden.
- Quantum gate function signatures need to be registered in the type system.

### Backward Compatibility

- ✅ Fully backward compatible
- New builtin types and functions do not affect existing code
- Quantum features are optional; there is no extra overhead when not enabled

## Trade-offs

### Advantages

- **No new syntax**: Developers only need to learn a small number of builtin functions to write
  quantum programs.
- **Type safety**: The ownership system automatically prevents qubit copying, avoiding common
  quantum programming errors.
- **Automatic parallelism**: The DAG scheduler provides gate-level parallelism for free, without
  extra compiler optimization.
- **Ecosystem compatibility**: The QIR backend allows YaoXiang to run on multiple quantum cloud
  platforms; the QCIS backend ensures autonomy and controllability.
- **Hybrid capability**: Classical-quantum fusion is natural, suitable for writing complex quantum
  algorithms (e.g., classical control in Shor, Grover).

### Disadvantages

- **Static qubit count**: The current design assumes the qubit count is known at compile time;
  dynamic allocation requires `List(Qubit)`, but heap allocation of `List` may introduce extra
  overhead (can be mitigated by optimization).
- **Reuse after measurement**: Void state reuse allows reinitializing qubits, but physical qubits
  may have relaxation times, which need to be handled by the runtime system (currently the user's
  responsibility).
- **Dynamic topology mapping**: When the physical topology is only known at runtime, compile-time
  checking cannot take effect, and runtime checking code must be generated (current version supports
  static checking only).

## Alternatives

| Approach                                              | Why Not Chosen                                                                                               |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Introduce `quantum` keyword and `circuit` type        | Adds new syntax, higher learning cost, violates YaoXiang's minimalist design principle                       |
| Implement quantum support as a library only           | Cannot leverage the compiler to guarantee quantum state safety; cannot deeply integrate with DAG scheduler   |
| Wait for quantum hardware to mature before supporting | Misses the critical window for quantum programming language design                                           |
| Reuse existing quantum frameworks (e.g., Qiskit)      | Quantum semantics are implemented via libraries, without the safety guarantees of type and ownership systems |
| Design a separate quantum sub-language                | Increases language complexity and maintenance cost                                                           |

## Implementation Strategy

### Phased Plan

| Phase   | Duration | Content                                                                                                                                                              |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 | 1 month  | Basic quantum types and builtin functions: add `Qubit`, `Complex` types to the compiler, implement type checking for builtin functions, extend the ownership checker |
| Phase 2 | 1 month  | DAG scheduler recognizes quantum gates: modify DAG construction logic, mark quantum gates as pure functions, implement parallel gate grouping output                 |
| Phase 3 | 2 months | QIR backend prototype: implement DAG-to-QIR code generator, integrate LLVM, connect QIR simulator for verification                                                   |
| Phase 4 | 2 months | QCIS backend prototype: implement DAG-to-QCIS instruction translation, design virtual-to-physical bit mapping, connect to domestic quantum platform for verification |
| Phase 5 | 2 months | Hybrid classical enhancement: ensure classical control flow and quantum gates are correctly cross-generated, support `List(Qubit)`, add example programs             |
| Phase 6 | 2 months | Optimization and documentation: implement basic layout and routing, write user guide and quantum programming tutorial, release preview version                       |

### Risks

1. **Quantum hardware availability**: Depends on the availability of external quantum simulators and
   real QPUs.
   - **Mitigation**: Prioritize integration with open-source simulators (QIR runner, Qiskit Aer);
     real QPUs as a long-term goal.

2. **Backend implementation complexity**: The QIR and QCIS specifications may change.
   - **Mitigation**: Abstract the code generation interface, isolate backend differences, and ease
     future adaptation.

3. **Performance uncertainty**: The performance characteristics of quantum programs differ from
   classical ones.
   - **Mitigation**: Provide performance profiling tools to help users understand gate-level
     parallelism effects.

## Open Questions

- [x] **Topology constraints**: Already implemented as compile-time checking via the
      `Qubit(Topology, N)` generics constant parameter.
- [ ] **Dynamic quantum registers**: How does `List(Qubit)` map in the QCIS backend? It can generate
      the corresponding number of physical bits, but a runtime allocation mechanism is required.
- [ ] **Error mitigation**: Whether to provide builtin error-mitigation constructs (e.g., dynamical
      decoupling)? Can be implemented as a library first.
- [ ] **Interoperability with existing quantum SDKs**: Can we import QASM or QIR modules? FFI may be
      considered in the future.
- [ ] **Automatic layout and routing**: How to automatically map when the number of virtual qubits
      exceeds the number of physical qubits?

## References

- [QIR Specification](https://github.com/qir-alliance/qir-spec)
- [QCIS: A Quantum Control Instruction Set](https://arxiv.org/abs/2005.12534) (USTC/QuantumCTek)
- [Rust Quantum Computing Examples](https://github.com/Rust-GPU/rust-gpu)
- [Qunity: A Unified Language for Quantum and Classical Computing](https://qunity-lang.org) (2025)

---

## Lifecycle and Destination

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
│  accepted/  │    │    rfc/     │
│ (official)  │    │  (kept in place) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status        | Location                 | Description                                                   |
| ------------- | ------------------------ | ------------------------------------------------------------- |
| **Draft**     | `docs/design/rfc/draft/` | Author's draft, awaiting submission for review                |
| **In Review** | `docs/design/rfc/`       | Open community discussion and feedback                        |
| **Accepted**  | `docs/design/accepted/`  | Becomes an official design document and enters implementation |
| **Rejected**  | `docs/design/rfc/`       | Remains in the RFC directory, status updated                  |
