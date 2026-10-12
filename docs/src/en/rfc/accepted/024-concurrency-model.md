---
title: 'RFC-024: Runtime Semantics of spawn-based Concurrency'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-05'
updated:
  '2026-07-05 (RFC sync check: implementation progress ~85%, core runtime and frontend analysis
  completed)'

issue: '#89'
---

# RFC-024: Runtime Semantics of spawn-based Concurrency

> **This document defines the runtime behavior semantics of `spawn`**. For syntactic orthogonality,
> AST/IR refactoring, and type system extensions, see
> [RFC-032](../review/032-spawn-unified-expression.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what it does", and 032 answers "how it is
> represented".

> **References**:
>
> - [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
> - [RFC-008: Decoupled Design of Runtime Concurrency Model and Scheduler](008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
> - [RFC-032: spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavior semantics** of the `spawn` keyword in the YaoXiang
programming language: `spawn <expr>` is the sole parallel primitive, can modify any expression, and
the caller blocks synchronously. The shape of the expression determines the task decomposition
granularity, and the runtime schedules tasks according to the GMP model — independent tasks are
dropped into the work queue and workers race to run them.

**Core design — one primitive, one set of rules**:

```
spawn <expr>               ← the only parallel primitive
Task decomposition is decided by the expression shape  ← the only rule
Synchronous blocking to wait for results              ← the only behavior
```

**Complexity eliminated**:

- ❌ No `@block` / `@eager` / `@auto` annotations
- ❌ No `Send` / `Sync` trait
- ❌ No `Mutex` / `RwLock` / `Atomic`
- ❌ No `future` / non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async / await)

> **User mental model**: your ordinary code runs sequentially. When you want multiple things to
> happen together, put them inside `spawn <expr>`. No callbacks, no `await`, no strange annotations.

## Design Provenance

| Document                                                        | Relationship                                                 |
| --------------------------------------------------------------- | ------------------------------------------------------------ |
| [RFC-001](../deprecated/001-concurrent-model-error-handling.md) | Superseded by this document                                  |
| [RFC-008](008-runtime-concurrency-model.md)                     | Runtime architecture, orthogonal to this document            |
| [RFC-009](009-ownership-model.md)                               | Ownership model, unchanged                                   |
| [RFC-010](010-unified-type-syntax.md)                           | Unified type syntax                                          |
| [RFC-032](../review/032-spawn-unified-expression.md)            | AST/IR refactoring, jointly defines spawn with this document |

## Motivation

### Why is this design needed?

Current mainstream languages' concurrency models have obvious flaws:

| Language   | Concurrency Model   | Problem                                                  |
| ---------- | ------------------- | -------------------------------------------------------- |
| Rust       | async/await + tokio | Async infection, function coloring, steep learning curve |
| Go         | goroutine           | No type safety, data races hard to detect                |
| Python     | asyncio             | GIL limitation, function coloring                        |
| JavaScript | Promise/async       | Callback hell, function coloring                         |

### Problems with the old design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed by RFC-001 has the following problems:

| Problem                  | Description                                                   |
| ------------------------ | ------------------------------------------------------------- |
| Complex mental model     | L1/L2/L3 three-layer abstraction increases learning burden    |
| Annotation bloat         | `@block` / `@eager` / `@auto` annotations make code noisy     |
| High analysis complexity | Whole-program DAG analysis incurs large compile-time overhead |
| Complex type constraints | `Send` / `Sync` trait adds cognitive burden                   |
| Uncontrollable           | Automatic concurrent behavior is hard to predict and debug    |

### Design Goals

1. **Simple**: only one parallel primitive (`spawn`), which can modify any expression
2. **Explicit**: users clearly know where things run in parallel and where sequentially
3. **Safe**: ownership rules extend naturally, with no extra type constraints required
4. **Controllable**: no implicit concurrency, no accidental parallel behavior
5. **Synchronous**: caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of a `{}` Block: a Dependency-driven Computation Unit

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property          | Description                                                                                                                                |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Dependency-driven | The block checks whether all internal variables are ready at execution; it runs immediately when all are ready, otherwise blocks and waits |
| Execution timing  | Determined by dependencies, unrelated to "immediate" or "delayed"                                                                          |
| Return value      | Use `return` to explicitly return a value; no `return` means `Void` is returned by default                                                 |
| Syntax unified    | Whether appearing in a function body, a variable initialization, or after `spawn`, the semantics are consistent                            |
| Scope isolation   | Variables are strictly confined to the inside of `{}` and do not leak to the outer scope                                                   |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // depends on x and y, runs immediately once both are ready
    return x + y
}
```

### 2. Semantics of the spawn Expression

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. It can modify any expression, and the
shape of the expression determines the task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape                | Task Decomposition                                                          | Synchronous Semantics                  |
| ------------------------------- | --------------------------------------------------------------------------- | -------------------------------------- |
| `spawn { a, b, c }`             | Direct sub-expressions → N independent tasks                                | Wait for all tasks to finish           |
| `spawn for x in items { body }` | Each iteration → 1 task                                                     | Wait for all iterations to finish      |
| `spawn while cond { body }`     | Each round of iteration → 1 task (condition-driven between iterations)      | Wait for the condition to become false |
| `spawn if c { a } else { b }`   | Condition c evaluated sequentially, the selected branch as a whole → 1 task | Wait for the selected branch to finish |
| `spawn call(x)`                 | The call itself → 1 task                                                    | Wait for the call to finish            |
| `spawn expr` (any expression)   | The expression itself → 1 task                                              | Wait for the expression to finish      |

> **Design motivation**: why can spawn modify any expression? See
> [RFC-032 §Core Design](../review/032-spawn-unified-expression.md).
>
> **Control flow orthogonality**: the semantic difference between `spawn <expr>` (spawn before) and
> `<expr> spawn { body }` (spawn after) is detailed in
> [RFC-032 §Control Flow Orthogonality](../review/032-spawn-unified-expression.md) (core
> definition). The runtime behavior of all reversed combinations (`for ... spawn { }` /
> `while ... spawn { }` / `if ... spawn { }`) — error propagation, resource types, nesting rules —
> inherits the rules in §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions run in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // direct sub-expression → parallel task 1
    t2 = fetch("url2")   // direct sub-expression → parallel task 2
    return (t1, t2)      // explicitly return a tuple
}

// spawn for: each iteration runs in parallel
results = spawn for item in items {
    process(item)        // each iteration → independent task
}

// spawn while: each round of iteration runs in parallel
spawn while has_next() {
    step()               // each round of iteration → independent task
}

// spawn if: the selected branch as a whole is a task
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 Scope Isolation

A spawn expression creates an independent scope; its internal variables do not affect the outside:

```yaoxiang
x = 10
result = spawn {
    x = 20              // this is a local x inside the spawn expression
    compute(x)
}
// x is still 10

result = spawn for item in items {
    item = item + 1     // iteration-local item, an independent copy per iteration
    process(item)
}
// the outer item is not affected
```

**Iteration variables** (the `x` of `for`) have an independent copy per round, and are automatically
destroyed when the iteration ends.

#### 2.3 Ownership Rules

After a variable enters a spawn expression, the outside can no longer use it (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // ownership of data moves into the spawn expression
}
// data is not usable here (already moved)
```

If sharing across multiple tasks is needed, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // the compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // shared reference
    process_b(shared)   // shared reference
}
```

**Sharing across iterations**: use `ref` to capture into the outer scope, so the same reference is
shared between iterations.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (block)

1. Wait for all tasks to finish (even if some tasks have already failed)
2. Propagate the first error encountered
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // may fail
    fetch("url2")?      // may fail
}
// if any task fails, the entire spawn expression propagates the first error
```

##### `spawn for x in items { body? }`

- Wait for all iterations to finish before returning the first error
- Remaining iterations **continue to execute** after a failed iteration (no cancellation)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // any iteration failing → wait for all to finish → propagate the first error
}
```

##### `spawn while cond { body? }`

Inherits while's own error semantics:

- If `step` uses `?` to propagate an error → the whole `spawn while` fails and no more iterations
  are started
- If `step` does not propagate the error (the error is swallowed) → the next iteration proceeds

```yaoxiang
spawn while has_next() {
    item = next()       // when errors are not propagated, failures still advance to the next round
    process(item)
}
```

##### `spawn if c { a } else { b }`

- The condition c is **evaluated sequentially**
- An error evaluating c → an overall error
- An error inside the selected branch → an overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks the use of resource types to ensure concurrency safety:

| Resource Type | Description         | Compiler Behavior                                              |
| ------------- | ------------------- | -------------------------------------------------------------- |
| `FilePath`    | Filesystem path     | Operations on the same path are automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Operations on the same URL are automatically serialized        |
| `DBUrl`       | Database connection | Operations on the same connection are automatically serialized |
| `Console`     | Standard output     | All Console operations are automatically serialized            |

##### Inside `spawn { ... }` Block

```yaoxiang
// operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // runs first
    write_file("data.txt", x)   // waits for the read to complete
}
```

##### `spawn for ... { ... }` Same Resource Across Iterations

When all iterations operate on the same resource type, the compiler **automatically degrades to
sequential** (spawn degenerates to a sequential `for`, without error):

```yaoxiang
// all iterations write to the same file path → automatically degraded to sequential
results = spawn for item in items {
    write_file("data.txt", item)
}
// the compiler automatically serializes all iterations
```

> **Design rationale**: the spawn keyword still expresses the parallel intent; when resources
> conflict, the compiler automatically degrades, which better follows the principle of least
> surprise than outright rejection.

##### `spawn while ... { ... }` Capturing `&mut`

**Compile-time error**: `spawn while` does not allow capturing `&mut` typed variables from the
outside:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // compile-time error
    item = iter.next()              // iter is &mut, shared mutable across iterations = data race
}
```

> **Not reintroducing the `Sync` trait**: consistent with RFC-024's "no Send/Sync" promise. Users
> are required to switch to `ref` or to a non-spawn form.

##### `spawn if c { ... } else { ... }` Same Resource in Both Branches

**Legal without warning**: the if conditions are mutually exclusive, at most one branch executes, so
there is no concurrency conflict:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // branch 1: read cache
} else {
    fetch(key)                      // branch 2: read URL
}
```

#### 2.6 Nested spawn

A spawn expression can be nested; the inner layer creates an **independent concurrency domain**:

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

- The inner spawn is an independent concurrency domain (independent task queue, independent error
  propagation)
- Inner errors propagate independently to the outer layer (the outer task receives the error while
  waiting for the inner to complete)
- Inner resource type rules are tracked independently (not jointly checked with the outer layer)

```yaoxiang
// spawn for nested inside spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Breaking with the Old Design

| Old Design (RFC-001)                      | New Design (RFC-024 + RFC-032)                                |
| ----------------------------------------- | ------------------------------------------------------------- |
| Whole-program automatic DAG analysis      | Analysis only inside spawn expressions                        |
| `@block` / `@eager` / `@auto` annotations | No annotations, dependency-driven                             |
| `Send` / `Sync` trait                     | Not needed; ownership + `ref` handle everything automatically |
| `future` / non-blocking handles           | Synchronous blocking, no callbacks                            |
| `Mutex` / `RwLock` / `Atomic`             | `ref` automatically selects Rc / Arc                          |
| L1 / L2 / L3 three-layer mental model     | Ordinary code sequential, spawn expressions parallel          |
| Function coloring (async/await)           | No function coloring                                          |
| `spawn` only modifies `{}` blocks         | `spawn` modifies any expression (see RFC-032)                 |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

