---
title: "RFC-024: Concurrency Model Based on spawn Blocks"
status: "Accepted"
author: "Chenxu"
created: "2026-06-05"
updated: "2026-06-11 (Added spawn module organization and migration notes)"
---

# RFC-024: Concurrency Model Based on spawn Blocks

> **References**:
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)

## Abstract

This document defines the new concurrency model for the YaoXiang programming language: using `spawn {}` blocks as the sole parallel primitive, with dependency-driven execution and synchronous blocking on the caller side. It replaces the old concurrency scheme based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` traits, and whole-program DAG analysis.

**Core Design — One Primitive, One Rule**:

```
spawn { ... }        ← The sole parallel primitive
Direct sub-expressions create tasks  ← The sole rule
Synchronously block and wait for results  ← The sole behavior
```

**Complexity Eliminated**:
- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` traits
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The ordinary code you write executes sequentially. When you want multiple things to happen together, put them inside `spawn { ... }` blocks. No callbacks, no `await`, no strange annotations.

## Design Origins

| Document | Relationship |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document |
| [RFC-008](./008-runtime-concurrency-model.md) | Runtime architecture, orthogonal to this document |
| [RFC-009](./009-ownership-model.md) | Ownership model, unchanged |
| [RFC-010](./010-unified-type-syntax.md) | Unified type syntax, return rules updated |
| [Concurrency Model Specification](/reference/language-spec/concurrency.md) | The formal specification reference for this document |

## Motivation

### Why This Design?

Current mainstream language concurrency models have obvious flaws:

| Language | Concurrency Model | Problems |
|------|----------|------|
| Rust | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go | goroutine | No type safety, data races hard to detect |
| Python | asyncio | GIL limitation, function coloring |
| JavaScript | Promise/async | Callback hell, function coloring |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem | Description |
|------|------|
| Complex mental model | L1/L2/L3 three-layer abstraction increases learning burden |
| Annotation redundancy | `@block`/`@eager`/`@auto` annotations make code noisy |
| High analysis complexity | High compile-time overhead from whole-program DAG analysis |
| Complex type constraints | `Send`/`Sync` traits add cognitive load |
| Uncontrollable | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), one rule (direct sub-expressions create tasks)
2. **Explicit**: Users clearly know where parallelism happens and where it's sequential
3. **Safe**: Ownership rules extend naturally, no extra type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of {} Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|------|------|
| Dependency-driven | The block checks whether all internal variables are ready when executing; runs immediately if all are ready, otherwise blocks and waits |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "deferred" |
| Return value | Use `return` to explicitly return a value; when no `return`, defaults to `Void` |
| Unified syntax | Semantics are consistent whether appearing in function bodies, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly confined inside `{}`, do not leak to outer scopes |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x ready
y = compute_y()        // y ready
result = {
    // Depends on x and y, executes immediately once both are ready
    return x + y
}
```

### 2. spawn Block Semantics

`spawn { ... }` is the **sole parallel primitive** in YaoXiang.

#### 2.1 Core Rules

- **Direct sub-expressions** of a spawn block create parallel tasks
- Expressions inside nested `{}` do not count as independent tasks
- spawn blocks follow the standard return rules: must use `return` to explicitly return a value, returns `Void` when no `return`
- The entire spawn block blocks synchronously, returning after all tasks complete
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks executing in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // direct sub-expression → parallel task 1
    t2 = fetch("url2")   // direct sub-expression → parallel task 2
    return (t1, t2)      // explicitly return a tuple
}

// Expressions inside nested {} are not direct sub-expressions
result = spawn {
    x = {               // This entire block is a direct sub-expression → one task
        inner_work()    // Not a direct sub-expression of spawn, won't become an independent task
    },
    process(x)          // direct sub-expression → parallel task
    return process(x)
}

#### 2.2 Scope Isolation

spawn blocks create independent scopes; internal variables do not affect the outside:

```yaoxiang
x = 10
result = spawn {
    x = 20              // This is the local x inside the spawn block
    compute(x)
}
// x is still 10
```

#### 2.3 Ownership Rules

Once a variable enters a spawn block, it cannot be used outside anymore (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into the spawn block
}
// data is unavailable here (already moved)
```

If sharing is needed across multiple tasks, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

#### 2.4 Error Handling

Error propagation inside spawn blocks follows these rules:

1. Wait for all tasks to complete (even if some have already failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If any task fails, the entire spawn block propagates the first error
```

#### 2.5 Resource Types

The compiler tracks the usage of resource types to ensure concurrency safety:

| Resource Type | Description | Compiler Behavior |
|----------|------|-----------|
| `FilePath` | File system path | Operations on the same path are automatically serialized |
| `HttpUrl` | HTTP endpoint | Operations on the same URL are automatically serialized |
| `DBUrl` | Database connection | Operations on the same connection are automatically serialized |
| `Console` | Standard output | All Console operations are automatically serialized |

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

#### 2.6 spawn for: Data-Parallel Loop

```yaoxiang
// Process each element in the list in parallel
results = spawn for item in items {
    result = process(item)
}
```

#### 2.7 Nested spawn

spawn blocks can be nested; inner spawn blocks create new concurrency domains:

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

### 3. Break with the Old Design

| Old Design (RFC-001) | New Design (RFC-024) |
|------------------|------------------|
| Whole-program automatic DAG analysis | Analysis only inside spawn blocks |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven |
| `Send`/`Sync` traits | Not needed, ownership + ref handles it automatically |
| `future`/non-blocking handles | Synchronous blocking, no callbacks |
| `Mutex`/`RwLock`/`Atomic` | `ref` automatically chooses Rc/Arc |
| L1/L2/L3 three-layer mental model | Ordinary code is sequential, spawn blocks are parallel |
| Function coloring (async/await) | No function coloring |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

| Notation | Return Value | Description |
|------|--------|------|
| `= expr` (no curly braces) | Returns `expr` directly | Expression is the value |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` | Blocks require explicit return |

```yaoxiang
// No curly braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With curly braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// With curly braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 5. User Mental Model

> **The ordinary code you write executes sequentially.**
>
> **When you want multiple things to happen together, put them inside `spawn { ... }` blocks.**
>
> Each direct sub-expression in the block starts immediately (in parallel), and you use `return` to explicitly return the result.
> The entire block waits for everything to finish, then gives you the final result.
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// Ordinary code: sequential execution
a = compute_a()         // Executes first
b = compute_b(a)        // Depends on a, executes after a completes
c = compute_c(b)        // Depends on b, executes after b completes

// When parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // parallel
    fetch("url2"),      // parallel
    fetch("url3")       // parallel
}
// Continue after waiting for all to complete
process(x, y, z)
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallel primitive (`spawn`), one rule (direct sub-expressions create tasks)
2. **Explicit**: Users clearly know where parallelism happens and where it's sequential; no implicit concurrency
3. **Safe**: Ownership rules extend naturally, no extra type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallel behavior, avoiding unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No function coloring problem like async/await
7. **Compilation efficient**: DAG analysis is limited to inside spawn blocks, compile time is controllable

### Disadvantages

1. **Explicit spawn required**: Cannot auto-parallelize; users must manually mark parallel points
2. **DAG analysis inside spawn blocks**: Compiler needs to perform dependency analysis inside spawn blocks
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs migration

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| Whole-program automatic DAG (RFC-001) | High complexity, long compile time, uncontrollable behavior |
| async/await | Function coloring, steep learning curve, poor code readability |
| goroutine | No type safety, data races hard to detect |
| Actor model | Complex message passing, difficult to debug |
| CSP (Go channel) | No type safety, deadlocks hard to detect |

---

## Implementation Strategy

### Compile-time Analysis

1. **DAG Construction**: Analyze dependency relationships inside spawn blocks
2. **Topological Sort**: Determine execution order inside spawn blocks
3. **Parallelism Identification**: Identify subtrees without dependencies inside spawn blocks
4. **Escape Analysis**: `ref` → Rc or Arc
5. **Resource Conflict Detection**: Detect potential conflicts on resource types

### Module Organization

spawn-related code is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry
├── placement.rs     # spawn occurrence location legality check
└── analysis.rs      # Task identification, dependency analysis, resource conflict detection (required for RFC-018 phase 4)
```

> **Migration Note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs` will be migrated to `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration under `typecheck/passes/` must be removed simultaneously. This migration is driven by RFC-018 (LLVM AOT compiler) — the LLVM backend needs to consume spawn analysis results; spawn analysis as an independent frontend shared module is more reasonable than being embedded in typecheck.

### Runtime Execution

Referencing the Runtime architecture in [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No spawn support, executes immediately
- **Standard Runtime**: Supports spawn blocks, concurrency inside spawn blocks
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generic system) → Completed

---

## Design Decision Record

| Decision | Determination | Reason | Date |
|------|------|------|------|
| Parallel primitive | `spawn {}` block | Simple, explicit, controllable | 2026-06-05 |
| Task creation | Direct sub-expression | Clear, unambiguous | 2026-06-05 |
| Execution model | Synchronous blocking | Easy to understand and debug | 2026-06-05 |
| DAG analysis scope | Inside spawn blocks only | Compilation efficient, behavior controllable | 2026-06-05 |
| Sharing mechanism | `ref` auto-select Rc/Arc | Simplifies user decisions | 2026-06-05 |
| Annotations | None | Reduces code noise | 2026-06-05 |
| Send/Sync | Removed | Ownership + ref is sufficient | 2026-06-05 |
| Mutex/RwLock | Removed | `ref` handles automatically | 2026-06-05 |
| future/handle | Removed | Synchronous blocking is simpler | 2026-06-05 |
| Function coloring | None | Avoids async/await problems | 2026-06-05 |
| Error propagation | Wait for all tasks, propagate the first error | Deterministic behavior | 2026-06-05 |
| Resource types | Built-in + user-defined | Automatic serialization | 2026-06-05 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](/reference/language-spec/concurrency.md)
- [RFC-001 Concurrency Model (Deprecated)](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic System](./011-generic-type-system.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |