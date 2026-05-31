---
title: "RFC-020: Dynamic Modules and FFI Integration"
---

# RFC-020: Dynamic Modules and FFI Integration

> **Status**: Draft
> **Author**: Chen Xu (Consolidated from community discussions)
> **Created**: 2026-03-14
> **Last Updated**: 2026-03-14

> **References**:
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)

## Summary

This document builds upon RFC-001, 008, and 018 to further refine and extend YaoXiang's concurrency model to address real-world scenarios including **dynamic module loading**, **foreign function interfaces (FFI)**, and **finer-grained scheduling optimizations**. The core design includes:

1. **Dynamic Module Metadata Contract**: Provides compile-time dependency descriptions for dynamically linked libraries written in the same language, enabling the main program to statically construct a DAG while maintaining transparent concurrency.
2. **Layered FFI Handling**: External code defaults to synchronous (`@block`), with developers able to provide metadata via annotations to gradually integrate into concurrent scheduling; external concurrency is managed by external languages.
3. **Call-Context-Based Optimization**: Instead of static threshold fallback, the compiler intelligently decides whether to inline or schedule as independent nodes based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control Flow and DAG Merging Mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` naturally integrate into the dataflow graph.
5. **Runtime Scheduler Memory and Performance Optimization**: Clarifies node lifecycle management, arena allocation, lock-free queues, and other low-cost abstractions.

This document aims to complete the language specification, ensuring YaoXiang's concurrency model can handle both static whole-program analysis and flexible dynamic and external interactions while maintaining high performance and developer experience.

## Motivation

### Limitations of Current Design

RFC-001/008/018 established an elegant transparent concurrency model, but there remain blind spots when facing real-world requirements:

- **Dynamic modules**: When a program supports plugins or dynamic link libraries, the main program cannot know the internal call relationships and dependencies of modules at compile time, leading to global DAG construction failure.
- **FFI calls**: External functions (such as C library functions) are completely black boxes—their internals may contain concurrency, blocking, or side effects. Treating them as ordinary nodes would break concurrency safety.
- **Small function scheduling overhead**: The "L1 automatic fallback" proposed in RFC-001 uses a static threshold (instruction count <50). This implicit rule makes behavior unpredictable for developers, and cannot adapt to complex call contexts.
- **Control flow and DAG fusion**: The representation of dynamic structures like `if` and `loop` in the DAG is not yet clear, potentially affecting the accuracy of dependency analysis.
- **Runtime overhead control**: As the number of DAG nodes increases, scheduler memory management and performance optimization need explicit design to avoid becoming bottlenecks.

### Goals

- Maintain the core philosophy of transparent concurrency while providing clear, safe, and progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-aware intelligent decisions," improving predictability and performance.
- Complete the DAG's representation of dynamic control flow, ensuring all program structures naturally integrate into the dataflow model.
- Clarify scheduler memory management and performance optimization strategies to achieve low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Contents

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled with YaoXiang must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported function list**: Complete type signatures for each function (parameters, return values, resource markers).
- **Side effect markers**: Automatically inferred by the compiler as `@pure` / `@io` (developers may also explicitly override).
- **Resource dependencies**: Whether each parameter is a resource type (such as `File`), and whether the return value contains new resources.
- **Call graph summary** (optional): Other exported function IDs that this function may call, used for cross-module cyclic dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrowed/moved), return value ownership.
- **Concurrency safety**: Automatically inferred results for satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-Time Processing

When the main program compiles and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording input/output dependencies, side effect markers, etc. obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can plan execution order in advance.

#### 1.3 Runtime Binding

When dynamic modules are loaded at runtime:

- The runtime verifies that the actual function signature matches the metadata (preventing version mismatches).
- Bind placeholder nodes to actual function pointers.

**Regarding subgraph scheduling semantics**: If a dynamic module has an independent internal subgraph (for example, the module itself contains concurrency logic), that subgraph executes as a single **independent scheduling unit**. Its boundary is defined by the module's exported functions: when an exported function is called, the subgraph begins execution as a whole, until that function returns. Node scheduling inside the subgraph is managed by the subgraph's own scheduler (the module may continue using the standard scheduler internally), but the subgraph's interaction with the main DAG is limited to input/output dataflow—the placeholder node in the main DAG only concerns the subgraph's start and end, without intervening in its internal scheduling. This design ensures module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claims `@pure` but modifies global state), consequences are the developer's responsibility (similar to FFI's unsafe boundaries). However, since it's the same language, runtime checks (such as memory isolation) can enhance safety but add overhead.
- Cross-module cyclic dependencies: If module A calls B, and B calls A, and the call relationship is declared in the metadata, the compiler can detect and report an error; if not declared, deadlock may occur at runtime, detected by the scheduler and triggering panic.

### 2. Layered FFI Handling

#### 2.1 Default Fallback

External functions (introduced via `extern`) are treated as `@block` synchronous calls by default:

- Calls do not enter DAG scheduling, executing directly on the current thread.
- The YaoXiang scheduler does not intervene in its internal concurrency during execution.
- Return values are available, but the call itself does not participate in dependency analysis.

> **⚠️ Developer Notice**: FFI calls do not participate in DAG scheduling by default, so any concurrency inside them (such as threads started by a C library) is entirely managed by the external language, and YaoXiang cannot track it. If you want FFI calls to integrate into concurrent scheduling, provide metadata via annotations (see Section 2.2). Do not assume "synchronous code automatically becomes concurrent"—FFI calls will not automatically parallelize.

#### 2.2 Optional Metadata Annotations

Developers can provide more precise metadata via annotations, allowing FFI calls to progressively integrate into concurrent scheduling:

```yaoxiang
@pure   // Pure function, no side effects, free to schedule
extern "C" fn sin(x: Double) -> Double

@io     // Involves I/O, needs serialization by resource dependency
extern "C" fn write(fd: i32, buf: *u8, count: usize) -> isize

@block  // Explicitly declared as synchronous (default, can be omitted)
extern "C" fn old_api() -> i32
```

- If marked as `@pure` and parameters do not involve resource variables, it is treated as an ordinary DAG node and can execute in parallel.
- If marked as `@io` and parameters include resource types (such as wrapped `File`), dependency edges are automatically established.

#### 2.3 Resource Wrapping

The standard library is encouraged to provide wrapper types for common FFI resources (such as file descriptors, database connections), wrapping raw resources into YaoXiang resource types to leverage DAG dependency analysis. For example:

```yaoxiang
struct File { fd: i32 }  // Marked as resource type
impl File {
    @io
    fn write(self: &mut File, data: &[u8]) -> Result { ... }
}
```

This way, multiple `write` calls on the same `File` variable will automatically serialize.

#### 2.4 External Concurrency Management

If FFI functions start threads or perform asynchronous operations internally, this is entirely the responsibility of the external language. YaoXiang only concerns itself with the start and end of calls, not internal details. Developers must ensure these concurrent operations do not conflict with YaoXiang's memory model (e.g., pay attention to ownership and Send constraints when using callbacks).

### 3. Call-Context-Based Optimization

Replacing the "L1 automatic fallback" static threshold from RFC-001, this is now **intelligent decision-making by the compiler based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Basis

The compiler analyzes each function call node:

- **Number of consumers**: How many downstream nodes use this node's result. If it is 1, it qualifies as an inline candidate; if greater than 1, it must remain as an independent node for result sharing.
- **Side effects**: If the node has `@io` side effects, it must remain as an independent node to ensure ordering.
- **Computation estimation**: Instruction count and other heuristics may still be referenced, but not as hard thresholds—only used for inline benefit assessment.
- **Resource dependencies**: If the node involves resource variables (such as `File`) that are passed between upstream and downstream, inlining may break dependency chains and requires caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- Directly embed the node's computation logic into the code of its single downstream node.
- Remove the node from the DAG, with its inputs directly becoming the downstream node's inputs.
- During final code generation, inlined functions do not produce independent scheduling units.

#### 3.3 Inlining Restrictions

- Calls inside recursive functions or loop bodies are typically not inlined to prevent infinite expansion.
- Functions across module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to explicitly prohibit inlining, or `@forceinline` to hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (enabled via `--emit-optimization-report`), containing:

- **Each inline site**: Lists of inlined function names, call locations, and inline reasons (e.g., "unique consumer and pure function").
- **Reasons for remaining as independent nodes**: E.g., "has multiple consumers," "contains side effects," "cross-module call," etc.
- **Decision statistics**: Total inlines, retained nodes, helping developers evaluate optimization effects.

The report output format can be text or JSON for easy tool parsing.

### 4. Control Flow and DAG Merging

#### 4.1 Conditional Branch (if) Handling

Introduce **Phi nodes** (borrowing from SSA form) to represent branch merge points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- The Phi node's semantics: When the condition variable is ready, select the corresponding branch's output as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it dynamically adds itself to the selected branch's downstream list and waits for that branch's result.

Example DAG:
```
        cond
       /    \
  then DAG  else DAG
       \    /
        Phi
         |
      downstream nodes
```

#### 4.2 Loop (loop/while) Handling

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled on demand** at runtime:

- At compile time, identify the loop body and construct a **loop template**, containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using accumulated values after the loop ends), the scheduler begins dynamically unrolling iterations:
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

In this pattern, the condition node `x < 10` depends on the updated `x` after each iteration. The DAG representation is:

- The loop template contains the state variable `x` with initial value 0.
- Each iteration: first execute the condition node (depending on current `x`); if true, execute `x = compute(x)` and produce new `x`, then re-enter the condition node.
- Dynamically unroll at runtime according to the above process until the condition is false.

Dependencies between iterations naturally form dataflow through state variables; iterations with dependencies automatically serialize, while independent iterations can parallelize (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes directly and synchronously as the main DAG (no scheduling overhead); multiple infinite loops execute as background DAGs with time-sliced concurrency by the scheduler.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable) representing the number of consumers depending on its results.
- When a node finishes executing and passes results to all downstream nodes, its reference count reaches zero, and the node's memory can be freed.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result is used by only one consumer, ownership is moved directly to avoid counting overhead.

#### 5.2 Arena Memory Allocation

For dynamically generated large numbers of short-lifecycle nodes (such as loop iterations), use an **arena allocator**:

- Allocate a memory arena for each loop unrolling.
- Nodes within the arena are allocated contiguously, released as a whole, reducing fragmentation and deallocation overhead.
- When the arena ends, all node memory is reclaimed at once.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize` with atomic decrement via `fetch_sub`.
- Ready queue: Adopt Chase-Lev double-ended queue (per-thread local queue + work stealing) to reduce lock contention.
- Downstream list: Read-only after creation, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute directly and synchronously with zero scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., adjust worker thread count by monitoring queue length).

#### 5.5 Low-Cost Abstraction Principles

All scheduling overhead is proportional to the number of tasks; additional overhead per task (creation, enqueuing, dependency handling) is controlled at the nanosecond level. For ultra-fine-grained tasks, scheduling is avoided through inlining optimization (see Section 3).

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
    arena_id: Option<ArenaId>,         // Arena affiliation (optional)
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
2. Calculate out-degree (number of consumers) for each function call node.
3. If out-degree is 1, the function has no side effects (`@pure`), and is not recursive, mark it as "inline candidate."
4. Combine heuristics (such as instruction count) to evaluate inline benefits and decide whether to inline.
5. When inlining, embed the call node's code into its downstream node and update dependencies.

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

## Trade-offs

### Advantages

- **Safe dynamic module integration**: The metadata contract enables dynamic libraries to seamlessly share the concurrency model with the main program while maintaining static DAG integrity.
- **Progressive FFI integration**: Developers can progressively add annotations to external functions, transitioning from safe fallback to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds, with transparent behavior and tools to help developers understand optimizations.
- **Natural control flow integration**: Phi nodes and dynamic unrolling enable the DAG to represent all program structures without special syntax.
- **Scalable performance**: Arena allocation, lock-free queues, and other designs ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata contract increases compilation complexity**: Generating and parsing metadata for dynamic libraries is required, with toolchain support needed.
- **FFI annotation depends on developer correctness**: Incorrect annotations may cause data races; documentation and tool hints help mitigate risks.
- **Context optimization analysis is time-consuming**: Global analysis may increase compile time, but can be alleviated through incremental compilation.
- **Dynamic unrolling increases runtime overhead**: Loop unrolling requires dynamically creating nodes, though arena allocation mitigates this.

## Implementation Strategy

### Phase Breakdown (with Priority Recommendations)

> **Implementation priority recommendation**: Perfection is not required initially. Start with simple solutions to get the system running, then optimize progressively. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can use a simple bump allocator first, optimized later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement FFI default fallback to `@block`.
- [ ] Add `@pure`, `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design metadata format, modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement main program compile-time metadata reading and placeholder node creation.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)
- [ ] Implement call graph analysis and node out-degree calculation.
- [ ] Add inline decision and code generation support.
- [ ] Implement optimization report output (including inline sites, reasons, etc.).

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

## Appendix: Design Decision Records

| Decision | Resolution | Date | Recorded By |
|------|------|------|--------|
| Dynamic modules provide metadata contract | Adopt metadata file + runtime binding | 2026-03-14 | Chen Xu |
| FFI defaults to @block fallback | Yes, with progressive developer annotations | 2026-03-14 | Chen Xu |
| Context optimization replaces static threshold | Intelligent decision based on out-degree, side effects, etc. | 2026-03-14 | Chen Xu |
| Introduce Phi nodes for conditional branches | Borrow from SSA, dynamic branch selection | 2026-03-14 | Chen Xu |
| Dynamic loop unrolling | On-demand iteration instantiation, with dependency serialization | 2026-03-14 | Chen Xu |
| Arena allocation for short-lifecycle nodes | Improve memory efficiency and cache locality | 2026-03-14 | Chen Xu |

## References

- [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-Ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)