| Notation                  | Return Value                                    | Description                           |
| ------------------------- | ----------------------------------------------- | ------------------------------------- |
| `= expr` (no braces)      | Directly return `expr`                          | The expression is the value           |
| `= { ... }` (with braces) | Must use `return`, otherwise `Void` is returned | The block requires an explicit return |

```yaoxiang
// no braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// with braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// with braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // no return, returns Void
}
```

### 5. User Mental Model

> **Your ordinary code runs sequentially.**
>
> **When you want multiple things to happen together, put them inside `spawn <expr>`.**
>
> The shape of the expression decides how tasks are decomposed: every direct sub-expression in a
> block runs in parallel; every iteration of a `for` runs in parallel; the selected branch of an
> `if` is one task.
>
> **The whole spawn expression blocks synchronously, waiting for all tasks to finish.**
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// ordinary code: runs sequentially
a = compute_a()         // runs first
b = compute_b(a)        // depends on a, runs after a is done
c = compute_c(b)        // depends on b, runs after b is done

// when parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // parallel
    fetch("url2"),      // parallel
    fetch("url3")       // parallel
}
// continue after all are done
process(x, y, z)

// data parallelism: spawn for
results = spawn for item in items {
    process(item)
}
```

---

## Trade-offs

### Advantages

1. **Simple**: only one parallel primitive (`spawn`), which can modify any expression
2. **Explicit**: users clearly know where things run in parallel and where sequentially, with no
   implicit concurrency
3. **Safe**: ownership rules extend naturally, with no extra type constraints like `Send` / `Sync`
   required
4. **Controllable**: no automatic parallel behavior, avoiding accidental concurrency issues
5. **Synchronous**: caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: no async/await function-coloring problem
7. **Compilation efficiency**: DAG analysis is confined within spawn expressions, compile time is
   controllable
8. **Orthogonality**: spawn composes naturally with any control flow structure (see RFC-032)

### Disadvantages

1. **Requires explicit spawn**: cannot parallelize automatically, users need to manually mark
   parallel points
2. **DAG analysis inside spawn expressions**: the compiler needs to perform dependency analysis
   within spawn expressions
3. **Incompatible with old code**: code using the old RFC-001 patterns needs to be migrated

---

## Alternatives

| Alternative                           | Why Not Chosen                                                        |
| ------------------------------------- | --------------------------------------------------------------------- |
| Whole-program automatic DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior          |
| async/await                           | Function coloring, steep learning curve, poor code readability        |
| goroutine                             | No type safety, data races hard to detect                             |
| Actor model                           | Message passing is complex, debugging is difficult                    |
| CSP (Go channel)                      | No type safety, deadlocks hard to detect                              |
| `spawn` only modifies `{}` blocks     | Breaks orthogonality, making `spawn for` a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: determine task decomposition based on the shape of the
   expression after spawn (see RFC-032 §DAG Analysis)
2. **DAG construction**: analyze dependency relations inside spawn expressions
3. **Topological sort**: determine the execution order inside spawn expressions
4. **Parallelism identification**: identify subtrees with no dependencies inside spawn expressions
5. **Escape analysis**: choose Rc or Arc for `ref`
6. **Resource conflict detection**: detect potential conflicts of resource types

### Module Organization

spawn-related code is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry point
├── placement.rs     # spawn occurrence location validity check
└── analysis.rs      # task identification, dependency analysis, resource conflict detection
```

