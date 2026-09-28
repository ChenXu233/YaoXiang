---
title: 'RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case'
status: 'Under Review'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn Unified Expression Modifier

> **This document defines the syntax and AST/IR restructuring of `spawn`**. Runtime behavior
> semantics (task decomposition granularity, ownership, scope, error propagation, resource types,
> nesting) are described in
> [RFC-024: Concurrency Runtime Semantics Based on spawn](../accepted/024-concurrency-model.md).
>
> The two RFCs jointly define `spawn` — RFC-024 answers "what to do", RFC-032 answers "how to
> represent it".

> **Core insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**.
> `spawn for` is not special syntax — it is the natural composition of `spawn` + a `for` expression.

## Summary

Extend `spawn` from `spawn { }` (modifying only blocks) to `spawn <expr>` (modifying any
expression). `Expr::SpawnFor` is removed from the AST and naturally replaced by
`Expr::Spawn { body: Expr::For { .. } }`. This RFC only performs AST/IR/Parser cleanup and does not
involve type system changes.

> **Computation Structure Types (`MonoType` extensions) are deferred to a separate RFC.** After this
> RFC removes the `SpawnFor` special case, the proof pipeline integration of `spawn` requires the
> type system to be aware of computation structure — this is a general mechanism, not limited to
> spawn, and deserves independent design.

## Motivation

### Why is this change needed?

Currently, `spawn for x in items { body }` is an independent keyword combination, with
`Expr::SpawnFor` in the AST dedicated to representing it. This breaks the orthogonality of the
language:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks, and `spawn for` is a hardcoded
   exception
2. **Lack of orthogonality**: combinations like `spawn while`, `spawn if` cannot be naturally
   expressed

### Current Issues

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
it is decomposed:

| Expression shape                | Concurrent behavior                     |
| ------------------------------- | --------------------------------------- |
| `spawn { a, b, c }`             | `a`, `b`, `c` run independently         |
| `spawn for x in items { f(x) }` | N iterations run independently          |
| `spawn while cond { step() }`   | Each iteration as independent task      |
| `spawn if c { a } else { b }`   | Selected branch as a whole spawn domain |
| `spawn call(x)`                 | The call itself as one task             |
| `spawn 42`                      | A single task                           |

The compiler is responsible for DAG analysis to determine dependencies. The runtime schedules
according to the GMP model — dependency-free tasks are thrown into the work queue and workers race
to execute them. Overall synchronous blocking, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget", while YaoXiang's `spawn` is "decompose and
run in parallel, then wait for all to complete before continuing".

### Control Flow Orthogonality

| Combination                     | Semantics                                                  | Difference                                                   |
| ------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------ |
| `spawn for x in items { body }` | Data parallelism: each iteration = independent task        | DAG analyzes dependencies across iterations                  |
| `for x in items spawn { body }` | Each iteration creates a spawn domain                      | Does not analyze across iterations                           |
| `spawn while cond { body }`     | Conditional parallelism: each iteration = independent task | Inter-iteration dependencies guaranteed by condition         |
| `while cond spawn { body }`     | Each iteration creates a spawn domain                      | Different semantics from above but needs no special handling |
| `spawn if c { a } else { b }`   | The entire if-else as one spawn domain                     | Selects branch by condition at runtime                       |
| `if c spawn { a } else { b }`   | Only single branch spawns                                  | if expression wraps spawn internally                         |

### Eliminated Complexity

- ❌ `Expr::SpawnFor` removed from AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer specially handled as a combined keyword in the Parser
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

| Syntax                           | AST structure                                       |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

The two have different semantics but are both natural compositions and require no special rules.

### 2. Parser Layer

`spawn` has the lowest binding precedence (same as `return`), consuming the following entire
expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser change: in `pratt/nud.rs`, `spawn` no longer requires `{`, but calls the general expression
parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer treated as a combined keyword — `for` is handled by the general expression
parser to produce `Expr::For`, and `spawn` is only responsible for wrapping.

### 3. DAG Analysis Layer

The current two entry points are merged into one:

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

`SpawnForAnalysis` struct is removed.

| body kind             | How to decompose into tasks                  |
| --------------------- | -------------------------------------------- |
| `Expr::Block`         | Direct sub-expressions → task list           |
| `Expr::For`           | Each iteration → one task (data parallelism) |
| `Expr::While`         | Each iteration → one task                    |
| `Expr::If`            | Selected branch as a whole → one task        |
| `Expr::Call` / others | The expression itself → one task             |

After DAG analysis is complete, the runtime schedules according to the GMP model — dependency-free
tasks are thrown into the work queue and workers race to execute them.

### 4. IR / Codegen Layer

`Ir::SpawnFor` is removed. Unified as `Ir::Spawn`, carrying `TaskSource` information.

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

The semantics of existing `spawn for` code remain unchanged. The Parser automatically parses
`spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. Internal representation
changes, user-visible behavior remains the same.

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

**Single-task spawn warning**: When modifying a single expression like `spawn call(x)` and
`spawn 42`, DAG analysis produces a compile warning: "spawn modifying a single expression has no
concurrent effect". The syntax is valid, but the user is reminded to check intent.

## Trade-offs

### Advantages

1. **Syntactic orthogonality**: `spawn` + any control flow = natural concurrent composition
2. **Eliminates special cases**: removes `Expr::SpawnFor` and related special handling code
3. **Extensibility**: future new control flow structures automatically compose with `spawn` without
   modifying spawn logic

### Disadvantages

1. **Breaking change**: internal AST/IR representation changes, requiring updates to all code that
   consumes `Expr::SpawnFor`
2. **Proof pipeline needs adaptation**: after removing `SpawnFor`, the proof pipeline dispatches
   through AST (`match body { Expr::For => ..., Expr::While => ... }`) — this adaptation is
   completed within the scope of this RFC through a unified DAG entry point

## Alternatives

| Approach                                                                     | Why not chosen                                                                                                      |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Keep `spawn for` as independent syntax                                       | Breaks orthogonality, becomes the only keyword combination special case in the language                             |
| `spawn` only modifies `{}`, data parallelism via standard library `par_iter` | Core language capability sinks into the library, losing compiler-level DAG analysis and resource conflict detection |

## Computation Structure Types (Deferred to Separate RFC)

After this RFC removes `SpawnFor`, the proof pipeline integration of `spawn` faces an architectural
issue: the proof pipeline works at the type level and needs to know the computation structure inside
spawn (For/While/Block/If/Call) to select the correct proof strategy. Currently the proof pipeline
dispatches through AST, but the long-term direction is to encode computation structure as `MonoType`
variants (`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`), so that the pipeline works
entirely at the type level.

This is a weakened practical version of
[RFC-019: Type-Level Homoiconicity](../draft/019-typed-homoiconicity.md) — the compiler's built-in
computation structure enters the type system, but does not open user-defined syntax. The theoretical
foundation is ECMTT (Contextual Modal Types for Algebraic Effects and Handlers, ICFP 2021):
`Spawn<T>` corresponds to the modal operator `□`, and the proof pipeline corresponds to the handler.

This mechanism is not limited to spawn — any future effect (pure computation, IO, fallible) can
enter the type system through the same pattern. spawn is the first consumer, not the only consumer.

> **The separate RFC will define**: complete semantics of 6 MonoType variants, type checker
> adaptation strategy, unified interface for proof pipeline dispatch by type, and integration plan
> with RFC-027.

## Implementation Strategy

### Phased Approach

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **DAG analysis unification**: merge entry points, unify `TaskSource` enum. Single-task spawn
   (`spawn call(x)`, `spawn 42`) produces compile warnings
3. **IR / Codegen adaptation**: remove `Ir::SpawnFor`, unified handling path
4. **Placement simplification**: remove `SpawnFor` branch
5. **Test verification**: all existing `spawn for` tests pass

### Scope of Impact

| File/Directory                               | Changes                                                  |
| -------------------------------------------- | -------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body changed to `Box<Expr>`, remove `SpawnFor`   |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` handler simplified to general expression parsing |
| `frontend/core/spawn/analysis.rs`            | Unified entry, `TaskSource` merged Explicit + Iterate    |
| `frontend/core/spawn/placement.rs`           | Remove `SpawnFor` branch                                 |
| `middle/core/ir.rs`                          | Remove `Ir::SpawnFor`                                    |
| `middle/` (IR gen, codegen)                  | Unified spawn path                                       |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verify passing                      |

### Dependencies

- RFC-024 (spawn block concurrency model) — this RFC is its orthogonality extension
- RFC-010 (unified type syntax) — foundation for syntax unification

## Design Decision Records

| Decision                      | Decision                                              | Reason                                                                                                           | Date       |
| ----------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ---------- |
| spawn modification scope      | Any expression                                        | Eliminate `spawn for` special case                                                                               | 2026-06-16 |
| `spawn while` support         | Supported                                             | Syntactic orthogonality, low implementation cost. Proof pipeline may reject cross-iteration dependency use cases | 2026-06-16 |
| `spawn if` semantics          | Modifies entire if-else                               | Distinguish from `if spawn { }`                                                                                  | 2026-06-16 |
| spawn binding precedence      | Lowest (same as return)                               | Consumes the following entire expression                                                                         | 2026-06-16 |
| DAG handling of for internals | Does not expand for internal sub-expressions          | Direct sub-expression rule unchanged, for as a whole is one task source                                          | 2026-06-16 |
| Single-task spawn warning     | `spawn call(x)` / `spawn 42` produces compile warning | No concurrent effect, remind user to check intent                                                                | 2026-08-19 |
| Computation structure types   | Deferred to separate RFC                              | General mechanism, not limited to spawn. ECMTT theoretical basis                                                 | 2026-08-19 |

---

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — Theoretical foundation of computation structure types
- [Concurrency Model Specification](../../../reference/language-spec/concurrency.md)

---

## Lifecycle and Destination

| Status           | Location                  | Description               |
| ---------------- | ------------------------- | ------------------------- |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion |
