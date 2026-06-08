```markdown
---
title: "RFC-020: Dynamic Module and FFI Integration"
status: "Deprecated"
author: "晨煦 (Compiled based on community discussions)"
created: "2026-03-14"
updated: "2026-06-05 (Deprecated)"
---

# RFC-020: Dynamic Module and FFI Integration

> **⚠️ Deprecated**: This document has been deprecated, and its content has been merged into [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md).

> **References**:
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)

## Summary

This document further refines and extends YaoXiang's concurrency model based on RFC-001, 008, and 018, addressing practical scenarios such as **dynamic module loading**, **Foreign Function Interface (FFI)**, and **finer-grained scheduling optimization**. The core designs include:

1. **Dynamic module metadata contract**: Provides compile-time dependency descriptions for dynamic libraries written in the same language, enabling the main program to construct a static DAG while maintaining transparent concurrency.
2. **FFI scheduling semantics**: External functions are represented as `@block` nodes in the DAG by default, and can be integrated into parallel scheduling through annotations (see [RFC-021](../review/021-library-driven-ffi-extension.md) for FFI toolchain details).
3. **Context-based optimization**: Instead of static threshold fallback, the compiler intelligently decides whether to inline or schedule as independent nodes based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control flow and DAG merging mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` are naturally integrated into the dataflow graph.
5. **Runtime scheduler memory and performance optimization**: Explicit node lifecycle management, arena allocation, lock-free queues, and other low-cost abstractions.

This document aims to perfect the language specification, ensuring YaoXiang's concurrency model can handle both static whole-program analysis and dynamic/external interactions while maintaining high performance and developer experience.

## Motivation

### Limitations of Existing Design

RFC-001/008/018 established an elegant transparent concurrency model, but there remain blind spots when facing real-world requirements:

- **Dynamic modules**: When a program supports plugins or dynamically linked libraries, the main program cannot know the internal call relationships and dependencies at compile time, leading to global DAG construction failure.
- **FFI calls**: External functions (such as C libraries) are completely a black box; they may contain concurrency, blocking, or side effects internally. Treating them as ordinary nodes would break concurrency safety.
- **Small function scheduling overhead**: RFC-001's "L1 automatic fallback" uses static thresholds (instruction count <50). This implicit rule makes behavior unpredictable for developers, and cannot adapt to complex call contexts.
- **Control flow and DAG fusion**: The representation of dynamic structures like `if` and `loop` in the DAG is not yet clear, potentially affecting dependency analysis accuracy.
- **Runtime overhead control**: As DAG node count increases, scheduler memory management and performance optimization need explicit design to avoid becoming a bottleneck.

### Goals

- While maintaining the core philosophy of transparent concurrency, provide clear, safe, and progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-based intelligent decisions," improving predictability and performance.
- Perfect the DAG's representation of dynamic control flow, ensuring all program structures can naturally integrate into the dataflow model.
- Explicit scheduler memory management and performance optimization strategies, implementing low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled with YaoXiang must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported function list**: Complete type signatures for each function (parameters, return values, resource markers).
- **Side effect markers**: Automatically inferred by the compiler as `@pure` / `@io` (developers can also explicitly override).
- **Resource dependencies**: Whether each parameter is a resource type (such as `File`), and whether the return value contains new resources.
- **Call graph summary** (optional): Other exported functions this function may call, used for cross-module circular dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrow/move), return value ownership.
- **Concurrency safety**: Auto-inferred results satisfying `Send`/`Sync`.

Metadata uses binary or structured text format (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-Time Processing

When the main program compiles and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording input/output dependencies, side effect markers, etc., obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can plan execution order in advance.

#### 1.3 Runtime Binding

When dynamic modules are loaded:

- Runtime verifies that the actual function signatures match the metadata (preventing version mismatches).
- Bind placeholder nodes to actual function pointers.

**Regarding subgraph scheduling semantics**: If a dynamic module has an independent internal subgraph (for example, the module itself contains concurrent logic), that subgraph executes as an **independent scheduling unit**. Its boundary is defined by the module's exported functions: when an exported function is called, the subgraph begins execution as a whole, until that function returns. Node scheduling within the subgraph is handled by the subgraph's own scheduler (the module can continue using the standard scheduler internally), but interaction between the subgraph and the main DAG is limited to input/output dataflow—the placeholder node in the main DAG only cares about the subgraph's start and end, not its internal scheduling. This design ensures module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claims `@pure` but modifies global state), consequences are the developer's responsibility (similar to FFI's unsafe boundary). However, since it's the same language, runtime checks (such as memory isolation) can enhance safety, but add overhead.
- Cross-module circular dependencies: If module A calls B, and B calls A, and the call relationship is declared in the metadata, the compiler can detect and report an error; if not declared, deadlock may occur at runtime, detected by the scheduler and triggering panic.

### 2. FFI Scheduling Semantics in DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion, memory ownership) is defined by [RFC-021](../review/021-library-driven-ffi-extension.md). This section only describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

External functions (declared via `native("symbol")`) are represented as **`@block` nodes** in the DAG by default:

- Do not participate in DAG parallel scheduling; execute synchronously on the current thread.
- The scheduler does not intervene in their internal concurrency during execution.
- Return values are available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can use annotations to integrate FFI calls into DAG scheduling (see [RFC-021 §2.2](../review/021-library-driven-ffi-extension.md) for details):

- `@pure`: Treated as an ordinary DAG node, can run in parallel with other nodes without dependencies.
- `@io`: Participates in resource dependency analysis; multiple calls to the same resource are automatically serialized.

#### 2.3 Impact on Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler, with the only difference being the `effect` marker set to `Block`. When the scheduler encounters a `Block` node, it skips parallel scheduling and executes synchronously.

### 3. Context-Based Call Optimization

Replacing RFC-001's "L1 automatic fallback" static threshold, the compiler now **intelligently decides based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Criteria

The compiler analyzes each function call node:

- **Number of consumers**: How many downstream nodes use the result of this node. If it's 1, it's a candidate for inlining; if greater than 1, it must remain as an independent node for result sharing.
- **Side effects**: If the node has `@io` side effects, it must remain as an independent node to ensure ordering.
- **Computation estimation**: Instruction count and other heuristics may still be referenced, but not as hard thresholds—only used for inline benefit evaluation.
- **Resource dependencies**: If the node involves resource variables (such as `File`), and the resource variable is passed between upstream and downstream, inlining may break the dependency chain and requires caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- Embed the node's computation logic directly into the code of its unique downstream node.
- Remove the node from the DAG; its inputs become direct inputs of the downstream node.
- During final code generation, inlined functions do not produce independent scheduling units.

#### 3.3 Inlining Restrictions

- Recursive functions or calls within loops are typically not inlined to prevent infinite expansion.
- Functions crossing module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use `@noinline` annotation to explicitly prohibit inlining, or `@forceinline` to hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (enabled via `--emit-optimization-report`), containing:

- **Each inlining point**: List of inlined function names, call locations, and reasons for inlining (e.g., "unique consumer and pure function").
- **Reasons for remaining as independent nodes**: e.g., "multiple consumers," "contains side effects," "cross-module call," etc.
- **Decision statistics**: Total inlines, retained node counts, helping developers evaluate optimization effects.

Report output can be text or JSON format for easy tool parsing.

### 4. Control Flow and DAG Merging

#### 4.1 Handling Conditional Branches (if)

Introduce **Phi nodes** (borrowed from SSA form) to represent branch merge points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- Phi node semantics: When the condition variable is ready, select the corresponding branch's output as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it dynamically adds itself to the downstream list of the selected branch and waits for that branch's result.

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

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled at runtime**:

- At compile time, identify the loop body and construct a **loop template**, containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using the accumulated value after the loop ends), the scheduler begins dynamically unrolling iterations:
  1. Schedule the condition node for the first time; if true, instantiate the sub-DAG for the first iteration, with inputs including initial state and external variables.
  2. After iteration completion, produce a new state, and schedule the condition node again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex example: Loop condition depends on updates within the loop body**
```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated within the loop body
}
```

In this pattern, the condition node `x < 10` depends on the updated `x` after each iteration. The DAG representation is as follows:
- The loop template contains the state variable `x`, with initial value 0.
- Each iteration: first execute the condition node (depending on current `x`), if true execute `x = compute(x)` and produce new `x`, then enter the condition node again.
- Dynamically unroll at runtime following the above process until the condition is false.

Dependencies between iterations naturally form dataflow through state variables; iterations with dependencies are automatically serialized, while independent iterations can run in parallel (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously as the main DAG (no scheduling overhead); multiple infinite loops run as background DAGs, with the scheduler time-slicing them for concurrent execution.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable), indicating the number of consumers depending on its result.
- When a node completes execution and passes its result to all downstream nodes, the reference count reaches zero, and the node's memory can be freed.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result is used by only one consumer, ownership can be moved directly, avoiding counting overhead.

#### 5.2 Arena Memory Allocation

For a large number of dynamically generated short-lifecycle nodes (such as loop iterations), use an **arena allocator**:

- Allocate a memory region for one loop unrolling.
- Nodes within the region are allocated contiguously, freed as a whole, reducing fragmentation and deallocation overhead.
- When the region ends, all node memory is reclaimed at once.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize` with `fetch_sub` for atomic decrement.
- Ready queue: Adopt Chase-Lev double-ended queue (per-thread local queue + work stealing), reducing lock contention.
- Downstream list: Created as read-only after creation, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute synchronously with zero scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., adjust worker thread count by monitoring queue length).

#### 5.5 Low-Cost Abstraction Principles

All scheduling overhead is proportional to task count, and the per-task overhead (creation, enqueuing, dependency handling) is controlled at the nanosecond level. For ultra-fine-grained tasks, scheduling is avoided through inlining optimization (see Section 3).

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
    send_sync: SendSync,      // Whether Send/Sync are satisfied
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
    arena_id: Option<ArenaId>,         // Arena to which it belongs (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queues
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Arena allocator
}
```

### 6.3 Context-Based Optimization Analysis

The compiler performs the following steps at the MIR level:

1. Build global call graph and data dependency graph.
2. For each function call node, calculate its out-degree (number of consumers).
3. If out-degree is 1, the function has no side effects (`@pure`), and is not recursive, mark it as "inline candidate."
4. Evaluate inline benefit using heuristics (such as instruction count) to decide whether to inline.
5. When inlining, embed the call node's code into its downstream node, updating dependencies.

### 6.4 Control Flow Node Representation

```rust
enum NodeKind {
    Normal(FuncId),
    Phi { cond: TaskId, then_branch: TaskId, else_branch: TaskId },
    LoopTemplate { cond: FuncId, body: FuncId, state_var: VarId },
    // ...
}
```

During runtime dynamic unrolling, `LoopTemplate` generates a series of `Normal` node instances.

## Tradeoffs

### Advantages

- **Safe dynamic module integration**: The metadata contract enables dynamic libraries to seamlessly share the concurrency model with the main program while maintaining static DAG integrity.
- **Progressive FFI integration**: Developers can progressively add annotations to external functions, transitioning from safe degradation to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds, with transparent behavior; developers can understand optimizations through tools.
- **Natural control flow integration**: Phi nodes and dynamic unrolling enable the DAG to represent all program structures without special syntax.
- **Scalable performance**: Arena allocation, lock-free queues, and other designs ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata contract increases compilation complexity**: Metadata generation and parsing for dynamic libraries is required, and the toolchain needs support.
- **FFI annotation relies on developer correctness**: Incorrect annotations may lead to data races; documentation and tool hints are needed to reduce risk.
- **Context optimization analysis is time-consuming**: Global analysis may increase compilation time, but can be mitigated through incremental compilation.
- **Dynamic unrolling increases runtime overhead**: Loop unrolling requires dynamically creating nodes, but arena allocation can alleviate this.

## Implementation Strategy

### Phase Division (with Priority Recommendations)

> **Implementation priority recommendation**: Perfection is not required initially. Start with simple solutions to get the system running, then optimize progressively. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can initially use a simple bump allocator, optimized later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure` and `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design metadata format, modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement compile-time metadata reading for main program and placeholder node creation.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)
- [ ] Implement call graph analysis and calculate node out-degrees.
- [ ] Add inline decision and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)
- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop template and runtime dynamic unrolling.
- [ ] Complete background scheduling for infinite loops.

#### Phase 5: Performance Optimization (v1.0)
- [ ] Implement arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmarking and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing automatic fallback with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining scheduler decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic unrolling.

## Appendix: Design Decision Log

| Decision | Decision Made | Date | Recorded By |
|----------|---------------|------|-------------|
| Dynamic modules provide metadata contract | Adopt metadata file + runtime binding | 2026-03-14 | 晨煦 |
| FFI defaults to @block | Yes, developers can annotate progressively | 2026-03-14 | 晨煦 |
| Context optimization replaces static threshold | Smart decision based on out-degree, side effects, etc. | 2026-03-14 | 晨煦 |
| Introduce Phi nodes for conditional branches | Borrow from SSA, dynamically select branches | 2026-03-14 | 晨煦 |
| Dynamic loop unrolling | Instance iterations on-demand, support dependency serialization | 2026-03-14 | 晨煦 |
| Arena allocation for short-lifecycle nodes | Improve memory efficiency and cache locality | 2026-03-14 | 晨煦 |

## References

- [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-Ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)
```