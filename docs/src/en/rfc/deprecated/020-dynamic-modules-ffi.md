---
title: 'RFC-020: Dynamic Modules and FFI Integration'
status: 'Deprecated'
author: 'Chen Xu'
created: '2026-03-14'
updated: '2026-06-05 (Deprecated)'
---

# RFC-020: Dynamic Modules and FFI Integration

> **⚠️ Deprecated**: This document has been deprecated, and its content has been merged into
> [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md).

> **References**:
>
> - [RFC-001: Concurrent Model and Error Handling System](001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
> - [RFC-021: Library-driven FFI Extension and Cross-language Call Support](021-library-driven-ffi-extension.md)

## Summary

This document builds upon RFC-001, 008, and 018 to further refine and extend YaoXiang's concurrency
model, addressing practical scenarios such as **dynamic module loading**, **Foreign Function
Interface (FFI)**, and **more refined scheduling optimization**. The core design includes:

1. **Dynamic module metadata contract**: Provides compile-time dependency descriptions for dynamic
   libraries written in the same language, enabling the main program to statically construct a DAG
   while maintaining transparent concurrency.
2. **FFI scheduling semantics**: External functions default to `@block` nodes in the DAG, and can be
   integrated into parallel scheduling through annotations (FFI toolchain details in
   [RFC-021](021-library-driven-ffi-extension.md)).
3. **Context-based optimization**: Replacing static threshold fallback, the compiler intelligently
   decides whether to inline or schedule as a separate node based on the function's actual role in
   the DAG (number of consumers, side effects, etc.).
4. **Merging control flow with DAG**: Through Phi nodes and dynamic unrolling, dynamic structures
   like `if` and `loop` are naturally integrated into the data flow graph.
5. **Runtime scheduler memory and performance optimization**: Specifies node lifecycle management,
   arena allocation, lock-free queues, and other low-cost abstraction implementations.

This document aims to refine the language specification, ensuring that YaoXiang's concurrency model
can both handle static whole-program analysis and flexibly deal with dynamism and external
interaction, while maintaining high performance and developer experience.

## Motivation

### Limitations of Existing Design

RFC-001/008/018 build an elegant transparent concurrency model, but still have blind spots when
facing real-world requirements:

- **Dynamic modules**: When programs support plugins or dynamically linked libraries, the main
  program cannot obtain the call relationships and dependencies inside the module at compile time,
  resulting in the failure of global DAG construction.
- **FFI calls**: External functions (e.g., C libraries) are completely a black box, potentially
  containing concurrency, blocking, or side effects internally. Treating them as ordinary nodes
  directly would break concurrency safety.
- **Small function scheduling overhead**: The "L1 automatic fallback" proposed in RFC-001 uses a
  static threshold (instruction count <50), and this implicit rule makes it hard for developers to
  predict behavior and cannot adapt to complex call contexts.
- **Fusion of control flow and DAG**: The representation of dynamic structures like `if` and `loop`
  in the DAG is not yet clear, which may affect the accuracy of dependency analysis.
- **Runtime overhead control**: As the number of DAG nodes increases, the scheduler's memory
  management and performance optimization need explicit design to avoid becoming a bottleneck.

### Goals

- Maintain the core philosophy of transparent concurrency while providing clear, safe, and
  progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-based intelligent
  decisions", improving predictability and performance.
- Refine the DAG's representation of dynamic control flow, ensuring all program structures can be
  naturally integrated into the data flow model.
- Specify the scheduler's memory management and performance optimization strategies to achieve
  low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library compiled with YaoXiang (`.yxo` / platform-specific dynamic library) must be
accompanied by a **metadata description file** (`.yxmeta`) containing:

- **Exported function list**: The complete type signature of each function (parameters, return
  values, resource markers).
- **Side effect markers**: Compiler-inferred `@pure` / `@io` (developers can also explicitly
  override).
- **Resource dependencies**: Whether each parameter is a resource type (e.g., `File`), and whether
  the return value contains new resources.
- **Call graph summary** (optional): IDs of other exported functions this function may call, used
  for cross-module circular dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrow/move), ownership of return
  value.
