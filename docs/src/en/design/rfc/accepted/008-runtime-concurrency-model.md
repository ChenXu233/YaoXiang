---
title: "RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design"
status: "Accepted"
author: "Chenxu"
created: "2025-01-05"
updated: "2026-07-05 (aligned with RFC-024, added Issue linkage)"
issue: "#89"
issues_impl:
  - "#50"
  - "#89"
pr_impl:
  - "#7"
---

# RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design

> **⚠️ Alignment Note**: This document has been aligned with [RFC-024: New Concurrency Model](/reference/language-spec/concurrency.md). The legacy whole-program DAG analysis, `@block`/`@eager` annotations, and L1/L2/L3 tier model have been superseded by the `spawn {}` block parallel primitive. DAG analysis now applies only inside `spawn {}` blocks.

> **References**:
> - [RFC-011: Generic Type System Design](./011-generic-type-system.md)
> - [Concurrency Model Specification (RFC-024)](/reference/language-spec/concurrency.md)

## Summary

This document defines key designs for the Runtime architecture:

1. **Three-tier runtime architecture**: Embedded (immediate execution) → Standard (spawn + DAG scheduling) → Full (work stealing)
2. **Compile/runtime separation**: The compile phase is identical; differences lie only in how the runtime executes
3. **Dual backend model**: VM (development/debug) and LLVM AOT (production release), with identical behavior
4. **Scheduler = static library**: At AOT compile time the scheduler is linked into the exe, ~200-500KB, no GC
5. **Synchronous is just a special case of scheduling**: num_workers=1 is synchronous mode

### Key Clarification: This Is Not Java

```
Java:   .java → .class → JVM (interpret/JIT + GC)        ← always needs a VM
YaoXiang dev:  .yx → IR → VM execution (fast iteration, step-through debugging)
YaoXiang prod: .yx → IR → LLVM → native exe (scheduler linked in)

The VM is a development tool, not the essence of the runtime. Same as Go's go run vs go build.
Final exe = your native code + scheduler static library + reflection metadata. No interpreter, no JIT, no GC.
```

## Motivation

### Core Contradictions

| Contradiction | Description |
|------|------|
| Transparency vs Controllability | spawn blocks provide explicit concurrency control; ordinary code executes sequentially |
| Core vs Optional | spawn is the core parallel primitive; WorkStealing is an advanced feature for num_workers>1 |
| Single-threaded vs Concurrent | In single-threaded mode, concurrency manifests as async; synchronous is just a special case of scheduling |

---

## Proposal

### 1. Three-Tier Runtime Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                  Compile phase (identical for all modes)         │
│                                                                  │
│  Source Code → Lexer → Parser → TypeCheck → Codegen → IR        │
│                                                                  │
│  ⚠️ Same set of syntax parsing, type checking, code generation,   │
│     and IR output                                                │
└──────────────────────────────────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
│ 🟢 Embedded      │ │ 🔵 Standard   │ │ 🟣 Full          │
│ Immediate exec   │ │ spawn + DAG   │ │ Full scheduler   │
│ Synchronous      │ │ in-spawn conc │ │ Parallel optim   │
│ No spawn support │ │ Auto-concur   │ │ Work stealing    │
└──────────────────┘ └───────────────┘ └──────────────────┘
```

| Stage | Embedded | Standard | Full |
|------|----------|----------|------|
| Compile | Same | Same | Same |
| Execution mode | Synchronous | Concurrency within spawn blocks | Parallel |
| Memory footprint | Low | Medium | High |
| Concurrency capability | None | Within spawn blocks | Within spawn blocks + parallel |
| spawn support | ❌ | ✅ | ✅ |
| DAG analysis | None | Within spawn blocks | Within spawn blocks |
| WorkStealer | None | None | ✅ |

**Embedded Runtime**: Targets WASM/game scripts/rule engines. Immediate executor, no spawn support, high performance and low footprint.

**Standard Runtime**: Targets web services/data pipelines. Supports `spawn {}` blocks, performing DAG analysis and automatic concurrency within spawn blocks. num_workers=1 is single-threaded async.

**Full Runtime**: Targets scientific computing/large-scale parallelism. Standard + WorkStealer load balancing.

### 2. Scheduler Decoupling: Generics + Injection

Core principle: The VM does not directly depend on a concrete scheduler; it is invoked through a generic parameter `[S]`.

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

# VM uses the scheduler through generics
create_vm: [S: Scheduler](scheduler: S) -> VM = (scheduler) => {
    VM(scheduler: scheduler, memory: create_memory(), dag: create_dag())
}
```

**Key points**:
- Compile-time polymorphism, zero runtime overhead
- No Trait objects needed
- The generic type constraint `[S: Scheduler]` is already defined in RFC-011

### 3. Synchronous = Special Case of Scheduling

```
❌ Misconception: Disable the scheduler
✅ Correct: Use a scheduler with a single worker

num_workers = 1 → single-threaded async scheduling
num_workers > 1 → multi-threaded parallel scheduling

Same scheduler interface, only the configuration differs. Eliminates special cases.
```

### 4. Role of the DAG

> **Important change**: DAG analysis no longer operates on the whole program; it only runs inside `spawn {}` blocks. Ordinary code (outside spawn blocks) executes sequentially, with no DAG analysis needed.

| Layer | spawn support | DAG analysis scope | Description |
|------|-----------|-------------|------|
| Core Runtime | ✅ | Within spawn blocks | Concurrency core |
| Standard Runtime | ✅ | Within spawn blocks | spawn + DAG scheduling |
| Embedded Runtime | ❌ | None | Immediate execution, no concurrency |

### 5. Bottom-Up Execution Model (inside spawn blocks)

> **Important change**: Bottom-up DAG analysis runs only inside `spawn {}` blocks, no longer over the whole program.

```
User code (concurrency within spawn block):
    (a, b) = spawn {
        fetch(url0),
        fetch(url1)
    }
    print(a)

Compile-time analysis (bottom-up within spawn block):
    fetch(url0) and fetch(url1) have no mutual dependencies → can execute in parallel
    print(a) outside the spawn block → executes sequentially, waits for spawn to complete

Runtime scheduling (start from leaves within spawn block):
    fetch(url0) ┐
                ├→ execute in parallel
    fetch(url1) ┘
    print(a)                       ← outside the spawn block, executes sequentially
```

**Key points**:
- Bottom-up dependency analysis is confined to the inside of `spawn {}` blocks
- Tasks without dependencies within a spawn block execute in parallel
- Code outside spawn blocks executes sequentially, waiting for the spawn block to complete

---

### 6. Compile Model: Dual Backend + Statically Linked Runtime

#### 6.1 Two Backends, One Behavior

```
                      ┌─────────────────────┐
                      │   Compile frontend  │
                      │   (unified)         │
                      │   Lexer → Parser    │
                      │   → TypeCheck       │
                      │   → DAG analysis    │
                      │     within spawn    │
                      │   → Escape analysis │
                      │   → Cycle detection │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │   VM backend      │     │  LLVM backend     │
        │   (development)   │     │  (production)     │
        │                   │     │                   │
        │  Generate IR/     │     │  Generate native  │
        │  bytecode         │     │  code             │
        │  VM interprets    │     │  Link runtime     │
        │  Step-through     │     │  static library   │
        │  debug supported  │     │  Output .exe      │
        │  Fast iteration   │     │  Zero interp      │
        └───────────────────┘     │  overhead         │
                 │                └───────────────────┘
                 ▼                         ▼
           Identical behavior       Identical behavior
```

**VM backend**: Used during development. Modify code → run immediately → step-through debug → fast iteration. Behavior is completely identical to the final exe.

**LLVM backend**: Used for release. AOT compile to native code; the scheduler is linked in as a static library. No interpreter, no JIT.

#### 6.2 Scheduler = Static Library, Not a Virtual Machine

```
Internal structure of the final exe:

┌────────────────────────────────────────────┐
│  Your code (native machine code)           │
│  ├── DAG execution plan determined at     │
│  │   compile time                          │
│  ├── Inlined Move/ref/clone operations    │
│  └── RAII release code                    │
├────────────────────────────────────────────┤
│  Runtime static library (~200-500KB)       │
│  ├── Thread pool (fixed size = num_workers)│
│  ├── Event loop (libuv / io_uring)        │
│  ├── Work-stealing queue                  │
│  │   (Full Runtime only)                  │
│  ├── Memory allocator                     │
│  │   (jemalloc / mimalloc)               │
│  └── Reflection metadata                  │
│      (loaded on demand, not resident)     │
├────────────────────────────────────────────┤
│  Not included:                             │
│  ❌ Bytecode interpreter                   │
│  ❌ JIT compiler                           │
│  ❌ GC                                     │
│  ❌ Virtual machine                        │
└────────────────────────────────────────────┘
```

Comparison:

| Language | Java | Go | YaoXiang |
|------|------|-----|-----------|
| Compile output | Bytecode | Native code | Native code |
| Execution mode | JVM interpret/JIT | Direct execution | Direct execution |
| Runtime size | ~200MB (JVM) | ~1-2MB (includes GC) | **~200-500KB (no GC)** |
| Memory management | GC | GC | **RAII (deterministic)** |
| Reflection | Always resident | Always resident | **Stored in exe, loaded on demand** |

#### 6.3 Why Scheduler Performance Is Constant

**Key insight**: Most work happens at compile time; the runtime only "executes".

```
Compile time (one-time, does not enter the runtime):
    ├── Analyze DAG within spawn blocks: who depends on whom
    ├── Topological sort: determine execution order within spawn blocks
    ├── Identify parallelizable tasks: dependency-free subtrees within spawn blocks
    ├── Escape analysis: ref → Rc or Arc
    ├── Cycle detection: automatically downgrade to Weak or report error
    └── Inlining: small functions expanded directly

Runtime (each execution, fixed data structure):
    ├── Dispatch tasks to the thread pool in the DAG order determined at compile time
    ├── On I/O → suspend current task, event loop takes over
    ├── Task ready → push back to ready queue
    └── That's it.
```

**The scheduler itself is a fixed-size data structure**: thread pool, event loop, work queue. No dynamic growth, no adaptive re-optimization, no GC scans. Behavior is fully predictable.

By compile time, "what to schedule" within spawn blocks has already been computed; the runtime only does "execution". This differs from tokio — tokio dynamically builds Future chains at runtime. YaoXiang's DAG is static, and confined within spawn blocks.

#### 6.4 Reflection: Stored, Not Resident

Reflection metadata is generated at compile time and stored in a separate section of the exe. It is not loaded at program startup. On the first reflection request, it is mmap'd into memory on demand. Similar to:

```
exe layout:
  .text     ← your code
  .rodata   ← constants
  .reflect  ← reflection metadata (type info, function signatures, etc.)
              mmap'd on demand; no memory cost if not accessed
```

**Trade-off**: The exe grows in size (includes reflection data), but the runtime has zero memory overhead if reflection is not accessed. The first access incurs a loading delay (like JIT warmup); subsequent accesses are zero overhead.

```
src/
├── lib.rs
├── main.rs
├── backends/                          # Runtime backends
│   ├── common/                       # Shared by all backends
│   │   │                              # (values, heap, opcodes)
│   ├── allocator.rs
│   ├── heap.rs
│   ├── opcode.rs
│   └── value.rs
│   ├── dev/                          # REPL + debugger
│   │   ├── debugger.rs
│   │   ├── shell.rs
│   │   └── repl/
│   ├── interpreter/                  # 🟢 Tree-walking interpreter
│   │   │                              #   (original Embedded/VM)
│   ├── ffi.rs
│   ├── frames.rs
│   ├── registers.rs
│   ├── runtime.rs
│   └── executor/
│   └── runtime/                      # 🔵 Compiled VM runtime
│       ├── engine.rs
│       ├── facade.rs
│       └── task.rs
├── frontend/                         # Compile frontend
│   │                                  #   (shared by all backends)
│   ├── compiler.rs
│   ├── config.rs
│   ├── pipeline.rs
│   ├── core/
│   │   ├── lexer/
│   │   ├── parser/
│   │   ├── typecheck/
│   │   │   ├── checker.rs
│   │   │   ├── spawn_placement.rs    # ★ DAG/concurrency analysis
│   │   │   │                          #   within spawn blocks
│   │   │   │                          #   (original frontend/dag/)
│   │   │   ├── inference/
│   │   │   └── traits/
│   │   └── types/
│   ├── events/
│   ├── module/
│   └── pipeline/
├── middle/                           # Middle-end
│   ├── core/                         # IR & bytecode
│   │   ├── bytecode.rs
│   │   ├── ir.rs                     # IR definition (shared by
│   │   │                              #   VM and LLVM)
│   │   └── ir_gen.rs
│   └── passes/                       # Compile passes
│       ├── codegen/                  # Code generation
│       │                              #   (original codegen/)
│       ├── lifetime/                 # Lifetime/borrow analysis
│       └── mono/                     # Monomorphization
├── lsp/                              # Language Server
├── formatter/                        # Source code formatter
├── package/                          # Package manager
├── std/                              # Standard library
│   ├── concurrent.rs
│   ├── io.rs
│   ├── list.rs
│   ├── math.rs
│   ├── net.rs
│   ├── string.rs
│   └── weak.rs
└── util/                             # Utility library
    ├── diagnostic/
    ├── i18n/
    └── config/
```

**Directory mapping description** (old → new):

| Old directory | New location | Description |
|--------|--------|------|
| `frontend/dag/` | `frontend/core/typecheck/spawn_placement.rs` | DAG analysis within spawn blocks integrated into type checking |
| `codegen/` | `middle/passes/codegen/` | Code generation moved into middle-end passes |
| `embedded/` | `backends/interpreter/` | Tree-walking interpreter |
| `runtime/` | `backends/runtime/` | Compiled VM runtime |
| `vm/` | `backends/interpreter/` | Merged with embedded |
| `full/` | (Not yet implemented) | Full Runtime + work stealing, future version |
| `reflect/` | (Not yet implemented) | Reflection metadata, future version |
| `core/` | `backends/common/` | Shared values/heap/opcodes |

---

## Trade-offs

### Advantages

- **Clear layering**: Embedded / Standard / Full three tiers
- **Compile reuse**: Frontend code is fully shared
- **Generic decoupling**: Compile-time polymorphism, zero overhead
- **Consistency**: Synchronous is just a special case of scheduling
- **Embedded-friendly**: High performance + low memory + fast startup

### Disadvantages

- **Initial complexity**: Requires defining the scheduler interface and multiple runtime variants
- **Compile-time binding**: The scheduler type is determined at compile time

---

## Design Decision Record

| Decision | Resolution | Date |
|------|------|------|
| Scheduler decoupling approach | Generics + injection | 2025-01-05 |
| Single-threaded mode | Synchronous is a special case of scheduling | 2025-01-05 |
| Async implementation | DAG natively supports it | 2025-01-05 |
| WorkStealer | Full Runtime advanced feature | 2025-01-05 |
| Embedded design | Immediate execution, no DAG scheduling | 2025-01-05 |
| Compile phase | All runtimes share the same frontend | 2025-01-05 |
| Runtime layering | Embedded / Standard / Full | 2025-01-05 |
| Type constraints | Already defined in RFC-011 | 2025-01-25 |
| Dependency graph construction | Static dependency graph, determined at compile time | 2025-01-05 |
| Dual backend model | VM (dev/debug) + LLVM AOT (production), identical behavior | 2026-05-11 |
| Scheduler form | Statically linked into exe, ~200-500KB, no GC | 2026-05-11 |
| Reflection metadata | Compiled into a separate exe section, mmap'd on demand | 2026-05-11 |
| Scheduler performance | DAG analysis done at compile time, runtime only executes | 2026-05-11 |
| DAG scope alignment | DAG analysis confined to within spawn blocks, aligned with RFC-024 | 2026-06-05 |
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
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author's draft |
| **Under review** | `docs/design/rfc/` | Open community discussion |
| **Accepted** | `docs/design/accepted/` | Official design document |
| **Rejected** | `docs/design/rfc/` | Retained in the RFC directory |