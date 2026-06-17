---
title: "RFC-020: Dynamic Modules and FFI Integration"
status: "Deprecated"
author: "Chenxu"
created: "2026-03-14"
updated: "2026-06-05 (Deprecated)"
---

# RFC-020: Dynamic Modules and FFI Integration

> **⚠️ Deprecated**: This document has been deprecated; its content has been merged into [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md).

> **References**:
> - [RFC-001: spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)

## Abstract

This document, building upon RFC-001, 008, and 018, further refines and extends YaoXiang's concurrency model to address practical scenarios such as **dynamic module loading**, **Foreign Function Interface (FFI)**, and **more fine-grained scheduling optimization**. The core designs include:

1. **Dynamic Module Metadata Contract**: Provides compile-time dependency descriptions for dynamic libraries written in the same language, enabling the main program to statically construct a DAG while maintaining transparent concurrency.
2. **FFI Scheduling Semantics**: External functions are treated as `@block` nodes in the DAG by default, and can be integrated into parallel scheduling through annotations (FFI toolchain details in [RFC-021](../review/021-library-driven-ffi-extension.md)).
3. **Call-Context-Based Optimization**: Replacing static threshold fallbacks, the compiler intelligently decides whether to inline or schedule as a separate node based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control Flow and DAG Merging Mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` are naturally integrated into the data flow graph.
5. **Runtime Scheduler Memory and Performance Optimization**: Clearly defines node lifecycle management, arena allocation, lock-free queues, and other low-overhead abstraction implementations.

This document aims to refine the language specification, ensuring that YaoXiang's concurrency model can handle both static whole-program analysis and dynamic/external interaction scenarios flexibly, while maintaining high performance and developer experience.

## Motivation

### Shortcomings of the Current Design

RFC-001/008/018 constructed an elegant transparent concurrency model, but there remain blind spots when facing real-world requirements:

- **Dynamic Modules**: When a program supports plugins or dynamic link libraries, the main program cannot learn the internal call relationships and dependencies of the modules at compile time, causing global DAG construction to fail.
- **FFI Calls**: External functions (such as C libraries) are completely black boxes; they may contain concurrency, blocking, or side effects internally. Treating them as ordinary nodes directly would compromise concurrency safety.
- **Small Function Scheduling Overhead**: The "L1 automatic fallback" proposed in RFC-001 uses static thresholds (instruction count < 50). Such implicit rules make it hard for developers to predict behavior and cannot adapt to complex call contexts.
- **Fusion of Control Flow and DAG**: The representation of dynamic structures like `if` and `loop` in the DAG is not yet clear, which may affect the accuracy of dependency analysis.
- **Runtime Overhead Control**: As the number of DAG nodes grows, the scheduler's memory management and performance optimization need clear designs to avoid becoming a bottleneck.

### Goals

- While preserving the core philosophy of transparent concurrency, provide clear, safe, and progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-based intelligent decisions", improving predictability and performance.
- Refine the DAG's representation of dynamic control flow, ensuring all program structures can be naturally integrated into the data flow model.
- Clarify the scheduler's memory management and performance optimization strategies, achieving low-overhead abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled with YaoXiang must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported Function List**: Complete type signatures of each function (parameters, return value, resource markers).
- **Side Effect Markers**: Compiler-inferred `@pure` / `@io` (developers can also explicitly override).
- **Resource Dependencies**: Whether each parameter is a resource type (e.g., `File`), and whether the return value contains new resources.
- **Call Graph Summary** (optional): IDs of other exported functions this function may call, for cross-module circular dependency detection.
- **Ownership Information**: Ownership semantics of parameters (borrow/move), ownership of return value.
- **Concurrency Safety**: Auto-inferred results satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-Time Processing

When the main program encounters a call to a dynamic module function during compilation:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording the input/output dependencies, side effect markers, etc. obtained from the metadata.
3. The placeholder node participates in dependency analysis like an ordinary node, allowing the scheduler to plan execution order in advance.

#### 1.3 Runtime Binding

When a dynamic module is loaded:

- The runtime verifies that the actual function signature is consistent with the metadata (preventing version mismatch).
- The placeholder node is bound to the actual function pointer.

**Regarding Subgraph Scheduling Semantics**: If a dynamic module has an independent sub-DAG internally (e.g., the module itself contains concurrent logic), that subgraph will execute as an **independent scheduling unit**. Its boundary is defined by the module's exported functions: when an exported function is called, the subgraph starts executing as a whole, until that function returns. The internal node scheduling of the subgraph is handled by the subgraph's own scheduler (the module can continue to use the standard scheduler internally), but the interaction between the subgraph and the main DAG is limited to input/output data flow—the placeholder node in the main DAG only cares about the start and end of the subgraph and does not intervene in its internal scheduling. This design ensures module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claims `@pure` but modifies global state), the consequences are borne by the developer (similar to FFI's unsafe boundary). However, because it is the same language, runtime checks (such as memory isolation) can be used to enhance safety, but this will increase overhead.
- Cross-module circular dependencies: If module A calls B, and B calls A, and the call relationships have been declared in the metadata, the compiler can detect and report an error; if not declared, the runtime may deadlock, which the scheduler will detect and panic.

### 2. FFI Scheduling Semantics in the DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion, memory ownership) is defined in [RFC-021](../review/021-library-driven-ffi-extension.md). This section only describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

External functions (declared via `native("symbol")`) are treated as **`@block` nodes** in the DAG by default:

- Do not participate in DAG parallel scheduling; execute synchronously on the current thread.
- The scheduler does not intervene in internal concurrency during execution.
- The return value is available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can use annotations to integrate FFI calls into DAG scheduling (see [RFC-021 §2.2](../review/021-library-driven-ffi-extension.md) for details):

- `@pure`: Treated as an ordinary DAG node, can run in parallel with other dependency-free nodes.
- `@io`: Participates in resource dependency analysis, automatically serializing multiple calls to the same resource.

#### 2.3 Impact on the Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler; the only difference is that `effect` is marked as `Block`. When the scheduler encounters a `Block` node, it skips parallel scheduling and executes synchronously.

### 3. Call-Context-Based Optimization

Replacing the "L1 automatic fallback" static threshold in RFC-001, the compiler makes **intelligent decisions based on the actual context of each call point in the DAG**.

#### 3.1 Optimization Decision Basis

The compiler analyzes each function call node:

- **Number of Consumers**: How many downstream nodes use the result of this node. If it is 1, it is a candidate for inlining; if greater than 1, it must be kept as a separate node for result sharing.
- **Side Effects**: If the node has `@io` side effects, it must be kept as a separate node to guarantee ordering.
- **Computation Estimation**: Heuristics such as instruction count can still be referenced, but not as a hard threshold, only for inline benefit evaluation.
- **Resource Dependencies**: If the node involves resource variables (e.g., `File`), and the resource variable is passed between upstream and downstream, inlining may break the dependency chain and must be handled carefully.

#### 3.2 Inlining Operation

If the decision is to inline:

- The computation logic of this node is directly embedded into the code of its sole downstream node.
- The node is removed from the DAG, and its inputs directly become inputs of the downstream node.
- During final code generation, the inlined function does not generate an independent scheduling unit.

#### 3.3 Inlining Restrictions

- Recursive functions or calls within loop bodies are typically not inlined, to prevent infinite expansion.
- Functions crossing module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prohibit inlining, or `@forceinline` to hint to the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (can be enabled via `--emit-optimization-report`), containing the following information:

- **Each Inlining Point**: Lists the inlined function name, call location, and reason for inlining (e.g., "single consumer and pure function").
- **Reasons for Keeping as Separate Node**: For example, "has multiple consumers", "contains side effects", "cross-module call", etc.
- **Decision Statistics**: Total inlined count, retained node count, helping developers evaluate optimization effects.

The report output format can be text or JSON, for easy tool parsing.

### 4. Merging Control Flow with DAG

#### 4.1 Handling Conditional Branches (if)

Introduce **Phi nodes** (borrowing from SSA form) to represent branch merge points:

- At compile time, for each `if` expression, construct:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node, whose inputs include the condition variable and the outputs of both branches.
- The semantics of the Phi node: When the condition variable is ready, the output of the corresponding branch is selected as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it dynamically adds itself to the downstream list of the selected branch and waits for the result of that branch.

Example DAG:
```
        cond
       /    \
  then DAG  else DAG
       \    /
        Phi
         |
     Subsequent Node
```

#### 4.2 Handling Loops (loop/while)

Loops are treated as sub-DAGs with feedback edges, and are **unrolled on demand** at runtime:

- At compile time, identify the loop body and construct a **loop template**, including:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when loop results are needed (e.g., when the accumulator value is used after the loop ends), the scheduler begins dynamic unrolling of iterations:
  1. Schedule the condition node for the first time; if true, instantiate the first iteration's sub-DAG, whose inputs include the initial state and external variables.
  2. After the iteration completes, a new state is produced, and the condition node (depending on the new state) is scheduled again to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex Example: Loop condition depends on updates inside the loop body**  
```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```
In this pattern, the condition node `x < 10` depends on the `x` updated after each iteration. The DAG representation is as follows:
- The loop template contains the state variable `x`, with an initial value of 0.
- Each iteration: first execute the condition node (depending on the current `x`); if true, execute `x = compute(x)` and produce a new `x`, then re-enter the condition node.
- At runtime, dynamically unroll according to the above flow until the condition is false.

Dependencies between iterations are naturally formed as data flow through state variables; iterations with dependencies are automatically serialized, while independent iterations can run in parallel (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously directly as the main DAG (zero scheduling overhead); multiple infinite loops execute as background DAGs, concurrently scheduled by the scheduler via time-slicing.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable), representing the number of consumers that depend on its result.
- After a node finishes executing and passes the result to all downstream consumers, the reference count reaches zero, and the node's memory can be released.
- The result value itself also uses reference counting (`Arc<T>`), but can be optimized: if the result is only used by one consumer, ownership is moved directly to avoid counting overhead.

#### 5.2 Arena Memory Allocation

For large numbers of dynamically generated short-lived nodes (such as loop iterations), use an **arena allocator**:

- Allocate a memory arena for one loop unrolling.
- Nodes within the arena are allocated contiguously and released as a whole, reducing fragmentation and release overhead.
- When the arena ends, all node memory is reclaimed at once.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize`, atomically decremented via `fetch_sub`.
- Ready queue: Use Chase-Lev double-ended queues (per-thread local queue + work stealing), reducing lock contention.
- Downstream list: Read-only after creation, avoiding concurrent modification.