- **Concurrency safety**: Auto-inferred result of satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (e.g., MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-time Processing

When the main program is compiled, encountering calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create **placeholder nodes** in the global DAG, recording the input/output dependencies, side
   effect markers, etc. obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can
   plan execution order in advance.

#### 1.3 Runtime Binding

When a dynamic module is loaded:

- The runtime verifies that the actual function signature matches the metadata (preventing version
  mismatches).
- Binds the placeholder node to the actual function pointer.

**Regarding subgraph scheduling semantics**: If the dynamic module has its own independent sub-DAG
(e.g., the module itself contains concurrent logic), this subgraph will execute as an **independent
scheduling unit**. Its boundaries are defined by the module's exported functions: when an exported
function is called, the subgraph starts executing as a whole until the function returns. The
scheduling of nodes inside the subgraph is handled by the subgraph's own scheduler (the module can
continue to use the standard scheduler internally), but the interaction between the subgraph and the
main DAG is limited to input/output data flow—the placeholder node in the main DAG only cares about
the start and end of the subgraph and does not intervene in its internal scheduling. This design
ensures module encapsulation while maintaining the static completeness of the main DAG.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claims `@pure` but modifies global state), the
  consequences are borne by the developer (similar to FFI's unsafe boundary). However, since it's
  the same language, runtime checks (e.g., memory isolation) can enhance security, but at the cost
  of increased overhead.
- Cross-module circular dependencies: If module A calls B, and B calls A, and the call relationships
  have been declared in the metadata, the compiler can detect and report an error; if not declared,
  deadlocks may occur at runtime, which the scheduler detects and panics on.

### 2. FFI Scheduling Semantics in DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion,
memory ownership) is defined by [RFC-021](021-library-driven-ffi-extension.md). This section only
describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

External functions (declared via `native("symbol")`) default to **`@block` nodes** in the DAG:

- Do not participate in DAG parallel scheduling, executed synchronously directly on the current
  thread.
- The scheduler does not intervene in internal concurrency during execution.
- The return value is available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can use annotations to make FFI calls participate in DAG scheduling (see
[RFC-021 §2.2](021-library-driven-ffi-extension.md)):

- `@pure`: Treated as a regular DAG node, can run in parallel with other nodes without dependencies.
- `@io`: Participates in resource dependency analysis, automatically serializing multiple calls to
  the same resource.

#### 2.3 Impact on the Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler, the only difference
being that `effect` is marked as `Block`. The scheduler skips parallel scheduling when encountering
a `Block` node and executes it synchronously directly.

### 3. Context-based Optimization

Replacing the "L1 automatic fallback" static threshold in RFC-001 with **the compiler making
intelligent decisions based on each call site's actual context in the DAG**.

#### 3.1 Decision Basis for Optimization

The compiler analyzes each function call node:

- **Number of consumers**: How many downstream nodes use the result of this node. If it is 1, it
  qualifies as an inline candidate; if greater than 1, it must remain as a separate node for result
  sharing.
- **Side effects**: If the node has `@io` side effects, it must remain as a separate node to ensure
  ordering.
- **Computation estimate**: Heuristics like instruction count can still be referenced, but not as a
  hard threshold, only for inline benefit assessment.
- **Resource dependencies**: If the node involves resource variables (e.g., `File`), and the
  resource variable is passed between upstream and downstream, inlining may break the dependency
  chain, requiring caution.

#### 3.2 Inline Operation

If the decision is to inline:

- Embed the node's computation logic directly into the code of its only downstream node.
- Remove the node from the DAG, its inputs become the inputs of the downstream node directly.
- In final code generation, inlined functions do not generate independent scheduling units.

#### 3.3 Inline Limitations

- Recursive functions or calls within loop bodies are usually not inlined, to prevent infinite
  expansion.
- Functions across module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prohibit inlining, or `@forceinline` to
  hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate optimization reports (can be enabled via `--emit-optimization-report`),
containing the following information:

- **Each inline point**: List the inlined function name, call location, and inline reason (e.g.,
  "sole consumer and pure function").
- **Reasons for remaining as separate nodes**: e.g., "multiple consumers", "contains side effects",
  "cross-module call", etc.
- **Decision statistics**: Total number of inlines, number of remaining nodes, helping developers
  evaluate optimization effects.

The report output format can be text or JSON, for easy tool parsing.

### 4. Merging Control Flow with DAG

#### 4.1 Handling Conditional Branches (if)

Introduce **Phi nodes** (borrowing from SSA form) to represent branch merge points:

- At compile time, build for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node, whose inputs include the condition variable and the outputs of both branches.
- Phi node semantics: When the condition variable is ready, the output of the corresponding branch
  is selected as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it
  dynamically adds itself to the downstream list of the selected branch and waits for that branch's
  result.

Example DAG:

```
        cond
       /    \
  then DAG  else DAG
       \    /
        Phi
         |
      subsequent nodes
```

#### 4.2 Handling Loops (loop/while)

Loops are treated as sub-DAGs with feedback edges, **unrolled on demand** at runtime:

- At compile time, identify the loop body and build a **loop template** containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using the accumulated value after the loop ends),
  the scheduler starts dynamically unrolling iterations:
  1. First, schedule the condition node. If true, instantiate the first iteration's sub-DAG, with
     inputs including the initial state and external variables.
  2. After the iteration completes, a new state is produced, and the condition node is scheduled
     again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the last iteration's output is the loop result.

**Complex example: Loop condition depends on updates inside the loop body**

```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```

In this pattern, the condition node `x < 10` depends on `x` updated after each iteration. The DAG
representation is as follows:

- The loop template contains the state variable `x`, with an initial value of 0.
- Each iteration: first execute the condition node (depending on the current `x`), if true, execute
  `x = compute(x)` and produce a new `x`, then re-enter the condition node.
- At runtime, dynamically unroll according to the above flow until the condition is false.

Dependencies between iterations are naturally formed through state variables as data flow;
iterations with dependencies are automatically serialized, while iterations without dependencies can
run in parallel (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop is executed synchronously directly as the main DAG (no scheduling overhead);
multiple infinite loops are executed as background DAGs, concurrently executed via scheduler time
slicing.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable), representing the number of consumers
  that depend on its result.
- After a node completes execution and passes the result to all downstream consumers, the reference
  count reaches zero, and the node's memory can be released.
- The result value itself also uses reference counting (`Arc<T>`), but can be optimized: if the
  result is used by only one consumer, ownership is moved directly to avoid counting overhead.

#### 5.2 Arena Memory Allocation

For large numbers of dynamically generated short-lived nodes (e.g., loop iterations), use an **arena
allocator**:

- Allocate a memory arena for one loop unrolling.
- Nodes within the arena are allocated contiguously, released in bulk, reducing fragmentation and
  release overhead.
- When the arena ends, all node memory is reclaimed at once.

#### 5.3 Lock-free Data Structures

- Dependency counter: Use `AtomicUsize`, atomically decremented via `fetch_sub`.
- Ready queue: Use Chase-Lev double-ended queue (per-thread local queue + work stealing), reducing
  lock contention.
- Downstream list: Read-only after creation, avoiding concurrent modification.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute it synchronously directly, with zero
  scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., adjust the number
  of worker threads by monitoring queue length).

#### 5.5 Low-cost Abstraction Principle

All scheduling overhead is proportional to the number of tasks; the additional overhead per task
(creation, enqueue, dependency handling) is controlled at the tens of nanoseconds level. For
extremely fine-grained tasks, avoid scheduling through inlining optimization (see Section 3).

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
    resource_params: Vec<usize>, // Parameter index list, indicating which parameters are resource types
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
    arena_id: Option<ArenaId>,         // Owning arena (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queue
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Arena allocator
}
```

### 6.3 Context-based Optimization Analysis

The compiler performs the following steps at the MIR layer:

1. Build a global call graph and data dependency graph.
2. For each function call node, calculate its out-degree (number of consumers).
3. If out-degree is 1, and the function has no side effects (`@pure`), and is non-recursive, mark it
   as an "inlinable candidate".
4. Combine heuristics (e.g., instruction count) to evaluate inline benefits, deciding whether to
   inline.
5. When inlining, embed the call node's code into its downstream, updating dependency relationships.

### 6.4 Control Flow Node Representation

```rust
enum NodeKind {
    Normal(FuncId),
    Phi { cond: TaskId, then_branch: TaskId, else_branch: TaskId },
    LoopTemplate { cond: FuncId, body: FuncId, state_var: VarId },
    // ...
}
```

When dynamically unrolled at runtime, `LoopTemplate` generates a series of `Normal` node instances.

## Trade-offs

### Advantages

- **Safe integration of dynamic modules**: The metadata contract allows dynamic libraries to
  seamlessly share the concurrency model with the main program while maintaining the integrity of
  the static DAG.
- **Progressive FFI integration**: Developers can gradually add annotations to external functions,
  transitioning from safe degradation to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds, behavior is
  transparent, and developers can understand optimizations through tools.
- **Natural integration of control flow**: Phi nodes and dynamic unrolling allow the DAG to
  represent all program structures without special syntax.
- **Performance scalability**: Designs such as arena allocation and lock-free queues ensure the
  scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata contract increases compile complexity**: Need to generate and parse metadata for
  dynamic libraries, toolchain support required.
- **FFI annotation correctness depends on developer**: Incorrect annotations may lead to data races,
  which need to be mitigated through documentation and tool hints.
- **Context optimization analysis is time-consuming**: Global analysis may increase compile time,
  but can be mitigated through incremental compilation.
- **Dynamic unrolling increases runtime overhead**: Loop unrolling requires dynamically creating
  nodes, but arena allocation can mitigate this.

## Implementation Strategy

### Phase Division (with Priority Suggestions)

> **Implementation priority suggestion**: It is not necessary to pursue perfect implementation in
> the early stage. Simple solutions can be adopted first to make the system run, and then gradually
> optimized. For example:
>
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (e.g., crossbeam's deque).
> - Arena allocation can first use a simple bump allocator, with further optimization later.

#### Phase 1: Basic Support (v0.7)

- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure`, `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (e.g., `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)

- [ ] Design metadata format, modify the compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement the main program reading metadata at compile time and creating placeholder nodes.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)

- [ ] Implement call graph analysis, calculate node out-degree.
- [ ] Add inline decision and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)

- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop templates and dynamic unrolling runtime.
- [ ] Refine background scheduling of infinite loops.

#### Phase 5: Performance Optimization (v1.0)

- [ ] Implement arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmarking and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing automatic fallback
  with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining scheduler
  decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic
  unrolling.

## Appendix: Design Decision Records

| Topic                                              | Decision                                                           | Date       | Recorder |
| -------------------------------------------------- | ------------------------------------------------------------------ | ---------- | -------- |
| Dynamic modules provide metadata contract          | Adopt metadata file + runtime binding                              | 2026-03-14 | Chen Xu  |
| FFI default degradation to @block                  | Yes, developers can annotate progressively                         | 2026-03-14 | Chen Xu  |
| Context optimization replaces static threshold     | Intelligent decision based on out-degree, side effects, etc.       | 2026-03-14 | Chen Xu  |
| Introduce Phi nodes to handle conditional branches | Borrow from SSA, dynamic branch selection                          | 2026-03-14 | Chen Xu  |
| Loop dynamic unrolling                             | Instantiate iterations on demand, support dependency serialization | 2026-03-14 | Chen Xu  |
| Arena allocation for short-lived nodes             | Improve memory efficiency and cache locality                       | 2026-03-14 | Chen Xu  |

## References

- [RFC-001: Concurrent Model and Error Handling System](001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
- [RFC-021: Library-driven FFI Extension and Cross-language Call Support](021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)
