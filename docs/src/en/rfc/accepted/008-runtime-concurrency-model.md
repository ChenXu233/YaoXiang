---
title: 'RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design'
status: 'Accepted'
author: '晨煦'
created: '2025-01-05'
updated: '2026-07-05 (aligned with RFC-024, added issue link)'
issue: '#89'
issues_impl:
  - '#50'
  - '#89'
pr_impl:
  - '#7'
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **⚠️ Alignment Notice**: This document has been aligned with
> [RFC-024 New Concurrency Model](../../reference/language-spec/concurrency.md). The old
> whole-program DAG analysis, `@block`/`@eager` annotations, and L1/L2/L3 layer model have been
> replaced by the `spawn {}` block parallel primitive. DAG analysis now applies only inside
> `spawn {}` blocks.

> **References**:
>
> - [RFC-011: Generic Type System Design](011-generic-type-system.md)
> - [Concurrency Model Spec (RFC-024)](../../reference/language-spec/concurrency.md)

## Summary

This document defines the key designs of the Runtime architecture:

1. **Three-tier Runtime architecture**: Embedded (immediate execution) → Standard (spawn + DAG
   scheduling) → Full (work stealing)
2. **Compile / runtime separation**: The compile phase is identical; differences lie only in how the
   runtime executes
3. **Dual backend model**: VM (development and debugging) and LLVM AOT (production release), with
   identical behavior
4. **Scheduler = static library**: At AOT compile time the scheduler is linked into the exe, about
   200–500 KB, no GC
5. **Synchronization is just a special case of scheduling**: `num_workers=1` is the synchronous mode

### Key Clarification: This Is Not Java

```
Java:           .java → .class → JVM (interpretation/JIT + GC)        ← Always needs a VM
YaoXiang dev:   .yx → IR → VM execution (fast iteration, step debugging)
YaoXiang prod:  .yx → IR → LLVM → native exe (scheduler linked in)

The VM is a development tool, not the essence of the runtime. Same as Go's `go run` vs `go build`.
Final exe = your native code + scheduler static library + reflection metadata. No interpreter, no JIT, no GC.
```

## Motivation

### Core Contradictions

| Contradiction                 | Description                                                                                                       |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Transparency vs control       | `spawn` blocks provide explicit concurrency control; ordinary code runs sequentially                              |
| Core vs optional              | `spawn` is the core parallel primitive; WorkStealing is an advanced feature when `num_workers>1`                  |
| Single-threaded vs concurrent | In single-threaded mode concurrency manifests as asynchrony; synchronization is just a special case of scheduling |

---

## Proposal

### 1. Three-tier Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                  Compile phase (identical for all modes)         │
│                                                                  │
│  Source Code → Lexer → Parser → TypeCheck → Codegen → IR        │
│                                                                  │
│  ⚠️ Same syntax parsing, type checking, code generation, IR output │
└──────────────────────────────────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
│ 🟢 Embedded      │ │ 🔵 Standard   │ │ 🟣 Full          │
│ Immediate        │ │ spawn + DAG   │ │ Full Scheduler   │
│ executor         │ │               │ │                  │
│ Synchronous      │ │ Concurrency   │ │ Parallel         │
│ execution        │ │ inside spawn  │ │ optimization     │
│ No spawn support │ │ block         │ │ Work stealing    │
│                  │ │ Auto          │ │                  │
│                  │ │ concurrency   │ │                  │
└──────────────────┘ └───────────────┘ └──────────────────┘
```

| Stage         | Embedded    | Standard                         | Full                            |
| ------------- | ----------- | -------------------------------- | ------------------------------- |
| Compile       | Same        | Same                             | Same                            |
| Execution     | Synchronous | Concurrency inside `spawn` block | Parallel                        |
| Memory        | Low         | Medium                           | High                            |
| Concurrency   | None        | Inside `spawn` block             | Inside `spawn` block + parallel |
| spawn support | ❌          | ✅                               | ✅                              |
| DAG analysis  | None        | Inside `spawn` block             | Inside `spawn` block            |
| WorkStealer   | None        | None                             | ✅                              |

**Embedded Runtime**: Targets WASM, game scripts, and rule engines. Immediate executor, no `spawn`
support, high performance and low footprint.

**Standard Runtime**: Targets web services and data pipelines. Supports the `spawn {}` block,
performs DAG analysis and automatic concurrency inside the `spawn` block. `num_workers=1` is
single-threaded asynchrony.

**Full Runtime**: Targets scientific computing and large-scale parallelism. Standard + WorkStealer
load balancing.

### 2. Scheduler Decoupling: Generics + Injection

Core principle: The VM does not depend directly on a concrete scheduler, but calls through a generic
parameter `[S]`.

```yaoxiang
# Scheduler interface definition
Scheduler: Type = {
    spawn: (Task) -> TaskId,
    await: (TaskId) -> Result,
    spawn_with_deps: (Task, List(TaskId)) -> TaskId,
    await_all: (List(TaskId)) -> List(Result),
    stats: () -> SchedulerStats,
}

# Single-threaded scheduler
SingleThreadScheduler: Scheduler = {
    spawn: (task) => { task_queue.push(task); generate_task_id() },
    await: (task_id) => { ... },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { queue_size: task_queue.len() },
}

# Multi-threaded scheduler
MultiThreadScheduler: Scheduler = {
    spawn: (task) => { work_queue.push(task); generate_task_id() },
    await: (task_id) => { wait_for_completion(task_id) },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { workers: get_worker_stats() },
}

# VM uses scheduler via generics
create_vm: [S: Scheduler](scheduler: S) -> VM = (scheduler) => {
    VM(scheduler: scheduler, memory: create_memory(), dag: create_dag())
}
```

**Key points**:

- Compile-time polymorphism, zero runtime overhead
- No trait objects required
- The generic type constraint `[S: Scheduler]` is already defined in RFC-011

### 3. Synchronization = A Special Case of Scheduling

```
❌ Misconception: disable the scheduler
✅ Correct: use a single-worker scheduler

num_workers = 1 → single-threaded asynchronous scheduling
num_workers > 1 → multi-threaded parallel scheduling

Same scheduler interface, just different configuration. Eliminate the special case.
```

### 4. Position of the DAG

> **Important change**: DAG analysis no longer applies to the whole program; it is performed only
> inside `spawn {}` blocks. Ordinary code (outside `spawn` blocks) runs sequentially and needs no
> DAG analysis.

| Layer            | spawn support | DAG analysis scope   | Description                         |
| ---------------- | ------------- | -------------------- | ----------------------------------- |
| Core Runtime     | ✅            | Inside `spawn` block | Concurrency core                    |
| Standard Runtime | ✅            | Inside `spawn` block | `spawn` + DAG scheduling            |
| Embedded Runtime | ❌            | None                 | Immediate execution, no concurrency |

### 5. Bottom-up Execution Model (Inside `spawn` Blocks)

> **Important change**: Bottom-up DAG analysis is performed only inside `spawn {}` blocks;
> whole-program DAG analysis is no longer applied.

```
User code (concurrency inside spawn block):
    (a, b) = spawn {
        fetch(url0),
        fetch(url1)
    }
    print(a)

Compile-time analysis (bottom-up inside spawn block):
    fetch(url0) and fetch(url1) have no mutual dependency → can run in parallel
    print(a) outside the spawn block → runs sequentially, waiting for spawn to complete

Runtime scheduling (starting from leaves inside spawn block):
    fetch(url0) ┐
                ├→ run in parallel
    fetch(url1) ┘
    print(a)                       ← outside the spawn block, runs sequentially
```

**Key points**:

- Bottom-up dependency analysis is limited to inside `spawn {}` blocks
- Tasks with no dependencies inside a `spawn` block run in parallel
- Code outside a `spawn` block runs sequentially, waiting for the `spawn` block to complete

---

### 6. Compile Model: Dual Backend + Statically Linked Runtime

#### 6.1 Two Backends, One Behavior

```
                      ┌─────────────────────┐
                      │   Compile front-end │
                      │   (unified)         │
                      │   Lexer → Parser    │
                      │   → TypeCheck       │
                      │   → DAG analysis    │
                      │     inside spawn    │
                      │     blocks          │
                      │   → Escape analysis │
                      │   → Cycle detection │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │  VM backend (dev) │     │ LLVM backend (prod)│
        │                   │     │                   │
        │  Emit IR/bytecode │     │  Emit native code │
        │  VM interprets    │     │  Link runtime     │
        │                   │     │  static library   │
        │  Step debugging   │     │  Output .exe      │
        │  Fast iteration   │     │  Zero interp.     │
        │                   │     │  overhead         │
        └───────────────────┘     └───────────────────┘
                 │                         │
                 ▼                         ▼
           Identical behavior         Identical behavior
```

**VM backend**: Used during development. Edit code → run immediately → step-debug → fast iteration.
Behavior is completely identical to the final exe.

**LLVM backend**: Used for release. AOT compile to native code; the scheduler is linked in as a
static library. No interpreter, no JIT.

#### 6.2 Scheduler = Static Library, Not a Virtual Machine

```
Internal structure of the final exe:

┌────────────────────────────────────────────┐
│  Your code (native machine code)           │
│  ├── Compile-time-determined DAG           │
│  │   execution plan                        │
│  ├── Inlined Move/ref/clone operations     │
│  └── RAII release code                     │
├────────────────────────────────────────────┤
│  Runtime static library (~200–500 KB)      │
│  ├── Thread pool (fixed size = num_workers)│
│  ├── Event loop (libuv / io_uring)        │
│  ├── Work-stealing queues                  │
│  │   (Full Runtime only)                   │
│  ├── Memory allocator                      │
│  │   (jemalloc / mimalloc)                │
│  └── Reflection metadata                   │
│       (loaded on demand, not always        │
│        resident in memory)                 │
├────────────────────────────────────────────┤
│  There is no:                              │
│  ❌ Bytecode interpreter                   │
│  ❌ JIT compiler                           │
│  ❌ GC                                     │
│  ❌ Virtual machine                        │
└────────────────────────────────────────────┘
```

Comparison:

| Language       | Java              | Go                | YaoXiang                            |
| -------------- | ----------------- | ----------------- | ----------------------------------- |
| Compile output | Bytecode          | Native code       | Native code                         |
| Execution      | JVM interpret/JIT | Direct execution  | Direct execution                    |
| Runtime size   | ~200 MB (JVM)     | ~1–2 MB (with GC) | **~200–500 KB (no GC)**             |
| Memory mgmt    | GC                | GC                | **RAII (deterministic)**            |
| Reflection     | Always resident   | Always resident   | **Stored in exe, loaded on demand** |

#### 6.3 Why Scheduler Performance Is Constant

**Key insight**: Most work happens at compile time; the runtime only "executes".

```
Compile time (one-time, not part of runtime):
    ├── Analyze DAG inside spawn blocks: who depends on whom
    ├── Topological sort: determine execution order inside spawn blocks
    ├── Identify parallelizable tasks: independent subtrees inside spawn blocks
    ├── Escape analysis: ref → Rc or Arc
    ├── Cycle detection: automatically downgrade to Weak or report an error
    └── Inlining: small functions expanded directly

Runtime (each execution, fixed data structure):
    ├── Dispatch tasks to thread pool following compile-time-determined
    │   DAG order inside spawn blocks
    ├── Encounter I/O → suspend current task, event loop takes over
    ├── Task ready → put back on ready queue
    └── That's it.
```

**The scheduler itself is a fixed-size data structure**: thread pool, event loop, work queue. No
dynamic growth, no adaptive re-optimization, no GC scanning. Behavior is fully predictable.

The compile phase has already figured out "what to schedule" inside `spawn` blocks; the runtime only
"executes". This is different from tokio — tokio builds its Future chain dynamically at runtime.
YaoXiang's DAG is static, and limited to inside `spawn` blocks.

#### 6.4 Reflection: Stored, Not Resident

Reflection metadata is generated at compile time and stored in a dedicated section of the exe. It is
not loaded at program startup. On the first reflection request, it is mmap'd into memory on demand.
Similar to:

```
exe layout:
  .text     ← Your code
  .rodata   ← Constants
  .reflect  ← Reflection metadata (type info, function signatures, etc.)
              mmap'd on demand, occupies no memory if never accessed
```

**Trade-off**: The exe size grows (it contains reflection data), but if the runtime never accesses
it, memory cost is zero. The first access incurs a load latency (similar to JIT warmup); subsequent
access has zero overhead.

```
src/
├── lib.rs
├── main.rs
├── backends/                          # Runtime backends
│   ├── common/                        # Shared by all backends (values, heap, opcodes)
│   │   ├── allocator.rs
│   │   ├── heap.rs
│   │   ├── opcode.rs
│   │   └── value.rs
│   ├── dev/                           # REPL + debugger
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
├── frontend/                          # Compile front-end (shared by all backends)
│   ├── compiler.rs
│   ├── config.rs
│   ├── pipeline.rs
│   ├── core/
│   │   ├── lexer/
│   │   ├── parser/
│   │   ├── typecheck/
│   │   │   ├── checker.rs
│   │   │   ├── spawn_placement.rs     # ★ DAG/concurrency analysis inside spawn blocks (formerly frontend/dag/)
│   │   │   ├── inference/
│   │   │   └── traits/
│   │   └── types/
│   ├── events/
│   ├── module/
│   └── pipeline/
├── middle/                            # Middle end
│   ├── core/                          # IR & bytecode
│   │   ├── bytecode.rs
│   │   ├── ir.rs                      #   IR definition (shared by VM and LLVM)
│   │   └── ir_gen.rs
│   └── passes/                        # Compile passes
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

