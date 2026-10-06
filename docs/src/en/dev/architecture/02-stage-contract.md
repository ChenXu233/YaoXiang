---
title: Compilation Stage Contract and Obligation Ledger
---

# Compilation Stage Contract and Obligation Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage ordering are in the RFC-039
> main text; the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses a
single question: **"Which stages ran in this compilation, which didn't, and why"** is currently
scattered across six paths and ten entry functions with manual wiring, and no single place can
answer it.

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are violated by the architecture, not by the logic.**

The comment in `src/frontend/core/typecheck/checker.rs:5165` reads "Unproven → compile error, no
degradation, no silent pass... do not let silent pass resurrect." But this invariant only holds on
the **single-file path**: refinement constraints of the form `x: Sorted(3)` **silently pass** under
the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP — the proof function is
never executed and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, not registered at runtime) and `StageScope`;
- `Obligations` obligation ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and per-function refactor plan for the ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and execution
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactor and the **order** in which to do it; this document gives
L1's **concrete form**, **per-file change list**, and **internal execution stages** (corresponding
to RFC-039 global sequence P3 "fix correctness bugs" and P4 "stage contract and unified Driver").
Where this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are executed according to category **C2 (orchestration changes)** in the
[Equivalence Criteria Document](07-equivalence-oracle.md): same diagnostic set at each entry + same
corpus behavior.

## Current State

> All items in this section are **verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect Where "One Place Wrong, Whole Repo Silent"

It differs in nature from the other two classes of defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation            | Is There a Signal    |
| ----------------------- | -------------------------------- | -------------------- |
| Lexical/syntactic error | Source code written incorrectly  | Diagnostic           |
| Type mismatch           | Type written incorrectly         | Diagnostic           |
| Stage wiring missed     | Some stage is never invoked      | **No signal at all** |
| Obligation not consumed | Field filled but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "more rigorous review" — code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed in RFC-039:
"this project treats 'design' as documentation convention, not as an executable constraint."

### The Repository Already Has Working Enforcement Mechanisms

The same repository already has mature enforcement mechanisms, just not yet extended to the stage
layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subjected to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, comparing each one against
  the RFC-013 code table; any inconsistency directly `panic!` and refuses to compile.
- `src/package/` (**76 files / 13012 lines**, of which the `tests/` directory has 6227 lines,
  accounting for 47.9%) has high test density, with each module's test subtree declared and wired —
  this is the most complete test wiring in the entire repo, and can serve as a form reference for
  the stage-layer gate.

**The design capability is sufficient. What is missing is "placing a gate of the same level in the
orchestration layer."**

(Line count basis: `(Get-Content).Count`, see the "Line Count Basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                              |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ---------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5-stage direct call                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                               |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + link            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics   |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)             |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR       |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1              |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                  |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                  |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1       |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded inside** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The following table gives evidence cell by cell. **A blank does not mean "this entry does not do
this thing", but "there is no code in this entry doing this thing"** — this is the very nature of
the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                       | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                  | **No**                                                       | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                  | Yes `356-377` (role-aware, no config gate)                   | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                  | Yes `336-348`                                                | **No**                             | **No**                                 |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never output**                          | Yes `350`                                                    | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                  | Yes (same as 2)                                              | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                  | N/A                                                          | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **first error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, drops `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                  | Yes (`321-328`)                                              | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                     | **No**                                                       | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                         | **No**                                                       | **No**                             | Yes (independent ModuleIR then merged) |

**Two places require precise formulation, otherwise it will be written incorrectly:**

- **The precise conclusion for row 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is in
  `compile_project:138` — `type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but this function **has no `result.warnings` read site anywhere in its body**, and
  the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **The `main` criterion for the entry in row 11 is also from different sources**:
  `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
  `is_bin_role` (`250-252`, only checks "is there a manifest"). The two functions give different
  answers to "what files must define `main`."

### Correctness Bug: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are reproducible.**

**Step 1 — The obligation is produced at one and only one place.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire
repository that constructs a non-empty `proof_calls`:

```rust
// predicate.rs:232-239
return ProofResult::Unproven {
    reason: UnprovenReason::ProofFunctionRequired,
    proof_calls: vec![ProofFunctionCall {
        func_name: func.clone(),
        args: const_args,
    }],
    budget: budget_report,
};
```

The semantics are "the predicate argument of this refinement constraint is a compile-time literal;
this proof function must actually be executed to make a determination."

**Step 2 — The consumption point is inside the checker, but only produced, never consumed.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment in `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch thus becomes a real line of defense, not a forward-looking fallback: do not let silent pass resurrect.`

That is, the author knew "not producing diagnostics" is a defect, and explicitly categorized it as
**a known intermediate state with nowhere to go**, assuming by design that "someone will definitely
come to read `proof_calls`".

**Step 3 — The field is indeed populated into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it; `checker.rs:1439` writes it into `TypeCheckResult` with
`proof_calls, // Phase 2.5 preregistered proof function obligation`. The field is defined in
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) has only one reader in production code: **`src/frontend/pipeline.rs:187`** (passes
at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repository fall into three categories, none
of which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` read `ProofResult`.)

**Step 5 — The four entries of the orchestrator all bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**; they directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they cannot even reach the only read
site at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also take the
discard path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means the **proof obligations of the embedded std module itself are
likewise not executed**.

**Step 7 — Consequence: the constraint silently passes.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and produces no diagnostics** on the
multi-file / check / LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent discard point within the layer.**
When `checker.rs:1303-1314` handles ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no ledger entry.** This directly contradicts the promise of `5164` "do not let silent
pass resurrect", and is **independent of the orchestrator issue** — even if the entry layer is
completely fixed, this point will still be silent. (Handling timing: should be processed in the same
batch as the obligation ledger, because it is the same kind of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping precisely before `pipeline.rs`**.

The test file's own documentation comment has already made this explicit.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation was filled in", while
the bug is "the consumer never read it".** A test that only tests the producer and not the consumer
is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Main Corpus is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) takes the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion** and the main acceptance
method for stage C2. But in practice:

- **There is no `yaoxiang.toml` in the `tests/` directory** (zero hits in the entire directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and all do not cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is the empirical
evidence. The file is annotated `// expect: compile-error E4018` (line 15), and its header comment
line 7 reads "Status: ❌ should be rejected at compile time" — **it passes precisely because it
takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs to add a multi-file corpus layer, otherwise it cannot
serve as the acceptance tool for this document.** See Implementation Point S1.

### Obligation Fields: 16 "Output-as-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
documentation comments explicitly state "produced by stage X → consumed by stage Y", i.e., they are
essentially **cross-stage obligations**:

| Field                      | Definition Line | Producer → Consumer                                                  |
| -------------------------- | --------------- | -------------------------------------------------------------------- |
| `proof_calls`              | `29`            | typecheck → **proof_execution**                                      |
| `release_plan`             | `31`            | ownership → IR generation (read at `ir_gen.rs:341`, used at `:1942`) |
| `escaped_refs`             | `33`            | ownership → IR generation                                            |
| `instantiation_requests`   | `35`            | typecheck → monomorphization (`pipeline.rs:381`)                     |
| `existential_coercions`    | `37`            | typecheck → IR generation                                            |
| `implementation_proofs`    | `39`            | typecheck → IR generation                                            |
| `interface_impl_registry`  | `42`            | typecheck → operator query / constraint solving / LSP                |
| `sum_types`                | `44`            | typecheck → IR generation                                            |
| `sum_type_param_names`     | `47`            | typecheck → IR generation                                            |
| `variant_ctor_calls`       | `49`            | typecheck → IR generation (span-keyed)                               |
| `operator_dispatches`      | `51`            | typecheck → IR generation (span-keyed)                               |
| `method_overload_ir_names` | `53`            | typecheck → IR generation                                            |
| `overload_resolutions`     | `56`            | typecheck → IR generation (span-keyed)                               |
| `try_expr_impls`           | `59`            | typecheck → IR generation (span-keyed)                               |
| `match_scrutinee_types`    | `63`            | typecheck → IR generation (span-keyed)                               |
| `module_namespaces`        | `66`            | typecheck → IR generation                                            |

**All these fields are passed across layers keyed by `Span` or name tables.** The Equivalence
Criteria Document has already pointed out that a span-keyed contract mismatch will "silently fail,
with no error" (`ReleasePlan` is typical: any inconsistency in how the two sides compute spans →
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer `ir_gen.rs:341` (`release_plan: type_result.release_plan.drops.clone()`)
  is inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it** — the consumer is in the downstream module, and the downstream module
  is on the necessary path of all entries.
- `proof_calls`'s consumer is at the **top level** of `pipeline.rs`, belonging to **another entry
  implementation**. The four entries of orchestrator don't even pass through `pipeline.rs` as a
  layer.

**Verified fact**: currently, the entire repository has **no mechanism** to ensure these 16 fields
are consumed. The only "protection" is that `ReleasePlan` happened to hitch a ride on IR generation.

### Four Additional Contract Defects in the Proof Layer

All below come from proof layer analysis, independent of the entry divergence problem, but all fall
under "stage contract not enforced".

**(a) The layer order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "execute in layer order; if a lower layer fails, the upper layer does not run".
Actual call sites in `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **opposite** to the declaration; the declared
Layer 0 `equivalence` **does not appear at all in the stage sequence of `check_module`** (it is only
used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time **there is no short-circuit at all** — `checker.rs:1271-1276` adds
each termination error one by one via `add_error` and continues downward, so the README's promise of
"lower layer failure prevents upper layer from running" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

| Consumption Point                                      | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Inject at construction via `with_solver_owned`           | `None` → **silently skipped** (not injected)              |
| `ownership.rs:627` (back-edge cut-off determination)   | `default_solver()` **per call**                          | `None => return false` (`629`) → conservative no-cut      |

The comment in `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure keeps
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'". Whereas the contract documented at `backend.rs:60` says "`None`: backend
unavailable... **the caller should conservatively degrade**". **Two philosophies coexist in the same
code, and the hard-failure one panics rather than returning an error.**

**(c) Each back-edge determination creates a new Z3 context; the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut-off determination. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call does `Z3Backend::new()`, i.e., creates
a new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the documentation comment
in `z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this calling pattern**.

(Comparison: the `LazyLock` in `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifecycle strategies.)

**(d) Checker silently degrades + comment does not match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call ownership table
```

When `body_checker` is `None`, the ownership check gets an **empty type ledger and empty call
table** and proceeds as usual — ownership analysis degenerates to "nothing conflicts", **without any
warning**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least leave
a trace in the obligation ledger.)

The comment in `checker.rs:1244` says termination check "runs after type check, before constraint
solving". But the actual `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at
`1256`, ownership is at `1296` after, i.e., the "before" said in the comment is actually "two layers
after"**.

### wasm Current State: Shim Crate Carries It, 27-File Branch Is Alive

The wasm target **has been built and is built in CI**, with `cdylib` not in the main crate but in an
independent shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                              |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                              |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                     |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                 |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                        |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from fixed URL, downgrade to warning if missing)                                                                                                                                                              |
| **27 files** contain the `wasm32` literal         | of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) only mention it in documentation                                                                                                                               |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                      |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                      |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call on the wasm target
— `lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (these three
all need `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Paths that consume `proof_calls`           | Whether proof function is executed |
| ------------------------------------------ | ---------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                            |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                            |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                            |
| Multi-file `run` (`compile_project`)       | **No**                             |
| `check` (`check_project`)                  | **No**                             |
| LSP in-project (`check_source_in_project`) | **No**                             |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
new `WasmPlayground` variant (see "Target Design" 3), otherwise unifying the Driver will miss this
path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — belongs to `06-cleanup-inventory.md`. The RFC-039 open issue "build wasm
target, or delete these branches" **is now obsolete**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Registered at Runtime)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is prohibited.**
The reason is that Rust's exhaustive `match` can compile-time force "new stages must be handled by
the orchestration layer" — this is exactly the mechanism that the opcode table has proven effective
(RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                   Project
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Parsing,            // Lexical + syntactic              PerModule
    Typecheck,          // Type check (including embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis        Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation           Project
    Monomorphization,   // Monomorphization                 Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Linking,            // Cross-module link / IR merge     Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at the project level) turn "which stages must be run per module, which must be run once at project
level" into a type fact. A `Project`-scope stage is structurally guaranteed by `dispatch` to run
only once — this eliminates questions like "should `allocate_global_slots` be called once per file"
in the current `orchestrator.rs` that require human reasoning.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on the order.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but not consumed" from undetectable into a compile-time or
test-time assertable fact. RFC-039 has already listed this as "the most valuable item".

```rust
// src/driver/obligations.rs
pub struct Obligations {
    pub proof_calls:             Vec<ProofFunctionCall>,     // types.rs:29
    pub release_plan:            ReleasePlan,                // types.rs:31
    pub escaped_refs:            HashSet<String>,            // types.rs:33
    pub instantiation_requests:  Vec<InstantiationRequest>,  // types.rs:35
    pub existential_coercions:   Vec<ExistentialCoercion>,   // types.rs:37
    pub overload_resolutions:    Vec<(Span, String)>,        // types.rs:56
    pub try_expr_impls:          Vec<(Span, String)>,        // types.rs:59
    // …remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                              | Disposition                                                                       |
| ------------------------------------ | ------------------------------------------------------ | --------------------------------------------------------------------------------- |
| Obligation read by declared consumer | The field in `Obligations` is `take()`/marked consumed | Pass                                                                              |
| Obligation non-empty but no consumer | Field non-empty and not consumed                       | **Produce diagnostic** (W-level by default; promoted to E-level in `strict` mode) |
| Obligation empty                     | Field is empty                                         | Pass (no consumption needed)                                                      |

**Why start with W-level instead of E-level**: fixing obligation discard will **change the
diagnostic set**. C2 criterion requires "same diagnostic set at each entry"; if we go straight to
E-level, `yaoxiang check` will suddenly have a lot of previously-silent `Unproven` diagnostics. Two
steps (W observation first, then promote to E) make every step's criteria usable. See Implementation
Point S4 for details.

**The call site of `assert_drained()` is unique**: `Driver::run` calls it at the end of the stage
table, before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third file reads it → fail). `assert_drained()` is the **runtime** gate, and that
script is the **static** gate; they are complementary, both needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, result filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`), and
does not accept a free array from the caller.** This is the key constraint to prevent the `Program`
abstraction from degrading into "free parameters that can take anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines skipping, not stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // must produce diagnostic, see below
                continue;
            }
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // the only settlement point
        Ok(state.into_result())
    }
}
```

**Refactor approach for the ten entries (function specified individually)**:

| Entry                                                      | After Refactor                                                                                                                                                                                                       |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body, change to construct `Program { kind: SingleFile, … }` and hand it to Driver                                                                                                                |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Thin down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                        |
| `check_project` (`orchestrator.rs:273-399`)                | Thin down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                          |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a single Driver call + filter result to the target file                                                                                                                                                    |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                            |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence of the single-file branch is deleted**, changes to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`), uniformly go to `Program { kind: Check }`                                                                                                |

**The overlap between `check_project` and `compile_project` (about 40% shared / 60% divergent)**.
What is shared: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parsing loop, per-file checker assembly, result processing. What is
divergent: items 2/3/4/5/7/8/9/10/11 of the 11-item table above. **This 40% shared skeleton is
exactly where `Driver`'s value lies** — the divergent 60% is all "different stage set" or "different
aggregation mode", which is exactly what the two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causing this stage to be skipped" does not belong in the
stage return value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped`, then the information "why didn't I run" gets scattered across 12 stages, and cannot be
centrally audited. Instead, `dispatch` determines at the beginning of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the bug this document is meant
to fix — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent "skip",
producing no diagnostic, only because the obligation happens to be empty. Once the rule is
established, any "because X wasn't done, so Y didn't run" must be explainable.

The diagnostic text needs to distinguish three reasons for skipping:

| Reason                                                    | Text Direction                                                                                                                             |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Upstream `Abort` caused                                   | "Stage Y did not run because upstream stage X failed"                                                                                      |
| Caused by empty precondition obligations                  | "Stage Y did not run because there were no pending obligations (normal)" — **this item has severity = Info, not counted in warning count** |
| Conditions not met (e.g., `config.mono.enabled == false`) | "Stage Y did not run because the configuration is not enabled"                                                                             |

The second category is key: it makes "normal skip" distinguishable from "abnormal skip" in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` relies on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop at first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (four early-exits at `pipeline.rs:150/162/174/193`)           | Exists                 |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                 |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent invocations; placing them on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ in more than just "early exit
or not" — they are two different checker entries. After unification, the same implementation should
be driven by the `Aggregation` parameter, rather than keeping two functions (**this is an item this
document needs to additionally verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Does not introduce new type representations, does not touch `MonoType` / `PolyType` / `ir::Type`                                           |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what is moved into the ledger is **settlement
responsibility**, not the type's ownership module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reading `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`** — this is consistent with the driver non-reverse-dependency red
line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. The presence of `Driver` in `TypeChecker`'s imports is considered a
violation, intercepted by the `scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                      |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                              |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | **Added** proof_execution; **added** warning output                                        |
| `yaoxiang check` (multi-file)       | No proof_execution                       | **Added** proof_execution                                                                  |
| LSP (in-project)                    | No proof_execution                       | **Added** proof_execution                                                                  |
| Z3 not installed + single file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**) |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above, unified                                                                     |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8 "not bound to a specific solver", and is also the contract
already declared at `backend.rs:60` ("the caller should conservatively degrade") — the current
implementation is the opposite of its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                         |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                          |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to construct `Program { kind: SingleFile }`                                                                                                 |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to construct `Program { kind: MultiFile }`                                                                                               |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                               |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                         |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                           |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` thinned down to Program constructor                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` thinned down to Program constructor                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changes to go through Driver                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changes to `Program { kind: Embedded }`                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | Either `check_module` or `check_module_collect_all` of `typecheck_with_registry_in` driven by the `Aggregation` parameter                                      |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics` manual stage sequence deleted, changed to call Driver                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` uniformly goes to `Program { kind: Check }`                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`) |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix the second silent discard point**)                                         |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment that does not match `1320`                                                                                                                     |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "if a lower layer fails, the upper layer does not run" (no short-circuit exists)               |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | `default_solver()` changed to return `&'static` singleton reference (fix creating new context per back-edge)                                                   |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse sequence
at `src/lsp/handlers/diagnostics.rs:161-227`.

**Not touched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic itself
is correct — **the problem is on the consumer side, not the producer side**; changing the producer
side would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch
(reachability determination of wasm branches belongs to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                                   | Disposition                                                                                                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: constraints that previously passed silently will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compile will start producing warnings               | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                                  |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set increases                                          | W first then E, S2/S4 in two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                              | No break                                                                                                                                             |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                                   | These two paths do not go through proof_execution                                                                                                    |
| `TypeCheckResult` fields migrate to `Obligations`         | **Internal refactor**                                                           | If the `pub` API surface has external dependencies, sync is needed; in-repo consumers are listed exhaustively in the change list                     |

---

## Implementation Points

This document corresponds to **P3 (fix correctness bugs)** and **P4 (stage contract and unified
Driver)** of the RFC-039 global stage sequence. The following S1-S5 are the implementation order
within this document, **each step's acceptance criteria reference the C2 category of the
[Equivalence Criteria Document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** at each entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Bug criteria must be written as failures first.

| Deliverable                                                                                                                                                 | Acceptance                                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                               | **Must be red**. If accidentally green, it means the bug analysis in this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (resolution D48 — not mixed into single-file corpus tree; with `yaoxiang.toml` project fixture) | The diagnostic sets of the single-file and multi-file corpora can be compared                              |
| `test_obligations_drained` skeleton                                                                                                                         | Mark `#[ignore]`, S4 turns green                                                                           |

**Rollback point**: no code changes, purely new tests.

### S2: Introduce `Stage` + `Program`, Without Changing Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code are byte-for-byte identical**                |
| Differential on all corpora (293 `.yx`)                  | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This phase deliberately does not fix any bugs** — it only moves existing behavior into the Driver.
Acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` can be restored.

### S3: Merge Four Orchestrator Entries + Fix Bugs

| Deliverable                                                                                                                     | Acceptance (C2)                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The diagnostic sets of the four entries are identical after normalization by `Aggregation` |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project                       |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                                 |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Bug fixed                                                                                  |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in any mode        |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in a Z3-less environment                                                  |

**Rollback point**: bug fix and structural merge are **two separate commits**. If the structural
merge has issues, you can only roll back the structural commit, keeping the bug fix commit — at this
time `test_multifile_proof_obligation_not_dropped` stays green; the reverse (keep the structure,
roll back the fix) will turn red, which is an unacceptable intermediate state, prohibited from being
merged.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                             |
| -------------------------------------------- | ---------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                                 |
| `scripts/ci/check-obligations.py`            | Static gate in effect                                                  |
| Obligation diagnostic W → E upgrade          | Manual review of every new E after the differential on the full corpus |

**Rollback point**: the severity of `assert_drained()` is controllable by a configuration item, and
W/E switching does not require changing code structure.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                           | Acceptance                                                                                          |
| --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                | Documentation corresponds one-to-one with `checker.rs` call sites                                   |
| `backend.rs:67-72` changed to singleton                               | Cache hit rate is observable (add counter)                                                          |
| `checker.rs:1283-1293` degradation adds warning                       | Diagnostic exists when there is no `body_checker`                                                   |
| Reachability determination of 12 wasm attributes in `orchestrator.rs` | Conclusion handed to `06-cleanup-inventory.md`, this document only registers the determination need |

**Note**: fixing the layer order will change the diagnostic set and may expose a lot of
previously-silent `Unproven`. **This item should go independently of S1-S4**, not mixed in the same
PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                          | Reason                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration is rejected**                                                                             | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant, `dispatch` fails to compile. This is isomorphic to the mechanism that the opcode table has been validated as in RFC-039 routing table B. `StageScope` further turns "per module or project level" into a type fact, eliminating call-count questions that require human reasoning |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used together with the **static** gate of `scripts/ci/check-obligations.py`; severity W first then E     | What is being fixed is a whole class of bugs, not one: at least 2 similar hidden dangers have been verified (the `proof_calls`, `checker.rs:1313`), 16 span-keyed fields are all in range. W first then E is to make every step's C2 criteria usable — going straight to E will make `yaoxiang check` suddenly have a lot of previously-silent diagnostics             |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the producer-side logic of the six `Unproven` branches in `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed", whereas the logic of these three branches is itself correct                                                                                                                                                               |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment promise" and "code behavior"**. "Do not let silent pass resurrect" in
  `checker.rs:5165` and "if a lower layer fails, the upper layer does not run" in
  `layers/README.md:3` are both **comment-level contracts** — the former is fulfilled by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"An array forgot to append"**. Currently there are five functions each hand-writing the call
  sequence; after unification there is a single stage table, and a missed stage connection in the
  new addition will report a compile error.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/link) **do not involve type bridging**, they are configuration questions of "whether to
  run a certain analysis"; it also cannot express `Aggregation`, it can only move the choice between
  `check_module` and `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, which is
  equivalent to replacing "five functions each hand-write the call" with "an array forgot to
  append", which is exactly the cause of the current 11 inconsistencies. The repository already has
  a same-shape counterexample: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` /
  `HotReloader` described in `docs/src/dev/design/check/` are all zero-implementation.
- **Only patch the bug without changing the structure** — fixes the 2 known bugs, but only covers 1
  of the 16 obligation fields, `Stage` is still scattered across 5 functions, and the next
  divergence point will continue to grow from here. **It must be done first** (it is part of S1/S3),
  because structural changes need a known red test to prove the criteria are valid.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it does
  not know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]` only
  warns when the field is **completely** discarded. This is finding the bug at the wrong level: the
  bug is in the orchestration layer, the detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs` thinning will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is the common cost of all refactorings that
  "centralize the wiring".
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously-silent problems"**. After the fix there may be a batch of "new error" user reports —
  they are real existing bugs, just never reported. Must be clearly stated in the CHANGELOG.
- **The severity switch of `assert_drained()` requires per-field manual judgment**. Of the 16
  fields, some (such as `module_namespaces`) may have "no consumer" by design, and should not be
  alerted. S4 needs to go through them one by one, cannot be a one-size-fits-all approach.
- **The `Program` abstraction may be premature**. If the stage set of some entries is unstable for a
  long time, `stages()` will degrade into a "free parameter that passes a different array every
  call", and the contract constraints will be lost. **The mitigation is
  `test_program_stage_coverage` asserting that `stages()` can only come from 6 predefined
  combinations.**
- **Criteria dependency**: S1/S3 bug fix criteria depend on the bug test in the
  [Equivalence Criteria Document](07-equivalence-oracle.md) being red first. **If that document does
  not first establish a multi-file corpus layer, S1 cannot be accepted** — because the existing 293
  corpora all go through the single-file path and have zero coverage of this class of defect. This
  document does not involve the IR verifier (`verify_loose`); that prerequisite work is not in the
  scope of this document.
- **Fixing the layer order will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, the order of `termination` and `ownership` is opposite to the declaration). This
  is a diagnostic set change risk and should go independently.
- **Thread safety after SMT backend is changed to singleton is undetermined**. `backend.rs:13-19`
  already explains that `Solver` is only `Send` not `Sync`, and `Z3Backend`'s cache is `RefCell`;
  after changing to a singleton shared across compilation units, it needs to be confirmed that all
  access paths go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). It should be opened after the equivalence criteria are stable.

> **All open issues originally listed in this section have been adjudicated.** Per-item decisions
> are in the [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` bug criteria
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Comparison code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "do not let silent
  pass resurrect"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "if a lower layer
  fails, the upper layer does not run" (no short-circuit in reality)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that must be consumed
  downstream
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledgment "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-acknowledgment "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Playground's fourth compilation caller (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Build-time gate for error codes, an example of this project's enforcement
  mechanism
