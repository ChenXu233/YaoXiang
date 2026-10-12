---
title: 'RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn Unified Expression Modifier

> **This document defines the syntax and AST/IR refactor for `spawn`.** Runtime behavior semantics
> (task decomposition granularity, ownership, scope, error propagation, resource types, nesting) are
> covered in
> [RFC-024: Concurrency Runtime Semantics Based on spawn](../accepted/024-concurrency-model.md).
>
> The two RFCs together define `spawn` — 024 answers "what it does", 032 answers "how it is
> represented".

> **Core insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**.
> `spawn for` is not special syntax — it is the natural composition of `spawn` + `for` expression.

## Summary

Extend `spawn` from `spawn { }` (modifying only blocks) to `spawn <expr>` (modifying any
expression). `Expr::SpawnFor` is removed from the AST and naturally replaced by
`Expr::Spawn { body: Expr::For { .. } }`. This RFC only performs AST/IR/Parser cleanup; it does not
involve changes to the type system.

> **Computational structure type (`MonoType` extension) is deferred to a separate RFC.** After this
> RFC removes the `SpawnFor` special case, the proof pipeline integration of `spawn` requires the
> type system to be aware of computational structure — this is a general mechanism, not limited to
> spawn, and deserves an independent design.

## Motivation

### Why is this change needed?

Currently `spawn for x in items { body }` is an independent keyword combination, and there is a
dedicated `Expr::SpawnFor` in the AST to represent it. This breaks the orthogonality of the
language:

1. **Syntax not unified**: `spawn` can only modify `{}` blocks; `spawn for` is a hard-coded
   exception
2. **Missing orthogonality**: Combinations such as `spawn while`, `spawn if` cannot be expressed
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

`spawn` = "Take this expression and run it concurrently". The shape of the expression determines how
it is split:

| Expression Shape                | Concurrent Behavior                                |
| ------------------------------- | -------------------------------------------------- |
| `spawn { a, b, c }`             | `a`, `b`, `c` run in parallel independently        |
| `spawn for x in items { f(x) }` | N iterations run in parallel independently         |
| `spawn while cond { step() }`   | Each iteration is an independent task              |
| `spawn if c { a } else { b }`   | The selected branch is the spawn domain as a whole |
| `spawn call(x)`                 | The call itself is one task                        |
| `spawn 42`                      | A single task                                      |

The compiler is responsible for DAG analysis to determine dependencies; the runtime schedules
according to the GMP model — tasks without dependencies are thrown into the work queue, and workers
race to run them. Overall synchronous blocking, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget", while YaoXiang's `spawn` is "split and run
in parallel, wait until all are done before continuing".

### Control Flow Orthogonality

| Combination                     | Semantics                                               | Difference                                                     |
| ------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------- |
| `spawn for x in items { body }` | Data parallel: each iteration = independent task        | DAG analyzes dependencies across iterations                    |
| `for x in items spawn { body }` | Each iteration creates a spawn domain                   | No cross-iteration analysis                                    |
| `spawn while cond { body }`     | Conditional parallel: each iteration = independent task | Dependencies between iterations guaranteed by condition        |
| `while cond spawn { body }`     | Each iteration creates a spawn domain                   | Different semantics from above, but no special handling needed |
| `spawn if c { a } else { b }`   | The entire if-else is one spawn domain                  | Selected branch is executed at runtime                         |
| `if c spawn { a } else { b }`   | Only one branch is spawn                                | spawn wrapped inside the if expression                         |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` removed from the AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer special-cased in the Parser as a combined keyword
- ❌ `Ir::SpawnFor` removed from the IR

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

`Expr::SpawnFor` is removed. The AST representation of `spawn for x in items { body }`:

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

**IF Special Case**:

| Syntax                           | AST Structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

The two have different semantics, but both are natural combinations and require no special rules.

### 2. Parser Layer

`spawn` has the lowest binding precedence (same as `return`), consuming the entire following
expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser change: in `pratt/nud.rs`, `spawn` no longer requires `{`, but instead calls the general
expression parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is handled by the general expression
parser to produce `Expr::For`, and `spawn` only handles wrapping.

### 3. DAG Analysis Layer

The two current entries are merged into one:

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

**Unified Result Structure**:

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — N direct sub-expressions known at compile-time
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

| body Kind            | How it is decomposed into tasks           |
| -------------------- | ----------------------------------------- |
| `Expr::Block`        | Direct sub-expressions → task list        |
| `Expr::For`          | Each iteration → one task (data parallel) |
| `Expr::While`        | Each iteration → one task                 |
| `Expr::If`           | Selected branch as a whole → one task     |
| `Expr::Call` / other | The expression itself → one task          |

After DAG analysis is complete, the runtime schedules according to the GMP model — tasks without
dependencies are thrown into the work queue, and workers race to run them.

### 4. IR / Codegen Layer

`Ir::SpawnFor` is removed. Unified into `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:

- `TaskSource::Explicit(tasks)` → Compile-time known task list
- `TaskSource::Iterate { .. }` → Runtime expansion (compiler-driven, like par_iter but zero-cost)

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

Existing `spawn for` code semantics are unchanged; the Parser automatically parses
`spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. The internal representation
changes, but user-visible behavior does not.

New syntax naturally obtained:

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
concurrent effect". The syntax is legal, but reminds the user to check their intent.

## Trade-offs

### Advantages

1. **Syntax orthogonality**: `spawn` + any control flow = natural concurrent composition
2. **Eliminates special cases**: Removes `Expr::SpawnFor` and related special handling code
3. **Extensible**: Future new control flow structures will automatically compose with `spawn`, no
   need to modify spawn logic

### Disadvantages

1. **Breaking change**: Internal AST/IR representation changes, all code consuming `Expr::SpawnFor`
   needs to be updated
2. **Proof pipeline adaptation required**: After removing `SpawnFor`, the proof pipeline dispatches
   via AST (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is completed
   within this RFC's scope through the unified DAG entry point

## Alternatives

| Alternative                                                               | Why not chosen                                                                                                     |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Keep `spawn for` as independent syntax                                    | Breaks orthogonality, becomes the only special keyword combination in the language                                 |
| `spawn` only modifies `{}`, data parallel via standard library `par_iter` | Language primitive capability sinks to library, losing compiler-level DAG analysis and resource conflict detection |

## Computational Structure Type (Deferred to Separate RFC)

After this RFC removes `SpawnFor`, the proof pipeline integration of `spawn` faces an architectural
problem: the proof pipeline operates at the type layer and needs to know the computational structure
inside spawn (For/While/Block/If/Call) in order to select the correct proof strategy. The current
proof pipeline dispatches via AST, but the long-term direction is to encode computational structure
as `MonoType` variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), making the pipeline
work entirely at the type layer.

This is a weakened practical version of
[RFC-019: Type-Level Homoiconicity](../deprecated/019-typed-homoiconicity.md) — the compiler's
built-in computational structure enters the type system, but user-defined syntax is not opened up.
The theoretical foundation is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP
2021): `Spawn<T>` corresponds to the modal operator `□`, and the proof pipeline corresponds to the
handler.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can
enter the type system through the same pattern. spawn is the first consumer, not the only one.

> **The separate RFC will define**: Complete semantics of the 6 MonoType variants, type checker
> adaptation strategy, unified interface for the proof pipeline dispatching by type, and the
> integration plan with RFC-027.

## Implementation Strategy

### Phased Plan

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **Unified DAG analysis**: Merge entry points, unify `TaskSource` enum. Single-task spawn
   (`spawn call(x)`, `spawn 42`) produces compile warnings
3. **IR / Codegen adaptation**: Remove `Ir::SpawnFor`, unify processing path
4. **Placement simplification**: Remove `SpawnFor` branch
5. **Test verification**: All existing `spawn for` tests pass

### Impact Scope

| File/Directory                               | Change                                                   |
| -------------------------------------------- | -------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body changed to `Box<Expr>`, remove `SpawnFor`   |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` handler simplified to general expression parsing |
| `frontend/core/spawn/analysis.rs`            | Unified entry, `TaskSource` merges Explicit + Iterate    |
| `frontend/core/spawn/placement.rs`           | Remove `SpawnFor` branch                                 |
| `middle/core/ir.rs`                          | Remove `Ir::SpawnFor`                                    |
| `middle/` (IR gen, codegen)                  | Unify spawn path                                         |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verified to pass                    |

### Dependencies

- RFC-024 (spawn block concurrency model) — This RFC is its orthogonality extension
- RFC-010 (unified type syntax) — Foundation for syntax unification

## Design Decision Records

| Decision                     | Determination                                         | Reason                                                                                                        | Date       |
| ---------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------- |
| spawn modification scope     | Any expression                                        | Eliminate `spawn for` special case                                                                            | 2026-06-16 |
| `spawn while` support        | Supported                                             | Syntax orthogonality, low implementation cost. Proof pipeline may reject cross-iteration dependency use cases | 2026-06-16 |
| `spawn if` semantics         | Modifies the entire if-else                           | Distinguish from `if spawn { }`                                                                               | 2026-06-16 |
| spawn binding precedence     | Lowest (same as return)                               | Consumes the entire following expression                                                                      | 2026-06-16 |
| DAG for inside of for        | Does not expand sub-expressions inside for            | Direct sub-expression rules unchanged, for as a whole is one task source                                      | 2026-06-16 |
| Single-task spawn warning    | `spawn call(x)` / `spawn 42` produces compile warning | No concurrent effect, reminds user to check intent                                                            | 2026-08-19 |
| Computational structure type | Deferred to separate RFC                              | General mechanism, not limited to spawn. ECMTT theoretical foundation                                         | 2026-08-19 |

---

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — Theoretical foundation for computational structure types
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)

---

## Lifecycle and Destination

| Status           | Location                  | Description               |
| ---------------- | ------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
