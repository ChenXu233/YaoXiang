> **Accessory Design Document**. This document is an accessory to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-stage sequence are in the main text of
> RFC-039; the positioning of each accessory document is in [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction drawing for RFC-039. It addresses one
problem: **"Which stages did this compilation run, which did it skip, and why" is currently
scattered across six paths and ten entry functions, each hand-wired, with no single place that can
answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture rather than by logic.**

The original comment in `src/frontend/core/typecheck/checker.rs:5165` reads "Unproven → compile
error, no degradation, no silent pass... silent pass must not be resurrected". But this invariant
only holds on the **single-file path**: proof function constraints of the form `x: Sorted(3)`
**silently pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths—proof functions
are never executed, and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry refactoring plan for the ten entry
  functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of Scope

- The four-layer model, dependency-direction rules, anti-regression gate → `01-routing.md`
- Equivalence criteria (C1–C6 tiers, three-level criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- The P1–P10 global execution order and G1–G10 acceptance gates → main text of RFC-039

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and **in what order**; this document gives L1's
**concrete form**, **per-file change inventory**, and the **implementation stages within this
document** (corresponding to RFC-039's global sequence P3 "Fix Correctness Bugs" and P4 "Stage
Contract and Unified Driver"). Wherever this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed under the **C2 (orchestration change)** category per the
[Equivalence Criteria document](07-equivalence-oracle.md): same diagnostic set per entry + same
corpus behavior.

## Current State

> All items in this section are **verified facts**, each with file path + line numbers. Line numbers
> are based on `9e02e4db`.

### Stage Boundary Is the Only "One Wrong Place, Whole Repo Silent" Structural Defect

It differs in nature from the other two defect classes (module boundary, test wiring):

| Defect                  | Typical Symptom                | Has a Signal?        |
| ----------------------- | ------------------------------ | -------------------- |
| Lexical/syntactic error | Source written wrong           | Has diagnostics      |
| Type mismatch           | Type written wrong             | Has diagnostics      |
| Stage wiring gap        | Some stage is never called     | **No signal at all** |
| Obligation unconsumed   | Field filled in but never read | **No signal at all** |

The common feature of the latter two: **failure does not produce an error.** They cannot be solved
by "writing more carefully" or "stricter review"—code review can only see what is written, not what
is **not written**. This is exactly the root cause diagnosed by RFC-039: "this project treats
'design' as a documentation convention, not as an executable constraint."

### Forced Mechanisms Already Present in the Repo

The same repository already has mature enforcement mechanisms—they just haven't been extended to the
stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subjected to a
  **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, which compares every
  code against the RFC-013 code table; any inconsistency triggers `panic!` and refuses compilation.
- `src/package/` (**76 files / 13,012 lines**, of which the `tests/` subtree is 6,227 lines / 47.9%)
  has high test density, and each module's test subtree is declared and wired—this is the most
  completely test-wired block in the repo and can serve as a formal reference for the stage-layer
  gate.

**Design capability is sufficient. The missing piece is "putting an equivalent-level gate at the
orchestration layer as well."**

(Line-count convention: `(Get-Content).Count`, see the "Line-Count Convention" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                               |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + link             |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics    |
| 3c  | **LSP in project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR        |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → Path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (Path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project: Path 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → Path 1        |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)—RFC-027 Phase 2.5                  |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined within** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The table below gives evidence cell by cell. **A blank does not mean "this entry does not do this",
it means "this entry has no code doing this"**—this is the very essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                      | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`               |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------ | ------------------------------------------------------------ | ---------------------------------- | --------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                 | **No**                                                       | **No**                             | **No**                                  |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                 | Yes `356-377` (role-aware, no config gate)                   | **No**                             | **No**                                  |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                 | Yes `336-348`                                                | **No**                             | **No**                                  |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                         | Yes `350`                                                    | **No**                             | **No**                                  |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                 | Yes (same as 2)                                              | **No**                             | **No**                                  |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                 | N/A                                                          | N/A                                | **No**                                  |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first-error return** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                 |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, discards `used_by`/`shadow_events`) | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | N/A                                     |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                 | Yes (`321-328`)                                              | **No**                             | **No**                                  |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                    | **No**                                                       | **No**                             | **No**                                  |
| 11  | IR generation + qualified-name rewrite + linking         | Yes (`205`)                                      | Yes (`152-236`)                                        | **No**                                                       | **No**                             | Yes (independent ModuleIR, then merged) |

> **Review Note (WBS 3.4.3, cell-by-cell verification on 2026-10-07)**: This table is the diagnostic
> snapshot for the `9e02e4db` baseline, kept unchanged. The post-P3 status differences:
>
> - **Row 1 is fixed**: proof_execution is shared by the five entry points through the same
>   implementation in `frontend/proof_execution.rs` (pipeline + the four orchestrator entries); the
>   single-consumer defect is gone.
> - **Rows 2/4/5 are unchanged** (multi-file `run` still does not output W1003 and still does not
>   run the dead code family) → WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization is single-file exclusive) → WBS 3.4.8 (prerequisite 4.1.3, potential
>   risk unverified).
> - **Row 7**: `check_module` / `check_module_collect_all` dual entry → 4.2.7 (Aggregation
>   parameter-driven).
> - The code-side reference to "Ruling #434" was previously unregistered in docs—now registered as
>   RFC-039 **D57**.
> - **Embedded-std asymmetry (new fact outside the table, added 2026-10-09)**: The single-file path
>   injects `std.list` unconditionally via `merge_embedded_std_ir` (required for for-loop
>   desugaring, #117 hard switch); multi-file `discover` previously only recognized explicit
>   `use`—in-project for-loops compile successfully but fail at runtime with E6006 (empirically
>   verified by probe). Fixed and WBS 4.10.2 closed; the sibling oddity "multi-file missing a step
>   that single-file has" (a hard-abort parse quirk) is filed as WBS 4.10.1—**also fixed** (landed
>   immediately after 4.2.2; the Check path was downgraded to per-file collection; from this audit's
>   lens of "stage coverage / field consumption" this is a compilation-unit membership difference—an
>   oversight dimension).
>
> - **Embedded-std registration surface coverage (discovered during 4.2.5 implementation, fixed)**:
>   The Registry arm would re-harvest registrations for the embedded-std unit via
>   `extract_module_info`, clobbering the native half-face already merged by `with_std()`
>   (`result.is_err` etc. lost → spurious E1043 in `std.test`). This is the third oversight
>   dimension of this audit: "merge semantics for two registration sources under the same module
>   key"—the single-file path's registry is formed once through `with_std()`, while the
>   multi-file/Check Registry arm inserts per-unit and only then has full coverage.

**Two places that need precise expression, otherwise the implementation will be wrong:**

- **The exact conclusion for rows 4/5**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult` (including
  `warnings`), but this function has **no `result.warnings` read sites anywhere**, and `result`'s
  only downstream use is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` entry criterion is also from a different source**: `check_project:385-395` uses
  `surfaces.bins` (the manifest-declared faces); `compile_project` uses `is_bin_role` (`250-252`,
  which only checks "is there a manifest"). The two functions give different answers to "which files
  must define `main`".

### Correctness Bug: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the heart of this document. All eight steps below are reproducible.**

**Step 1 — The obligation's production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire repo
that constructs a non-empty `proof_calls`:

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
the proof function must actually be executed to decide".

**Step 2 — The consumption point is inside the checker, but it only emits, it doesn't consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter-refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real line of defense rather than forward-looking hedging: silent pass must not be resurrected.`

That is: the author knew "producing no diagnostic" is a defect, explicitly classified it as a
**known, orphaned intermediate state**, and designed under the assumption that "someone will
definitely come to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects into it; `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — The repo has only one read site.** `TypeCheckResult.proof_calls` (`types.rs:29`) has only
one read site in production code—`src/frontend/pipeline.rs:187` (passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the whole repo fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` is production-side collection;
`verdict.rs:61` is a same-name field on `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entry points bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**—they directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they never even reach that sole read
site at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations go through the drop path
too.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded-std module itself are
not executed either**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP three paths **compiles, runs,
with no diagnostics**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-drop point inside the layer.**
`checker.rs:1303-1314` handling the ownership layer's result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership-check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the `5164` branch's promise of "silent pass
must not be resurrected", and is **independent of the orchestrator problem**—even if the entry layer
is completely fixed, this spot will still be silent. (Handling timing: should be done in the same
batch as the obligations ledger, because it is the same class of problem as the obligations ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** Zero coverage of the multi-file path's proof obligations.

The three single-file tests of RFC-027
(`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`, `rfc027_refined_transparency.rs`,
`rfc027_return_refinement.rs`) do indeed assert that `proof_calls` is non-empty—for example
`rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists in `result.proof_calls`.
But they **all go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping precisely
before `pipeline.rs`**.

The test file's own doc-comment states this. From `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof invocation (E4018) is performed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation was filled in", but
the bug is "the consumer didn't read it."** A test that only exercises the producer side and not the
consumer side is inherently immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Primary Corpus Is the Single-File Path

The [Equivalence Criteria document](07-equivalence-oracle.md) uses the end-to-end differential of
the 293 `.yx` corpus files in `tests/yaoxiang/` as the **third-level criterion**, as the primary
acceptance tool for C2-stage changes. But empirical measurement shows:

- Under the `tests/` directory there is **no `yaoxiang.toml`** (zero hits across the whole directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) → `check_single_file` (`623-661`) → `Compiler::compile_with_source` (`635`) →
  `Pipeline::run` → **proof_execution is executed** for every corpus file.
- In other words, **all 293 corpus files go through the single-file path, all of them cover
  proof_execution, none of them cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is exactly empirical
evidence of this. That file is annotated `// expect: compile-error E4018` (line 15), and the comment
at line 7 in its header says "Status: ❌ should be rejected at compile time"—**it passes precisely
because it goes through the only path that actually executes the proof function**.

**Conclusion: the third-level criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation Notes" S1.

### Obligation Fields: 16 "Produce-equals-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
doc-comments explicitly state "produced by stage X → consumed by stage Y", i.e. they are essentially
**cross-stage obligations**:

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

**All of these fields are passed across layers keyed by `Span` or name table.** The
equivalence-criteria document has already pointed out that any mismatch in the span-keying contract
"fails silently, with no error" (`ReleasePlan` is typical: any inconsistency in how the two sides
compute spans → `Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` is dead? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), sits inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the
  consumer is in a downstream module, and the downstream module is on the necessary path of every
  entry point.
- The consumer of `proof_calls` sits at the **top level of `pipeline.rs`**, which is **another entry
  implementation**. The four orchestrator entries never pass through `pipeline.rs` at all.

**Verified fact**: the repo currently has **no mechanism whatsoever** that guarantees these 16
fields are consumed. The only "protection" is that `ReleasePlan` happened to ride on the coattails
of IR generation.

### Four Additional Contract Defects in the Proof Layer

All of the following come from proof-layer analysis; they are independent of the entry-branching
problem but all fall under "stage contracts are not enforced".

**(a) The layer order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README claims "executing in layer order; if a lower layer fails, higher ones do not
run". The actual call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Declared Layer |
| ------------ | ------------------------------------------------- | -------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2        |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1        |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3        |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0        |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear at all in the stage sequence of `check_module`**
(it is only used as an `is_subtype` utility function by `inference/assignment.rs:17`, unrelated to
`ProofResult`). And there is **no short-circuit whatsoever**—`checker.rs:1271-1276` adds each
termination error via `add_error` and keeps going, so the README's promise of "if a lower layer
fails, higher ones do not run" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review Note (2026-10-07)**: The hard-failure panic has been eliminated by 3.3.1 of P3 (the
> SOLVER slot is Option-ized, with a missing backend conservatively downgraded to
> `SMTResult::Unknown`); "silently skipped (not injected)" is supplemented with a W1081 signal by
> 3.4.2. Unification of the three forms (singleton-ization) is executed per **RFC-039 D58**, with
> the actual fix in `proof/smt/backend.rs` (a new process-level shared singleton +
> `with_shared_solver` closure entry point); the `default_solver() → &'static` formulation in the
> change inventory of 02 is governed by D58 (the `&'static` bare reference is not feasible because
> `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Is Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction time via `with_solver_owned`    | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge cut decision)            | **Every call** calls `default_solver()`                  | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure remains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". But `backend.rs:60`'s contract doc says "`None`: backend unavailable... **caller
should conservatively degrade**". **Two philosophies coexist in the same code, and the hard-fail one
panics instead of returning an error.**

**(c) Every back-edge decision creates a new Z3 context, so the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge cut decision. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call does `Z3Backend::new()`, i.e. creates a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, while `z3_backend.rs:17`'s
doc-comment claims "SMT query results are cached in `cache`". Not shared across calls ⇒ **this cache
will never hit under this call pattern**.

(For comparison: the `LazyLock` in `predicate.rs:34-36` is a real singleton. Same backend, two
lifetime strategies.)

**(d) Checker silently degrades + comment contradicts reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check gets an **empty type ledger and an empty call
table** and proceeds as usual—ownership analysis degenerates to "nothing conflicts", **with no
warning of any kind**. (Suggested handling: should be recorded as a warning-level diagnostic, or at
least leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after typecheck and before constraint
solving". But `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at `1256`,
ownership at `1296` after that, so the "before" stated in the comment is actually "two layers
after"**.

### wasm Status: Shim Crate Carries It, 27-File Branch Is Alive

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
an independent shim crate.

| Fact                                               | Evidence                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                          | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                    |
| **Shim crate provides `cdylib` + `wasm-bindgen`**  | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                    |
| Shim depends on the main crate (rlib) as a library | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                           |
| Main crate has a wasm-target dependencies section  | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                       |
| Shim directory is excluded from the main workspace | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                              |
| **CI has 4 wasm build points**                     | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten   | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetch `libz3.a` from a fixed URL, fall back to warning if missing)                                                                                                                                                                 |
| **27 files** contain the literal `wasm32`          | Of these, **25** have actual `#[cfg(...)]` attributes; the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are mentioned only in docs                                                                                                                                          |
| `orchestrator.rs` 20 occurrences                   | 12 attributes + 8 comments                                                                                                                                                                                                                                                            |
| `lib.rs` 11 occurrences                            | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                            |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches across these 27 files are
load-bearing.** They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call
under the wasm target—`lib.rs:139/153/170` gate out `run_file` / `run_project` / `build_bytecode`
entirely (all three need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Executes proof functions? |
| ------------------------------------------ | ------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                   |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                   |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                   |
| Multi-file `run` (`compile_project`)       | **No**                    |
| `check` (`check_project`)                  | **No**                    |
| LSP in-project (`check_source_in_project`) | **No**                    |

That is: **the existence of `wasm/` turns "proof_execution has one consumer" into "has three"**, but
all three are on the single-file-path side. The `ProgramKind` of `Driver` must add a
`WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver will miss this path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—goes to `06-cleanup-inventory.md`. The open RFC-039 question of "build the wasm
target, or delete these branches" is **already moot**: the target is built.)

---

## Target Design

### 1. Stage Model: The `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer"—this is exactly the mechanism already proven by the opcode table
(RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check              Project
    Discovery,          // File discovery                       Project
    Parsing,            // Lex + parse                          PerModule
    Registry,           // Module registry construction         Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type check (with inline proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis            Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation               Project
    IrGeneration,       // AST → ModuleIR                       PerModule
    Monomorphization,   // Monomorphization                     Project
    Linking,            // Cross-module linking / IR merge      Project
}

impl Stage {
    /// All stages. When a new variant is added, both this array and the
    /// exhaustive `match` in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs
once project-wide) make "which stages must run per module and which must run once project-wide" a
type-level fact. A `Project`-scoped stage is structurally guaranteed by `dispatch` to run only
once—eliminating current questions in `orchestrator.rs` like "should `allocate_global_slots` be
called per file" that need human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on order.

> **Revision Note (P4 implementation, 2026-10-07)**: Two deviations from the original draft,
> corrected by implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original draft's order
>    contradicted the data flow.
> 2. The Check-form stage table, normalized by the `Stage::ALL` topological order, has dead code
>    **before** proof: `check_project` currently runs proof before dead code, with no data
>    dependency between the two and identical diagnostic sets (C2 set semantics); only the in-file
>    diagnostic order is normalized. The single-file path (dead code inlined in typecheck, before
>    proof) already matches the ALL order, so byte-by-byte acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    original draft's order would force the Registry arm to "hide parse", causing the stage table to
>    lie about data flow; and multi-file parse would drop from 2 to 1 run. The status quo, recorded
>    truthfully: multi-file path parse errors used to be a hard abort (`?` propagation in
>    `build_registry_from`, the tension with CollectAll semantics was filed as WBS 4.10.1)—**4.10.1
>    is fixed** (2026-10-09 user ruling of rust-style collection semantics + Plan B): the Check path
>    downgrades parse failure to per-file diagnostic collection, the sick file exits the compilation
>    unit (does not enter the registry, the importer reports E5001); the MultiFile path retains the
>    hard abort (under FailFast a bad file cannot produce IR—correct semantics, pinned long-term).
> 4. Check-form landing (4.2.2, 2026-10-09 user ruling), two extra registrations: a. Data-dependency
>    edges 14 → 16: `RoleClassification` consumes `Discovery`'s used_by edge set, `DeadCodeAnalysis`
>    consumes `Discovery`'s W1006 shadow events—the original draft missed listing these (both
>    products were produced by `discover_with_used` in the orchestrator era, wrapped and discarded
>    by `discover`; the data flow got lost because it never made it into the table); b. The in-file
>    diagnostic-order normalization extends to the full form of Note #2: the E3020 entry check moves
>    into the RoleClassification arm (Check has no Linking stage, the arm holding `surfaces` and AST
>    is responsible for the check); the in-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set semantics),
>    only the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no one consumes it" from undetectable into a fact that can
be asserted at compile time or test time. RFC-039 has listed this as "the highest-value item".

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
    /// Each unconsumed field produces one W-level diagnostic (default) or a
    /// hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Case                                        | Criterion                                                  | Disposition                                                                  |
| ------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Obligation is read by its declared consumer | The field in `Obligations` is `take()`'d / marked consumed | Pass                                                                         |
| Obligation is non-empty but has no consumer | Field is non-empty and not consumed                        | **Produce diagnostic** (W-level default; `strict` mode escalates to E-level) |
| Obligation is empty                         | Field is empty                                             | Pass (no consumption required)                                               |

**Why W-level first, not E-level**: fixing obligation dropping **changes the diagnostic set**. C2
requires "same diagnostic set per entry"; going straight to E-level would suddenly make
`yaoxiang check` produce a flood of `Unproven` diagnostics that were previously silent. A two-step
approach (W first to observe, then escalate to E) lets every step's criteria stay usable. See
"Implementation Notes" S4.

**`assert_drained()` is called at exactly one site**: `Driver::run`, at the end of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appears ≥2 times but is read in no third file → fail). `assert_drained()` is the **runtime** gate;
that script is the **static** gate; the two are complementary and both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode (lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Must come from one of the 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); it does
not accept an arbitrary caller-provided array.** This is the key constraint preventing the `Program`
abstraction from degenerating into "anything goes" free parameters, and is asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topological decision about whether to skip, not driven by stage return value
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
        state.obligations.assert_drained();           // The sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision Note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, recorded for reference—
>
> 1. The `StageOutcome` three-state in §4 is not returned by stage arms; it is expressed by `State`
>    failure flags + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, no per-arm
>    boilerplate return);
> 2. `Driver` has no `config` field—the only source of configuration is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" ruling covers the sketch's field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded into the internal `DriverOutcome.skipped`
>    ledger, not emitted externally—the zero-diff criterion of S2 "deliberately not fixing any bug"
>    requires this; external emission comes with 4.3's obligations ledger;
> 4. The `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the only
>    new caller (L1→L2 is the allowed direction); the orchestrator's existing call sites will be
>    migrated away in 4.2 and re-evaluated later.
> 5. In the multi-file form, ProofExecution is an independent stage that runs **after all**
>    typechecking (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof
>    inside the per-file typecheck loop (file N's proof runs before file N+1's typecheck).
>    Single-failure cases are byte-by-byte identical; in the multi-failure case "file1's proof
>    fails + file2's typecheck fails", the first report changes from a proof error to a typecheck
>    error (test pinboard).
> 6. `DriverOutcome` carries products in channels split by `ProgramKind`: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    each entry (`PipelineError` / `OrchestratorError`) are not types that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry; clean files have an empty Vec—matches `check_project`'s
>    current contract); Discovery's product is extended to a triple of "file set + used_by edge
>    set + shadow events" entered into State (no downstream consumer in MultiFile form, only
>    recorded, not emitted externally); the Check path's per-file parse drops from 2 to 1 run,
>    following the C3 topology.
> 8. Lsp form landing (4.2.3, ruled by file-source split, 2026-10-09 user decision): disk-file parse
>    failures follow Check's Plan B (collect + exit compilation unit—both ends degrade
>    consistently); edited-buffer parse failures retain a partial AST and continue to typecheck
>    (editor philosophy—a mid-typing state should not cause semantic features to disappear). The
>    Discovery arm for Lsp lets the buffer source override stale disk content; Typecheck under Lsp
>    only checks the target file (other units only supply signatures to the registry); the in-file
>    diagnostic-order normalization principle extends to LSP (typecheck diagnostics → W-code
>    warnings → proof errors). The old LSP behavior of hard-aborting on unrelated disk-file parse
>    errors and the handler silently falling back to the single-file path is fixed by this step.
> 9. Embedded form landing (4.2.4): `Program` gains a new `shared_registry` field—the embedded-std
>    module is a sub-compilation, the registry is an **input** rather than a product
>    (EMBEDDED_STAGES has no Registry stage), so the #94 SymbolTable sharing contract is made
>    explicit from "caller remembers to pass the same registry" into a program declaration.
>    Error-path text is normalized from `<std.test> (embedded std)` to the unit virtual path
>    `<std/test>` (compiler-internal error surface only, no test pinning). All four orchestrator
>    entries have now been migrated into the Driver.
> 10. Standalone check unified (4.2.5, Ruling A + IR-stage ruling, 2026-10-09): The Check form is a
>     faithful realization of the single-file semantics for a program with no project root—the
>     warning surface only covers the entry file (neighboring files only get errors, prefer omission
>     over false positives); relative `use` is resolved along the importer's directory (rustc
>     single-file mod alignment). The CHECK stage table adds GlobalSlotAlloc + IrGeneration: the
>     standalone old path (the full pipeline) already ran IR generation, and only ir_gen produces
>     E3019/E1014/E1015 (empirically verified by runner gate). IrGeneration's Check form is pure
>     check and does not consume IR; the Script/Bin shape is expressed as module_key's None/Some
>     (E3023's existing switch, ir_gen.rs:1660). Monomorphization is not in CHECK (only produces
>     resource-overrun / internal errors, no corpus dependency). **〔Withdrawn 2026-10-10 after
>     3.4.8 evidence〕**—the "no corpus dependency" is actually evidence-blind for a corpus with no
>     pathological-recursion fixtures; E3005 is the only compile-time line of defense for such
>     programs. See Note #14. Side fix for a latent defect: the Registry arm's duplicate-harvest
>     registration for the embedded-std unit would clobber the native half-face already merged by
>     `with_std()` (spurious E1043)—the embedded unit now skips duplicate registration.
> 11. LSP single-file fallback unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted, replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share one Driver. The
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains a partial AST and
>     continues to typecheck (editor philosophy, extended from the 4.2.3 ruling; not marked as a
>     stage failure, otherwise topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now gains proof (E4018), W-code warnings (W1001–W1005, test pinboard), and IR-level errors;
>     lexical failures now report the real diagnostic (the old synthetic "E0001 lexical error" text
>     disappears with the sequence deletion—real diagnostics carry precise span, with no information
>     loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries have long since converged—the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` with a single bool; the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, with pass-3 and drain modes running in parallel—their difference
>     carried by whether `collected_errors` is non-empty; the five collection points in
>     statements.rs (function body / use / for / block / while). Verification confirmed and fixed a
>     duplicate-diagnostic defect: the collection points put the first error into both
>     `collected_errors` and the Err return channel, and pass-3's `add_error` on Err causes the same
>     code, same span, same message to appear ×2 (×3 when nested); there are also two independent
>     mechanisms: annotation-validation signature parameters and overall annotation double-visit
>     (E1003/E1103), and the ownership layer double-emitting at the same site (E2014/E2018). The fix
>     lands at the module-result boundary: deduplicate by (code, span, message)—a diagnostic is a
>     positional fact, and duplication carries no information; 63 corpus entries have duplicate
>     copies removed, with the `run` column and exit codes showing zero drift (the "duplicate" in
>     `borrow_conflict_err` is an artifact of the baseline triple format not containing column
>     numbers—two E2018s are actually legitimate diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code` / `test_compile` goes through `compile_playground`
>     to construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper is no longer
>     involved. Error text is aligned byte-by-byte with the old `CompileError` Display prefix (Parse
>     error: / Type error: / Internal error:), with zero visible difference in the playground UI.
>     WasmPlayground shares SINGLE_FILE_STAGES + FailFast with SingleFile (the 4.1 declaration; from
>     this step it has a real producer).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, 2026-10-10 user ruling,
>     **partially withdrawing the mono exclusion of Note #10**). Empirical evidence overturns "mono
>     only produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) causes the
>     compiler process to stack-overflow (0xc00000fd—the interpreter recurses in the Rust layer with
>     no graceful runtime error) on the no-mono multi-file path, and the mono depth gate is the only
>     compile-time line of defense; standalone check went through the full pipeline before 4.2.5 and
>     had mono, so the exclusion ruling would cause a silent coverage regression ("no corpus
>     dependency" only proves the corpus has no pathological fixtures). The "type-erasure fallback"
>     assumption is voided by the RFC-033 reflection ruling—`^^List(Int)` needs the real identity of
>     the instantiation, and monomorphization is the foundation of reflection, not an optional
>     optimization. Four design points: ① typecheck produces an `instantiation_request` for
>     qualified calls (`lib.f(x)`, FieldAccess form), the `generic_id` uses the qualified name,
>     sharing the namespace with the merged IR (currently only recognizes bare Var,
>     expressions.rs:2455); ② the `Stage::ALL` topology is changed to Linking before
>     Monomorphization, and the MULTI_FILE stage table adds Monomorphization at the end—consumes
>     `merged_ir`, aggregates all-units requests (the `containing_fn` of the deferred bucket is
>     qualified), elevating mono from a single-module pass to a whole-program pass; the single-file
>     stage table is unchanged (no Linking, subsequence properties are preserved, IR snapshot
>     zero-drift); ③ CHECK adds Linking (pure merge—E3020 entry check stays in RoleClassification to
>     prevent double-reporting) and Monomorphization (pure check, no IR consumption, same as
>     IrGeneration's Check form) two stages; ④ resource protection follows the whole-program BFS to
>     cover cross-unit mutual recursion. The Monomorphizer body is unchanged (consuming ModuleIR +
>     request-set is exactly the merged-IR form's contract).

**Refactoring plan for the ten entries (function-by-function)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                                |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Function body deleted, replaced with constructing `Program { kind: SingleFile, … }` and handing to the Driver                                                                                                    |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                          |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                    |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                      |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to a single Driver call + filtering results to the target file                                                                                                                                           |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                       |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                           |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                                  |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project: change to `Program { kind: Lsp, aggregation: CollectAll }`; **manual lex→parse→… sequence deleted** in the single-file branch, replaced with `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified through `Program { kind: Check }`                                                                                               |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% diverging).**
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
parse loop, per-file checker assembly, result handling. Diverging: rows 2/3/4/5/7/8/9/10/11 in the
11-row table above. **This 40% shared skeleton is exactly the value range of `Driver`**—the 60% that
diverges is all "different stage set" or "different aggregation mode", which is exactly what
`Program`'s two fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking problems, continue
}
```

**Key design decision: "upstream failure causes this stage to be skipped" is NOT a stage return
value.**

The reason: skipping is **topologically** determined, not by the stage itself. If every stage
returned its own `Skipped`, then the information of "why I didn't run" would be scattered across 12
stages and impossible to audit centrally. Change to: `dispatch` decides at the start of the loop
based on dependency:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` MUST produce a diagnostic.** This rule directly targets the bug this document aims to
fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent "skip" that
produces no diagnostic, just because the obligation happens to be empty. Once the rule is
established, every "X wasn't done so Y didn't run" must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                  | Text Direction                                                                                                                                 |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort`                                        | "Stage Y was not executed because upstream stage X failed"                                                                                     |
| Predecessor obligation empty                            | "Stage Y was not executed because there were no pending obligations (normal)"—**this one has severity = Info, does not count toward warnings** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y was not executed because the configuration is not enabled"                                                                            |

The second is key: it lets "normal skip" and "abnormal skip" be distinguished in the diagnostic
stream, and does not pollute `yaoxiang check`'s `warning_count` (`diagnostic/mod.rs:637-641` depends
on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error stops (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                             | Current Equivalent |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error return `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early returns)               | Exists             |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists             |

**`Aggregation` must be a field of `Program`, not a global setting of the Driver**—LSP serves both
in-project and single-file files within the same process, while CLI's `run` and `check` are two
independent calls; placing it on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early-return vs
not"—they are two different checker entries. After unification, the same implementation should be
driven by the `Aggregation` parameter, rather than keeping two functions (**this is an additional
item this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-level Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation; does not touch `MonoType` / `PolyType` / `ir::Type`                                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is the **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the rest of the field types stay in
their original places; `driver/obligations.rs` only holds the aggregate container and
`assert_drained()`. L3 consumers (e.g. `ir_gen` reading `release_plan`) continue to receive concrete
field types via parameters, **and must not `use crate::driver`**—consistent with the
driver-no-backward-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not `use crate::driver`
backward**. A `Driver` appearing in `TypeChecker`'s imports is a violation, caught by the
`scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                                                                            |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution is already integrated as a P3 stopgap; **the warning surface does not gain new entries** (Decision B1, 2026-10-09: dead-code-family warnings do not enter the compile path—the pool semantics are being corrected by the 4.9 bug ruling, and normalizing the warning surface will be revisited after 4.9; the original draft "new warning outputs" on this line is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already integrated as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                       |
| LSP (in-project)                    | No proof_execution                       | Already integrated as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                       |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                                               |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                                                                                   |

**The only intentional behavior break** is the `.expect()` at `predicate.rs:35` being changed to
return a diagnostic. This is consistent with RFC-027 §8's "not bound to a specific solver" stance,
and also with the contract already declared at `backend.rs:60` ("caller should conservatively
degrade")—the current implementation is opposite to its own contract document.

### Compiler Change Inventory

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                                                   |
| -------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                           |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, replaced with `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and used as the `Stage::ProofExecution` implementation                                                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, replaced with `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` routed through Driver                                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The `check_module` / `check_module_collect_all` two-way in `typecheck_with_registry_in` is now driven by the `Aggregation` parameter                                                                                                                                                                                                                                                     |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | The manual stage sequence in `run_diagnostics` is deleted, replaced with a Driver call                                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified through `Program { kind: Check }`                                                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                                          |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently falling through `Compiler::compile_with_source` to Path 1 implicitly)                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | The empty `ProofResult::Unproven { .. } => {}` arm changed to produce a diagnostic (**fixes the second silent-drop point**)                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct the comment that contradicts `1320`                                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to the **actual** execution order; delete "if a lower layer fails, higher ones do not run" (no short-circuit exists)                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add a process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure entry point; the three consumer sites in `predicate.rs` / `checker.rs` / `ownership.rs` are routed through the shared entry. The `&'static` bare-reference form is not feasible (because `dyn Solver` is not `Sync`); the original text is governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` (the logic
itself is correct—**the problem is on the consumer side, not the producer side**, changing the
producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (wasm-branch reachability determination goes to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                       | Compatibility                                                       | Disposition                                                                                                                                         |
| ------------------------------------------------------------ | ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                         | **Break**: previously silently-passing constraints now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; ship in batches, expect diagnostic-set changes to go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                       | **Break**: programs that compiled now produce warnings              | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                                    |
| First run of `assert_drained()` produces W-level diagnostics | **Break**: diagnostic set grows                                     | W first, then E; S2/S4 are two steps                                                                                                                |
| `predicate.rs` panic changed to diagnostic                   | **Improvement**: no longer crashes                                  | No break                                                                                                                                            |
| `build` / `dump_bytecode` subcommands                        | **No impact**                                                       | These two paths do not go through proof_execution                                                                                                   |
| `TypeCheckResult` fields migrated to `Obligations`           | **Internal refactoring**                                            | If the `pub` API surface has external dependencies, they need to be synchronized; in-repo consumers are fully enumerated in the change inventory    |

---

## Implementation Notes

This document corresponds to **P3 (Fix Correctness Bugs)** and **P4 (Stage Contract and Unified
Driver)** in the RFC-039 global stage sequence. S1–S5 below are the implementation order within this
document; **each step's acceptance criteria reference the C2 category of the
[Equivalence Criteria document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Bug criteria must first be written to fail.

| Deliverable                                                                                                                                                    | Acceptance                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red**. If it is unexpectedly green, the bug analysis in this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Resolution D48—not mixed into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of single-file and multi-file versions of the corpus can be compared                |
| `test_obligations_drained` skeleton                                                                                                                            | Marked `#[ignore]`, turned green in S4                                                                  |

**Rollback point**: no code change, pure test addition.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to a Driver call           | **Single-file-path diagnostic set and exit code byte-for-byte identical**                       |
| Whole corpus (293 `.yx`) diff                            | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately does not fix any bug**—it only moves existing behavior into the Driver.
The acceptance criterion is zero-diff at the C1/C2 level.

**Rollback point**: `git revert` a single commit, and `pipeline.rs` is restored.

### S3: Merge the Four Orchestrator Entries + Fix the Bug

| Deliverable                                                                                                                     | Acceptance (C2)                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The diagnostic sets of the four entries, normalized by `Aggregation`, are the same |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in and out of projects                       |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI produce the same diagnostic set for the same file                      |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | The bug is fixed                                                                   |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` produces a diagnostic in any mode    |
| `predicate.rs:34-36` `.expect()` changed to a diagnostic                                                                        | No longer panics in the absence of Z3                                              |

**Rollback point**: the bug fix and the structural merge are **two commits**. If the structural
merge has problems, only the structural commit can be rolled back while keeping the bug-fix
commit—`test_multifile_proof_obligation_not_dropped` stays green; the reverse (keep structure, roll
back fix) goes red, which is an unacceptable intermediate state and must not be merged.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                   |
| -------------------------------------------- | ------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                       |
| `scripts/ci/check-obligations.py`            | Static gate active                                           |
| Obligation diagnostic W → E escalation       | After whole-corpus diff, manually review every newly added E |

**Rollback point**: the severity of `assert_drained()` is configurable; switching W/E does not
require code-structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                          | Acceptance                                                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to the actual order                                                                                                                                                                           | Documentation corresponds one-to-one with the call sites in `checker.rs`                                                                                                  |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: `Unknown` must not enter the cache**—otherwise a single timeout would be固化 across the process via the cache and pollute the cargo test thread-shared state) | Production-path `default_solver()` call sites go to zero (tests only); cache hit rate is observable across the three consumer sites (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds a warning                                                                                                                                                                                    | Diagnostic exists when there is no `body_checker`                                                                                                                         |
| Reachability determination of the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                            | Conclusion goes to `06-cleanup-inventory.md`; this document only records the determination need                                                                           |

**Note**: correcting the layer order will change the diagnostic set and may expose many
previously-silent `Unproven`s. **This item should walk independently of S1–S4**, not mixed with the
entry merge in the same PR.

## Key Decisions and Rationale

| Decision                 | Determination                                                                                                                                                    | Rationale                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum; **runtime registration is rejected**                                                                       | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism already validated by the opcode table in RFC-039 routing table B. `StageScope` further turns "per-module vs project-level" into a type fact, eliminating the call-count questions that need human reasoning |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, combined with `scripts/ci/check-obligations.py` for **static** gate; W first, then E for severity | What this fixes is a whole class of bugs, not one: verified same-class hazards are at least 2 (`proof_calls`, `checker.rs:1313`), and all 16 span-keyed fields are in scope. W first then E is so that every step's C2 criterion stays usable—going straight to E would suddenly make `yaoxiang check` produce a flood of previously-silent diagnostics             |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the six `Unproven` branches' producer-side logic at `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic is fixed", but the logic of those three branches is itself correct                                                                                                                                                             |

This design additionally closes two kinds of cracks:

- **The crack between "comment promises" and "code behavior"**. The "silent pass must not be
  resurrected" at `checker.rs:5165` and the "if a lower layer fails, higher ones do not run" at
  `layers/README.md:3` are both **comment-level contracts**—the former is honored by
  `assert_drained()`, the latter is exposed by the topology-driven `Skipped` diagnostic.
- **"One array forgotten append"**. Currently five functions each hand-write the call sequence;
  after unification there is one stage table, and missing-wiring of a new stage is a compile-time
  error.

### Unadopted Directions

- **Generic stage chain `Stage<A, B>`**
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)—7 of the 11 inconsistencies (two
  implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type hand-off**; they are configuration questions of "should
  we run a certain analysis"; it also cannot express `Aggregation`, only relocating the
  `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—abandons compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile—equivalent to
  swapping "five functions each hand-writing the call" for "one array forgotten append", which is
  exactly the cause of the 11 inconsistencies. The repo already has a same-form counterexample: the
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only patch the bug without touching the structure**—can fix the 2 known bugs, but only covers 1
  of the 16 obligation fields; `Stage` is still scattered across 5 functions, and the next fork will
  keep growing from here. **It must be done first** (it is part of S1/S3), because the structural
  refactoring needs a known-red test to prove the criterion is valid.
- **Make `proof_calls` `pub` and add a `debug_assert`**—`check_module` is a generic entry, it
  doesn't know who the caller is, and `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is **wholly** discarded. This is looking for the bug at
  the wrong level: the bug is at the orchestration layer, and detection must be at the orchestration
  layer.

## Known Limitations and Risks

- **The slimming of `orchestrator.rs` will make it harder to read in the short term**. The 139-line
  `compile_project` is split into "Program constructor + several driver arms", and readers need to
  cross two files to understand the flow. This is the shared cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, in the direction of "exposing previously-silent
  problems"**. After the fix there may be a batch of "new error" user reports—they're real bugs that
  just weren't being reported. This must be clearly stated in the CHANGELOG.
- **`assert_drained()`'s severity switching requires per-field manual judgment**. Among the 16
  fields, some (e.g. `module_namespaces`) may have "no consumer" by design and should not alarm. S4
  needs to walk through them one by one—not a blanket switch.
- **The `Program` abstraction may be premature**. If some entry's stage set remains unstable for a
  long time, `stages()` degenerates into "every call passes a different array" free parameters, and
  the contract constraint becomes empty. **The mitigation is `test_program_stage_coverage`, which
  asserts that `stages()` can only come from the 6 predefined combinations.**
- **Criterion dependency**: the bug-fix criteria of S1/S3 depend on the bug tests being red first,
  per the [Equivalence Criteria document](07-equivalence-oracle.md). **If that document has not
  first established a multi-file corpus layer, S1 cannot be accepted**—because the existing 293
  corpus files all go through the single-file path, with zero coverage of this class of defect. This
  document does not involve the IR validator (`verify_loose`); that prerequisite work is out of
  scope for this document.
- **Layer-order correction will expose previously-silent `Unproven`s** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order contradicts the declaration). This is a
  diagnostic-set change risk and should walk independently.
- **Thread safety after changing the SMT backend to a singleton is undetermined**.
  `backend.rs:13-19` already states that `Solver` is only `Send`, not `Sync`; the `Z3Backend` cache
  is `RefCell`; after changing to a cross-compilation-unit shared singleton, all access paths must
  be confirmed to go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelism would **mask order-dependence defects** (such as the actual order dependence between
  termination and ownership). This should be revisited after the equivalence criteria are stable.

> **The open questions originally listed in this section have all been ruled on.** See the
> [RFC-039 Rulings Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> determination. **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup; the
  `include!` refactoring of `checker/semantic_tokens.rs` (implementation steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three-level criteria, the
  `test_multifile_proof_obligation_not_dropped` bug criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design origin of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — The code
  table compared by the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "silent pass must
  not be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — The explicit declaration of "if a lower layer
  fails, higher ones do not run" (in reality there is no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit rationale for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — The 16 obligation fields that downstream must
  consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The single test that
  self-declares "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — The corpus
  self-declares "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, so it
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — `cdylib` + `wasm-bindgen` live in the shim crate, not the main crate
- `build.rs:19-55` — The error-code build-time gate, a paradigm of this project's enforcement
  mechanism
