# RFC-020: Dynamic Modules and FFI Integration

> **Status**: Draft
> **Author**: Chen Xu (consolidated from community discussions)
> **Created**: 2026-03-14
> **Last Updated**: 2026-03-14

> **References**:
> - [RFC-001: Concurrency Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)

## Summary

This document further refines and extends YaoXiang's concurrency model based on RFC-001, 008, and 018, addressing practical scenarios such as **dynamic module loading**, **foreign function interface (FFI)**, and **finer-grained scheduling optimizations**. Core designs include:

1. **Dynamic Module Metadata Contract**: Provides compile-time dependency description for dynamic libraries written in the same language, enabling the main program to construct a static DAG while maintaining transparent concurrency.
2. **Layered FFI Handling**: External code is degraded to synchronous by default (`@block`), and developers can provide metadata via annotations to progressively integrate with concurrent scheduling. External concurrency is managed by the external language itself.
3. **Call Context-Based Optimization**: Instead of static threshold fallback, the compiler intelligently decides whether to inline or schedule as an independent node based on the function's actual role in the DAG (number of consumers, side effects, etc.).
4. **Control Flow and DAG Merging Mechanism**: Through Phi nodes and dynamic unrolling, dynamic structures like `if` and `loop` are naturally integrated into the dataflow graph.
5. **Runtime Scheduler Memory and Performance Optimization**: Clearly defines node lifecycle management, arena allocation, lock-free queues, and other low-cost abstraction implementations.

This document aims to complete the language specification, ensuring YaoXiang's concurrency model can handle both static whole-program analysis and dynamic/external interactions while maintaining high performance and developer experience.

## Motivation

### Limitations of Current Design

RFC-001/008/018 establish an elegant transparent concurrency model, but still have blind spots when facing real-world requirements:

- **Dynamic Modules**: When a program supports plugins or dynamically linked libraries, the main program cannot know the internal call relationships and dependencies of modules at compile time, causing global DAG construction to fail.
- **FFI Calls**: External functions (such as C libraries) are completely a black box. Their internals may contain concurrency, blocking, or side effects. Treating them as ordinary nodes would break concurrency safety.
- **Small Function Scheduling Overhead**: RFC-001's "L1 Auto Fallback" uses static thresholds (instruction count <50). This implicit rule makes behavior unpredictable for developers, and cannot adapt to complex call contexts.
- **Control Flow and DAG Fusion**: The representation of dynamic structures like `if` and `loop` in the DAG is not yet clear, potentially affecting the accuracy of dependency analysis.
- **Runtime Overhead Control**: As the number of DAG nodes increases, scheduler memory management and performance optimization need explicit design to avoid becoming bottlenecks.

### Goals

- Maintain the core philosophy of transparent concurrency while providing clear, safe, and progressive support for dynamic modules and FFI.
- Transform scheduling optimization from "implicit global rules" to "context-based intelligent decisions," improving predictability and performance.
- Complete the DAG representation of dynamic control flow, ensuring all program structures naturally integrate into the dataflow model.
- Clearly define scheduler memory management and performance optimization strategies to achieve low-cost abstractions.

## Proposal

### 1. Dynamic Module Metadata Contract

#### 1.1 Contract Contents

Each dynamic library (`.yxo` / platform-specific dynamic library) compiled by YaoXiang must be accompanied by a **metadata description file** (`.yxmeta`), containing:

- **Exported Function List**: Complete type signatures for each function (parameters, return values, resource markers).
- **Side Effect Tags**: Compiler-auto-derived `@pure` / `@io` (developers can also explicitly override).
- **Resource Dependencies**: Whether each parameter is a resource type (such as `File`), and whether the return value contains new resources.
- **Call Graph Summary** (optional): Other exported functions this function may call, used for cross-module circular dependency detection.
- **Ownership Information**: Ownership semantics for parameters (borrowed/moved), return value ownership.
- **Concurrency Safety**: Auto-derived results for satisfying `Send`/`Sync`.

Metadata format uses binary or structured text (such as MessagePack) to ensure parsing efficiency.

#### 1.2 Compile-Time Processing

When the main program compiles and encounters calls to dynamic module functions:

1. Read the corresponding module's `.yxmeta` file.
2. Create a **placeholder node** in the global DAG, recording input/output dependencies, side effect tags, etc. obtained from the metadata.
3. Placeholder nodes participate in dependency analysis like ordinary nodes, and the scheduler can plan execution order in advance.

#### 1.3 Runtime Binding

When dynamic modules are loaded:

- Runtime verifies that actual function signatures match the metadata (preventing version mismatches).
- Bind placeholder nodes to actual function pointers.

**Regarding Subgraph Scheduling Semantics**: If a dynamic module has an independent internal subgraph (for example, the module itself contains concurrent logic), that subgraph executes as an **independent scheduling unit**. Its boundaries are defined by the module's exported functions: when an exported function is called, the subgraph begins execution as a whole and continues until that function returns. Internal node scheduling within the subgraph is handled by the subgraph's own scheduler (modules can continue using the standard scheduler internally), but interactions between the subgraph and the main DAG are limited to input/output dataflow—the placeholder node in the main DAG only cares about the subgraph's start and end, not its internal scheduling. This design ensures module encapsulation while keeping the main DAG statically complete.

#### 1.4 Safety Guarantees

- If a dynamic module violates the contract (e.g., claiming `@pure` but modifying global state), consequences are borne by the developer (similar to FFI's unsafe boundaries). However, since it's the same language, runtime checks (such as memory isolation) can enhance safety but increase overhead.
- Cross-module Circular Dependencies: If module A calls B, and B calls A, and the call relationship is already declared in the metadata, the compiler can detect and report an error; if not declared, deadlock may occur at runtime, detected by the scheduler and triggering a panic.

### 2. Layered FFI Handling

#### 2.1 Default Degradation

External functions (introduced via `extern`) are treated as `@block` synchronous calls by default:

- Calls do not enter DAG scheduling and execute directly on the current thread.
- The YaoXiang scheduler does not intervene in their internal concurrency during execution.
- Return values can be used, but the call itself does not participate in dependency analysis.

> **⚠️ Developer Notice**: FFI calls do not participate in DAG scheduling by default, so any concurrency within them (such as threads started by C libraries) is entirely managed by the external language, and YaoXiang cannot track it. If you want FFI calls to integrate with concurrent scheduling, you must provide metadata via annotations (see section 2.2). Do not assume "synchronous code can automatically become concurrent"—FFI calls will not automatically parallelize.

#### 2.2 Optional Metadata Annotations

Developers can provide more precise metadata through annotations, enabling FFI calls to progressively integrate with concurrent scheduling:

```yaoxiang
@pure   // Pure function, no side effects, can be freely scheduled
extern "C" fn sin(x: Double) -> Double

@io     // Involves I/O, must be serialized by resource dependency
extern "C" fn write(fd: i32, buf: *u8, count: usize) -> isize

@block  // Explicitly declared as synchronous (default, can be omitted)
extern "C" fn old_api() -> i32
```

- If marked as `@pure` and parameters do not involve resource variables, treated as an ordinary DAG node and can execute in parallel.
- If marked as `@io` and parameters include resource types (such as wrapped `File`), dependency edges are automatically established.

#### 2.3 Resource Wrapping

The standard library is recommended to provide wrapper types for common FFI resources (such as file descriptors, database connections), encapsulating raw resources as YaoXiang resource types, thereby leveraging DAG dependency analysis. For example:

```yaoxiang
struct File { fd: i32 }  // Marked as resource type
impl File {
    @io
    fn write(self: &mut File, data: &[u8]) -> Result { ... }
}
```

Multiple `write` calls on the same `File` variable will automatically serialize.

#### 2.4 External Concurrency Management

If FFI functions start threads or perform async operations internally, this is entirely the responsibility of the external language. YaoXiang only cares about the start and end of the call, not internal details. Developers must ensure these concurrent operations do not conflict with YaoXiang's memory model (e.g., pay attention to ownership and Send constraints when using callbacks).

### 3. Call Context-Based Optimization

Instead of the "L1 Auto Fallback" static threshold from RFC-001, this approach uses **intelligent compiler decisions based on the actual context of each call site in the DAG**.

#### 3.1 Optimization Decision Basis

The compiler analyzes each function call node:

- **Number of Consumers**: How many downstream nodes use the results of this node. If it is 1, it qualifies as an inline candidate; if greater than 1, it must remain an independent node for result sharing.
- **Side Effects**: If the node has `@io` side effects, it must remain an independent node to ensure ordering.
- **Computational Cost Estimation**: Instruction count and other heuristics can still be referenced, but not as hard thresholds—they are only used for inline benefit assessment.
- **Resource Dependencies**: If the node involves resource variables (such as `File`) and those resource variables are passed between upstream and downstream, inlining may break dependency chains and requires caution.

#### 3.2 Inlining Operation

If the decision is to inline:

- Embed the node's computation logic directly into the code of its single downstream node.
- Remove the node from the DAG, and its inputs become direct inputs to the downstream node.
- During final code generation, inlined functions do not produce independent scheduling units.

#### 3.3 Inlining Restrictions

- Calls in recursive functions or loop bodies are typically not inlined to prevent infinite expansion.
- Functions across module boundaries (dynamic modules, FFI) are not inlined.
- Developers can use the `@noinline` annotation to forcibly prohibit inlining, or `@forceinline` to hint the compiler to attempt inlining.

#### 3.4 Observability

The compiler should generate optimization reports (can be enabled via `--emit-optimization-report`), containing the following information:

- **Each Inlining Point**: Lists inlined function names, call locations, and reasons for inlining (e.g., "single consumer and pure function").
- **Reasons for Keeping as Independent Node**: E.g., "has multiple consumers," "contains side effects," "cross-module call," etc.
- **Decision Statistics**: Total inlines, retained node counts, helping developers evaluate optimization effects.

Report output format can be text or JSON for easy tool parsing.

### 4. Control Flow and DAG Merging

#### 4.1 Conditional Branch (if) Handling

Introduce **Phi nodes** (borrowed from SSA form) to represent branch merge points:

- At compile time, construct for each `if` expression:
  - Two branch sub-DAGs (corresponding to `then` and `else` respectively).
  - A Phi node whose inputs include the condition variable and outputs from both branches.
- The semantics of a Phi node: When the condition variable is ready, it selects the corresponding branch's output as its own output based on the condition value.
- At runtime, the Phi node depends on the condition variable; after the condition is ready, it dynamically adds itself to the downstream list of the selected branch and waits for that branch's result.

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

Loops are treated as sub-DAGs with feedback edges, **dynamically unrolled at runtime as needed**:

- At compile time, identify loop bodies and construct **loop templates** containing:
  - Condition node.
  - Loop body sub-DAG.
  - State variables passed between iterations.
- At runtime, when the loop result is needed (e.g., using accumulated values after the loop ends), the scheduler begins dynamically unrolling iterations:
  1. Schedule the condition node for the first time; if true, instantiate the first iteration's sub-DAG with inputs including initial state and external variables.
  2. After iteration completion, produce new state, and schedule the condition node again (depending on the new state) to decide whether to continue.
  3. Repeat until the condition is false; the last iteration's output is the loop result.

**Complex Example: Loop Condition Depends on Updates Inside the Loop Body**  
```yaoxiang
let mut x = 0
while x < 10 {
    x = compute(x)  // x is updated inside the loop body
}
```

In this pattern, the condition node `x < 10` depends on the updated `x` after each iteration. The DAG representation is as follows:
- The loop template contains the state variable `x` with initial value 0.
- Each iteration: first execute the condition node (depending on current `x`), if true execute `x = compute(x)` producing new `x`, then re-enter the condition node.
- Dynamically unroll at runtime following the above process until the condition is false.

Dependencies between iterations naturally form dataflow through state variables; iterations with dependencies automatically serialize, while independent iterations can execute in parallel (e.g., `map`).

#### 4.3 Special Handling of Infinite Loops

A single infinite loop executes synchronously as the main DAG (zero scheduling overhead); multiple infinite loops execute as background DAGs with time-sliced concurrency managed by the scheduler.

### 5. Runtime Scheduler Memory and Performance Optimization

#### 5.1 Node Lifecycle Management

- Each node maintains a **reference count** (atomic variable) indicating how many consumers depend on its results.
- After a node finishes executing and passes results to all downstream nodes, its reference count reaches zero and the node's memory can be freed.
- Result values themselves also use reference counting (`Arc<T>`), but can be optimized: if a result is used by only one consumer, ownership is moved directly, avoiding counting overhead.

#### 5.2 Arena Memory Allocation

For dynamically generated nodes with short lifecycles (such as loop iterations), use an **arena allocator**:

- Allocate one memory arena for a single loop unrolling.
- Nodes within the arena are allocated consecutively, and the entire arena is freed at once, reducing fragmentation and deallocation overhead.
- When the arena ends, all node memory is reclaimed at once.

#### 5.3 Lock-Free Data Structures

- Dependency counters: Use `AtomicUsize` with `fetch_sub` for atomic decrement.
- Ready queue: Implement Chase-Lev deque (per-thread local queue + work stealing), reducing lock contention.
- Downstream list: Created as read-only after creation, avoiding concurrent modifications.

#### 5.4 Adaptive Scheduling

- If there is only one infinite loop in the system, execute directly and synchronously with zero scheduling overhead.
- Dynamically adjust parallelism based on task granularity and system load (e.g., adjust worker thread count by monitoring queue length).

#### 5.5 Low-Cost Abstraction Principles

All scheduling overhead is proportional to the number of tasks. Additional overhead per task (creation, enqueueing, dependency handling) is controlled at the nanosecond level. For ultra-fine-grained tasks, inline optimization (see section 3) avoids scheduling altogether.

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
2. Calculate the out-degree (number of consumers) for each function call node.
3. If the out-degree is 1, the function has no side effects (`@pure`), and is not recursive, mark it as "inline candidate."
4. Evaluate inline benefits using heuristics (such as instruction count) to decide whether to inline.
5. When inlining, embed the call node's code into its downstream and update dependency relationships.

### 6.4 Control Flow Node Representation

```rust
enum NodeKind {
    Normal(FuncId),
    Phi { cond: TaskId, then_branch: TaskId, else_branch: TaskId },
    LoopTemplate { cond: FuncId, body: FuncId, state_var: VarId },
    // ...
}
```

When dynamically unrolling at runtime, `LoopTemplate` generates a series of `Normal` node instances.

## Trade-offs

### Advantages

- **Safe Dynamic Module Integration**: The metadata contract enables dynamic libraries to seamlessly share the concurrency model with the main program while maintaining static DAG integrity.
- **Progressive FFI Integration**: Developers can progressively add annotations to external functions, transitioning from safe degradation to efficient concurrency.
- **Predictable Optimization**: Context-based decisions replace implicit thresholds; behavior is transparent, and developers can understand optimizations through tools.
- **Natural Control Flow Integration**: Phi nodes and dynamic unrolling enable the DAG to represent all program structures without special syntax.
- **Scalable Performance**: Arena allocation, lock-free queues, and other designs ensure the scheduler can handle large-scale concurrency.

### Disadvantages

- **Metadata Contract Increases Compile Complexity**: Generating and parsing metadata for dynamic libraries requires toolchain support.
- **FFI Annotations Rely on Developer Correctness**: Mislabeling may cause data races; documentation and tool hints are needed to reduce risk.
- **Context Optimization Analysis is Time-Consuming**: Global analysis may increase compilation time, but can be mitigated through incremental compilation.
- **Dynamic Unrolling Increases Runtime Overhead**: Loop unrolling requires dynamically creating nodes, but arena allocation can mitigate this.

## Implementation Strategy

### Phase Division (with Priority Recommendations)

> **Implementation Priority Recommendation**: Early stages need not pursue perfect implementation. Simple solutions can be used first to make the system functional, then optimized progressively. For example:
> - Reference counting can directly use `Arc`.
> - Lock-free queues can use mature libraries (such as crossbeam's deque).
> - Arena allocation can first use a simple bump allocator, optimized later.

#### Phase 1: Basic Support (v0.7)
- [ ] Implement FFI default degradation to `@block`.
- [ ] Add `@pure` and `@io` annotations for FFI use.
- [ ] Implement resource wrapper types (such as `File`) and their basic methods.

#### Phase 2: Dynamic Module Metadata (v0.8)
- [ ] Design metadata format and modify compiler to generate `.yxmeta` for dynamic libraries.
- [ ] Implement metadata reading and placeholder node creation at main program compile time.
- [ ] Implement runtime binding mechanism.

#### Phase 3: Context Optimization (v0.9)
- [ ] Implement call graph analysis and calculate node out-degrees.
- [ ] Add inline decision and code generation support.
- [ ] Implement optimization report output (including inline points, reasons, etc.).

#### Phase 4: Control Flow DAG Fusion (v0.10)
- [ ] Implement compile-time representation of Phi nodes and conditional branches.
- [ ] Implement loop templates and dynamic unrolling at runtime.
- [ ] Complete background scheduling for infinite loops.

#### Phase 5: Performance Optimization (v1.0)
- [ ] Implement arena allocator.
- [ ] Optimize lock-free queues and work stealing.
- [ ] Benchmarking and tuning.

## Relationship with Other RFCs

- **RFC-001**: Extends side effect handling and concurrency levels, replacing auto-fallback with context optimization.
- **RFC-008**: Supplements runtime support for dynamic modules and FFI, maintaining scheduler decoupling design.
- **RFC-018**: Refines DAG construction and scheduler implementation, adding Phi nodes and dynamic unrolling.

## Appendix: Design Decision Records

| Decision | Determination | Date | Recorder |
|----------|---------------|------|----------|
| Dynamic modules provide metadata contract | Adopt metadata file + runtime binding | 2026-03-14 | Chen Xu |
| FFI defaults to @block degradation | Yes, developers can progressively annotate | 2026-03-14 | Chen Xu |
| Context optimization replaces static thresholds | Intelligent decisions based on out-degree, side effects, etc. | 2026-03-14 | Chen Xu |
| Introduce Phi nodes for conditional branches | Borrow from SSA, dynamically select branches | 2026-03-14 | Chen Xu |
| Dynamic loop unrolling | Instantiate iterations on demand, support dependency serialization | 2026-03-14 | Chen Xu |
| Arena allocation for short-lifecycle nodes | Improve memory efficiency and cache locality | 2026-03-14 | Chen Xu |

## References

- [RFC-001: Concurrency Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
- [RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design](./018-llvm-aot-compiler.md)
- [SSA Form and Phi Functions](https://en.wikipedia.org/wiki/Static_single_assignment_form)
- [Chase-Lev Deque](https://en.wikipedia.org/wiki/Double-ended_queue#Chase-Lev_deque)