#### 5.4 Adaptive Scheduling

- If the system has only one infinite loop, execute synchronously directly with zero scheduling overhead.
- Based on task granularity and system load, dynamically adjust parallelism (e.g., by monitoring queue length to adjust the number of worker threads).

#### 5.5 Low-Overhead Abstraction Principle

All scheduling overhead is proportional to the number of tasks; the per-task overhead (creation, enqueue, dependency handling) is controlled at the tens of nanoseconds level. For ultra-fine-grained tasks, avoid scheduling through inlining optimization (see Section 3).

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
    arena_id: Option<ArenaId>,         // Owning arena (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queue
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Arena allocator
}
```

### 6.3 Context-Based Optimization Analysis

The compiler performs the following steps at the MIR layer:

1. Construct the global call graph and data dependency graph.
2. For each function call node, compute its out-degree (number of consumers).
3. If the out-degree is 1, and the function has no side effects (`@pure`), and is not recursive, mark it as an "inlinable candidate".
4. Combined with heuristics (such as instruction count) to evaluate inlining benefits, decide whether to inline.
5. When inlining, embed the call node's code into its downstream, and update the dependency relationship.

### 6.4 Control Flow Node Representation

```rust
enum NodeKind {
    Normal(FuncId),
    Phi { cond: TaskId, then_branch: TaskId, else_branch: TaskId },
    LoopTemplate { cond: FuncId, body: FuncId, state_var: VarId },
    // ...
}
```

During runtime dynamic unrolling, a `LoopTemplate` will generate a series of `Normal` node instances.

## Trade-offs

### Advantages

- **Safe Integration of Dynamic Modules**: The metadata contract enables dynamic libraries to seamlessly share the concurrency model with the main program, while maintaining the integrity of the static DAG.
- **Progressive FFI Integration**: Developers can gradually add annotations to external functions, transitioning from safe degradation to efficient concurrency.
- **Predictable Optimization**: Context-based decisions replace implicit thresholds, with transparent behavior that developers can understand through tools.
- **Natural Control Flow Integration**: Phi nodes and dynamic unrolling allow the DAG to represent all program structures without special syntax.
- **Scalable Performance**: Designs such as arena allocation and lock-free queues ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata Contract Increases Compilation Complexity**: Requires generating and parsing metadata for dynamic libraries; the toolchain must support this.
- **FFI Annotations Depend on Developer Correctness**: Incorrect annotation may lead to data races; documentation and tooling are needed to mitigate this risk.
- **Context-Based Optimization Analysis Is Time-Consuming**: Global analysis may increase compile time, but this can be mitigated through incremental compilation.
- **Dynamic Unrolling Increases Runtime Overhead**: Loop unrolling requires dynamically creating nodes, but arena allocation can mitigate this.

## Implementation Strategy

### Phased Plan (with Priority Recommendations)

> **Implementation Priority Recommendation**: It's not necessary to pursue perfect implementation at the initial stage; simple solutions can be used first to get the system running, and then optimized gradually. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can first use a simple bump allocator, and be optimized later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement default FFI degradation to `@block`.
- [ ] Add `@pure` and `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design the metadata format, modify the compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement main program compile-time reading of metadata and creation of placeholder nodes.
- [ ] Implement the runtime binding mechanism.

#### Phase 3: Context-Based Optimization (v0.9)
- [ ] Implement call graph analysis, compute node out-degree.
- [ ] Add inlining decision and code generation support.
- [ ] Implement optimization report output (including inlining points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)
- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop templates and dynamic unrolling runtime.
- [ ] Refine infinite loop background scheduling.

#### Phase 5: Performance Optimization (v1.0)
- [ ] Implement the arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmarking and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing automatic fallback with context-based optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining the scheduler decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic unrolling.

## Appendix: Design Decision Records

| Decision | Decision Made | Date | Recorder |
|------|------|------|------|
| Dynamic modules provide metadata contract | Adopt metadata file + runtime binding | 2026-03-14 | Chenxu |
| FFI defaults to degradation as @block | Yes, developers can annotate gradually | 2026-03-14 | Chenxu |
| Context-based optimization replaces static threshold | Intelligent decisions based on out-degree, side effects, etc. | 2026-03-14 | Chenxu |
| Introduce Phi nodes for conditional branches | Borrowed from SSA, dynamic branch selection | 2026-03-14 | Chenxu |
| Dynamic unrolling of loops | Instantiate iterations on demand, supports dependent serialization | 2026-03-14 | Chenxu |
| Arena allocation for short-lived nodes | Improves memory efficiency and cache locality | 2026-03-14 | Chenxu |

## References

- [RFC-001: spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-Ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)