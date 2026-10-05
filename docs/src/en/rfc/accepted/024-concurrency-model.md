---
title: 'RFC-024: spawn-based Concurrency Runtime Semantics'
status: 'accepted'
author: 'Chenxu'
created: '2026-06-05'
updated:
  '2026-07-05 (RFC sync check: implementation progress ~85%, core runtime and frontend analysis
  completed)'

issue: '#89'
---

# RFC-024: spawn-based Concurrency Runtime Semantics

> **This document defines the runtime behavior semantics of `spawn`**. Syntax orthogonality, AST/IR
> refactoring, and type system extensions are covered in
> [RFC-032](../review/032-spawn-unified-expression.md).
>
> The two RFCs jointly define `spawn` — RFC-024 answers "what to do", RFC-032 answers "how to
> represent it".

> **References**:
>
> - [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model & Scheduler Decoupling Design](008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
> - [RFC-032: spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavior semantics** of `spawn` in the YaoXiang programming
language: `spawn <expr>` is the sole parallelism primitive, can modify any expression, and the
caller blocks synchronously. The shape of the expression determines the granularity of task
decomposition, and the runtime schedules according to the GMP model — tasks without dependencies are
thrown into the work queue, and workers race to run them.

**Core design — one primitive, one set of rules**:

```
spawn <expr>               ← The only parallelism primitive
Task decomposition determined by expression shape    ← The only rule
Synchronously block waiting for results              ← The only behavior
```

**Complexity eliminated**:

- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User mental model**: The ordinary code you write is executed sequentially. When you want
> multiple things to happen together, put them in `spawn <expr>`. No callbacks, no `await`, no
> strange annotations.

## Design Origins

| Document                                                        | Relationship                                                 |
| --------------------------------------------------------------- | ------------------------------------------------------------ |
| [RFC-001](../deprecated/001-concurrent-model-error-handling.md) | Superseded by this document                                  |
| [RFC-008](008-runtime-concurrency-model.md)                   | Runtime architecture, orthogonal to this document            |
| [RFC-009](009-ownership-model.md)                             | Ownership model, unchanged                                   |
| [RFC-010](010-unified-type-syntax.md)                         | Unified type syntax                                          |
| [RFC-032](../review/032-spawn-unified-expression.md)            | AST/IR refactoring, jointly defines spawn with this document |

## Motivation

### Why is this design needed?

The concurrency models of current mainstream languages have obvious defects:

| Language   | Concurrency Model   | Problem                                                  |
| ---------- | ------------------- | -------------------------------------------------------- |
| Rust       | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go         | goroutine           | No type safety, data races hard to detect                |
| Python     | asyncio             | GIL limitation, function coloring                        |
| JavaScript | Promise/async       | Callback hell, function coloring                         |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following problems:

| Problem                  | Description                                                |
| ------------------------ | ---------------------------------------------------------- |
| Complex mental model     | L1/L2/L3 three-layer abstraction increases learning burden |
| Annotation noise         | `@block`/`@eager`/`@auto` annotations make code noisy      |
| High analysis complexity | Whole-program DAG analysis has large compile-time overhead |
| Complex type constraints | `Send`/`Sync` trait increases cognitive burden             |
| Uncontrollable           | Automatic concurrent behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallelism primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism is, where sequential
3. **Safe**: Ownership rules extend naturally, no additional type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks and `await`

---

## Proposal

### 1. The Nature of `{}` Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property          | Description                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven | The block checks at execution whether all internal variables are ready; if so it executes immediately, otherwise blocks waiting |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "deferred"                                                              |
| Return value      | Use `return` to explicitly return a value; without `return` it defaults to returning `Void`                                     |
| Unified syntax    | Whether appearing in function body, variable initialization, or after `spawn`, semantics are consistent                         |
| Scope isolation   | Variables are strictly limited to the inside of `{}`, do not leak into the outer scope                                          |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // Depends on x and y; executes immediately once both are ready
    return x + y
}
```

### 2. spawn Expression Semantics

`spawn <expr>` is the **only parallelism primitive** in YaoXiang. It can modify any expression, and
the shape of the expression determines the granularity of task decomposition.

#### 2.1 Task Creation Rules

| Expression Shape                | Task Decomposition                                                      | Synchronous Semantics                |
| ------------------------------- | ----------------------------------------------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | Direct sub-expressions → N independent tasks                            | Wait for all tasks to complete       |
| `spawn for x in items { body }` | Each iteration → 1 task                                                 | Wait for all iterations to complete  |
| `spawn while cond { body }`     | Each loop iteration → 1 task (condition-driven between iterations)      | Wait for condition to become false   |
| `spawn if c { a } else { b }`   | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)`                 | The call itself → 1 task                                                | Wait for the call to complete        |
| `spawn expr` (any expression)   | The expression itself → 1 task                                          | Wait for the expression to complete  |

> **Design motivation**: Why can `spawn` modify any expression? See
> [RFC-032 §Core Design](../review/032-spawn-unified-expression.md).
>
> **Control flow orthogonality**: The semantic differences between `spawn <expr>` (`spawn` in front)
> and `<expr> spawn { body }` (`spawn` after), see
> [RFC-032 §Control Flow Orthogonality](../review/032-spawn-unified-expression.md) (core
> definition). The runtime behavior of all reverse-write combinations (`for ... spawn { }` /
> `while ... spawn { }` / `if ... spawn { }`) — error propagation, resource types, nesting rules —
> inherits the rules in §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions run in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // Direct sub-expression → parallel task 1
    t2 = fetch("url2")   // Direct sub-expression → parallel task 2
    return (t1, t2)      // Explicitly return tuple
}

// spawn for: each iteration runs in parallel
results = spawn for item in items {
    process(item)        // Each iteration → independent task
}

// spawn while: each loop iteration runs in parallel
spawn while has_next() {
    step()               // Each loop iteration → independent task
}

// spawn if: the selected branch as a whole is one task
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 Scope Isolation

A `spawn` expression creates an independent scope; internal variables do not affect the outside:

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
// The outer item is unaffected
```

**Iteration variable** (the `x` in `for`) gets an independent copy per loop; automatically destroyed
when the iteration ends.

#### 2.3 Ownership Rules

Once a variable enters a `spawn` expression, the outside can no longer use it (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // Ownership of data moves into the spawn expression
}
// data is unusable here (already moved)
```

If you need to share across multiple tasks, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // The compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // Shared reference
    process_b(shared)   // Shared reference
}
```

**Cross-iteration sharing**: capture with `ref` to the outside, the same reference is shared between
iterations.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (block)

1. Wait for all tasks to complete (even if some tasks have already failed)
2. Propagate the first encountered error
3. Use `?` to explicitly mark the error propagation point

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // May fail
    fetch("url2")?      // May fail
}
// If any task fails, the whole spawn expression propagates the first error
```

##### `spawn for x in items { body? }`

- Wait for all iterations to complete before returning the first error
- After a failed iteration, remaining iterations **continue to execute** (not cancelled)
- Use `?` to explicitly mark the error propagation point

```yaoxiang
results = spawn for item in items {
    process(item)?      // Any iteration fails → wait for all to complete → propagate the first error
}
```

##### `spawn while cond { body? }`

Inherits the error semantics of `while` itself:

- `step` uses `?` to propagate error → the whole `spawn while` fails, no next iteration
- `step` does not propagate error (error is swallowed) → proceed to the next iteration

```yaoxiang
spawn while has_next() {
    item = next()       // When not propagating error, failure still goes to next iteration
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c is **evaluated sequentially**
- Error during c evaluation → whole error
- Error inside the selected branch → whole error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially; failure → whole error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks the use of resource types to ensure concurrency safety:

| Resource Type | Description         | Compiler Behavior                          |
| ------------- | ------------------- | ------------------------------------------ |
| `FilePath`    | Filesystem path     | Same-path operations auto-serialized       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations auto-serialized        |
| `DBUrl`       | Database connection | Same-connection operations auto-serialized |
| `Console`     | Standard output     | All Console operations auto-serialized     |

##### Inside `spawn { ... }` Block

```yaoxiang
// Operations on the same file are auto-serialized
(a, b) = spawn {
    read_file("data.txt"),      // Executes first
    write_file("data.txt", x)   // Waits for the read to complete
}
```

##### `spawn for ... { ... }` Same Resource Across Iterations

When all iterations operate on the same resource type, the compiler **automatically degrades to
sequential** (`spawn` degenerates into sequential `for`, no error):

```yaoxiang
// All iterations write to the same file path → automatically degrade to sequential
results = spawn for item in items {
    write_file("data.txt", item)
}
// The compiler automatically serializes all iterations
```

> **Design rationale**: The `spawn` keyword still expresses the intent of parallelism; on resource
> conflict the compiler auto-degrades, which aligns better with the principle of least surprise than
> outright rejection.

##### `spawn while ... { ... }` Captures `&mut`

**Compile-time error**: `spawn while` does not allow capturing external variables of `&mut` type:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // Compile-time error
    item = iter.next()              // iter is &mut; sharing mutable across iterations = data race
}
```

> **Not reintroducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" promise. Require the
> user to switch to `ref` or a non-`spawn` form.

##### `spawn if c { ... } else { ... }` Same Resource in Both Branches

**Legal, no warning**: The `if` condition is mutually exclusive; at most one branch executes, so no
concurrency conflict exists:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // Branch 1: read cache
} else {
    fetch(key)                      // Branch 2: read URL
}
```

#### 2.6 Nested spawn

`spawn` expressions can be nested; the inner one creates an **independent concurrency domain**:

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

- The inner `spawn` is an independent concurrency domain (independent task queue, independent error
  propagation)
- Inner errors propagate independently to the outer (the outer task receives the error while waiting
  for the inner to complete)
- Inner resource type rules are tracked independently (not jointly checked with the outer)

```yaoxiang
// spawn for nested in spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Break with the Old Design

| Old Design (RFC-001)                  | New Design (RFC-024 + RFC-032)                              |
| ------------------------------------- | ----------------------------------------------------------- |
| Whole-program automatic DAG analysis  | Analysis only inside `spawn` expressions                    |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven                           |
| `Send`/`Sync` trait                   | Not needed; ownership + `ref` handles it automatically      |
| `future`/non-blocking handles         | Synchronous blocking, no callbacks                          |
| `Mutex`/`RwLock`/`Atomic`             | `ref` automatically chooses Rc/Arc                          |
| L1/L2/L3 three-layer mental model     | Ordinary code is sequential; `spawn` expression is parallel |
| Function coloring (async/await)       | No function coloring                                        |
| `spawn` only modifies `{}` blocks     | `spawn` modifies any expression (see RFC-032)               |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

| Form                      | Return Value                                | Description                    |
| ------------------------- | ------------------------------------------- | ------------------------------ |
| `= expr` (no braces)      | Directly returns `expr`                     | Expression is the value        |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Block requires explicit return |

