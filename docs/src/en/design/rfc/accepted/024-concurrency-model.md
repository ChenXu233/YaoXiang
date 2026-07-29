---
title: 'RFC-024: Spawn-based Concurrent Runtime Semantics'
status: 'Accepted (Revised)'
author: 'Chen Xu'
created: '2026-06-05'
updated:
  '2026-07-05 (RFC Sync Check: Implementation ~85%, core runtime and frontend analysis completed)'

issue: '#89'
---

# RFC-024: Spawn-based Concurrent Runtime Semantics

> **This document defines the runtime behavior semantics of `spawn`**. For syntax orthogonality,
> AST/IR refactoring, and type system extensions, see [RFC-032](./032-spawn-unified-expression.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **References**:
>
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-032: Spawn Unified Expression Decorator — AST/IR Refactoring](./032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavior semantics** of `spawn` in the YaoXiang programming
language: `spawn <expr>` is the sole parallel primitive, can decorate any expression, and the caller
blocks synchronously. The shape of the expression determines task decomposition granularity, and the
runtime schedules according to the GMP model — tasks without dependencies are thrown into the work
queue, workers race to run them.

**Core Design — One Primitive, One Set of Rules**:

```
spawn <expr>               ← Sole parallel primitive
Task decomposition determined by expression shape    ← Sole rule
Synchronous blocking waiting for results             ← Sole behavior
```

**Eliminated Complexity**:

- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The ordinary code you write executes sequentially. When you want multiple
> things to happen together, put them inside `spawn <expr>`. No callbacks, no `await`, no strange
> annotations.

## Design Sources

| Document                                                                 | Relationship                                        |
| ------------------------------------------------------------------------ | --------------------------------------------------- |
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document                         |
| [RFC-008](./008-runtime-concurrency-model.md)                            | Runtime architecture, orthogonal to this            |
| [RFC-009](./009-ownership-model.md)                                      | Ownership model, unchanged                          |
| [RFC-010](./010-unified-type-syntax.md)                                  | Unified type syntax                                 |
| [RFC-032](./032-spawn-unified-expression.md)                             | AST/IR refactoring, jointly defines spawn with this |

## Motivation

### Why is this design needed?

Current mainstream languages have significant flaws in their concurrency models:

| Language   | Concurrency Model   | Problems                                                 |
| ---------- | ------------------- | -------------------------------------------------------- |
| Rust       | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go         | goroutine           | No type safety, data races hard to detect                |
| Python     | asyncio             | GIL limitations, function coloring                       |
| JavaScript | Promise/async       | Callback hell, function coloring                         |

### Problems with the Old Design (RFC-001)

The three-tier concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem                  | Description                                                 |
| ------------------------ | ----------------------------------------------------------- |
| Complex mental model     | L1/L2/L3 three-layer abstraction increases learning burden  |
| Redundant annotations    | `@block`/`@eager`/`@auto` annotations clutter code          |
| High analysis complexity | Whole-program DAG analysis has large compile-time overhead  |
| Complex type constraints | `Send`/`Sync` trait increases cognitive load                |
| Uncontrollable           | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), can decorate any expression
2. **Explicit**: Users know exactly where parallelism and sequencing occur
3. **Safe**: Ownership rules extend naturally, no extra type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks and `await`

---

## Proposal

### 1. The Essence of {} Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property          | Description                                                                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven | When executing, the block checks whether all internal variables are ready; if complete, it executes immediately, otherwise blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "deferred"                                                                          |
| Return value      | Use `return` to explicitly return a value; without `return`, defaults to `Void`                                                             |
| Uniform syntax    | Consistent semantics whether appearing in function body, variable initialization, or after `spawn`                                          |
| Scope isolation   | Variables are strictly limited to inside `{}`, not leaking to outer scope                                                                   |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y, executes immediately after both are ready
    return x + y
}
```

### 2. Spawn Expression Semantics

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. It can decorate any expression, and
the shape of the expression determines task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape                | Task Decomposition                                                      | Synchronous Semantics                |
| ------------------------------- | ----------------------------------------------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | Direct sub-expressions → N independent tasks                            | Wait for all tasks to complete       |
| `spawn for x in items { body }` | Each iteration → 1 task                                                 | Wait for all iterations to complete  |
| `spawn while cond { body }`     | Each iteration → 1 task (iteration-driven by condition)                 | Wait for condition to be false       |
| `spawn if c { a } else { b }`   | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)`                 | The call itself → 1 task                                                | Wait for call to complete            |
| `spawn expr` (any expression)   | The expression itself → 1 task                                          | Wait for expression to complete      |

> **Design Motivation**: Why can spawn decorate any expression? See
> [RFC-032 §Core Design](./032-spawn-unified-expression.md).
>
> **Control Flow Orthogonality**: The semantic difference between `spawn <expr>` (spawn first) and
> `<expr> spawn { body }` (spawn after), see
> [RFC-032 §Control Flow Orthogonality](./032-spawn-unified-expression.md) (core definition). The
> runtime behavior of all reversed combinations (`for ... spawn { }` / `while ... spawn { }` /
> `if ... spawn { }`) — error propagation, resource types, nested rules — inherits the rules from
> §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // Direct sub-expression → parallel task 1
    t2 = fetch("url2")   // Direct sub-expression → parallel task 2
    return (t1, t2)      // Explicitly return tuple
}

// spawn for: each iteration in parallel
results = spawn for item in items {
    process(item)        // Each iteration → independent task
}

// spawn while: each iteration in parallel
spawn while has_next() {
    step()               // Each iteration → independent task
}

// spawn if: selected branch as a whole as task
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 Scope Isolation

Spawn expressions create independent scopes; internal variables do not affect the outside:

```yaoxiang
x = 10
result = spawn {
    x = 20              // This is local x inside the spawn expression
    compute(x)
}
// x is still 10

result = spawn for item in items {
    item = item + 1     // Iteration-local item, independent copy per iteration
    process(item)
}
// Outer item is unaffected
```

**Iteration variables** (the `x` in for) have independent copies per iteration, automatically
destroyed after iteration ends.

#### 2.3 Ownership Rules

After a variable enters a spawn expression, it cannot be used externally (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into spawn expression
}
// data is unavailable here (already moved)
```

If sharing among multiple tasks is needed, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // Compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Cross-iteration sharing**: Use `ref` to capture to the outer scope, sharing the same reference
across iterations.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (block)

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

- Wait for all iterations to complete, then return the first error
- Remaining iterations **continue executing** after a failed iteration (not cancelled)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // Any iteration fails → wait for all to complete → propagate first error
}
```

##### `spawn while cond { body? }`

Inherits the error semantics of while itself:

- step uses `?` to propagate error → entire spawn while fails, no more iterations
- step does not propagate error (error swallowed) → proceed to next iteration

```yaoxiang
spawn while has_next() {
    item = next()       // When errors are not propagated, failures still proceed to next iteration
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c **evaluated sequentially**
- Error evaluating c → overall error
- Error in selected branch → overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks resource type usage to ensure concurrency safety:

| Resource Type | Description         | Compiler Behavior                                   |
| ------------- | ------------------- | --------------------------------------------------- |
| `FilePath`    | File system path    | Same-path operations automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations automatically serialized        |
| `DBUrl`       | Database connection | Same-connection operations automatically serialized |
| `Console`     | Standard output     | All Console operations automatically serialized     |

##### Inside `spawn { ... }` block

```yaoxiang
// Operations on the same file automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

##### `spawn for ... { ... }` cross-iteration same resource

When all iterations operate on the same resource type, the compiler **automatically downgrades to
serial** (spawn degrades to sequential for, no error reported):

```yaoxiang
// All iterations write to the same file path → automatically downgraded to serial
results = spawn for item in items {
    write_file("data.txt", item)
}
// Compiler automatically serializes all iterations
```

> **Design Rationale**: The spawn keyword still expresses parallel intent; when resource conflicts
> occur, the compiler automatically degrades, which aligns better with the principle of least
> surprise than outright rejection.

##### `spawn while ... { ... }` capturing `&mut`

**Compile-time error**: `spawn while` does not allow capturing external variables of `&mut` type:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // Compile-time error
    item = iter.next()              // iter is &mut, cross-iteration shared mutability = data race
}
```

> **Not re-introducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" commitment. Users
> are required to use `ref` or non-spawn alternatives.

##### `spawn if c { ... } else { ... }` both branches same resource

**Legal, no warning**: The if conditions are mutually exclusive, at most one branch executes, no
concurrency conflict:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // Branch 1: read cache
} else {
    fetch(key)                      // Branch 2: read URL
}
```

#### 2.6 Nested Spawn

Spawn expressions can be nested; inner layers create **independent concurrency domains**:

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**Nested semantics**:

- Inner spawn is an independent concurrency domain (independent task queue, independent error
  propagation)
- Inner errors propagate independently to outer (outer task receives error when waiting for inner to
  complete)
- Inner resource type rules are tracked independently (not jointly checked with outer)

```yaoxiang
// spawn for nested spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Breaking from the Old Design

| Old Design (RFC-001)                  | New Design (RFC-024 + RFC-032)                              |
| ------------------------------------- | ----------------------------------------------------------- |
| Whole-program automatic DAG analysis  | Analysis only within spawn expressions                      |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven                           |
| `Send`/`Sync` trait                   | Not needed, ownership + ref handles automatically           |
| `future`/non-blocking handles         | Synchronous blocking, no callbacks                          |
| `Mutex`/`RwLock`/`Atomic`             | `ref` automatically chooses Rc/Arc                          |
| L1/L2/L3 three-tier mental model      | Ordinary code is sequential, spawn expressions are parallel |
| Function coloring (async/await)       | No function coloring                                        |
| `spawn` only decorates `{}` blocks    | `spawn` decorates any expression (see RFC-032)              |

### 4. Return Rules

YaoXiang's return rules are unified and clear:

| Syntax                    | Return Value                                | Description                 |
| ------------------------- | ------------------------------------------- | --------------------------- |
| `= expr` (no braces)      | Directly returns `expr`                     | Expression is value         |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Block needs explicit return |

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
> **When you want multiple things to happen together, put them inside `spawn <expr>`.**
>
> The shape of the expression determines how tasks are decomposed: each direct sub-expression in a
> block runs in parallel; each iteration of for runs in parallel; the selected branch of if runs as
> one task.
>
> **The entire spawn expression blocks synchronously, waiting for all tasks to complete.**
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

// Data parallelism: spawn for
results = spawn for item in items {
    process(item)
}
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallel primitive (`spawn`), can decorate any expression
2. **Explicit**: Users know exactly where parallelism and sequencing occur, no implicit concurrency
3. **Safe**: Ownership rules extend naturally, no extra type constraints like `Send`/`Sync` needed
4. **Controllable**: No automatic parallelism, avoids unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function coloring problems
7. **Compile-efficient**: DAG analysis only within spawn expressions, compile time is controllable
8. **Orthogonal**: Spawn naturally combines with any control flow structure (see RFC-032)

### Disadvantages

1. **Explicit spawn required**: Cannot automatically parallelize, users need to manually mark
   parallel points
2. **DAG analysis within spawn expressions**: Compiler needs to perform dependency analysis within
   spawn expressions
3. **Incompatible with old code**: Code using old RFC-001 patterns needs migration

---

## Alternative Approaches

| Approach                              | Why Not Chosen                                                         |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Whole-program automatic DAG (RFC-001) | High complexity, long compile time, unpredictable behavior             |
| async/await                           | Function coloring, steep learning curve, poor code readability         |
| goroutine                             | No type safety, data races hard to detect                              |
| Actor model                           | Complex message passing, difficult to debug                            |
| CSP (Go channel)                      | No type safety, deadlocks hard to detect                               |
| `spawn` only decorates `{}` block     | Breaks orthogonality, `spawn for` becomes a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the shape of the
   expression after spawn (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependencies within the spawn expression
3. **Topological sorting**: Determine execution order within the spawn expression
4. **Parallelism identification**: Identify dependency-free subtrees within the spawn expression
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts for resource types

### Module Organization

Spawn-related code is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # Spawn module entry
├── placement.rs     # Spawn position legality checks
└── analysis.rs      # Task identification, dependency analysis, resource conflict detection
```

