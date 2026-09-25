---
title: 'RFC-020: Dynamic Modules and FFI Integration'
status: 'Deprecated'
author: 'Chen Xu'
created: '2026-03-14'
updated: '2026-06-05 (Deprecated)'
---

# RFC-020: Dynamic Modules and FFI Integration

> **⚠️ Deprecated**: This document has been deprecated and its content has been merged into
> [RFC-026: FFI Core Mechanism](../accepted/026-ffi-core-mechanism.md).

> **References**:
>
> - [RFC-001: Spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
> - [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./021-library-driven-ffi-extension.md)

## Abstract

Based on RFC-001, 008, and 018, this document further refines and extends YaoXiang's concurrency
model to address **dynamic module loading**, **Foreign Function Interface (FFI)**, and
**finer-grained scheduling optimization** in real-world scenarios. The core designs include:

1. **Dynamic Module Metadata Contract**: Provides compile-time dependency descriptions for dynamic
   libraries written in the same language, enabling the main program to statically construct a DAG
   while maintaining transparent concurrency.
2. **FFI Scheduling Semantics**: External functions are treated as `@block` nodes in the DAG by
   default, and can be integrated into parallel scheduling through annotations (FFI toolchain
   details in [RFC-021](./021-library-driven-ffi-extension.md)).
3. **Call Context-Based Optimization**: Instead of static threshold fallback, the compiler
   intelligently decides whether to inline or schedule as independent nodes based on the function's
   actual role in the DAG (number of consumers, side effects, etc.).
4. **Control Flow and DAG Merge Mechanism**: Through Phi nodes and dynamic unrolling, dynamic
   structures like `if` and `loop` are naturally integrated into the dataflow graph.
5. **Runtime Scheduler Memory and Performance Optimization**: Explicit node lifecycle management,
   arena allocation, lock-free queues, and other low-cost abstractions.

This document aims to perfect the language specification, ensuring YaoXiang's concurrency model can
handle both static whole-program analysis and flexible dynamic and external interactions, while
maintaining high performance and developer experience.

## Motivation

### Limitations of the Current Design

RFC-001/008/018 built an elegant transparent concurrency model, but still have blind spots when
facing real-world requirements:

- **Dynamic Modules**: When a program supports plugins or dynamically linked libraries, the main
  program cannot know the internal call relationships and dependencies of modules at compile time,
  leading to global DAG construction failure.
- **FFI Calls**: External functions (such as C libraries) are completely a black box, potentially
  containing concurrency, blocking, or side effects. Treating them as ordinary nodes would break
  concurrency safety.
- **Small Function Scheduling Overhead**: The "L1 Auto Fallback" in RFC-001 uses static thresholds
  (instruction count < 50), making this implicit rule difficult for developers to predict behavior,
  and it cannot adapt to complex call contexts.
- **Control Flow and DAG Fusion**: The representation of dynamic structures like `if` and `loop` in
  the DAG is not yet clear, potentially affecting dependency analysis accuracy.
- **Runtime Overhead Control**: As DAG node count increases, scheduler memory management and
  performance optimization require explicit design to avoid becoming bottlenecks.

### Goals

- Maintain the core philosophy of transparent concurrency while providing clear, safe, and
  progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-based intelligent
  decisions" to improve predictability and performance.
- Perfect DAG representation of dynamic control flow, ensuring all program structures can naturally
  integrate into the dataflow model.
- Explicit scheduler memory management and performance optimization strategies to achieve low-cost
  abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled with YaoXiang must be
accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported Function List**: Complete type signatures for each function (parameters, return values,
  resource markers).
- **Side Effect Markers**: Compiler automatically infers `@pure` / `@io` (developers can also
  explicitly override).
- **Resource Dependencies**: Whether each parameter is a resource type (such as `File`), and whether
  the return value contains new resources.
- **Call Graph Summary** (optional): Other exported function IDs that this function may call, used
  for cross-module circular dependency detection.
- **Ownership Information**: Ownership semantics of parameters (borrowed/moved), return value
  ownership.
- **Concurrency Safety**: Automatically inferred results satisfying `Send`/`Sync`.

Metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-time Processing

When the main program is compiled and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create **placeholder nodes** in the global DAG, recording input-output dependencies, side effect
   markers, etc. obtained from metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can
   plan execution order in advance.

#### 1.3 Runtime Binding

When the dynamic module is loaded:

- Runtime verifies that actual function signatures match metadata (preventing version mismatch).
- Bind placeholder nodes to actual function pointers.

**Regarding Subgraph Scheduling Semantics**: If a dynamic module has an independent internal
subgraph (for example, the module itself contains concurrent logic), that subgraph will execute as
an **independent scheduling unit**. Its boundaries are defined by the module's exported functions:
when an exported function is called, the subgraph begins execution as a whole and continues until
that function returns. Node scheduling inside the subgraph is managed by the subgraph's own
scheduler (modules can continue using the standard scheduler internally), but the subgraph's
interaction with the main DAG is limited to input-output dataflow—the placeholder node in the main
DAG only cares about the subgraph's start and end, without intervening in its internal scheduling.
This design guarantees module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (such as claiming `@pure` but modifying global state),
  the consequences are borne by the developer (similar to FFI's unsafe boundary). But since it's the
  same language, runtime checks (such as memory isolation) can enhance safety but add overhead.
- Cross-module circular dependencies: If module A calls B, and B calls A, and the call relationship
  is declared in metadata, the compiler can detect and report an error; if not declared, deadlock
  may occur at runtime, detected and panicked by the scheduler.

### 2. FFI Scheduling Semantics in the DAG

The complete FFI toolchain support (dynamic library loading, binding generation, type conversion,
memory ownership) is defined by [RFC-021](./021-library-driven-ffi-extension.md). This section only
describes the behavior of FFI calls in DAG scheduling.

#### 2.1 Default Scheduling Behavior

External functions (declared via `native("symbol")`) are treated as **`@block` nodes** in the DAG by
default:

- Do not participate in DAG parallel scheduling, execute synchronously on the current thread.
- The scheduler does not intervene in its internal concurrency during execution.
- The return value is available, but the call itself does not generate dependency edges.

#### 2.2 Optional Concurrency Annotations

Developers can use annotations to integrate FFI calls into DAG scheduling (see
[RFC-021 §2.2](./021-library-driven-ffi-extension.md)):

- `@pure`: Treated as an ordinary DAG node, can run in parallel with other nodes without
  dependencies.
- `@io`: Participates in resource dependency analysis; multiple calls to the same resource are
  automatically serialized.

#### 2.3 Impact on the Scheduler

FFI nodes use the same `TaskNode` structure as ordinary nodes in the scheduler, differing only in
that the `effect` marker is set to `Block`. When the scheduler encounters a `Block` node, it skips
parallel scheduling and executes synchronously.

### 3. Call Context-Based Optimization

Replacing the "L1 Auto Fallback" static threshold in RFC-001, this instead uses **intelligent
decisions made by the compiler based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Basis

The compiler analyzes each function call node:

- **Number of Consumers**: How many downstream nodes use this node's result. If it's 1, it qualifies
  as an inline candidate; if greater than 1, it must be kept as an independent node for result
  sharing.
- **Side Effects**: If the node has `@io` side effects, it must be kept as an independent node to
  guarantee order.
- **Computation Estimation**: Instruction count and other heuristics can still be referenced, but
  not as hard thresholds—only used for inline benefit evaluation.
- **Resource Dependencies**: If the node involves resource variables (such as `File`), and the
  resource variable is passed between upstream and downstream, inlining may break the dependency
  chain and requires caution.

#### 3.2 Inline Operation

If the decision is to inline:

- Embed the node's computation logic directly into its single downstream node's code.
- Remove the node from the DAG; its inputs become the downstream node's inputs directly.
- During final code generation, inlined functions do not generate independent scheduling units.

#### 3.3 Inline Restrictions

- Calls inside recursive functions or loop bodies are usually not inlined to prevent infinite
  expansion.
- Functions crossing module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prohibit inlining, or `@forceinline` to
  hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (can be enabled via
`--emit-optimization-report`), containing:

- **Each Inline Point**: Lists inlined function names, call locations, and inline reasons (e.g.,
  "unique consumer and pure function").
- **Reasons for Keeping as Independent Node**: E.g., "has multiple consumers", "contains side
  effects", "cross-module call", etc.
- **Decision Statistics**: Total inlines, retained node count, helping developers evaluate
  optimization effects.

The report output format can be text or JSON, for easy tool parsing.

### 4. Control Flow and DAG Merge

#### 4.1 Handling Conditional Branches (`if`)

Introduce **Phi nodes** (borrowed from SSA form) to represent branch convergence points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- Phi node semantics: When the condition variable is ready, select the corresponding branch's output
  as its own output based on the condition value.
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

#### 4.2 Handling Loops (`loop`/`while`)

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled at runtime as needed**:

- At compile time, identify the loop body and construct a **loop template**, containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using the accumulated value after the loop ends),
  the scheduler begins dynamically unrolling iterations:
  1. Schedule the condition node for the first time; if true, instantiate the sub-DAG for the first
     iteration, with inputs including initial state and external variables.
  2. After iteration completion produces new state, schedule the condition node again (depending on
     the new state) to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex Example: Loop Condition Depends on Update Inside Loop Body**

```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```

In this pattern, the condition node `x < 10` depends on `x` updated after each iteration. The DAG
representation is as follows:

- The loop template contains the state variable `x` with initial value 0.
- Each iteration: First execute the condition node (depending on current `x`); if true, execute
  `x = compute(x)` and produce new `x`, then enter the condition node again.
- Dynamically unrolled at runtime following the above process until the condition is false.

Dependencies between iterations naturally form dataflow through state variables; iterations with
dependencies are automatically serialized, while iterations without dependencies can run in parallel
(e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously as the main DAG (no scheduling overhead); multiple
infinite loops execute as background DAGs with time-slice concurrent execution by the scheduler.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable) representing the number of consumers
  depending on its result.
- After a node completes execution and passes results to all downstream nodes, the reference count
  reaches zero and the node's memory can be released.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result
  is only used by one consumer, ownership is moved directly, avoiding counting overhead.

#### 5.2 Region Memory Allocation (Arena)

For dynamically generated short-lifecycle nodes (such as loop iterations), use an **arena
allocator**:

- Allocate a memory arena for one loop unrolling.
- Nodes within the arena are allocated contiguously and released as a whole, reducing fragmentation
  and deallocation overhead.
- When the arena ends, all node memory is reclaimed at once.

#### 5.3 Lock-free Data Structures

- Dependency counter: Use `AtomicUsize` with `fetch_sub` atomic decrement.
- Ready queue: Adopt Chase-Lev deque (per-thread local queue + work stealing), reducing lock
  contention.
- Downstream list: Read-only after creation, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute directly synchronously with zero
  scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., adjust worker
  thread count by monitoring queue length).

#### 5.5 Low-Cost Abstraction Principle

All scheduling overhead is proportional to task count, and per-task overhead (creation, enqueuing,
dependency handling) is controlled at the nanosecond level. For ultra-fine-grained tasks, scheduling
is avoided through inline optimization (see Section 3).

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
    arena_id: Option<ArenaId>,         // Arena affiliation (optional)
}

struct Scheduler {
    ready_queues: PerThreadQueue<TaskId>, // Per-thread local queue
    global_work_stealer: WorkStealer,
    arenas: ArenaAllocator,               // Region allocator
}
```

### 6.3 Context-based Optimization Analysis

The compiler performs the following steps at the MIR layer:

1. Build global call graph and data dependency graph.
2. For each function call node, calculate its out-degree (number of consumers).
3. If out-degree is 1, the function has no side effects (`@pure`), and is not recursive, mark as
   "inline candidate".
4. Evaluate inline benefits by combining heuristics (such as instruction count) to decide whether to
   inline.
5. When inlining, embed the call node's code into its downstream and update dependencies.

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

## Tradeoffs

### Advantages

- **Safe Dynamic Module Integration**: Metadata contracts enable dynamic libraries to seamlessly
  share the concurrency model with the main program while maintaining static DAG integrity.
- **Progressive FFI Integration**: Developers can progressively add annotations to external
  functions, transitioning from safe degradation to efficient concurrency.
- **Predictable Optimization**: Context-based decisions replace implicit thresholds, with
  transparent behavior; developers can understand optimizations through tools.
- **Natural Control Flow Integration**: Phi nodes and dynamic unrolling enable the DAG to represent
  all program structures without special syntax.
- **Scalable Performance**: Arena allocation, lock-free queues, and other designs ensure the
  scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata Contracts Increase Compilation Complexity**: Requires generating and parsing metadata
  for dynamic libraries; toolchain support is needed.
- **FFI Annotations Rely on Developer Correctness**: Incorrect annotations may lead to data races;
  documentation and tool hints can mitigate this risk.
- **Context Optimization Analysis is Time-Consuming**: Global analysis may increase compilation
  time, but can be alleviated through incremental compilation.
- **Dynamic Unrolling Increases Runtime Overhead**: Loop unrolling dynamically creates nodes, but
  arena allocation can mitigate.

## Implementation Strategy

### Phased Plan (Including Priority Suggestions)

> **Implementation Priority Recommendation**: Perfection is not required initially; simpler
> solutions can be used to get the system running, then optimized progressively. For example:
>
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can initially use a simple bump allocator, optimized later.

#### Phase 1: Basic Support (v0.7)

- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure`, `@io` annotations for FFI usage.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)

- [ ] Design metadata format; modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement compile-time metadata reading and placeholder node creation for main program.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)

- [ ] Implement call graph analysis; calculate node out-degree.
- [ ] Add inline decision and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)

- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop template and dynamic unrolling runtime.
- [ ] Complete background scheduling for infinite loops.

#### Phase 5: Performance Optimization (v1.0)

- [ ] Implement region allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmark testing and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels; replaces auto-fallback with
  context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI; maintains scheduler
  decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation; adds Phi nodes and dynamic
  unrolling.

## Appendix: Design Decision Records

| Decision                                        | Decision                                                           | Date       | Recorder |
| ----------------------------------------------- | ------------------------------------------------------------------ | ---------- | -------- |
| Dynamic modules provide metadata contracts      | Adopt metadata files + runtime binding                             | 2026-03-14 | Chen Xu  |
| FFI defaults to @block                          | Yes, developers can annotate progressively                         | 2026-03-14 | Chen Xu  |
| Context optimization replaces static thresholds | Intelligent decisions based on out-degree, side effects, etc.      | 2026-03-14 | Chen Xu  |
| Introduce Phi nodes for conditional branches    | Borrow from SSA; dynamically select branches                       | 2026-03-14 | Chen Xu  |
| Dynamic loop unrolling                          | Instantiate iterations on-demand; support dependency serialization | 2026-03-14 | Chen Xu  |
| Arena allocation for short-lifecycle nodes      | Improve memory efficiency and cache locality                       | 2026-03-14 | Chen Xu  |

## References

- [RFC-001: Spawn Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](../accepted/018-llvm-aot-compiler.md)
- [RFC-021: Library-Driven FFI Extension and Cross-Language Call Support](./021-library-driven-ffi-extension.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Deque](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)
