---
title: 'RFC-032: Unified spawn Expression Modifier — Eliminating spawn for Special Case'
status: 'Under Review'
author: '晨煦'
created: '2026-06-16'
updated: '2026-07-03'
issue: '#98'
---

# RFC-032: Unified spawn Expression Modifier

> **This document defines the syntax, AST/IR restructuring, and type system extension for `spawn`**.
> Runtime behavioral semantics (task decomposition granularity, ownership, scope, error propagation,
> resource types, nesting) are covered in
> [RFC-024: Spawn-based Concurrency Runtime Semantics](./024-concurrency-model.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **Core Insight**: `spawn` should not only decorate `{}` blocks. It can decorate **any
> expression**. `spawn for` is not special syntax — it is the natural combination of `spawn` + `for`
> expression.

## Summary

Extend `spawn` from `spawn { }` (block-only) to `spawn <expr>` (any expression). `Expr::SpawnFor` is
removed from the AST, naturally replaced by `Expr::Spawn { body: Expr::For { .. } }`. Expression
structure types (Block, For, While, If, etc.) enter the type system as new `MonoType` variants.
`Spawn<T>` wraps concurrently executed computational structures, compile-time marker, erased after
type checking.

## Motivation

### Why is this change needed?

Currently `spawn for x in items { body }` is an independent keyword combination with
`Expr::SpawnFor` in the AST specifically representing it. This breaks language orthogonality:

1. **Non-uniform syntax**: `spawn` can only decorate `{}` blocks; `spawn for` is a hardcoded
   exception
2. **Missing orthogonality**: Combinations like `spawn while`, `spawn if` cannot be naturally
   expressed
3. **Incomplete type system**: spawn is invisible in the type system; concurrent structures cannot
   be obtained via type reflection

### Current Problems

```rust
// Two spawn variants in AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType only has value types, no computational structure types
// spawn { a, b } type = Tuple(T_a, T_b)  ← loses "this is spawn" info
// spawn for    type = List(T)             ← loses "this is data parallelism" info
```

## Proposal

### Core Design

`spawn <expr>`: `spawn` decorates any expression. The shape of the expression determines how the DAG
decomposes tasks.

**Everything is a type**: `MonoType` expands from "value types" to "value types + computational
structure types". Each key expression structure has a corresponding type variant in the type system.
`Spawn<T>` wraps the computational structure being concurrently executed.

### User Mental Model

`spawn` = "take this expression and do it concurrently". The shape of the expression determines how
to split:

| Expression Shape                | Concurrency Behavior                  | Type                                                 |
| ------------------------------- | ------------------------------------- | ---------------------------------------------------- |
| `spawn { a, b, c }`             | `a`, `b`, `c` run independently       | `Spawn(Block(Tuple(T_a, T_b, T_c)))`                 |
| `spawn for x in items { f(x) }` | N iterations run independently        | `Spawn(ForExpr { body_ty: List(T) })`                |
| `spawn while cond { step() }`   | Each iteration is an independent task | `Spawn(WhileExpr { body_ty: List(T) })`              |
| `spawn if c { a } else { b }`   | Selected branch is entire spawn scope | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)`                 | The call itself is one task           | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })`       |
| `spawn 42`                      | Single task                           | `Spawn(Int)`                                         |

The compiler handles DAG analysis to determine dependencies, runtime schedules according to GMP
model — tasks without dependencies go into work queue, workers compete to run them. Overall blocks
synchronously until all tasks complete.

**Difference from Go**: Go's `go` is "throw it out and don't care". YaoXiang's `spawn` is "split it
up for parallel execution, wait for everything to finish before continuing".

### Control Flow Orthogonality

| Combination                     | Semantics                                      | Difference                                      |
| ------------------------------- | ---------------------------------------------- | ----------------------------------------------- |
| `spawn for x in items { body }` | Data parallelism: each iteration = task        | DAG cross-iteration dependency analysis         |
| `for x in items spawn { body }` | Each iteration creates a spawn scope           | No cross-iteration analysis                     |
| `spawn while cond { body }`     | Conditional parallelism: each iteration = task | Iteration dependencies guaranteed by condition  |
| `while cond spawn { body }`     | Each iteration creates a spawn scope           | Different semantics, no special handling needed |
| `spawn if c { a } else { b }`   | Entire if-else is one spawn scope              | Branch selected at execution time               |
| `if c spawn { a } else { b }`   | Only single branch spawns                      | spawn inside if expression                      |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` removed from AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer special combination keyword handling in Parser
- ❌ `Ir::SpawnFor` removed from IR

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

`Expr::SpawnFor` deleted. `spawn for x in items { body }` AST representation:

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

**IF Special Case:**

| Syntax                           | AST Structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

Both have different semantics but are natural combinations, requiring no special rules.

### 2. Parser Layer

`s` binds with lowest precedence (same as `return`), consuming the entire following expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser change: In `pratt/nud.rs`, `spawn` no longer requires `{`, instead calls generic expression
parsing:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`s` no longer handles `spawn for` as a combined keyword — `for` is processed by the generic
expression parser to produce `Expr::For`, and `spawn` only handles wrapping.

### 3. Type System

**New `MonoType` variants:**

```rust
// ========== Computational Structure Types ==========

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

/// spawn concurrency wrapper: inner expression is concurrently executed
/// Compile-time marker, erased after type checking
Spawn(Box<MonoType>),
```

**Type Inference Rules**: Each expression's type inference returns "computational structure type".
Without `Spawn` wrapper = sequential execution, with `Spawn` wrapper = concurrent execution. After
type checking completes, `Spawn` is erased and type downgrades to inner value type.

**Type Checking Flow**:

1. Infer body expression's type T (computational structure type)
2. If wrapped in spawn, wrap as `Spawn(T)`
3. During assignment inference: `results: List(Data) = spawn for ... {}` — extract `List(Data)` from
   `Spawn(ForExpr { body_ty: List(Data) })`

`Spawn<T>` is erased after type checking; runtime doesn't need to know if data came from concurrent
or sequential execution. But compile-time reflection (`type_of(x)`) can obtain the complete
concurrent topology.

### 4. DAG Analysis Layer

Two current entry points merged into one:

```rust
/// Unified entry: dispatch based on body expression kind
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

**Unified Result Structure:**

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — N direct sub-expressions known at compile time
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N tasks generated by runtime iteration
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for has this, while doesn't
        condition: Option<Expr>,     // while has this, for doesn't
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` struct deleted.

| Body Kind            | How Decomposed into Tasks                    |
| -------------------- | -------------------------------------------- |
| `Expr::Block`        | Direct sub-expressions → task list           |
| `Expr::For`          | Each iteration → one task (data parallelism) |
| `Expr::While`        | Each iteration → one task                    |
| `Expr::If`           | Selected branch → one task                   |
| `Expr::Call` / other | Expression itself → one task                 |

After DAG analysis completes, runtime schedules according to GMP model — tasks without dependencies
go into work queue, workers compete to run them.

### 5. IR / Codegen Layer

`Ir::SpawnFor` deleted. Unified to `Ir::Spawn` with `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → compile-time known task list
- `TaskSource::Iterate { .. }` → runtime expansion (compiler-driven, similar to par_iter but
  zero-cost)

### 6. Placement Layer

Two current branches merged into one:

```rust
// Before
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// After
Expr::Spawn { body, .. } => self.check_expr(body),   // body is Expr, recurse
```

### 7. Backward Compatibility

Existing `spawn for` code has unchanged semantics; Parser automatically parses
`spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. Internal representation
changed, user-visible behavior unchanged.

New syntax naturally available:

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

## Tradeoffs

### Advantages

1. **Syntax orthogonality**: `spawn` + any control flow = natural concurrency combination
2. **Everything is a type**: Type system completely records computational structures; compile-time
   reflection obtains concurrency topology
3. **Eliminate special cases**: Delete `Expr::SpawnFor` and related special handling code
4. **Extensible**: Future new control flow structures automatically combine with `spawn` without
   modifying spawn logic

### Disadvantages

1. **Type system bloat**: 6 new `MonoType` variants, increases type checking complexity
2. **Breaking change**: Internal AST/IR representation changed; need to update all code consuming
   `Expr::SpawnFor`
3. **Expression type inference**: Every expression now needs to return computational structure type,
   large impact surface

## Alternative Approaches

| Approach                                                                                | Why Not Chosen                                                                                                      |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Keep `spawn for` as independent syntax                                                  | Breaks orthogonality, becomes the only keyword combination special case in the language                             |
| `spawn` only decorates `{}`, data parallelism via standard library `par_iter`           | Language primitive capability demoted to library, loses compiler-level DAG analysis and resource conflict detection |
| Only delete `SpawnFor` but don't introduce computational structure types in type system | Type system loses reflection capability; spawn invisible at type level                                              |

## Relationship with RFC-019

The 6 `MonoType` variants introduced by this RFC (Block/ForExpr/WhileExpr/IfExpr/Call/Spawn) are the
**compiler-builtin subset** of [RFC-019: Typed Homoiconicity](./019-typed-homoiconicity.md). The
core idea of RFC-019 — "syntax structure enters the type system" — is implemented here as: 6
compiler-natively understood computational structures have corresponding type representations. Users
cannot define new computational structure types via `SyntaxRule`, but the 6 compiler-builtin ones
already cover all key control flows.

## Proof Pipeline Integration

The 6 `MonoType` variants exist because they tell the
[RFC-027 Compile-Time Proof Pipeline](../accepted/027-compile-time-evaluation-types.md) **what shape
the proposition to verify is**. The pipeline itself is responsible for the actual proof work (free
variable analysis, effect classification, alias analysis, conflict detection). MonoType does one
thing only — provides a structured input interface.

### Variant → Proposition Mapping

| Type                                 | Proposition Shape                                                                            | Proof Strategy                                                                                |
| ------------------------------------ | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `Spawn(ForExpr { body_ty })`         | Data parallelism: N iteration tasks with no cross-iteration conflicts                        | Extract body free variables → effect classification → check no Write(Shared) / `&mut`(Shared) |
| `Spawn(WhileExpr { body_ty })`       | Conditional parallelism: each iteration independent + no cross-iteration causal dependencies | Same as above + check if iteration condition has cross-iteration side effects                 |
| `Spawn(Block(T))`                    | Explicit task group: inter-task dependencies given by DAG                                    | Verify DAG analysis dependency graph — each task's required inputs are ready when it starts   |
| `Spawn(IfExpr { then_ty, else_ty })` | Branch spawn: selected branch is entire spawn scope                                          | No branch selection conflicts; recursive check inside body                                    |
| `Spawn(Call { fn_ty, result_ty })`   | Call spawn: called function is one independent task                                          | Verify function purity or isolation                                                           |
| `Spawn(T)` (value, like `spawn 42`)  | Single value spawn: no concurrency                                                           | Trivially passes                                                                              |

### Proof Scenarios

**Scenario 1 — Pure Data Parallelism (Passes):**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// Type: Spawn(ForExpr { body_ty: List(Int) })
```

1. Free variables: `item` (loop-local, independent copy each iteration), `items` (external,
   read-only in body)
2. Effect classification: all Read(Local) or Read(Shared), no writes
3. Proved ✓

**Scenario 2 — Read-Only Shared (Passes):**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// Type: Spawn(ForExpr { body_ty: List(Result) })
```

1. Free variables: `item` (Read(Local)), `config` (external, no write path in body → Read(Shared))
2. Effect classification: all read-only
3. Proved ✓

**Scenario 3 — Write Conflict (Rejected):**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. Free variables: `item` (Read(Local)), `counter` (external, `+=` desugared to write)
2. Effect classification: `counter` is Write(Shared), cross-iteration writes to same memory
3. Conflict instantiation: `Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. Disproved ✗ → Compile error:
   `Error: cross-iteration write conflict in spawn for body. Variable counter is written by multiple concurrent tasks.`

**Scenario 4 — while + Stateful Iterator (Warning/Reject):**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// Type: Spawn(WhileExpr { body_ty: List(Processed) })
```

1. Free variables: `iter` (external, `next()` → `&mut self` → `&mut`(Shared))
2. `next()` modifies iterator state; iteration N+1 depends on iteration N's side effects
3. This is not independent tasks → violates `Spawn(WhileExpr)` independence constraint
4. Compiler reports cross-iteration causal dependency; suggests using `spawn for` instead

**Scenario 5 — spawn if (Passes):**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// Type: Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. Only one branch executes; no cross-task conflicts
2. If body has nested spawns, recursive check applies
3. Proved ✓

**Scenario 6 — spawn block inter-task dependencies (DAG + Pipeline Verification):**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // depends on a
    c = compute_stats()           // independent
}
// Type: Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG analysis: `a` and `c` are independent (can parallelize), `b` depends on `a` (scheduled after
   a)
2. Pipeline verification: `b`'s input (`a.user_id`) is computed before b starts
3. Proved ✓

### What MonoType Does NOT Do

| Does                                          | Does NOT                                                           |
| --------------------------------------------- | ------------------------------------------------------------------ |
| Identifies proposition shape                  | Execute proofs                                                     |
| Records computational structure at type level | Replace DAG analysis                                               |
| Provides typed input to RFC-027 pipeline      | Replace free variable analysis, alias analysis, conflict detection |

The actual proof work is done by compiler standard analysis passes. MonoType's value is making these
passes be scheduled under a unified type framework — the proof pipeline doesn't need to write
special branches for each AST node type.

## Implementation Strategy

### Phase Division

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, delete `SpawnFor`
2. **Type System**: Add 6 new `MonoType` variants; all expression type inference returns
   computational structure type
3. **DAG Analysis Unification**: Merge entry points, unify `TaskSource` enum
4. **IR / Codegen Adaptation**: Delete `Ir::SpawnFor`, unify processing path
5. **Placement Simplification**: Delete `SpawnFor` branch
6. **Test Verification**: All existing `spawn for` tests pass

### Impact Scope

| File/Directory                               | Changes                                                              |
| -------------------------------------------- | -------------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body changed to `Box<Expr>`, delete `SpawnFor`               |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` handler simplified to generic expression parsing             |
| `frontend/core/types/mono.rs`                | New `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` variants   |
| `frontend/core/spawn/analysis.rs`            | Unified entry, `TaskSource` merged Explicit + Iterate                |
| `frontend/core/spawn/placement.rs`           | Delete `SpawnFor` branch                                             |
| `frontend/core/typecheck/`                   | All expression nodes adapt to computational structure type inference |
| `middle/core/ir.rs`                          | Delete `Ir::SpawnFor`                                                |
| `middle/` (IR gen, codegen)                  | Unified spawn path, Spawn type erasure                               |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verify passes                                   |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — foundation for type system changes
- RFC-027 (compile-time proof pipeline) — MonoType variants provide proposition shape input for the
  pipeline
- RFC-019 (typed homoiconicity) — MonoType variants are its compiler-builtin subset

## Design Decision Record

| Decision                   | Decision                                                                                                     | Reason                                                                       | Date       |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------- | ---------- |
| spawn decoration scope     | Any expression                                                                                               | Eliminate `spawn for` special case                                           | 2026-06-16 |
| `spawn while` support      | Supported                                                                                                    | Syntax orthogonal, low implementation cost                                   | 2026-06-16 |
| `spawn if` semantics       | Decorates entire if-else                                                                                     | Distinguishes from `if spawn { }`                                            | 2026-06-16 |
| Type system                | Introduced computational structure types                                                                     | "Everything is a type", supports compile-time reflection                     | 2026-06-16 |
| spawn type erasure         | Erased after type checking                                                                                   | Runtime doesn't need concurrency structure info                              | 2026-06-16 |
| spawn binding precedence   | Lowest (same as return)                                                                                      | Consumes entire following expression                                         | 2026-06-16 |
| DAG for for internals      | Don't expand for internals                                                                                   | Direct sub-expression rules unchanged; for as one task source                | 2026-06-16 |
| Proof pipeline integration | MonoType variants map to RFC-027 proof propositions                                                          | Pipeline needs to know proposition shape; MonoType provides structured input | 2026-07-03 |
| RFC-019 relationship       | Compiler-builtin subset                                                                                      | Users cannot customize, but share "syntax-as-type" philosophy                | 2026-07-03 |
| Proof boundaries           | 6 scenarios covered: pure parallelism/read-only sharing/write conflict/while dependency/spawn if/spawn block | Clarify each MonoType variant's proof obligations and failure conditions     | 2026-07-03 |

---

## References

- [RFC-024: Spawn-based Concurrency Model](./024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-027: Compile-Time Predicate and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: Typed Homoiconicity](./019-typed-homoiconicity.md)
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
- [spawn for Orthogonality Discussion (Draft)](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## Lifecycle and Disposition

| Status           | Location                  | Description                   |
| ---------------- | ------------------------- | ----------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion |
