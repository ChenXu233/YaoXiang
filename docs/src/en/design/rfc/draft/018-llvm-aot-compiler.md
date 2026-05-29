---
title: RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design
---

# RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency Design

> **Status**: Draft
> **Author**: Chen Xu
> **Created**: 2026-02-15
> **Last Updated**: 2026-03-10

> **References**:
> - [RFC-001: Concurrency Model and Error Handling System](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md)

## Abstract

This document designs the LLVM AOT compiler for the YaoXiang language, with the goal of generating machine code + DAG metadata through ahead-of-time compilation, executed by the runtime's **global DAG scheduler** based on **bottom-up** dependency analysis.

**Core Innovations**:
- Not "generate a Future when encountering a function call", but **reverse analysis of dependencies starting from "where the result is needed"**
- **Leaf nodes execute in parallel first**, dependency chains are traversed upward in order
- **Isolated DAGs execute independently in parallel**: nodes without consumers do not block the main flow
- **Infinite loops as background DAGs**: the scheduler slices execution to avoid deadlocks

This design is fundamentally different from the Rust async/await + tokio runtime model:
- Rust: Users write `async fn`, compiler generates a state machine
- YaoXiang: Users write ordinary functions, **compiler automatically analyzes DAG**, scheduler executes bottom-up

Follows RFC-001's L3 transparent concurrency design: default @auto (automatic parallelism), @block synchronous is the special case, solving the "colored function" problem.

## Motivation

### Why is an LLVM AOT Compiler Needed?

Currently, YaoXiang only has an interpreter as the execution backend, with the following issues:

| Issue | Impact |
|-------|--------|
| Performance bottleneck | Interpreted execution is 10-100x slower than machine code |
| Complex deployment | Need to carry interpreter and runtime |
| Colored function problem | Synchronous functions cannot call concurrent functions |

### The Colored Function Problem and L3 Transparent Concurrency

**Traditional design (current)**:
- Synchronous functions (blue) → cannot call → concurrent functions (red)
- Synchronous is the default, concurrency requires `spawn` marking
- Colors "spread": once concurrency is used, the entire call chain becomes concurrent

**RFC-001 L3 transparent concurrency (goal)**:
- L3: Default transparent concurrency (@auto)
- L2: Explicit spawn concurrency
- L1: @block synchronous mode

**Inverted design (RFC-018)**:
- Default L3 transparent concurrency, automatic DAG dependency analysis at compile time
- Solves the colored function problem: synchronous functions can directly call code with "default concurrency"
- @block only as a special case to force serial execution

### Core Innovation: Bottom-Up Execution + Global DAG

The core innovation of this design is the **bottom-up execution model**:

```
Traditional calls (top-down):
  call fetch(url) → execute → return result

Bottom-up execution:
  print(a) ← start from "where the result is needed"
       ↑
  fetch(url0) ← analyze dependencies, search backwards

  fetch(url1) ← isolated, execute independently in parallel
```

**Key differences**:
- Not "generate a Future when encountering a function call"
- But reverse analysis of dependencies starting from "the final result needed"
- Nodes without consumers (isolated) are not executed or run independently in parallel
- Infinite loops as background DAGs, scheduler slices execution

### Comparison with Rust async

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async Model                            │
├─────────────────────────────────────────────────────────────────┤
│  Compile time: Generate state machine + machine code            │
│  Runtime: tokio scheduler schedules based on state machine      │
│  Characteristics: await points determined at compile time,      │
│                   state machine manages execution               │
│  Granularity: Function level                                     │
│  User experience: Need to write async/await keywords           │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT Model                    │
├─────────────────────────────────────────────────────────────────┤
│  Compile time: Generate machine code + DAG metadata            │
│  Runtime: Global DAG scheduler, bottom-up execution             │
│  Characteristics: Reverse analysis of dependencies from          │
│                   "where the result is needed", leaf nodes      │
│                   execute in parallel                           │
│  Granularity: DAG within function blocks + cross-function DAG   │
│  User experience: Ordinary functions, automatic parallelism      │
└─────────────────────────────────────────────────────────────────┘
```

### Global DAG Scheduler

```
DAG view of the entire program:

        print(result) ─────────────────────────┐
           │                                    │
    ┌──────┴──────┐                             │
    │             │                             │
process(a)   process(b)                        │
    │             │                             │
compute(x)   compute(y)  ←── isolated DAG ─────┤
    │                                           │
fetch(url0)  fetch(url1)  fetch(url2)          │
    (executed)                                   │

There's also a background DAG (while True):
    ┌─────────────────────────────────────────┐ │
    │  while True:                            │ │
    │      update_ui()                        │ │
    │      fetch_new() ──→ process(data)      │ │
    └─────────────────────────────────────────┘ │
```

**How the scheduler works**:
```
1. Reverse analysis from "final result":
   print(result) → depends on process → depends on fetch

2. Build global DAG:
   - Leaf nodes: fetch (no dependencies)
   - Internal nodes: process, compute
   - Root node: print

3. Execution:
   - fetch executes in parallel
   - process waits for fetch to complete
   - print waits for process to complete
   - Isolated compute runs independently in parallel

4. Skip already executed:
   - If a node has been executed, subsequent nodes depending
     on it can reuse the result
```

### Infinite Loop Handling

```
Scenario 1: Single while/for (no scheduling overhead)
──────────────────────────────────────────────────────
main: () -> () = {
    while True {
        update_ui()
        fetch_data()
    }
}
→ Only one infinite loop
→ Execute synchronously directly, no different from ordinary code

Scenario 2: Multiple while (automatic slicing)
──────────────────────────────────────────────────────
main: () -> () = {
    while True { update_ui() }      # background task 1
    while True { network_poll() }  # background task 2
    server_loop()                   # main task
}
→ 3 independent tasks
→ Scheduler slices and switches
→ True concurrency

Scheduler adaptation:
──────────────────────────────────────────────────────
if num_tasks == 1:
    execute directly (synchronous)
else:
    slice scheduling (concurrent)
```

**Background DAG handling**:
```
Main DAG (has end):
    fetch → process → print → end

Background DAG (infinite loop):
    while True → update_ui → fetch_new → process → back to start

Scheduler:
    - Main DAG ends when execution completes
    - Background DAG runs forever, but scheduler executes it in "slices"
    - Will not get stuck in the loop
```

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────┐
│  Compile Time                                        │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG Analysis│→│LLVM Codegen│→ Machine Code │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              Generate: DAG Metadata                │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  Runtime                                             │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG Scheduler Library                       │ │
│  │  • Load machine code                        │ │
│  │  • Read DAG metadata                        │ │
│  │  • Lazy scheduling: suspend calls,           │ │
│  │    execute on demand                        │ │
│  │  • Support parallel/serial execution         │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Bottom-Up Execution Flow

```
User code:
    main: () -> () = {
        pages = urls.map(|url| fetch(url))
        results = pages.map(|page| parse_page(page))
        save_results(results)
    }

Phase 1: Bottom-up analysis (compile time)
─────────────────────────────────────────
Starting from save_results(results):
    "need results" → depends on parse_page(results)
    "need page0" → depends on fetch(url0)
    "need page1" → depends on fetch(url1)
    ...

Build global DAG:
    fetch(url0), fetch(url1), fetch(url2) ← leaf nodes
           ↓
    parse_page(page0), parse_page(page1)   ← depends on leaves
           ↓
    save_results                          ← root node

Phase 2: Execute leaves in parallel (runtime)
─────────────────────────────────────────
Scheduler finds all leaf nodes:
    - fetch(url0), fetch(url1), fetch(url2) have no dependencies → execute in parallel
    - Control concurrency (e.g., 16 at a time)

Phase 3: Traverse upward
─────────────────────────────────────────
When parse_page needs page0:
    - Check if page0 is ready
    - Ready → execute parse_page
    - Not ready → wait, continue after completion

Phase 4: Isolated DAGs execute independently
─────────────────────────────────────────
If some fetch has no one wanting the result:
    - Execute as "isolated DAG" independently
    - Can use another core, doesn't affect main flow
```

### Compiled Artifact Structure

```rust
/// Compiled artifact: machine code + DAG metadata
pub struct CompiledArtifact {
    /// LLVM compiled machine code (ELF/Mach-O/COFF)
    machine_code: Vec<u8>,

    /// DAG metadata: describes function dependency relationships
    dag: DAGMetadata,

    /// Entry point table
    entries: Vec<EntryPoint>,

    /// Type information (for FFI)
    type_info: TypeInfo,
}

/// DAG metadata
pub struct DAGMetadata {
    /// Nodes: function calls
    nodes: Vec<DAGNode>,
    /// Edges: dependency relationships (from, to)
    edges: Vec<(usize, usize)>,
}

/// Single DAG node
pub struct DAGNode {
    /// Function ID
    pub function_id: usize,
    /// Node IDs this depends on
    pub deps: Vec<usize>,
    /// Side effect tag (@IO / @Pure)
    pub effect: EffectTag,
}
```

### Runtime Scheduler Interface

```rust
/// DAG scheduler trait
pub trait DAGScheduler: Send + Sync {
    /// Schedule execution
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue;

    /// Execute single function
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}

/// Scheduler implementation
pub struct DefaultDAGScheduler {
    /// Thread pool
    thread_pool: ThreadPool,
    /// Compiled artifact
    artifact: CompiledArtifact,
    /// Maximum parallelism
    max_parallelism: usize,
}

impl DefaultDAGScheduler {
    pub fn new(artifact: CompiledArtifact, num_workers: usize) -> Self {
        Self {
            thread_pool: ThreadPool::new(num_workers),
            artifact,
            max_parallelism: num_workers * 2, // Adaptive granularity control
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. Traverse function body, suspend all calls
        // 2. Build task list to execute
        // 3. Schedule execution in dependency order (control concurrency)
        // 4. Trigger execution when value is needed
        // 5. Return result
    }
}
```

### DAG Example: Web Crawler

```
main function DAG:
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  fetch(url0) ──┐                                           │
│  fetch(url1) ──┼──→ parse_page ──→ filter_links ──┐      │
│  fetch(url2) ──┘                                       │      │
│                                                          │      │
│                     save_result ──→ print              │      │
│                          ↑                              │      │
│                          └──────────────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Node description:
┌──────────────────┬────────────┬────────────────────────────┐
│ Node              │ Side Effect│ Description               │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ Concurrent download         │
│ fetch(url1)      │ @IO       │ Concurrent download         │
│ fetch(url2)      │ @IO       │ Concurrent download         │
│ parse_page       │ @Pure     │ Parallel parsing            │
│ filter_links     │ @Pure     │ Parallel filtering          │
│ save_result      │ @IO       │ Sequential save (I/O order) │
│ print            │ @IO       │ Execute last                │
└──────────────────┴────────────┴────────────────────────────┘
```

### Scheduler Execution Phases

```
Phase 1: Concurrent downloads
─────────────────────────────────────────
Thread 1: fetch(url0) ──────────┐
Thread 2: fetch(url1) ─────────┼──→ 3 concurrent tasks (max concurrency limit)
Thread 3: fetch(url2) ──────────┘

Phase 2: Concurrent parsing
─────────────────────────────────────────
Thread 1: parse_page(page0) ──┐
Thread 2: parse_page(page1) ──┼──→ 3 concurrent tasks
Thread 3: parse_page(page2) ──┘

Phase 3: Concurrent filtering
─────────────────────────────────────────
Thread 1: filter_links(result0) ──┐
Thread 2: filter_links(result1) ──┼──→ 3 concurrent tasks
Thread 3: filter_links(result2) ──┘

Phase 4: Sequential save
─────────────────────────────────────────
Thread 1: save_result(result0) → wait for completion
Thread 1: save_result(result1) → wait for completion
Thread 1: save_result(result2) → wait for completion

Phase 5: Output
─────────────────────────────────────────
Thread 1: print("Fetched 3 pages")
```

## Detailed Design

### Module Structure

```
src/backends/llvm/
├── mod.rs           # Module entry + Executor implementation
├── context.rs       # LLVM context management
├── types.rs         # Type mapping (YaoXiang → LLVM)
├── values.rs        # Value mapping (registers → LLVM Value)
├── codegen.rs       # Core code generation
├── dag.rs           # DAG analysis and generation
├── scheduler.rs     # Runtime scheduler
└── tests.rs         # Tests
```

### Type Mapping

| YaoXiang Type | LLVM Type |
|---------------|----------|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `i1` |
| `String` | `ptr` (struct) |
| `Arc(T)` | `{ i32, T }` (reference-counted struct) |
| `ref T` | `ptr` (Arc pointer) |
| `List(T)` | `ptr` (dynamic array) |
| `Struct` | `struct` (corresponding struct) |

### Instruction Translation

Each `BytecodeInstr` is directly translated to corresponding LLVM IR instructions:

| BytecodeInstr | LLVM IR |
|---------------|---------|
| `BinaryOp { add }` | `llvm.add` |
| `CallStatic` | `llvm.call` |
| `ArcNew` | `call @Arc_new` |
| `LoadElement` | `llvm.getelementptr` + `llvm.load` |

### Runtime Library

```rust
// Core runtime functions
extern "C" {
    // Reference counting
    fn Arc_new(ptr: *mut u8) -> i32;
    fn Arc_clone(ref_count: *mut i32) -> i32;
    fn Arc_drop(ref_count: *mut i32);

    // Heap allocation
    fn Alloc(size: usize) -> *mut u8;
    fn Dealloc(ptr: *mut u8);

    // DAG scheduling
    fn dag_schedule(dag: *const DAGMetadata, entry: usize) -> RuntimeValue;
}
```

### Scheduling Strategies

| Annotation | Scenario | Scheduling Strategy |
|------------|----------|---------------------|
| `@auto` (default, L3) | Transparent concurrency | DAG lazy scheduling, execute in parallel when no dependencies |
| `@block` (L1) | Force synchronous | No DAG, purely serial execution |
| Cyclic dependency | Runtime detection | Error |

### Side Effect Handling: Implicit Effect System

User-transparent side effect handling, compiler automatically infers:

```
User code:
  print("a");
  print("b");
  let x = compute(1);
  let y = compute(2);

Compiler inference:
  print → @IO (external call)
  compute → @Pure (pure function)

Scheduler execution:
  print("a") ──→ sequential (all @IO)
  print("b") ──→ sequential
  compute(1) ─┬─→ parallel (DAG scheduling)
  compute(2) ─┘
```

### Relationship with Three-Layer Runtime

RFC-008 defines the Embedded / Standard / Full three-layer runtime architecture. The correspondence between the LLVM AOT compiler and the three-layer runtime:

| Runtime | LLVM AOT Behavior |
|---------|-------------------|
| **Embedded** | No DAG scheduling, directly generate sequential machine code |
| **Standard** | DAG + single-threaded scheduling (num_workers=1) |
| **Full** | DAG + multi-threaded scheduling (num_workers>1), supports WorkStealing |

### Scheduler Interface Design

```rust
/// Scheduling strategy
pub enum ScheduleStrategy {
    /// @block: force serial, no DAG
    Serial,
    /// @eager: eager evaluation, wait for dependencies to complete
    Eager,
    /// @auto (default): lazy scheduling, DAG automatic scheduling
    Lazy,
}

/// Side effect tag
pub enum EffectTag {
    /// Pure function, no side effects
    Pure,
    /// Has I/O side effects
    IO,
}

/// DAG scheduler trait
pub trait DAGScheduler: Send + Sync {
    /// Schedule execution (with strategy parameter)
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint], strategy: ScheduleStrategy) -> RuntimeValue;

    /// Execute single function
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## Trade-offs

### Advantages

1. **Performance improvement**: AOT compilation is 10-100x faster than interpreted execution
2. **Solves colored functions**: Default concurrency, synchronous is the special case
3. **Unified runtime**: Interpreter and LLVM share the same scheduler
5. **Implicit side effects**: User-transparent, compiler handles automatically
6. **Ownership safety**: Relies on Rust-style ownership model, no data races

### Disadvantages

1. **Implementation complexity**: Requires LLVM integration experience
2. **Compilation time**: AOT compilation is slower than interpreter
3. **Debugging difficulty**: AOT code is harder to debug than interpreter

### Consistency with RFC Designs

| RFC | Consistency |
|-----|-------------|
| RFC-001 Concurrency Model | ✅ DAG dependency analysis is core |
| RFC-008 Runtime Architecture | ✅ Runtime scheduler design consistent |
| RFC-009 Ownership Model | ✅ ARC runtime correctly implemented |

## Alternative Solutions

| Solution | Description | Why Not Chosen |
|----------|-------------|----------------|
| Interpreter only | No AOT needed | Insufficient performance, colored function problem |
| Pure static compilation | No runtime scheduling | Lazy scheduling needs runtime |
| Link external LLVM runtime | Use LLVM's runtime | Needs additional dependencies |

## Implementation Strategy

### Phase Division

#### Phase 1: Basic Framework (1-2 days)

- [ ] Add inkwell dependency to `Cargo.toml`
- [ ] Create `src/backends/llvm/` module
- [ ] Implement LLVM context initialization

#### Phase 2: Type Mapping (2-3 days)

- [ ] Implement `TypeMap`: YaoXiang types → LLVM types
- [ ] Basic types: i32, i64, f32, f64, bool
- [ ] Composite types: struct, array, tuple
- [ ] Special types: Arc, ref, Option

#### Phase 3: Instruction Translation (3-5 days)

- [ ] Implement `codegen_instruction()`
- [ ] Arithmetic instructions: add, sub, mul, div
- [ ] Control flow: jmp, jmp_if, ret
- [ ] Function calls: call, call_virt, call_dyn

