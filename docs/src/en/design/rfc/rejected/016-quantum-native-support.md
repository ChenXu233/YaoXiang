---
title: 'RFC 016: Quantum Native Support and Multi-Backend Integration'
status: 'Rejected'
author: 'Chenxu'
created: '2026-02-13'
updated: '2026-06-05'
---

# RFC 016: Quantum Native Support and Multi-Backend Integration

> **Rejection Reason**: Insufficient prerequisites. The Primitive::Extension mechanism has not been implemented, the language compiler is incomplete, and there is no actual user demand. Quantum support should be re-evaluated after the language matures as a consumer of the Extension mechanism.

> **Dependencies**:
>
> - [RFC-001: Concurrent Model and Error Handling System](../deprecated/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)

## Summary

This document defines **quantum-native support** and **multi-backend integration** for YaoXiang. Core idea: **YaoXiang's existing design (default spawn, ownership reflux, opaque types, DAG scheduler, generic literal parameters) naturally forms a complete foundation for quantum programming language, without introducing any new quantum-specific syntax**. We add a small number of builtin types (`Qubit`, `Complex`, `Topology`) and builtin functions (quantum gates, measurement, topology constraints), and utilize existing language mechanisms to achieve quantum-native semantics, automatic parallelism to maximize quantum utilization, hybrid classical programming, and multi-backend support.

## Motivation

### Why is quantum native support needed?

The current quantum programming ecosystem has severe fragmentation:

- **Low-level languages (QCIS, OpenQASM)**: Directly manipulate physical quantum gates, but lack type systems and abstraction mechanisms, making it difficult to write complex algorithms.
- **High-level frameworks (Qiskit, Cirq, Q#)**: Built on top of classical languages (Python, C#) extensions, quantum semantics implemented through libraries, leading to:
  - Quantum no-cloning theorem must be manually enforced by users (or rely on linear type systems retrofitted later).
  - Quantum gate operations are syntactically separated from classical code, high learning curve.
- **Hybrid computing**: Quantum and classical parts need explicit separation, lacking a unified dataflow model.

### Current Problems

YaoXiang's existing design provides exactly the complete foundation for solving these problems:

| Quantum Computing Requirement | YaoXiang Existing Design | Description |
| ------------------- | ------------------ | ------------------------------------------------------- |
| No-cloning of quantum states | **Default spawn Semantics** | Assignment moves ownership, no implicit copying, naturally complies with no-cloning theorem |
| Quantum gates as unitary transformations | **Ownership Reflux** | `q = H(q)` consumes original qubit, returns new qubit, precisely corresponding to gate semantics |
| Entangled states | **Opaque Types** | `BellPair` can only be operated as a whole, compiler tracks lifetime, prevents erroneous decomposition |
| Physical topology constraints | **Generic Literal Parameters** | `Qubit(Topology, N)` compile-time checks adjacency for two-qubit gates |
| Measurement collapse | **Void State Reuse** | After measurement, qubit becomes void, can be reinitialized, simulating quantum state collapse |
| Automatic quantum circuit parallelism | **DAG Scheduler** | Statements within functions auto-parallelize based on data dependencies, independent gates naturally concurrent |
| Hybrid classical-quantum control flow | **Unified Syntax** | Quantum and classical operations use the same `name: type = value` form |

**YaoXiang is not "adding quantum support", but discovering its design is already quantum-native.**

> **Semantic Note**: This document uses YaoXiang's **ownership semantics** to express quantum operations. The compiler guarantees no-cloning (ownership safety) at the compiler level. At the language level, "consume-create" is a **syntax expression of ownership transfer**—consume = acquire ownership, return = transfer ownership. The underlying implementation can be true reversible quantum gates (in-place quantum state modification), not truly "creating new quantum states".

### Design Goals

1. **Zero new syntax**: No introduction of keywords like `quantum`, `circuit`; all quantum features expressed through existing language mechanisms.
2. **Type safety**: Compiler guarantees quantum states are not copied or illegally used.
3. **Compile-time topology constraint checking**: Using generic literal parameters `Qubit(T, N)` to verify at compile-time whether two-qubit gate operations comply with physical topology.
4. **Transparent multi-backend**: The same quantum code can be compiled to QIR (universal ecosystem) or QCIS (domestic quantum instruction set), switched via command line arguments.
5. **Seamless hybrid classical**: Quantum computing can freely call classical functions; classical code can also operate on quantum data (via `ref` sharing, but subject to ownership constraints).

## Proposal

### Core Design

#### 1. Quantum Type System Mapping

**Base Types**:

```yaoxiang
Qubit: Type0 = primitive_qubit
Complex: Type0 = { re: Float, im: Float }
```

- `Qubit` is a first-class type, following ownership rules (spawn, RAII).
- `Complex` is used for representing amplitudes, compiler can inline optimize.

**Quantum Gates as Functions**:

```yaoxiang
# Builtin function signatures
H: (Qubit) -> Qubit = builtin_hadamard
X: (Qubit) -> Qubit = builtin_pauli_x
Y: (Qubit) -> Qubit = builtin_pauli_y
Z: (Qubit) -> Qubit = builtin_pauli_z
CNOT: (control: Qubit, target: Qubit) -> { Qubit, Qubit } = builtin_cnot
```

- All gates consume input qubits, return new qubits (or entangled pairs). Ownership reflux syntax `q = H(q)` directly corresponds to mathematical semantics.
- Multi-qubit gates return structs, accessed via pattern matching or field access.

**Measurement**:

```yaoxiang
measure: (Qubit) -> Int = builtin_measure   # Consume qubit, return classical bit
measure_all: (List(Qubit)) -> List(Int) = builtin_measure_all
```

- After measurement, qubit is consumed (becomes void), user can reinitialize via void state reuse.

**Initialization**:

```yaoxiang
qubit: (Int) -> Qubit = builtin_qubit   # 0 or 1 initialize basis state
```

#### 2. Entanglement and Opaque Type Encapsulation

Entangled pairs are encapsulated as opaque types, providing only combined operations, prohibiting decomposition:

```yaoxiang
# Builtin opaque type
BellPair: Type0 = primitive_bell_pair

# Builtin functions - can only operate as a whole
CNOT: (Qubit, Qubit) -> BellPair
measure_bell: (BellPair) -> { Int, Int }
split_bell: (BellPair) -> { Qubit, Qubit }  # split the entangled pair (use with caution)
apply_cnot_to_bell: (BellPair, Qubit) -> BellPair
```

**Key Design Points**:

- No field accessors provided, only allow whole operation via builtin functions
- `measure_bell(bp)` consumes the entire entangled pair at once, returns classical bits
- Compiler can track the complete lifetime of entangled pairs

**Comparison with Python/Qiskit**:

```
Python (Qiskit): Circuit built at runtime, errors may only be discovered after submission
YaoXiang:        Most logic errors caught at compile-time
```

**Remaining 10%** (such as physical decoherence, gate errors) are hardware issues, not solvable by the language.

#### 3. Physical Topology Constraints

Quantum chips are constrained topology graphs, **not any two qubits can perform two-qubit gates**, they must be adjacent. YaoXiang uses **generic literal parameters** to guarantee topology constraints at compile-time.

**Topology Type Definition**:

```yaoxiang
# Topology as type, containing adjacency matrix
Topology: Type0 = primitive_topology

# Builtin topology constants
Linear8: Topology = topology(8)          # Linear 8-bit: 0-1-2-3-4-5-6-7
Grid3x3: Topology = topology(3, 3)        # 3x3 grid
Ring16: Topology = topology(16, ring)    # Ring 16-bit
```

**Qubit Binding to Topology and Position**:

```yaoxiang
# Qubit(T, N) - T is topology type, N is literal position parameter
q0: Qubit(Grid3x3, 0)   # Grid3x3 topology, position (0,0)
q1: Qubit(Grid3x3, 1)   # Grid3x3 topology, position (0,1)
q2: Qubit(Grid3x3, 2)   # Grid3x3 topology, position (0,2)
q3: Qubit(Grid3x3, 3)   # Grid3x3 topology, position (1,0)
```

**Gate Operations with Automatic Constraints**:

```yaoxiang
# CNOT type signature with topology constraint
CNOT: (T: Topology, I: Int, J: Int) -> (
    (Qubit(T, I), Qubit(T, J)) -> { Qubit(T, I), Qubit(T, J) }
) when adjacent(T, I, J)

# Compile-time check
CNOT(q0, q1)  # ✅ In Grid3x3, (0,0) and (0,1) are adjacent
CNOT(q0, q2)  # ❌ Compile error: (0,0) and (0,2) are not adjacent
```

**`adjacent` Compile-Time Constraint**:

- `adjacent` is a compile-time function, using topology's adjacency matrix for static checking
- For literal indices, 100% compile-time verification
- For dynamic indices, generates runtime check code

**Virtual-to-Physical Mapping**:

```yaoxiang
# Don't know specific physical position at compile-time? Use type inference
q = qubit(Grid3x3)  # Auto-allocate position 0, subsequent derivation
```

#### 4. Ownership and Quantum State Linearity

All quantum operations follow spawn semantics, ensuring qubits are not copied:

```yaoxiang
q = qubit(0)
q2 = q          # ❌ Compile error: q has already been spawned, cannot use again
q = H(q)        # ✅ Consume q, return new q
measure(q)      # ✅ Consume q, after which q becomes void
q = qubit(0)    # ✅ Void state reuse
```

#### 4. Automatic Parallelism and DAG Scheduling

Under Standard or Full Runtime, the DAG scheduler automatically analyzes quantum programs:

```yaoxiang
apply_two_qubit_gates: () -> {Qubit, Qubit} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    # The above two lines have no data dependency, DAG auto-parallelizes execution
    CNOT(q1, q2)   # Depends on q1 and q2, auto-waits
}
```

- Scheduler utilizes `num_workers` configuration (number of physical quantum processors) to achieve true parallelism.
- Users don't need to manually arrange gate order, just describe dataflow.

#### 5. Hybrid Classical Computing

Classical and quantum code fully integrate:

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

- Quantum gates and classical control flow can be arbitrarily mixed within the same function.
- Ownership system ensures quantum variables are not incorrectly copied in classical branches.

#### 6. Multi-Backend Support Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   YaoXiang Source │     │   Type Checking │     │   DAG Intermediate│
│   (Unified Syntax)│────▶│   + Ownership Analysis │────▶│   (Dataflow Graph) │
└─────────────────┘     └─────────────────┘     └────────┬────────┘
                                                          │
                                                          ▼
                          ┌─────────────────────────────────────────────┐
                          │           Code Generation Backend (Pluggable) │
                          ├─────────────────┬───────────────────────────┤
                          │  QIR Backend    │  QCIS Backend              │
                          │  (Universal Ecosystem) │ (Domestic Quantum ISA) │
                          ├─────────────────┼───────────────────────────┤
                          │  - Output .ll file │  - Output .qcis text     │
                          │  - Adapt multiple QPUs │ - Adapt CAS/QuantumCTek hardware │
                          └─────────────────┴───────────────────────────┘
```

- **Compilation Flow**: Unified frontend → DAG construction → Backend selection → Target code generation.
- **QIR Backend**: Maps DAG nodes to QIR quantum gate intrinsics, generates LLVM bitcode, can further leverage LLVM optimizations.
- **QCIS Backend**: Serializes DAG to QCIS instructions (such as `H q0`), supports direct submission to quantum chip console.

### Examples

#### Bell State Preparation and Measurement

```yaoxiang
bell_measure: () -> {Int, Int} = () => {
    q1 = H(qubit(0))
    q2 = H(qubit(0))
    bell = CNOT(q1, q2)  # Returns BellPair opaque type
    result = measure_bell(bell)  # Measure both bits at once
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

    # Classical message passing (handled automatically by scheduler)
    # Bob's operations
    if a2 == 1 { bob_qubit = X(bob_qubit) }
    if a1 == 1 { bob_qubit = Z(bob_qubit) }
    return bob_qubit
}
```

## Detailed Design

### Builtin Types and Function Definitions

In the `compiler/builtins` module, add:

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

- `Qubit` is marked `!Copy` (default spawn), preventing implicit copying.
- Measurement function `measure` takes `Qubit` parameter (by value), consuming ownership.
- Multi-qubit gate returned record types have all fields as `Qubit`, still subject to ownership rules.

### DAG Scheduler Quantum Gate Optimization

- Quantum gate nodes are treated as pure functions (no side effects), scheduler can arbitrarily reorder independent gates.
- When scheduler outputs "quantum instruction sequence", it preserves data dependencies and groups parallel gates (applicable to multi-qubit processors).
- Supports configuring `--target-num-qubits` and `--target-topology` for future layout and routing (future expansion).

### QIR Backend Detailed Mapping

| YaoXiang Operation | QIR Instruction |
| -------------- | ----------------------------------------------------------------- |
| `H(q)` | `call void @__quantum__qis__h__body(%Qubit* %q)` |
| `CNOT(q1, q2)` | `call void @__quantum__qis__cnot__body(%Qubit* %q1, %Qubit* %q2)` |
| `measure(q)` | `%result = call i1 @__quantum__qis__mz__body(%Qubit* %q)` |
| `qubit(0)` | `%q = call %Qubit* @__quantum__rt__qubit_allocate()` |

QIR backend leverages LLVM's `-O2` for further optimization, and outputs QIR Alliance compatible bitcode.

### QCIS Backend Detailed Mapping

| YaoXiang Operation | QCIS Instruction |
| ------------------------------------ | ------------------------------------ |
| `H(q)` (q corresponds to physical qubit 2) | `H 2` |
| `CNOT(q1,q2)` (q1→qubit 0, q2→qubit 1) | `CNOT 0 1` |
| `measure(q)` (qubit 0) | `M 0` |
| `qubit(0)` initialization | Implicit in first usage instruction, no extra instruction needed |

- Must maintain mapping from virtual qubits (YaoXiang variables) to physical qubits.
- Supports topology constraint checking (future implementation).

### Hybrid Classical Code Generation

- Classical parts (loops, conditions, integer operations) normally generate native code (x86/ARM), interacting with quantum backend via FFI or embedded calls.
- In QIR backend, classical parts can be lowered to LLVM IR, compiled together with QIR.

### Type System Impact

- New primitive types `Qubit` and `Complex` added.
- `Qubit` automatically has spawn semantics, copying prohibited.
- Quantum gate function signatures need registration in type system.

### Backward Compatibility

- ✅ Fully backward compatible
- New builtin types and functions don't affect existing code
- Quantum features are optional, no extra overhead when not enabled

## Trade-offs

### Advantages

- **No new syntax**: Developers only need to learn a few builtin functions to write quantum programs.
- **Type safety**: Ownership system automatically prevents qubit copying, avoiding common quantum programming errors.
- **Automatic parallelism**: DAG scheduler provides gate-level parallelism for free, no additional compiler optimization needed.
- **Ecosystem compatibility**: QIR backend allows YaoXiang to run on multiple quantum cloud platforms; QCIS backend ensures autonomy and control.
- **Hybrid capability**: Classical-quantum integration is natural, suitable for writing complex quantum algorithms (like classical control in Shor, Grover).

### Disadvantages

- **Static qubit count**: Current design assumes qubit count is known at compile-time; dynamic allocation via `List(Qubit)`, but `List`'s heap allocation may introduce extra overhead (can be mitigated via optimization).
- **Post-measurement reuse**: Void state reuse allows re-initializing qubits, but physical qubits may have relaxation time, requiring runtime system handling (currently user responsibility).
- **Dynamic topology mapping**: When physical topology is only known at runtime, compile-time checking cannot take effect, requiring runtime check code generation (current version only supports static checking).

## Alternative Solutions

| Approach | Why Not Chosen |
| -------------------------------------- | ---------------------------------------------------------- |
| Introduce `quantum` keyword and `circuit` type | Adds new syntax, high learning cost, violates YaoXiang's simplicity design principle |
| Implement quantum support only as a library | Cannot use compiler to guarantee quantum state safety, cannot deeply integrate with DAG scheduler |
| Wait for quantum hardware to mature before supporting | Misses critical window for quantum programming language design |
| Reuse existing quantum frameworks (like Qiskit) | Quantum semantics implemented through library, cannot get type system and ownership system safety guarantees |
| Design separate quantum sublanguage | Increases language complexity, high maintenance cost |

## Implementation Strategy

### Phase Division

| Phase | Duration | Content |
| ------- | ----- | -------------------------------------------------------------------------------------------------------- |
| Phase 1 | 1 month | Basic quantum types and builtin functions: add `Qubit`, `Complex` types to compiler, implement type checking for builtin functions, extend ownership checker |
| Phase 2 | 1 month | DAG scheduler recognizes quantum gates: modify DAG construction logic, mark quantum gates as pure functions, implement parallel gate grouping output |
| Phase 3 | 2 months | QIR backend prototype: implement DAG to QIR code generator, integrate LLVM, connect QIR simulator for verification |
| Phase 4 | 2 months | QCIS backend prototype: implement DAG to QCIS instruction translator, design virtual-physical qubit mapping, connect domestic quantum platform for verification |
| Phase 5 | 2 months | Hybrid classical enhancement: ensure correct code generation for classical control flow interleaved with quantum gates, support `List(Qubit)`, add example programs |
| Phase 6 | 2 months | Optimization and documentation: implement basic layout and routing, write user guide and quantum programming tutorial, release preview version |

### Risks

1. **Quantum hardware availability**: Depends on external quantum simulator and real QPU availability.
   - **Mitigation**: Prioritize connecting to open-source simulators (QIR runner, Qiskit Aer), real QPU as long-term goal.

2. **Backend implementation complexity**: QIR and QCIS specifications may change.
   - **Mitigation**: Abstract code generation interface, isolate backend differences, facilitate subsequent adaptation.

3. **Performance uncertainty**: Quantum program performance characteristics differ from classical programs.
   - **Mitigation**: Provide performance profiling tools, let users understand gate-level parallelism effects.

## Open Questions

- [x] **Topology constraints**: Already implemented via `Qubit(Topology, N)` generic literal parameter compile-time checking.
- [ ] **Dynamic quantum registers**: How does `List(Qubit)` map in QCIS backend? Can generate corresponding number of physical qubits, but needs runtime allocation mechanism.
- [ ] **Error mitigation**: Whether to provide builtin error mitigation (like dynamical decoupling) constructs? Can first implement as library.
- [ ] **Interoperability with existing quantum SDKs**: Can import QASM or QIR modules? Can consider FFI in the future.
- [ ] **Automatic layout and routing**: When virtual qubit count exceeds physical qubit count, how to auto-map?

## References

- [QIR Specification](https://github.com/qir-alliance/qir-spec)
- [QCIS: A Quantum Control Instruction Set](https://arxiv.org/abs/2005.12534) (USTC/QuantumCTek)
- [Rust Quantum Computing Examples](https://github.com/Rust-GPU/rust-gpu)
- [Qunity: A Unified Language for Quantum and Classical Computing](https://qunity-lang.org) (2025)

---

## Lifecycle and Fate

```
┌─────────────┐
│   Draft     │  ← Created by author
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← Community discussion
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
│ (formal design) │    │ (preserved in place) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
| ---------- | ------------------------ | ------------------------------ |
| **Draft** | `docs/design/rfc/draft/` | Author draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory, status updated |