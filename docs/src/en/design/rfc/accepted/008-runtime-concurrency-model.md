---
title: "RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design"
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-06-05 (aligned with RFC-024 new concurrency model: DAG analysis scope narrowed to within spawn blocks, old L1/L2/L3 references removed)

> **⚠️ Alignment Note**: This document has been aligned with the [RFC-024 new concurrency model](/reference/language-spec/concurrency.md). Old full-program DAG analysis, `@block`/`@eager` annotations, and L1/L2/L3 tier models have been replaced by `spawn {}` block parallelism primitives. DAG analysis now only operates within `spawn {}` blocks.

> **References**:
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)
> - [Concurrency Model Specification (RFC-024)](/reference/language-spec/concurrency.md)

## Summary

This document defines the key design of the YaoXiang Runtime architecture:

1. **Three-tier Runtime Architecture**: Embedded (immediate execution) → Standard (spawn + DAG scheduling) → Full (work stealing)
2. **Compilation and Execution Separation**: Compilation phase is identical across all modes; differences lie only in runtime execution
3. **Dual Backend Model**: VM (development/debugging) and LLVM AOT (production/release), with identical behavior
4. **Scheduler = Static Library**: Scheduler links into exe at AOT compile time, ~200-500KB, no GC
5. **Synchronous is Just a Special Case of Scheduling**: num_workers=1 equals synchronous mode

### Key Clarification: This is Not Java

```
Java:   .java → .class → JVM (interpreter/JIT + GC)        ← Always needs a virtual machine
YaoXiang Development: .yx → IR → VM execution (fast iteration, step debugging)
YaoXiang Production: .yx → IR → LLVM → Native exe (scheduler linked in)

VM is a development tool, not the essence of the runtime. Similar to Go's go run vs go build.
Final exe = your native code + scheduler static library + reflection metadata. No interpreter, no JIT, no GC.
```

## Motivation

### Core Contradictions

| Contradiction | Description |
|---------------|-------------|
| Transparency vs Controllability | spawn blocks provide explicit concurrency control; normal code executes sequentially |
| Core vs Optional | spawn is a core parallelism primitive; WorkStealing is an advanced feature when num_workers>1 |
| Single-threaded vs Concurrent | In single-threaded mode, concurrency manifests as async; synchronous is just a special case of scheduling |

---

## Proposal

### 1. Three-Tier Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                    Compilation Phase (identical for all modes)    │
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
│ Immediate        │ │ spawn + DAG   │ │ Full Scheduler   │
│ Executor         │ │ Concurrent    │ │ Parallel         │
│ Synchronous      │ │ within spawn  │ │ Optimizations    │
│ No spawn support │ │ blocks        │ │ Work Stealing    │
└──────────────────┘ └───────────────┘ └───────────────┘
```

| Tier | Embedded | Standard | Full |
|------|----------|----------|------|
| Compilation | Same | Same | Same |
| Execution mode | Synchronous | Concurrent within spawn blocks | Parallel |
| Memory footprint | Low | Medium | High |
| Concurrency | None | Within spawn blocks | Within spawn blocks + parallel |
| spawn support | ❌ | ✅ | ✅ |
| DAG analysis | None | Within spawn blocks | Within spawn blocks |
| WorkStealer | None | None | ✅ |

**Embedded Runtime**: Target WASM/game scripts/rule engines. Immediate executor, no spawn support, high performance, low footprint.

**Standard Runtime**: Target web services/data pipelines. Supports `spawn {}` blocks, performs DAG analysis and automatic concurrency within spawn blocks. num_workers=1 equals single-threaded async.

**Full Runtime**: Target scientific computing/large-scale parallelism. Standard + WorkStealer load balancing.

### 2. Scheduler Decoupling: Generics + Injection

Core principle: VM does not directly depend on specific schedulers; uses generic parameter `[S]` for calls.

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

**Core Points**:
- Compile-time polymorphism, zero runtime overhead
- No trait objects needed
- Generic type constraint `[S: Scheduler]` was already defined in RFC-011

### 3. Synchronous = Special Case of Scheduling

```
❌ Misconception: Disable the scheduler
✅ Correct: Use a scheduler with single worker

num_workers = 1 → Single-threaded async scheduling
num_workers > 1 → Multi-threaded parallel scheduling

Same scheduler interface, just different configuration. Eliminates special cases.
```

### 4. The Status of DAG

> **Important Change**: DAG analysis no longer operates on the full program; it only occurs within `spawn {}` blocks. Normal code (outside spawn blocks) executes sequentially and requires no DAG analysis.

| Tier | spawn support | DAG analysis scope | Description |
|------|---------------|---------------------|-------------|
| Core Runtime | ✅ | Within spawn blocks | Concurrency core |
| Standard Runtime | ✅ | Within spawn blocks | spawn + DAG scheduling |
| Embedded Runtime | ❌ | None | Immediate execution, no concurrency |

### 5. Bottom-Up Execution Model (Within spawn Blocks)

> **Important Change**: Bottom-up DAG analysis only occurs within `spawn {}` blocks; it no longer performs full-program DAG analysis.

```
User code (concurrent within spawn block):
    (a, b) = spawn {
        fetch(url0),
        fetch(url1)
    }
    print(a)

Compile-time analysis (bottom-up within spawn block):
    fetch(url0) and fetch(url1) have no mutual dependency → can execute in parallel
    print(a) outside spawn block → sequential execution, waits for spawn to complete

Runtime scheduling (from leaves within spawn block):
    fetch(url0) ┐
                ├→ execute in parallel
    fetch(url1) ┘
    print(a)                       ← outside spawn block, sequential execution
```

**Core Points**:
- Bottom-up dependency analysis is limited to within `spawn {}` blocks
- Tasks within spawn blocks without dependencies execute in parallel
- Code outside spawn blocks executes sequentially, waiting for spawn blocks to complete

---

### 6. Compilation Model: Dual Backend + Statically Linked Runtime

#### 6.1 Two Backends, One Behavior

```
                      ┌─────────────────────┐
                      │   Compiler Frontend  │
                      │   (shared)           │
                      │   Lexer → Parser     │
                      │   → TypeCheck        │
                      │   → DAG analysis     │
                      │     within spawn     │
                      │   → Escape analysis  │
                      │   → Cycle detection  │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │   VM Backend      │     │  LLVM Backend     │
        │   (Development)   │     │  (Production)    │
        │                   │     │                   │
        │  Generate IR/     │     │  Generate native │
        │  bytecode         │     │  code            │
        │  VM interprets     │     │  Link runtime    │
        │  Supports step     │     │  static lib      │
        │  debugging         │     │  Output .exe     │
        │  Fast iteration    │     │  Zero interpret  │
        │                   │     │  overhead        │
        └───────────────────┘     └───────────────────┘
                 │                         │
                 ▼                         ▼
           Identical behavior         Identical behavior
```

**VM Backend**: Used during development. Modify code → run immediately → step debug → fast iteration. Behavior is exactly consistent with final exe.

**LLVM Backend**: Used for release. AOT compile to native code, scheduler linked as static library. No interpreter, no JIT.

#### 6.2 Scheduler = Static Library, Not Virtual Machine

```
Internal structure of final exe:

┌────────────────────────────────────────────┐
│  Your code (native machine code)           │
│  ├── DAG execution plan determined at      │
│  │   compile-time within spawn blocks      │
│  ├── Inlined Move/ref/clone operations     │
│  └── RAII release code                     │
├────────────────────────────────────────────┤
│  Runtime static library (~200-500KB)        │
│  ├── Thread pool (fixed size = num_workers)│
│  ├── Event loop (libuv / io_uring)        │
│  ├── Work-stealing queue (Full Runtime    │
│  │   only)                                 │
│  ├── Memory allocator (jemalloc /         │
│  │   mimalloc)                             │
│  └── Reflection metadata (loaded on        │
│      demand, not resident)                 │
├────────────────────────────────────────────┤
│  Not present:                              │
│  ❌ Bytecode interpreter                    │
│  ❌ JIT compiler                            │
│  ❌ GC                                      │
│  ❌ Virtual machine                         │
└────────────────────────────────────────────┘
```

Comparison:

| Language | Java | Go | YaoXiang |
|----------|------|-----|----------|
| Compilation output | Bytecode | Native code | Native code |
| Execution method | JVM interpreter/JIT | Direct execution | Direct execution |
| Runtime size | ~200MB (JVM) | ~1-2MB (with GC) | **~200-500KB (no GC)** |
| Memory management | GC | GC | **RAII (deterministic)** |
| Reflection | Resident in memory | Resident in memory | **Stored in exe, loaded on demand** |

#### 6.3 Why Scheduler Performance is Constant

**Key Insight**: Most work is done at compile-time; runtime only does "execution".

```
Compile-time (one-time, not part of runtime):
    ├── Analyze spawn block DAG: who depends on whom
    ├── Topological sort: determine execution order within spawn blocks
    ├── Identify parallelizable tasks: independent subtrees within spawn blocks
    ├── Escape analysis: ref → Rc or Arc
    ├── Cycle detection: auto-downgrade to Weak or error
    └── Inlining: small functions directly expanded

Runtime (per execution, fixed data structures):
    ├── Dispatch tasks to thread pool according to compile-time determined spawn block DAG order
    ├── Encounter I/O → suspend current task, event loop takes over
    ├── Task ready → put back in ready queue
    └── That's about it.
```

**The scheduler itself is a fixed-size data structure**: thread pool, event loop, work queues. No dynamic growth, no adaptive re-optimization, no GC scanning. Behavior is completely predictable.

Compile-time has already computed "what to schedule" within spawn blocks; runtime only does "execution". This is different from tokio—tokio dynamically builds Future chains at runtime. YaoXiang's DAG is static and limited to within spawn blocks.

#### 6.4 Reflection: Stored, Not Resident

Reflection metadata is generated at compile-time and stored in a separate section of the exe. It is not loaded at program startup. When reflection is first requested, it is mmap'd into memory on demand. Like this:

```
exe layout:
  .text     ← your code
  .rodata   ← constants
  .reflect  ← reflection metadata (type info, function signatures, etc.)
              mmap loaded on demand, no memory if not accessed
```

**Trade-off**: Larger exe size (includes reflection data), but zero memory overhead if not accessed at runtime. First access has loading delay (similar to JIT warm-up), subsequent access has zero overhead.



```
src/
├── lib.rs
├── main.rs
├── backends/                          # Runtime backends
│   ├── common/                        # Shared across all backends (values, heap, opcodes)
│   │   ├── allocator.rs
│   │   ├── heap.rs
│   │   ├── opcode.rs
│   │   └── value.rs
│   ├── dev/                           # REPL + debugger
│   │   ├── debugger.rs
│   │   ├── shell.rs
│   │   └── repl/
│   ├── interpreter/                   # 🟢 Tree-walking interpreter (former Embedded/VM)
│   │   ├── ffi.rs
│   │   ├── frames.rs
│   │   ├── registers.rs
│   │   ├── runtime.rs
│   │   └── executor/
│   └── runtime/                       # 🔵 Compiled VM runtime
│       ├── engine.rs
│       ├── facade.rs
│       └── task.rs
├── frontend/                          # Compiler frontend (shared by all backends)
│   ├── compiler.rs
│   ├── config.rs
│   ├── pipeline.rs
│   ├── core/
│   │   ├── lexer/
│   │   ├── parser/
│   │   ├── typecheck/
│   │   │   ├── checker.rs
│   │   │   ├── spawn_placement.rs     # ★ DAG/concurrency analysis within spawn blocks
│   │   │   │     (former frontend/dag/)
│   │   │   ├── inference/
│   │   │   └── traits/
│   │   └── types/
│   ├── events/
│   ├── module/
│   └── pipeline/
├── middle/                            # Middle-end
│   ├── core/                          # IR & bytecode
│   │   ├── bytecode.rs
│   │   ├── ir.rs                      #   IR definition (shared by VM and LLVM)
│   │   └── ir_gen.rs
│   └── passes/                        # Compilation passes
│       ├── codegen/                   # Code generation (former codegen/)
│       ├── lifetime/                  # Lifetime/borrow analysis
│       └── mono/                      # Monomorphization
├── lsp/                               # Language Server
├── formatter/                         # Source formatter
├── package/                           # Package manager
├── std/                               # Standard library
│   ├── concurrent.rs
│   ├── io.rs
│   ├── list.rs
│   ├── math.rs
│   ├── net.rs
│   ├── string.rs
│   └── weak.rs
└── util/                              # Utilities
    ├── diagnostic/
    ├── i18n/
    └── config/
```

**Directory Mapping** (old → new):

| Old directory | New location | Description |
|---------------|--------------|-------------|
| `frontend/dag/` | `frontend/core/typecheck/spawn_placement.rs` | DAG analysis within spawn blocks integrated into type checking |
| `codegen/` | `middle/passes/codegen/` | Code generation moved to middle-end passes |
| `embedded/` | `backends/interpreter/` | Tree-walking interpreter |
| `runtime/` | `backends/runtime/` | Compiled VM runtime |
| `vm/` | `backends/interpreter/` | Merged with embedded |
| `full/` | (not implemented) | Full Runtime + work stealing, future version |
| `reflect/` | (not implemented) | Reflection metadata, future version |
| `core/` | `backends/common/` | Shared values/heap/opcodes |

---

## Trade-offs

### Advantages

- **Clear layering**: Three tiers—Embedded / Standard / Full
- **Compilation reuse**: Frontend code completely shared
- **Generic decoupling**: Compile-time polymorphism, zero overhead
- **Consistency**: Synchronous is just a special case of scheduling
- **Embedded-friendly**: High performance + low memory + fast startup

### Disadvantages

- **Initial complexity**: Need to define scheduler interface and multiple runtime variants
- **Compile-time binding**: Scheduler type is determined at compile-time

---

## Design Decision Log

| Decision | Resolution | Date |
|----------|------------|------|
| Scheduler decoupling scheme | Generics + injection | 2025-01-05 |
| Single-threaded mode | Synchronous is a special case of scheduling | 2025-01-05 |
| Async implementation | DAG naturally supports it | 2025-01-05 |
| WorkStealer | Full Runtime advanced feature | 2025-01-05 |
| Embedded design | Immediate execution, no DAG scheduling | 2025-01-05 |
| Compilation phase | All runtimes share same frontend | 2025-01-05 |
| Runtime tiering | Embedded / Standard / Full | 2025-01-05 |
| Type constraints | Already defined in RFC-011 | 2025-01-25 |
| Dependency graph construction | Static dependency graph, determined at compile-time | 2025-01-05 |
| Dual backend model | VM (dev/debug) + LLVM AOT (production), consistent behavior | 2026-05-11 |
| Scheduler form | Static library linked into exe, ~200-500KB, no GC | 2026-05-11 |
| Reflection metadata | Compiled into separate exe section, mmap loaded on demand | 2026-05-11 |
| Scheduler performance | DAG analysis completed at compile-time, runtime only executes | 2026-05-11 |
| DAG scope alignment | DAG analysis limited to spawn blocks only, aligned with RFC-024 | 2026-06-05 |
| Three-tier architecture update | Embedded has no spawn, Standard supports spawn | 2026-06-05 |

---

## References

- [Concurrency Model Specification (RFC-024)](/reference/language-spec/concurrency.md)
- [RFC-011: Generic Type System Design](./011-generic-type-system.md)
- [Rust async runtime design](https://tokio.rs/)
- [Go scheduler design](https://golang.org/src/runtime/proc.go)

---

## Lifecycle and Disposition

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft |
| **Under Review** | `docs/design/rfc/` | Open for community discussion |
| **Accepted** | `docs/design/accepted/` | Official design document |
| **Rejected** | `docs/design/rfc/` | Retained in RFC directory |