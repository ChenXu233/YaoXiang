---
title: "RFC-001: Spawn Model and Error Handling System"
---

# RFC-001: Spawn Model and Error Handling System

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-05
> **Last Updated**: 2026-05-11 (Pruning: Removed @auto, L1 fallback heuristics, streamlined discussion log)

## Design Sources

| Document | Relationship |
|----------|--------------|
| [async-whitepaper](/src/archive/async-whitepaper) | Design origin, theoretical foundation |
| [language-spec](/src/design/language-spec) | Specification target |

## Abstract

Proposes YaoXiang's spawn model: describe logic with synchronous syntax, runtime automatically executes concurrently. Core mechanisms: three-tier concurrency architecture + DAG dependency analysis + Result type system.

## Quick Reference

| Scenario | Syntax | Description |
|----------|--------|-------------|
| Auto-parallel | No annotation (default) | Maximize parallelism |
| Synchronous wait | `@eager` | Wait for dependencies to complete |
| Fully sequential | `@block` | No concurrency, for debugging |
| Local concurrency | `spawn` | Concurrency within @block scope |

## Motivation

Current mainstream language concurrency models have significant flaws:

| Language | Concurrency Model | Issues |
|----------|-------------------|--------|
| Rust | async/await + tokio | Async contagion, steep learning curve |
| Go | goroutine | No type safety |
| Python | asyncio | GIL limitations |
| JavaScript | Promise/async | Callback complexity |

### Core Conflicts

1. **Transparency vs. Controllability**: Fully transparent but uncontrollable vs. fully controllable but opaque
2. **Concurrency vs. Debuggability**: Concurrent programs are hard to debug vs. debuggable programs are hard to parallelize

---

## Proposal

### 1. Spawn Model: Three-Tier Concurrency Architecture

> **Note**: L1/L2/L3 are mental models to help users understand different scenarios. The actual implementation has only one mechanism: automatic DAG analysis + annotation control.

| Level | Mental Model | Syntax | Execution Mode | Parallelism |
|-------|--------------|--------|----------------|-------------|
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
    # Wait for all spawns to complete (stdlib controlled)
    process(data1, data2)
}
```

#### L3: Fully Transparent (Default)

```yaoxiang
# No annotations needed, compiler automatically analyzes DAG
heavy_calc: (n: Int) -> Int = fibonacci(n)

auto_parallel: (n: Int) -> Int = {
    a = heavy_calc(1)    # Auto-parallel
    b = heavy_calc(2)    # Auto-parallel
    c = heavy_calc(3)    # Auto-parallel
    a + b + c            # Wait for all results when values are needed
}
```

### 2. Annotation Complete Comparison

| Dimension | Default (no annotation) | `@eager` | `@block` | `spawn` |
|-----------|------------------------|----------|----------|---------|
| **Execution Mode** | Automatic DAG analysis | Synchronous wait for dependencies | Pure sequential | Concurrency within @block |
| **Parallelism** | ✅ Full | ⚠️ Dependency-ordered | ❌ None | ⚠️ Partial |
| **DAG Construction** | ✅ | ✅ | ❌ | ✅ |

**Selection Guide**:
- Maximum concurrency → No annotation (default)
- Need ordered side effects → `@eager`
- Debugging/beginners/critical code → `@block`
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
    a = heavy_calc(1)    # Forced synchronous
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
    fetch(url1) no one needs → isolated DAG

Runtime scheduling (from leaves):
    fetch(url0) → print(a)    ← dependency chain, in order
    fetch(url1)                ← isolated, independent parallel
```

**Key Insight**: Not "top-down" Future generation, but "bottom-up" reverse analysis of dependencies from results.

#### 3.2 Isolated DAG: Independent Parallelism

```
Main flow: fetch(url0) → process → print
Isolated:  fetch(url1)  ← no one needs result, independent parallel

Scheduler: main flow executes by dependency chain, isolated uses another core in parallel
```

#### 3.3 Resource Types and Side Effects

**Core Idea**: Resource operations are marked via types, DAG automatically constructs dependencies. Same resource automatically serializes, different resources automatically parallelize.

**Resource Type Boundaries—Clear Definition**:

Resource types are compiler-built-in marked types. The following types are recognized as resources by the compiler:

| Resource Type | Description | Compiler Behavior |
|---------------|-------------|------------------|
| `FilePath` | File system path | Same path operations auto-serialize |
| `HttpUrl` | HTTP endpoint | Same URL operations auto-serialize |
| `DBUrl` | Database connection | Same connection operations auto-serialize |
| `Console` | Standard output | All Console operations auto-serialize |

User-defined resource types require explicit marking:
```yaoxiang
Database: Resource              # Explicitly marked as resource type
query: (Database, String) -> Result(Row, Error)
# Parameter Database is Resource, auto-recognized as resource operation
```

Types not marked as Resource are not tracked by the compiler for resource dependencies.

**Usage Rules**:
- Pass resource handles via variables, DAG auto-manages ordering
- literal direct usage of same resource is user design issue, not language responsibility

```yaoxiang
# ✅ Correct: variable passing, DAG auto-serializes
filename: String = "data.txt"
File.write(filename, x)
File.write(filename, y)    # DAG serializes

# ⚠️ User responsibility: literal
File.write("data.txt", x)
File.write("data.txt", y)  # May parallelize, user's own responsibility
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

## Trade-offs

### Advantages

1. **Gradual Adoption**: Three-tier model adapts to different skill levels
2. **Natural Syntax**: Synchronous code gains parallel performance
3. **Compile-Time Safety**: Send/Sync constraints eliminate data races
4. **Debuggable**: Error graph provides clear view of error propagation

### Disadvantages

1. **Learning Curve**: Need to understand DAG dependency concept
2. **Compile Time**: Full-program DAG analysis may be slow
3. **Toolchain Complexity**: Need brand new debugging and visualization tools

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| Explicit async/await only | Cannot achieve transparent concurrency |
| Fully transparent concurrency only | Users lose control |
| Go-style goroutine | No type safety, no compile-time checking |
| L1 mode only | Abandons core value of spawn model |

## Implementation Strategy

### Phase Breakdown

1. **Phase 1 (v0.1)**: @block synchronous mode, basic types
2. **Phase 2 (v0.2)**: FlowScheduler dispatcher
3. **Phase 3 (v0.3)**: spawn block, explicit concurrency
4. **Phase 4 (v0.5)**: L3 full transparency, automatic DAG analysis
5. **Phase 5 (v0.6)**: Error graph, graph debugger
6. **Phase 6 (v1.0)**: Production-ready optimization

### Dependencies

- RFC-001 has no external dependencies (core foundation)
- RFC-008 (Runtime Concurrency Model) → Design complete
- RFC-011 (Generics System) → Design complete

### Risks

1. **DAG Analysis Performance**: Full-program analysis may be O(n²), needs optimization
2. **Toolchain Absence**: Debugger needs to be developed from scratch
3. **User Adoption**: Transparent concurrency needs good documentation

---

## Design Decision Log

| Decision | Decision Made | Date |
|----------|---------------|------|
| Three-tier concurrency architecture | L1/L2/L3 progressive | 2025-01-05 |
| @block annotation position | After return type | 2025-01-05 |
| DAG error propagation | Propagate along dependency edges upstream | 2025-01-06 |
| DAG performance optimization | Incremental construction + caching | 2025-01-06 |
| Runtime selection | Generics + compile-time injection | 2025-01-06 |
| Node interface | Generics + function injection (no trait) | 2025-01-06 |
| Error graph memory | DAG only constructed within single function | 2025-01-06 |
| Resource conflict detection | DAG data flow dependency, user variable passing | 2025-01-06 |
| Resource type system | Resource marking + DAG auto-dependency | 2026-01-06 |
| L1/L2/L3 mental model | Three-tier abstraction, not implementation mechanism | 2026-01-06 |
| @auto annotation | Removed, duplicates default behavior | 2026-05-11 |
| L1 auto-fallback | Removed, behavior unpredictable | 2026-05-11 |

---

## Appendix: Glossary

| Term | Definition |
|------|------------|
| Spawn Model | YaoXiang's concurrency paradigm: synchronous syntax, asynchronous nature |
| DAG | Directed Acyclic Graph, describes computational dependency relationships |
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