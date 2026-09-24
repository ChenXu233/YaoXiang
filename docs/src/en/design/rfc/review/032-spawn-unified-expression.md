---
title: 'RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn Unified Expression Modifier

> **This document defines the syntax and AST/IR refactor for `spawn`**. Runtime behavior semantics
> (task decomposition granularity, ownership, scope, error propagation, resource types, nesting) are
> described in
> [RFC-024: spawn-based Concurrency Runtime Semantics](../accepted/024-concurrency-model.md).
>
> The two RFCs jointly define `spawn` — 024 answers "what to do", 032 answers "how to represent it".

> **Core Insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**.
> `spawn for` is not special syntax — it is simply the natural composition of `spawn` + a `for`
> expression.

## Summary

Extend `spawn` from `spawn { }` (modifying only a block) to `spawn <expr>` (modifying any
expression). `Expr::SpawnFor` is removed from the AST and naturally replaced by
`Expr::Spawn { body: Expr::For { .. } }`. This RFC only performs AST/IR/Parser cleanup, with no
changes to the type system.

> **Computation structure types (`MonoType` extension) are deferred to a separate RFC.** After this
> RFC removes the `SpawnFor` special case, integrating `spawn` with the proof pipeline requires the
> type system to be aware of computation structure — this is a general mechanism, not limited to
> spawn, and deserves its own design.

## Motivation

### Why is this change needed?

The current `spawn for x in items { body }` is an independent keyword combination, with a dedicated
`Expr::SpawnFor` in the AST to represent it. This breaks the orthogonality of the language:

1. **Syntax is not unified**: `spawn` can only modify `{}` blocks; `spawn for` is a hardcoded
   exception
2. **Loss of orthogonality**: Combinations like `spawn while`, `spawn if` cannot be expressed
   naturally

### Current Problems

```rust
// Two spawn variants in the AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## Proposal

### Core Design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG
decomposes tasks.

### User Mental Model

`spawn` = "take this expression and run it concurrently". The shape of the expression determines how
to split:

| Expression shape                | Concurrent behavior            |
| ------------------------------- | ------------------------------ |
| `spawn { a, b, c }`             | `a`, `b`, `c` run in parallel  |
| `spawn for x in items { f(x) }` | N iterations run in parallel   |
| `spawn while cond { step() }`   | Each iteration is a task       |
| `spawn if c { a } else { b }`   | Selected branch as spawn scope |
| `spawn call(x)`                 | Call itself is a task          |
| `spawn 42`                      | A single task                  |

The compiler is responsible for DAG analysis to determine dependencies; the runtime schedules
according to the GMP model — tasks with no dependencies are thrown into the work queue, workers race
to run them. The whole thing synchronously blocks, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget", whereas YaoXiang's `spawn` is "split and run
in parallel, then wait until all are done before continuing".

### Control Flow Orthogonality

| Combination                     | Semantics                                   | Difference                                                 |
| ------------------------------- | ------------------------------------------- | ---------------------------------------------------------- |
| `spawn for x in items { body }` | Data parallel: each iteration = task        | DAG analyzes dependencies across iterations                |
| `for x in items spawn { body }` | Each iteration creates a spawn scope        | No cross-iteration analysis                                |
| `spawn while cond { body }`     | Conditional parallel: each iteration = task | Inter-iteration deps guaranteed by condition               |
| `while cond spawn { body }`     | Each iteration creates a spawn scope        | Different semantics from above, no special handling needed |
| `spawn if c { a } else { b }`   | Entire if-else is one spawn scope           | Select branch at runtime                                   |
| `if c spawn { a } else { b }`   | Only single branch is spawn                 | Spawn wrapped inside if expression                         |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` removed from AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer specially handled as a combined keyword in Parser
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

`Expr::SpawnFor` is removed. The AST representation of `spawn for x in items { body }` is:

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

**IF special cases**:

| Syntax                           | AST structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

Both have different semantics but are natural combinations, requiring no special rules.

### 2. Parser Layer

`spawn` has the lowest binding precedence (same as `return`), consuming the entire following
expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser changes: in `pratt/nud.rs`, `spawn` no longer requires `{`, but instead calls the general
expression parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is handled by the general expression
parser to produce `Expr::For`, and `spawn` is only responsible for wrapping.

### 3. DAG Analysis Layer

The two current entries are merged into one:

```rust
/// Unified entry: dispatches based on body expression kind
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

**Unified result structure**:

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — N direct subexpressions known at compile time
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

| body kind             | How decomposed into tasks                 |
| --------------------- | ----------------------------------------- |
| `Expr::Block`         | Direct subexpressions → task list         |
| `Expr::For`           | Each iteration → one task (data parallel) |
| `Expr::While`         | Each iteration → one task                 |
| `Expr::If`            | Selected branch as a whole → one task     |
| `Expr::Call` / others | The expression itself → one task          |

After DAG analysis, the runtime schedules according to the GMP model — tasks with no dependencies
are thrown into the work queue, workers race to run them.

### 4. IR / Codegen Layer

`Ir::SpawnFor` is removed. Unified as `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → task list known at compile time
- `TaskSource::Iterate { .. }` → runtime expansion (compiler-driven, similar to par_iter but
  zero-cost)

### 5. Placement Layer

The two current branches are merged into one:

```rust
// Before
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// After
Expr::Spawn { body, .. } => self.check_expr(body),   // body is Expr, just recurse
```

### 6. Backward Compatibility

The semantics of existing `spawn for` code remain unchanged; the Parser automatically parses
`spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. The internal representation
changes; the user-visible behavior does not.

New syntax comes naturally:

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

**Single-task spawn warning**: When modifying a single expression like `spawn call(x)` and
`spawn 42`, DAG analysis produces a compile-time warning: "spawn modifying a single expression has
no concurrent effect". The syntax is legal, but the user is reminded to check their intent.

## Trade-offs

### Advantages

1. **Syntactic orthogonality**: `spawn` + any control flow = natural concurrent combination
2. **Eliminates special cases**: Removes `Expr::SpawnFor` and related special-handling code
3. **Extensibility**: Future new control flow structures automatically combine with `spawn`, no need
   to modify spawn logic

### Disadvantages

1. **Breaking change**: Internal AST/IR representation changes, requires updating all code that
   consumes `Expr::SpawnFor`
2. **Proof pipeline needs adaptation**: After removing `SpawnFor`, the proof pipeline dispatches via
   AST (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is accomplished
   within this RFC's scope through the unified DAG entry point

## Alternatives

| Approach                                                                     | Why not chosen                                                                                                           |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Keep `spawn for` as independent syntax                                       | Breaks orthogonality, becomes the only keyword-combination special case in the language                                  |
| `spawn` only modifies `{}`, data parallelism via standard library `par_iter` | Language primitive capability sinks into the library, losing compiler-level DAG analysis and resource conflict detection |

## Computation Structure Types (Deferred to Separate RFC)

After this RFC removes `SpawnFor`, integrating `spawn` with the proof pipeline faces an
architectural issue: the proof pipeline operates at the type level and needs to know the computation
structure inside spawn (For/While/Block/If/Call) to choose the correct proof strategy. Currently the
proof pipeline dispatches via AST, but the long-term direction is to encode computation structure as
`MonoType` variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), so the pipeline works
entirely at the type level.

This is a weakened practical version of
[RFC-019: Type-level Homoiconicity](../draft/019-typed-homoiconicity.md) — compiler-built-in
computation structures enter the type system, but user-defined syntax is not opened up. The
theoretical foundation is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP
2021): `Spawn<T>` corresponds to the modal operator `□`, and the proof pipeline corresponds to the
handler.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can
enter the type system through the same pattern. spawn is the first consumer, not the only consumer.

> **The separate RFC will define**: complete semantics of 6 MonoType variants, type checker
> adaptation strategy, unified interface for the proof pipeline's type-based dispatch, and
> integration plan with RFC-027.

## Implementation Strategy

### Phasing

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **Unified DAG analysis**: Merge entry points, unify `TaskSource` enum. Single-task spawn
   (`spawn call(x)`, `spawn 42`) produces compile-time warnings
3. **IR / Codegen adaptation**: Remove `Ir::SpawnFor`, unify processing paths
4. **Placement simplification**: Remove `SpawnFor` branch
5. **Test verification**: All existing `spawn for` tests pass

### Scope of Impact

| File/Directory                               | Changes                                                |
| -------------------------------------------- | ------------------------------------------------------ |
| `frontend/core/parser/ast.rs`                | Change `Spawn` body to `Box<Expr>`, remove `SpawnFor`  |
| `frontend/core/parser/pratt/nud.rs`          | Simplify `spawn` handler to general expression parsing |
| `frontend/core/spawn/analysis.rs`            | Unified entry, merge `TaskSource` Explicit + Iterate   |
| `frontend/core/spawn/placement.rs`           | Remove `SpawnFor` branch                               |
| `middle/core/ir.rs`                          | Remove `Ir::SpawnFor`                                  |
| `middle/` (IR gen, codegen)                  | Unify spawn paths                                      |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verify passing                    |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — foundation of syntax unification

## Design Decision Log

| Decision                    | Determination                                         | Reason                                                                                                       | Date       |
| --------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------- |
| spawn modification scope    | Any expression                                        | Eliminate `spawn for` special case                                                                           | 2026-06-16 |
| `spawn while` support       | Supported                                             | Syntactic orthogonality, low implementation cost. Proof pipeline may reject cross-iteration dependency cases | 2026-06-16 |
| `spawn if` semantics        | Modify entire if-else                                 | Distinguish from `if spawn { }`                                                                              | 2026-06-16 |
| spawn binding precedence    | Lowest (same as return)                               | Consume the entire following expression                                                                      | 2026-06-16 |
| DAG handling of for body    | Do not expand for body's subexpressions               | Direct subexpression rule unchanged, for as a whole is one task source                                       | 2026-06-16 |
| Single-task spawn warning   | `spawn call(x)` / `spawn 42` produces compile warning | No concurrent effect, remind user to check intent                                                            | 2026-08-19 |
| Computation structure types | Deferred to separate RFC                              | General mechanism, not limited to spawn. ECMTT theoretical basis                                             | 2026-08-19 |

---

## References

- [RFC-024: spawn Block-based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — theoretical foundation of computation structure types
- [Concurrency Model Specification](../../../reference/language-spec/concurrency.md)
- [spawn for Orthogonality Suspension (Discussion Draft)](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## Lifecycle and Destination

| Status           | Location                  | Description               |
| ---------------- | ------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
