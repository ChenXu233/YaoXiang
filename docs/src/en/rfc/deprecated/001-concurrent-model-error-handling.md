---
title: 'RFC-001: YaoXiang Spawn Model and Error Handling System'
status: 'Deprecated'
author: 'Chenxu'
created: '2025-01-05'
updated:
  '2026-05-11 (Pruning: removed @auto, L1 fallback heuristics, simplified discussion records)'
---

> **⚠️ Deprecated (DEPRECATED)**
>
> This RFC has been superseded by
> **[RFC-024: New Concurrency Model](../accepted/024-concurrency-model.md)**.
>
> The three-layer concurrency architecture (L1/L2/L3), the @block/@eager annotations, DAG
> auto-analysis, and other designs from RFC-001 have been removed. The new design uses `spawn {}`
> blocks as the only parallel primitive, with no annotations required.
>
> This document is retained for historical reference only.

---

# RFC-001: YaoXiang Spawn Model and Error Handling System

## Design Source

| Document                                                | Relationship                          |
| ------------------------------------------------------- | ------------------------------------- |
| async-whitepaper                                        | Design source, theoretical foundation |
| [language-spec](../../reference/language-spec/index.md) | Specification target                  |

## Summary

Proposes the YaoXiang spawn model: logic is described with synchronous syntax and is automatically
executed concurrently at runtime. Core mechanisms: three-layer concurrency architecture + DAG
dependency analysis + Result type system.

## Quick Selection

| Scenario          | Syntax                  | Description                       |
| ----------------- | ----------------------- | --------------------------------- |
| Auto-parallel     | No annotation (default) | Maximize parallelism              |
| Synchronous wait  | `@eager`                | Wait for dependencies to complete |
| Fully sequential  | `@block`                | No concurrency, for debugging     |
| Local concurrency | `spawn`                 | Concurrency within @block scope   |

## Motivation

Mainstream languages have obvious flaws in their concurrency models:

| Language   | Concurrency model   | Issues                                |
| ---------- | ------------------- | ------------------------------------- |
| Rust       | async/await + tokio | Async infection, steep learning curve |
| Go         | goroutine           | No type safety                        |
| Python     | asyncio             | GIL limitation                        |
| JavaScript | Promise/async       | Complex callbacks                     |

### Core Contradictions

1. **Transparency vs Controllability**: Fully transparent but uncontrollable vs fully controllable
   but opaque
2. **Concurrency vs Debuggability**: Concurrent programs are hard to debug vs debuggable programs
   are hard to make concurrent

---

## Proposal

### 1. YaoXiang Spawn Model: Three-Layer Concurrency Architecture

> **Description**: L1/L2/L3 are mental models that help users understand different scenarios. The
> actual implementation has only one set of mechanisms: automatic DAG analysis + annotation control.

| Layer  | Mental Model              | Syntax                  | Execution Mode                         | Parallelism |
| ------ | ------------------------- | ----------------------- | -------------------------------------- | ----------- |
| **L1** | No concurrency            | `@block`                | Pure sequential execution              | ❌ None     |
| **L2** | Concurrency within @block | `spawn`                 | Controllable concurrency within @block | ⚠️ Partial  |
| **L3** | Fully concurrent          | Default (no annotation) | Automatic DAG analysis                 | ✅ Full     |

#### L1: @block Synchronous Mode

```yaoxiang
main: () -> Void @block = {
    data1 = fetch_sync("api1")
    data2 = fetch_sync("api2")
    process(data1, data2)    # Strict order, no concurrency
}
```

#### L2: Controllable Concurrency within @block

```yaoxiang
# spawn can only be used inside @block functions
main: () -> Void @block = {
    spawn { data1 = fetch_data("api1") }
    spawn { data2 = fetch_data("api2") }
    # Wait for all spawns to complete (controlled by the standard library)
    process(data1, data2)
}
```

#### L3: Fully Transparent (Default)

```yaoxiang
# No annotation needed; the compiler automatically analyzes the DAG
heavy_calc: (n: Int) -> Int = fibonacci(n)

auto_parallel: (n: Int) -> Int = {
    a = heavy_calc(1)    # Auto-parallel
    b = heavy_calc(2)    # Auto-parallel
    c = heavy_calc(3)    # Auto-parallel
    a + b + c            # Wait for all results when values are needed
}
```

### 2. Complete Annotation Comparison

| Dimension            | Default (no annotation) | `@eager`                  | `@block`        | `spawn`                   |
| -------------------- | ----------------------- | ------------------------- | --------------- | ------------------------- |
| **Execution Mode**   | Automatic DAG analysis  | Synchronous wait for deps | Pure sequential | Concurrency within @block |
| **Parallelism**      | ✅ Full                 | ⚠️ In dependency order    | ❌ None         | ⚠️ Partial                |
| **DAG Construction** | ✅                      | ✅                        | ❌              | ✅                        |

**Selection Guide**:

- Maximum concurrency → No annotation (default)
- Need ordered side effects → `@eager`
- Debugging/beginner/critical code → `@block`
- Need concurrency within @block → `spawn`

```yaoxiang
# Default: maximize parallelism
calc_all: () -> Int = {
    a = heavy_calc(1)    # Auto-parallel
    b = heavy_calc(2)    # Auto-parallel
    a + b
}

# @eager: synchronous wait
calc_seq: () -> Int @eager = {
    a = heavy_calc(1)    # Synchronous execution
    b = heavy_calc(2)    # Synchronous execution
    a + b
}

# @block: pure sequential
calc_simple: () -> Int @block = {
    a = heavy_calc(1)    # Force synchronous
    b = heavy_calc(2)    # Synchronous
    a + b
}

# spawn: concurrency within @block
calc_mixed: () -> Int @block = {
    spawn { heavy_calc(1) }
    spawn { heavy_calc(2) }
    heavy_calc(3)        # Synchronous
}
```

### 3. DAG Dependency Analysis

#### 3.1 Core Principle: Bottom-up Execution

```
User code (synchronous syntax):
    a = fetch(url0)
    b = fetch(url1)
    print(a)

Compile-time analysis (bottom-up):
    print(a) needs a → depends on fetch(url0)
    fetch(url1) — no one needs it → island DAG

Runtime scheduling (starting from leaves):
    fetch(url0) → print(a)    ← dependency chain, in order
    fetch(url1)                ← island, runs independently in parallel
```

**Key Insight**: It is not "top-down" Future generation, but "bottom-up" reverse analysis of
dependencies from results.

#### 3.2 Island DAG: Independent Parallelism

```
Main flow: fetch(url0) → process → print
Island:   fetch(url1)  ← no one needs the result, runs independently in parallel

Scheduler: main flow follows the dependency chain, island runs in parallel on another core
```

#### 3.3 Resource Types and Side Effects

**Core Idea**: Resource operations are marked via types, and the DAG automatically builds
dependencies. Operations on the same resource are automatically serialized; operations on different
resources run in parallel automatically.

**Resource Type Boundary — Clear Definition**:

Resource types are compiler-built-in marked types. The following types are recognized by the
compiler as resources:

| Resource Type | Description         | Compiler Behavior                                              |
| ------------- | ------------------- | -------------------------------------------------------------- |
| `FilePath`    | Filesystem path     | Operations on the same path are automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Operations on the same URL are automatically serialized        |
| `DBUrl`       | Database connection | Operations on the same connection are automatically serialized |
| `Console`     | Standard output     | All Console operations are automatically serialized            |

User-defined resource types need to be explicitly marked:

```yaoxiang
Database: Resource              # Explicitly marked as a resource type
query: (Database, String) -> Result(Row, Error)
# Parameter Database is Resource, automatically recognized as a resource operation
```

Types not marked as Resource will not have resource dependencies tracked by the compiler.

**Usage Rules**:

- Pass resource handles via variables, and the DAG automatically manages the order
- Direct use of the same resource via literals is a user design issue, not a language responsibility

```yaoxiang
# ✅ Correct: pass via variable, DAG auto-serializes
filename: String = "data.txt"
File.write(filename, x)
File.write(filename, y)    # DAG serializes

# ⚠️ User responsibility: literals
File.write("data.txt", x)
File.write("data.txt", y)  # May run in parallel; user is responsible
```

#### 3.4 Infinite Loop Handling

```
1 loop → directly synchronous execution, zero scheduling overhead
Multiple loops → scheduler slices and switches, true concurrency
```

### 4. Result Type and Error Handling

```yaoxiang
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }

# ? operator transparently propagates
process: () -> Result(Data, Error) = {
    data = fetch_data()?
    processed = transform(data)?
    save(processed)?
}
```

### 5. DAG Node Design

```rust
enum NodeKind {
    Task,      // Task node
    Value,     // Value node
    Control,   // Control flow node
}

struct Node {
    id: NodeId,
    kind: NodeKind,
    inputs: Vec<ValueNodeId>,   // Input dependencies
    outputs: Vec<ValueNodeId>,  // Output values
    span: Span,                 // Source location
}
```

| Edge Type   | Symbol | Semantics                                 |
| ----------- | ------ | ----------------------------------------- |
| DataEdge    | →      | Data dependency (value flow)              |
| ControlEdge | ●      | Control dependency (sequential execution) |
| SpawnEdge   | ◎      | Concurrent entry (parallel start point)   |

### 6. Type System

```
Send → can be safely transferred across threads
Sync → can be safely shared across threads
Arc(T) implements Send + Sync (thread-safe reference counting)
```

---

## Trade-offs

### Pros

1. **Progressive adoption**: Three-layer model adapts to different skill levels
2. **Natural syntax**: Synchronous code gets parallel performance
3. **Compile-time safety**: Send/Sync constraints eliminate data races
4. **Debuggable**: Error graph provides a clear view of error propagation

### Cons

1. **Learning curve**: Need to understand DAG dependency concepts
2. **Compile time**: Whole-program DAG analysis may be slow
3. **Toolchain complexity**: Need brand-new debugging and visualization tools

## Alternatives

| Option                                     | Why Not Chosen                            |
| ------------------------------------------ | ----------------------------------------- |
| Support only explicit async/await          | Cannot achieve transparent concurrency    |
| Support only fully transparent concurrency | User loses control                        |
| Go-style goroutine                         | No type safety, no compile-time checks    |
| L1 mode only                               | Abandon the core value of the spawn model |

## Implementation Strategy

### Phase Division

1. **Phase 1 (v0.1)**: @block synchronous mode, basic types
2. **Phase 2 (v0.2)**: FlowScheduler
3. **Phase 3 (v0.3)**: spawn blocks, explicit concurrency
4. **Phase 4 (v0.5)**: L3 fully transparent, automatic DAG analysis
5. **Phase 5 (v0.6)**: Error graph, graph debugger
6. **Phase 6 (v1.0)**: Production-ready optimization

### Dependencies

- RFC-001 has no external dependencies (core foundation)
- RFC-008 (Runtime concurrency model) → design complete
- RFC-011 (Generics system) → design complete

### Risks

1. **DAG analysis performance**: Whole-program analysis may be O(n²), needs optimization
2. **Missing toolchain**: Debugger needs to be developed from scratch
3. **User acceptance**: Transparent concurrency needs good documentation

---

## Design Decision Records

| Decision                             | Decision                                              | Date       |
| ------------------------------------ | ----------------------------------------------------- | ---------- |
| Three-layer concurrency architecture | L1/L2/L3 progressive                                  | 2025-01-05 |
| @block annotation position           | After return type                                     | 2025-01-05 |
| DAG error propagation                | Propagates upstream along dependency edges            | 2025-01-06 |
| DAG performance optimization         | Incremental build + cache                             | 2025-01-06 |
| Runtime choice                       | Generics + compile-time injection                     | 2025-01-06 |
| Node interface                       | Generics + function injection (no trait)              | 2025-01-06 |
| Error graph memory                   | DAG built only within a single function               | 2025-01-06 |
| Resource conflict detection          | DAG data flow dependencies, user variable passing     | 2025-01-06 |
| Resource type system                 | Resource marking + automatic DAG dependencies         | 2026-01-06 |
| L1/L2/L3 mental model                | Three-layer abstraction, not implementation mechanism | 2026-01-06 |
| @auto annotation                     | Removed, duplicates default behavior                  | 2026-05-11 |
| L1 automatic fallback                | Removed, behavior unpredictable                       | 2026-05-11 |

---

## Appendix: Glossary

| Term        | Definition                                                               |
| ----------- | ------------------------------------------------------------------------ |
| Spawn model | YaoXiang's concurrency paradigm: synchronous syntax, asynchronous nature |
| DAG         | Directed acyclic graph, describes computational dependencies             |
| spawn       | Controllable concurrency within @block scope                             |
| @block      | Synchronous annotation, disables concurrency optimization                |
| @eager      | Eager evaluation, waits for dependencies to complete                     |
| Resource    | Resource type marking, operations automatically build DAG dependencies   |
| Error graph | Visualized error propagation path                                        |

## References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go Concurrency Patterns](https://golang.org/doc/effective_go#concurrency)
- [Work stealing scheduling](https://en.wikipedia.org/wiki/Work_stealing)
- Spawn Model Whitepaper
- [YaoXiang Language Specification](../../reference/language-spec/index.md)