> **Migration note** (2026-06-11): the existing `frontend/core/typecheck/passes/spawn_placement.rs`
> will be migrated to `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration
> under the `typecheck/passes/` directory needs to be removed accordingly.

### Runtime Execution

Referencing the Runtime architecture in [RFC-008](008-runtime-concurrency-model.md):

- **Embedded Runtime**: no spawn support, executes immediately
- **Standard Runtime**: supports spawn expressions
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generics system) → Completed
- RFC-032 (AST/IR refactoring) → jointly defines spawn with this document

---

## Design Decision Record

| Decision                       | Decision                                          | Reason                                                         | Date       |
| ------------------------------ | ------------------------------------------------- | -------------------------------------------------------------- | ---------- |
| Parallel primitive             | `spawn <expr>`                                    | Simple, explicit, controllable                                 | 2026-06-05 |
| Modification scope of spawn    | Any expression                                    | Syntactic orthogonality, eliminates `spawn for` special-casing | 2026-07-04 |
| Task decomposition             | Determined by expression shape                    | Strong expressiveness, unified rules                           | 2026-07-04 |
| Execution model                | Synchronous blocking                              | Easy to understand and debug                                   | 2026-06-05 |
| DAG analysis scope             | Only within spawn expressions                     | Efficient compilation, controllable behavior                   | 2026-06-05 |
| Sharing mechanism              | `ref` automatically selects Rc / Arc              | Simplifies user decisions                                      | 2026-06-05 |
| Annotations                    | None                                              | Reduce code noise                                              | 2026-06-05 |
| Send / Sync                    | Removed                                           | Ownership + `ref` are sufficient                               | 2026-06-05 |
| Mutex / RwLock                 | Removed                                           | `ref` handles automatically                                    | 2026-06-05 |
| future / handles               | Removed                                           | Synchronous blocking is simpler                                | 2026-06-05 |
| Function coloring              | None                                              | Avoid async / await problems                                   | 2026-06-05 |
| Resource types                 | Built-in + user-defined                           | Automatic serialization                                        | 2026-06-05 |
| `spawn {}` errors              | Wait for all to finish, propagate the first error | Deterministic behavior                                         | 2026-06-05 |
| `spawn for` errors             | Wait for all to finish, propagate the first error | Consistent with `spawn {}`                                     | 2026-07-04 |
| `spawn while` errors           | Inherits while error semantics                    | Standard while behavior                                        | 2026-07-04 |
| `spawn if` condition error     | c evaluated sequentially, failure → overall error | Intuitive                                                      | 2026-07-04 |
| `spawn for` same resource      | Automatically degraded to sequential              | Safe degradation, not brutal rejection                         | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error                                | Avoid data races, do not introduce Sync                        | 2026-07-04 |
| `spawn if` same resource       | Legal without warning                             | Mutually exclusive branches do not conflict                    | 2026-07-04 |
| Nested spawn                   | Inner layer is an independent concurrency domain  | Independent task queues, errors, resources                     | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
- [RFC-001 Concurrency Model (Deprecated)](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](009-ownership-model.md)
- [RFC-010 Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011 Generics System](011-generic-type-system.md)
- [RFC-032 spawn Unified Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| State        | Location                    | Description                                                                |
| ------------ | --------------------------- | -------------------------------------------------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Supersedes RFC-001, jointly defines spawn with RFC-032 (runtime semantics) |
