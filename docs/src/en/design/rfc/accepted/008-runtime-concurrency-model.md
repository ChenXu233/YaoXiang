---
title: 'RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design'
status: 'Accepted'
author: 'Chenxu'
created: '2025-01-05'
updated: '2026-07-05 (Aligned with RFC-024, added Issue references)'
issue: '#89'
issues_impl:
  - '#50'
  - '#89'
pr_impl:
  - '#7'
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **⚠️ Alignment Note**: This document has been aligned with
> [RFC-024 New Concurrency Model](../../../reference/language-spec/concurrency.md). The old
> whole-program DAG analysis, `@block`/`@eager` annotations, and L1/L2/L3 hierarchy model have been
> replaced by the `spawn {}` block parallel primitive. DAG analysis now applies only within
> `spawn {}` blocks.

> **References**:
>
> - [RFC-011: Generics System Design](./011-generic-type-system.md)
> - [Concurrency Model Specification (RFC-024)](../../../reference/language-spec/concurrency.md)

## Summary

This document defines the key designs of the Runtime architecture:

1. **Three-Tier Runtime Architecture**: Embedded (immediate execution) → Standard (spawn + DAG
   scheduling) → Full (work stealing)
2. **Compile and Run Separation**: The compilation phase is identical; differences lie only in the
   runtime execution
3. **Dual Backend Model**: VM (development/debugging) and LLVM AOT (production release) with
   identical behavior
4. **Scheduler = Static Library**: The scheduler is linked into the exe during AOT compilation,
   ~200-500KB, no GC
5. **Synchronization Is Just a Special Case of Scheduling**: num_workers=1 means synchronous mode

### Key Clarification: This Is Not Java

```
Java:   .java → .class → JVM (Interpret/JIT + GC)         ← Always needs a virtual machine
YaoXiang Dev:     .yx → IR → VM execution (fast iteration, step debugging)
YaoXiang Prod:    .yx → IR → LLVM → native exe (scheduler linked in)

VM is a development tool, not a runtime essence. Just like Go's go run vs go build.
The final exe = your native code + scheduler static library + reflection metadata. No interpreter, no JIT, no GC.
```

## Motivation

### Core Contradictions

| Contradiction                   | Description                                                                                                      |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Transparency vs Controllability | spawn blocks provide explicit concurrency control; regular code executes sequentially                            |
| Core vs Optional                | spawn is a core parallel primitive; WorkStealing is an advanced feature when num_workers>1                       |
| Single-threaded vs Concurrent   | In single-threaded mode concurrency behaves asynchronously; synchronization is just a special case of scheduling |

---

## Proposal

### 1. Three-Tier Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│              Compilation Phase (Same for All Modes)               │
│                                                                  │
│  Source Code → Lexer → Parser → TypeCheck → Codegen → IR        │
│                                                                  │
│  ⚠️ Same syntax parsing, type checking, code generation, IR output│
└──────────────────────────────────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
│ 🟢 Embedded      │ │ 🔵 Standard   │ │ 🟣 Full          │
│ Immediate Exec.  │ │ spawn + DAG   │ │ Full Scheduler   │
│ Sync Execution   │ │ In-block Conc.│ │ Parallel Opt.    │
│ No spawn support │ │ Auto-conc.    │ │ Work stealing    │
└──────────────────┘ └───────────────┘ └──────────────────┘
```

| Phase                  | Embedded | Standard                   | Full                           |
| ---------------------- | -------- | -------------------------- | ------------------------------ |
| Compilation            | Same     | Same                       | Same                           |
| Execution Mode         | Sync     | In-spawn-block concurrency | Parallel                       |
| Memory Usage           | Low      | Medium                     | High                           |
| Concurrency Capability | None     | Within spawn blocks        | Within spawn blocks + Parallel |
| spawn Support          | ❌       | ✅                         | ✅                             |
| DAG Analysis           | None     | Within spawn blocks        | Within spawn blocks            |
| WorkStealer            | None     | None                       | ✅                             |

- **Embedded Runtime**: Targets WASM/game scripts/rule engines. Immediate executor, no spawn
  support, high performance with low footprint.
- **Standard Runtime**: Targets web services/data pipelines. Supports `spawn {}` blocks, performing
  DAG analysis and automatic concurrency within spawn blocks. num_workers=1 means single-threaded
  asynchronous.
- **Full Runtime**: Targets scientific computing/massive parallelism. Standard + WorkStealer load
  balancing.

### 2. Scheduler Decoupling: Generics + Injection

Core principle: The VM does not directly depend on a concrete scheduler; it uses a generic parameter
`[S]` for invocation.

```yaoxiang
# 调度器接口定义
Scheduler: Type = {
    spawn: (Task) -> TaskId,
    await: (TaskId) -> Result,
    spawn_with_deps: (Task, List(TaskId)) -> TaskId,
    await_all: (List(TaskId)) -> List(Result),
    stats: () -> SchedulerStats,
}

# 单线程调度器
SingleThreadScheduler: Scheduler = {
    spawn: (task) => { task_queue.push(task); generate_task_id() },
    await: (task_id) => { ... },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { queue_size: task_queue.len() },
}

# 多线程调度器
MultiThreadScheduler: Scheduler = {
    spawn: (task) => { work_queue.push(task); generate_task_id() },
    await: (task_id) => { wait_for_completion(task_id) },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { workers: get_worker_stats() },
}

# VM 通过泛型使用调度器
create_vm: [S: Scheduler](scheduler: S) -> VM = (scheduler) => {
    VM(scheduler: scheduler, memory: create_memory(), dag: create_dag())
}
```

**Key Points**:

- Compile-time polymorphism, zero runtime overhead
- No Trait objects needed
- Generic type constraint `[S: Scheduler]` is already defined in RFC-011

### 3. Synchronization = Special Case of Scheduling

```
❌ Misconception: Disable the scheduler
✅ Correct: Use a single-worker scheduler

num_workers = 1 → Single-threaded asynchronous scheduling
num_workers > 1 → Multi-threaded parallel scheduling

The same scheduler interface, just different configuration. Eliminate special cases.
```

### 4. Role of DAG

> **Important Change**: DAG analysis no longer applies to the whole program; it occurs only within
> `spawn {}` blocks. Regular code (outside spawn blocks) executes sequentially and does not require
> DAG analysis.

| Tier             | spawn Support | DAG Analysis Scope  | Description                         |
| ---------------- | ------------- | ------------------- | ----------------------------------- |
| Core Runtime     | ✅            | Within spawn blocks | Concurrency core                    |
| Standard Runtime | ✅            | Within spawn blocks | spawn + DAG scheduling              |
| Embedded Runtime | ❌            | None                | Immediate execution, no concurrency |

### 5. Bottom-up Execution Model (Within spawn blocks)

> **Important Change**: Bottom-up DAG analysis is only performed within `spawn {}` blocks;
> whole-program DAG analysis is no longer performed.

```
User code (concurrency within spawn block):
    (a, b) = spawn {
        fetch(url0),
        fetch(url1)
    }
    print(a)

Compile-time analysis (bottom-up within spawn block):
    fetch(url0) and fetch(url1) have no mutual dependency → can run in parallel
    print(a) outside spawn block → runs sequentially, waiting for spawn to complete

Runtime scheduling (starts from leaves within spawn block):
    fetch(url0) ┐
                ├→ run in parallel
    fetch(url1) ┘
    print(a)                       ← outside spawn block, runs sequentially
```

**Key Points**:

- Bottom-up dependency analysis is limited to within `spawn {}` blocks
- Tasks without dependencies within a spawn block execute in parallel
- Code outside spawn blocks executes sequentially, waiting for the spawn block to complete

---

### 6. Compilation Model: Dual Backend + Static-Linked Runtime

#### 6.1 Two Backends, One Behavior

```
                      ┌─────────────────────┐
                      │   Compilation Front  │
                      │      (Unified)       │
                      │   Lexer → Parser     │
                      │   → TypeCheck        │
                      │   → In-spawn DAG An. │
                      │   → Escape Analysis  │
                      │   → Cycle Detection  │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │  VM Backend (Dev) │     │LLVM Backend (Prod)│
        │                   │     │                   │
        │  Generates IR/BC  │     │ Generates native  │
        │  VM Interprets    │     │ Links runtime lib │
        │  Step Debugging   │     │ Outputs .exe      │
        │  Fast Iteration   │     │ Zero interp. ovh. │
        └───────────────────┘     └───────────────────┘
                 │                         │
                 ▼                         ▼
         Identical Behavior         Identical Behavior
```

**VM Backend**: Used during development. Edit code → run immediately → step debug → fast iteration.
Behavior is completely identical to the final exe.

**LLVM Backend**: Used for release. AOT compile to native code; the scheduler is linked in as a
static library. No interpreter, no JIT.

#### 6.2 Scheduler = Static Library, Not a VM

```
Internal structure of the final exe:

┌────────────────────────────────────────────┐
│  Your Code (Native Machine Code)            │
│  ├── Compile-time determined DAG exec plan │
│  ├── Inlined Move/ref/clone operations     │
│  └── RAII release code                     │
├────────────────────────────────────────────┤
│  Runtime Static Library (~200-500KB)        │
│  ├── Thread pool (fixed size = num_workers) │
│  ├── Event loop (libuv / io_uring)         │
│  ├── Work-stealing queue (Full only)        │
│  ├── Memory allocator (jemalloc / mimalloc)│
│  └── Reflection metadata (lazy-loaded)     │
├────────────────────────────────────────────┤
│  Not included:                              │
│  ❌ Bytecode interpreter                    │
│  ❌ JIT compiler                            │
│  ❌ GC                                      │
│  ❌ Virtual machine                         │
└────────────────────────────────────────────┘
```

Comparison:

| Language           | Java               | Go                 | YaoXiang                       |
| ------------------ | ------------------ | ------------------ | ------------------------------ |
| Compilation Output | Bytecode           | Native code        | Native code                    |
| Execution          | JVM Interpret/JIT  | Direct             | Direct                         |
| Runtime Size       | ~200MB (JVM)       | ~1-2MB (incl. GC)  | **~200-500KB (no GC)**         |
| Memory Mgmt        | GC                 | GC                 | **RAII (Deterministic)**       |
| Reflection         | Resident in memory | Resident in memory | **Stored in exe, lazy-loaded** |

#### 6.3 Why Scheduler Performance Is Constant

**Key Insight**: Most of the work is done at compile-time; the runtime only "executes".

```
Compile-time (one-time, not in runtime):
    ├── Analyze DAG within spawn blocks: who depends on whom
    ├── Topological sort: determine exec order within spawn blocks
    ├── Identify parallelizable tasks: dependency-free subtrees in spawn blocks
    ├── Escape analysis: ref → Rc or Arc
    ├── Cycle detection: auto-degrade to Weak or error
    └── Inlining: small functions expanded directly

Runtime (per execution, fixed data structures):
    ├── Dispatch tasks to thread pool per compile-time determined spawn block DAG order
    ├── Encounter I/O → suspend current task, event loop takes over
    ├── Task ready → put back in ready queue
    └── That's it.
```

**The scheduler itself is a fixed-size data structure**: thread pool, event loop, work queue. No
dynamic growth, no adaptive re-optimization, no GC scanning. Behavior is completely predictable.

The compile-time has already computed "what to schedule" within spawn blocks; the runtime only
"executes". This differs from tokio—tokio dynamically builds Future chains at runtime. YaoXiang's
DAG is static, and limited to within spawn blocks.

#### 6.4 Reflection: Stored, Not Resident

Reflection metadata is generated at compile-time and stored in a dedicated section of the exe. It is
not loaded at program startup. On the first reflection request, it is mmap'd into memory on demand.
Similar to:

```
exe layout:
  .text     ← Your code
  .rodata   ← Constants
  .reflect  ← Reflection metadata (type info, function signatures, etc.)
              mmap lazy-loaded; no memory if not accessed
```

**Trade-off**: exe size increases (contains reflection data), but there is zero memory overhead if
not accessed at runtime. The first access has a loading delay (similar to JIT warmup), and zero
overhead thereafter.

```
src/
├── lib.rs
├── main.rs
├── backends/                          # Runtime backend
│   ├── common/                        # Shared by all backends (values, heap, opcodes)
│   │   ├── allocator.rs
│   │   ├── heap.rs
│   │   ├── opcode.rs
│   │   └── value.rs
│   ├── dev/                           # REPL + Debugger
│   │   ├── debugger.rs
│   │   ├── shell.rs
│   │   └── repl/
│   ├── interpreter/                   # 🟢 Tree-walking interpreter (formerly Embedded/VM)
│   │   ├── ffi.rs
│   │   ├── frames.rs
│   │   ├── registers.rs
│   │   ├── runtime.rs
│   │   └── executor/
│   └── runtime/                       # 🔵 Compiled VM runtime
│       ├── engine.rs
│       ├── facade.rs
│       └── task.rs
├── frontend/                          # Compilation frontend (shared by all backends)
│   ├── compiler.rs
│   ├── config.rs
│   ├── pipeline.rs
│   ├── core/
│   │   ├── lexer/
│   │   ├── parser/
│   │   ├── typecheck/
│   │   │   ├── checker.rs
│   │   │   ├── spawn_placement.rs     # ★ In-spawn-block DAG/concurrency analysis (formerly frontend/dag/)
│   │   │   ├── inference/
│   │   │   └── traits/
│   │   └── types/
│   ├── events/
│   ├── module/
│   └── pipeline/
├── middle/                            # Middle-end
│   ├── core/                          # IR & Bytecode
│   │   ├── bytecode.rs
│   │   ├── ir.rs                      #   IR definition (shared by VM and LLVM)
│   │   └── ir_gen.rs
│   └── passes/                        # Compilation passes
│       ├── codegen/                   # Code generation (formerly codegen/)
│       ├── lifetime/                  # Lifetime/borrow analysis
│       └── mono/                      # Monomorphization
├── lsp/                               # Language Server
├── formatter/                         # Source code formatter
├── package/                           # Package manager
├── std/                               # Standard library
│   ├── concurrent.rs
│   ├── io.rs
│   ├── list.rs
│   ├── math.rs
│   ├── net.rs
│   ├── string.rs
│   └── weak.rs
└── util/                              # Utility library
    ├── diagnostic/
    ├── i18n/
    └── config/
```

**Directory mapping notes** (old → new):

| Old Directory   | New Location                                 | Description                                                        |
| --------------- | -------------------------------------------- | ------------------------------------------------------------------ |
| `frontend/dag/` | `frontend/core/typecheck/spawn_placement.rs` | In-spawn-block DAG analysis has been integrated into type checking |
| `codegen/`      | `middle/passes/codegen/`                     | Code generation moved into middle-end passes                       |
| `embedded/`     | `backends/interpreter/`                      | Tree-walking interpreter                                           |
| `runtime/`      | `backends/runtime/`                          | Compiled VM runtime                                                |
| `vm/`           | `backends/interpreter/`                      | Merged with embedded                                               |
| `full/`         | (Not implemented)                            | Full Runtime + work stealing, for future versions                  |
| `reflect/`      | (Not implemented)                            | Reflection metadata, for future versions                           |
| `core/`         | `backends/common/`                           | Shared values/heap/opcodes                                         |

---

## Trade-offs

### Advantages

- **Clear layering**: Three tiers Embedded / Standard / Full
- **Compile reuse**: Frontend code is fully shared
- **Generics decoupling**: Compile-time polymorphism, zero overhead
- **Consistency**: Synchronization is just a special case of scheduling
- **Embedded-friendly**: High performance + low memory + fast startup

### Disadvantages

- **Initial complexity**: Need to define scheduler interface and multiple runtime variants
- **Compile-time binding**: Scheduler type is determined at compile-time

---

## Design Decision Records

| Decision                       | Determination                                                     | Date       |
| ------------------------------ | ----------------------------------------------------------------- | ---------- |
| Scheduler Decoupling Approach  | Generics + Injection                                              | 2025-01-05 |
| Single-threaded Mode           | Synchronization is a special case of scheduling                   | 2025-01-05 |
| Async Implementation           | DAG naturally supports it                                         | 2025-01-05 |
| WorkStealer                    | Full Runtime advanced feature                                     | 2025-01-05 |
| Embedded Design                | Immediate execution, no DAG scheduling                            | 2025-01-05 |
| Compilation Phase              | All runtimes share the same frontend                              | 2025-01-05 |
| Runtime Tiering                | Embedded / Standard / Full                                        | 2025-01-05 |
| Type Constraints               | Already defined in RFC-011                                        | 2025-01-25 |
| Dependency Graph Construction  | Static dependency graph, determined at compile-time               | 2025-01-05 |
| Dual Backend Model             | VM (dev/debug) + LLVM AOT (prod), identical behavior              | 2026-05-11 |
| Scheduler Form                 | Static library linked into exe, ~200-500KB, no GC                 | 2026-05-11 |
| Reflection Metadata            | Compiled into a dedicated exe section, lazy-loaded via mmap       | 2026-05-11 |
| Scheduler Performance          | DAG analysis done at compile-time; runtime only executes          | 2026-05-11 |
| DAG Scope Alignment            | DAG analysis limited to within spawn blocks, aligned with RFC-024 | 2026-06-05 |
| Three-Tier Architecture Update | Embedded without spawn, Standard supports spawn                   | 2026-06-05 |

---

## References

- [Concurrency Model Specification (RFC-024)](../../../reference/language-spec/concurrency.md)
- [RFC-011: Generics System Design](./011-generic-type-system.md)
- [Rust async runtime design](https://tokio.rs/)
- [Go scheduler design](https://golang.org/src/runtime/proc.go)

---

## Lifecycle and Disposition

| Status           | Location                | Description               |
| ---------------- | ----------------------- | ------------------------- |
| **Draft**        | `docs/design/rfc/`      | Author's draft            |
| **Under Review** | `docs/design/rfc/`      | Open community discussion |
| **Accepted**     | `docs/design/accepted/` | Official design document  |
| **Rejected**     | `docs/design/rfc/`      | Retained in RFC directory |