| Old directory   | New location                                 | Description                                                  |
| --------------- | -------------------------------------------- | ------------------------------------------------------------ |
| `frontend/dag/` | `frontend/core/typecheck/spawn_placement.rs` | DAG analysis inside `spawn` blocks integrated into typecheck |
| `codegen/`      | `middle/passes/codegen/`                     | Code generation moved into middle-end passes                 |
| `embedded/`     | `backends/interpreter/`                      | Tree-walking interpreter                                     |
| `runtime/`      | `backends/runtime/`                          | Compiled VM runtime                                          |
| `vm/`           | `backends/interpreter/`                      | Merged with embedded                                         |
| `full/`         | (Not yet implemented)                        | Full Runtime + work stealing, in a later version             |
| `reflect/`      | (Not yet implemented)                        | Reflection metadata, in a later version                      |
| `core/`         | `backends/common/`                           | Shared values/heap/opcodes                                   |

---

## Trade-offs

### Advantages

- **Clear layering**: Embedded / Standard / Full
- **Compile reuse**: Front-end code is fully shared
- **Generics decoupling**: Compile-time polymorphism, zero overhead
- **Consistency**: Synchronization is just a special case of scheduling
- **Embedded-friendly**: High performance + low memory + fast startup

### Disadvantages

- **Initial complexity**: Need to define the scheduler interface and multiple runtime variants
- **Compile-time binding**: Scheduler type is fixed at compile time

---

## Design Decision Log

| Decision               | Decision                                                            | Date       |
| ---------------------- | ------------------------------------------------------------------- | ---------- |
| Scheduler decoupling   | Generics + injection                                                | 2025-01-05 |
| Single-threaded mode   | Synchronization is a special case of scheduling                     | 2025-01-05 |
| Asynchrony impl.       | DAG natively supports it                                            | 2025-01-05 |
| WorkStealer            | Advanced feature of Full Runtime                                    | 2025-01-05 |
| Embedded design        | Immediate execution, no DAG scheduling                              | 2025-01-05 |
| Compile phase          | All runtimes share the same front-end                               | 2025-01-05 |
| Runtime tiers          | Embedded / Standard / Full                                          | 2025-01-05 |
| Type constraints       | Already defined in RFC-011                                          | 2025-01-25 |
| Dependency graph build | Static dependency graph, determined at compile time                 | 2025-01-05 |
| Dual backend model     | VM (dev/debug) + LLVM AOT (prod), identical behavior                | 2026-05-11 |
| Scheduler form         | Static library linked into exe, ~200–500 KB, no GC                  | 2026-05-11 |
| Reflection metadata    | Compiled into a dedicated exe section, mmap'd on demand             | 2026-05-11 |
| Scheduler performance  | DAG analysis at compile time, runtime only executes                 | 2026-05-11 |
| DAG scope alignment    | DAG analysis limited to inside `spawn` blocks, aligned with RFC-024 | 2026-06-05 |
| Three-tier update      | Embedded has no `spawn`, Standard supports `spawn`                  | 2026-06-05 |

---

## References

- [Concurrency Model Spec (RFC-024)](../../reference/language-spec/concurrency.md)
- [RFC-011: Generic Type System Design](011-generic-type-system.md)
- [Rust async runtime design](https://tokio.rs/)
- [Go scheduler design](https://golang.org/src/runtime/proc.go)

---

## Lifecycle and Destination

| State            | Location                | Description               |
| ---------------- | ----------------------- | ------------------------- |
| **Draft**        | `docs/design/rfc/`      | Author's draft            |
| **Under review** | `docs/design/rfc/`      | Open community discussion |
| **Accepted**     | `docs/design/accepted/` | Official design document  |
| **Rejected**     | `docs/design/rfc/`      | Kept in the RFC directory |
