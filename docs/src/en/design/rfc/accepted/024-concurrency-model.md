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

> **This document defines the runtime behavior semantics of `spawn`.** For syntax orthogonality,
> AST/IR refactoring, and type system extensions, see
> [RFC-032](../review/032-spawn-unified-expression.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **References**:
>
> - [Concurrency Model Specification](../../../reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-032: Unified spawn Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

## Summary

This document defines the **runtime behavior semantics** of YaoXiang's `spawn`: `spawn <expr>` is
the sole parallel primitive, can modify any expression, and the caller blocks synchronously. The
shape of the expression determines the task decomposition granularity, and the runtime schedules
according to the GMP model — dependency-free tasks are thrown into the work queue, and workers race
to run them.

**Core design — one primitive, one set of rules**:

```
spawn <expr>               ← sole parallel primitive
task decomposition determined by expression shape    ← sole rule
synchronous blocking wait for results    ← sole behavior
```

**Eliminated complexity**:

- ❌ No `@block`/`@eager`/`@auto` annotations
- ❌ No `Send`/`Sync` trait
- ❌ No `Mutex`/`RwLock`/`Atomic`
- ❌ No `future`/non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User mental model**: Your normal code executes sequentially. When you want multiple things to
> happen together, put them inside `spawn <expr>`. No callbacks, no `await`, no strange annotations.

## Design Origins

| Document                                                        | Relationship                                                 |
| --------------------------------------------------------------- | ------------------------------------------------------------ |
| [RFC-001](../deprecated/001-concurrent-model-error-handling.md) | Superseded by this document                                  |
| [RFC-008](./008-runtime-concurrency-model.md)                   | Runtime architecture, orthogonal to this document            |
| [RFC-009](./009-ownership-model.md)                             | Ownership model, invariant                                   |
| [RFC-010](./010-unified-type-syntax.md)                         | Unified type syntax                                          |
| [RFC-032](../review/032-spawn-unified-expression.md)            | AST/IR refactoring, jointly defines spawn with this document |

## Motivation

### Why is this design needed?

Current mainstream language concurrency models have obvious flaws:

| Language   | Concurrency Model   | Problems                                                 |
| ---------- | ------------------- | -------------------------------------------------------- |
| Rust       | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go         | goroutine           | No type safety, hard to detect data races                |
| Python     | asyncio             | GIL limitation, function coloring                        |
| JavaScript | Promise/async       | Callback hell, function coloring                         |

### Problems with the Old Design (RFC-001)

The three-layer concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following problems:

| Problem                  | Description                                                 |
| ------------------------ | ----------------------------------------------------------- |
| Complex mental model     | L1/L2/L3 three-layer abstraction increases learning burden  |
| Redundant annotations    | `@block`/`@eager`/`@auto` annotations make code noisy       |
| High analysis complexity | Whole-program DAG analysis has high compile-time overhead   |
| Complex type constraints | `Send`/`Sync` trait increases cognitive burden              |
| Uncontrollable           | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism happens and where it is sequential
3. **Safe**: Ownership rules extend naturally, no additional type constraints needed
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: Caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of {} Blocks: Dependency-driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property           | Description                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Dependency-driven  | The block checks whether all internal variables are ready during execution; if so it executes immediately, otherwise it blocks and waits |
| Execution timing   | Determined by dependencies, unrelated to "immediate" or "deferred"                                                                       |
| Return value       | Use `return` to explicitly return a value; without `return`, it defaults to returning `Void`                                             |
| Syntax unification | The semantics are consistent regardless of whether it appears in a function body, variable initialization, or after `spawn`              |
| Scope isolation    | Variables are strictly limited to the inside of `{}`, do not leak into the outer scope                                                   |

```yaoxiang
// Dependency-driven example
x = compute_x()        // x is ready
y = compute_y()        // y is ready
result = {
    // depends on x and y, executes immediately when both are ready
    return x + y
}
```

### 2. spawn Expression Semantics

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. It can modify any expression, and the
shape of the expression determines the task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape                | Task Decomposition                                                      | Synchronous Semantics                |
| ------------------------------- | ----------------------------------------------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | Direct sub-expressions → N independent tasks                            | Wait for all tasks to complete       |
| `spawn for x in items { body }` | Each iteration → 1 task                                                 | Wait for all iterations to complete  |
| `spawn while cond { body }`     | Each round of iteration → 1 task (inter-iteration condition-driven)     | Wait for condition to be false       |
| `spawn if c { a } else { b }`   | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)`                 | The call itself → 1 task                                                | Wait for call to complete            |
| `spawn expr` (any expression)   | The expression itself → 1 task                                          | Wait for expression to complete      |

> **Design motivation**: Why can spawn modify any expression? See
> [RFC-032 §Core Design](../review/032-spawn-unified-expression.md).
>
> **Control flow orthogonality**: The semantic difference between `spawn <expr>` (spawn in front)
> and `<expr> spawn { body }` (spawn behind), see
> [RFC-032 §Control Flow Orthogonality](../review/032-spawn-unified-expression.md) (core
> definition). The runtime behavior of all reverse combinations (`for ... spawn { }` /
> `while ... spawn { }` / `if ... spawn { }`) — error propagation, resource types, nesting rules —
> inherits the rules in §2.4 / §2.5 / §2.6 of this document.

```yaoxiang
// spawn block: direct sub-expressions in parallel
(a, b) = spawn {
    t1 = fetch("url1")   // direct sub-expression → parallel task 1
    t2 = fetch("url2")   // direct sub-expression → parallel task 2
    return (t1, t2)      // explicit tuple return
}

// spawn for: each iteration in parallel
results = spawn for item in items {
    process(item)        // each iteration → independent task
}

// spawn while: each round of iteration in parallel
spawn while has_next() {
    step()               // each round of iteration → independent task
}

// spawn if: selected branch as a whole as a task
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
    x = 20              // this is a local x inside the spawn expression
    compute(x)
}
// x is still 10

result = spawn for item in items {
    item = item + 1     // iteration-local item, independent copy for each iteration
    process(item)
}
// outer item is unaffected
```

**Iteration variables** (for's `x`) have independent copies for each round and are automatically
destroyed when the iteration ends.

#### 2.3 Ownership Rules

Once a variable enters a spawn expression, it cannot be used externally (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // ownership of data moves into the spawn expression
}
// data is unavailable here (moved)
```

If you need to share it between multiple tasks, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // shared reference
    process_b(shared)   // shared reference
}
```

**Cross-iteration sharing**: Use `ref` to capture to the outer scope; iterations share the same
reference.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (Block)

1. Wait for all tasks to complete (even if some tasks have failed)
2. Propagate the first encountered error
3. Use `?` to explicitly mark error propagation points

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // may fail
    fetch("url2")?      // may fail
}
// if any task fails, the entire spawn expression propagates the first error
```

##### `spawn for x in items { body? }`

- Wait for all iterations to complete before returning the first error
- After a failed iteration, remaining iterations **continue to execute** (not cancelled)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // any iteration fails → wait for all to complete → propagate first error
}
```

##### `spawn while cond { body? }`

Inherits while's own error semantics:

- step uses `?` to propagate error → the entire spawn while fails, no more rounds
- step does not propagate error (error is swallowed) → enter next round of iteration

```yaoxiang
spawn while has_next() {
    item = next()       // when error is not propagated, failure still enters next round
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c **evaluated sequentially**
- c evaluation error → overall error
- Error inside the selected branch → overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks the usage of resource types to ensure concurrency safety:

| Resource Type | Description         | Compiler Behavior                                   |
| ------------- | ------------------- | --------------------------------------------------- |
| `FilePath`    | Filesystem path     | Same-path operations automatically serialized       |
| `HttpUrl`     | HTTP endpoint       | Same-URL operations automatically serialized        |
| `DBUrl`       | Database connection | Same-connection operations automatically serialized |
| `Console`     | Standard output     | All Console operations automatically serialized     |

##### Inside `spawn { ... }` Block

```yaoxiang
// operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // executes first
    write_file("data.txt", x)   // waits for read to complete
}
```

##### `spawn for ... { ... }` Cross-iteration Same Resource

When all iterations operate on the same resource type, the compiler **automatically downgrades to
serial** (spawn degenerates to sequential for, no error):

```yaoxiang
// all iterations write to the same file path → automatically downgraded to serial
results = spawn for item in items {
    write_file("data.txt", item)
}
// compiler automatically serializes all iterations
```

> **Design rationale**: The spawn keyword still expresses parallel intent; on resource conflict, the
> compiler automatically downgrades, which is more aligned with the principle of least surprise than
> outright rejection.

##### `spawn while ... { ... }` Capturing `&mut`

**Compile-time error**: `spawn while` does not allow capturing `&mut` typed external variables:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // compile-time error
    item = iter.next()              // iter is &mut, shared mutable across iterations = data race
}
```

> **Not reintroducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" promise. Require the
> user to use `ref` or non-spawn writing.

##### `spawn if c { ... } else { ... }` Two Branches Same Resource

**Legal without warning**: The if condition is mutually exclusive; at most one branch executes; no
concurrency conflict:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // branch 1: read cache
} else {
    fetch(key)                      // branch 2: read URL
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

**Nesting semantics**:

- Inner spawn is an independent concurrency domain (independent task queue, independent error
  propagation)
- Inner errors propagate independently to the outer layer (the outer task receives the error when
  waiting for the inner layer to complete)
- Inner resource type rules are tracked independently (not jointly checked with the outer layer)

```yaoxiang
// spawn for nested spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Rupture with the Old Design

| Old Design (RFC-001)                  | New Design (RFC-024 + RFC-032)                            |
| ------------------------------------- | --------------------------------------------------------- |
| Whole-program automatic DAG analysis  | Analysis only within spawn expressions                    |
| `@block`/`@eager`/`@auto` annotations | No annotations, dependency-driven                         |
| `Send`/`Sync` trait                   | Not needed, ownership + ref handled automatically         |
| `future`/non-blocking handles         | Synchronous blocking, no callbacks                        |
| `Mutex`/`RwLock`/`Atomic`             | `ref` automatically chooses Rc/Arc                        |
| L1/L2/L3 three-layer mental model     | Normal code is sequential, spawn expressions are parallel |
| Function coloring (async/await)       | No function coloring                                      |
| `spawn` only modifies `{}` blocks     | `spawn` modifies any expression (see RFC-032)             |

### 4. Return Rules

YaoXiang's return rules are unified and clear:

| Syntax                          | Return Value                                | Description                    |
| ------------------------------- | ------------------------------------------- | ------------------------------ |
| `= expr` (no curly braces)      | Returns `expr` directly                     | Expression is value            |
| `= { ... }` (with curly braces) | Must use `return`, otherwise returns `Void` | Block requires explicit return |

```yaoxiang
// no curly braces: direct return
add: (a: Int, b: Int) -> Int = a + b

// with curly braces: must use return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// with curly braces but no return: returns Void
log: (message: String) -> Void = {
    print(message)  // no return, returns Void
}
```

### 5. User Mental Model

> **Your normal code executes sequentially.**
>
> **When you want multiple things to happen together, put them inside `spawn <expr>`.**
>
> The shape of the expression determines how tasks are decomposed: each direct sub-expression in a
> block is parallel; each iteration of for is parallel; the selected branch of if is one task.
>
> **The entire spawn expression blocks synchronously, waiting for all tasks to complete.**
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// normal code: sequential execution
a = compute_a()         // executes first
b = compute_b(a)        // depends on a, executes after a is done
c = compute_c(b)        // depends on b, executes after b is done

// when parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // parallel
    fetch("url2"),      // parallel
    fetch("url3")       // parallel
}
// wait for all to complete before continuing
process(x, y, z)

// data parallelism: spawn for
results = spawn for item in items {
    process(item)
}
```

---

## Trade-offs

### Advantages

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: Users clearly know where parallelism happens and where it is sequential, no
   implicit concurrency
3. **Safe**: Ownership rules extend naturally, no need for additional type constraints like
   `Send`/`Sync`
4. **Controllable**: No automatic parallel behavior, avoiding unexpected concurrency issues
5. **Synchronous**: Caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No async/await function coloring issues
7. **Compile-time efficient**: DAG analysis is limited to within spawn expressions, compile time is
   controllable
8. **Orthogonality**: spawn naturally composes with any control flow structure (see RFC-032)

### Disadvantages

1. **Requires explicit spawn**: Cannot auto-parallelize, users need to manually mark parallel points
2. **DAG analysis within spawn expressions**: The compiler needs to perform dependency analysis
   within spawn expressions
3. **Incompatible with old code**: Code using the old RFC-001 pattern needs to be migrated

---

## Alternatives

| Alternative                           | Why Not Choose                                                         |
| ------------------------------------- | ---------------------------------------------------------------------- |
| Whole-program automatic DAG (RFC-001) | High complexity, long compile time, uncontrollable behavior            |
| async/await                           | Function coloring, steep learning curve, poor code readability         |
| goroutine                             | No type safety, hard to detect data races                              |
| Actor model                           | Complex message passing, difficult debugging                           |
| CSP (Go channel)                      | No type safety, hard to detect deadlocks                               |
| `spawn` only modifies `{}` blocks     | Breaks orthogonality, `spawn for` becomes a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the expression shape
   after spawn (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependency relationships within spawn expressions
3. **Topological sort**: Determine execution order within spawn expressions
4. **Parallelism identification**: Identify dependency-free sub-trees within spawn expressions
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts of resource types

### Module Organization

spawn-related code is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry
├── placement.rs     # spawn occurrence position legality check
└── analysis.rs      # task identification, dependency analysis, resource conflict detection
```

> **Migration notes** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs`
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

| Decision                       | Determination                                     | Reason                                                     | Date       |
| ------------------------------ | ------------------------------------------------- | ---------------------------------------------------------- | ---------- |
| Parallel primitive             | `spawn <expr>`                                    | Simple, explicit, controllable                             | 2026-06-05 |
| spawn modification range       | Any expression                                    | Syntax orthogonality, eliminate `spawn for` specialization | 2026-07-04 |
| Task decomposition             | Determined by expression shape                    | Expressive, unified rules                                  | 2026-07-04 |
| Execution model                | Synchronous blocking                              | Easy to understand and debug                               | 2026-06-05 |
| DAG analysis range             | Only within spawn expressions                     | Compile-time efficient, controllable behavior              | 2026-06-05 |
| Sharing mechanism              | `ref` automatically selects Rc/Arc                | Simplify user decisions                                    | 2026-06-05 |
| Annotations                    | None                                              | Reduce code noise                                          | 2026-06-05 |
| Send/Sync                      | Remove                                            | Ownership + ref is sufficient                              | 2026-06-05 |
| Mutex/RwLock                   | Remove                                            | ref handled automatically                                  | 2026-06-05 |
| future/handle                  | Remove                                            | Synchronous blocking is simpler                            | 2026-06-05 |
| Function coloring              | None                                              | Avoid async/await issues                                   | 2026-06-05 |
| Resource types                 | Built-in + user-defined                           | Automatic serialization                                    | 2026-06-05 |
| `spawn {}` error               | Wait for all to complete, propagate first error   | Deterministic behavior                                     | 2026-06-05 |
| `spawn for` error              | Wait for all to complete, propagate first error   | Consistent with `spawn {}`                                 | 2026-07-04 |
| `spawn while` error            | Inherits while error semantics                    | Standard while behavior                                    | 2026-07-04 |
| `spawn if` condition error     | c evaluated sequentially, failure → overall error | Intuitive                                                  | 2026-07-04 |
| `spawn for` same resource      | Automatically downgraded to serial                | Safe downgrade, not blunt rejection                        | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error                                | Avoid data races, not introduce Sync                       | 2026-07-04 |
| `spawn if` same resource       | Legal without warning                             | Mutually exclusive branches do not constitute conflict     | 2026-07-04 |
| Nested spawn                   | Inner independent concurrency domain              | Independent task queue, errors, resources                  | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](../../../reference/language-spec/concurrency.md)
- [RFC-001 spawn Model (Deprecated)](../deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generics System](./011-generic-type-system.md)
- [RFC-032 Unified spawn Expression Modifier — AST/IR Refactoring](../review/032-spawn-unified-expression.md)

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
