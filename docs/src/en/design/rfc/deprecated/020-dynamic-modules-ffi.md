---
title: 'RFC-020: Dynamic Modules and FFI Integration'
status: 'Deprecated'
author: 'Chenxu'
created: '2026-03-14'
updated: '2026-06-05 (Deprecated)'
---

# RFC-020: Dynamic Modules and FFI Integration

> **⚠️ Deprecated**: This document is deprecated; its content has been merged into
> [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md).

> **References**:
>
> - [RFC-001: spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
> - [RFC-021: Library-driven FFI Extension and Cross-language Invocation Support](./021-library-driven-ffi-extension.md)

## Abstract

Building on RFC-001, 008, and 018, this document further refines and extends YaoXiang's concurrency
model to address practical scenarios such as **dynamic module loading**, **Foreign Function
Interface (FFI)**, and **more fine-grained scheduling optimization**. The core designs include:

1. **Dynamic module metadata contract**: Provides compile-time dependency descriptions for dynamic
   libraries written in the same language, enabling the main program to statically construct a DAG
   while preserving transparent concurrency.
2. **FFI scheduling semantics**: Foreign functions default to `@block` nodes in the DAG, and can be
   integrated into parallel scheduling via annotations (see RFC-021 for the FFI toolchain).
3. **Call-context-based optimization**: Replacing static threshold fallbacks, the compiler
   intelligently decides whether to inline or schedule as an independent node based on the
   function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Mechanism for merging control flow with the DAG**: Through Phi nodes and dynamic unrolling,
   dynamic structures like `if` and `loop` are naturally incorporated into the data flow graph.
5. **Runtime scheduler memory and performance optimization**: Clearly defining node lifecycle
   management, region allocation, lock-free queues, and other low-cost abstraction implementations.

This document aims to refine the language specification, ensuring that YaoXiang's concurrency model
can both handle static whole-program analysis and flexibly address dynamism and external
interactions, while maintaining high performance and developer experience.

## Motivation

### Limitations of the Current Design

RFC-001/008/018 constructs an elegant transparent concurrency model, but there are still blind spots
when facing real-world requirements:

- **Dynamic modules**: When programs support plugins or dynamic libraries, the main program cannot
  know the internal call relationships and dependencies of the modules at compile time, leading to
  failure of global DAG construction.
- **FFI calls**: Foreign functions (such as C libraries) are completely black boxes, internally
  containing concurrency, blocking, or side effects; treating them as ordinary nodes directly would
  break concurrency safety.
- **Scheduling overhead for small functions**: The "L1 automatic fallback" proposed in RFC-001 uses
  a static threshold (instruction count < 50). Such implicit rules make it hard for developers to
  predict behavior and cannot adapt to complex call contexts.
- **Fusion of control flow and DAG**: The representation of dynamic structures like `if` and `loop`
  in the DAG is not yet clear, which may affect the accuracy of dependency analysis.
- **Runtime overhead control**: As the number of DAG nodes increases, scheduler memory management
  and performance optimization need to be explicitly designed to avoid becoming a bottleneck.

### Goals

- While preserving the core philosophy of transparent concurrency, provide clear, safe, and
  progressive support for dynamic modules and FFI.
- Shift scheduling optimization from "implicit global rules" to "context-aware intelligent
  decisions", improving predictability and performance.
- Refine the DAG representation of dynamic control flow, ensuring all program structures can
  naturally fit into the data flow model.
- Clarify the scheduler's memory management and performance optimization strategies, achieving
  low-cost abstraction.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library compiled with YaoXiang (`.yxo` / platform-specific dynamic library) must be
accompanied by a **metadata description file** (`.yxmeta`), containing:

- **List of exported functions**: Complete type signature of each function (parameters, return
  value, resource markers).
- **Side-effect markers**: Compiler-inferred `@pure` / `@io` (developers can also explicitly
  override).
- **Resource dependencies**: Whether each parameter is a resource type (e.g., `File`), whether the
  return value contains new resources.
- **Call graph summary (optional)**: IDs of other exported functions that this function may call,
  used for cross-module circular dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrow/move), ownership of return
  value.
- **Concurrency safety**: Auto-inferred results for satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (e.g., MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-time Processing

When the main program is compiled and encounters a call to a dynamic module function:

1. Read the corresponding module's `.yxmeta` file.
2. Create **placeholder nodes** in the global DAG, recording the input/output dependencies,
   side-effect markers, etc. obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can
   plan execution order in advance.

#### 1.3 Runtime Binding

When the dynamic module is loaded:

- The runtime verifies whether the actual function signature is consistent with the metadata (to
  prevent version mismatch).
- Bind the placeholder node to the actual function pointer.

**Regarding subgraph scheduling semantics**: If the dynamic module has an independent sub-DAG
internally (e.g., the module itself contains concurrent logic), the subgraph will execute as an
**independent scheduling unit**. Its boundary is defined by the module's exported functions: when an
exported function is called, the subgraph begins execution as a whole until the function returns.
The internal node scheduling of the subgraph is handled by the subgraph's own scheduler (the module
can continue to use the standard scheduler internally), but the interaction between the subgraph and
the main DAG is limited to input/output data flow—placeholder nodes in the main DAG only care about
the start and end of the subgraph, not its internal scheduling. This design guarantees module
encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If the dynamic module violates the contract (e.g., claims `@pure` but modifies global state), the
  consequences are borne by the developer (similar to the unsafe boundary in FFI). However, since it
  is the same language, safety can be enhanced through runtime checks (e.g., memory isolation), but
  this will add overhead.
- **Cross-module circular dependencies**: If module A calls B, and B calls A, and the call
  relationship has been declared in the metadata, the compiler can detect and report an error; if
  not declared, a deadlock may occur at runtime, detected by the scheduler and resulting in a panic.

### 2. FFI Scheduling Semantics in the DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion,
memory ownership) is defined in [RFC-021](./021-library-driven-ffi-extension.md). This section only
describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

Foreign functions (declared via `native("symbol")`) default to **`@block` nodes** in the DAG:

- Do not participate in DAG parallel scheduling, execute synchronously on the current thread.
- The scheduler does not intervene in its internal concurrency during execution.
- The return value is available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can integrate FFI calls into DAG scheduling via annotations (see
[RFC-021 §2.2](./021-library-driven-ffi-extension.md) for details):

- `@pure`: Treated as a regular DAG node, can run in parallel with other dependency-free nodes.
- `@io`: Participates in resource dependency analysis, automatically serializing multiple calls to
  the same resource.

#### 2.3 Impact on the Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler, the only difference
being that the `effect` is marked as `Block`. When the scheduler encounters a `Block` node, it skips
parallel scheduling and executes synchronously.

### 3. Call-context-based Optimization

Replacing the "L1 automatic fallback" static threshold in RFC-001, the compiler now makes
**intelligent decisions based on the actual context of each call site in the DAG**.

#### 3.1 Basis for Optimization Decisions

The compiler analyzes each function call node:

- **Number of consumers**: How many downstream nodes use the result of this node. If 1, it is an
  inlining candidate; if greater than 1, it must be retained as an independent node for result
  sharing.
- **Side effects**: If the node has `@io` side effects, it must be retained as an independent node
  to guarantee ordering.
- **Computation cost estimation**: Heuristics such as instruction count can still be referenced, but
  not as a hard threshold, only for evaluating inlining benefits.
- **Resource dependencies**: If the node involves resource variables (e.g., `File`) and the resource
  variables are passed between upstream and downstream, inlining may break the dependency chain and
  should be done with caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- Embed the computation logic of this node directly into the code of its sole downstream node.
- Remove this node from the DAG, its inputs directly become the inputs of the downstream node.
- During final code generation, inlined functions do not generate independent scheduling units.

#### 3.3 Inlining Limitations

- Calls within recursive functions or loop bodies are usually not inlined, to prevent infinite
  unrolling.
- Functions across module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to force-disable inlining, or `@forceinline` to hint
  the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (can be enabled via
`--emit-optimization-report`), containing the following information:

- **Each inlining point**: List the inlined function name, call location, and inlining reason (e.g.,
  "single consumer and pure function").
- **Reason for retaining as an independent node**: e.g., "multiple consumers", "has side effects",
  "cross-module call", etc.
- **Decision statistics**: Total inlinings, number of retained nodes, to help developers evaluate
  the optimization effect.

The report output format can be text or JSON, for easy tool parsing.

### 4. Merging Control Flow with the DAG

#### 4.1 Handling Conditional Branches (`if`)

Introduce **Phi nodes** (borrowed from SSA form) to represent branch convergence points:

- At compile time, for each `if` expression, construct:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and the outputs of both branches.
- Semantics of the Phi node: when the condition variable is ready, select the output of the
  corresponding branch as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it
  dynamically adds itself to the downstream list of the chosen branch and waits for that branch's
  result.

Example DAG:

```
        cond
       /    \
  then DAG  else DAG
       \    /
        Phi
         |
      Subsequent nodes
```

#### 4.2 Handling Loops (`loop`/`while`)

A loop is treated as a sub-DAG with feedback edges, **dynamically unrolled on demand at runtime**:

- At compile time, identify the loop body and construct a **loop template**, containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when the loop result is needed (e.g., the accumulated value is used after the loop
  ends), the scheduler begins dynamically unrolling iterations:
  1. First schedule the condition node; if true, instantiate the sub-DAG for the first iteration,
     whose inputs include the initial state and external variables.
  2. After the iteration completes, a new state is generated, and the condition node is scheduled
     again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex example: loop condition depends on internal updates within the loop body**

```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```

In this pattern, the condition node `x < 10` depends on `x` updated after each iteration. The DAG is
represented as follows:

- The loop template contains the state variable `x` with an initial value of 0.
- Each iteration: first execute the condition node (depending on the current `x`), if true, execute
  `x = compute(x)` and generate a new `x`, then enter the condition node again.
- The runtime dynamically unrolls according to the above process until the condition is false.

Dependencies between iterations naturally form a data flow through state variables; iterations with
dependencies are automatically serialized, and iterations without dependencies can run in parallel
(e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop is executed synchronously directly as the main DAG (no scheduling overhead);
multiple infinite loops serve as background DAGs, concurrently executed by the scheduler via
time-slicing.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable), representing the number of consumers
  that depend on its result.
- After the node finishes executing and passes the result to all downstream nodes, the reference
  count reaches zero, and the node memory can be released.
- The result value itself also uses reference counting (`Arc<T>`), but can be optimized: if the
  result is used by only one consumer, ownership is moved directly, avoiding counting overhead.

#### 5.2 Region Memory Allocation (Arena)

For dynamically generated large numbers of short-lifecycle nodes (e.g., loop iterations), use a
**region allocator**:

- Allocate a memory region for a single loop unrolling.
- Nodes within the region are allocated contiguously and released as a whole, reducing fragmentation
  and deallocation overhead.
- When the region ends, all node memory is reclaimed at once.

#### 5.3 Lock-free Data Structures

- **Dependency counters**: Use `AtomicUsize`, decrementing atomically via `fetch_sub`.
- **Ready queue**: Use Chase-Lev double-ended queue (per-thread local queue + work stealing),
  reducing lock contention.
- **Downstream list**: Read-only after creation, avoiding concurrent modification.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute synchronously directly, with zero
  scheduling overhead.
- Dynamically adjust the degree of parallelism based on task granularity and system load (e.g.,
  adjust the number of worker threads by monitoring queue length).

#### 5.5 Low-cost Abstraction Principle

All scheduling overhead is proportional to the number of tasks; the additional overhead per task
(creation, enqueuing, dependency handling) is controlled at tens of nanoseconds. For
ultra-fine-grained tasks, scheduling is avoided through inlining optimization (see Section 3).

## Detailed Design

### 6.1 Dynamic Module Metadata Format (Draft)

```rust
// Metadata file structure (simplified)
struct Metadata {
    version: u32,
    functions: Vec<FuncMeta>,
}

struct FuncMeta {
    name: String,
    signature: TypeSignature,
    effects: EffectTag,      // Pure | IO | Block
    resource_params: Vec<usize>, // List of parameter indices indicating which parameters are resource types
    calls: Vec<String>,       // Names of other exported functions called (optional)
    ownership: OwnershipInfo,
    send_sync: SendSync,      // Whether Send/Sync is satisfied
}
```

### 6.2 Scheduler Core Data Structures

```rust
struct TaskNode {
    id: TaskId,
    deps: Vec<TaskId>,                // Upstream dependencies
    remaining_deps: AtomicUsize,
    inputs: Vec<Option<Value>>,
    result: Option<Value>,
    func: Executable,
    downstream: Vec<TaskId>,           // Downstream nodes (read-only after creation)
    effect: EffectTag,
    arena_id: Option<ArenaId>,         // Associated region (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queue
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Region allocator
}
```

### 6.3 Context-based Optimization Analysis

The compiler performs the following steps at the MIR layer:

1. Build a global call graph and data dependency graph.
2. For each function call node, calculate its out-degree (number of consumers).
3. If the out-degree is 1, and the function has no side effects (`@pure`), and is non-recursive,
   mark it as an "inlining candidate".
4. Combine heuristics (e.g., instruction count) to evaluate inlining benefits, and decide whether to
   inline.
5. When inlining, embed the call node's code into its downstream, and update the dependency
   relationships.

### 6.4 Control Flow Node Representation

```rust
enum NodeKind {
    Normal(FuncId),
    Phi { cond: TaskId, then_branch: TaskId, else_branch: TaskId },
    LoopTemplate { cond: FuncId, body: FuncId, state_var: VarId },
    // ...
}
```

When the runtime dynamically unrolls, `LoopTemplate` will generate a series of `Normal` node
instances.

## Trade-offs

### Advantages

- **Safe integration of dynamic modules**: The metadata contract allows dynamic libraries to
  seamlessly share the concurrency model with the main program while keeping the static DAG
  complete.
- **Progressive FFI integration**: Developers can gradually add annotations to foreign functions,
  transitioning from safe degradation to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds, behavior is
  transparent, and developers can understand optimizations through tools.
- **Natural integration of control flow**: Phi nodes and dynamic unrolling allow the DAG to
  represent all program structures without special syntax.
- **Scalable performance**: Designs such as region allocation and lock-free queues ensure that the
  scheduler can handle large-scale concurrency.

### Disadvantages

- **Increased compile-time complexity from the metadata contract**: Requires generating and parsing
  metadata for dynamic libraries, requiring toolchain support.
- **FFI annotations depend on developer correctness**: Incorrect annotation may lead to data races;
  documentation and tool hints are needed to reduce risk.
- **Context optimization analysis is time-consuming**: Global analysis may increase compile time,
  but can be mitigated by incremental compilation.
- **Dynamic unrolling increases runtime overhead**: Loop unrolling requires dynamic node creation,
  but region allocation can alleviate this.

## Implementation Strategy

### Phased Plan (Including Priority Suggestions)

> **Implementation priority suggestions**: The initial stage does not need to pursue perfect
> implementation; simple solutions can be adopted first to make the system run, then gradually
> optimize. For example:
>
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (e.g., crossbeam's deque).
> - Region allocation can first use a simple bump allocator, and then be optimized later.

#### Phase 1: Basic Support (v0.7)

- [ ] Implement FFI default fallback to `@block`.
- [ ] Add `@pure`, `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (e.g., `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)

- [ ] Design metadata format, modify the compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement main program reading metadata at compile time and creating placeholder nodes.
- [ ] Implement the runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)

- [ ] Implement call graph analysis, calculate node out-degree.
- [ ] Add inlining decision and code generation support.
- [ ] Implement optimization report output (including inlining points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)

- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop template and dynamic unrolling runtime.
- [ ] Improve background scheduling of infinite loops.

#### Phase 5: Performance Optimization (v1.0)

- [ ] Implement region allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmarking and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side-effect handling and concurrency levels, replacing automatic fallback
  with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining the scheduler
  decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic
  unrolling.

## Appendix: Design Decision Records

| Decision                                           | Resolution                                                        | Date       | Recorder |
| -------------------------------------------------- | ----------------------------------------------------------------- | ---------- | -------- |
| Dynamic modules provide a metadata contract        | Adopt metadata file + runtime binding                             | 2026-03-14 | Chenxu   |
| FFI defaults to fallback as `@block`               | Yes, developers can gradually annotate                            | 2026-03-14 | Chenxu   |
| Context optimization replaces static threshold     | Intelligent decisions based on out-degree, side effects, etc.     | 2026-03-14 | Chenxu   |
| Introduce Phi nodes to handle conditional branches | Borrow from SSA, dynamically select branches                      | 2026-03-14 | Chenxu   |
| Dynamic loop unrolling                             | Instantiate iterations on demand, support dependent serialization | 2026-03-14 | Chenxu   |
| Region allocation for short-lifecycle nodes        | Improve memory efficiency and cache locality                      | 2026-03-14 | Chenxu   |

## References

- [RFC-001: spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
- [RFC-021: Library-driven FFI Extension and Cross-language Invocation Support](./021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)
