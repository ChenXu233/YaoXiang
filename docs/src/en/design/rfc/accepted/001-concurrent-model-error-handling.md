---
title: RFC-001: Spawn Model and Error Handling System
---

# RFC-001: Spawn Model and Error Handling System

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-05-11 (pruning: removed @auto, L1 fallback heuristics, streamlined discussion log)

## Design Sources

| Document | Relationship |
|----------|--------------|
| [async-whitepaper](/src/archive/async-whitepaper) | Design origin, theoretical foundation |
| [language-spec](/src/design/language-spec) | Specification target |

## Abstract

This RFC proposes YaoXiang's spawn model: describing logic in synchronous syntax while the runtime automatically executes concurrently. Core mechanism: three-layer concurrency architecture + DAG dependency analysis + Result type system.

## Quick Selection

| Scenario | Syntax | Description |
|----------|--------|-------------|
| Automatic parallel | No annotation (default) | Maximizes parallelism |
| Synchronous wait | `@eager` | Wait for dependencies to complete |
| Fully sequential | `@block` | No concurrency, for debugging |
| Local concurrency | `spawn` | Concurrency within @block scope |

## Motivation

Current mainstream languages' concurrency models have clear flaws:

| Language | Concurrency Model | Issues |
|----------|-------------------|--------|
| Rust | async/await + tokio | Async contagion, steep learning curve |
| Go | goroutine | No type safety |
| Python | asyncio | GIL limitations |
| JavaScript | Promise/async | Callback complexity |

### Core Tensions

1. **Transparency vs. Controllability**: Fully transparent but uncontrollable vs. fully controllable but opaque
2. **Concurrency vs. Debuggability**: Concurrent programs are hard to debug vs. debuggable programs are hard to parallelize

---

## Proposal

### 1. Spawn Model: Three-Layer Concurrency Architecture

> **Note**: L1/L2/L3 are mental models to help users understand different scenarios. The actual implementation has only one mechanism: automatic DAG analysis + annotation control.

| Layer | Mental Model | Syntax | Execution | Parallelism |
|-------|--------------|--------|-----------|-------------|
| **L1** | Concurrency forbidden | `@block` | Pure sequential execution | ❌ None |
| **L2** | Concurrency within @block | `spawn` | Controllable concurrency within @block scope | ⚠️ Partial |
| **L3** | Full concurrency | Default (no annotation) | Automatic DAG analysis | ✅ Full |

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
    # Wait for all spawns to complete (standard library controlled)
    process(data1, data2)
}
```

#### L3: Fully Transparent (Default)

```yaoxiang
# No annotations needed, compiler automatically analyzes DAG
heavy_calc: (n: Int) -> Int = fibonacci(n)

auto_parallel: (n: Int) -> Int = {
    a = heavy_calc(1)    # Automatic parallel
    b = heavy_calc(2)    # Automatic parallel
    c = heavy_calc(3)    # Automatic parallel
    a + b + c            # Wait for all results when values are needed
}
```

### 2. Annotation Complete Comparison

| Dimension | Default (no annotation) | `@eager` | `@block` | `spawn` |
|-----------|------------------------|----------|----------|---------|
| **Execution** | Automatic DAG analysis | Synchronous wait for dependencies | Pure sequential | Concurrency within @block |
| **Parallelism** | ✅ Full | ⚠️ In dependency order | ❌ None | ⚠️ Partial |
| **DAG Construction** | ✅ | ✅ | ❌ | ✅ |

**Selection Guide**:
- Maximum concurrency → No annotation (default)
- Need ordered side effects → `@eager`
- Debugging/beginners/critical code → `@block`
- Need concurrency within @block → `spawn`

```yaoxiang
# Default: maximize parallelism
calc_all: () -> Int = {
    a = heavy_calc(1)    # Automatic parallel
    b = heavy_calc(2)    # Automatic parallel
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

#### 3.1 Core Principle: Bottom-Up Execution

```
User code (synchronous syntax):
    a = fetch(url0)
    b = fetch(url1)
    print(a)

Compile-time analysis (bottom-up):
    print(a) needs a → depends on fetch(url0)
    fetch(url1) nobody needs → isolated DAG

Runtime scheduling (starting from leaves):
    fetch(url0) → print(a)    ← dependency chain, in order
    fetch(url1)                ← isolated, parallel independently
```

**Key Insight**: Not "top-down" generating Futures, but "bottom-up" reverse analysis of dependencies from results.

#### 3.2 Isolated DAG: Independent Parallelism

```
Main flow: fetch(url0) → process → print
Isolated:  fetch(url1)  ← nobody needs the result, parallel independently

Scheduler: main flow executes by dependency chain, isolated uses another core in parallel
```

#### 3.3 Resource Types and Side Effects

**Core Idea**: Resource operations are marked through types, DAG automatically constructs dependencies. Same resource operations are automatically serialized, different resources are automatically parallelized.

**Resource Type Boundaries — Clear Definition**:

Resource types are compiler-built-in marked types. The following types are recognized by the compiler as resources:

| Resource Type | Description | Compiler Behavior |
|---------------|-------------|-------------------|
| `FilePath` | Filesystem path | Same-path operations auto-serialized |
| `HttpUrl` | HTTP endpoint | Same URL operations auto-serialized |
| `DBUrl` | Database connection | Same connection operations auto-serialized |
| `Console` | Standard output | All Console operations auto-serialized |

User-defined resource types require explicit marking:
```yaoxiang
Database: Resource              # Explicitly mark as resource type
query: (Database, String) -> Result(Row, Error)
# Parameter Database is Resource, automatically recognized as resource operation
```

Types not marked as Resource are not tracked by the compiler for resource dependencies.

**Usage Rules**:
- Pass resource handles through variables, DAG automatically manages order
- Using literals directly with the same resource is a user design issue, not the language's responsibility

```yaoxiang
# ✅ Correct: variable passing, DAG auto-serializes
filename: String = "data.txt"
File.write(filename, x)
File.write(filename, y)    # DAG serializes

# ⚠️ User responsibility: literals
File.write("data.txt", x)
File.write("data.txt", y)  # May be parallel, user is responsible
```

#### 3.4 Infinite Loop Handling

```
1 loop → Direct synchronous execution, zero scheduling overhead
Multiple loops → Scheduler slices and switches, true concurrency
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

| Edge Type | Symbol | Semantics |
|-----------|--------|-----------|
| DataEdge | → | Data dependency (value flow) |
| ControlEdge | ● | Control dependency (sequential execution) |
| SpawnEdge | ◎ | Concurrency entry (parallelizable starting point) |

### 6. Type System

```
Send → Can safely transfer across threads
Sync → Can safely share across threads
Arc(T) implements Send + Sync (thread-safe reference counting)
```

---

## Tradeoffs

### Advantages

1. **Progressive Adoption**: Three-layer model adapts to different skill levels
2. **Natural Syntax**: Synchronous code gains parallel performance
3. **Compile-time Safety**: Send/Sync constraints eliminate data races
4. **Debuggability**: Error graph provides clear view of error propagation

### Disadvantages

1. **Learning Curve**: Need to understand DAG dependency concepts
2. **Compile Time**: Whole-program DAG analysis may be slow
3. **Toolchain Complexity**: Need brand new debugging and visualization tools

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| Only explicit async/await | Cannot achieve transparent concurrency |
| Only fully transparent concurrency | Users lose control |
| Go-style goroutine | No type safety, no compile-time checking |
| Only L1 mode | Abandon core value of spawn model |

## Implementation Strategy

### Phase Breakdown

1. **Phase 1 (v0.1)**: @block synchronous mode, basic types
2. **Phase 2 (v0.2)**: FlowScheduler scheduler
3. **Phase 3 (v0.3)**: spawn block, explicit concurrency
4. **Phase 4 (v0.5)**: L3 full transparency, automatic DAG analysis
5. **Phase 5 (v0.6)**: Error graph, graph debugger
6. **Phase 6 (v1.0)**: Production-ready optimization

### Dependencies

- RFC-001 has no external dependencies (base core)
- RFC-008 (Runtime Concurrency Model) → Design complete
- RFC-011 (Generics System) → Design complete

### Risks

1. **DAG Analysis Performance**: Whole-program analysis may be O(n²), needs optimization
2. **Missing Toolchain**: Debugger needs to be developed from scratch
3. **User Adoption**: Transparent concurrency needs good documentation

---

## Design Decision Log

| Decision | Resolution | Date |
|----------|------------|------|
| Three-layer concurrency architecture | L1/L2/L3 progressive | 2025-01-05 |
| @block annotation position | After return type | 2025-01-05 |
| DAG error propagation | Propagate upstream along dependency edges | 2025-01-06 |
| DAG performance optimization | Incremental construction + caching | 2025-01-06 |
| Runtime selection | Generics + compile-time injection | 2025-01-06 |
| Node interface | Generics + function injection (no trait) | 2025-01-06 |
| Error graph memory | DAG only constructed within single function | 2025-01-06 |
| Resource conflict detection | DAG data flow dependency, user variable passing | 2025-01-06 |
| Resource type system | Resource marker + DAG auto-dependency | 2026-01-06 |
| L1/L2/L3 mental model | Three-layer abstraction, not implementation mechanism | 2026-01-06 |
| @auto annotation | Removed, duplicates default behavior | 2026-05-11 |
| L1 auto-fallback | Removed, behavior unpredictable | 2026-05-11 |

---

## Appendix: Glossary

| Term | Definition |
|------|------------|
| Spawn Model | YaoXiang's concurrency paradigm: synchronous syntax, async nature |
| DAG | Directed Acyclic Graph, describes computation dependency relationships |
| spawn | Controllable concurrency within @block scope |
| @block | Synchronous annotation, disables concurrency optimization |
| @eager | Eager evaluation, wait for dependencies to complete |
| Resource | Resource type marker, operations auto-construct DAG dependencies |
| Error Graph | Visualized error propagation path |

## References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go Concurrency Patterns](https://golang.org/doc/effective_go#concurrency)
- [Work Stealing Scheduling](https://en.wikipedia.org/wiki/Work_stealing)
- [Spawn Model Whitepaper](/src/archive/async-whitepaper)
- [YaoXiang Language Specification](/src/design/language-spec)