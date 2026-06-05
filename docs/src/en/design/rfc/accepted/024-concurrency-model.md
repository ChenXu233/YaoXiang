```markdown
---
title: "RFC-024: Concurrency Model Based on spawn Blocks"
status: "Accepted"
author: "Chen Xu"
created: "2026-06-05"
updated: "2026-05"
---

# RFC-024: Concurrency Model Based on spawn Blocks

> **Reference**:
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)

## Abstract

This document defines the new concurrency model for the YaoXiang programming language: using `spawn {}` blocks as the sole parallel primitive, dependency-driven execution, and synchronous blocking on the caller side. It replaces the old concurrency approach based on `@block`/`@eager`/`@auto` annotations, `Send`/`Sync` trait, and whole-program DAG analysis.

**Core Design — One Primitive, One Rule**:

```
spawn { ... }        ← Sole parallel primitive
Direct child expressions create tasks  ← Sole rule
Synchronous blocking for results      ← Sole behavior
```

**Eliminated Complexity**:
- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The regular code you write executes sequentially. When you want multiple things to happen together, put them inside a `spawn { ... }` block. No callbacks, no `await`, no strange annotations.

## Design Sources

| Document | Relationship |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document |
| [RFC-008](./008-runtime-concurrency-model.md) | Runtime architecture, orthogonal to this document |
| [RFC-009](./009-ownership-model.md) | Ownership model, unchanged |
| [RFC-010](./010-unified-type-syntax.md) | Unified type syntax, return rules updated |
| [Concurrency Model Specification](/reference/language-spec/concurrency.md) | Formal specification reference for this document |

## Motivation

### Why This Design Is Needed

Current mainstream languages have obvious concurrency model flaws:

| Language | Concurrency Model | Problems |
|------|----------|------|
| Rust | async/await + tokio | Async infection, function coloring, steep learning curve |
| Go | goroutine | No type safety, data races hard to detect |
| Python | asyncio | GIL limitations, function coloring |
| JavaScript | Promise/async | Callback hell, function coloring |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem | Description |
|------|------|
| Complex mental model | L1/L2/L3 three-layer abstraction increases learning burden |
| Redundant annotations | `@block`/`@eager`/`@auto` annotations make code noisy |
| High analysis complexity | Whole-program DAG analysis has significant compile-time overhead |
| Complex type constraints | `Send`/`Sync` trait increases cognitive burden |
| Uncontrollable | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), one rule (direct child expressions create tasks)
2. **Explicit**: Users clearly know where parallelism exists and where sequential execution occurs
3. **Safe**: Ownership rules extend naturally, no extra type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Callers block synchronously, no callbacks and `await`

---

## Proposal

### 1. The Nature of {} Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Attribute | Description |
|------|------|
| Dependency-driven | When executing, the block checks if all internal variables are ready; if so, it executes immediately; otherwise, it blocks and waits |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "deferred" |
| Return value | Explicitly returned using `return`; defaults to `Void` when no `return` |
| Uniform syntax | Consistent semantics whether appearing in function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly limited to inside `{}`, not leaking to outer scopes |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately once both are ready
    return x + y
}
```

### 2. spawn Block Semantics

`spawn { ... }` is the **sole parallel primitive** in YaoXiang.

#### 2.1 Core Rules

- **Direct child expressions** of spawn blocks create parallel tasks
- Expressions inside nested `{}` are not considered independent tasks
- spawn blocks follow standard return rules: must use `return` for explicit return values, otherwise returns `Void`
- The entire spawn block blocks synchronously and waits for all tasks to complete before returning
- No callbacks, `await`, or annotations

```yaoxiang
// Two tasks execute in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // Direct child expression → parallel task 1
    t2 = fetch("url2")   // Direct child expression → parallel task 2
    return (t1, t2)      // Explicitly return tuple
}

// Nested {} are not direct child expressions
result = spawn {
    x = {               // This entire block is one direct child expression → one task
        inner_work()    // Not a direct child expression of spawn, won't become an independent task
    },
    process(x)          // Direct child expression → parallel task
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

Once a variable enters a spawn block, it cannot be used outside (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into the spawn block
}
// data is not available here (already moved)
```

To share across multiple tasks, use `ref`:

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
// If any task fails, the spawn block propagates the first error
```

#### 2.5 Resource Types

The compiler tracks resource type usage to ensure concurrency safety:

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

### 3. Breaking from the Old Design

| Old Design (RFC-001) | New Design (RFC-024) |
|------------------|------------------|
| Whole-program automatic DAG analysis | Analysis only within spawn blocks |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven |
| `Send`/`Sync` trait | Unnecessary; ownership + ref handles it automatically |
| `future`/non-blocking handles | Synchronous blocking, no callbacks |
| `Mutex`/`RwLock`/`Atomic` | `ref` automatically chooses Rc/Arc |
| L1/L2/L3 three-layer mental model | Regular code is sequential, spawn blocks are parallel |
| Function coloring (async/await) | No function coloring |

### 4. Return Rules

YaoXiang's return rules are unified and clear:

| Syntax | Return Value | Description |
|------|--------|------|
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

> **The regular code you write executes sequentially.**
>
> **When you want multiple things to happen together, put them inside a `spawn { ... }` block.**
>
> Every direct child expression in the block starts immediately (in parallel), and you explicitly return results with `return`.
> The entire block waits for everything to finish, then gives you the final result.
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// Regular code: sequential execution
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

1. **Simple**: Only one parallel primitive (`spawn`), one rule (direct child expressions create tasks)
2. **Explicit**: Users clearly know where parallelism exists and where sequential execution occurs; no implicit concurrency
3. **Safe**: Ownership rules extend naturally; no extra type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallel behavior; avoids unexpected concurrency issues
5. **Synchronous**: Callers block synchronously; code is easy to understand and debug
6. **No function coloring**: No async/await function coloring problems
7. **Efficient compilation**: DAG analysis limited to spawn blocks; compile time is manageable

### Disadvantages

1. **Requires explicit spawn**: Cannot automatically parallelize; users must manually mark parallel points
2. **DAG analysis within spawn blocks**: Compiler needs to perform dependency analysis within spawn blocks
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs migration

---

## Alternative Approaches

| Approach | Why Not Chosen |
|------|--------------|
| Whole-program automatic DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior |
| async/await | Function coloring, steep learning curve, poor code readability |
| goroutine | No type safety, data races hard to detect |
| Actor model | Complex message passing, hard to debug |
| CSP (Go channels) | No type safety, deadlocks hard to detect |

---

## Implementation Strategy

### Compile-time Analysis

1. **DAG construction**: Analyze dependency relationships within spawn blocks
2. **Topological sorting**: Determine execution order within spawn blocks
3. **Parallelism identification**: Identify independent subtrees within spawn blocks
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
- RFC-011 (Generics system) → Completed

---

## Design Decision Log

| Decision | Decision | Reason | Date |
|------|------|------|------|
| Parallel primitive | `spawn {}` block | Simple, explicit, controllable | 2026-06-05 |
| Task creation | Direct child expressions | Clear, unambiguous | 2026-06-05 |
| Execution model | Synchronous blocking | Easy to understand, debug | 2026-06-05 |
| DAG analysis scope | Only within spawn blocks | Efficient compilation, controllable behavior | 2026-06-05 |
| Sharing mechanism | `ref` automatically selects Rc/Arc | Simplifies user decisions | 2026-06-05 |
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
- [RFC-011 Generics System](./011-generic-type-system.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Disposition

| Status | Location | Description |
|------|------|------|
| **Accepted** | `docs/design/rfc/accepted/` | Formal design document |
```