# Compilation Stage Contracts and Obligation Ledger

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution stage sequence are in the RFC-039
> main text; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document is the **L1 Orchestration Layer** construction drawing for RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each manually wired, with no single place that can answer
this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture rather than logic.**

The comment in `src/frontend/core/typecheck/checker.rs:5165` originally reads "Unproven →
compilation error, no degradation, no silent pass... do not let silent pass be resurrected." But
this invariant only holds on the **single-file path**: refinement constraints like `x: Sorted(3)`
silently pass on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths, the proof function
never executes, and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, non-runtime registration) and `StageScope`;
- `Obligations` obligation ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-entry migration plan for ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order in which to do it**; this document
gives L1's **specific form**, **per-file change list**, and **internal implementation stages**
(corresponding to P3 "Fix Correctness Vulnerabilities" and P4 "Stage Contracts and Unified Driver"
in the RFC-039 global sequence). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed per the **C2 (orchestration changes)** category of the
[Equivalence Oracle document](07-equivalence-oracle.md): the diagnostic set at each entry is
identical + the corpus behavior is identical.

## Current State

> This section consists entirely of **verified facts**, each with file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only "One Place Wrong, Whole Repo Silent" Structural Defect

It differs in nature from the other two categories of defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation            | Is There a Signal    |
| ----------------------- | -------------------------------- | -------------------- |
| Lexical/Syntax Error    | Source code written wrong        | Has diagnostic       |
| Type Mismatch           | Type written wrong               | Has diagnostic       |
| Stage Mis-wiring        | Some stage simply not called     | **No signal at all** |
| Obligation Not Consumed | Field filled but no one reads it | **No signal at all** |

The common feature of the latter two categories is: **failure produces no error**. Therefore they
cannot be solved by "writing code more carefully" or "reviewing more strictly" — code review can
only see what was written, not **what was not written**. This is exactly the root cause diagnosed in
RFC-039: "This project treats 'design' as a documentation convention rather than an executable
constraint."

### Enforcement Mechanisms Already Present in This Repository

The same repository already has mature enforcement mechanisms, just not yet extended to the stage
layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subject to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, which compares them
  one-by-one against the RFC-013 code table; any inconsistency directly `panic!`s to refuse
  compilation.
- `src/package/` (**76 files / 13012 lines**, of which the `tests/` directory is 6227 lines, 47.9%)
  has high test density, with each module's test subtree declared and wired — this is the most
  completely wired piece of test infrastructure in the entire repo and can serve as a form reference
  for stage-layer gates.

**The design capability is sufficient. What is lacking is "placing a gate of the same level in the
orchestration layer as well."**

(Line count metric: `(Get-Content).Count`, see the "Line Count Metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation It Takes                      |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages, direct calls                          |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking          |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics    |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR        |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project takes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1        |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The following table provides evidence cell by cell. **A blank does not mean that the entry "doesn't
do this thing", but rather "there is no code in that entry that does this thing"** — this is
precisely the nature of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **yes** `187-203`                                   | **no**                                                    | **no**                                                      | **no**                             | **no**                                |
| 2   | Dead code analysis                                       | yes `275-278` (gated by `config.dead_code.enabled`) | **no**                                                    | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                                |
| 3   | W1006 local module shadowing                             | **no**                                              | **no**                                                    | yes `336-348`                                               | **no**                             | **no**                                |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)         | **collected but never emitted**                           | yes `350`                                                   | **no**                             | **no**                                |
| 5   | W1001/W1002 dead code family                             | yes (same as 2)                                     | **no**                                                    | yes (same as 2)                                             | **no**                             | **no**                                |
| 6   | **Monomorphization**                                     | yes `381-389` (gated by `config.mono.enabled`)      | **no**                                                    | N/A                                                         | N/A                                | **no**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **no**                                              | **no**                                                    | yes (`321-328`)                                             | **no**                             | **no**                                |
| 10  | Global slot allocation                                   | N/A                                                 | yes `allocate_global_slots` (`147`)                       | **no**                                                      | **no**                             | **no**                                |
| 11  | IR generation + qualified name rewriting + linking       | yes (`205`)                                         | yes (`152-236`)                                           | **no**                                                      | **no**                             | yes (independent ModuleIR then merge) |

> **Review Note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, preserved unchanged. Differences in current state after P3
> lands:
>
> - **Row 1 has been fixed**: proof_execution's five entries share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + orchestrator's four entries); the single-consumer
>   defect no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not emit W1003, still doesn't run
>   dead code family) → categorized as WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization is single-file only) → categorized as WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk not-yet-confirmed defect).
> - **Row 7** check_module / check_module_collect_all dual entry → categorized as 4.2.7 (driven by
>   Aggregation parameter).
> - The code-side reference to "Arbitration #434" was previously zero-registered in docs — now
>   registered as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new fact outside the table, registered 2026-10-09)**: The
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch), while the multi-file `discover` previously only
>   recognized explicit `use` — in-project for loops compile successfully but produce runtime E6006
>   (probe evidence). Fixed and WBS 4.10.2 crossed out; same family "multi-file missing one step
>   from single-file" parse hard-abort quirk registered as WBS 4.10.1 — **also fixed** (lands
>   immediately after 4.2.2; the Check path downgrades to per-file collection; from this audit's
>   perspective of "stage coverage / field consumption" dimension, this one is a compilation unit
>   member difference, a missed dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm used `extract_module_info` to redundantly harvest registration for embedded std
>   units, overwriting the native half already registered by `with_std()` via "native + yx surface
>   merge" (e.g., `result.is_err` lost → std.test false-positives E1043). This audit's third missed
>   dimension: the merge semantics of "two registration sources for the same module key" — the
>   single-file path's registry is formed only once via `with_std()`, only the multi-file/Check
>   Registry arm has the issue when inserting unit by unit.

**Two places require precise wording, otherwise the writing will be wrong:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but the function **has no `result.warnings` read site anywhere in its
  body**; `result`'s sole downstream use is `generate_ir_with_context` (`154-156`).
- **The `main` criteria at entry row 11 also differ in source**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "whether manifest exists"). The two functions give different answers to "which file must
  define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. The following eight steps are all reproducible.**

**Step 1 — The obligation's production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the entire
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

The semantics are: "the predicate argument of this refinement constraint is a compile-time literal,
the proof function must actually be executed to determine the result."

**Step 2 — The consumption point is inside the checker, but only emits, never consumes.** There are
**three isomorphic branches** in `src/frontend/core/typecheck/checker.rs` handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The comment in `checker.rs:5165-5169` reads:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch is therefore a true defense line rather than a forward-looking safety net: do not let silent pass be resurrected.`

That is: the author knows that "not emitting diagnostics" is a defect, and explicitly classifies it
as a **known, nowhere-to-go intermediate state**, with the design assumption that "someone will
definitely come to read `proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares the local
`proof_calls` and collects them, `checker.rs:1439` writes to `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined in
`types.rs:29`.

**Step 4 — There is only one read point in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code is read in only **one place, `src/frontend/pipeline.rs:187`**
(passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries skip that layer entirely.** The four entries
`compile_project` (`99`), `check_project` (`273`), `check_source_in_project` (`450`), and
`compile_embedded_module` (`1374`) in `src/frontend/module/orchestrator.rs` **all do not call
`pipeline.rs`**, they call `TypeChecker::check_module` directly (`124` / `493` / `1386`). So they
don't even reach the only read point at `pipeline.rs:187`.

**Step 6 — Consequence: even the standard library's own refinement obligations take the discard
path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std module itself are
also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP three paths **compiles, runs,
and emits no diagnostic at all**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point in the same layer.**
When `checker.rs:1303-1314` handles ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` from the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the "do not let silent pass be resurrected"
promise of the `5164` branch, and is **independent of the orchestrator issue** — even if the entry
layer is fully fixed, this point remains silent. (Processing timing: should be handled in the same
batch as the obligation ledger, because it's the same class of problem as the obligation ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example, `rfc027_return_refinement.rs:350-359` asserts that `SumUpTo` exists in
`result.proof_calls`. But they **all go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping precisely
before `pipeline.rs`**.

The test files' own documentation comments already state this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is precisely the shape of the problem: tests verify "the obligation is filled", and the bug
is "the consumer doesn't read it".** A test that only tests the production side and not the
consumption side is naturally immune to this class of defect.

### Stronger Finding: The Equivalence Oracle's Main Corpus Is the Single-File Path

The [Equivalence Oracle document](07-equivalence-oracle.md) uses the end-to-end diff of the 293
`.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion**, as the main acceptance
means for C2 stages. But empirical testing shows:

- There is **no `yaoxiang.toml`** anywhere under the `tests/` directory (zero hits directory-wide).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical evidence
of this point. The file is annotated `// expect: compile-error E4018` (line 15), and its header
comment line 7 says "Status: ❌ should be rejected at compile-time" — **it passes precisely because
it takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See Implementation Point S1.

### Obligation Fields: 16 "Produce-equals-Contract" Fields, Zero Mechanism Guarantee

Of the `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields**
whose documentation explicitly states "produced by stage X → consumed by stage Y", i.e., they are
essentially **cross-stage obligations**:

| Field                      | Definition Line | Producer → Consumer                                             |
| -------------------------- | --------------- | --------------------------------------------------------------- |
| `proof_calls`              | `29`            | typecheck → **proof_execution**                                 |
| `release_plan`             | `31`            | ownership → IR generation (`ir_gen.rs:341` reads, `:1942` uses) |
| `escaped_refs`             | `33`            | ownership → IR generation                                       |
| `instantiation_requests`   | `35`            | typecheck → monomorphization (`pipeline.rs:381`)                |
| `existential_coercions`    | `37`            | typecheck → IR generation                                       |
| `implementation_proofs`    | `39`            | typecheck → IR generation                                       |
| `interface_impl_registry`  | `42`            | typecheck → operator query / constraint solving / LSP           |
| `sum_types`                | `44`            | typecheck → IR generation                                       |
| `sum_type_param_names`     | `47`            | typecheck → IR generation                                       |
| `variant_ctor_calls`       | `49`            | typecheck → IR generation (span-keyed)                          |
| `operator_dispatches`      | `51`            | typecheck → IR generation (span-keyed)                          |
| `method_overload_ir_names` | `53`            | typecheck → IR generation                                       |
| `overload_resolutions`     | `56`            | typecheck → IR generation (span-keyed)                          |
| `try_expr_impls`           | `59`            | typecheck → IR generation (span-keyed)                          |
| `match_scrutinee_types`    | `63`            | typecheck → IR generation (span-keyed)                          |
| `module_namespaces`        | `66`            | typecheck → IR generation                                       |

**All of these fields are passed across layers keyed by `Span` or name tables.** The equivalence
oracle document has already pointed out that span-keyed contract mismatches "silently fail with no
error" (`ReleasePlan` is typical: any inconsistency in how spans are computed on either side →
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan` at `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`) is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the consumer
  is in the downstream module, and the downstream module is on the necessary path of all entries.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry implementation**. The orchestrator's four entries do not go through the `pipeline.rs` layer
  at all.

**Verified fact**: the repository currently has **no mechanism** that guarantees these 16 fields are
consumed. The only "protection" is that `ReleasePlan` happens to hitchhike on IR generation.

### Four Additional Contract Defects in the Proof Layer

The following are all from proof-layer analysis, independent of the entry forking problem, but all
fall under the category of "stage contracts not enforced".

**(a) The declared layer order contradicts the actual execution order, and the `equivalence` layer
is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "Executed in layer order, lower layer fails upper layer doesn't run." The actual
call points in `TypeChecker::check_module`:

| Actual Order | Call Point                                        | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear in `check_module`'s stage sequence at all** (it is
only used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time, **there is no short-circuit** — `checker.rs:1271-1276` continues
to run after `add_error`ing each termination error, so the "lower layer fails upper layer doesn't
run" promise in README does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review Note (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (SOLVER slot
> Option-ized, missing cases downgrade conservatively to `SMTResult::Unknown`); "silent skip (no
> injection)" is supplemented with W1081 signal by 3.4.2. Unification of the three forms (singleton)
> is executed per **RFC-039 D58**, the actual fix location is `proof/smt/backend.rs` (new
> process-level shared singleton + `with_shared_solver` closure port), the 02 change list's
> `default_solver() → &'static` expression is subject to D58 (the bare `&'static` reference is
> infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                   | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ---------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via injection at `checker.rs:1265-1268`) | Construction-time injection `with_solver_owned`          | `None` → **silent skip** (no injection)                   |
| `ownership.rs:627` (back-edge cutting decision)            | **`default_solver()` on every call**                     | `None => return false` (`629`) → conservatively not cut   |

The comment in `predicate.rs:31-32` explicitly chooses hard failure: "Initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds
kernel capability'." But the contract doc at `backend.rs:60` says "`None`: backend
unavailable……**caller should downgrade conservatively**". **Two philosophies coexist in the same
code, and the hard-fail one panics rather than returning an error.**

**(c) A new Z3 context is created on every back-edge decision, making caching effectively
nonexistent.** `ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutting
decisions. While `src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call invokes `Z3Backend::new()`, which
creates a new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits in this call pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifetime strategies.)

**(d) Checker silent downgrade + comment inconsistent with reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call ownership table
```

When `body_checker` is `None`, ownership check gets **an empty type ledger and empty call table**
and runs normally — ownership analysis degrades to "nothing conflicts", **with no warning at all**.
(Processing suggestion: should be recorded as a warning-level diagnostic, or at least left as a
trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after typecheck, before constraint
solving". In reality `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at
`1256`, ownership at `1296` after, i.e., the "before" the comment mentions is actually "two layers
after"**.

### wasm Status: Carried by Shim Crate, 27 Files with Live Branches

The wasm target **has been built and is constructed in CI**; the `cdylib` is in a separate shim
crate, not in the main crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has a wasm target dependency section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (the corresponding non-wasm section is `Cargo.toml:125-131`: tokio/ureq/tempfile)                                                                                                  |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm build sites**                     | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetch `libz3.a` from a fixed URL, downgrade to warning if missing)                                                                                                                                                            |
| **27 files** contain the `wasm32` literal         | **25 of them** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are documentation mentions only                                                                                                                                  |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                       |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target — `lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry in
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path Consuming `proof_calls`               | Whether Proof Function Executes |
| ------------------------------------------ | ------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                         |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                         |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                         |
| Multi-file `run` (`compile_project`)       | **No**                          |
| `check` (`check_project`)                  | **No**                          |
| LSP in-project (`check_source_in_project`) | **No**                          |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see Target Design section 3), otherwise unifying the Driver will miss this
path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is filed under `06-cleanup-inventory.md`. The open question in RFC-039 "build
wasm target, or delete these branches" is **already expired**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-runtime Registration)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "a new stage must be
handled by the orchestration layer" — this is exactly the mechanism that the opcode table has proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexing + syntax               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type check (including embedded proof layer)   PerModule
    DeadCodeAnalysis,   // Dead code family analysis              Project
    ProofExecution,     // Compile-time proof function execution         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive `dispatch` match will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The use of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs once
at project level) make "which stages must run per module, which must run once at project level" a
type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in `dispatch`
— this eliminates the current orchestrator.rs question of "should `allocate_global_slots` be called
once per file" that requires human reasoning.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

> **Revision Note (P4 Implementation, 2026-10-07)**: Two deviations from the draft, corrected per
> implementation evidence —
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), the draft order contradicts the data
>    flow.
> 2. The Check-form stage table is normalized to dead code **before** proof per the topological
>    order of `Stage::ALL`: the current `check_project` executes proof before dead code, with no
>    data dependency between the two, the diagnostic set is the same (C2 set semantics), only the
>    per-file diagnostic order is normalized. The single-file path (dead code embedded in typecheck,
>    before proof) already conforms to ALL order, byte-by-byte acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user
>    arbitration): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products — the draft order would force the Registry arm to "hide parse", making the stage table
>    lie about the data flow; and multi-file parse drops from 2 times to 1 time. Attached status
>    registration: the multi-file path's parse error was a hard abort (`?` propagation in
>    `build_registry_from`, the tension with CollectAll semantics registered as WBS 4.10.1) —
>    **4.10.1 has been fixed** (2026-10-09 user arbitration of rust-style collection semantics +
>    Plan B): Check path parse failure downgrades to per-file diagnostic collection, diseased files
>    exit the compilation unit (not entering registry, importers report E5001); MultiFile path
>    retains hard abort (under FailFast a bad file cannot produce IR, which is correct semantics,
>    nailed long-term).
> 4. Check form landing (4.2.2, 2026-10-09 user arbitration) adds two registrations: a. Data
>    dependency edges 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events — the draft missed listing these
>    (these two products were produced by `discover_with_used` in the orchestrator era, discarded by
>    the `discover` wrapper, the data flow was lost by not entering the table); b. The per-file
>    diagnostic order normalization is extended to the complete form of note #2: E3020 entry check
>    moves into the RoleClassification arm (Check has no Linking stage, the arm holding surfaces and
>    AST is responsible for the check), per-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set semantics),
>    only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field is produced but no one consumes it" from undetectable into a fact that
can be asserted at compile time or test time. RFC-039 has already listed this as "the highest-value
item".

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
    /// Settlement: call once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three levels)**:

| Situation                            | Criterion                                           | Disposition                                                         |
| ------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------------- |
| Obligation read by declared consumer | Field in `Obligations` is `take()`n/marked consumed | Pass                                                                |
| Obligation non-empty but no consumer | Field non-empty and not consumed                    | **Produce diagnostic** (W level, default; E level in `strict` mode) |
| Obligation empty                     | Field is empty                                      | Pass (no consumption needed)                                        |

**Why W level first rather than E level**: fixing obligation discarding **changes the diagnostic
set**. The C2 criterion requires "the diagnostic set at each entry is identical", and going straight
to E level will cause `yaoxiang check` to suddenly produce a large number of previously silenced
`Unproven` diagnostics. Splitting into two steps (observe at W first, then upgrade to E) makes the
criteria at each step available. See Implementation Point S4 for details.

**The single call point of `assert_drained()`**: `Driver::run` is at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (field
appears ≥2 times but no third file reads it → fail). `assert_drained()` is a **runtime** gate; that
script is a **static** gate, the two complement each other, both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, filter result to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — fourth caller of the single-file path
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
does not accept caller-supplied free arrays.** This is the key constraint preventing the `Program`
abstraction from degrading into "anything goes free parameter", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Single settlement point
        Ok(state.into_result())
    }
}
```

> **Revision Note (P4 Implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for the record —
>
> 1. The three-state `StageOutcome` in §4 is not returned by the stage arm, but is expressed by
>    `State` failure markers + `Aggregation` gating (Continue / Warn / Abort semantics unchanged,
>    eliminating per-arm boilerplate returns);
> 2. `Driver` has no `config` field — the only source of configuration is `Program.config` (§5's
>    "Avoid Driver holding mutable global state" determination overrides the sketch field);
> 3. `Skipped` diagnostics are recorded in 4.1.3 only into the internal ledger of
>    `DriverOutcome.skipped`, not emitted externally — the S2 "deliberately fix no bugs" zero-diff
>    criterion requires it; external emission comes with 4.3 obligation ledger;
> 4. `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is the allowed direction); the orchestrator's existing call sites will be removed
>    with the 4.2 migration, and visibility will be revisited afterwards.
> 5. Under the multi-file form, ProofExecution is an independent stage, executed **after all**
>    typecheck (A1, 2026-10-09 user arbitration): `compile_project` originally interleaved proof
>    within the per-file typecheck loop (file N's proof before file N+1's typecheck). For a single
>    failure, the two are byte-by-byte identical; in the multi-failure scenario "file 1 proof
>    failure + file 2 typecheck failure", the first reported error changes from proof error to
>    typecheck error (test nailed).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — the external error contracts of each
>    entry (PipelineError / OrchestratorError) are not types Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each discovered file has an entry, clean files have empty Vec — current contract of
>    check_project); Discovery product is extended to a triple "file set + used_by edge set + shadow
>    events" into State (MultiFile form has no downstream consumer, just recorded not emitted);
>    per-file parse in check path drops from 2 to 1 with the C3 topology.
> 8. Lsp form landing (4.2.3, arbitration by file source split, 2026-10-09 user ruling): disk file
>    parse failure follows Check's Plan B (collect + exit compilation unit — both ends downgrade
>    converge); parse failure of the edited buffer retains a broken AST to continue typecheck
>    (editor philosophy — intermediate typing state should not make semantic features disappear).
>    The Discovery arm preserves buffer source content over disk stale content for Lsp; Typecheck
>    under Lsp only checks the target file (other units only for registry signature provision);
>    per-file diagnostic order normalization principle extends to LSP (typecheck diagnostic → W code
>    warning → proof error). The old LSP behavior of hard-aborting on irrelevant disk file parse
>    errors and the handler silently falling back to the single-file path is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` adds a new `shared_registry` field — the embedded std
>    module is a sub-compilation, the registry is an **input** rather than a product
>    (EMBEDDED_STAGES has no Registry stage), #94's SymbolTable sharing contract is thereby made
>    explicit from "caller remembers to pass the same registry" to a program declaration. Error path
>    text normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler internal error surface only, no test nailed). At this point all four orchestrator
>    entries are migrated into Driver.
> 10. Standalone check unification (4.2.5, arbitration A + IR stage arbitration, 2026-10-09): Check
>     form faithfully carries single-file semantics for programs without a project root — warning
>     surface only covers the entry file (adjacent files only get errors, rather miss than
>     false-positive); relative `use` resolved along the importer's directory (rustc single-file mod
>     alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path
>     (pipeline full chain) already runs IR generation, E3019/E1014/E1015 are only produced in
>     ir_gen (runner gate evidence). IrGeneration's Check form is pure check, does not consume IR;
>     Script/Bin split is expressed with None/Some of module_key (E3023's existing switch,
>     ir_gen.rs:1660). Monomorphization does not enter CHECK (only produces resource exhaustion /
>     internal error, zero corpus dependency). Attendant fix for latent defect: the Registry arm's
>     redundant harvest registration of embedded std units would overwrite the native half already
>     merged by with_std() (E1043 false-positive) — embedded units now skip duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted, replaced by
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI use the same Driver.
>     SingleFile+CollectAll form: Parsing collects full parse errors, retains broken AST to continue
>     typecheck (editor philosophy extended from 4.2.3 arbitration; not marked as stage failure,
>     otherwise topology skip would prevent typecheck from ever being reached); Typecheck dispatches
>     `check_module_collect_all` free function. The LSP single-file path from this point on
>     supplements proof (E4018), W code warnings (W1001–W1005, test nailed), and IR-level errors;
>     lexical failure now reports real diagnostics (the old synthesized "E0001 lexical error" text
>     disappears with the sequence deletion — real diagnostics carry precise spans, no information
>     loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory
>     item): the two entries have long since converged — the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single bool, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain run in both modes, the difference is carried by
>     whether collected_errors is empty; five collection points in statements.rs (function
>     body/use/for/block/while). Verification confirmed and fixed a duplicate diagnostic defect: the
>     collection point places the first error into both collected_errors and the Err return channel,
>     pass-3's add_error on Err causes the same code same span same message ×2 (nested can be ×3);
>     plus two independent mechanisms: the annotation check signature parameter and the whole
>     annotation double-visit (E1003/E1103), and the ownership layer same-point double-emit
>     (E2014/E2018). The fix lands at the module result boundary: deduplicate by (code, span,
>     message) — diagnostics are positional facts, duplication carries no information; 63 corpus
>     entries remove duplicate copies, run column and exit code zero-drift (the "duplication" in
>     borrow_conflict_err is an artifact of the baseline triple format not containing column number
>     — the two E2018s are actually legitimate diagnostics on different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` constructs
>     `Program { kind: WasmPlayground }` + Driver via `compile_playground` — the `Compiler` wrapper
>     no longer passes through. Error text aligned byte-by-byte with the old `CompileError` Display
>     prefixes (Parse error:/Type error:/Internal error:), zero visible difference in playground UI.
>     WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (4.1's existing declaration,
>     has real producers from this step on).

**Migration plan for the ten entries (function-by-function)**:

| Entry                                                      | After Migration                                                                                                                                                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand off to Driver                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                        |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                          |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter result to target file                                                                                                                                                             |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                            |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project path changes to `Program { kind: Lsp, aggregation: CollectAll }`; the single-file branch's **manual lex→parse→… sequence is deleted**, changes to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                        |

**Duplication between `check_project` and `compile_project` (approximately 40% shared / 60%
forked)**. The shared part is: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parse loop, per-file checker assembly, result handling. The forked parts are
items 2/3/4/5/7/8/9/10/11 in the 11 items table above. **This 40% shared skeleton is exactly the
value zone of `Driver`** — the 60% that forks is all "different stage sets" or "different
aggregation modes", which is exactly what two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, abort this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to skip" is not a stage return value.**

Reason: skip is **topology**-determined, not self-determined by the stage. If we let each stage
return `Skipped` itself, then the information "why didn't I run" is scattered across 12 stages and
cannot be centrally audited. Change to: `dispatch` decides at the beginning of the loop based on
dependency relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability to be fixed by
this document — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent
"skip" that produces no diagnostic, just because the obligation happens to be empty. Once the rule
is established, any "Y didn't run because X didn't happen" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                     |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                               | "Stage Y did not execute due to failure of upstream stage X"                                                                       |
| Caused by empty prerequisite obligation                  | "Stage Y did not execute because no pending obligation (normal)" — **severity = Info for this item, not counted in warning count** |
| Condition not met (e.g., `config.mono.enabled == false`) | "Stage Y did not execute because configuration is not enabled"                                                                     |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic flow, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of that count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Already exists         |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists         |

**`Aggregation` must be a field of `Program`, not a global Driver setting** — LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
separate calls; placing them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of "early
exit or not" — they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter rather than retaining two functions (**this is an
additional item that this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation, does not touch `MonoType` / `PolyType` / `ir::Type`                                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what is migrated into the ledger is **settlement
responsibility**, not the type's home module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay in place;
`driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reading `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`** — this is consistent with the driver's no-reverse-dependency red
line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not
reverse-`use crate::driver`**. Driver appearing in `TypeChecker`'s import is considered a violation,
intercepted by `scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before Migration                         | After Migration                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                              |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution has been emergency-patched in P3; **warning surface maintained, no new addition** (Decision B1, 2026-10-09: dead code family warnings do not enter compile path — pool semantics is being defect-corrected in 4.9, normalization of warning surface waits for 4.9; the "new warning output" in this row's draft is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Emergency-patched in P3                                                                                                                                                                                                                                                                                                                    |
| LSP (in-project)                    | No proof_execution                       | Emergency-patched in P3                                                                                                                                                                                                                                                                                                                    |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                  |
| Z3 not installed + multi-file       | Silent skip                              | Same, unified                                                                                                                                                                                                                                                                                                                              |

**The only intentional behavior break** is changing the `.expect()` at `predicate.rs:35` to return a
diagnostic. This is consistent with the positioning of RFC-027 §8 "not bound to a specific solver",
and also the contract already declared in `backend.rs:60` ("caller should downgrade conservatively")
— the current implementation is the opposite of its own contract documentation.

### Compiler Change List

**New Files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified Files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                        |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changes to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changes to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changes to Driver                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changes to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                                         |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changes to call Driver                                                                                                                                                                                                                                                                                    |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                          |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                              |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changes to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                               |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changes to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changes to produce diagnostic (**fix second silent-discard point**)                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` downgrade path adds warning diagnostic                                                                                                                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changes to **actual** execution order; remove "lower layer fails upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: new process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; three consumer points in `predicate.rs`/`checker.rs`/`ownership.rs` change to use the shared port. Bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`), original expression is subject to D58 |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is on the consumer side, not the producer side**, modifying
the producer would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branches (reachability determination of wasm branches filed under
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                      | Compatibility                                                       | Disposition                                                                                                                                     |
| ----------------------------------------------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                        | **Break**: previously silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **red first, then fix**; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                      | **Break**: programs that compiled now produce warnings              | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), does not change exit code                          |
| `assert_drained()` produces W-level diagnostic on first run | **Break**: diagnostic set adds items                                | W first then E, S2/S4 in two steps                                                                                                              |
| `predicate.rs` panic changes to diagnostic                  | **Improvement**: no longer crashes                                  | No break                                                                                                                                        |
| `build` / `dump_bytecode` subcommands                       | **No impact**                                                       | These two paths do not go through proof_execution                                                                                               |
| `TypeCheckResult` field migration to `Obligations`          | **Internal refactoring**                                            | If `pub` API surface has external dependencies, need to synchronize; in-repo consumers are exhaustively listed in the change list               |

---

## Implementation Points

This document corresponds to **P3 (Fix Correctness Vulnerabilities)** and **P4 (Stage Contracts and
Unified Driver)** in the RFC-039 global stage sequence. The following S1-S5 are the implementation
order internal to this document, **each step's acceptance criterion refers to the C2 category of the
[Equivalence Oracle document](07-equivalence-oracle.md)** (orchestration changes: each entry's
**diagnostic set** is identical + corpus behavior is identical).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The vulnerability criteria must be written as failing first.

| Deliverable                                                                                                                                                   | Acceptance                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                 | **Must be red**. If accidentally green, the vulnerability analysis of this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Resolution D48 — do not mix into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file corpus versions can be compared                               |
| `test_obligations_drained` skeleton                                                                                                                           | Mark `#[ignore]`, S4 turns green                                                                            |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This phase deliberately fixes no bugs** — it just moves existing behavior into Driver. The
acceptance standard is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` can be restored to its original
state.

### S3: Merge Orchestrator's Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four-entry diagnostic set identical after normalization by `Aggregation`          |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project              |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                        |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fix                                                                                           | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changes to diagnostic                                                                          | No longer panics in Z3 environment                                                |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has issues, you can only revert the structural commit and keep the vulnerability
fix commit — at this point `test_multifile_proof_obligation_not_dropped` stays green; the reverse
(keep structure, revert fix) would turn red, which is an unacceptable intermediate state and is
forbidden from merging.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                           |
| -------------------------------------------- | ---------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green               |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                             |
| Obligation diagnostic W → E upgrade          | After full corpus diff, manual review of every new E |

**Rollback point**: `assert_drained()`'s severity is controllable by a configuration item, the W/E
switch does not require code structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                            | Acceptance                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                 | Documentation corresponds item-by-item to `checker.rs` call points                                                                                                    |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter cache** — otherwise one timeout becomes process-level cache-solidified across compilations, and pollutes cargo test thread-shared state) | Production path `default_solver()` call points drop to zero (tests retain only); cache hit rate observable across three consumer points (counter landed in 89576fafd) |
| `checker.rs:1283-1293` downgrade adds warning                                                                                                                                                                                          | Has diagnostic when no `body_checker`                                                                                                                                 |
| Reachability determination of orchestrator.rs's 12 wasm attributes                                                                                                                                                                     | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination requirement                                                              |

**Note**: correcting the layer order will change the diagnostic set and may expose many previously
silenced `Unproven`. **This item should be independent of S1-S4**, do not mix it with the entry
merge in the same PR.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                   | Reason                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage Model**          | `Stage` is a 12-variant compile-time exhaustive enum, **rejects runtime registration**                                                                          | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant `dispatch` will not compile. This is isomorphic to the mechanism the opcode table has already validated in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the call-count question that requires human reasoning                        |
| **Obligation Mechanism** | `Obligations` + `assert_drained()` does **runtime** settlement, used together with `scripts/ci/check-obligations.py`'s **static** gate; severity W first then E | What's being fixed is a whole class of bug, not a single bug: at least 2 same-class hidden dangers have been verified ( `proof_calls`, `checker.rs:1313`), all 16 span-keyed fields are in range. W first then E is to make C2 criteria at each step available — going straight to E will cause `yaoxiang check` to suddenly produce a large number of previously silenced diagnostics |
| **Fix Scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the six `Unproven` branch producer-side logics at `checker.rs:5164/5179/5306/5318/5420/5448`  | The problem is on the consumer side, not the producer side. Modifying the producer side would mask the architectural defect as "logic is fixed", while the logic of these three branches is itself correct                                                                                                                                                                             |

This design additionally eliminates two types of cracks:

- **The crack between "comment promise" and "code behavior"**. The "do not let silent pass be
  resurrected" at `checker.rs:5165` and the "lower layer fails upper layer doesn't run" at
  `layers/README.md:3` are both **comment-level contracts** — the former is fulfilled by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"One array forgetting to append"**. Currently five functions each write their own call sequence;
  after unification, there is a single stage table, and missed wiring of a new stage will cause a
  compile error.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type bridging**, they are configuration issues of "whether to
  run a certain analysis"; it also cannot express `Aggregation`, only moves the `check_module` /
  `check_module_collect_all` either-or from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding a new stage `dispatch` no longer fails to compile, which is equivalent
  to swapping "five functions each writing their own calls" for "one array forgetting to append",
  the very cause of the current 11 inconsistencies. The repository has the same-form counterexample:
  the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix the bug without touching the structure** — can fix the known 2 bugs, but only covers 1
  of the 16 obligation fields, `Stage` is still scattered across 5 functions, the next fork point
  will continue to grow from here. **It must be done first** (part of S1/S3), because the structural
  refactoring needs a known red test to prove the criterion is valid.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it does
  not know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]` only
  warns when the field is **entirely** discarded. This is looking for the bug at the wrong layer:
  the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **The slimming of `orchestrator.rs` will make it harder to read in the short term**. After the
  139-line `compile_project` is split into "Program constructor + several driver arms", readers need
  to cross two files to understand the flow. This is a common cost of all "centralizing the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously silenced problems"**. After the fix, there may be a batch of "new errors" user reports
  — they are real bugs that just never got reported. Must be clearly stated in the CHANGELOG.
- **The severity switch of `assert_drained()` requires manual judgment field by field**. Of the 16
  fields, some (e.g., `module_namespaces`) having "no consumer" may be by design, should not warn.
  S4 needs to go through field by field, cannot be a one-size-fits-all approach.
- **The `Program` abstraction may be premature**. If the stage set of some entry is unstable for a
  long time, `stages()` will degrade to a free parameter "different array each call", and the
  contract constraint will fall through. **The mitigation is `test_program_stage_coverage` asserting
  that `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: the vulnerability fix criteria of S1/S3 depend on the vulnerability test
  of the [Equivalence Oracle document](07-equivalence-oracle.md) being red first. **If that document
  does not first establish a multi-file corpus layer, S1 cannot be accepted** — because the existing
  293 corpus files all take the single-file path and have zero coverage of this class of defect.
  This document does not involve the IR validator (`verify_loose`); that part of the prerequisite
  work is not within this document's scope.
- **Layer order correction will expose previously silenced `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order is reversed from the declaration). This is a
  diagnostic set change risk, should be done independently.
- **Thread safety after SMT backend becomes singleton is undetermined**. `backend.rs:13-19` already
  states that `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after changing
  to a singleton shared across compilation units, need to confirm all access paths go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization would **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). Should be opened after the equivalence criteria are stable.

> **The open questions originally listed in this section have all been arbitrated.** See
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for item-by-item
> decisions (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (construction steps filed under
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "do not let silent
  pass be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower layer fails
  upper layer doesn't run" (no actual short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  downgrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledgment of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-acknowledgment of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of the playground (single-file path, therefore
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate, not the main crate
- `build.rs:19-55` — Error code build-time gate, exemplar of this project's enforcement mechanism