> **Migration Note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs`
> will be migrated to `frontend/core/spawn/placement.rs`. The spawn_placement module declaration in
> `typecheck/passes/` needs to be removed simultaneously.

### Runtime Execution

Refer to the Runtime architecture from [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No spawn support, immediate execution
- **Standard Runtime**: Supports spawn expressions
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generics system) → Completed
- RFC-032 (AST/IR refactoring) → Jointly defines spawn with this document

---

## Design Decision Record

| Decision                       | Decision                                          | Reason                                                     | Date       |
| ------------------------------ | ------------------------------------------------- | ---------------------------------------------------------- | ---------- |
| Parallel primitive             | `spawn <expr>`                                    | Simple, explicit, controllable                             | 2026-06-05 |
| Spawn decoration scope         | Any expression                                    | Syntax orthogonality, eliminate `spawn for` specialization | 2026-07-04 |
| Task decomposition             | Determined by expression shape                    | Strong expressiveness, unified rules                       | 2026-07-04 |
| Execution model                | Synchronous blocking                              | Easy to understand, easy to debug                          | 2026-06-05 |
| DAG analysis scope             | Only within spawn expressions                     | Compile-efficient, controllable behavior                   | 2026-06-05 |
| Sharing mechanism              | `ref` automatically chooses Rc/Arc                | Simplifies user decisions                                  | 2026-06-05 |
| Annotations                    | None                                              | Reduces code noise                                         | 2026-06-05 |
| Send/Sync                      | Removed                                           | Ownership + ref is sufficient                              | 2026-06-05 |
| Mutex/RwLock                   | Removed                                           | ref handles automatically                                  | 2026-06-05 |
| future/handles                 | Removed                                           | Synchronous blocking is simpler                            | 2026-06-05 |
| Function coloring              | None                                              | Avoids async/await problems                                | 2026-06-05 |
| Resource types                 | Built-in + user-defined                           | Automatic serialization                                    | 2026-06-05 |
| `spawn {}` errors              | Wait for all to complete, propagate first error   | Deterministic behavior                                     | 2026-06-05 |
| `spawn for` errors             | Wait for all to complete, propagate first error   | Consistent with `spawn {}`                                 | 2026-07-04 |
| `spawn while` errors           | Inherits while error semantics                    | Standard while behavior                                    | 2026-07-04 |
| `spawn if` condition errors    | c evaluated sequentially, failure → overall error | Intuitive                                                  | 2026-07-04 |
| `spawn for` same resource      | Automatically downgrade to serial                 | Safe degradation, not harsh rejection                      | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error                                | Avoid data races, don't introduce Sync                     | 2026-07-04 |
| `spawn if` same resource       | Legal, no warning                                 | Mutually exclusive branches don't conflict                 | 2026-07-04 |
| Nested spawn                   | Inner layer is independent concurrency domain     | Independent task queue, errors, resources                  | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](/reference/language-spec/concurrency.md)
- [RFC-001 Spawn Model (Deprecated)](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generics System](./011-generic-type-system.md)
- [RFC-032 Spawn Unified Expression Decorator — AST/IR Refactoring](./032-spawn-unified-expression.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status                 | Location                    | Description                                            |
| ---------------------- | --------------------------- | ------------------------------------------------------ |
| **Accepted (Revised)** | `docs/design/rfc/accepted/` | Jointly defines spawn with RFC-032 (runtime semantics) |