```yaoxiang
// No braces: directly return
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

> **The ordinary code you write is executed sequentially.**
>
> **When you want multiple things to happen together, put them in `spawn <expr>`.**
>
> The shape of the expression determines how tasks are split: every direct sub-expression inside a
> block runs in parallel; each iteration of `for` runs in parallel; the selected branch of `if` is
> one task.
>
> **The whole `spawn` expression blocks synchronously, waiting for all tasks to complete.**
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// Ordinary code: sequential execution
a = compute_a()         // Executes first
b = compute_b(a)        // Depends on a; executes after a is done
c = compute_c(b)        // Depends on b; executes after b is done

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

1. **Simple**: Only one parallelism primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism is, where sequential; no implicit concurrency
3. **Safe**: Ownership rules extend naturally; no extra type constraints such as `Send`/`Sync`
   needed
4. **Controllable**: No automatic parallel behavior, avoiding unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function-coloring problem
7. **Compile-efficient**: DAG analysis is limited to inside `spawn` expressions, compile time is
   controllable
8. **Orthogonal**: `spawn` composes naturally with any control-flow structure (see RFC-032)

### Disadvantages

1. **Requires explicit spawn**: Cannot parallelize automatically; users must manually mark parallel
   points
2. **DAG analysis inside spawn expressions**: The compiler needs to perform dependency analysis
   inside `spawn` expressions
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs to be migrated

---

## Alternatives

| Alternative                           | Why Not Chosen                                                         |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Whole-program automatic DAG (RFC-001) | High complexity, long compile time, uncontrollable behavior            |
| async/await                           | Function coloring, steep learning curve, poor readability              |
| goroutine                             | No type safety, data races hard to detect                              |
| Actor model                           | Message passing is complex, hard to debug                              |
| CSP (Go channel)                      | No type safety, deadlocks hard to detect                               |
| `spawn` only modifies `{}` blocks     | Breaks orthogonality; `spawn for` becomes a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the expression shape
   after `spawn` (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependency relations inside the `spawn` expression
3. **Topological sorting**: Determine execution order inside the `spawn` expression
4. **Parallelism identification**: Identify dependency-free sub-trees inside the `spawn` expression
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts in resource types

### Module Organization

`spawn`-related code is placed together under `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry
├── placement.rs     # Legality check of spawn occurrence position
└── analysis.rs      # Task identification, dependency analysis, resource conflict detection
```

> **Migration note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs`
> will migrate to `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration under
> the `typecheck/passes/` directory must be removed in sync.

### Runtime Execution

Refer to the Runtime architecture in [RFC-008](008-runtime-concurrency-model.md):

- **Embedded Runtime**: No `spawn` support; immediate execution
- **Standard Runtime**: Supports `spawn` expressions
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generic system) → Completed
- RFC-032 (AST/IR refactoring) → Jointly defines `spawn` with this document

---

## Design Decision Records

| Decision                      | Determination                                   | Reason                                                   | Date       |
| ----------------------------- | ----------------------------------------------- | -------------------------------------------------------- | ---------- |
| Parallelism primitive         | `spawn <expr>`                                  | Simple, explicit, controllable                           | 2026-06-05 |
| Scope of `spawn` modification | Any expression                                  | Syntax orthogonality, eliminate `spawn for` special-case | 2026-07-04 |
| Task decomposition            | Determined by expression shape                  | Highly expressive, unified rules                         | 2026-07-04 |
| Execution model               | Synchronous blocking                            | Easy to understand and debug                             | 2026-06-05 |
| DAG analysis scope            | Only inside `spawn` expressions                 | Compile-efficient, controllable behavior                 | 2026-06-05 |
| Sharing mechanism             | `ref` automatically chooses Rc/Arc              | Simplifies user decision                                 | 2026-06-05 |
| Annotations                   | None                                            | Reduce code noise                                        | 2026-06-05 |
| Send/Sync                     | Removed                                         | Ownership + `ref` is enough                              | 2026-06-05 |
| Mutex/RwLock                  | Removed                                         | `ref` handles it automatically                           | 2026-06-05 |
| future/handle                 | Removed                                         | Synchronous blocking is simpler                          | 2026-06-05 |
| Function coloring             | None                                            | Avoid async/await problems                               | 2026-06-05 |
| Resource types                | Built-in + user-defined                         | Automatic serialization                                  | 2026-06-05 |
| `spawn {}` error              | Wait for all, propagate first error             | Deterministic behavior                                   | 2026-06-05 |
| `spawn for` error             | Wait for all, propagate first error             | Consistent with `spawn {}`                               | 2026-07-04 |
| `spawn while` error           | Inherits `while` error semantics                | Standard `while` behavior                                | 2026-07-04 |
| `spawn if` condition error    | c evaluated sequentially; failure → whole error | Intuitive                                                | 2026-07-04 |
| `spawn for` same resource     | Auto-degrade to sequential                      | Safe degradation, not blunt refusal                      | 2026-07-04 |
| `spawn while` capture `&mut`  | Compile-time error                              | Avoid data races, do not introduce `Sync`                | 2026-07-04 |
| `spawn if` same resource      | Legal, no warning                               | Mutually exclusive branches do not conflict              | 2026-07-04 |
| Nested spawn                  | Inner independent concurrency domain            | Independent task queue, error, resource                  | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
- [RFC-001 spawn Model (Deprecated)](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](009-ownership-model.md)
- [RFC-010 Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011 Generic System](011-generic-type-system.md)
- [RFC-032 spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status       | Location                    | Description                                                                  |
| ------------ | --------------------------- | ---------------------------------------------------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Supersedes RFC-001, jointly defines `spawn` with RFC-032 (runtime semantics) |