#### Phase 4: DAG Collection (2-3 days)

- [ ] Collect DAG information during code generation
- [ ] Record function dependency relationships
- [ ] Side effect inference (@IO / @Pure)
- [ ] Generate DAG metadata

#### Phase 5: Runtime Library (3-5 days)

- [ ] Implement lazy scheduling
- [ ] Implement DAG scheduler
- [ ] Implement granularity control
- [ ] Implement ARC runtime

#### Phase 6: Integration and Testing (2-3 days)

- [ ] Link runtime library
- [ ] End-to-end testing
- [ ] Performance benchmarks

### Dependencies

- RFC-001: Concurrency Model (accepted)
- RFC-008: Runtime Concurrency Model (accepted)
- RFC-009: Ownership Model (accepted)

### Risks

1. **LLVM integration complexity**: Need deep understanding of inkwell API
2. **Scheduler and AOT code integration**: Need careful interface design
3. **ABI compatibility**: Need to ensure ABI compatibility with interpreter runtime

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute | Description |
|-----------|-------------|
| Institution | MIT |
| Authors | James R. Larus, Robert H. Halstead Jr. |
| Core | Lazily create subtasks, create on demand |
| Reference value | Technical foundation, origin of lazy scheduling concept |

**Core idea**: Don't create tasks immediately, but delay creation. Only create a subtask when the parent task needs its value. This solves the performance overhead problem of fine-grained parallel tasks[^1].

### Lazy Scheduling (2014)[^2]

| Attribute | Description |
|-----------|-------------|
| Institution | University of Maryland |
| Authors | Tzannes, Caragea |
| Core | Runtime adaptive scheduling, no extra state |
| Reference value | Scheduler design, adaptive granularity control |

**Core idea**: Automatically control granularity through "delayed execution", no need to maintain complex state. Tasks automatically merge when the system is busy and split when idle[^2].

### SISAL Language[^3]

| Attribute | Description |
|-----------|-------------|
| Institution | Lawrence Livermore National Laboratory (LLNL) |
| Core | Single-assignment language, Dataflow graph, implicit parallelism |
| Reference value | Feasibility proof, performance close to Fortran |

**Core contribution**: SISAL proved that the Dataflow model can achieve near-Fortran performance in industrial applications[^3].

### Mul-T Parallel Scheme[^4]

| Attribute | Description |
|-----------|-------------|
| Institution | MIT |
| Core | Future construct, Lazy Task Creation implementation |
| Reference value | Specific implementation reference |

**Core mechanism**:
```scheme
;; Multilisp / Mul-T syntax
(let ((a (future compute-a))      ;; return future immediately
      (b (future compute-b)))      ;; return future immediately
  (join a b))                      ;; wait for completion
```

### Comparison Summary

| Technology | Lazy Creation | DAG Analysis | Side Effect Handling | Ownership |
|------------|---------------|--------------|----------------------|-----------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (global) | N/A (single assignment) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (within functions) | ✅ (implicit) | ✅ (ARC) |

**YaoXiang's innovation**: Simplify traditional design with modern language features (ownership + implicit side effects), reduce complexity by keeping DAG constraints within function blocks.

## Comparison with Traditional Automatic Parallelization Methods

### Traditional Compilers: Loop-Level Parallelization

Commercial compilers (Intel Fortran, Oracle Fortran) use **loop-level automatic parallelization**[^5]:

**Core flow**:
```
1. Identify parallelizable loops
2. Perform dependency analysis on array accesses within loops
3. Determine if there are dependencies between loop iterations
4. If no dependencies, generate multi-threaded code
```

**Dependency analysis techniques**:

| Technique | Description |
|-----------|-------------|
| **Data dependency** | Whether two accesses reference the same memory location |
| **Use-Def** | Variable definition and use relationships |
| **Alias analysis** | Whether pointers reference the same memory |

**Conditions for loop parallelizability**:
```fortran
! Can be parallelized
DO I = 1, N
  A(I) = C(I)
END DO

! Cannot be parallelized (depends on previous iteration)
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell: Spark Mechanism

GHC (Glasgow Haskell Compiler) uses the **Spark mechanism** for pure function parallelization[^6]:

```haskell
-- rpar: execute in parallel, create spark
-- rseq: execute serially, wait for completion

example = do
  a <- rpar (f x)   -- create spark, execute f x in parallel
  b <- rpar (g y)   -- create spark, execute g y in parallel
  rseq a            -- wait for a to complete
  rseq b            -- wait for b to complete
  return (a, b)
