---
title: 'RFC-024: spawn-based Concurrency Runtime Semantics'
status: 'Accepted (Revised)'
author: 'Chenxu'
created: '2026-06-05'
updated:
  '2026-07-05 (RFC sync check: implementation progress ~85%, core runtime and frontend analysis
  completed)'

issue: '#89'
---

# RFC-024: spawn-based Concurrency Runtime Semantics

> **This document defines the runtime behavioral semantics of `spawn`**. Syntax orthogonality,
> AST/IR refactoring, and type system extensions are covered in
> [RFC-032](../review/032-spawn-unified-expression.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **References**:
>
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-032: spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavioral semantics** of `spawn` in the YaoXiang programming
language: `spawn <expr>` is the sole parallel primitive, can modify any expression, and the caller
blocks synchronously. The shape of the expression determines task decomposition granularity, and the
runtime schedules according to the GMP model — tasks without dependencies are thrown into the work
queue, and workers race to run them.

**Core Design — One Primitive, One Set of Rules**:

```
spawn <expr>               ← The sole parallel primitive
Task decomposition determined by expression shape    ← The sole rule
Synchronous blocking wait for results                ← The sole behavior
```

**Complexity Eliminated**:

- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` traits
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User Mental Model**: The normal code you write executes sequentially. When you want to do
> multiple things simultaneously, put them inside `spawn <expr>`. No callbacks, no `await`, no weird
> annotations.

## Design Origins

| Document                                                                 | Relationship                              |
| ------------------------------------------------------------------------ | ----------------------------------------- |
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document               |
| [RFC-008](./008-runtime-concurrency-model.md)                            | Runtime architecture, orthogonal to this  |
| [RFC-009](./009-ownership-model.md)                                      | Ownership model, unchanged                |
| [RFC-010](./010-unified-type-syntax.md)                                  | Unified type syntax                       |
| [RFC-032](../review/032-spawn-unified-expression.md)                     | AST/IR refactoring, jointly defines spawn |

## Motivation

### Why is this design needed?

Current mainstream language concurrency models have obvious flaws:

| Language   | Concurrency Model   | Problems                                                 |
| ---------- | ------------------- | -------------------------------------------------------- |
| Rust       | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go         | goroutine           | No type safety, data races hard to detect                |
| Python     | asyncio             | GIL limitation, function coloring                        |
| JavaScript | Promise/async       | Callback hell, function coloring                         |

### Problems with the old design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following issues:

| Problem                  | Description                                                   |
| ------------------------ | ------------------------------------------------------------- |
| Complex mental model     | L1/L2/L3 three-layer abstraction increases learning burden    |
| Annotation redundancy    | `@block`/`@eager`/`@auto` annotations make code noisy         |
| High analysis complexity | Whole-program DAG analysis incurs large compile-time overhead |
| Complex type constraints | `Send`/`Sync` traits increase cognitive load                  |
| Uncontrollable           | Automatic concurrent behavior is hard to predict and debug    |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallel and where sequential
3. **Safe**: Ownership rules extend naturally, no additional type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of `{}` Blocks: Dependency-driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Attribute         | Description                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven | The block checks whether all internal variables are ready during execution; runs immediately if all are ready, otherwise blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "deferred"                                                                        |
| Return value      | Use `return` to explicitly return a value; no `return` defaults to returning `Void`                                                       |
| Syntax unified    | Semantics are consistent whether appearing in function body, variable initialization, or after `spawn`                                    |
| Scope isolation   | Variables are strictly confined within `{}`, do not leak to outer scope                                                                   |

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

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. It can modify any expression, and the
shape of the expression determines task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape                | Task Decomposition                                                      | Synchronous Semantics                |
| ------------------------------- | ----------------------------------------------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | Direct sub-expressions → N independent tasks                            | Wait for all tasks to complete       |
| `spawn for x in items { body }` | Each iteration → 1 task                                                 | Wait for all iterations to complete  |
| `spawn while cond { body }`     | Each round of iteration → 1 task (condition-driven between iterations)  | Wait for condition to be false       |
| `spawn if c { a } else { b }`   | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)`                 | The call itself → 1 task                                                | Wait for call to complete            |
| `spawn expr` (any expression)   | The expression itself → 1 task                                          | Wait for expression to complete      |

> **Design motivation**: Why can spawn modify any expression? See
> [RFC-032 §Core Design](../review/032-spawn-unified-expression.md).
>
> **Control flow orthogonality**: The semantic difference between `spawn <expr>` (spawn first) and
> `<expr> spawn { body }` (spawn last) is detailed in
> [RFC-032 §Control Flow Orthogonality](../review/032-spawn-unified-expression.md) (core
> definition). The runtime behavior of all reversed combinations (`for ... spawn { }` /
> `while ... spawn { }` / `if ... spawn { }`) — error propagation, resource types, nesting rules —
> inherits the rules in §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // Direct sub-expression → parallel task 1
    t2 = fetch("url2")   // Direct sub-expression → parallel task 2
    return (t1, t2)      // Explicit tuple return
}

// spawn for: each iteration in parallel
results = spawn for item in items {
    process(item)        // Each iteration → independent task
}

// spawn while: each round of iteration in parallel
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

spawn expressions create independent scopes; internal variables do not affect the outside:

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
// Outer item is unaffected
```

**Iteration variables** (the `x` in `for`) get an independent copy per round, automatically
destroyed when the iteration ends.

#### 2.3 Ownership Rules

Once a variable enters a spawn expression, it cannot be used outside (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into the spawn expression
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

**Cross-iteration sharing**: Use `ref` to capture into the outer scope, sharing the same reference
across iterations.

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
- Remaining iterations **continue executing** after a failed iteration (not cancelled)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // Any iteration fails → wait for all to complete → propagate first error
}
```

##### `spawn while cond { body? }`

Inherits the error semantics of `while` itself:

- `step` uses `?` to propagate error → the entire `spawn while` fails, no more rounds
- `step` does not propagate error (error is swallowed) → proceeds to next iteration

```yaoxiang
spawn while has_next() {
    item = next()       // When not propagating errors, failures still enter the next round
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c is **evaluated sequentially**
- Error evaluating c → overall error
- Error inside selected branch → overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks the usage of resource types to ensure concurrency safety:

| Resource Type | Description         | Compiler Behavior                         |
| ------------- | ------------------- | ----------------------------------------- |
| `FilePath`    | Filesystem path     | Same-path operations auto-serialize       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations auto-serialize        |
| `DBUrl`       | Database connection | Same-connection operations auto-serialize |
| `Console`     | Standard output     | All Console operations auto-serialize     |

##### Inside `spawn { ... }` Block

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for read to complete
}
```

##### `spawn for ... { ... }` Same Resource Across Iterations

When all iterations operate on the same resource type, the compiler **automatically degrades to
serial** (spawn degenerates to sequential for, no error):

```yaoxiang
// All iterations write to the same file path → automatically degrades to serial
results = spawn for item in items {
    write_file("data.txt", item)
}
// Compiler automatically serializes all iterations
```

> **Design rationale**: The `spawn` keyword still expresses parallel intent; the compiler
> automatically degrades on resource conflicts, which is more aligned with the principle of least
> surprise than outright rejection.

##### `spawn while ... { ... }` Capturing `&mut`

**Compile-time error**: `spawn while` does not allow capturing external variables of `&mut` type:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // Compile-time error
    item = iter.next()              // iter is &mut, sharing mutable across iterations = data race
}
```

> **Not reintroducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" promise. Users are
> required to use `ref` or non-spawn syntax.

##### `spawn if c { ... } else { ... }` Same Resource in Both Branches

**Legal without warning**: The if conditions are mutually exclusive; at most one branch executes, so
no concurrency conflict:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // Branch 1: read cache
} else {
    fetch(key)                      // Branch 2: read URL
}
```

#### 2.6 Nested spawn

spawn expressions can be nested; the inner layer creates an **independent concurrency domain**:

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**Nesting Semantics**:

- The inner spawn is an independent concurrency domain (independent task queue, independent error
  propagation)
- Inner errors are independently propagated to the outer layer (outer task receives error when
  waiting for inner completion)
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

### 3. Break with the Old Design

| Old Design (RFC-001)                  | New Design (RFC-024 + RFC-032)                       |
| ------------------------------------- | ---------------------------------------------------- |
| Whole-program automatic DAG analysis  | Only inside spawn expressions                        |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven                    |
| `Send`/`Sync` traits                  | Not needed, ownership + ref handles it automatically |
| `future`/non-blocking handles         | Synchronous blocking, no callbacks                   |
| `Mutex`/`RwLock`/`Atomic`             | `ref` auto-selects Rc/Arc                            |
| L1/L2/L3 three-layer mental model     | Normal code sequential, spawn expressions parallel   |
| Function coloring (async/await)       | No function coloring                                 |
| `spawn` only modifies `{}` blocks     | `spawn` modifies any expression (see RFC-032)        |

### 4. Return Rules

YaoXiang's return rules are unified and clear:

| Syntax                          | Return Value                                | Description                 |
| ------------------------------- | ------------------------------------------- | --------------------------- |
| `= expr` (no curly braces)      | Directly returns `expr`                     | Expression is the value     |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` | Block needs explicit return |

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
> The shape of the expression determines how tasks are decomposed: each direct sub-expression in a
> block runs in parallel; each iteration in `for` runs in parallel; the selected branch in `if` is a
> single task.
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
2. **Explicit**: Users clearly know where parallel and where sequential, no implicit concurrency
3. **Safe**: Ownership rules extend naturally, no additional type constraints like `Send`/`Sync`
   needed
4. **Controllable**: No automatic parallel behavior, avoiding unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function coloring issues
7. **Compilation efficient**: DAG analysis only within spawn expressions, compile time is
   controllable
8. **Orthogonality**: spawn naturally composes with any control flow structure (see RFC-032)

### Disadvantages

1. **Requires explicit spawn**: Cannot auto-parallelize, users need to manually mark parallel points
2. **DAG analysis within spawn expressions**: The compiler needs to perform dependency analysis
   within spawn expressions
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs migration

---

## Alternatives

| Alternative                           | Why Not Chosen                                                         |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Whole-program automatic DAG (RFC-001) | High complexity, long compile time, uncontrollable behavior            |
| async/await                           | Function coloring, steep learning curve, poor readability              |
| goroutine                             | No type safety, data races hard to detect                              |
| Actor model                           | Message passing complex, debugging difficult                           |
| CSP (Go channel)                      | No type safety, deadlocks hard to detect                               |
| `spawn` only modifies `{}` blocks     | Breaks orthogonality, `spawn for` becomes a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the shape of the
   expression after spawn (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependency relationships within spawn expressions
3. **Topological sorting**: Determine execution order within spawn expressions
4. **Parallelism identification**: Identify dependency-free subtrees within spawn expressions
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts on resource types

### Module Organization

spawn-related code is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry
├── placement.rs     # spawn occurrence position legality check
└── analysis.rs      # task identification, dependency analysis, resource conflict detection
```

> **Migration note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs`
> will be migrated to `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration
> under the `typecheck/passes/` directory needs to be removed in sync.

### Runtime Execution

Referencing the Runtime architecture from [RFC-008](./008-runtime-concurrency-model.md):

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

## Design Decision Records

| Decision                       | Decision                                          | Reason                                                      | Date       |
| ------------------------------ | ------------------------------------------------- | ----------------------------------------------------------- | ---------- |
| Parallel primitive             | `spawn <expr>`                                    | Simple, explicit, controllable                              | 2026-06-05 |
| spawn modifier scope           | Any expression                                    | Syntax orthogonality, eliminates `spawn for` specialization | 2026-07-04 |
| Task decomposition             | Determined by expression shape                    | Expressive, unified rules                                   | 2026-07-04 |
| Execution model                | Synchronous blocking                              | Easy to understand, debug                                   | 2026-06-05 |
| DAG analysis scope             | Only inside spawn expressions                     | Compilation efficient, controllable behavior                | 2026-06-05 |
| Sharing mechanism              | `ref` auto-selects Rc/Arc                         | Simplifies user decisions                                   | 2026-06-05 |
| Annotations                    | None                                              | Reduces code noise                                          | 2026-06-05 |
| Send/Sync                      | Removed                                           | Ownership + ref is sufficient                               | 2026-06-05 |
| Mutex/RwLock                   | Removed                                           | `ref` handles it automatically                              | 2026-06-05 |
| future/handle                  | Removed                                           | Synchronous blocking is simpler                             | 2026-06-05 |
| Function coloring              | None                                              | Avoids async/await issues                                   | 2026-06-05 |
| Resource types                 | Built-in + user-defined                           | Auto-serialization                                          | 2026-06-05 |
| `spawn {}` error               | Wait for all to complete, propagate first error   | Deterministic behavior                                      | 2026-06-05 |
| `spawn for` error              | Wait for all to complete, propagate first error   | Consistent with `spawn {}`                                  | 2026-07-04 |
| `spawn while` error            | Inherits `while` error semantics                  | Standard `while` behavior                                   | 2026-07-04 |
| `spawn if` condition error     | c evaluated sequentially, failure → overall error | Intuitive                                                   | 2026-07-04 |
| `spawn for` same resource      | Auto-degrade to serial                            | Safe degradation, no abrupt rejection                       | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error                                | Avoid data races, no Sync introduction                      | 2026-07-04 |
| `spawn if` same resource       | Legal without warning                             | Mutually exclusive branches don't conflict                  | 2026-07-04 |
| Nested spawn                   | Inner independent concurrency domain              | Independent task queue, errors, resources                   | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](/reference/language-spec/concurrency.md)
- [RFC-001 Concurrent Model (Deprecated)](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generics System](./011-generic-type-system.md)
- [RFC-032 spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

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
