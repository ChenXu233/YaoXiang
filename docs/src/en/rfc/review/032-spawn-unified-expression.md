---
title: 'RFC-032: Unified `spawn` Expression Modifier — Eliminating the `spawn for` Special Case'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: Unified `spawn` Expression Modifier

> **This document defines the syntax and AST/IR restructuring of `spawn`. Runtime behavior semantics
> (task decomposition granularity, ownership, scoping, error propagation, resource types, nesting)
> are covered in
> [RFC-024: spawn-based Concurrency Runtime Semantics](../accepted/024-concurrency-model.md).**
>
> **The two RFCs jointly define `spawn` — RFC-024 answers "what to do", RFC-032 answers "how to
> express it".**

> **Core insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**.
> `spawn for` is not special syntax — it is the natural combination of `spawn` + a `for` expression.

## Summary

Extend `spawn` from `spawn { }` (only modifying blocks) to `spawn <expr>` (modifying any
expression). `Expr::SpawnFor` is removed from the AST and naturally replaced by
`Expr::Spawn { body: Expr::For { .. } }`. This RFC only performs AST/IR/Parser cleanup, without type
system changes.

> **Computational structure types (`MonoType` extension) are deferred to a separate RFC.** After
> this RFC removes the `SpawnFor` special case, spawn's proof pipeline integration requires the type
> system to be aware of computational structure — this is a general mechanism not limited to spawn,
> and deserves its own design.

## Motivation

### Why is this change needed?

The current `spawn for x in items { body }` is an independent keyword combination, with
`Expr::SpawnFor` in the AST dedicated to representing it. This breaks the language's orthogonality:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks; `spawn for` is a hardcoded
   exception
2. **Missing orthogonality**: combinations like `spawn while`, `spawn if` cannot be expressed
   naturally

### Current Problem

```rust
// Two spawn variants in AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## Proposal

### Core Design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG
decomposes tasks.

### User Mental Model

`spawn` = "take this expression and run it concurrently". The shape of the expression determines how
to decompose:

| Expression shape                | Concurrent behavior                              |
| ------------------------------- | ------------------------------------------------ |
| `spawn { a, b, c }`             | `a`, `b`, `c` run in parallel                    |
| `spawn for x in items { f(x) }` | N iterations run in parallel                     |
| `spawn while cond { step() }`   | Each iteration is an independent task            |
| `spawn if c { a } else { b }`   | The selected branch as a whole is a spawn domain |
| `spawn call(x)`                 | The call itself as a single task                 |
| `spawn 42`                      | A single task                                    |

The compiler is responsible for DAG analysis to determine dependencies; the runtime schedules
according to the GMP model — dependency-free tasks are thrown into the work queue, workers grab and
run them. The whole operation synchronously blocks, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget"; YaoXiang's `spawn` is "decompose, run in
parallel, then wait for everything to finish before proceeding".

### Control Flow Orthogonality

| Combination                     | Semantics                                               | Difference                                                 |
| ------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------- |
| `spawn for x in items { body }` | Data parallel: each iteration = independent task        | DAG analyzes dependencies across iterations                |
| `for x in items spawn { body }` | Each iteration creates a spawn domain                   | No cross-iteration analysis                                |
| `spawn while cond { body }`     | Conditional parallel: each iteration = independent task | Inter-iteration dependencies guaranteed by condition       |
| `while cond spawn { body }`     | Each iteration creates a spawn domain                   | Different semantics from above, no special handling needed |
| `spawn if c { a } else { b }`   | The whole if-else is a single spawn domain              | Selects branch at execution time                           |
| `if c spawn { a } else { b }`   | Only single branch spawns                               | spawn wrapped inside if expression                         |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` removed from AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer specially handled in Parser as a combined keyword
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

**IF special case**:

| Syntax                           | AST Structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

The two have different semantics but are both natural combinations, requiring no special rules.

### 2. Parser Layer

`spawn` binds at the lowest precedence (same as `return`), consuming the entire following
expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser changes: in `pratt/nud.rs`, `spawn` no longer requires `{`, but calls the general expression
parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is handled by the general expression
parser to produce `Expr::For`, and `spawn` is only responsible for wrapping.

### 3. DAG Analysis Layer

The current two entries are merged into one:

```rust
/// Unified entry: dispatch based on body expression type
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
    /// spawn { a, b, c } — N directly known sub-expressions at compile time
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N tasks generated by runtime iteration
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for has, while doesn't
        condition: Option<Expr>,     // while has, for doesn't
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

The `SpawnForAnalysis` struct is removed.

| body type             | How to decompose into tasks               |
| --------------------- | ----------------------------------------- |
| `Expr::Block`         | Direct sub-expressions → task list        |
| `Expr::For`           | Each iteration → one task (data parallel) |
| `Expr::While`         | Each iteration → one task                 |
| `Expr::If`            | Selected branch as a whole → one task     |
| `Expr::Call` / others | Expression itself → one task              |

After DAG analysis, the runtime schedules according to the GMP model — dependency-free tasks are
thrown into the work queue, workers grab and run them.

### 4. IR / Codegen Layer

`Ir::SpawnFor` is removed. Unified into `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → compile-time known task list
- `TaskSource::Iterate { .. }` → runtime expansion (compiler-driven, similar to par_iter but
  zero-cost)

### 5. Placement Layer

The current two branches are merged into one:

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

The semantics of existing `spawn for` code are unchanged. The Parser automatically parses
`spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. Internal representation
changes, user-visible behavior remains the same.

New syntax naturally acquired:

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

**Single-task spawn warning**: When `spawn` modifies a single expression like `spawn call(x)` and
`spawn 42`, DAG analysis produces a compile warning: "spawn modifying a single expression has no
concurrency effect". The syntax is legal, but it reminds the user to check their intent.

## Trade-offs

### Advantages

1. **Syntactic orthogonality**: `spawn` + any control flow = natural concurrency combination
2. **Eliminates special cases**: removes `Expr::SpawnFor` and related special handling code
3. **Extensibility**: future new control flow structures automatically combine with `spawn`, no need
   to modify spawn logic

### Disadvantages

1. **Breaking change**: internal AST/IR representation changes, all code consuming `Expr::SpawnFor`
   needs to be updated
2. **Proof pipeline needs adaptation**: after removing `SpawnFor`, the proof pipeline dispatches via
   AST (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is completed within
   this RFC's scope through the DAG unified entry

## Alternatives

| Approach                                                                  | Why not chosen                                                                                                       |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Keep `spawn for` as independent syntax                                    | Breaks orthogonality, becomes the only keyword combination special case in the language                              |
| `spawn` only modifies `{}`, data parallel via standard library `par_iter` | Language primitive capability sinks into library, losing compiler-level DAG analysis and resource conflict detection |

## Computational Structure Types (Deferred to Separate RFC)

After this RFC removes `SpawnFor`, spawn's proof pipeline integration faces an architectural
problem: the proof pipeline works at the type level and needs to know the computational structure
inside spawn (For/While/Block/If/Call) to choose the correct proof strategy. The current proof
pipeline dispatches via AST, but the long-term direction is to encode computational structure as
`MonoType` variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), so the pipeline works
entirely at the type level.

This is a weakened practical version of
[RFC-019: Type-level Homoiconicity](../deprecated/019-typed-homoiconicity.md) — the compiler's
built-in computational structures enter the type system, but user-defined syntax is not opened up.
The theoretical basis is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP
2021): `Spawn<T>` corresponds to the modal operator `□`, and the proof pipeline corresponds to the
handler.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can
enter the type system through the same pattern. spawn is the first consumer, not the only one.

> **A separate RFC will define**: the complete semantics of the 6 MonoType variants, the type
> checker adaptation strategy, the unified interface for the proof pipeline to dispatch by type, and
> the integration plan with RFC-027.

## Implementation Strategy

### Phase Division

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **DAG analysis unification**: merge entries, unify `TaskSource` enum. Single-task spawn
   (`spawn call(x)`, `spawn 42`) produces compile warnings
3. **IR / Codegen adaptation**: remove `Ir::SpawnFor`, unify processing paths
4. **Placement simplification**: remove `SpawnFor` branch
5. **Test verification**: all existing `spawn for` tests pass

### Impact Scope

| File/Directory                               | Changes                                                  |
| -------------------------------------------- | -------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body changed to `Box<Expr>`, remove `SpawnFor`   |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` handler simplified to general expression parsing |
| `frontend/core/spawn/analysis.rs`            | Unified entry, `TaskSource` merges Explicit + Iterate    |
| `frontend/core/spawn/placement.rs`           | Remove `SpawnFor` branch                                 |
| `middle/core/ir.rs`                          | Remove `Ir::SpawnFor`                                    |
| `middle/` (IR gen, codegen)                  | Unify spawn path                                         |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verify pass                         |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — foundation of syntax unification

## Design Decision Records

| Decision                      | Decision                                              | Reason                                                                                                           | Date       |
| ----------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------- |
| spawn modification scope      | Any expression                                        | Eliminate `spawn for` special case                                                                               | 2026-06-16 |
| `spawn while` support         | Supported                                             | Syntactic orthogonality, low implementation cost. Proof pipeline may reject cross-iteration dependency use cases | 2026-06-16 |
| `spawn if` semantics          | Modifies the whole if-else                            | Distinguish from `if spawn { }`                                                                                  | 2026-06-16 |
| spawn binding precedence      | Lowest (same as return)                               | Consumes the entire following expression                                                                         | 2026-06-16 |
| DAG for inside `for`          | Does not expand sub-expressions inside `for`          | Direct sub-expression rules unchanged, `for` as a whole is a single task source                                  | 2026-06-16 |
| Single-task spawn warning     | `spawn call(x)` / `spawn 42` produces compile warning | No concurrency effect, remind user to check intent                                                               | 2026-08-19 |
| Computational structure types | Deferred to separate RFC                              | General mechanism, not limited to spawn. ECMTT theoretical basis                                                 | 2026-08-19 |

---

## References

- [RFC-024: spawn-block-based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — theoretical basis for computational structure types
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)

---

## Lifecycle and Destination

| Status           | Location                  | Description               |
| ---------------- | ------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
