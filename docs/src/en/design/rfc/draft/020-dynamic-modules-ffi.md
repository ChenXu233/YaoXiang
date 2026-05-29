# RFC-020: Dynamic Modules and FFI Integration

> **Status**: Draft
> **Author**: Morning Dawn (compiled from community discussions)
> **Created**: 2026-03-14
> **Last Updated**: 2026-03-14

> **References**:
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)

## Abstract

Building on RFC-001, 008, and 018, this document further refines and extends YaoXiang's concurrent model to address real-world scenarios such as **dynamic module loading**, **foreign function interface (FFI)**, and **finer-grained scheduling optimizations**. The core design includes:

1. **Dynamic module metadata contract**: Provides compile-time dependency description for dynamic libraries written in the same language, enabling the main program to construct a static DAG while maintaining transparent concurrency.
2. **FFI layered processing**: External code is degraded to synchronous (`@block`) by default; developers can provide metadata via annotations to gradually integrate with concurrent scheduling; external concurrency is managed by the external language itself.
3. **Call context-based optimization**: Replaces static threshold fallback; the compiler intelligently decides whether to inline or schedule as independent nodes based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control flow and DAG merging mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` naturally integrate into the data flow graph.
5. **Runtime scheduler memory and performance optimization**: Clarifies low-cost abstraction implementations such as node lifecycle management, arena allocation, and lock-free queues.

This document aims to perfect the language specification, ensuring YaoXiang's concurrent model can handle both static whole-program analysis and dynamic/external interactions, while maintaining high performance and developer experience.

## Motivation

### Limitations of Existing Design

RFC-001/008/018 established an elegant transparent concurrency model, but still have blind spots when facing real-world requirements:

- **Dynamic modules**: When a program supports plugins or dynamically linked libraries, the main program cannot know the internal call relationships and dependencies of modules at compile time, causing global DAG construction to fail.
- **FFI calls**: External functions (such as C libraries) are completely a black box; their internals may contain concurrency, blocking, or side effects. Treating them as ordinary nodes would break concurrency safety.
- **Small function scheduling overhead**: The "L1 automatic fallback" in RFC-001 uses a static threshold (instruction count < 50). This implicit rule makes it difficult for developers to predict behavior, and it cannot adapt to complex call contexts.
- **Control flow and DAG fusion**: The representation of dynamic structures like `if` and `loop` in the DAG is not yet clear, which may affect the accuracy of dependency analysis.
- **Runtime overhead control**: As DAG node count increases, scheduler memory management and performance optimization need explicit design to avoid becoming a bottleneck.

### Goals

- Under the premise of preserving the core philosophy of transparent concurrency, provide clear, safe, and progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-aware intelligent decisions," improving predictability and performance.
- Complete the DAG's representation of dynamic control flow, ensuring all program structures naturally integrate into the data flow model.
- Clarify scheduler memory management and performance optimization strategies to achieve low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Content

Each dynamic library compiled by YaoXiang (`.yxo` / platform-specific dynamic library) must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported function list**: Complete type signatures for each function (parameters, return values, resource markers).
- **Side effect markers**: Automatically inferred by the compiler as `@pure` / `@io` (developers can also explicitly override).
- **Resource dependencies**: Whether each parameter is a resource type (such as `File`), and whether the return value contains new resources.
- **Call graph summary** (optional): Other exported functions this function may call, used for cross-module circular dependency detection.
- **Ownership information**: Ownership semantics of parameters (borrow/move), return value ownership.
- **Concurrency safety**: Automatically inferred results for satisfying `Send`/`Sync`.

The metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-time Processing

When the main program compiles and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording input/output dependencies, side effect markers, etc., obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can plan execution order in advance.

#### 1.3 Runtime Binding

When dynamic modules are loaded:

- The runtime verifies that the actual function signature matches the metadata (preventing version mismatches).
- Binds the placeholder node to the actual function pointer.

**Regarding subgraph scheduling semantics**: If a dynamic module has an independent internal subgraph (for example, the module itself contains concurrent logic), that subgraph executes as an **independent scheduling unit**. Its boundary is defined by the module's exported functions: when an exported function is called, the subgraph begins execution as a whole and continues until that function returns. Internal node scheduling within the subgraph is handled by the subgraph's own scheduler (modules can continue using the standard scheduler internally), but the subgraph's interaction with the main DAG is limited to input/output data flow — the placeholder node in the main DAG only concerns the subgraph's start and end, not its internal scheduling. This design guarantees module encapsulation while keeping the main DAG statically intact.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claiming `@pure` but modifying global state), the consequences are the developer's responsibility (similar to FFI's unsafe boundary). However, since it is the same language, runtime checks (such as memory isolation) can enhance safety but add overhead.
- Cross-module circular dependencies: If module A calls B, and B calls A, and the call relationship is already declared in the metadata, the compiler can detect and report an error; if not declared, deadlock may occur at runtime, which the scheduler detects and panics.

### 2. FFI Layered Processing

#### 2.1 Default Degradation

External functions (introduced via `extern`) are treated as `@block` synchronous calls by default:

- Calls do not enter DAG scheduling and execute directly on the current thread.
- The YaoXiang scheduler does not intervene in their internal concurrency during execution.
- Return values can be used, but the call itself does not participate in dependency analysis.

> **⚠️ Developer Notice**: FFI calls do not participate in DAG scheduling by default, so any concurrency within them (such as threads started by C libraries) is entirely managed by the external language, and YaoXiang cannot track it. If you want FFI calls to integrate with concurrent scheduling, you need to provide metadata via annotations (see Section 2.2). Do not assume "synchronous code will automatically parallelize" — FFI calls will not automatically be parallelized.

#### 2.2 Optional Metadata Annotations

Developers can provide more precise metadata via annotations to gradually integrate FFI calls into concurrent scheduling:

```yaoxiang
@pure   // Pure function, no side effects, can be freely scheduled
extern "C" fn sin(x: Double) -> Double

@io     // Involves I/O, needs serialization by resource dependency
extern "C" fn write(fd: i32, buf: *u8, count: usize) -> isize

@block  // Explicitly declared as synchronous (default, can be omitted)
extern "C" fn old_api() -> i32
```

- If marked `@pure` and parameters do not involve resource variables, it is treated as an ordinary DAG node and can be executed in parallel.
- If marked `@io` and parameters include resource types (such as wrapped `File`), dependency edges are automatically established.

#### 2.3 Resource Wrapping

It is recommended that the standard library provide wrapper types for common FFI resources (such as file descriptors, database connections), wrapping raw resources into YaoXiang resource types to leverage DAG dependency analysis. For example:

```yaoxiang
struct File { fd: i32 }  // Marked as resource type
impl File {
    @io
    fn write(self: &mut File, data: &[u8]) -> Result { ... }
}
```

This way, multiple `write` calls on the same `File` variable will automatically be serialized.

#### 2.4 External Concurrency Management

If an FFI function starts threads or asynchronous operations internally, it is entirely the responsibility of the external language. YaoXiang only cares about the start and end of the call, not internal details. Developers must ensure these concurrent operations do not conflict with YaoXiang's memory model (for example, pay attention to ownership and Send constraints when using callbacks).

### 3. Call Context-Based Optimization

Replacing the "L1 automatic fallback" static threshold from RFC-001, this is now **an intelligent decision by the compiler based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Basis

The compiler analyzes each function call node:

- **Consumer count**: How many downstream nodes use this node's result. If it is 1, it is a candidate for inlining; if greater than 1, it must remain an independent node for result sharing.
- **Side effects**: If the node has `@io` side effects, it must remain an independent node to guarantee ordering.
- **Computation estimation**: Instruction count and other heuristics can still be referenced, but not as a hard threshold; they are only used for inline benefit evaluation.
- **Resource dependencies**: If the node involves resource variables (such as `File`), and the resource variable is passed between upstream and downstream, inlining may break the dependency chain and requires caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- The node's computation logic is directly embedded into the code of its single downstream node.
- The node is removed from the DAG, and its inputs directly become the downstream node's inputs.
- During final code generation, inlined functions do not produce independent scheduling units.

#### 3.3 Inlining Restrictions

- Recursive functions or calls within loops are typically not inlined to prevent infinite expansion.
- Calls across module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prohibit inlining, or `@forceinline` to hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate an optimization report (available via `--emit-optimization-report`), containing:

- **Each inlining point**: Lists inlined function names, call locations, and reasons for inlining (e.g., "unique consumer and pure function").
- **Reasons for keeping as independent nodes**: e.g., "has multiple consumers," "contains side effects," "cross-module call," etc.
- **Decision statistics**: Total inlines, retained node counts, helping developers evaluate optimization effects.

Report output can be text or JSON for tool parsing.

### 4. Control Flow and DAG Merging

#### 4.1 Conditional Branch (if) Handling

Introduce **Phi nodes** (borrowing from SSA form) to represent branch confluence points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- The semantics of a Phi node: Once the condition variable is ready, it selects the output from the corresponding branch based on the condition value as its own output.
- At runtime, the Phi node depends on the condition variable; once the condition is ready, it dynamically adds itself to the downstream list of the selected branch and waits for that branch's result.

Example DAG:
```
        cond
       /    \
  then DAG  else DAG
       \    /
        Phi
         |
      subsequent node
```

#### 4.2 Loop (loop/while) Handling

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled at runtime on demand**:

- At compile time, identify the loop body and construct a **loop template** containing:
  - A condition node.
  - A loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when a loop result is needed (e.g., using an accumulated value after the loop ends), the scheduler begins dynamically unrolling iterations:
  1. Schedule the condition node for the first time; if true, instantiate the sub-DAG for the first iteration, with inputs including initial state and external variables.
  2. After iteration completion, produce new state, and schedule the condition node again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the output of the last iteration is the loop result.

**Complex example: Loop condition depends on updates inside the loop body**
```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```

In this pattern, the condition node `x < 10` depends on `x` updated after each iteration. The DAG representation is as follows:
- The loop template contains the state variable `x` with an initial value of 0.
- Each iteration: first execute the condition node (depending on current `x`), if true execute `x = compute(x)` and produce new `x`, then enter the condition node again.
- Dynamically unroll at runtime according to the above process until the condition is false.

Dependencies between iterations naturally form data flow through state variables; iterations with dependencies automatically serialize, while independent iterations can parallelize (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously as part of the main DAG (no scheduling overhead); multiple infinite loops run as background DAGs, executed concurrently with scheduler time-slice slicing.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable) indicating how many consumers depend on its result.
- After a node finishes execution and passes its result to all downstream nodes, its reference count reaches zero, and the node's memory can be freed.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result is only used by one consumer, ownership is moved directly to avoid counting overhead.

#### 5.2 Arena Memory Allocation

For dynamically generated short-lifecycle nodes (such as loop iterations), use an **arena allocator**:

- Allocate a memory region for one loop unrolling.
- Nodes within the region are allocated contiguously and freed as a whole, reducing fragmentation and deallocation overhead.
- When the region ends, all node memory is reclaimed in one pass.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize` with `fetch_sub` for atomic decrement.
- Ready queue: Adopt the Chase-Lev double-ended queue (per-thread local queue + work stealing), reducing lock contention.
- Downstream list: Created once and then read-only, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute directly synchronously with zero scheduling overhead.
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
    arena_id: Option<ArenaId>,         // Arena membership (optional)
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
4. Evaluate inline benefits using heuristics (such as instruction count) to decide whether to inline.
5. During inlining, embed the call node's code into its downstream and update dependencies.

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

- **Safe dynamic module integration**: The metadata contract allows dynamic libraries to seamlessly share the concurrency model with the main program while maintaining a static DAG's integrity.
- **Progressive FFI integration**: Developers can gradually add annotations to external functions, transitioning from safe degradation to efficient concurrency.
- **Predictable optimization**: Context-based decisions replace implicit thresholds; behavior is transparent, and developers can understand optimizations through tools.
- **Natural control flow integration**: Phi nodes and dynamic unrolling allow the DAG to represent all program structures without special syntax.
- **Scalable performance**: Arena allocation, lock-free queues, and other designs ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata contract increases compilation complexity**: Requires generating and parsing metadata for dynamic libraries, and toolchain support.
- **FFI annotation relies on developer correctness**: Incorrect annotations may cause data races; documentation and tool hints can mitigate this risk.
- **Context optimization analysis is time-consuming**: Global analysis may increase compilation time, but can be alleviated through incremental compilation.
- **Dynamic unrolling adds runtime overhead**: Loop unrolling requires dynamically creating nodes, though arena allocation can mitigate.

## Implementation Strategy

### Phase Breakdown (with Priority Recommendations)

> **Implementation priority recommendation**: It is not necessary to pursue perfect implementation initially; simpler solutions can be used to get the system running, then optimized gradually. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can initially use a simple bump allocator, optimized later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure` and `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design metadata format, modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement main program reading metadata at compile time and creating placeholder nodes.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)
- [ ] Implement call graph analysis, calculate node out-degrees.
- [ ] Add inline decision-making and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)
- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement runtime loop templates and dynamic unrolling.
- [ ] Complete background scheduling for infinite loops.

#### Phase 5: Performance Optimization (v1.0)
- [ ] Implement arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmark testing and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing automatic fallback with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining scheduler decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic unrolling.

## Appendix: Design Decision Log

| Decision | Decision Made | Date | Recorder |
|----------|---------------|------|----------|
| Dynamic modules provide metadata contract | Adopt metadata file + runtime binding | 2026-03-14 | Morning Dawn |
| FFI defaults to @block degradation | Yes, developers can annotate progressively | 2026-03-14 | Morning Dawn |
| Context optimization replaces static threshold | Intelligent decisions based on out-degree, side effects, etc. | 2026-03-14 | Morning Dawn |
| Introduce Phi nodes for conditional branches | Borrow from SSA, dynamically select branches | 2026-03-14 | Morning Dawn |
| Loop dynamic unrolling | Instantiate iterations on demand, support dependency serialization | 2026-03-14 | Morning Dawn |
| Arena allocation for short-lifecycle nodes | Improve memory efficiency and cache locality | 2026-03-14 | Morning Dawn |

## References

- [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Double-Ended Queue](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)