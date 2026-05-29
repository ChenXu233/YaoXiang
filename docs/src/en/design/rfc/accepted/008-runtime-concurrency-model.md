---
title: "RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design"
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-05-11 (Pruning + New dual-backend model, compiler-runtime separation, scheduler static library, on-demand reflection loading)

> **References**:
> - [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)

## Abstract

This document defines the key design of the Runtime architecture:

1. **Three-tier Runtime Architecture**: Embedded (immediate execution) → Standard (DAG scheduling) → Full (work-stealing)
2. **Compilation-Runtime Separation**: Identical compilation phase, difference only in runtime execution method
3. **Dual Backend Model**: VM (development/debugging) and LLVM AOT (production/release), completely consistent behavior
4. **Scheduler = Static Library**: Scheduler linked into exe at AOT compile time, ~200-500KB, no GC
5. **Synchronous is Just a Special Case of Scheduling**: num_workers=1 is synchronous mode

### Key Clarification: This is Not Java

```
Java:   .java → .class → JVM (interpret/JIT + GC)        ← Always needs VM
YaoXiang Development: .yx → IR → VM execution (fast iteration, step debugging)
YaoXiang Production: .yx → IR → LLVM → Native exe (scheduler linked in)

VM is a development tool, not the essence of runtime. Similar to Go's go run vs go build.
Final exe = your native code + scheduler static library + reflection metadata. No interpreter, no JIT, no GC.
```

## Motivation

### Core Contradictions

| Contradiction | Description |
|------|------|
| Transparency vs Controllability | Concurrency should be the default behavior, but users should be able to control it |
| Core vs Optional | DAG is core, but WorkStealing is an advanced feature for num_workers>1 |
| Single-threaded vs Concurrent | In single-threaded mode, concurrency manifests as async, synchronous is just a special case of scheduling |

---

## Proposal

### 1. Three-tier Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    Compilation Phase (identical for all modes)   │
│                                                                  │
│  Source Code → Lexer → Parser → TypeCheck → Codegen → IR        │
│                                                                  │
│  ⚠️ Identical syntax parsing, type checking, code generation, IR output │
└──────────────────────────────────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
│ 🟢 Embedded      │ │ 🔵 Standard   │ │ 🟣 Full          │
│ Immediate Executor│ │ DAG Scheduler │ │ Full Scheduler   │
│ Synchronous execution│ │ Lazy evaluation│ │ Parallel optimization│
│ No DAG scheduling │ │ Auto-concurrency│ │ Work-stealing    │
└──────────────────┘ └───────────────┘ └──────────────────┘
```

| Stage | Embedded | Standard | Full |
|------|----------|----------|------|
| Compilation | Identical | Identical | Identical |
| Execution mode | Synchronous | Lazy + Concurrent | Parallel |
| Memory footprint | Low | Medium | High |
| Concurrency capability | None | Auto | Auto + Parallel |
| DAG lazy evaluation | No | ✅ | ✅ |
| WorkStealer | No | No | ✅ |

**Embedded Runtime**: Targets WASM/game scripting/rule engines. Immediate executor, no DAG, high performance, low footprint.

**Standard Runtime**: Targets web services/data pipelines. DAG lazy evaluation + auto-concurrency. num_workers=1 is single-threaded async.

**Full Runtime**: Targets scientific computing/large-scale parallelism. Standard + WorkStealer load balancing.

### 2. Scheduler Decoupling: Generics + Injection

Core principle: VM does not directly depend on concrete schedulers; calls are made through generic parameter `[S]`.

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

**Key Points**:
- Compile-time polymorphism, zero runtime overhead
- No Trait objects needed
- Generic type constraint `[S: Scheduler]` is already defined in RFC-011

### 3. Synchronous = Special Case of Scheduling

```
❌ Misconception: Disabling the scheduler
✅ Correct: Using a scheduler with a single worker

num_workers = 1 → Single-threaded async scheduling
num_workers > 1 → Multi-threaded parallel scheduling

Same scheduler interface, just different configuration. Eliminates special cases.
```

### 4. DAG's Position

| Layer | Contains DAG | Description |
|------|----------|------|
| Core Runtime | ✅ | Lazy evaluation core |
| Standard Runtime | ✅ | DAG scheduler |
| Embedded Runtime | ❌ | Immediate execution, no DAG |

### 5. Bottom-Up Execution Model

```
User code (synchronous syntax):
    a = fetch(url0)
    b = fetch(url1)
    print(a)

Compile-time analysis (bottom-up):
    print(a) needs a → depends on fetch(url0)
    fetch(url1) is not needed by anyone → isolated island DAG

Runtime scheduling (starting from leaves):
    fetch(url0) → print(a)    ← dependency chain, sequential
    fetch(url1)                ← isolated island, independent parallel
```

**Key Points**:
- Reverse analysis of dependencies from "where the result is needed"
- Leaf nodes take priority for parallel execution
- Isolated island DAGs execute independently in parallel, not blocking the main flow

---

### 6. Compilation Model: Dual Backend + Statically Linked Runtime

#### 6.1 Two Backends, One Behavior

```
                      ┌─────────────────────┐
                      │   Compiler Frontend (unified) │
                      │   Lexer → Parser     │
                      │   → TypeCheck        │
                      │   → DAG Analysis     │
                      │   → Escape Analysis  │
                      │   → Cycle Detection  │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │   VM Backend (development)   │     │  LLVM Backend (production)  │
        │                   │     │                   │
        │  Generate IR/bytecode      │     │  Generate native code        │
        │  VM interpretation │     │  Link runtime static lib   │
        │  Step debugging supported  │     │  Output .exe         │
        │  Fast iteration    │     │  Zero interpretation overhead │
        └───────────────────┘     └───────────────────┘
                 │                         │
                 ▼                         ▼
           Behavior identical                 Behavior identical
```

**VM Backend**: Used during development. Modify code → run immediately → step debug → fast iteration. Behavior is completely consistent with the final exe.

**LLVM Backend**: Used for release. AOT compile to native code, scheduler linked as static library. No interpreter, no JIT.

#### 6.2 Scheduler = Static Library, Not Virtual Machine

```
Internal structure of final exe:

┌────────────────────────────────────────────┐
│  Your code (native machine code)            │
│  ├── DAG execution plan determined at compile time  │
│  ├── Inlined Move/ref/clone operations      │
│  └── RAII release code                      │
├────────────────────────────────────────────┤
│  Runtime static library (~200-500KB)        │
│  ├── Thread pool (fixed size = num_workers) │
│  ├── Event loop (libuv / io_uring)         │
│  ├── Work-stealing queues (Full Runtime only) │
│  ├── Memory allocator (jemalloc / mimalloc)│
│  └── Reflection metadata (loaded on-demand, not resident) │
├────────────────────────────────────────────┤
│  No:                                        │
│  ❌ Bytecode interpreter                    │
│  ❌ JIT compiler                            │
│  ❌ GC                                      │
│  ❌ Virtual machine                         │
└────────────────────────────────────────────┘
```

Comparison:

| | Java | Go | YaoXiang |
|------|------|-----|-----------|
| Compiled artifact | Bytecode | Native code | Native code |
| Execution method | JVM interpretation/JIT | Direct execution | Direct execution |
| Runtime size | ~200MB (JVM) | ~1-2MB (with GC) | **~200-500KB (no GC)** |
| Memory management | GC | GC | **RAII (deterministic)** |
| Reflection | Resident memory | Resident memory | **Stored in exe, loaded on-demand** |

#### 6.3 Why Scheduler Performance is Constant

**Key Insight**: Most work is done at compile time; runtime only does "execution".

```
Compile time (one-time, not in runtime):
    ├── Build global DAG: who depends on whom
    ├── Topological sort: determine execution order
    ├── Identify isolated islands: subtrees that can be parallelized
    ├── Escape analysis: ref → Rc or Arc
    ├── Cycle detection: auto-downgrade Weak or error
    └── Inlining: small functions directly expanded

Runtime (per execution, fixed data structures):
    ├── Dispatch tasks to thread pool in compile-time DAG order
    ├── Encounter I/O → suspend current task, event loop takes over
    ├── Task ready → return to ready queue
    └── That's about it.
```

**The scheduler itself is a fixed-size data structure**: thread pool, event loop, work queues. No dynamic growth, no adaptive re-optimization, no GC scanning. Behavior is completely predictable.

Compile time has already figured out "what to schedule"; runtime only does "execution". This is different from tokio—tokio dynamically builds Future chains at runtime. YaoXiang's DAG is static.

#### 6.4 Reflection: Stored, Not Resident

Reflection metadata is generated at compile time and stored in a separate section of the exe. Not loaded at program startup. When reflection is first requested, it's mmap'd into memory on-demand. Similar to:

```
exe layout:
  .text     ← your code
  .rodata   ← constants
  .reflect  ← reflection metadata (type info, function signatures, etc.)
              mmap loaded on-demand, no memory if not accessed
```

**Trade-off**: exe size increases (includes reflection data), but zero memory overhead when not accessed at runtime. First access has loading latency (similar to JIT warmup), subsequent accesses have zero overhead.



```
src/
├── core/                    # Shared by all runtimes
│   ├── value.rs
│   ├── allocator.rs
│   └── ownership.rs
├── frontend/                # Shared by all backends
│   ├── lexer/
│   ├── parser/
│   ├── typecheck/
│   └── dag/                 # ★ DAG analysis (compile time)
│       ├── builder.rs       #   Build dependency graph
│       ├── escape.rs        #   Escape analysis (ref → Rc/Arc)
│       ├── cycle.rs         #   Cycle detection + auto-downgrade
│       └── topology.rs      #   Topological sort
├── codegen/                 # Code generation
│   ├── ir.rs                # IR definition (shared by VM and LLVM)
│   ├── vm_backend/          # VM backend (development/debugging)
│   │   ├── bytecode.rs
│   │   └── compiler.rs
│   └── llvm_backend/        # LLVM backend (production/release)
│       └── compiler.rs
├── embedded/                # 🟢 Embedded Runtime
│   └── executor.rs
├── runtime/                 # 🔵 Runtime static library (linked into exe)
│   ├── thread_pool.rs       #   Fixed-size thread pool
│   ├── event_loop.rs        #   I/O event loop (libuv/io_uring)
│   ├── dag_executor.rs      #   Execute according to compile-time DAG
│   └── scheduler/
│       ├── single_thread.rs
│       └── multi_thread.rs
├── full/                    # 🟣 Full Runtime (optional linking)
│   └── work_stealer.rs      #   Work stealing
├── reflect/                 # Reflection metadata
│   ├── metadata.rs          #   Metadata generation (compile time)
│   └── loader.rs            #   On-demand loading (runtime)
└── vm/                      # VM interpreter (development only)
    └── executor.rs
```

---

## Trade-offs

### Advantages

- **Clear layering**: Embedded / Standard / Full three tiers
- **Compilation reuse**: Frontend code completely shared
- **Generic decoupling**: Compile-time polymorphism, zero overhead
- **Consistency**: Synchronous is just a special case of scheduling
- **Embedded-friendly**: High performance + low memory + fast startup

### Disadvantages

- **Initial complexity**: Need to define scheduler interface and multiple runtime variants
- **Compile-time binding**: Scheduler type is determined at compile time

---

## Design Decision Log

| Decision | Determination | Date |
|------|------|------|
| Scheduler decoupling scheme | Generics + injection | 2025-01-05 |
| Single-threaded mode | Synchronous is a special case of scheduling | 2025-01-05 |
| Async implementation | DAG naturally supports it | 2025-01-05 |
| WorkStealer | Full Runtime advanced feature | 2025-01-05 |
| Embedded design | Immediate execution, no DAG scheduling | 2025-01-05 |
| Compilation phase | All runtimes share the same frontend | 2025-01-05 |
| Runtime layering | Embedded / Standard / Full | 2025-01-05 |
| Type constraints | Already defined in RFC-011 | 2025-01-25 |
| Dependency graph construction | Static dependency graph, determined at compile time | 2025-01-05 |
| Dual backend model | VM (development/debugging) + LLVM AOT (production), consistent behavior | 2026-05-11 |
| Scheduler form factor | Static library linked into exe, ~200-500KB, no GC | 2026-05-11 |
| Reflection metadata | Compiled into separate exe section, mmap on-demand loading | 2026-05-11 |
| Scheduler performance | DAG analysis completed at compile time, runtime only executes | 2026-05-11 |

---

## References

- [RFC-001: Concurrent Model and Error Handling System](./001-concurrent-model-error-handling.md)
- [RFC-011: Generic Type System Design](./011-generic-type-system.md)
- [Rust async runtime design](https://tokio.rs/)
- [Go scheduler design](https://golang.org/src/runtime/proc.go)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author draft |
| **Under Review** | `docs/design/rfc/` | Open for community discussion |
| **Accepted** | `docs/design/accepted/` | Official design document |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory |