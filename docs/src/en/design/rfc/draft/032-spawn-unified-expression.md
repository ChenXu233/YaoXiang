---
title: "RFC-032: spawn Unified Expression Modifier — Eliminating the spawn for Special Case"
status: "Draft"
author: "Chenxu"
created: "2026-06-16"
updated: "2026-06-16"
---

# RFC-032: spawn Unified Expression Modifier

> **Core insight**: `spawn` should not only modify `{}` blocks. It can modify **any expression**. `spawn for` is not special syntax — it is the natural combination of `spawn` + `for` expression.

## Summary

Extend `spawn` from `spawn { }` (modifying blocks only) to `spawn <expr>` (modifying any expression). `Expr::SpawnFor` is removed from the AST, naturally replaced by `Expr::Spawn { body: Expr::For { .. } }`. The types of expression structures (Block, For, While, If, etc.) enter the type system as new `MonoType` variants. `Spawn<T>` wraps the computation structure executed concurrently, is marked at compile-time, and is erased after type checking.

## Motivation

### Why is this change needed?

The current `spawn for x in items { body }` is an independent keyword combination, and the AST has `Expr::SpawnFor` dedicated to representing it. This breaks the language's orthogonality:

1. **Inconsistent syntax**: `spawn` can only modify `{}` blocks, and `spawn for` is a hardcoded exception
2. **Lack of orthogonality**: Combinations like `spawn while`, `spawn if` cannot be expressed naturally
3. **Incomplete type system**: `spawn` is invisible in the type system, so the concurrent structure cannot be obtained through type reflection

### Current problems

```rust
// Two spawn variants in the AST
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType only has value types, no computation structure types
// spawn { a, b } type = Tuple(T_a, T_b)  ← loses the "this is spawn" information
// spawn for    type = List(T)             ← loses the "this is data-parallel" information
```

## Proposal

### Core design

`spawn <expr>`: `spawn` modifies any expression. The shape of the expression determines how the DAG decomposes tasks.

**Everything is a type**: `MonoType` is extended from "value types" to "value types + computation structure types". Each key expression structure has a corresponding type variant in the type system. `Spawn<T>` wraps the computation structure executed concurrently.

### User mental model

`spawn` = "take this expression and run it concurrently". The shape of the expression determines how to split it:

| Expression shape | Concurrent behavior | Type |
|-----------------|--------------------|-----|
| `spawn { a, b, c }` | `a`, `b`, `c` run independently in parallel | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N iterations run independently in parallel | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | Each iteration is an independent task | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | The selected branch as a whole is the spawn domain | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | The call itself as one task | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | A single task | `Spawn(Int)` |

The compiler is responsible for DAG analysis to determine dependencies, and the runtime schedules according to the GMP model — tasks with no dependencies are thrown into the work queue, and workers grab them to run. Overall synchronization blocks, waiting for all tasks to complete.

**Difference from Go**: Go's `go` is "fire and forget", while YaoXiang's `spawn` is "split and run in parallel, then proceed after all are done".

### Control flow orthogonality

| Combination | Semantics | Difference |
|------------|-----------|------------|
| `spawn for x in items { body }` | Data parallel: each iteration = independent task | DAG analyzes dependencies across iterations |
| `for x in items spawn { body }` | Each iteration creates a spawn domain | Does not analyze across iterations |
| `spawn while cond { body }` | Conditional parallel: each iteration = independent task | Inter-iteration dependencies guaranteed by condition |
| `while cond spawn { body }` | Each iteration creates a spawn domain | Different semantics from the above, but no special handling needed |
| `spawn if c { a } else { b }` | The whole if-else is one spawn domain | Branch is selected by condition at execution time |
| `if c spawn { a } else { b }` | Only single branch spawns | if expression internally wraps spawn |

### Complexity eliminated

- ❌ `Expr::SpawnFor` removed from the AST
- ❌ `SpawnForAnalysis` removed from DAG analysis
- ❌ `spawn for` no longer specially handled as a combined keyword in the Parser
- ❌ `Ir::SpawnFor` removed from the IR

## Detailed Design

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

`Expr::SpawnFor` is deleted. The AST representation of `spawn for x in items { body }` is:

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

| Syntax | AST structure |
|--------|--------------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

The two have different semantics but are both natural combinations, requiring no special rules.

### 2. Parser layer

`spawn` has the lowest binding precedence (same as `return`), consuming the entire subsequent expression:

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser changes: in `pratt/nud.rs`, `spawn` no longer requires `{`, but calls the general expression parser:

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` is no longer handled as a combined keyword — `for` is handled by the general expression parser to produce `Expr::For`, and `spawn` is only responsible for wrapping.

### 3. Type system

**New `MonoType` variants:**

```rust
// ========== Computation structure types ==========

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

/// spawn concurrent wrapper: the inner expression is executed concurrently
/// Marked at compile-time, erased after type checking
Spawn(Box<MonoType>),
```

**Type inference rules**: Each expression's type inference returns a "computation structure type". Without `Spawn` wrapping = sequential execution, with `Spawn` wrapping = concurrent execution. After type checking, `Spawn` is erased, and the type degrades to the inner value type.

**Type checking flow**:
1. Infer the type T of the body expression (computation structure type)
2. If wrapped in spawn, wrap as `Spawn(T)`
3. Decompose on assignment inference: `results: List(Data) = spawn for ... {}` — extract `List(Data)` from `Spawn(ForExpr { body_ty: List(Data) })`

`Spawn<T>` is erased after type checking; the runtime does not need to know whether data comes from concurrent or sequential execution. But compile-time reflection (`type_of(x)`) can obtain the complete concurrent topology.

### 4. DAG analysis layer

The current two entry points are merged into one:

```rust
/// Unified entry point: dispatch based on body expression kind
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

`SpawnForAnalysis` struct is deleted.

| Body kind | How to decompose into tasks |
|-----------|----------------------------|
| `Expr::Block` | Direct subexpressions → task list |
| `Expr::For` | Each iteration → one task (data parallel) |
| `Expr::While` | Each iteration → one task |
| `Expr::If` | Selected branch as a whole → one task |
| `Expr::Call` / other | Expression itself → one task |

After DAG analysis is complete, the runtime schedules according to the GMP model — tasks with no dependencies are thrown into the work queue, and workers grab them to run.

### 5. IR / Codegen layer

`Ir::SpawnFor` is removed. Unified as `Ir::Spawn`, carrying `TaskSource` information.

HIR → IR translation generates runtime calls based on `SpawnAnalysis.source`:
- `TaskSource::Explicit(tasks)` → task list known at compile time
- `TaskSource::Iterate { .. }` → expanded at runtime (compiler-driven, similar to par_iter but zero-cost)

### 6. Placement layer

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

### 7. Backward compatibility

The semantics of existing `spawn for` code remain unchanged. The Parser automatically parses `spawn for x in items { body }` as `Expr::Spawn { body: Expr::For }`. Internal representation changes, but user-visible behavior is unchanged.

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

1. **Syntactic orthogonality**: `spawn` + any control flow = natural concurrent combination
2. **Everything is a type**: The type system fully records computation structures, and compile-time reflection obtains concurrent topology
3. **Eliminates special cases**: Removes `Expr::SpawnFor` and related special handling code
4. **Extensibility**: Future new control flow structures automatically combine with `spawn`, no need to modify spawn logic

### Disadvantages

1. **Type system bloat**: 6 new `MonoType` variants, increasing type checking complexity
2. **Breaking change**: Internal AST/IR representation changes, requiring updates to all code consuming `Expr::SpawnFor`
3. **Expression type inference**: Each expression now needs to return a computation structure type, with a large impact surface

## Alternatives

| Alternative | Why not chosen |
|------------|---------------|
| Keep `spawn for` as independent syntax | Breaks orthogonality, becomes the only keyword combination special case in the language |
| `spawn` only modifies `{}`, data parallelism via standard library `par_iter` | Language primitive capabilities sink into libraries, losing compiler-level DAG analysis and resource conflict detection |
| Only remove `SpawnFor` but don't introduce computation structure types in the type system | Type system loses reflection ability, spawn is invisible at the type level |

## Implementation Strategy

### Phases

1. **AST + Parser**: `Spawn { body: Box<Expr> }`, remove `SpawnFor`
2. **Type system**: Add 6 `MonoType` variants, all expression type inference returns computation structure type
3. **DAG analysis unification**: Merge entry points, unify `TaskSource` enum
4. **IR / Codegen adaptation**: Remove `Ir::SpawnFor`, unify handling path
5. **Placement simplification**: Remove `SpawnFor` branch
6. **Test verification**: All existing `spawn for` tests pass

### Impact scope

| File/Directory | Changes |
|----------------|---------|
| `frontend/core/parser/ast.rs` | `Spawn` body changes to `Box<Expr>`, remove `SpawnFor` |
| `frontend/core/parser/pratt/nud.rs` | `spawn` handler simplified to general expression parsing |
| `frontend/core/types/mono.rs` | Add `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` variants |
| `frontend/core/spawn/analysis.rs` | Unified entry point, `TaskSource` merges Explicit + Iterate |
| `frontend/core/spawn/placement.rs` | Remove `SpawnFor` branch |
| `frontend/core/typecheck/` | All expression nodes adapt to computation structure type inference |
| `middle/core/ir.rs` | Remove `Ir::SpawnFor` |
| `middle/` (IR gen, codegen) | Unify spawn path, erase Spawn type |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | Semantics unchanged, verify pass |

### Dependencies

- RFC-024 (spawn block-based concurrency model) — This RFC is its orthogonal extension
- RFC-010 (unified type syntax) — Foundation for type system changes

## Design Decision Records

| Decision | Resolution | Reason | Date |
|----------|-----------|--------|------|
| spawn modification scope | Any expression | Eliminate the `spawn for` special case | 2026-06-16 |
| `spawn while` support | Supported | Syntactic orthogonality, low implementation cost | 2026-06-16 |
| `spawn if` semantics | Modify the whole if-else | Distinguish from `if spawn { }` | 2026-06-16 |
| Type system | Introduce computation structure types | "Everything is a type", support compile-time reflection | 2026-06-16 |
| spawn type erasure | Erase after type checking | Runtime does not need concurrent structure information | 2026-06-16 |
| spawn binding precedence | Lowest (same as return) | Consumes the entire subsequent expression | 2026-06-16 |
| DAG on for internals | Do not expand for internal subexpressions | Direct subexpression rules unchanged, for as a whole is one task source | 2026-06-16 |

---

## References

- [RFC-024: Spawn Block-Based Concurrency Model](./024-concurrency-model.md)
- [Concurrency Model Specification](../../reference/language-spec/concurrency.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [spawn for Orthogonality Pending (Discussion Draft)](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## Lifecycle and Destination

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/draft/` | Author's draft |