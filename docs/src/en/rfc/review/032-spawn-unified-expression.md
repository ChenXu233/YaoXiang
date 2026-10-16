---
title: 'RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-10-14'
issue: '#98'
---

# RFC-032: spawn Unified Expression Modifier

> **This document defines the syntax and AST/IR refactor for `spawn`.** Runtime behavior semantics
> (task decomposition granularity, ownership, scope, error propagation, resource types, nesting) are
> defined in
> [RFC-024: Concurrency Runtime Semantics Based on spawn](../accepted/024-concurrency-model.md).
>
> These two RFCs jointly define `spawn` — RFC-024 answers "what to do", RFC-032 answers "how to
> represent it".

> **Core insight**: `spawn` should not only modify `{ }` blocks. It can modify **any expression**.
> `spawn for` is not special syntax — it is simply the natural composition of `spawn` + `for`
> expression.

## Summary

Extend `spawn` from `spawn { }` (only modifies blocks) to `spawn <expr>` (modifies any expression).
`Expr::SpawnFor` is removed from the AST and naturally replaced by
`Expr::Spawn { body: Expr::For { .. } }`. This RFC only performs AST/IR/Parser cleanup, with no type
system changes.

> **Computation structure type (`MonoType` extension) is deferred to a separate RFC.** After this
> RFC removes the `SpawnFor` special case, the proof pipeline integration for `spawn` requires the
> type system to be aware of computation structure — this is a general mechanism, not limited to
> spawn, and deserves an independent design.

## Motivation

### Why is this change needed?

Currently, `spawn for x in items { body }` is a standalone keyword combination, with
`Expr::SpawnFor` in the AST to represent it specifically. This breaks the orthogonality of the
language:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks; `spawn for` is a hardcoded
   exception
2. **Lack of orthogonality**: Combinations like `spawn while`, `spawn if` cannot be expressed
   naturally

### Current problem

```rust
// Two spawn variants in the AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## Proposal

### Core design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG
decomposes tasks.

### User mental model

`spawn` = "take this expression and run it concurrently". The shape of the expression determines how
to decompose it:

| Expression shape                | Concurrent behavior                         |
| ------------------------------- | ------------------------------------------- |
| `spawn { a, b, c }`             | `a`, `b`, `c` run independently in parallel |
| `spawn for x in items { f(x) }` | N iterations run independently in parallel  |
| `spawn while cond { step() }`   | Each iteration is an independent task       |
| `spawn if c { a } else { b }`   | The selected branch is a whole spawn domain |
| `spawn call(x)`                 | The call itself is a single task            |
| `spawn 42`                      | A single task                               |

The compiler is responsible for DAG analysis to determine dependencies; the runtime schedules
according to the GMP model — tasks without dependencies are thrown into the work queue, and workers
compete to run. Synchronous blocking as a whole, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget"; YaoXiang's `spawn` is "decompose for
parallel execution, wait for all to complete before continuing".

### Control flow orthogonality

| Combination                     | Semantics                                                  | Difference                                                    |
| ------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------- |
| `spawn for x in items { body }` | Data parallelism: each iteration = independent task        | DAG analyzes dependencies across iterations                   |
| `for x in items spawn { body }` | Each iteration creates a spawn domain                      | Does not analyze across iterations                            |
| `spawn while cond { body }`     | Conditional parallelism: each iteration = independent task | Inter-iteration dependencies guaranteed by condition          |
| `while cond spawn { body }`     | Each iteration creates a spawn domain                      | Different semantics from above but no special handling needed |
| `spawn if c { a } else { b }`   | The entire if-else is one spawn domain                     | Select branch at runtime based on condition                   |
| `if c spawn { a } else { b }`   | Only single-branch is spawned                              | spawn wrapped inside if expression                            |

### Eliminated complexity

- ❌ `Expr::SpawnFor` removed from the AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer specially handled as a combined keyword in the Parser
- ❌ ir_gen independent lowering path `generate_spawn_for_expr_ir` removed (merged into the unified
  `Spawn` translation path)

## Detailed design

### 1. AST layer

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

**IF special case**:

| Syntax                           | AST structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

These have different semantics but are both natural compositions, requiring no special rules.

### 2. Parser layer

`spawn` has the lowest binding precedence (same as `return`), consuming the entire subsequent
expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser change: in `pratt/nud.rs`, `spawn` no longer requires `{`, but instead invokes the general
expression parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is processed by the general
expression parser to produce `Expr::For`, and `spawn` only handles wrapping.

### 3. DAG analysis layer

The two current entry points are merged into one:

```rust
/// Unified entry point: dispatch by body expression kind
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
    /// spawn for/while — N tasks produced by runtime iteration
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for has it, while does not
        condition: Option<Expr>,     // while has it, for does not
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

The `SpawnForAnalysis` struct is removed.

| body type            | How to decompose into tasks                  |
| -------------------- | -------------------------------------------- |
| `Expr::Block`        | Direct subexpressions → task list            |
| `Expr::For`          | Each iteration → one task (data parallelism) |
| `Expr::While`        | Each iteration → one task                    |
| `Expr::If`           | Selected branch as a whole → one task        |
| `Expr::Call` / other | The expression itself → one task             |

After DAG analysis is complete, the runtime schedules according to the GMP model — tasks without
dependencies are thrown into the work queue, and workers compete to run.

### 4. IR / Codegen layer

**Factual correction (2026-10-14)**: The IR instruction layer (`middle/core/ir.rs`) never had a
`SpawnFor` variant to begin with, only `Instruction::Spawn`(:330) and
`Instruction::SpawnFromList`(:340), both of which **remain unchanged**. The two paths exist in the
ir_gen translation layer: `Expr::SpawnFor` → `generate_spawn_for_expr_ir`(:8605) and `Expr::Spawn` →
`generate_spawn_expr_ir`(:8624). This RFC merges the two translation paths, with unified dispatch by
`SpawnAnalysis.source` to generate the `Instruction::Spawn` family.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → task list known at compile time
- `TaskSource::Iterate { .. }` → runtime expansion (compiler-driven, similar to par_iter but
  zero-cost)

### 5. Placement layer

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

### 6. Backward compatibility

The semantics of existing `spawn for` code are unchanged; the Parser automatically parses
`spawn for x in items { body }` into `Expr::Spawn { body: Expr::For }`. The internal representation
changes, while the user-visible behavior remains unchanged.

New syntax is obtained naturally:

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

**Single-task spawn warning**: When modifying a single expression such as `spawn call(x)` and
`spawn 42`, DAG analysis produces a compile warning: "spawn modifying a single expression has no
concurrent effect". The syntax is valid, but the user is reminded to verify their intent.

## Trade-offs

### Advantages

1. **Syntax orthogonality**: `spawn` + any control flow = natural concurrent composition
2. **Elimination of special cases**: Remove `Expr::SpawnFor` and related special-case handling code
3. **Extensibility**: Future new control flow structures automatically compose with `spawn`, no need
   to modify spawn logic

### Disadvantages

1. **Breaking change**: Internal AST representation changes, requires updating all 13 source files
   consuming `Expr::SpawnFor` (see "Impact Scope" for the list); IR snapshots must be regenerated in
   the same commit
2. **Proof pipeline must adapt**: After removing `SpawnFor`, the proof pipeline dispatches via AST
   (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is completed in this
   RFC's scope through the unified DAG entry point

## Alternatives

| Plan                                                                          | Why not choose it                                                                                               |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Keep `spawn for` as independent syntax                                        | Breaks orthogonality, becomes the only keyword combination special case in the language                         |
| `spawn` only modifies `{}`, data parallelism uses standard library `par_iter` | Primitive capabilities moved to the library, losing compiler-level DAG analysis and resource conflict detection |

## Computation structure type (deferred to a separate RFC)

After this RFC removes `SpawnFor`, the proof pipeline integration for `spawn` faces an architectural
problem: the proof pipeline operates at the type layer and needs to know the computation structure
(For/While/Block/If/Call) inside spawn to select the correct proof strategy. The current proof
pipeline dispatches via AST, but the long-term direction is to encode computation structure as
`MonoType` variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), allowing the pipeline
to work entirely at the type layer.

This is a weakened practical version of
[RFC-019: Type-level Homoiconicity](../deprecated/019-typed-homoiconicity.md) — the compiler's
built-in computation structure enters the type system, but does not open up user-defined syntax. The
theoretical foundation is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP
2021): `Spawn<T>` corresponds to the modal operator `□`, and the proof pipeline corresponds to the
handler.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can
enter the type system through the same pattern. spawn is the first consumer, not the only consumer.

> **A separate RFC will define**: the complete semantics of 6 MonoType variants, the type checker
> adaptation strategy, the unified interface for proof pipeline dispatch by type, and the
> integration plan with RFC-027.

## Implementation strategy

### Phases

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **Unified DAG analysis**: Merge entry points, unified `TaskSource` enum. Single-task spawn
   (`spawn call(x)`, `spawn 42`) produces a compile warning
3. **IR / Codegen adaptation**: Remove `Ir::SpawnFor`, unified processing path
4. **Placement simplification**: Remove `SpawnFor` branch
5. **Test verification**: All existing `spawn for` tests pass

### Impact scope

| File/Directory                                     | Changes                                                                                              |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                      | `Spawn` body changed to `Box<Expr>`, remove `SpawnFor` (:79/:1070)                                   |
| `frontend/core/parser/pratt/nud.rs`                | Remove `parse_spawn_for` (:154), `spawn` changed to general expression parsing                       |
| `frontend/core/parser/pratt/mod.rs`                | span helper remove `SpawnFor` branch (:52)                                                           |
| `frontend/core/spawn/analysis.rs`                  | Unified entry point; remove `SpawnForAnalysis` (:46), `TaskSource` merged                            |
| `frontend/core/spawn/placement.rs`                 | Remove `SpawnFor` branch (:218)                                                                      |
| `frontend/core/typecheck/checker.rs`               | `For \| SpawnFor` merged arm changed to single `For` (:4812)                                         |
| `frontend/core/typecheck/passes/dead_code.rs`      | Remove `SpawnFor` branch (:325)                                                                      |
| `frontend/core/typecheck/inference/expressions.rs` | Remove `SpawnFor` branch (:3950)                                                                     |
| `frontend/core/typecheck/inference/existential.rs` | span helper remove `SpawnFor` branch (:60)                                                           |
| `frontend/core/typecheck/layers/ownership.rs`      | Remove `SpawnFor` branches (:1271, :2291)                                                            |
| `frontend/core/typecheck/layers/termination.rs`    | `For \| SpawnFor` merged arm changed to single `For` (:1994)                                         |
| `middle/core/ir_gen.rs`                            | Merge `generate_spawn_for_expr_ir` (:8605); remove branches at :4638/:5418                           |
| `middle/core/ir.rs`                                | **Unchanged** — instruction layer never had `SpawnFor`; `Spawn`(:330)/`SpawnFromList`(:340) retained |
| `formatter/handlers/expr.rs`                       | Remove `SpawnFor` formatting branch (:186), `spawn for` synthesized from `Spawn`+`For`               |
| `tests/yaoxiang/04-concurrency/spawn_for.yx`       | Semantics unchanged, passes verification                                                             |
| IR snapshot `src/middle/core/tests/snapshots/`     | Related expression IR shape changes, **regenerate in the same commit** (AGENTS.md red line)          |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — foundation of unified syntax

## Design decision log

| Decision                    | Decision                                                                       | Reason                                                                                                    | Date       |
| --------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- | ---------- |
| spawn modification scope    | Any expression                                                                 | Eliminate the `spawn for` special case                                                                    | 2026-06-16 |
| `spawn while` support       | Supported                                                                      | Syntax orthogonality, low implementation cost. Proof pipeline may reject cross-iteration dependency cases | 2026-06-16 |
| `spawn if` semantics        | Modify the entire if-else                                                      | Distinguish from `if spawn { }`                                                                           | 2026-06-16 |
| spawn binding precedence    | Lowest (same as return)                                                        | Consume the entire subsequent expression                                                                  | 2026-06-16 |
| DAG on for internals        | Do not expand for internal subexpressions                                      | Direct subexpression rule unchanged, for as a whole is a task source                                      | 2026-06-16 |
| Single-task spawn warning   | `spawn call(x)` / `spawn 42` produces a compile warning                        | No concurrent effect, remind user to verify intent                                                        | 2026-08-19 |
| Computation structure type  | Deferred to a separate RFC                                                     | General mechanism, not limited to spawn. ECMTT theoretical foundation                                     | 2026-08-19 |
| IR layer factual correction | Instruction layer has no `SpawnFor`; merging is in the ir_gen translation path | `ir.rs` only has `Spawn`/`SpawnFromList`, original statement was incorrect                                | 2026-10-14 |

---

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — Theoretical foundation of computation structure type
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)

---

## Lifecycle and destination

| Status           | Location                  | Description                   |
| ---------------- | ------------------------- | ----------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion |
