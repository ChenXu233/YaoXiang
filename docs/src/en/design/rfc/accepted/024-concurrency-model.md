---
title: 'RFC-024: Spawn-based Concurrency Runtime Semantics'
status: 'Accepted (Revised)'
author: 'Chenxu'
created: '2026-06-05'
updated: '2026-07-05 (RFC Sync Check: Implementation progress ~85%, core runtime and frontend analysis completed)'

issue: '#89'
---

# RFC-024: Spawn-based Concurrency Runtime Semantics

> **This document defines the runtime behavior semantics of `spawn`**. For syntax orthogonality, AST/IR restructuring,
> and type system extensions, see [RFC-032](../review/032-spawn-unified-expression.md).
>
> The two RFCs together define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **References**:
>
> - [Concurrency Model Specification](../../../../reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model Decoupled from Scheduler](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-032: Spawn Unified Expression Modifier — AST/IR Restructuring](../review/032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavior semantics** of `spawn` in the YaoXiang programming language: `spawn <expr>`
is the sole parallel primitive, can modify any expression, and the caller blocks synchronously. The shape of the expression
determines the granularity of task decomposition, and the runtime schedules according to the GMP model — tasks without
dependencies are thrown into the work queue, workers compete to run them.

**Core Design — One Primitive, One Rule Set**:

```
spawn <expr>               ← Sole parallel primitive
Task decomposition determined by expression shape    ← Sole rule
Synchronous blocking wait for results            ← Sole behavior
```

**Complexity Eliminated**:

- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` traits
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The ordinary code you write executes sequentially. When you want multiple things to happen
> together, put them inside `spawn <expr>`. No callbacks, no `await`, no strange annotations.

## Design Origins

| Document                                                                   | Relationship                      |
| --------------------------------------------------------------------------- | --------------------------------- |
| [RFC-001](../../../../design/rfc/deprecated/001-concurrent-model-error-handling.md)   | Superseded by this document       |
| [RFC-008](./008-runtime-concurrency-model.md)                              | Runtime architecture, orthogonal to this |
| [RFC-009](./009-ownership-model.md)                                        | Ownership model, unchanged        |
| [RFC-010](./010-unified-type-syntax.md)                                    | Unified type syntax               |
| [RFC-032](../review/032-spawn-unified-expression.md)                      | AST/IR restructuring, jointly defines spawn with this |

## Motivation

### Why Is This Design Needed?

Current mainstream language concurrency models have obvious flaws:

| Language    | Concurrency Model          | Problems                                   |
| ----------- | -------------------------- | ------------------------------------------ |
| Rust        | async/await + tokio        | Async contagion, function coloring, steep learning curve |
| Go          | goroutine                  | No type safety, data races hard to detect  |
| Python      | asyncio                    | GIL restrictions, function coloring        |
| JavaScript  | Promise/async              | Callback hell, function coloring           |

### Problems with the old design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 had the following problems:

| Problem          | Description                                         |
| ---------------- | --------------------------------------------------- |
| Complex mental model | L1/L2/L3 three-layer abstraction increased learning burden |
| Redundant annotations | `@block`/`@eager`/`@auto` annotations made code noisy |
| High analysis complexity | Whole-program DAG analysis had large compile-time overhead |
| Complex type constraints | `Send`/`Sync` trait increased cognitive burden     |
| Uncontrollable   | Automatic concurrency behavior hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism is and where it's sequential
3. **Safe**: Ownership rules extend naturally, no extra type constraints needed
4. **Controllable**: No implicit concurrency, no accidental parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks and `await`

---

## Proposal

### 1. The Essence of {} Blocks: Dependency-driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property       | Description                                                               |
| -------------- | ------------------------------------------------------------------------- |
| Dependency-driven | Block checks at execution whether all internal variables are ready; if ready, executes immediately; otherwise blocks waiting |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "deferred"      |
| Return value   | Use `return` for explicit return; default `Void` if no `return`         |
| Uniform syntax | Consistent semantics whether appearing in function body, variable initialization, or after `spawn` |
| Scope isolation | Variables strictly limited inside `{}`, not leaking to outer scope      |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately when both are ready
    return x + y
}
```

### 2. spawn Expression Semantics

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. Can modify any expression, and the expression shape
determines task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape                    | Task Decomposition                             | Synchronous Semantics         |
| ----------------------------------- | ---------------------------------------------- | ----------------------------- |
| `spawn { a, b, c }`                 | Direct sub-expressions → N independent tasks   | Wait for all tasks to complete |
| `spawn for x in items { body }`     | Each iteration → 1 task                        | Wait for all iterations to complete |
| `spawn while cond { body }`         | Each iteration → 1 task (iteration driven by condition) | Wait for condition to be false |
| `spawn if c { a } else { b }`       | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)`                     | The call itself → 1 task                       | Wait for call to complete     |
| `spawn expr` (any expression)       | The expression itself → 1 task                | Wait for expression to complete |

> **Design Motivation**: Why can spawn modify any expression? See
> [RFC-032 §Core Design](../review/032-spawn-unified-expression.md).
>
> **Control Flow Orthogonality**: Semantic differences between `spawn <expr>` (spawn first) and
> `<expr> spawn { body }` (spawn last), see
> [RFC-032 §Control Flow Orthogonality](../review/032-spawn-unified-expression.md) (core definition). The runtime behavior
> of all reversed combinations (`for ... spawn { }` / `while ... spawn { }` / `if ... spawn { }`) — error propagation,
> resource types, nesting rules — inherit the rules from §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions parallel
(a, b) = spawn {
    t1 = fetch("url1")   // Direct sub-expression → parallel task 1
    t2 = fetch("url2")   // Direct sub-expression → parallel task 2
    return (t1, t2)      // Explicit tuple return
}

// spawn for: each iteration parallel
results = spawn for item in items {
    process(item)        // Each iteration → independent task
}

// spawn while: each iteration parallel
spawn while has_next() {
    step()               // Each round → independent task
}

// spawn if: selected branch as a whole as task
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 Scope Isolation

Spawn expressions create independent scopes; internal variables don't affect external ones:

```yaoxiang
x = 10
result = spawn {
    x = 20              // This is a local x inside the spawn expression
    compute(x)
}
// x is still 10

