---
title: "RFC-024: Concurrency Model Based on spawn Blocks"
---

# RFC-024: Concurrency Model Based on spawn Blocks

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2026-06-05
> **Last Updated**: 2026-06-05

> **References**:
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model Decoupled from Scheduler](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)

## Summary

This document defines the new concurrency model for the YaoXiang programming language: using `spawn {}` blocks as the sole parallelism primitive, dependency-driven execution, and synchronous blocking by the caller. This replaces the old concurrency scheme based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` traits, and whole-program DAG analysis.

**Core Design—One Primitive, One Rule**:

```
spawn { ... }        ← sole parallelism primitive
Direct sub-expressions create tasks  ← sole rule
Synchronous blocking for results     ← sole behavior
```

**Complexity Eliminated**:
- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: Your regular code executes sequentially. When you want multiple things to happen together, put them inside a `spawn { ... }` block. No callbacks, no `await`, no weird annotations.

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

Current mainstream languages have significant concurrency model flaws:

| Language | Concurrency Model | Problems |
|----------|-------------------|----------|
| Rust | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go | goroutine | No type safety, data races hard to detect |
| Python | asyncio | GIL limitations, function coloring |
| JavaScript | Promise/async | Callback hell, function coloring |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem | Description |
|---------|-------------|
| Complex mental model | L1/L2/L3 triple abstraction increases learning burden |
| Verbose annotations | `@block`/`@eager`/`@auto` annotations make code noisy |
| High analysis complexity | Whole-program DAG analysis has significant compile-time overhead |
| Complex type constraints | `Send`/`Sync` trait increase cognitive load |
| Uncontrollable | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallelism primitive (`spawn`), one rule (direct sub-expressions create tasks)
2. **Explicit**: Users clearly know where parallelism occurs and where execution is sequential
3. **Safe**: Ownership rules extend naturally, no extra type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Callers block synchronously, no callbacks or `await`

---

## Proposal

### 1. The Nature of {} Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|----------|-------------|
| Dependency-driven | When executing, the block checks if all internal variables are ready; if ready, executes immediately, otherwise blocks and waits |
| Execution timing | Determined by dependencies, not related to "immediate" or "delayed" |
| Return value | Explicitly returned using `return`; defaults to `Void` when no `return` |
| Unified syntax | Semantics are consistent whether appearing in function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly limited to inside `{}`, not leaking to outer scopes |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately after both are ready
    return x + y
}
```

### 2. spawn Block Semantics

`spawn { ... }` is the **sole parallelism primitive** in YaoXiang.

#### 2.1 Core Rules

- **Direct sub-expressions** of a spawn block create parallel tasks
- Expressions inside nested `{}` do not count as independent tasks
- spawn blocks follow standard return rules: must use `return` for explicit return value, returns `Void` without `return`
- The entire spawn block blocks synchronously, waiting for all tasks to complete before returning
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks executing in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // direct sub-expression → parallel task 1
    t2 = fetch("url2")   // direct sub-expression → parallel task 2
    return (t1, t2)      // explicit tuple return
}

// Nested {} are not direct sub-expressions
result = spawn {
    x = {               // this entire block is one direct sub-expression → one task
        inner_work()    // not a direct sub-expression of spawn, won't become an independent task
    },
    process(x)          // direct sub-expression → parallel task
    return process(x)
}
```

#### 2.2 Scope Isolation

spawn blocks create independent scopes; internal variables do not affect the outside:

```yaoxiang
x = 10
result = spawn {
    x = 20              // this is the local x inside the spawn block
    compute(x)
}
// x is still 10
```

#### 2.3 Ownership Rules

Once a variable enters a spawn block, it cannot be used outside (Move semantics):

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
shared = ref data       // compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // shared reference
    process_b(shared)   // shared reference
}
```

#### 2.4 Error Handling

Error propagation inside spawn blocks follows these rules:

1. Wait for all tasks to complete (even if some tasks have already failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // may fail
    fetch("url2")?      // may fail
}
// If any task fails, the entire spawn block propagates the first error
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
    read_file("data.txt"),      // executes first
    write_file("data.txt", x)   // waits for read to complete
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

### 3. Breaking Away from the Old Design

| Old Design (RFC-001) | New Design (RFC-024) |
|---------------------|---------------------|
| Whole-program automatic DAG analysis | Analysis only within spawn blocks |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven |
| `Send`/`Sync` trait | Not needed, ownership + ref handles automatically |
| `future`/non-blocking handles | Synchronous blocking, no callbacks |
| `Mutex`/`RwLock`/`Atomic` | `ref` automatically chooses Rc/Arc |
| L1/L2/L3 triple mental model | Regular code is sequential, spawn blocks are parallel |
| Function coloring (async/await) | No function coloring |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

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
    print(message)  // no return, returns Void
}
```

### 5. User Mental Model

> **Your regular code executes sequentially.**
>
> **When you want multiple things to happen together, put them inside a `spawn { ... }` block.**
>
> Each direct sub-expression in the block starts immediately (in parallel), and uses `return` to explicitly return results.
> The entire block waits for everything to finish, then gives you the final result.
>
> **No callbacks, no `await`, no weird annotations.**

```yaoxiang
// Regular code: sequential execution
a = compute_a()         // executes first
b = compute_b(a)        // depends on a, executes after a completes
c = compute_c(b)        // depends on b, executes after b completes

// When parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // parallel
    fetch("url2"),      // parallel
    fetch("url3")       // parallel
}
// Continue after all complete
process(x, y, z)
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallelism primitive (`spawn`), one rule (direct sub-expressions create tasks)
2. **Explicit**: Users clearly know where parallelism occurs and where execution is sequential, no implicit concurrency
3. **Safe**: Ownership rules extend naturally, no extra type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallel behavior, avoids unexpected concurrency issues
5. **Synchronous**: Callers block synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function coloring problems
7. **Efficient compilation**: DAG analysis only within spawn blocks, compile time is controllable

### Disadvantages

1. **Requires explicit spawn**: No automatic parallelism, users must manually mark parallel points
2. **DAG analysis within spawn blocks**: Compiler needs dependency analysis within spawn blocks
3. **Incompatible with old code**: Code using old RFC-001 patterns needs migration

---

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| Whole-program automatic DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior |
| async/await | Function coloring, steep learning curve, poor code readability |
| goroutine | No type safety, data races hard to detect |
| Actor model | Message passing is complex, debugging is difficult |
| CSP (Go channels) | No type safety, deadlocks hard to detect |

---

## Implementation Strategy

### Compile-time Analysis

1. **DAG construction**: Analyze dependency relationships within spawn blocks
2. **Topological sort**: Determine execution order within spawn blocks
3. **Parallelism identification**: Identify dependency-free subtrees within spawn blocks
4. **Escape analysis**: `ref` → Rc or Arc
5. **Resource conflict detection**: Detect potential conflicts of resource types

### Runtime Execution

Refer to the Runtime architecture from [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No spawn support, immediate execution
- **Standard Runtime**: Supports spawn blocks, concurrency within spawn blocks
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generics system) → Completed

---

## Design Decision Log

| Decision | Decision Made | Reason | Date |
|----------|---------------|--------|------|
| Parallelism primitive | `spawn {}` block | Simple, explicit, controllable | 2026-06-05 |
| Task creation | Direct sub-expressions | Clear, unambiguous | 2026-06-05 |
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
| **Accepted** | `docs/design/rfc/accepted/` | Official design document |