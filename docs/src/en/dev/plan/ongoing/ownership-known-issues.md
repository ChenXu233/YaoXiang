# Ownership Check Known Issues

> Last updated: 2026-06-16
> Implementation location: `src/frontend/core/typecheck/layers/ownership.rs`
> Test location: `src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests, 0 failures

## Correctness Defects

- [x] ### 1. ref alias into spawn leaves escape unmarked (P0) — Fixed (2026-06-15)

**Scenario**:
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → unmarked escape → selects Rc (non-atomic, not thread-safe)
```

**Root cause**: `OwnershipChecker` only tracks variable names directly assigned by `Expr::Ref` (`ref_vars`). When a ref variable is Moved to an intermediate variable, the intermediate variable does not update `ref_vars`.

**Impact**: refs that flow into a spawn may be incorrectly compiled as `RcNew`; the non-atomic reference count can cause data races across threads.

**Fix**: In the `StmtKind::Var` and `BinOp::Assign` handlers, when the right-hand side is `Expr::Var(name)` and `name ∈ ref_vars`, add the left-hand target to `ref_vars` (commit `9029d5b`).

- [ ] ### 2. After Move of a spawn-captured variable, the outer scope remains usable (P1 — undefined semantics)

**Scenario**:
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn body walk (save/restore) → shared Moved inside body → outer scope restored
use(shared)            // outer shared still Alive—correct, but shared is already Moved inside the spawn body
```

**Root cause**: `Expr::Spawn` uses save/restore, so ownership changes inside the spawn body do not affect the outer scope. This design is correct, but the Move of `a = shared` inside the spawn body is only detected in the spawn's "temporary walk". If the spawn body performs a Move of `shared`, save/restore restores the outer scope, **but nothing prevents the outer scope from continuing to use `shared` after the spawn**.

**Impact**: If the spawn actually Moves `shared` at runtime (e.g. `a = shared`), the outer code can still access `shared` after the spawn — this may be correct under YaoXiang's concurrency model (spawn obtains an independent copy), but the semantics are not clearly defined.

**Fix direction**: The language specification must clarify whether the Move semantics of a spawn capture affect the outer scope. If "spawn obtains an independent copy", the current behavior is correct. If "spawn consumes ownership", save/restore must be removed, or a closure-style `Captures` mechanism must be introduced.

## Precision Trade-offs

- [ ] ### 3. Mutual exclusivity of branches conservatively reported as conflict (P1)

**Scenario**:
```yaoxiang
if cond {
    a = &mut x   // branch A
} else {
    b = &mut x   // branch B
}
// Theoretically: A and B are mutually exclusive; should not conflict
// In practice: two WriteTokens are created sequentially → reports BorrowConflict
```

**Root cause**: Architectural limitation of `NLL without fixpoint` — a single-pass AST walk does not model path conditions, so it cannot distinguish branch mutual exclusivity from sequential execution.

**Fix direction**: The CFG's SMT slow path needs to be involved (currently `smt_cut` is implemented but only triggered in `while + path_condition` scenarios). Extending it to if/else branches requires propagating `path_condition` into the Borrow handler.

- [ ] ### 4. `ref` type not recognized as Dup (P1)

**Scenario**:
```yaoxiang
shared = ref x
a = shared    // Move—but ref is theoretically a Dup type and should be copyable
b = shared    // use after move—should actually be allowed
```

**Root cause**: The ownership checker does not know that `ref T` is a Dup type (a copyable, reference-counted handle). The Move logic in `StmtKind::Var` treats all types uniformly.

**Impact**: ref value semantics end up stricter than expected — they cannot be "freely copied" as specified by the RFC-009 design.

**Fix direction**: Look up the variable's type from `TypeEnvironment` and skip Move logic for Dup types. This is consistent with the overall design that requires explicit `clone()` calls — the current conservative behavior is no looser than the correct semantics.

## Infrastructure

- [ ] ### 5. Error code format not unified (P2)

**Description**: The front-end ownership checker uses `DisproofModel.into_diagnostic()` → error codes E2014–E2020. The legacy `lifetime/error.rs` in the Middle layer uses its own independent `ValueState` + `Checker` trait. The two systems currently coexist.

**Fix direction**: Remove the `ValueState` and `Checker` trait from the middle layer's `error.rs` (only 2 references remain: `lifecycle.rs` and the `cycle_check` test), and unify everything under the front-end error-code system.

- [ ] ### 6. Parameterized form of nested functions is not analyzed (P2)

**Description**: `StmtKind::Binding` only runs capture analysis on closures where `params.is_empty() && !body.is_empty()`. Parameterized nested functions return `vec![]` (the body is independently checked by `check_module`, but capture semantics are not analyzed).

**Impact**: Ownership errors inside the body of a parameterized nested function go undetected (currently skipped outright), and no capture information is produced. If a parameterized nested function uses outer-scope variables, the ownership semantics of those captures are never analyzed.

**Fix direction**: Unify the handling of parameterized and parameterless `Binding`, running both `check_function` and capture analysis on their bodies.