```

**Spark pool mechanism**:
- Take sparks from the pool and assign to idle processing cores
- If a spark is unused (no one is waiting for the result), it gets garbage collected
- This solves the granularity problem: too-small sparks are discarded

### Clean Language: Uniqueness Types

The Clean language achieves parallel safety through **Uniqueness Types**[^7]:

```clean
-- *Array indicates uniqueness, can be safely modified
modify :: *Array Int -> *Array Int
```

**Core idea**: If a value has a single reference, it can be safely modified in a parallel environment because no other references will see intermediate state.

### Program Slicing and Dependency Graphs

**Program Dependency Graph (PDG)** is the foundation for parallelism detection:

```
Nodes: statements
Edges: data dependency + control dependency

Parallelism detection:
  If there is no reachable path between two nodes → can execute in parallel
```

### Comprehensive Comparison

| Method | Dependency Analysis | Granularity | Side Effect Handling | Typical Scenario |
|--------|---------------------|-------------|----------------------|------------------|
| Intel/Oracle Fortran[^5] | Complex array analysis | Loop iterations | N/A | Scientific computing |
| GHC Spark[^6] | Pure function assumption | Expressions | N/A | Functional programming |
| Clean[^7] | Uniqueness types | Graph rewriting | N/A | Functional programming |
| **YaoXiang** | Ownership guarantee | Function calls | Implicit inference | General purpose |

---

## Appendices

### Appendix A: Detailed Comparison with Rust async

| Feature | Rust async | YaoXiang LLVM AOT |
|---------|------------|-------------------|
| Compiled output | State machine + machine code | Machine code + DAG |
| Runtime | tokio | DAG Scheduler |
| Scheduling time | Compile-time determined await points | Runtime on-demand scheduling |
| Concurrency control | State machine states | DAG dependency edges |
| Colored functions | async spreads | **L3 transparent concurrency, @block is special case** |
| Annotations | async/await | @auto/@eager/@block |

### Appendix B: Scheduler Optimization Examples

**Scenario 1: Scheduler detects execution can be merged**

```
Original DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

After scheduler optimization:
  Merge compute_a + compute_b into a single task
  → Reduces scheduling overhead
```

**Scenario 2: Dependency not used**

```
let a = expensive_compute(); // computed
let b = other_thing();       // doesn't need a
print(b);                    // return b directly, skip a
```

### Appendix C: Design Discussion Log

| Decision | Resolution | Date |
|----------|------------|------|
| Adopt LLVM AOT | Direct codegen, no over-abstraction | 2026-02-15 |
| DAG scope | Within function blocks, not cross-function | 2026-02-15 |

| Execution model | **Bottom-up**: reverse analyze dependencies from results, leaf nodes parallel | 2026-03-10 |
| Isolated DAG | Nodes without consumers execute independently in parallel | 2026-03-10 |
| Infinite loops | Background DAG, scheduler slices execution | 2026-03-10 |
| Side effect handling | Implicit Effect System, transparent to user | 2026-02-15 |
| Granularity control | Concurrency limit + adaptive | 2026-02-16 |
| Paper citations | Added Lazy Task Creation, etc. | 2026-02-16 |

---

## References

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT. Retrieved from https://people.csail.mit.edu/riastradh/t/halstead90lazy-task.pdf

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland. Retrieved from https://user.eng.umd.edu/~barua/tzannes-TOPLAS-2014.pdf

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory. Retrieved from https://www.sciencedirect.com/science/article/abs/pii/074373159090035N

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT. Retrieved from https://link.springer.com/content/pdf/10.1007/bfb0024163.pdf

[^5]: Intel Corporation. *Automatic Parallelization with Intel Compilers*. Retrieved from https://www.intel.com/content/www/us/en/developer/articles/technical/automatic-parallelization-with-intel-compilers.html

[^6]: Marlow, S. (2010). *Parallel and Concurrent Programming in Haskell*. Retrieved from https://www.cse.chalmers.se/edu/year/2015/course/pfp/Papers/strategies-tutorial-v2.pdf

[^7]: Plasmeijer, R., & van Eekelen, M. (2011). *Clean Language Documentation*. University of Nijmegen. Retrieved from https://clean.cs.ru.nl/Documentation

- [Rust async book](https://rust-lang.github.io/async-book/)
- [inkwell LLVM bindings](https://cranelift.dev/)
- [tokio runtime design](https://tokio.rs/)
- [RFC-001: Concurrency Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |