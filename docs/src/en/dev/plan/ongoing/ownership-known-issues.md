# Known Issues in Ownership Checking

> Last updated: 2026-06-16
> Implementation location: `src/frontend/core/typecheck/layers/ownership.rs`
> Test location: `src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests, 0 failures

## Correctness Defects

- [x] ### 1. ref alias entering spawn fails to be flagged as escape (P0) — Fixed (2026-06-15)

**Scenario**:
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → escape not flagged → chooses Rc (non-atomic, unsafe across threads)
```

**Root cause**: `OwnershipChecker` only tracks variable names directly assigned from `Expr::Ref` (`ref_vars`). When a ref variable is Moved to an intermediate variable, the intermediate variable does not update `ref_vars`.

**Impact**: A ref used across spawn may be incorrectly compiled as `RcNew`; non-atomic reference counting can cause data races across threads.

**Fix**: In the handlers for `StmtKind::Var` and `BinOp::Assign`, when the right-hand side is `Expr::Var(name)` and `name ∈ ref_vars`, the left-hand side target is added to `ref_vars` (commit `9029d5b`).

- [x] ### 2. spawn-captured variable remains usable in outer scope after Move (P1) — Fixed (2026-06-16)

**Scenario**:
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn body walk (save/restore) → shared Moved inside body → outer restored
use(shared)            // outer shared still Alive — correct, but shared is already Moved inside spawn
```

**Root cause**: `Expr::Spawn` uses save/restore, so ownership changes inside the spawn body do not affect the outer scope. This is correct design, but the Move of `shared` via `a = shared` inside spawn is only detected in the spawn's "temporary walk". If the spawn body performs a Move of `shared`, save/restore restores the outer state, **yet nothing prevents the outer scope from continuing to use `shared` after spawn**.

**Impact**: If the spawn at runtime Moves `shared` (e.g., `a = shared`), outer code can still access `shared` after spawn — this may be correct under YaoXiang's concurrency model (spawn obtains an independent copy), but the semantics are not clearly defined.

**Fix direction**: The language specification needs to clarify whether the Move semantics of a spawn capture affect the outer scope. If it is "spawn obtains an independent copy", the current behavior is correct. If it is "spawn consumes ownership", the save/restore must be removed or a Captures mechanism similar to closures must be introduced.

## Precision Trade-offs

- [ ] ### 3. Conservative conflict reporting on mutually exclusive branches (P1)

**Scenario**:
```yaoxiang
if cond {
    a = &mut x   // branch A
} else {
    b = &mut x   // branch B
}
// Theoretically: A and B are mutually exclusive, should not conflict
// In practice: two WriteTokens created sequentially → reports BorrowConflict
```

**Root cause**: Architectural limitation of `NLL without fixpoint` — a single-pass AST walk does not model path conditions, so it cannot distinguish mutually exclusive branches from sequential execution.

**Fix direction**: Requires CFG's SMT slow-path involvement (currently `smt_cut` is implemented but only triggers in `while + path_condition` scenarios). Extending to if/else branches requires propagating path_condition to the Borrow handler.

- [ ] ### 4. ref type not recognized as Dup (P1)

**Scenario**:
```yaoxiang
shared = ref x
a = shared    // Move — but ref is theoretically a Dup type, should be copyable
b = shared    // use after move — should actually be allowed
```

**Root cause**: The ownership checker does not know that `ref T` is a Dup type (a copyable reference-counted handle). The Move logic in `StmtKind::Var` treats all types the same.

**Impact**: The semantics of ref values are stricter than expected — they cannot be "freely copied" as designed in RFC-009.

**Fix direction**: Requires querying the variable type from `TypeEnvironment` and skipping the Move logic for Dup types. This is consistent with the overall design that requires an explicit `clone()` call — the current conservative behavior is no looser than the correct semantics.

## Infrastructure

- [ ] ### 5. Error code format not unified (P2)

**Description**: The frontend ownership checker uses `DisproofModel.into_diagnostic()` → error codes E2014–E2020. The legacy `lifetime/error.rs` in the middle layer uses an independent `ValueState` + `Checker` trait. The two systems currently coexist.

**Fix direction**: Remove the `ValueState` and `Checker` trait from the middle layer's `error.rs` (only 2 references remain: `lifecycle.rs` and the `cycle_check` test), and unify to the frontend error code system.

- [ ] ### 6. Nested functions with parameters are not analyzed (P2)

**Description**: `StmtKind::Binding` only performs capture analysis on closures with `params.is_empty() && !body.is_empty()`. Nested functions with parameters return `vec![]` (the body is independently checked by `check_module`, but capture semantics are not analyzed).

**Impact**: Ownership errors in the body of a nested function with parameters will not be detected (currently skipped directly), and no capture information is produced. If a nested function with parameters uses an outer variable, its ownership semantics are not analyzed.

**Fix direction**: Unify the handling of parameterized and parameterless Bindings, performing both `check_function` and capture analysis on their bodies.