result = spawn for item in items {
    item = item + 1     // Iteration-local item, independent copy per iteration
    process(item)
}
// Outer item unaffected
```

**Iteration variables** (the `x` in for) are independent copies per iteration, automatically destroyed after iteration ends.

#### 2.3 Ownership Rules

After a variable enters a spawn expression, it cannot be used externally anymore (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into the spawn expression
}
// data is unavailable here (already moved)
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

**Cross-iteration sharing**: Use `ref` to capture to the outer scope, sharing the same reference across iterations.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (Block)

1. Wait for all tasks to complete (even if some have failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If any task fails, the entire spawn expression propagates the first error
```

##### `spawn for x in items { body? }`

- Wait for all iterations to complete before returning the first error
- Remaining iterations **continue executing** after a failed iteration (no cancellation)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // Any iteration fails → wait for all → propagate first error
}
```

##### `spawn while cond { body? }`

Inherits the error semantics of `while` itself:

- step uses `?` to propagate error → entire spawn while fails, no more iterations
- step doesn't propagate error (error swallowed) → proceed to next iteration

```yaoxiang
spawn while has_next() {
    item = next()       // When not propagating errors, failure proceeds to next round
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c **evaluated sequentially**
- Error during c evaluation → overall error
- Error in selected branch → overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks the usage of resource types to ensure concurrency safety:

| Resource Type   | Description         | Compiler Behavior                |
| --------------- | ------------------- | -------------------------------- |
| `FilePath`      | File system path    | Operations on same path auto-serialized |
| `HttpUrl`       | HTTP endpoint      | Operations on same URL auto-serialized |
| `DBUrl`         | Database connection | Operations on same connection auto-serialized |
| `Console`       | Standard output    | All Console operations auto-serialized |

##### Inside `spawn { ... }` Block

```yaoxiang
// Operations on the same file auto-serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

##### `spawn for ... { ... }` Same Resource Across Iterations

When all iterations operate on the same resource type, the compiler **automatically downgrades to serial**
(spawn degrades to sequential for, no error reported):

```yaoxiang
// All iterations write to the same file path → auto downgraded to serial
results = spawn for item in items {
    write_file("data.txt", item)
}
// Compiler auto-serializes all iterations
```

> **Design Rationale**: The spawn keyword still expresses parallel intent; when resource conflicts occur,
> the compiler auto-downgrades, which aligns better with the principle of least surprise than outright rejection.

##### `spawn while ... { ... }` Capturing `&mut`

**Compile-time error**: `spawn while` does not allow capturing external variables of `&mut` type:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // Compile-time error
    item = iter.next()              // iter is &mut, cross-iteration mutable sharing = data race
}
```

> **Not reintroducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" promise. Users should use `ref`
> or non-spawn写法.

##### `spawn if c { ... } else { ... }` Same Resource in Both Branches

**Legal with no warnings**: if conditions are mutually exclusive, at most one branch executes, no concurrency conflict:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // Branch 1: read cache
} else {
    fetch(key)                      // Branch 2: read URL
}
```

#### 2.6 Nested spawn

Spawn expressions can be nested, inner layers create **independent concurrency domains**:

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**Nesting semantics**:

- Inner spawn is an independent concurrency domain (independent task queue, independent error propagation)
- Inner errors propagate independently to outer (outer task receives error when waiting for inner to complete)
- Inner resource type rules tracked independently (not jointly checked with outer)

```yaoxiang
// spawn for nested spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Breaking with the Old Design

| Old Design (RFC-001)              | New Design (RFC-024 + RFC-032)            |
| --------------------------------- | ----------------------------------------- |
| Whole-program automatic DAG analysis | Analysis only within spawn expressions    |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven         |
| `Send`/`Sync` trait               | Not needed, ownership + ref handles automatically |
| `future`/non-blocking handles     | Synchronous blocking, no callbacks        |
| `Mutex`/`RwLock`/`Atomic`         | `ref` auto-selects Rc/Arc                 |
| L1/L2/L3 three-layer mental model | Ordinary code sequential, spawn expressions parallel |
| Function coloring (async/await)   | No function coloring                      |
| `spawn` only modifies `{}` blocks | `spawn` modifies any expression (see RFC-032) |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

| Syntax                      | Return Value                           | Description           |
| --------------------------- | -------------------------------------- | --------------------- |
| `= expr` (no braces)        | Directly return `expr`                 | Expression is value  |
| `= { ... }` (with braces)  | Must use `return`, otherwise `Void`    | Block needs explicit return |

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

> **The normal code you write executes sequentially.**
>
> **When you want to do multiple things simultaneously, put them inside `spawn <expr>`.**
>
> The shape of the expression determines how tasks are decomposed: each direct sub-expression in a block runs
> in parallel; each iteration of for runs in parallel; the selected branch of if runs as one task.
>
> **The entire spawn expression blocks synchronously, waiting for all tasks to complete.**
>
> **No callbacks, no `await`, no weird annotations.**

```yaoxiang
// Normal code: sequential execution
a = compute_a()         // Executes first
b = compute_b(a)        // Depends on a, executes after a completes
c = compute_c(b)        // Depends on b, executes after b completes

// When parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // Parallel
    fetch("url2"),      // Parallel
    fetch("url3")       // Parallel
}
// Wait for all to complete before continuing
process(x, y, z)

// Data parallelism: spawn for
results = spawn for item in items {
    process(item)
}
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism is and where it's sequential, no implicit concurrency
3. **Safe**: Ownership rules extend naturally, no extra type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallel behavior, avoids accidental concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function coloring problems
7. **Compile-efficient**: DAG analysis limited within spawn expressions, compile time controllable
8. **Orthogonal**: spawn naturally combines with any control flow structure (see RFC-032)

### Disadvantages

1. **Explicit spawn required**: Cannot auto-parallelize, users need to manually mark parallel points
2. **DAG analysis within spawn expressions**: Compiler needs dependency analysis within spawn expressions
3. **Incompatible with old code**: Code using old RFC-001 patterns needs migration

---

## Alternatives

| Approach                      | Why Not Chosen                                   |
| ----------------------------- | ------------------------------------------------ |
| Whole-program auto DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior |
| async/await               | Function coloring, steep learning curve, poor code readability |
| goroutine                 | No type safety, data races hard to detect        |
| Actor model               | Complex message passing, hard to debug           |
| CSP (Go channel)         | No type safety, deadlocks hard to detect         |
| `spawn` only modifies `{}` blocks | Breaks orthogonality, `spawn for` becomes a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the expression shape after spawn (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependencies within the spawn expression
3. **Topological sort**: Determine execution order within the spawn expression
4. **Parallelism identification**: Identify dependency-free subtrees within the spawn expression
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts on resource types

### Module Organization

Spawn-related code unified under `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # Spawn module entry point
├── placement.rs     # Spawn occurrence position legality checks
└── analysis.rs      # Task recognition, dependency analysis, resource conflict detection
```

> **Migration Note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs` will be migrated to
> `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration under `typecheck/passes/`
> needs to be removed synchronously.

### Runtime Execution

Reference the Runtime architecture from [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No spawn support, immediate execution
- **Standard Runtime**: Supports spawn expressions
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generic system) → Completed
- RFC-032 (AST/IR restructuring) → Jointly defines spawn with this document

---

## Design Decision Records

| Decision                      | Decision                        | Rationale                              | Date       |
| ----------------------------- | ------------------------------- | -------------------------------------- | ---------- |
| Parallel primitive            | `spawn <expr>`                  | Simple, explicit, controllable         | 2026-06-05 |
| spawn modification scope      | Any expression                  | Syntax orthogonality, eliminate `spawn for` specialization | 2026-07-04 |
| Task decomposition            | Determined by expression shape  | Expressive power, unified rules        | 2026-07-04 |
| Execution model               | Synchronous blocking            | Easy to understand, debuggable         | 2026-06-05 |
| DAG analysis scope            | Only within spawn expressions   | Compile-efficient, controllable behavior | 2026-06-05 |
| Sharing mechanism            | `ref` auto-selects Rc/Arc       | Simplifies user decisions              | 2026-06-05 |
| Annotations                   | None                            | Reduce code noise                      | 2026-06-05 |
| Send/Sync                     | Removed                         | Ownership + ref sufficient             | 2026-06-05 |
| Mutex/RwLock                  | Removed                         | ref handles automatically               | 2026-06-05 |
| future/handles                | Removed                         | Synchronous blocking is simpler        | 2026-06-05 |
| Function coloring             | None                            | Avoid async/await problems             | 2026-06-05 |
| Resource types                | Built-in + user-defined         | Auto-serialization                     | 2026-06-05 |
| `spawn {}` errors             | Wait for all, propagate first error | Deterministic behavior             | 2026-06-05 |
| `spawn for` errors           | Wait for all, propagate first error | Consistent with `spawn {}`           | 2026-07-04 |
| `spawn while` errors         | Inherit while error semantics    | while standard behavior                | 2026-07-04 |
| `spawn if` condition errors  | c evaluated sequentially, failure → overall error | Intuitive                    | 2026-07-04 |
| `spawn for` same resource    | Auto downgrade to serial         | Safe downgrade, not brutal rejection   | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error              | Avoid data races, don't introduce Sync | 2026-07-04 |
| `spawn if` same resource      | Legal, no warnings              | Mutually exclusive branches don't conflict | 2026-07-04 |
| Nested spawn                  | Inner layer independent concurrency domain | Independent task queues, errors, resources | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](../../../../reference/language-spec/concurrency.md)
- [RFC-001 Spawn Model (Deprecated)](../../../../design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generic System](./011-generic-type-system.md)
- [RFC-032 Spawn Unified Expression Modifier — AST/IR Restructuring](../review/032-spawn-unified-expression.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status                | Location                       | Description                                    |
| --------------------- | ------------------------------ | ---------------------------------------------- |
| **Accepted (Revised)** | `docs/design/rfc/accepted/`    | Jointly defines spawn with RFC-032 (runtime semantics) |