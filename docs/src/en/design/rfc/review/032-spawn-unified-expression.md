---
title: "RFC-032: Unified spawn Expression Modifier — Eliminating the spawn for Special Case"
status: "Under Review"
author: "Chenxu"
created: "2026-06-16"
updated: "2026-07-03"
issue: "#98"
---

# RFC-032: Unified spawn Expression Modifier

> **This document defines the syntax, AST/IR refactoring, and type system extensions for `spawn`.**
> Runtime behavior semantics (task decomposition granularity, ownership, scope, error propagation, resource types, nesting) are specified in [RFC-024: Concurrency Runtime Semantics Based on spawn](./024-concurrency-model.md).
>
> These two RFCs jointly define `spawn` — RFC-024 answers "what it does", and RFC-032 answers "how it is expressed".

> **Core Insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**. `spawn for` is not special syntax — it is the natural combination of `spawn` + `for` expression.

## Summary

Extend `spawn` from `spawn { }` (modifying blocks only) to `spawn <expr>` (modifying any expression). `Expr::SpawnFor` is removed from the AST, naturally replaced by `Expr::Spawn { body: Expr::For { .. } }`. The types of expression structures (Block, For, While, If, etc.) enter the type system as new `MonoType` variants, `Spawn<T>` wraps the computation structure being executed concurrently, is marked at compile-time, and is erased after type checking.


## Motivation

### Why is this change needed?

Currently, `spawn for x in items { body }` is an independent keyword combination, with a dedicated `Expr::SpawnFor` in the AST to represent it. This breaks the language's orthogonality:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks; `spawn for` is a hardcoded exception
2. **Missing orthogonality**: combinations like `spawn while`, `spawn if` cannot be expressed naturally
3. **Incomplete type system**: spawn is invisible in the type system, and the concurrency structure cannot be obtained through type reflection

### Current Problems

```rust
// Two spawn variants in the AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType only has value types, no computation structure types
// spawn { a, b } type = Tuple(T_a, T_b)  ← loses the "this is spawn" information
// spawn for    type = List(T)             ← loses the "this is data-parallel" information
```

## Proposal

### Core Design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG decomposes tasks.

**Everything is a type**: `MonoType` is extended from "value types" to "value types + computation structure types". Each key expression structure has a corresponding type variant in the type system. `Spawn<T>` wraps the computation structure being executed concurrently.

### User Mental Model

`spawn` = "run this expression concurrently". The shape of the expression determines how it is decomposed:

| Expression Shape | Concurrent Behavior | Type |
|-----------------|---------------------|------|
| `spawn { a, b, c }` | `a`, `b`, `c` in independent parallel | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N iterations in independent parallel | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | Each iteration as an independent task | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | The selected branch as a whole is the spawn domain | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | The call itself as one task | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | A single task | `Spawn(Int)` |

The compiler is responsible for DAG analysis to determine dependencies, and the runtime schedules according to the GMP model — tasks without dependencies are thrown into the work queue, and workers grab and run them. The whole thing synchronously blocks, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget"; YaoXiang's `spawn` is "decompose and run in parallel, then wait for all to finish before continuing".

### Control Flow Orthogonality

| Combination | Semantics | Difference |
|-------------|-----------|------------|
| `spawn for x in items { body }` | Data-parallel: each iteration = independent task | DAG analyzes cross-iteration dependencies |
| `for x in items spawn { body }` | Each iteration creates a spawn domain | No cross-iteration analysis |
| `spawn while cond { body }` | Condition-parallel: each iteration = independent task | Inter-iteration dependencies guaranteed by condition |
| `while cond spawn { body }` | Each iteration creates a spawn domain | Different semantics from above but needs no special handling |
| `spawn if c { a } else { b }` | The entire if-else is one spawn domain | Branch is selected at runtime |
| `if c spawn { a } else { b }` | Only one branch spawns | spawn wrapped inside if expression |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` is removed from the AST
- ❌ `SpawnForAnalysis` is removed from DAG analysis
- ❌ `spawn for` is no longer special-cased as a combined keyword in the Parser
- ❌ `Ir::SpawnFor` is removed from the IR

## Detailed Design

### 1. AST Layer

**Before:**

```rust
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

**After:**

```rust
Spawn { body: Box<Expr>, span: Span },           // spawn <any expression>
```

`Expr::SpawnFor` is removed. The AST representation of `spawn for x in items { body }` becomes:

```rust
Expr::Spawn {
    body: Box::new(Expr::For {
        var: "x",
        iterable: items,
        body: body_block,
        ..
    })
}
```

**IF Special Cases**:

| Syntax | AST Structure |
|--------|---------------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

The two have different semantics but are both natural combinations, requiring no special rules.

### 2. Parser Layer

`spawn` binds with the lowest precedence (same as `return`), consuming the entire following expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser change: in `pratt/nud.rs`, `spawn` no longer requires `{`, but instead invokes the general expression parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is processed by the general expression parser producing `Expr::For`, and `spawn` is only responsible for wrapping.

### 3. Type System

**New `MonoType` variants:**

```rust
// ========== Computation Structure Types ==========

/// {} block expression
Block(Box<MonoType>),

/// for loop expression
ForExpr { body_ty: Box<MonoType> },

/// while loop expression
WhileExpr { body_ty: Box<MonoType> },

/// if-else branch expression
IfExpr {
    then_ty: Box<MonoType>,
    else_ty: Option<Box<MonoType>>,
},

/// function call expression
Call {
    fn_ty: Box<MonoType>,
    result_ty: Box<MonoType>,
},

/// spawn concurrency wrapper: the inner expression is executed concurrently
/// marked at compile-time, erased after type checking
Spawn(Box<MonoType>),
```

**Type inference rules**: each expression's type inference returns a "computation structure type". Without `Spawn` wrapper = sequential execution, with `Spawn` wrapper = concurrent execution. After type checking, `Spawn` is erased, and the type degrades to the inner value type.

**Type checking flow**:
1. Infer the type T (computation structure type) of the body expression
2. If wrapped by spawn, wrap as `Spawn(T)`
3. Destructure on assignment inference: `results: List(Data) = spawn for ... {}` — extract `List(Data)` from `Spawn(ForExpr { body_ty: List(Data) })`

`Spawn<T>` is erased after type checking, and the runtime does not need to know whether data comes from concurrent or sequential execution. But compile-time reflection (`type_of(x)`) can obtain the complete concurrency topology.

### 4. DAG Analysis Layer

The current two entry points are merged into one:

```rust
/// Unified entry: dispatch based on the body expression kind
fn analyze_spawn_expr(body: &Expr, ...) -> SpawnAnalysis {
    match body {
        Expr::Block(block)       => analyze_block_tasks(block, ...),
        Expr::For { .. }         => analyze_iter_tasks(IterKind::For, body, ...),
        Expr::While { .. }       => analyze_iter_tasks(IterKind::While, body, ...),
        Expr::If { .. }          => analyze_if_task(body, ...),
        _                        => single_task(body, ...),
    }
}
```

**Unified result structure:**

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — N direct subexpressions known at compile-time
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N tasks generated by runtime iteration
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // present for for, absent for while
        condition: Option<Expr>,     // present for while, absent for for
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

The `SpawnForAnalysis` struct is removed.

| body kind | How it is decomposed into tasks |
|-----------|--------------------------------|
| `Expr::Block` | Direct subexpressions → task list |
| `Expr::For` | Each iteration → one task (data-parallel) |
| `Expr::While` | Each iteration → one task |
| `Expr::If` | The selected branch as a whole → one task |
| `Expr::Call` / others | The expression itself → one task |

After DAG analysis is complete, the runtime schedules according to the GMP model — tasks without dependencies are thrown into the work queue, and workers grab and run them.

### 5. IR / Codegen Layer

`Ir::SpawnFor` is removed. Unified into `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:
- `TaskSource::Explicit(tasks)` → task list known at compile-time
- `TaskSource::Iterate { .. }` → expanded at runtime (compiler-driven, similar to par_iter but zero-cost)

### 6. Placement Layer

The current two branches are merged into one:

```rust
// Before
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// After
Expr::Spawn { body, .. } => self.check_expr(body),   // body is Expr, recursion suffices
```

### 7. Backward Compatibility

The semantics of existing `spawn for` code remain unchanged; the Parser automatically parses `spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. The internal representation changes, but the user-visible behavior does not.

New syntax is naturally obtained:
```yx
spawn while has_next() {
    item = next()
    process(item)
}

spawn if use_cache {
    load_from_cache(key)
} else {
    fetch(key)
}
```

## Trade-offs

### Advantages

1. **Syntactic orthogonality**: `spawn` + any control flow = natural concurrency combination
2. **Everything is a type**: the type system completely records computation structure, and compile-time reflection obtains the concurrency topology
3. **Eliminates special cases**: removes `Expr::SpawnFor` and related special-handling code
4. **Extensibility**: future new control flow structures automatically combine with `spawn`, no need to modify spawn logic

### Disadvantages

1. **Type system bloat**: 6 new `MonoType` variants, increasing type checking complexity
2. **Breaking change**: internal AST/IR representation changes, requiring updates to all code consuming `Expr::SpawnFor`
3. **Expression type inference**: each expression now needs to return a computation structure type, large impact surface

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| Keep `spawn for` as independent syntax | Breaks orthogonality, becomes the only keyword combination special case in the language |
| `spawn` only modifies `{}`, data-parallel through standard library `par_iter` | Language's primitive capability is delegated to libraries, losing compiler-level DAG analysis and resource conflict detection |
| Only remove `SpawnFor` without introducing computation structure types to the type system | Type system loses reflection capability, spawn is invisible at the type level |


## Relationship with RFC-019

The 6 `MonoType` variants introduced by this RFC (Block/ForExpr/WhileExpr/IfExpr/Call/Spawn) are the **compiler built-in subset** of [RFC-019: Type-Level Homoiconicity](./019-typed-homoiconicity.md). RFC-019's core idea "syntax structure enters the type system" is realized here as: the 6 computation structures natively understood by the compiler have corresponding type representations. Users cannot define new computation structure types through `SyntaxRule`, but the 6 built into the compiler cover all key control flows.

## Proof Pipeline Integration

The reason the 6 `MonoType` variants exist: they tell [RFC-027 Compile-time Proof Pipeline](../accepted/027-compile-time-evaluation-types.md) **what shape the proposition to verify is**. The pipeline itself is responsible for the actual proof work (free variable analysis, effect classification, alias analysis, conflict detection), and MonoType only does one thing — providing a structured input interface.

### Variant → Proposition Mapping

| Type | Proposition Shape | Proof Strategy |
|------|-------------------|----------------|
| `Spawn(ForExpr { body_ty })` | Data-parallel: N iteration tasks with no cross-iteration conflicts | Extract body free variables → effect classification → check no Write(Shared) / `&mut`(Shared) |
| `Spawn(WhileExpr { body_ty })` | Condition-parallel: each iteration independent + no cross-iteration causal dependencies | Same as above + check whether iteration condition has cross-iteration side effects |
| `Spawn(Block(T))` | Explicit task group: task-to-task dependencies given by DAG | Verify the dependency graph from DAG analysis — each task's required inputs are ready at its start |
| `Spawn(IfExpr { then_ty, else_ty })` | Branch spawn: selected branch as a whole is one spawn domain | Branch selection has no conflicts, recursive check inside body |
| `Spawn(Call { fn_ty, result_ty })` | Call spawn: called function as one independent task | Verify the function's purity or isolation |
| `Spawn(T)` (value, e.g. `spawn 42`) | Single-value spawn: no concurrency | Trivially passes |

### Proof Scenarios

**Scenario 1 — Pure data-parallel (pass):**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// Type: Spawn(ForExpr { body_ty: List(Int) })
```

1. Free variables: `item` (loop-local, independent copy per iteration), `items` (external, read-only in body)
2. Effect classification: all Read(Local) or Read(Shared), no writes
3. Proved ✓

**Scenario 2 — Read-only sharing (pass):**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// Type: Spawn(ForExpr { body_ty: List(Result) })
```

1. Free variables: `item` (Read(Local)), `config` (external, no write path in body → Read(Shared))
2. Effect classification: all read-only
3. Proved ✓

**Scenario 3 — Write conflict (rejected):**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. Free variables: `item` (Read(Local)), `counter` (external, `+=` desugars to write)
2. Effect classification: `counter` is Write(Shared), cross-iteration writes to the same memory
3. Instantiated conflict: `Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. Disproved ✗ → compile error: `Error: cross-iteration write conflict in spawn for body. Variable counter is written by multiple concurrent tasks.`

**Scenario 4 — while + stateful iterator (warning/reject):**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// Type: Spawn(WhileExpr { body_ty: List(Processed) })
```

1. Free variables: `iter` (external, `next()` → `&mut self` → `&mut`(Shared))
2. `next()` modifies the iterator's state, iteration N+1 depends on iteration N's side effect
3. This is not independent tasks → violates the independence constraint of `Spawn(WhileExpr)`
4. Compiler reports cross-iteration causal dependency, suggests using `spawn for` instead

**Scenario 5 — spawn if (pass):**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// Type: Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. Only one branch executes, no cross-task conflicts
2. If there is a sub-spawn in the body, recursive check
3. Proved ✓

**Scenario 6 — spawn block with inter-task dependencies (DAG + pipeline validation):**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // depends on a
    c = compute_stats()           // independent
}
// Type: Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG analysis: `a` and `c` are independent (can run in parallel), `b` depends on `a` (scheduled after a)
2. Pipeline validation: `b`'s input (`a.user_id`) is already computed when b starts
3. Proved ✓

### What MonoType Does Not Do

| What it does | What it does not do |
|--------------|---------------------|
| Identifies the proposition shape | Does not perform proofs |
| Records computation structure at the type level | Does not replace DAG analysis |
| Provides type input to the RFC-027 pipeline | Does not replace free variable analysis, alias analysis, conflict detection |

The actual proof work is performed by the compiler's standard analysis passes. MonoType's value is allowing these passes to be scheduled within a unified type framework — the proof pipeline does not need to write special branches for each AST node.
## Implementation Strategy

### Phased Division

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **Type system**: add 6 `MonoType` variants, all expression type inference returns computation structure types
3. **DAG analysis unification**: merge entry points, unify `TaskSource` enum
4. **IR / Codegen adaptation**: remove `Ir::SpawnFor`, unify handling path
5. **Placement simplification**: remove `SpawnFor` branch
6. **Test verification**: all existing `spawn for` tests pass

### Impact Scope

| File/Directory | Change |
|----------------|--------|
| `frontend/core/parser/ast.rs` | `Spawn` body changed to `Box<Expr>`, remove `SpawnFor` |
| `frontend/core/parser/pratt/nud.rs` | `spawn` handler simplified to general expression parsing |
| `frontend/core/types/mono.rs` | add `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` variants |
| `frontend/core/spawn/analysis.rs` | unified entry, `TaskSource` merges Explicit + Iterate |
| `frontend/core/spawn/placement.rs` | remove `SpawnFor` branch |
| `frontend/core/typecheck/` | all expression nodes adapt to computation structure type inference |
| `middle/core/ir.rs` | remove `Ir::SpawnFor` |
| `middle/` (IR gen, codegen) | unify spawn path, Spawn type erasure |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | semantics unchanged, verification passes |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — basis for type system changes
- RFC-027 (compile-time proof pipeline) — MonoType variants provide proposition shape input to the pipeline
- RFC-019 (type-level homoiconicity) — MonoType variants are its compiler built-in subset

## Design Decision Record

| Decision | Determination | Reason | Date |
|----------|---------------|--------|------|
| spawn modification scope | Any expression | Eliminate `spawn for` special case | 2026-06-16 |
| `spawn while` support | Supported | Syntactic orthogonality, low implementation cost | 2026-06-16 |
| `spawn if` semantics | Modifies the entire if-else | Distinguished from `if spawn { }` | 2026-06-16 |
| Type system | Introduce computation structure types | "Everything is a type", supports compile-time reflection | 2026-06-16 |
| spawn type erasure | Erase after type checking | Runtime does not need concurrency structure information | 2026-06-16 |
| spawn binding precedence | Lowest (same as return) | Consumes the entire following expression | 2026-06-16 |
| DAG for internals of for | Do not expand for internal subexpressions | Direct subexpression rules unchanged, for as a whole is one task source | 2026-06-16 |
| Proof pipeline integration | MonoType variants map to RFC-027 proof propositions | Pipeline needs to know the proposition shape to verify, MonoType provides structured input | 2026-07-03 |
| Relationship with RFC-019 | Compiler built-in subset | Users cannot customize, but share the "syntax is type" idea | 2026-07-03 |
| Proof boundary | 6 scenarios cover: pure-parallel/read-only sharing/write conflict/while dependency/spawn if/spawn block | Clarify each MonoType variant's proof obligation and failure condition | 2026-07-03 |

---

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](./024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: Type-Level Homoiconicity](./019-typed-homoiconicity.md)
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
- [spawn for Orthogonality Suspension (Discussion Draft)](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |