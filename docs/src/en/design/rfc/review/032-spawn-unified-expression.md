---
title: 'RFC-032: Unified Expression Modifier for spawn — Eliminating Special Cases for spawn for'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: Unified Expression Modifier for spawn

> **This document defines `spawn` syntax and AST/IR restructuring.** For runtime behavior semantics (task decomposition granularity, ownership, scope, error propagation, resource types, nesting), see [RFC-024: Concurrency Runtime Semantics Based on Spawn](../accepted/024-concurrency-model.md).
>
> The two RFCs collaboratively define `spawn` — 024 answers "what to do", 032 answers "how to represent".

> **Core Insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**. `spawn for` is not a special syntax — it is the natural combination of `spawn` + `for` expression.

## Summary

Extend `spawn` from `spawn { }` (modifying only blocks) to `spawn <expr>` (modifying any expression). `Expr::SpawnFor` is removed from the AST, naturally replaced by `Expr::Spawn { body: Expr::For { .. } }`. This RFC only does AST/IR/Parser cleanup, with no type system changes.

> **Computational structure types (`MonoType` extension) is deferred to a separate RFC.** After this RFC removes `SpawnFor`, integrating `spawn` into the proof pipeline requires the type system to be aware of computational structures — this is a general mechanism, not limited to spawn, and deserves independent design.

## Motivation

### Why is this change needed?

Currently `spawn for x in items { body }` is an independent keyword combination, and `Expr::SpawnFor` exists in the AST specifically to represent it. This breaks language orthogonality:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks, `spawn for` is a hardcoded exception
2. **Missing orthogonality**: Combinations like `spawn while`, `spawn if` cannot be expressed naturally

### Current problems

```rust
// Two spawn variants in AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## Proposal

### Core Design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG decomposes tasks.

### User Mental Model

`spawn` = "take this expression and do it concurrently". The expression's shape determines decomposition:

| Expression Shape              | Concurrency Behavior                     |
| ----------------------------- | ---------------------------------------- |
| `spawn { a, b, c }`           | `a`, `b`, `c` run independently in parallel |
| `spawn for x in items { f(x) }` | N iterations run independently in parallel |
| `spawn while cond { step() }` | Each iteration is an independent task    |
| `spawn if c { a } else { b }` | Selected branch as a whole is the spawn domain |
| `spawn call(x)`               | The call itself is one task              |
| `spawn 42`                    | A single task                            |

The compiler is responsible for DAG analysis to determine dependencies, and the runtime schedules according to the GMP model — tasks with no dependencies are thrown into the work queue, workers compete to run them. Overall synchronization blocks, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "throw it out and don't care", YaoXiang's `spawn` is "decompose for parallel execution, wait for all to finish before continuing".

### Control Flow Orthogonality

| Combination                        | Semantics                               | Differences                            |
| ---------------------------------- | --------------------------------------- | -------------------------------------- |
| `spawn for x in items { body }`    | Data parallelism: each iteration = independent task | DAG analyzes dependencies across iterations |
| `for x in items spawn { body }`   | Each iteration creates a spawn domain   | No cross-iteration analysis            |
| `spawn while cond { body }`        | Conditional parallelism: each iteration = independent task | Inter-iteration dependencies guaranteed by condition |
| `while cond spawn { body }`        | Each iteration creates a spawn domain   | Different semantics from above but no special handling needed |
| `spawn if c { a } else { b }`      | The entire if-else is one spawn domain   | Executes according to condition at runtime |
| `if c spawn { a } else { b }`      | Only the single branch is spawned       | spawn wraps inside the if expression   |

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

`Expr::SpawnFor` deleted. AST representation of `spawn for x in items { body }`:

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

**IF edge cases:**

| Syntax                               | AST Structure                                     |
| ------------------------------------ | ------------------------------------------------- |
| `spawn if cond { a } else { b }`    | `Spawn { body: Expr::If { ... } }`                |
| `if cond spawn { a } else { b }`    | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

Both have different semantics but are natural combinations, requiring no special rules.

### 2. Parser Layer

`spawn` binds with the lowest precedence (same as `return`), consuming the entire following expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser changes: In `pratt/nud.rs`, `spawn` no longer requires `{`, but calls generic expression parsing:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer treated as a combined keyword — `for` is processed by the generic expression parser to produce `Expr::For`, and `spawn` only handles wrapping.

### 3. DAG Analysis Layer

Two entry points merged into one:

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

**Unified result structure:**

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — N direct child expressions known at compile time
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N tasks generated by runtime iteration
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for has it, while doesn't
        condition: Option<Expr>,     // while has it, for doesn't
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` struct deleted.

| body kind            | How decomposed into tasks                |
| -------------------- | ---------------------------------------- |
| `Expr::Block`        | Direct child expressions → task list     |
| `Expr::For`          | Each iteration → one task (data parallelism) |
| `Expr::While`        | Each iteration → one task                |
| `Expr::If`           | Selected branch → one task               |
| `Expr::Call` / other | The expression itself → one task         |

After DAG analysis completes, the runtime schedules according to the GMP model — tasks with no dependencies are thrown into the work queue, workers compete to run them.

### 4. IR / Codegen Layer

`Ir::SpawnFor` deleted. Unified to `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → tasks known at compile time
- `TaskSource::Iterate { .. }` → runtime expansion (compiler-driven, similar to par_iter but zero-cost)

### 5. Placement Layer

Two branches merged into one:

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

### 6. Backward Compatibility

Existing `spawn for` code has unchanged semantics. Parser automatically parses `spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. Internal representation changes, user-visible behavior unchanged.

New syntax naturally gained:

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

**Single-task spawn warning**: When `spawn` modifies a single expression like `spawn call(x)` and `spawn 42`, DAG analysis produces a compile warning: "spawn modifying a single expression has no concurrency effect". Syntax is legal, but reminds user to check intent.

## Trade-offs

### Advantages

1. **Syntax orthogonality**: `spawn` + any control flow = natural concurrency combination
2. **Eliminate special cases**: Remove `Expr::SpawnFor` and related special handling code
3. **Extensible**: Future control flow structures automatically combine with `spawn`, no spawn logic modification needed

### Disadvantages

1. **Breaking change**: Internal AST/IR representation changes, all code consuming `Expr::SpawnFor` must be updated
2. **Proof pipeline adaptation needed**: After removing `SpawnFor`, the proof pipeline dispatches through AST (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is completed within this RFC scope through the DAG unified entry point

## Alternative Approaches

| Approach                                        | Why not chosen                                                 |
| ----------------------------------------------- | -------------------------------------------------------------- |
| Keep `spawn for` as independent syntax           | Breaks orthogonality, becomes the only keyword combination special case in the language |
| `spawn` only modifies `{}`, data parallelism via standard library `par_iter` | Language's primitive capability moved to library, loses compiler-level DAG analysis and resource conflict detection |

## Computational Structure Types (Deferred to Separate RFC)

After this RFC removes `SpawnFor`, integrating `spawn` into the proof pipeline faces an architectural issue: the proof pipeline works at the type level and needs to know the computational structure inside spawn (For/While/Block/If/Call) to select the correct proof strategy. Currently the proof pipeline dispatches through AST, but the long-term direction is to encode computational structures as `MonoType` variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), making the pipeline work entirely at the type level.

This is a weakened practical version of [RFC-019: Typed Homoiconicity](../draft/019-typed-homoiconicity.md) — compiler-built-in computational structures enter the type system, but without exposing user-defined syntax. The theoretical foundation is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP 2021): `Spawn<T>` corresponds to modal operator `□`, and the proof pipeline corresponds to handlers.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can enter the type system through the same pattern. spawn is the first consumer, not the only consumer.

> **The independent RFC will define**: Complete semantics for 6 MonoType variants, type checker adaptation strategy, unified interface for proof pipeline dispatch by type, and integration plan with RFC-027.

## Implementation Strategy

### Phasing

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, delete `SpawnFor`
2. **DAG Analysis unification**: Merge entry points, unify `TaskSource` enum. Single-task spawn (`spawn call(x)`, `spawn 42`) produces compile warnings
3. **IR / Codegen adaptation**: Delete `Ir::SpawnFor`, unify processing path
4. **Placement simplification**: Delete `SpawnFor` branch
5. **Testing validation**: All existing `spawn for` tests pass

### Impact Scope

| File/Directory                                | Changes                                                         |
| --------------------------------------------- | --------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                 | `Spawn` body changed to `Box<Expr>`, delete `SpawnFor`          |
| `frontend/core/parser/pratt/nud.rs`           | `spawn` handler simplified to generic expression parsing        |
| `frontend/core/spawn/analysis.rs`             | Unified entry, `TaskSource` merges Explicit + Iterate          |
| `frontend/core/spawn/placement.rs`            | Delete `SpawnFor` branch                                        |
| `middle/core/ir.rs`                           | Delete `Ir::SpawnFor`                                          |
| `middle/` (IR gen, codegen)                   | Unified spawn path                                             |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, validation passes                         |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonal extension
- RFC-010 (unified type syntax) — foundation for syntax unification

## Design Decision Log

| Decision                  | Decision                                                          | Reason                                                    | Date        |
| ------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------- | ----------- |
| spawn modifier scope      | Any expression                                                    | Eliminate `spawn for` special case                        | 2026-06-16  |
| `spawn while` support     | Supported                                                         | Syntax orthogonal, low implementation cost. Proof pipeline may reject cross-iteration dependency use cases | 2026-06-16  |
| `spawn if` semantics      | Modifies entire if-else                                           | Distinguish from `if spawn { }`                          | 2026-06-16  |
| spawn binding precedence  | Lowest (same as return)                                           | Consumes the entire expression following it              | 2026-06-16  |
| DAG for for body          | Does not expand inside for                                        | Direct child expression rules unchanged, for as a whole is one task source | 2026-06-16  |
| Single-task spawn warning | `spawn call(x)` / `spawn 42` produce compile warnings             | No concurrency effect, remind user to check intent       | 2026-08-19  |
| Computational structure types | Deferred to separate RFC                                         | General mechanism, not limited to spawn. ECMTT theoretical foundation | 2026-08-19  |

---

## References

- [RFC-024: Concurrency Model Based on Spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976) — theoretical foundation for computational structure types
- [Concurrency Model Specification](../../../reference/language-spec/concurrency.md)

---

## Lifecycle and Disposition

| Status            | Location                        | Description             |
| ----------------- | ------------------------------- | ----------------------- |
| **Under Review** | `docs/design/rfc/review/`       | Open for community discussion |