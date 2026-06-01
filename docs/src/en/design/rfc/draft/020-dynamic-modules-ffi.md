---
title: "RFC-020: Dynamic Modules and FFI Integration"
---

# RFC-020: Dynamic Modules and FFI Integration

> **Status**: Draft
> **Author**: Chen Xu (organized based on community discussions)
> **Created**: 2026-03-14
> **Last Updated**: 2026-03-14

> **References**:
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)

## Summary

Building upon RFC-001, 008, and 018, this document further refines and extends YaoXiang's concurrent model to address practical scenarios such as **dynamic module loading**, **Foreign Function Interface (FFI)**, and **finer-grained scheduling optimization**. The core design includes:

1. **Dynamic Module Metadata Contract**: Provides compile-time dependency descriptions for dynamic libraries written in the same language, enabling the main program to statically construct a DAG while maintaining transparent concurrency.
2. **FFI Scheduling Semantics**: External functions are treated as `@block` nodes in the DAG by default, but can be integrated into parallel scheduling through annotations (see [RFC-021](../review/021-library-driven-ffi-extension.md) for FFI toolchain details).
3. **Call Context-Based Optimization**: Instead of static threshold fallback, the compiler intelligently decides whether to inline or schedule as independent nodes based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control Flow and DAG Merging Mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` naturally integrate into the data flow graph.
5. **Runtime Scheduler Memory and Performance Optimization**: Explicit node lifecycle management, arena allocation, lock-free queues, and other low-cost abstraction implementations.

This document aims to perfect the language specification, ensuring YaoXiang's concurrent model can handle both static whole-program analysis and flexible dynamic/external interactions, while maintaining high performance and developer experience.

## Motivation

### Limitations of Existing Design

RFC-001/008/018 established an elegant transparent concurrency model, but there remain blind spots when facing real-world requirements:

- **Dynamic modules**: When a program supports plugins or dynamically linked libraries, the main program cannot know the internal call relationships and dependencies of modules at compile time, leading to failure in global DAG construction.
- **FFI calls**: External functions (such as C libraries) are completely a black box—they may contain concurrency, blocking, or side effects internally. Treating them as ordinary nodes would break concurrency safety.
- **Small function scheduling overhead**: The "L1 automatic fallback" proposed in RFC-001 uses static thresholds (instruction count <50). This implicit rule makes behavior unpredictable for developers, and cannot adapt to complex call contexts.
- **Control flow and DAG fusion**: The representation of dynamic structures like `if` and `loop` in the DAG is still unclear, potentially affecting the accuracy of dependency analysis.
- **Runtime overhead control**: As the number of DAG nodes increases, scheduler memory management and performance optimization need explicit design to avoid becoming a bottleneck.

### Goals

- While maintaining the core philosophy of transparent concurrency, provide clear, safe, and gradual support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-aware intelligent decisions," improving predictability and performance.
- Perfect DAG representation for dynamic control flow, ensuring all program structures can naturally integrate into the data flow model.
- Explicit scheduler memory management and performance optimization strategies, implementing low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled with YaoXiang must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported function list**: Complete type signatures for each function (parameters, return value, resource markers).
- **Side effect markers**: `@pure` / `@io` automatically inferred by the compiler (developers can also explicitly override).
- **Resource dependencies**: Whether each parameter is a resource type (such as `File`), and whether the return value contains new resources.
- **Call graph summary** (optional): Other exported functions this function may call, used for cross-module cyclic dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrow/move), return value ownership.
- **Concurrency safety**: Automatically inferred results for satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-Time Processing

When the main program compiles and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording input/output dependencies, side effect markers, etc., obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can plan execution order in advance.

#### 1.3 Runtime Binding

When dynamic modules are loaded:

- The runtime verifies that the actual function signature matches the metadata (preventing version mismatch).
- The placeholder node is bound to the actual function pointer.

**Regarding subgraph scheduling semantics**: If the dynamic module has an independent internal subgraph (for example, the module itself contains concurrent logic), that subgraph executes as an **independent scheduling unit**. Its boundaries are defined by the module's exported functions: when an exported function is called, the subgraph begins execution as a whole and continues until that function returns. Node scheduling inside the subgraph is handled by the subgraph's own scheduler (modules can continue using the standard scheduler internally), but the subgraph's interaction with the main DAG is limited to input/output data flow—the placeholder node in the main DAG only cares about the subgraph's start and end, without intervening in its internal scheduling. This design guarantees module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claims `@pure` but modifies global state), the consequences are borne by the developer (similar to FFI's unsafe boundary). However, since it's the same language, runtime checks (such as memory isolation) can enhance safety but add overhead.
- Cross-module cyclic dependencies: If module A calls B, and B calls A, and the call relationship is declared in the metadata, the compiler can detect and report an error; if undeclared, deadlock may occur at runtime, detected by the scheduler and resulting in a panic.

### 2. FFI Scheduling Semantics in DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion, memory ownership) is defined by [RFC-021](../review/021-library-driven-ffi-extension.md). This section only describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

External functions (declared via `native("symbol")`) are treated as **`@block` nodes** in the DAG by default:

- They do not participate in DAG parallel scheduling, executing synchronously on the current thread.
- The scheduler does not intervene in their internal concurrency during execution.
- Return values are available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can integrate FFI calls into DAG scheduling through annotations (see [RFC-021 §2.2](../review/021-library-driven-ffi-extension.md) for details):

- `@pure`: Treated as an ordinary DAG node, can run in parallel with other independent nodes.
- `@io`: Participates in resource dependency analysis; multiple calls to the same resource are automatically serialized.

#### 2.3 Impact on Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler, differing only in the `effect` flag set to `Block`. When the scheduler encounters a `Block` node, it skips parallel scheduling and executes synchronously.

### 3. Call Context-Based Optimization

Replacing the "L1 automatic fallback" static threshold from RFC-001, this approach uses **intelligent compiler decisions based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Criteria

The compiler analyzes each function call node:

- **Number of consumers**: How many downstream nodes use this node's result. If it is 1, it qualifies as an inline candidate; if greater than 1, it must remain as an independent node to enable result sharing.
- **Side effects**: If the node has `@io` side effects, it must remain as an independent node to guarantee ordering.
- **Computation estimation**: Instruction count and other heuristics may still be referenced, but not as hard thresholds—only used for inline benefit assessment.
- **Resource dependencies**: If the node involves resource variables (such as `File`), and the resource variable is passed between upstream and downstream, inlining may break the dependency chain and requires caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- The node's computation logic is directly embedded into its single downstream node's code.
- The node is removed from the DAG, and its inputs become the downstream node's inputs directly.
- During final code generation, inlined functions do not produce independent scheduling units.

#### 3.3 Inlining Restrictions

- Calls within recursive functions or loop bodies are typically not inlined to prevent infinite expansion.
- Functions crossing module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prevent inlining, or `@forceinline` to suggest the compiler attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (enabled via `--emit-optimization-report`), containing:

- **Each inline point**: Lists inlined function names, call locations, and reasons for inlining (e.g., "single consumer and pure function").
- **Reasons for remaining as independent nodes**: e.g., "has multiple consumers," "contains side effects," "cross-module call," etc.
- **Decision statistics**: Total inlines, retained node counts, helping developers evaluate optimization effectiveness.

The report output format can be text or JSON for easy tool parsing.

### 4. Control Flow and DAG Merging

#### 4.1 Handling Conditional Branches (if)

Introduce **Phi nodes** (borrowing from SSA form) to represent branch convergence points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- The Phi node's semantics: when the condition variable is ready, it selects the corresponding branch's output as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it dynamically adds itself to the selected branch's downstream list and waits for that branch's result.

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

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled at runtime on demand**:

- At compile time, identify the loop body and construct a **loop template**, containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using an accumulated value after the loop ends), the scheduler begins dynamically unrolling iterations:
  1. Schedule the condition node for the first time; if true, instantiate the sub-DAG for the first iteration, with inputs including initial state and external variables.
  2. After the iteration completes and produces new state, schedule the condition node again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex example: Loop condition depends on updates inside the loop body**
```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```
In this pattern, the condition node `x < 10` depends on `x` updated after each iteration. The DAG representation is as follows:
- The loop template contains the state variable `x`, initially with value 0.
- Each iteration: first execute the condition node (depending on current `x`); if true, execute `x = compute(x)` and produce new `x`, then enter the condition node again.
- The runtime dynamically unrolls following the above process until the condition is false.

Dependencies between iterations naturally form data flow through state variables—iterations with dependencies are automatically serialized, while independent iterations can run in parallel (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously as part of the main DAG (zero scheduling overhead); multiple infinite loops execute as background DAGs, with the scheduler slicing time for concurrent execution.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable) representing the number of consumers depending on its result.
- After a node completes execution and passes results to all downstream nodes, its reference count reaches zero, and the node's memory can be freed.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result is used by only one consumer, ownership can be moved directly, avoiding counting overhead.

#### 5.2 Arena Memory Allocation

For dynamically generated large numbers of short-lifecycle nodes (such as loop iterations), use an **arena allocator**:

- Allocate a memory arena for each loop unrolling.
- Nodes within the arena are allocated contiguously, released as a whole, reducing fragmentation and deallocation overhead.
- When the arena ends, all node memory is reclaimed in one pass.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize` with `fetch_sub` atomic decrement.
- Ready queue: Adopt Chase-Lev deque (per-thread local queue + work stealing), reducing lock contention.
- Downstream list: Created read-only after initialization, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute directly and synchronously with zero scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., monitor queue length to adjust the number of worker threads).

#### 5.5 Low-Cost Abstraction Principles

All scheduling overhead is proportional to the number of tasks; additional overhead per task (creation, enqueueing, dependency handling) is controlled at the nanosecond level. For ultra-fine-grained tasks, avoid scheduling through inlining optimization (see Section 3).

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
    resource_params: Vec<usize>, // List of parameter indices indicating which are resource types
    calls: Vec<String>,       // Names of other exported functions called (optional)
    ownership: OwnershipInfo,
    send_sync: SendSync,      // Whether it satisfies Send/Sync
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
    arena_id: Option<ArenaId>,         // Arena it belongs to (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queues
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Arena allocator
}
```

### 6.3 Context-Based Optimization Analysis

The compiler performs the following steps at the MIR level:

1. Construct the global call graph and data dependency graph.
2. For each function call node, calculate its out-degree (number of consumers).
3. If out-degree is 1, the function has no side effects (`@pure`), and is not recursive, mark it as "inline candidate."
4. Evaluate inline benefits combining heuristics (such as instruction count) to decide whether to inline.
5. During inlining, embed the call node's code into its downstream node and update dependency relationships.

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
- **Gradual FFI integration**: Developers can progressively add annotations to external functions, transitioning from safe degradation to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds, behavior is transparent, and developers can understand optimizations through tools.
- **Natural control flow integration**: Phi nodes and dynamic unrolling enable the DAG to represent all program structures without special syntax.
- **Scalable performance**: Arena allocation, lock-free queues, and other designs ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata contract increases compilation complexity**: Dynamic libraries need to generate and parse metadata, requiring toolchain support.
- **FFI annotation relies on developer correctness**: Incorrect annotations may cause data races; documentation and tooling hints can mitigate this risk.
- **Context optimization analysis is time-consuming**: Global analysis may increase compilation time, but can be alleviated through incremental compilation.
- **Dynamic unrolling increases runtime overhead**: Loop unrolling dynamically creates nodes, but arena allocation can alleviate this.

## Implementation Strategy

### Phase Breakdown (with Priority Recommendations)

> **Implementation priority recommendation**: Initial implementation need not pursue perfection; a simple approach can get the system running first, then optimize gradually. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can use a simple bump allocator first, optimizing later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure` and `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design metadata format, modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement main program compile-time metadata reading and placeholder node creation.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)
- [ ] Implement call graph analysis and calculate node out-degrees.
- [ ] Add inline decision-making and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)
- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop templates and runtime dynamic unrolling.
- [ ] Complete background scheduling for infinite loops.

#### Phase 5: Performance Optimization (v1.0)
- [ ] Implement arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmark and tune.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing automatic fallback with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining scheduler decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic unrolling.

## Appendix: Design Decision Record

| Decision | Resolution | Date | Recorded By |
|------|------|------|--------|
| Dynamic modules provide metadata contracts | Adopt metadata file + runtime binding | 2026-03-14 | Chen Xu |
| FFI defaults to @block | Yes, developers can add annotations gradually | 2026-03-14 | Chen Xu |
| Context optimization replaces static thresholds | Intelligent decisions based on out-degree, side effects, etc. | 2026-03-14 | Chen Xu |
| Introduce Phi nodes for conditional branches | Borrow from SSA, dynamically select branches | 2026-03-14 | Chen Xu |
| Dynamic loop unrolling | Instantiate iterations on demand, support dependency serialization | 2026-03-14 | Chen Xu |
| Arena allocation for short-lifecycle nodes | Improve memory efficiency and cache locality | 2026-03-14 | Chen Xu |

## References

- [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](../review/021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Deque](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)