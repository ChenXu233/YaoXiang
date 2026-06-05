```md
---
title: "RFC-024: Concurrency Model Based on Spawn Blocks"
---

# RFC-024: Concurrency Model Based on Spawn Blocks

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2026-06-05
> **Last Updated**: 2026-06-05

> **References**:
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model Decoupled from Scheduler](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)

## Abstract

This document defines the new concurrency model for YaoXiang programming language: using `spawn {}` blocks as the sole parallelism primitive, dependency-driven execution, and synchronous blocking by the caller. It replaces the old concurrency approach based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` traits, and whole-program DAG analysis.

**Core Design—One Primitive, One Rule**:

```
spawn { ... }        ← Sole parallelism primitive
Direct child assignments create tasks    ← Sole rule
Synchronous blocking for results         ← Sole behavior
```

**Eliminated Complexity**:
- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The ordinary code you write executes sequentially. When you want multiple things to happen together, put them inside a `spawn { ... }` block. No callbacks, no `await`, no strange annotations.

## Design Sources

| Document | Relationship |
|----------|--------------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document |
| [RFC-008](./008-runtime-concurrency-model.md) | Runtime architecture, orthogonal to this document |
| [RFC-009](./009-ownership-model.md) | Ownership model, unchanged |
| [RFC-010](./010-unified-type-syntax.md) | Unified type syntax, return rules updated |
| [Concurrency Model Specification](/reference/language-spec/concurrency.md) | Formal specification reference for this document |

## Motivation

### Why Is This Design Needed?

Current mainstream language concurrency models have obvious flaws:

| Language | Concurrency Model | Problems |
|----------|-------------------|----------|
| Rust | async/await + tokio | Async infection, function coloring, steep learning curve |
| Go | goroutine | No type safety, data races hard to detect |
| Python | asyncio | GIL limitations, function coloring |
| JavaScript | Promise/async | Callback hell, function coloring |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem | Description |
|---------|-------------|
| Complex mental model | L1/L2/L3 three-layer abstraction increases learning burden |
| Redundant annotations | `@block`/`@eager`/`@auto` annotations make code noisy |
| High analysis complexity | Whole-program DAG analysis has significant compile-time overhead |
| Complex type constraints | `Send`/`Sync` trait increases cognitive load |
| Uncontrollable | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallelism primitive (`spawn`), one rule (direct child assignments create tasks)
2. **Explicit**: Users know exactly where parallelism occurs and where execution is sequential
3. **Safe**: Ownership rules extend naturally, no additional type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of {} Blocks: A Dependency-Driven Computation Unit

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|----------|-------------|
| Dependency-driven | When executing, the block checks if all internal variables are ready; if complete, it executes immediately, otherwise it blocks and waits |
| Execution timing | Determined by dependencies, not related to "immediate" or "delayed" |
| Return value | Use `return` for explicit return; default returns `Void` when no `return` |
| Uniform syntax | Consistent semantics whether appearing in function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly limited within `{}`, not leaking to outer scope |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately when both are ready
    return x + y
}
```

### 2. Spawn Block Semantics

`spawn { ... }` is the **sole parallelism primitive** in YaoXiang.

#### 2.1 Core Rules

- **Direct child assignments** within spawn blocks create parallel tasks
- Assignments inside nested `{}` are not considered independent tasks
- The entire spawn block blocks synchronously, waiting for all tasks to complete before returning results
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks execute in parallel
(a, b) = spawn {
    fetch("url1"),      // Task 1
    fetch("url2")       // Task 2
}
// Continue after both complete
```

#### 2.2 Scope Isolation

Spawn blocks create independent scopes; internal variables do not affect the outside:

```yaoxiang
x = 10
result = spawn {
    x = 20              // This is the local x inside the spawn block
    compute(x)
}
// x is still 10
```

#### 2.3 Ownership Rules

Once a variable enters a spawn block, it cannot be used externally (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data's ownership moves into the spawn block
}
// data is unavailable here (already moved)
```

If sharing across multiple tasks is needed, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

#### 2.4 Error Handling

Error propagation within spawn blocks follows these rules:

1. Wait for all tasks to complete (even if some have failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If either task fails, the spawn block propagates the first error
```

#### 2.5 Resource Types

The compiler tracks resource type usage to ensure concurrency safety:

| Resource Type | Description | Compiler Behavior |
|---------------|-------------|-------------------|
| `FilePath` | File system path | Same-path operations automatically serialized |
| `HttpUrl` | HTTP endpoint | Same-URL operations automatically serialized |
| `DBUrl` | Database connection | Same-connection operations automatically serialized |
| `Console` | Standard output | All Console operations automatically serialized |

```yaoxiang
// Same-file operations are automatically serialized
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

#### 2.7 Nested Spawn

Spawn blocks can be nested; inner spawn blocks create new concurrency domains:

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

### 3. Breaking from the Old Design

| Old Design (RFC-001) | New Design (RFC-024) |
|---------------------|---------------------|
| Whole-program automatic DAG analysis | Analysis only within spawn blocks |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven |
| `Send`/`Sync` trait | Unnecessary; ownership + ref handles it |
| `future`/non-blocking handles | Synchronous blocking, no callbacks |
| `Mutex`/`RwLock`/`Atomic` | `ref` automatically chooses Rc/Arc |
| L1/L2/L3 three-layer mental model | Normal code is sequential, spawn blocks are parallel |
| Function coloring (async/await) | No function coloring |

### 4. Return Rules

YaoXiang's return rules are uniform and clear:

| Syntax | Return Value | Description |
|--------|--------------|-------------|
| `= expr` (no braces) | Directly returns `expr` | Expression is the value |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Block requires explicit return |

```yaoxiang
// No braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// With braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// With braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // No return, returns Void
}
```

### 5. User Mental Model

> **The ordinary code you write executes sequentially.**
>
> **When you want multiple things to happen together, put them inside a `spawn { ... }` block.**
>
> Each direct assignment in the block starts immediately (in parallel), and the result you need will be automatically waited for.
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
    fetch("url1"),      // Parallel
    fetch("url2"),      // Parallel
    fetch("url3")       // Parallel
}
// Continue after all complete
process(x, y, z)
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallelism primitive (`spawn`), one rule (direct child assignments create tasks)
2. **Explicit**: Users know exactly where parallelism occurs and where execution is sequential; no implicit concurrency
3. **Safe**: Ownership rules extend naturally; no additional type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallelism behavior; avoids unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously; code is easy to understand and debug
6. **No function coloring**: No async/await function coloring problems
7. **Efficient compilation**: DAG analysis limited to spawn blocks; compile time is controllable

### Disadvantages

1. **Requires explicit spawn**: Cannot automatically parallelize; users need to manually mark parallelism points
2. **DAG analysis within spawn blocks**: Compiler needs to perform dependency analysis within spawn blocks
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs migration

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| Whole-program automatic DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior |
| async/await | Function coloring, steep learning curve, poor code readability |
| goroutine | No type safety, data races hard to detect |
| Actor model | Message passing is complex, difficult to debug |
| CSP (Go channels) | No type safety, deadlocks hard to detect |

---

## Implementation Strategy

### Compile-Time Analysis

1. **DAG construction**: Analyze dependency relationships within spawn blocks
2. **Topological sorting**: Determine execution order within spawn blocks
3. **Parallelism identification**: Identify subtrees within spawn blocks with no dependencies
4. **Escape analysis**: `ref` → Rc or Arc
5. **Resource conflict detection**: Detect potential conflicts for resource types

### Runtime Execution

Refer to the Runtime architecture in [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No spawn support, immediate execution
- **Standard Runtime**: Supports spawn blocks, concurrency within spawn blocks
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generic type system) → Completed

---

## Design Decision Record

| Decision | Decision Made | Reason | Date |
|----------|---------------|--------|------|
| Parallelism primitive | `spawn {}` block | Simple, explicit, controllable | 2026-06-05 |
| Task creation | Direct child assignments | Clear, unambiguous | 2026-06-05 |
| Execution model | Synchronous blocking | Easy to understand, debug | 2026-06-05 |
| DAG analysis scope | Only within spawn blocks | Efficient compilation, controllable behavior | 2026-06-05 |
| Sharing mechanism | `ref` automatically chooses Rc/Arc | Simplifies user decisions | 2026-06-05 |
| Annotations | None | Reduce code noise | 2026-06-05 |
| Send/Sync | Removed | Ownership + ref is sufficient | 2026-06-05 |
| Mutex/RwLock | Removed | ref handles automatically | 2026-06-05 |
| future/handles | Removed | Synchronous blocking is simpler | 2026-06-05 |
| Function coloring | None | Avoid async/await problems | 2026-06-05 |
| Error propagation | Wait for all tasks, propagate first error | Deterministic behavior | 2026-06-05 |
| Resource types | Built-in + user-defined | Automatic serialization | 2026-06-05 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](/reference/language-spec/concurrency.md)
- [RFC-001 Concurrency Model (Deprecated)](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic Type System](./011-generic-type-system.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |
```