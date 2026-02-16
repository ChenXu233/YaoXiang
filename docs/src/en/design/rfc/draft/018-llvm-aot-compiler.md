---
title: RFC-018: LLVM AOT Compiler with L3 Transparent Concurrency Design
---

# RFC-018: LLVM AOT Compiler with L3 Transparent Concurrency Design

> **Status**: Draft
> **Author**: ChenXu
> **Created Date**: 2026-02-15
> **Last Updated**: 2026-02-16

> **References**:
> - [RFC-001: Concurrent Model and Error Handling](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./accepted/009-ownership-model.md)

## Summary

This document designs the LLVM AOT compiler for the YaoXiang language, aiming to generate machine code + DAG metadata through ahead-of-time compilation, with runtime scheduler performing **lazy scheduling** based on DAG dependencies. This design fundamentally differs from Rust async/await + tokio: Rust determines await points at compile time, while YaoXiang performs on-demand scheduling at runtime[^1]. It follows RFC-001's L3 transparent concurrency design: default @auto (automatic parallelism), @block sync as a special case, solving the color function problem.

## Motivation

### Why LLVM AOT Compiler?

Currently, YaoXiang only has an interpreter as the execution backend, leading to:

| Problem | Impact |
|---------|--------|
| Performance bottleneck | Interpreted execution is 10-100x slower than machine code |
| Complex deployment | Requires interpreter and runtime |
| Color function problem | Sync functions cannot call concurrent functions |

### Color Function Problem and L3 Transparent Concurrency

**Traditional design (current)**:
- Sync functions (blue) → cannot call → concurrent functions (red)
- Sync is default, concurrency requires `spawn` marking
- Color "contaminates": once concurrency is used, entire call chain becomes concurrent

**RFC-001 L3 transparent concurrency (target)**:
- L3: Default transparent concurrency (@auto)
- L2: Explicit spawn concurrency
- L1: @block sync mode

**Flipped design (RFC-018)**:
- Default L3 transparent concurrency, compile-time automatic DAG dependency analysis
- Solving color function problem: sync functions can directly call "default concurrent" code
- @block as special case only for forced serial execution

### Core Innovation: Lazy Scheduling

The core innovation of this design is **Lazy Scheduling**[^2]:

```
Traditional function call:
  call fetch(url) → execute → return result

Lazy scheduling:
  call fetch(url) → skip execution, suspend function → record "needs execution"
                  → continue to next line
                  → execute when result is needed
```

**Key differences**:
- No "lazy iterator" or other intermediate data structures returned
- Directly suspends function, no extra cycle overhead
- Scheduler triggers execution on-demand

### Comparison with Rust Async

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async model                            │
├─────────────────────────────────────────────────────────────────┤
│  Compile time: generate state machine + machine code            │
│  Runtime: tokio scheduler based on state machine               │
│  Features: await points determined at compile time             │
│  Granularity: function level                                    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT model                   │
├─────────────────────────────────────────────────────────────────┤
│  Compile time: generate machine code + DAG metadata            │
│  Runtime: DAG scheduler based on dependency lazy scheduling    │
│  Features: calls execute when needed, on-demand scheduling      │
│  Granularity: DAG within function block                         │
└─────────────────────────────────────────────────────────────────┘
```

## Proposal

### Core Design

```
┌─────────────────────────────────────────────────────┐
│  Compile Time                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG分析  │→│LLVM Codegen│→ 机器码  │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              Generate: DAG metadata                   │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  Runtime                                             │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG Scheduler Library                        │ │
│  │  • Load machine code                        │ │
│  │  • Read DAG metadata                        │ │
│  │  • Lazy scheduling: suspend calls, execute │ │
│  │    on-demand                               │ │
│  │  • Support parallel/serial execution        │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Lazy Scheduling Execution Flow

```
Phase 1: Traverse function body, suspend calls
─────────────────────────────────────────
Encounter fetch(url0) → skip (suspend), record "needs execution"
Encounter fetch(url1) → skip (suspend), record "needs execution"
Encounter fetch(url2) → skip (suspend), record "needs execution"
    ↓
At this point, no fetch executes, just build pending execution list

Phase 2: Parallel execution (control parallelism)
─────────────────────────────────────────
Scheduler takes tasks from pending list
    ↓
    ↓ Control parallelism (e.g., 16)
    ↓
Execute fetch(url0), fetch(url1), ... fetch(url15)

Phase 3: Trigger when value is needed
─────────────────────────────────────────
When parse_page(page0) needs page0
    ↓
Check if page0 is ready
    ↓
Ready → execute parse_page
Not ready → wait, then continue
```

### Compiled Artifact Structure

```rust
/// Compiled artifact: machine code + DAG metadata
pub struct CompiledArtifact {
    /// LLVM compiled machine code (ELF/Mach-O/COFF)
    machine_code: Vec<u8>,

    /// DAG metadata: describes function dependencies
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
    /// Dependency node IDs
    pub deps: Vec<usize>,
    /// Effect tag (@IO / @Pure)
    pub effect: EffectTag,
}
```

### Runtime Scheduler Interface

```rust
/// DAG scheduler trait
pub trait DAGScheduler: Send + Sync {
    /// Schedule execution
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue;

    /// Single function execution
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
            max_parallelism: num_workers * 2, // adaptive granularity control
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. Traverse function body, suspend all calls
        // 2. Build pending execution task list
        // 3. Schedule execution in dependency order (control parallelism)
        // 4. Trigger execution when value is needed
        // 5. Return result
    }
}
```

### Syntax Design (Unified: name: type = expression)

```yaoxiang
# Variable
x: Int = 42

# Function (parameter names in signature)
add: (a: Int, b: Int) -> Int = a + b

# Main function (default @auto concurrency)
main: (urls: Vec[String]) -> () = {
    # Download all pages concurrently (lazy scheduling)
    let pages = urls.map(|url| fetch(url));

    # Parse all pages concurrently
    let results = pages.map(|page| parse_page(page));

    # Filter links (pure function, can parallel)
    let all_links = results.flat_map(|r| filter_links(r.links));

    # Save sequentially (@IO ensures order)
    for result in results {
        save_result(result);
    }

    print(`Fetched ${results.len()} pages`);
}

# Pure function: parse page
parse_page: (page: Page) -> Result = {
    title = extract_title(page.content);
    links = extract_links(page.content);
    Result { title, links }
}

# Pure function: filter valid links
filter_links: (links: Vec[String]) -> Vec[String] =
    links.filter(|l| l.starts_with("http"))

# External I/O: download page (implicit @IO, user unaware)
fetch: (url: String) -> Page = {
    content = http_get(url);
    Page { url, content }
}

# External I/O: save result
save_result: (result: Result) -> () = {
    database.save(result);
}

# L2 explicit concurrency
spawn_main: () -> () = {
    spawn { fetch(url0) };
    spawn { fetch(url1) };
}

# L1 forced serial
serial_main: () -> () = {
    block {
        db.begin();
        db.write(data1);
        db.write(data2);
        db.commit();
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

Node descriptions:
┌──────────────────┬────────────┬────────────────────────────┐
│ Node              │ Effect    │ Description               │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ Concurrent download       │
│ fetch(url1)      │ @IO       │ Concurrent download       │
│ fetch(url2)      │ @IO       │ Concurrent download       │
│ parse_page       │ @Pure     │ Concurrent parsing        │
│ filter_links     │ @Pure     │ Concurrent filtering      │
│ save_result      │ @IO       │ Sequential save (I/O)     │
│ print            │ @IO       │ Final execution           │
└──────────────────┴────────────┴────────────────────────────┘
```

### Scheduler Execution Phases

```
Phase 1: Concurrent Download
─────────────────────────────────────────
Thread1: fetch(url0) ──────────┐
Thread2: fetch(url1) ─────────┼──→ 3 concurrent tasks (max parallelism)
Thread3: fetch(url2) ──────────┘

Phase 2: Concurrent Parsing
─────────────────────────────────────────
Thread1: parse_page(page0) ──┐
Thread2: parse_page(page1) ──┼──→ 3 concurrent tasks
Thread3: parse_page(page2) ──┘

Phase 3: Concurrent Filtering
─────────────────────────────────────────
Thread1: filter_links(result0) ──┐
Thread2: filter_links(result1) ──┼──→ 3 concurrent tasks
Thread3: filter_links(result2) ──┘

Phase 4: Sequential Save
─────────────────────────────────────────
Thread1: save_result(result0) → wait
Thread1: save_result(result1) → wait
Thread1: save_result(result2) → wait

Phase 5: Output
─────────────────────────────────────────
Thread1: print("Fetched 3 pages")
```

## Detailed Design

### Module Structure

```
src/backends/llvm/
├── mod.rs           # Module entry + Executor implementation
├── context.rs       # LLVM context management
├── types.rs         # Type mapping (YaoXiang → LLVM)
├── values.rs        # Value mapping (register → LLVM Value)
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
| `Arc[T]` | `{ i32, T }` (reference counted struct) |
| `ref T` | `ptr` (Arc pointer) |
| `List[T]` | `ptr` (dynamic array) |
| `Struct` | `struct` (corresponding struct) |

### Instruction Translation

Each `BytecodeInstr` directly translates to corresponding LLVM IR:

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

### Scheduling Strategy

| Annotation | Scenario | Scheduling Strategy |
|------------|----------|---------------------|
| `@auto` (default, L3) | Transparent concurrency | DAG lazy scheduling, parallel execution of independent nodes |
| `@eager` | Eager evaluation | Wait for dependencies, then execute, ensure order |
| `@spawn` (L2) | Manual concurrency | Force background execution |
| `@block` (L1) | Forced sync | No DAG, pure serial execution |
| Cyclic dependency | Runtime detection | Error |

### Side Effect Handling: Implicit Effect System

User-unaware side effect handling, compiler infers automatically:

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
  print("a") ──→ sequential (both @IO)
  print("b") ──→ sequential
  compute(1) ─┬─→ parallel (DAG scheduling)
  compute(2) ─┘
```

### Granularity Control: Solving Task Explosion

When too many `fetch` tasks exist, creating无数 concurrent tasks may cause stalling. Solutions:

**1. Parallelism Limit**

```rust
// Scheduler strategy
scheduler.max_parallelism = num_cores * 2; // e.g., 8 cores = 16 parallel
```

**2. Adaptive Granularity**[^2]

```
High load: merge multiple small tasks into one large task
Low load: maintain fine-grained parallelism
```

**3. Lazy Task Creation**[^1]

```
Traditional eager creation:
  urls.map(|url| fetch(url))
  → immediately creates 10000 tasks

Lazy creation:
  Only create task when value is needed
  → memory usage O(1) or O(parallelism)
```

### Relationship with Three-Tier Runtime

RFC-008 defines Embedded / Standard / Full three-tier runtime architecture. Correspondence with LLVM AOT:

| Runtime | LLVM AOT Behavior |
|---------|-------------------|
| **Embedded** | No DAG scheduling, direct sequential machine code |
| **Standard** | DAG + single-threaded scheduling (num_workers=1) |
| **Full** | DAG + multi-threaded scheduling (num_workers>1), WorkStealing support |

### Scheduler Interface Design

```rust
/// Scheduling strategy
pub enum ScheduleStrategy {
    /// @block: forced serial, no DAG
    Serial,
    /// @eager: eager evaluation, wait for dependencies
    Eager,
    /// @auto (default): lazy scheduling, DAG auto scheduling
    Lazy,
}

/// Effect tag
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

    /// Single function execution
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## Trade-offs

### Advantages

1. **Performance improvement**: AOT compilation 10-100x faster than interpretation
2. **Solving color function**: Default concurrency, sync is special case
3. **Unified runtime**: Interpreter and LLVM share same scheduler
4. **Lazy scheduling**: Calls don't execute immediately, on-demand scheduling[^1][^2]
5. **Implicit side effects**: User unaware, compiler handles automatically
6. **Ownership safety**: Relies on Rust-style ownership model, no data races

### Disadvantages

1. **Implementation complexity**: Requires LLVM integration experience
2. **Compilation time**: AOT compilation slower than interpreter
3. **Debugging difficulty**: AOT code debugging more complex than interpreter

### Consistency with RFC Design

| RFC | Consistency |
|-----|-------------|
| RFC-001 Concurrent Model | ✅ DAG dependency analysis is core |
| RFC-008 Runtime Architecture | ✅ Runtime scheduler design consistent |
| RFC-009 Ownership Model | ✅ ARC runtime correctly implemented |

## Alternative Approaches

| Approach | Description | Why Not Chosen |
|----------|-------------|----------------|
| Interpreter only | No AOT needed | Insufficient performance, color function problem |
| Pure static compilation | No runtime scheduling | Lazy scheduling requires runtime |
| Link external LLVM runtime | Use LLVM's runtime | Extra dependency |

## Implementation Strategy

### Phase Breakdown

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
- [ ] Record function dependencies
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
- [ ] Performance benchmarking

### Dependencies

- RFC-001: Concurrent model (accepted)
- RFC-008: Runtime concurrency model (accepted)
- RFC-009: Ownership model (accepted)

### Risks

1. **LLVM integration complexity**: Requires deep understanding of inkwell API
2. **Scheduler and AOT code integration**: Requires carefully designed interfaces
3. **ABI compatibility**: Need to ensure ABI compatibility with interpreter runtime

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute | Description |
|-----------|-------------|
| Institution | MIT |
| Authors | James R. Larus, Robert H. Halstead Jr. |
| Core | Lazy child task creation, create on-demand |
| Reference Value | Technical foundation, origin of lazy scheduling concept |

**Core idea**: Instead of immediately creating tasks, create lazily. When a parent task needs the child task's value, only then is the child task created. This solves the performance overhead problem of fine-grained parallel tasks[^1].

### Lazy Scheduling (2014)[^2]

| Attribute | Description |
|-----------|-------------|
| Institution | University of Maryland |
| Authors | Tzannes, Caragea |
| Core | Runtime adaptive scheduling, no extra state |
| Reference Value | Scheduler design, adaptive granularity control |

**Core idea**: Automatically controls granularity through "lazy execution", without maintaining complex state. When the system is busy, tasks automatically merge; when idle, they split[^2].

### SISAL Language[^3]

| Attribute | Description |
|-----------|-------------|
| Institution | Lawrence Livermore National Laboratory (LLNL) |
| Core | Single-assignment language, Dataflow graph, implicit parallelism |
| Reference Value | Feasibility proof, performance close to Fortran |

**Key contribution**: SISAL proved that Dataflow models can achieve near-Fortran performance in industrial applications[^3].

### Mul-T Parallel Scheme[^4]

| Attribute | Description |
|-----------|-------------|
| Institution | MIT |
| Core | Future construct, Lazy Task Creation implementation |
| Reference Value | Implementation reference |

**Core mechanism**:
```scheme
;; Multilisp / Mul-T syntax
(let ((a (future compute-a))      ;; immediately returns future
      (b (future compute-b)))      ;; immediately returns future
  (join a b))                      ;; wait for completion
```

### Comparison Summary

| Technique | Lazy Creation | DAG Analysis | Side Effect | Ownership |
|-----------|---------------|--------------|--------------|-----------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (global) | N/A (single-assignment) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (function-scoped) | ✅ (implicit) | ✅ (ARC) |

**YaoXiang's innovation**: Uses modern language features (ownership + implicit side effects) to simplify traditional design, constraining DAG within function blocks to reduce complexity.

## Comparison with Traditional Automatic Parallelization

### Traditional Compilers: Loop-Level Parallelization

Commercial compilers (like Intel Fortran, Oracle Fortran) use **loop-level automatic parallelization**[^5]:

**Core flow**:
```
1. Identify parallelizable loops
2. Perform dependency analysis on array accesses within loops
3. Determine if loop iterations have dependencies
4. If no dependency, generate multithreaded code
```

**Dependency analysis techniques**:

| Technique | Description |
|-----------|-------------|
| **Data Dependence** | Whether two accesses reference same memory location |
| **Use-Def** | Definition and use relationships of variables |
| **Alias Analysis** | Whether pointers reference same memory |

**Conditions for loop parallelization**:
```fortran
! Can parallelize
DO I = 1, N
  A(I) = C(I)
END DO

! Cannot parallelize (depends on previous iteration)
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell: Spark Mechanism

GHC (Glasgow Haskell Compiler) uses **Spark mechanism** for pure function parallelism[^6]:

```haskell
-- rpar: execute in parallel, create spark
-- rseq: execute sequentially, wait for completion

example = do
  a <- rpar (f x)   -- create spark, execute f x in parallel
  b <- rpar (g y)   -- create spark, execute g y in parallel
  rseq a            -- wait for a to complete
  rseq b            -- wait for b to complete
  return (a, b)
```

**Spark pool mechanism**:
- Take sparks from pool, assign to idle cores
- If spark unused (no one waits for result), garbage collected
- Solves granularity problem: too-small sparks get discarded

### Clean Language: Uniqueness Types

Clean language achieves parallel safety through **Uniqueness Types**[^7]:

```clean
-- *Array indicates uniqueness, can safely modify
modify :: *Array Int -> *Array Int
```

**Core idea**: If a value is uniquely referenced, it can be safely modified in a parallel environment because no other reference will see intermediate state.

### Program Slicing and Dependence Graphs

**Program Dependence Graph (PDG)** is the foundation of parallelism detection:

```
Nodes: statements
Edges: data dependence + control dependence

Parallelism detection:
  If no path between two nodes → can execute in parallel
```

### Comprehensive Comparison

| Method | Dependency Analysis | Granularity | Side Effect Handling | Typical Scenario |
|--------|---------------------|-------------|----------------------|------------------|
| Intel/Oracle Fortran[^5] | Complex array analysis | Loop iteration | N/A | Scientific computing |
| GHC Spark[^6] | Pure function assumption | Expression | N/A | Functional programming |
| Clean[^7] | Uniqueness types | Graph rewriting | N/A | Functional programming |
| **YaoXiang** | Ownership guarantee | Function call | Implicit inference | General purpose |

### Unique Advantages of Your Design

**YaoXiang's advantages over traditional approaches**:

```
1. Simplified dependency analysis
   Traditional: requires complex pointer/alias analysis, conservative (serialize if uncertain)
   YaoXiang: ownership guarantees safety, no complex analysis needed

2. Coarser-grained parallelism
   Traditional: loop iteration level, requires precise analysis
   YaoXiang: function call level, DAG scheduling

3. Implicit side effect handling
   Traditional: requires manual marking or assumes pure functions
   YaoXiang: compiler infers automatically, user unaware

4. Function-level scope
   Traditional: global dependence graph, high complexity
   YaoXiang: DAG constrained within function blocks, reduced complexity
```

## Open Questions

- [ ] Does DAG metadata format need versioning?
- [ ] Does it support incremental AOT compilation?
- [ ] How to handle FFI calls?
- [ ] Performance benchmarking plan?

---

## Appendices

### Appendix A: Detailed Comparison with Rust Async

| Feature | Rust async | YaoXiang LLVM AOT |
|---------|-----------|-------------------|
| Compiled artifact | State machine + machine code | Machine code + DAG |
| Runtime | tokio | DAG Scheduler |
| Scheduling timing | Await points determined at compile time | On-demand at runtime |
| Concurrency control | State machine states | DAG dependency edges |
| Color function | async contaminates | **L3 transparent concurrency, @block special case** |
| Annotations | async/await | @auto/@eager/@block |

### Appendix B: Scheduler Optimization Examples

**Scenario 1: Scheduler detects tasks can be merged**

```
Original DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

After scheduler optimization:
  Merge compute_a + compute_b into single task
  → reduces scheduling overhead
```

**Scenario 2: Dependency not used**

```
let a = expensive_compute(); // computed
let b = other_thing();       // doesn't need a
print(b);                    // directly returns b, skips a
```

### Appendix C: Design Discussion Records

| Decision | Choice | Date |
|----------|--------|------|
| Use LLVM AOT | Direct codegen, no excessive abstraction | 2026-02-15 |
| DAG scope | Within function block, not cross-function | 2026-02-15 |
| Lazy scheduling | Skip execution, suspend function, schedule on-demand | 2026-02-15 |
| Side effect handling | Implicit Effect System, user unaware | 2026-02-15 |
| Granularity control | Parallelism limit + adaptive | 2026-02-16 |
| Paper references | Added Lazy Task Creation etc. | 2026-02-16 |

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
- [RFC-001: Concurrent Model](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime Concurrency Model](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |
