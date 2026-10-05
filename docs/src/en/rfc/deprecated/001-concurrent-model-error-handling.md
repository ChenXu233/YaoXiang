---
title: 'RFC-001: Spawn Model and Error Handling System'
status: 'Deprecated'
author: 'Chenxu'
created: '2025-01-05'
updated:
  '2026-05-11 (trimmed: removed @auto, L1 fallback heuristics, simplified discussion records)'
---

> **⚠️ DEPRECATED**
>
> This RFC has been replaced by
> **[RFC-024: New Concurrency Model](../accepted/024-concurrency-model.md)**.
>
> The three-layer concurrency architecture (L1/L2/L3), @block/@eager annotations, DAG auto-analysis
> and other designs in RFC-001 have been removed. The new design uses `spawn {}` blocks as the only
> parallel primitive, with no annotations needed.
>
> This document is retained for historical reference only.

---

# RFC-001: Spawn Model and Error Handling System

## Design Sources

| Document                                                   | Relationship                          |
| ---------------------------------------------------------- | ------------------------------------- |
| async-whitepaper                                           | Design source, theoretical foundation |
| [language-spec](../../reference/language-spec/index.md) | Specification target                  |

## Summary

Proposes YaoXiang's spawn model: describe logic with synchronous syntax, automatically execute
concurrently at runtime. Core mechanisms: three-layer concurrency architecture + DAG dependency
analysis + Result type system.

## Quick Selection

| Scenario          | Syntax                  | Description                       |
| ----------------- | ----------------------- | --------------------------------- |
| Auto-parallel     | No annotation (default) | Maximum parallelism               |
| Synchronous wait  | `@eager`                | Wait for dependencies to complete |
| Fully sequential  | `@block`                | No concurrency, for debugging     |
| Local concurrency | `spawn`                 | Concurrency within @block scope   |

## Motivation

Current mainstream languages' concurrency models have obvious flaws:

| Language   | Concurrency Model   | Problems                              |
| ---------- | ------------------- | ------------------------------------- |
| Rust       | async/await + tokio | Async contagion, steep learning curve |
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

### 1. Spawn Model: Three-Layer Concurrency Architecture

> **Note**: L1/L2/L3 are mental models that help users understand different scenarios. The actual
> implementation is only one mechanism: DAG auto-analysis + annotation control.

| Level  | Mental Model              | Syntax                  | Execution Method                             | Parallelism |
| ------ | ------------------------- | ----------------------- | -------------------------------------------- | ----------- |
| **L1** | Disable concurrency       | `@block`                | Purely sequential execution                  | ❌ None     |
| **L2** | Concurrency within @block | `spawn`                 | Controllable concurrency within @block scope | ⚠️ Partial  |
| **L3** | Fully concurrent          | Default (no annotation) | Automatic DAG analysis                       | ✅ Full     |

#### L1: @block Synchronous Mode

```yaoxiang
main: () -> Void @block = {
    data1 = fetch_sync("api1")
    data2 = fetch_sync("api2")
    process(data1, data2)    # strictly sequential, no concurrency
}
```

#### L2: Controllable Concurrency within @block

```yaoxiang
# spawn can only be used inside @block functions
main: () -> Void @block = {
    spawn { data1 = fetch_data("api1") }
    spawn { data2 = fetch_data("api2") }
    # wait for all spawn to complete (standard library controlled)
    process(data1, data2)
}
```

#### L3: Fully Transparent (Default)

```yaoxiang
# no annotation needed, compiler automatically analyzes DAG
heavy_calc: (n: Int) -> Int = fibonacci(n)

auto_parallel: (n: Int) -> Int = {
    a = heavy_calc(1)    # auto-parallel
    b = heavy_calc(2)    # auto-parallel
    c = heavy_calc(3)    # auto-parallel
    a + b + c            # wait for all results when value is needed
}
```

### 2. Complete Annotation Comparison

| Dimension            | Default (no annotation) | `@eager`                          | `@block`          | `spawn`                   |
| -------------------- | ----------------------- | --------------------------------- | ----------------- | ------------------------- |
| **Execution Method** | Automatic DAG analysis  | Synchronous wait for dependencies | Purely sequential | Concurrency within @block |
| **Parallelism**      | ✅ Full                 | ⚠️ In dependency order            | ❌ None           | ⚠️ Partial                |
| **DAG Construction** | ✅                      | ✅                                | ❌                | ✅                        |

**Selection Guide**:

- Maximum concurrency → No annotation (default)
- Need ordered side effects → `@eager`
- Debugging / beginners / critical code → `@block`
- Need concurrency within @block → `spawn`

```yaoxiang
# default: maximize parallelism
calc_all: () -> Int = {
    a = heavy_calc(1)    # auto-parallel
    b = heavy_calc(2)    # auto-parallel
    a + b
}

# @eager: synchronous wait
calc_seq: () -> Int @eager = {
    a = heavy_calc(1)    # synchronous execution
    b = heavy_calc(2)    # synchronous execution
    a + b
}

# @block: purely sequential
calc_simple: () -> Int @block = {
    a = heavy_calc(1)    # force synchronous
    b = heavy_calc(2)    # synchronous
    a + b
}

# spawn: concurrency within @block
calc_mixed: () -> Int @block = {
    spawn { heavy_calc(1) }
    spawn { heavy_calc(2) }
    heavy_calc(3)        # synchronous
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
    fetch(url1) nobody needs → island DAG

Runtime scheduling (starting from leaves):
    fetch(url0) → print(a)    ← dependency chain, in order
    fetch(url1)                ← island, independent parallel
```

**Key Insight**: Not "top-down" Future generation, but "bottom-up" reverse analysis of dependencies
from results.

#### 3.2 Island DAG: Independent Parallel

```
Main flow: fetch(url0) → process → print
Island:    fetch(url1)  ← no one needs the result, independent parallel

Scheduler: main flow executes by dependency chain, island uses another core to run in parallel
```

#### 3.3 Resource Types and Side Effects

**Core Idea**: Resource operations are marked by types, DAG automatically builds dependencies. Same
resource is automatically serialized, different resources are automatically parallel.

**Resource Type Boundary — Clear Definition**:

Resource types are types marked by the compiler as built-in. The following types are recognized by
the compiler as resources:

| Resource Type | Description         | Compiler Behavior                                   |
| ------------- | ------------------- | --------------------------------------------------- |
| `FilePath`    | Filesystem path     | Same-path operations automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations automatically serialized        |
| `DBUrl`       | Database connection | Same-connection operations automatically serialized |
| `Console`     | Standard output     | All Console operations automatically serialized     |

User-defined resource types need explicit marking:

```yaoxiang
Database: Resource              # explicitly marked as a resource type
query: (Database, String) -> Result(Row, Error)
# parameter Database is Resource, automatically recognized as a resource operation
```

Non-Resource-marked types are not tracked for resource dependencies by the compiler.

**Usage Rules**:

- Pass resource handles through variables, DAG automatically manages order
- Using the same resource directly with literals is a user design issue, not a language
  responsibility

```yaoxiang
# ✅ correct: pass through variables, DAG auto-serializes
filename: String = "data.txt"
File.write(filename, x)
File.write(filename, y)    # DAG serialized

# ⚠️ user responsibility: literals
File.write("data.txt", x)
File.write("data.txt", y)  # may run in parallel, user's responsibility
```

#### 3.4 Infinite Loop Handling

```
1 loop   → directly synchronous execution, zero scheduling overhead
Multiple loops → scheduler slice switching, true concurrency
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
    Task,      // task node
    Value,     // value node
    Control,   // control flow node
}

struct Node {
    id: NodeId,
    kind: NodeKind,
    inputs: Vec<ValueNodeId>,   // input dependencies
    outputs: Vec<ValueNodeId>,  // output values
    span: Span,                 // source location
}
```

| Edge Type   | Symbol | Semantics                                 |
| ----------- | ------ | ----------------------------------------- |
| DataEdge    | →      | Data dependency (value flow)              |
| ControlEdge | ●      | Control dependency (sequential execution) |
| SpawnEdge   | ◎      | Concurrency entry (parallel start point)  |

### 6. Type System

```
Send → safely transferable across threads
Sync → safely shareable across threads
Arc(T) implements Send + Sync (thread-safe reference counting)
```

---

## Trade-offs

### Pros

1. **Progressive Adoption**: The three-layer model adapts to different skill levels
2. **Natural Syntax**: Synchronous code gets parallel performance
3. **Compile-time Safety**: Send/Sync constraints eliminate data races
4. **Debuggable**: Error graph provides a clear view of error propagation

### Cons

1. **Learning Curve**: Need to understand the DAG dependency concept
2. **Compile Time**: Whole-program DAG analysis may be slow
3. **Toolchain Complexity**: Needs brand-new debugging and visualization tools

## Alternatives

| Plan                                       | Why Not Chosen                            |
| ------------------------------------------ | ----------------------------------------- |
| Only support explicit async/await          | Cannot achieve transparent concurrency    |
| Only support fully transparent concurrency | User loses control                        |
| Go-style goroutine                         | No type safety, no compile-time checks    |
| L1 mode only                               | Abandon the core value of the spawn model |

## Implementation Strategy

### Phase Division

1. **Phase 1 (v0.1)**: @block synchronous mode, basic types
2. **Phase 2 (v0.2)**: FlowScheduler scheduler
3. **Phase 3 (v0.3)**: spawn block, explicit concurrency
4. **Phase 4 (v0.5)**: L3 fully transparent, DAG auto-analysis
5. **Phase 5 (v0.6)**: Error graph, graph debugger
6. **Phase 6 (v1.0)**: Production-ready optimization

### Dependencies

- RFC-001 has no external dependencies (foundational core)
- RFC-008 (Runtime Concurrency Model) → design completed
- RFC-011 (Generics System) → design completed

### Risks

1. **DAG Analysis Performance**: Whole-program analysis may be O(n²), needs optimization
2. **Toolchain Missing**: Debugger needs to be developed from scratch
3. **User Acceptance**: Transparent concurrency needs good documentation

---

## Design Decision Records

| Decision                             | Resolution                                            | Date       |
| ------------------------------------ | ----------------------------------------------------- | ---------- |
| Three-layer concurrency architecture | L1/L2/L3 progressive                                  | 2025-01-05 |
| @block annotation position           | After the return type                                 | 2025-01-05 |
| DAG error propagation                | Propagate upstream along dependency edges             | 2025-01-06 |
| DAG performance optimization         | Incremental build + cache                             | 2025-01-06 |
| Runtime choice                       | Generics + compile-time injection                     | 2025-01-06 |
| Node interface                       | Generics + function injection (no trait)              | 2025-01-06 |
| Error graph memory                   | DAG only built within a single function               | 2025-01-06 |
| Resource conflict detection          | DAG data flow dependency, user variable passing       | 2025-01-06 |
| Resource type system                 | Resource marker + DAG auto-dependency                 | 2026-01-06 |
| L1/L2/L3 mental model                | Three-layer abstraction, not implementation mechanism | 2026-01-06 |
| @auto annotation                     | Removed, redundant with default behavior              | 2026-05-11 |
| L1 auto fallback                     | Removed, behavior unpredictable                       | 2026-05-11 |

---

## Appendix: Glossary

| Term        | Definition                                                                |
| ----------- | ------------------------------------------------------------------------- |
| spawn model | YaoXiang's concurrency paradigm: synchronous syntax, asynchronous essence |
| DAG         | Directed Acyclic Graph, describing computational dependencies             |
| spawn       | Controllable concurrency within @block scope                              |
| @block      | Synchronous annotation, disables concurrency optimization                 |
| @eager      | Eager evaluation, waits for dependencies to complete                      |
| Resource    | Resource type marker, operations automatically build DAG dependencies     |
| error graph | Visualized error propagation path                                         |

## References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://golang.org/doc/effective_go#concurrency)
- [Work-stealing scheduling](https://en.wikipedia.org/wiki/Work_stealing)
- Spawn model whitepaper
- [YaoXiang Language Specification](../../reference/language-spec/index.md)
