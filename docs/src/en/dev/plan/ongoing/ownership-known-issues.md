# Known Issues in Ownership Checking

> Last updated: 2026-06-16
> Implementation location: `src/frontend/core/typecheck/layers/ownership.rs`
> Test location: `src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests, 0 failures

## Correctness Defects

- [x] ### 1. ref alias entering spawn fails to mark escape (P0) — Fixed (2026-06-15)

**Scenario**:
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → escape not marked → Rc selected (non-atomic, unsafe across threads)
```

**Root cause**: `OwnershipChecker` only tracks variable names directly assigned by `Expr::Ref` (`ref_vars`). When a ref variable is Moved to an intermediate variable, the intermediate variable does not modify `ref_vars`.

**Impact**: refs used across spawn may be incorrectly compiled to `RcNew`; the non-atomic reference count may cause data races across threads.

**Fix**: In the `StmtKind::Var` and `BinOp::Assign` handlers, when the right-hand side is `Expr::Var(name)` and `name ∈ ref_vars`, the left-hand side target is added to `ref_vars` (commit `9029d5b`).

- [x] ### 2. After spawn captures variable Move, outer scope remains usable (P1) — Fixed (2026-06-16)

**Scenario**:
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn body walk (save/restore) → shared is Moved in body → outer restored
use(shared)            // outer shared is still Alive—correct, but shared is already Moved in spawn body
```

**Root cause**: `Expr::Spawn` uses save/restore, so ownership changes inside the spawn body do not affect the outer scope. This is correct design, but the Move of `shared` in `a = shared` inside the spawn body is only detected in the spawn's "temporary walk". If the spawn body performs a Move of `shared`, save/restore restores the outer state, **but nothing prevents the outer scope from continuing to use `shared` after spawn**.

**Impact**: If the spawn at runtime actually Moves `shared` (e.g., `a = shared`), the outer code can still access `shared` after spawn—this may be correct under YaoXiang's concurrency model (spawn obtains an independent copy), but the semantics are not explicitly defined.

**Fix direction**: The language specification needs to clarify whether the Move semantics of spawn captures affect the outer scope. If "spawn obtains an independent copy," the current behavior is correct. If "spawn consumes ownership," the save/restore must be removed or a Captures mechanism similar to closures introduced.

## Precision Trade-offs

- [x] ### 3. Conservative conflict reporting on branch mutual exclusivity (P1) — Fixed (2026-06-16)

**Scenario**:
```yaoxiang
if cond {
    a = &mut x   // branch A
} else {
    b = &mut x   // branch B
}
// Theoretically: A and B are mutually exclusive, should not conflict
// Actual: two WriteTokens created sequentially → BorrowConflict reported
```

**Root cause**: Architectural limitation of `NLL without fixpoint`—a single-pass AST walk cannot model path conditions, so it cannot distinguish branch mutual exclusivity from sequential execution.

**Fix direction**: Requires the slow SMT channel of CFG to intervene (currently `smt_cut` is implemented but only triggers in `while + path_condition` scenarios). Extending to if/else branches requires propagating `path_condition` to the Borrow handler.

- [ ] ### 4. ref type does not recognize Dup (P1)

**Scenario**:
```yaoxiang
shared = ref x
a = shared    // Move—but ref is theoretically a Dup type and should be copyable
b = shared    // use after move—should actually be allowed
```

**Root cause**: The ownership checker does not know that `ref T` is a Dup type (copyable reference-counted handle). The Move logic in `StmtKind::Var` treats all types uniformly.

**Impact**: The semantics of ref values are stricter than expected—cannot "freely copy" as designed in RFC-009.

**Fix direction**: Need to query variable types from `TypeEnvironment` and skip Move logic for Dup types. This is consistent with the overall design that requires explicit `clone()` calls—the current conservative behavior is not looser than the correct semantics.

## Infrastructure

- [ ] ### 5. Error code format not unified (P2)

**Description**: The frontend ownership checker uses `DisproofModel.into_diagnostic()` → error codes E2014-E2020. The legacy `lifetime/error.rs` in the middle layer uses an independent `ValueState` + `Checker` trait. The two systems currently coexist.

**Fix direction**: Remove the `ValueState` and `Checker` trait in the middle layer's `error.rs` (only 2 remaining references: `lifecycle.rs` and `cycle_check` test), and unify to the frontend error code system.

- [ ] ### 6. Parameterized form of nested functions not analyzed (P2)

**Description**: `StmtKind::Binding` only performs capture analysis on closures where `params.is_empty() && !body.is_empty()`. Parameterized nested functions return `vec![]` (the body is independently checked by `check_module`, but capture semantics are not analyzed).

**Impact**: Ownership errors inside parameterized nested function bodies are not detected (currently skipped directly), nor is capture information produced. If a parameterized nested function uses outer-scope variables, its ownership semantics are not analyzed.

**Fix direction**: Unify the handling of parameterized/non-parameterized Binding, and apply both `check_function` and capture analysis to their bodies.