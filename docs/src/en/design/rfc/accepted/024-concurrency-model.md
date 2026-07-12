---
title: "RFC-024: Spawn-Based Concurrent Runtime Semantics"
status: "Accepted (Revised)"
author: "晨煦 (Chenxu)"
created: "2026-06-05"
updated: "2026-07-05 (RFC sync check: implementation progress ~85%, core runtime and frontend analysis complete)"

issue: "#89"
---

# RFC-024: Spawn-Based Concurrent Runtime Semantics

> **This document defines the runtime behavior semantics of `spawn`.**
> Syntax orthogonality, AST/IR refactoring, and type system extensions are covered in [RFC-032](./032-spawn-unified-expression.md).
>
> The two RFCs together define `spawn` — 024 answers "what it does", 032 answers "how it is represented".

> **References**:
> - [Concurrency Model Specification](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](./008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](./009-ownership-model.md)
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
> - [RFC-032: spawn Unified Expression Modifier — AST/IR Refactoring](./032-spawn-unified-expression.md)

## Abstract

This document defines the **runtime behavior semantics** of the `spawn` construct in the YaoXiang programming language: `spawn <expr>` is the sole parallel primitive, can modify any expression, and synchronously blocks the caller. The shape of the expression determines task decomposition granularity, and the runtime schedules according to the GMP model — tasks with no dependencies are thrown into the work queue, and workers race to execute them.

**Core design — one primitive, one set of rules**:

```
spawn <expr>               ← the sole parallel primitive
Task decomposition determined by expression shape    ← the sole rule
Synchronous blocking wait for results  ← the sole behavior
```

**Complexity eliminated**:
- ❌ No `@block` / `@eager` / `@auto` annotations
- ❌ No `Send` / `Sync` traits
- ❌ No `Mutex` / `RwLock` / `Atomic`
- ❌ No `future` / non-blocking handles
- ❌ No whole-program DAG analysis
- ❌ No function coloring (async/await)

> **User mental model**: The ordinary code you write executes sequentially. When you want multiple things to happen together, put them inside `spawn <expr>`. No callbacks, no `await`, no strange annotations.

## Design Provenance

| Document | Relationship |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | Superseded by this document |
| [RFC-008](./008-runtime-concurrency-model.md) | Runtime architecture, orthogonal to this document |
| [RFC-009](./009-ownership-model.md) | Ownership model, unchanged |
| [RFC-010](./010-unified-type-syntax.md) | Unified type syntax |
| [RFC-032](./032-spawn-unified-expression.md) | AST/IR refactoring, co-defines `spawn` with this document |

## Motivation

### Why is this design needed?

Current mainstream language concurrency models have obvious defects:

| Language | Concurrency Model | Problems |
|------|----------|------|
| Rust | async/await + tokio | Async contagion, function coloring, steep learning curve |
| Go | goroutine | No type safety, data races hard to detect |
| Python | asyncio | GIL limitation, function coloring |
| JavaScript | Promise/async | Callback hell, function coloring |

### Problems with the Old Design (RFC-001)

The three-tier concurrency architecture (L1/L2/L3) proposed in RFC-001 has the following problems:

| Problem | Description |
|------|------|
| Complex mental model | L1/L2/L3 three-tier abstraction increases learning burden |
| Annotation redundancy | `@block` / `@eager` / `@auto` annotations make code noisy |
| High analysis complexity | Whole-program DAG analysis incurs significant compile-time overhead |
| Complex type constraints | `Send` / `Sync` traits add cognitive load |
| Uncontrollable | Automatic concurrency behavior is hard to predict and debug |

### Design Goals

1. **Simple**: Only one parallel primitive (`spawn`), can modify any expression
2. **Explicit**: The user clearly knows where parallel execution happens and where it is sequential
3. **Safe**: Ownership rules extend naturally, no extra type constraints required
4. **Controllable**: No implicit concurrency, no unexpected parallel behavior
5. **Synchronous**: The caller blocks synchronously, no callbacks or `await`

---

## Proposal

### 1. The Essence of `{}` Blocks: Dependency-Driven Computation Units

In YaoXiang, `{}` is a **dependency-driven computation unit**.

| Property | Description |
|------|------|
| Dependency-driven | The block checks at execution time whether all internal variables are ready; if they are, it executes immediately, otherwise it blocks waiting |
| Execution timing | Determined by dependencies, unrelated to "immediate" or "deferred" |
| Return value | Use `return` to explicitly return a value; without `return`, defaults to `Void` |
| Unified syntax | The semantics are consistent whether it appears in a function body, variable initialization, or after `spawn` |
| Scope isolation | Variables are strictly confined to the inside of `{}`, not leaking to the outer scope |

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

`spawn <expr>` is the **sole parallel primitive** in YaoXiang. It can modify any expression, and the shape of the expression determines task decomposition granularity.

#### 2.1 Task Creation Rules

| Expression Shape | Task Decomposition | Synchronous Semantics |
|-----------|---------|---------|
| `spawn { a, b, c }` | Direct sub-expressions → N independent tasks | Wait for all tasks to complete |
| `spawn for x in items { body }` | Each iteration → 1 task | Wait for all iterations to complete |
| `spawn while cond { body }` | Each iteration round → 1 task (condition-driven between iterations) | Wait until condition is false |
| `spawn if c { a } else { b }` | Condition c evaluated sequentially, selected branch as a whole → 1 task | Wait for selected branch to complete |
| `spawn call(x)` | The call itself → 1 task | Wait for call to complete |
| `spawn expr` (any expression) | The expression itself → 1 task | Wait for expression to complete |

> **Design rationale**: Why can `spawn` modify any expression? See [RFC-032 §Core Design](./032-spawn-unified-expression.md).
>
> **Control flow orthogonality**: The semantic differences between `spawn <expr>` (spawn first) and `<expr> spawn { body }` (spawn last) are detailed in [RFC-032 §Control Flow Orthogonality](./032-spawn-unified-expression.md) (core definition). The runtime behavior of all reversed-writing combinations (`for ... spawn { }` / `while ... spawn { }` / `if ... spawn { }`) — error propagation, resource types, nesting rules — inherits the rules in §2.4 / §2.5 / §2.6 of this document.

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

// spawn while: each iteration round in parallel
spawn while has_next() {
    step()               // each iteration round → independent task
}

// spawn if: selected branch as a whole as a task
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
    x = 20              // this is the local x inside the spawn expression
    compute(x)
}
// x is still 10

result = spawn for item in items {
    item = item + 1     // iteration-local item, independent copy per iteration
    process(item)
}
// outer item is unaffected
```

**Iteration variables** (the `x` in `for`) get an independent copy per round, automatically destroyed when the iteration ends.

#### 2.3 Ownership Rules

Once a variable enters a `spawn` expression, it can no longer be used externally (Move semantics):

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // ownership of data is moved into the spawn expression
}
// data is unusable here (moved)
```

If sharing is needed across multiple tasks, use `ref`:

```yaoxiang
data = load_data()
shared = ref data       // compiler automatically chooses Rc or Arc

result = spawn {
    process_a(shared),  // shared reference
    process_b(shared)   // shared reference
}
```

**Cross-iteration sharing**: Use `ref` to capture to the outer scope, sharing the same reference across iterations.

#### 2.4 Error Propagation Rules

##### `spawn { a, b, c }` (block)

1. Wait for all tasks to complete (even if some have already failed)
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

- Wait for all iterations to complete before returning the first error
- Remaining iterations **continue executing** after a failed iteration (no cancellation)
- Use `?` to explicitly mark error propagation points

```yaoxiang
results = spawn for item in items {
    process(item)?      // any iteration failure → wait for all to complete → propagate first error
}
```

##### `spawn while cond { body? }`

Inherits `while`'s own error semantics:

- Step uses `?` to propagate error → entire `spawn while` fails, no next round
- Step does not propagate error (error swallowed) → enter next iteration round

```yaoxiang
spawn while has_next() {
    item = next()       // when error is not propagated, failure still enters next round
    process(item)
}
```

##### `spawn if c { a } else { b }`

- Condition c is evaluated **sequentially**
- c evaluation error → overall error
- Error inside selected branch → overall error

```yaoxiang
result = spawn if cond()? {  // cond evaluated sequentially, failure → overall error
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 Resource Type Rules

The compiler tracks resource type usage to ensure concurrency safety:

| Resource Type | Description | Compiler Behavior |
|----------|------|-----------|
| `FilePath` | File system path | Operations on the same path automatically serialized |
| `HttpUrl` | HTTP endpoint | Operations on the same URL automatically serialized |
| `DBUrl` | Database connection | Operations on the same connection automatically serialized |
| `Console` | Standard output | All Console operations automatically serialized |

##### Inside `spawn { ... }` block

```yaoxiang
// Operations on the same file are automatically serialized
(a, b) = spawn {
    read_file("data.txt"),      // executes first
    write_file("data.txt", x)   // waits for read to complete
}
```

##### `spawn for ... { ... }` cross-iteration same resource

When all iterations operate on the same resource type, the compiler **automatically downgrades to sequential** (spawn degrades to a sequential `for`, no error reported):

```yaoxiang
// All iterations writing to the same file path → automatically downgrade to sequential
results = spawn for item in items {
    write_file("data.txt", item)
}
// compiler automatically serializes all iterations
```

> **Design rationale**: The `spawn` keyword still expresses parallel intent; the compiler automatically downgrades on resource conflicts, which aligns with the principle of least surprise rather than outright rejection.

##### `spawn while ... { ... }` capturing `&mut`

**Compile-time error**: `spawn while` is not allowed to capture external variables of type `&mut`:

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // compile-time error
    item = iter.next()              // iter is &mut, shared mutable across iterations = data race
}
```

> **Not re-introducing `Sync` trait**: Consistent with RFC-024's "no Send/Sync" commitment. Users are required to switch to `ref` or a non-spawn form.

##### `spawn if c { ... } else { ... }` with both branches on the same resource

**Legal without warning**: `if` conditions are mutually exclusive; at most one branch executes, so no concurrency conflict exists:

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // branch 1: read cache
} else {
    fetch(key)                      // branch 2: read URL
}
```

#### 2.6 Nested spawn

`spawn` expressions can be nested; the inner layer creates an **independent concurrency domain**:

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
- The inner `spawn` is an independent concurrency domain (independent task queue, independent error propagation)
- Inner errors propagate independently to the outer layer (outer tasks receive errors when waiting for the inner to complete)
- Inner resource type rules are tracked independently (not jointly checked with the outer layer)

```yaoxiang
// spawn for nested with spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. Break with the Old Design

| Old Design (RFC-001) | New Design (RFC-024 + RFC-032) |
|------------------|---------------------------|
| Whole-program automatic DAG analysis | Analysis only inside `spawn` expressions |
| `@block` / `@eager` / `@auto` annotations | No annotations, dependency-driven |
| `Send` / `Sync` traits | Not needed; ownership + `ref` handle it automatically |
| `future` / non-blocking handles | Synchronous blocking, no callbacks |
| `Mutex` / `RwLock` / `Atomic` | `ref` automatically chooses Rc/Arc |
| L1/L2/L3 three-tier mental model | Ordinary code is sequential, `spawn` expressions are parallel |
| Function coloring (async/await) | No function coloring |
| `spawn` only modifies `{}` blocks | `spawn` modifies any expression (see RFC-032) |

### 4. Return Rules

YaoXiang's return rules are unified and explicit:

| Form | Return Value | Description |
|------|--------|------|
| `= expr` (no braces) | Directly returns `expr` | The expression is the value |
| `= { ... }` (with braces) | Must use `return`, otherwise returns `Void` | Blocks require explicit return |

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

> **The ordinary code you write executes sequentially.**
>
> **When you want multiple things to happen together, put them inside `spawn <expr>`.**
>
> The shape of the expression determines how tasks are decomposed: each direct sub-expression in a block runs in parallel; each iteration of `for` runs in parallel; the selected branch of `if` is one task.
>
> **The entire `spawn` expression blocks synchronously, waiting for all tasks to complete.**
>
> **No callbacks, no `await`, no strange annotations.**

```yaoxiang
// Ordinary code: sequential execution
a = compute_a()         // executes first
b = compute_b(a)        // depends on a, executes after a completes
c = compute_c(b)        // depends on b, executes after b completes

// When parallelism is needed: use spawn
(x, y, z) = spawn {
    fetch("url1"),      // parallel
    fetch("url2"),      // parallel
    fetch("url3")       // parallel
}
// wait for all to complete before continuing
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
2. **Explicit**: The user clearly knows where parallel execution happens and where it is sequential; no implicit concurrency
3. **Safe**: Ownership rules extend naturally; no extra type constraints like `Send` / `Sync` needed
4. **Controllable**: No automatic parallel behavior, avoiding unexpected concurrency issues
5. **Synchronous**: The caller blocks synchronously, code is easy to understand and debug
6. **No function coloring**: No function coloring problem like async/await
7. **Efficient compilation**: DAG analysis is confined inside `spawn` expressions, compile time is controllable
8. **Orthogonality**: `spawn` composes naturally with any control flow structure (see RFC-032)

### Disadvantages

1. **Requires explicit `spawn`**: Cannot parallelize automatically; users must manually mark parallel points
2. **DAG analysis inside `spawn` expressions**: The compiler must perform dependency analysis inside `spawn` expressions
3. **Backward incompatibility**: Code using the old RFC-001 pattern needs migration

---

## Alternatives

| Alternative | Why Not Chosen |
|------|--------------|
| Whole-program automatic DAG (RFC-001) | High complexity, long compile times, uncontrollable behavior |
| async/await | Function coloring, steep learning curve, poor code readability |
| goroutine | No type safety, data races hard to detect |
| Actor model | Complex message passing, difficult debugging |
| CSP (Go channel) | No type safety, deadlocks hard to detect |
| `spawn` only modifies `{}` blocks | Breaks orthogonality, makes `spawn for` a special case (see RFC-032) |

---

## Implementation Strategy

### Compile-Time Analysis

1. **Expression shape recognition**: Determine task decomposition based on the expression shape after `spawn` (see RFC-032 §DAG Analysis)
2. **DAG construction**: Analyze dependency relationships inside `spawn` expressions
3. **Topological sorting**: Determine execution order inside `spawn` expressions
4. **Parallelism identification**: Identify subtrees without dependencies inside `spawn` expressions
5. **Escape analysis**: `ref` → Rc or Arc
6. **Resource conflict detection**: Detect potential conflicts on resource types

### Module Organization

Code related to `spawn` is uniformly placed in `frontend/core/spawn/`:

```
frontend/core/spawn/
├── mod.rs           # spawn module entry point
├── placement.rs     # spawn occurrence position validity check
└── analysis.rs      # task identification, dependency analysis, resource conflict detection
```

> **Migration note** (2026-06-11): The existing `frontend/core/typecheck/passes/spawn_placement.rs` will be migrated to `frontend/core/spawn/placement.rs`. The `spawn_placement` module declaration under the `typecheck/passes/` directory must be removed in sync.

### Runtime Execution

Referencing the Runtime architecture from [RFC-008](./008-runtime-concurrency-model.md):

- **Embedded Runtime**: No `spawn` support, immediate execution
- **Standard Runtime**: Supports `spawn` expressions
- **Full Runtime**: Standard + WorkStealer load balancing

### Dependencies

- RFC-008 (Runtime architecture) → Completed
- RFC-009 (Ownership model) → Completed
- RFC-010 (Unified type syntax) → Completed
- RFC-011 (Generics system) → Completed
- RFC-032 (AST/IR refactoring) → Co-defines `spawn` with this document

---

## Design Decision Record

| Decision | Resolution | Reason | Date |
|------|------|------|------|
| Parallel primitive | `spawn <expr>` | Simple, explicit, controllable | 2026-06-05 |
| `spawn` modification scope | Any expression | Syntax orthogonality, eliminates `spawn for` specialization | 2026-07-04 |
| Task decomposition | Determined by expression shape | Expressive, unified rules | 2026-07-04 |
| Execution model | Synchronous blocking | Easy to understand and debug | 2026-06-05 |
| DAG analysis scope | Only inside `spawn` expressions | Efficient compilation, controllable behavior | 2026-06-05 |
| Sharing mechanism | `ref` automatically chooses Rc/Arc | Simplifies user decisions | 2026-06-05 |
| Annotations | None | Reduces code noise | 2026-06-05 |
| Send/Sync | Removed | Ownership + `ref` is sufficient | 2026-06-05 |
| Mutex/RwLock | Removed | `ref` handles automatically | 2026-06-05 |
| future/handle | Removed | Synchronous blocking is simpler | 2026-06-05 |
| Function coloring | None | Avoids async/await problems | 2026-06-05 |
| Resource types | Built-in + user-defined | Automatic serialization | 2026-06-05 |
| `spawn {}` error | Wait for all to complete, propagate first error | Deterministic behavior | 2026-06-05 |
| `spawn for` error | Wait for all to complete, propagate first error | Consistent with `spawn {}` | 2026-07-04 |
| `spawn while` error | Inherits `while` error semantics | Standard `while` behavior | 2026-07-04 |
| `spawn if` condition error | c evaluated sequentially, failure → overall error | Intuitive | 2026-07-04 |
| `spawn for` same resource | Automatically downgrade to sequential | Safe degradation, not blunt rejection | 2026-07-04 |
| `spawn while` capturing `&mut` | Compile-time error | Avoids data races, no re-introduction of Sync | 2026-07-04 |
| `spawn if` same resource | Legal without warning | Mutually exclusive branches don't constitute a conflict | 2026-07-04 |
| Nested `spawn` | Inner independent concurrency domain | Independent task queue, errors, resources | 2026-07-04 |

---

## References

### YaoXiang Official Documentation

- [Concurrency Model Specification](/reference/language-spec/concurrency.md)
- [RFC-001 并发模型（Deprecated）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime Concurrency Model](./008-runtime-concurrency-model.md)
- [RFC-009 Ownership Model](./009-ownership-model.md)
- [RFC-010 Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-011 Generics System](./011-generic-type-system.md)
- [RFC-032 spawn Unified Expression Modifier — AST/IR Refactoring](./032-spawn-unified-expression.md)

### External References

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Accepted (Revised)** | `docs/design/rfc/accepted/` | Co-defines `spawn` with RFC-032 (runtime semantics) |