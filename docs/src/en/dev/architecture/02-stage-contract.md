# Compilation Stage Contract and Obligation Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage sequence are in the main body
> of RFC-039; the positioning of each subsidiary document is at [this directory index](index.md).

## Positioning and Scope

This document is the **L1 Orchestration Layer** construction blueprint of RFC-039. It addresses one
problem: **"which stages ran in this compilation, which didn't, and why" is currently scattered
across six paths and ten entry functions, each hand-wired, with no single place that can answer this
question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by the architecture, not the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally reads "Unproven → compile
error, no degradation, no silent pass... silent pass must not be allowed to revive". But this
invariant only holds on the **single-file path**: proof function constraints like `x: Sorted(3)`
silently pass on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths—the proof function
never executes and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and per-entry-function transformation for all ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  sequence.

### Not Covered

- Four-layer model, dependency direction rules, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA-ization →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution sequence and G1-G10 acceptance gates → main body of RFC-039

### Division of Labor with RFC-039

RFC-039 provides the **why** of the refactoring and the **order in which to do it**; this document
provides L1's **specific form**, **per-file change list**, and **implementation stages within this
document** (corresponding to RFC-039's global sequence P3 "fix correctness holes" and P4 "stage
contract and unified Driver"). Where this document conflicts with RFC-039, RFC-039 takes precedence.

Equivalence criteria are executed under the **C2 (orchestration change)** category of the
[equivalence criteria document](07-equivalence-oracle.md): identical diagnostic set per entry +
identical corpus behavior.

## Current State

> All items in this section are **verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Unique "One Wrong Write, Whole Repo Silent" Structural Defect

It differs in nature from the other two classes of defects (module boundaries, test wiring):

| Defect                   | Typical Manifestation            | Is There a Signal    |
| ------------------------ | -------------------------------- | -------------------- |
| Lexical/syntax error     | Source code written wrong        | Has diagnostic       |
| Type mismatch            | Type written wrong               | Has diagnostic       |
| Stage missing connection | Some stage simply isn't called   | **No signal at all** |
| Obligation unconsumed    | Field filled but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
is written, not **what is not written**. This is exactly the root cause diagnosed in RFC-039: "this
project treats 'design' as a documentation convention, not as an executable constraint".

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms, just not extended to the stage layer:

- **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are hard-gated at build time by
  `build.rs:19-55` via `tools/code-tables`, which compares against the RFC-013 code table
  item-by-item; any inconsistency results in `panic!` refusing compilation.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` directory is 6227 lines, 47.9%) has
  high test density, and each module's test subtree is wired— this is the most complete piece of
  test wiring in the entire repo and can serve as a formal reference for the stage-layer gate.

**The design capability is sufficient. What's missing is "putting a gate of the same level in the
orchestration layer too".**

(Line count basis: `(Get-Content).Count`, see the "Line count basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                    |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | Direct 5-stage call                                  |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                     |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking               |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics          |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                   |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR             |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → Path 1                    |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (Path 3a)                        |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project takes Path 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                        |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → Path 1             |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)— RFC-027 Phase 2.5                  |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below provides evidence cell by cell. **A blank does not mean that entry "doesn't do this
thing", but "no code in that entry does this thing"**—this is the essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single file)                         | `compile_project`                                       | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                  | **No**                                                       | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                  | Yes `356-377` (role-aware, no config gating)                 | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                  | Yes `336-348`                                                | **No**                             | **No**                                |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                          | Yes `350`                                                    | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                  | Yes (same as 2)                                              | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                  | Not applicable                                               | Not applicable                     | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | Not applicable                                   | `discover` (`101`, discards `used_by`/`shadow_events`)  | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | Not applicable                        |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                  | Yes (`321-328`)                                              | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | Not applicable                                   | Yes `allocate_global_slots` (`147`)                     | **No**                                                       | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                      | Yes (`152-236`)                                         | **No**                                                       | **No**                             | Yes (independent ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline, preserved unchanged. Differences from current state after P3
> lands:
>
> - **Row 1 has been fixed**: proof_execution shares the same implementation in
>   `frontend/proof_execution.rs` across all five entry points (pipeline + four orchestrator
>   entries); the single-consumer defect no longer exists.
> - **Rows 2/4/5 current state unchanged** (multi-file `run` still does not output W1003, does not
>   run dead code family) → goes to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single file) → goes to WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk unverified defect).
> - **Row 7** check_module / check_module_collect_all dual entry → goes to 4.2.7 (Aggregation
>   parameter-driven).
> - The "Ruling #434" referenced in the code side had no prior docs registration—registered as
>   RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new fact outside table, registered 2026-10-09)**: the
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch #117), but multi-file `discover` previously only recognized
>   explicit `use`—in-project for-loops compiled successfully but failed at runtime with E6006
>   (probe evidence). Fixed and crossed off WBS 4.10.2; the "multi-file missing a single-file step"
>   parse hard-abort quirk in the same family is registered as WBS 4.10.1—**also fixed** (landed
>   right after 4.2.2, Check path degraded to per-file collection; from this audit's "stage coverage
>   / field consumption" dimension, this is a compilation unit member difference, a missed
>   dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   the Registry arm repeatedly harvests registration for embedded std units via
>   `extract_module_info`, which overwrites the native half-face that `with_std()` has already
>   registered as "native + yx surface merge" (`result.is_err` etc. lost → std.test false-reporting
>   E1043). This audit's third missed dimension: "merge semantics for two registration sources of
>   the same module key"—the single-file path's registry is formed only once via `with_std()`, while
>   the multi-file/Check Registry arm inserts unit by unit to have coverage.
>
> **Two items require precise expression, otherwise mistakes will be made:**
>
> - **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
>   (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
>   **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason
>   is at `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult`
>   (including `warnings`), but this function **has no `result.warnings` read point anywhere**; the
>   only downstream use of `result` is `generate_ir_with_context` (`154-156`).
> - **The `main` criterion for the entry in row 11 is also not from the same source**:
>   `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
>   `is_bin_role` (`250-252`, only checks "whether there is a manifest"). The two functions give
>   different answers to "what file must define `main`".

### Correctness Hole: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the entire
repo that constructs a non-empty `proof_calls`:

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
so this proof function must actually be executed to decide".

**Step 2 — The consumption point is inside the checker, but it only sends without consuming.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original text of the comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a true defense line rather than a forward-looking fallback: silent pass must not be allowed to revive.`

That is: the author knowingly "produces no diagnostic" as a defect, classifying it explicitly as a
**known intermediate state with nowhere to go**, with the design assumption that "someone will
definitely come to read `proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it, `checker.rs:1439` writes into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined in
`types.rs:29`.

**Step 4 — The entire repo has only one read point.** `TypeCheckResult.proof_calls` (`types.rs:29`)
is read in production code in **only one place, `src/frontend/pipeline.rs:187`** (passed as argument
at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repo fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` is the production-side collection;
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four entry points of the orchestrator do not go through that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, directly calling `TypeChecker::check_module` (`124` / `493` / `1386`). Therefore
they don't even reach the sole read point at `pipeline.rs:187`.

**Step 6 — Consequence: even the refinement obligations of the standard library itself take the
dropped path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std module itself are
likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP paths **compiles, runs, with
no diagnostic at all**. The proof function is never called.

**Step 8 (Supplementary verification) — There is a second silent drop point within the layer.** When
`checker.rs:1303-1314` handles the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and
no bookkeeping.** This directly contradicts the promise of `5164` branch "silent pass must not be
allowed to revive", and is **independent of the orchestrator problem**—even if the entry layer is
completely fixed, this point will still be silent. (Processing timing: should be processed in the
same batch as the obligation ledger, because it is the same kind of problem.)

### Why Tests Did Not Catch This

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** The multi-file path's proof obligation has zero coverage.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping exactly before `pipeline.rs`**.

The test files' own doc comments have written this out. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are in .yx layer`

**This is exactly the shape of the problem: tests verify "obligations are filled", while the bug is
"the consumer doesn't read them".** A test that only tests the production side and not the
consumption side is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Main Corpus Is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) treats the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion**, as the main acceptance
means for category C2 stages. But actually tested:

- **There is no `yaoxiang.toml` anywhere in the `tests/` directory** (zero hits by full-directory
  glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for each corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, none
  covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is exactly
the proof of this point. This file is annotated `// expect: compile-error E4018` (line 15), and its
header comment line 7 says "Status: ❌ should be rejected at compile time"—**it can pass precisely
because it takes the only path that actually executes the proof function**.

**Conclusion: The third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation Points"
S1.

### Obligation Fields: 14 "Production Is Contract" Fields, Zero Mechanism Guarantee

In `src/frontend/core/typecheck/types.rs:16-73`'s `TypeCheckResult`, there are **14 fields** whose
documentation comments explicitly say "produced by stage X → consumed by stage Y", meaning they are
essentially **cross-stage obligations**:

| Field                     | Definition Line | Producer → Consumer                                             |
| ------------------------- | --------------- | --------------------------------------------------------------- |
| `proof_calls`             | `36`            | typecheck → **proof_execution**                                 |
| `release_plan`            | `38`            | ownership → IR generation (`ir_gen.rs:341` reads, `:1942` uses) |
| `escaped_refs`            | `40`            | ownership → IR generation                                       |
| `instantiation_requests`  | `42`            | typecheck → monomorphization (`pipeline.rs:381`)                |
| `existential_coercions`   | `44`            | typecheck → IR generation                                       |
| `implementation_proofs`   | `46`            | typecheck → IR generation                                       |
| `interface_impl_registry` | `49`            | typecheck → operator query / constraint solver / LSP            |
| `sum_types`               | `51`            | typecheck → IR generation                                       |
| `sum_type_param_names`    | `54`            | typecheck → IR generation                                       |
| `operator_dispatches`     | `56`            | typecheck → IR generation (span-keyed)                          |
| `overload_resolutions`    | `59`            | typecheck → IR generation (span-keyed)                          |
| `try_expr_impls`          | `62`            | typecheck → IR generation (span-keyed)                          |
| `match_scrutinee_types`   | `66`            | typecheck → IR generation (span-keyed)                          |
| `module_namespaces`       | `69`            | typecheck → IR generation                                       |

**All these fields are passed across layers keyed by `Span` or name tables.** The equivalence
criteria document has already pointed out that span-keyed contract mismatches will "silently fail,
with no error" (`ReleasePlan` is typical: any inconsistency in how the two sides compute span →
`Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer `ir_gen.rs:341` (`release_plan: type_result.release_plan.drops.clone()`)
  is inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in the downstream module, and the downstream module is
  on the must-pass path of all entries.
- `proof_calls`'s consumer is at the **top level of `pipeline.rs`**, belonging to **another entry
  implementation**. The orchestrator's four entries don't go through `pipeline.rs` at all.

**Verified fact**: there is currently **no mechanism** in the entire repo that guarantees these 14
fields are consumed. The only "protection" is that `ReleasePlan` happens to ride on the coattails of
IR generation.

### Four Additional Contract Defects in the Proof Layer

All below come from the proof layer analysis, independent of the entry-fork problem, but all belong
to the category "stage contracts are not enforced".

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "execute in layer order, lower layer failure means upper layer doesn't run".
Actual call points in `TypeChecker::check_module`:

| Actual Order | Call Point                                        | Declared Layer |
| ------------ | ------------------------------------------------- | -------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2        |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1        |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3        |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0        |

That is: the order of `termination` and `ownership` is **reversed** from declaration; the declared
Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all** (it is only
used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time **there is no short-circuit**—`checker.rs:1271-1276` adds
termination errors one by one via `add_error` and continues, so the README's promise of "lower layer
failure means upper layer doesn't run" doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic was eliminated by P3's 3.3.1 (SOLVER slot
> made `Option`, missing is conservatively degraded to `SMTResult::Unknown`); "silently skip (don't
> inject)" was supplemented with a W1081 signal by 3.4.2. The unification of the three forms
> (singleton-ization) is executed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure entry),
> and the `default_solver() → &'static` statement in the 02 change list is governed by D58
> (`&'static` raw reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Construction-time injection `with_solver_owned`          | `None` → **Silently skip** (don't inject)                 |
| `ownership.rs:627` (back-edge cut decision)             | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively don't cut |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure keeps
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'". But the contract document at `backend.rs:60` says "`None`: backend
unavailable…**caller should conservatively degrade**". **Two philosophies coexist in the same code,
and the hard-failure one panics rather than returning an error.**

**(c) A new Z3 context is created for every back-edge decision, the cache is in name only.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge cut decision. But
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—each call does `Z3Backend::new()`, i.e. creates a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(Comparison: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. The same backend, two
lifecycle strategies.)

**(d) Checker silently degrades + comment doesn't match reality.** `checker.rs:1283-1287` and
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
table** and runs as normal—ownership analysis degrades to "nothing conflicts", **with no warning**.
(Handling suggestion: should be recorded as a warning-level diagnostic, or at least left as a trace
in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving". Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination at
`1256`, ownership after at `1296`, i.e. the comment's "before" is actually "two layers after"**.

### wasm Status: Shim Crate Carries It, 27-File Branch Is Alive

The wasm target **is built and built in CI**, with `cdylib` not in the main crate but in a separate
shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                      |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate only `rlib`                            | `Cargo.toml:29-31`                                                                                                                                                                                                                                                            |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                            |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                   |
| Main crate has wasm target dependencies           | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (corresponding non-wasm segment at `Cargo.toml:125-131`: tokio/ureq/tempfile)                                                                                                   |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                      |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack to `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetch `libz3.a` from fixed URL, degrade to warning if missing)                                                                                                                                                             |
| **27 files** have `wasm32` literal                | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only doc-mentioned                                                                                                                                       |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                    |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                    |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate can be called under the wasm target by `wasm/src/lib.rs`
(73 lines)—`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole (all
three need `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Does it execute the proof function |
| ------------------------------------------ | ---------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                            |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                            |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                            |
| Multi-file `run` (`compile_project`)       | **No**                             |
| `check` (`check_project`)                  | **No**                             |
| LSP in-project (`check_source_in_project`) | **No**                             |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—goes to `06-cleanup-inventory.md`. The open question in RFC-039 "build wasm
target, or delete these branches" **is already obsolete**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is prohibited.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer"—this is exactly the mechanism already proven effective by the
opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check         Project
    Discovery,          // File discovery                    Project
    Parsing,            // Lex + syntax                      PerModule
    Registry,           // Module registry construction      Project
    RoleClassification, // Role classification (Script/Bin/…)  Project
    Typecheck,          // Type check (includes embedded proof layers)  PerModule
    DeadCodeAnalysis,   // Dead code family analysis         Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                    PerModule
    Monomorphization,   // Monomorphization                  Project
    Linking,            // Cross-module linking / IR merge   Project
}

impl Stage {
    /// All stages. When adding a new variant, this array and the exhaustive match in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs once
at project level) make "which stages must run per module and which must run once at project level" a
type-level fact. The `Project`-scope stages are structurally guaranteed to run only once in
`dispatch`—this eliminates questions like "should `allocate_global_slots` be called per file" in the
current `orchestrator.rs` that require human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the original draft are
> corrected per implementation evidence—
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the original draft's order contradicts
>    the data flow.
> 2. The Check form's stage table is normalized per `Stage::ALL` topological order: dead code
>    **before** proof: `check_project`'s current state is proof before dead code, with no data
>    dependency between them and identical diagnostic sets (C2 set semantics); only intra-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to ALL order, so byte-level acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    original draft's order would force the Registry arm to "hide parse", making the stage table lie
>    about data flow; and multi-file parse is thus reduced from 2 times to 1 time. The current state
>    is honestly recorded: multi-file path's parse error was once a hard abort
>    (`build_registry_from`'s `?` propagation, the tension with CollectAll semantics registered as
>    WBS 4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling: Rust-style collection
>    semantics + Plan B): Check path parse failure is degraded to per-file diagnostic collection,
>    sick files exit the compilation unit (don't enter registry, importers report E5001); MultiFile
>    path retains hard abort (under FailFast a bad file can't produce IR, which is correct
>    semantics, the pinning is long-term valid).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edges 14→16: `RoleClassification` consumes Discovery's used_by edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadowing events—the original draft missed these (these two products
>    were produced by `discover_with_used` in the orchestrator era, wrapped and discarded by
>    `discover`, the data flow was lost when not put in the table); b. Intra-file diagnostic order
>    normalization extended to the complete form of note #2: E3020 entry check is moved into the
>    RoleClassification arm (Check has no Linking stage, the arm holding surfaces and AST is
>    responsible for the check), intra-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); diagnostic set is unchanged (C2 set semantics),
>    only stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but unconsumed" from undetectable into a fact assertable at
compile time or test time. RFC-039 has already listed this as "the highest-value item".

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
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                                   | Disposition                                                         |
| ------------------------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------- |
| Obligation read by declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                |
| Obligation non-empty but no consumer | Field non-empty and not consumed                            | **Produce diagnostic** (W-level, default; E-level in `strict` mode) |
| Obligation empty                     | Field is empty                                              | Pass (no consumption required)                                      |

**Why W-level first, not E-level**: fixing obligation drops will **change the diagnostic set**. The
C2 criterion requires "identical diagnostic set per entry"; if promoted to E-level in one step,
`yaoxiang check` will suddenly have a large number of previously-silent `Unproven` diagnostics. Two
steps (W observation first, then E) allow each step's criterion to be usable. See "Implementation
Points" S4.

**`assert_drained()`'s sole call point**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Supporting static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third file reads → fail). `assert_drained()` is the **runtime** gate; this script is
the **static** gate; both are complementary and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note registers the
> **implementation form, measured numbers, and 4.3.2 interface** of the above sketch and three-tier
> table, with item-by-item correspondence to code file:line. The landing commit is `1941cc47a`
> (verification tree `99079bc52`). It is simultaneously the **third** of the red list "non-silent"
> three-point burden (the other two: audit surface, static gate).
>
> **I. Landing artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 14-item obligation inventory `INVENTORY` (:136, field
>   name/consumer stage/red list), delivery mark ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (:518), outbound `emit_obligation_diagnostics` (:595), non-consumer gate
>   `consumer_visible_unit` (:479), skip criterion `consumer_stage_skipped` (:490), IrGeneration
>   delivery set `IR_GENERATION_DELIVERED` (:232), W1090 code string constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult` — `src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:511). The early-exit path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; remove any one, runtime W1090 must fire): ProofExecution
>   `src/driver/arms.rs:554` (`proof_calls()` is the only external read path of `TypeCheckResult`
>   for this obligation, `types.rs:84`); IrGeneration `arms.rs:672` (Check) / `:726` (Embedded) /
>   `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four places with the same delivery
>   port: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   conservation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by use cases.
> - Diagnostic code: W1090 registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, the reason is written above the entry :26-31 — settlement granularity is
>   "unit × obligation", without a user-pointable source location, so no dummy span is inserted);
>   six-language text in `locales/*.json`.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, includes `--selftest`).
>
> **II. Three-tier table item-by-item landing + two additional tiers**
>
> | Three-Tier Table Situation Above     | Implementation Criterion                                                               | Code                   |
> | ------------------------------------ | -------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer | **Delivery mark exists** (mark ↔ real read reconciled by static gate, see paragraph 5) | obligations.rs:559-563 |
> | Obligation non-empty but no consumer | Non-empty + consumer stage ran + no mark → W1090                                       | obligations.rs:565-570 |
> | Obligation empty                     | `Obligation::is_produced` checks field-by-field emptiness → pass                       | obligations.rs:529-531 |
>
> Two **additional tiers** (2026-10-13 user ruling, not in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **no
>    judgment of underflow** (obligations.rs:534-537). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (:121-126).
> 2. **Skip exemption**: Consumer stage is skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → no judgment of underflow (obligations.rs:539-542). `NoObligations` does
>    **not** get exemption—when the obligation is non-empty but the stage self-declares no
>    obligation, the ledger is unbalanced, and it must report.
>
> **III. Outbound specification (ruling "C") and the measured numbers behind it**
>
> This mechanism only outputs **W-level** (W1090); **no strict switch**, "same code promoted to E or
> new E code" goes to WBS 4.3.2 (RFC-039 D5 two-step). Mechanism facts (measured): severity is taken
> explicitly via `.severity()`, otherwise "code starting with `W` → Warning"
> (`codes/builder.rs:186-203`), the registry's `ErrorCategory` does **not** participate in severity;
> exit code is determined by `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and
> `--deny-warnings` (`src/main.rs:567-568`); W itself doesn't change exit code.
>
> **Red list fields don't enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C),
> the reason is reproducible:
>
> - Ruling A (red list output as W) measured cost: `interface_impl_registry` is filled by the
>   typecheck pre-set table, **always non-empty**, per-compilation output is a **constant signal**
>   (no discriminating power), but it breaks three existing external contracts—"clean program zero
>   diagnostic / LSP zero diagnostic / `warning_count == 0`": `cargo test --lib --all-features`
>   measured **12 existing use cases turn red**, of which **8 are in out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 are in `src/driver/tests/`), corpus
>   baseline 24/342 fixtures change, and the only feasible fix is "filter W1090 out of
>   zero-diagnostic assertions" = relax existing criteria.
> - Ruling B (downgrade to Info) would still break 9/12 of the list-class assertions, and Info is
>   not "output by ruling level".
> - Ruling C measured: **zero existing use case changes**, corpus baseline **zero change** (342/342
>   "diff: empty, behavior equivalent"), runtime W1090 becomes a **discriminating regression
>   probe**—healthy tree is silent all the way, only fires when a delivery mark is removed (each of
>   the three delivery points did mark-removal mutation, CLI `check` prints 2 `warning [W1090]` when
>   the Check arm's mark is removed).
>
> The "non-silent" of the red list is borne by three points: ① `DriverOutcome.obligations.findings`
> audit surface (carried with every run, including WBS id and evidence pointers); ②
> `check-obligations.py` prints the red list every run (currently 1 item) + staleness detection; ③
> this note.
>
> **IV. Red list current state (clearance progress) and 3.4.4/3.4.5 landing records**
>
> The red list went from 3 to **1**—3.4.4 and 3.4.5 have been cleared and landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is fulfilled by IR-side in-place form detection equivalence:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST form
>   `FieldAccess{Call(Var)/Var}` + takes declaration order from `sum_type_variants`), call point
>   `:7496` replaces the entire Call with `CreateVariant`; this function's doc explicitly states
>   "span-keyed cross-system is unreliable (the two sides have different semantics for Call node
>   span), so in-place detection" —**wiring the consumer side would bring back the already-fixed
>   span mismatch defect**. Deletion surface: `types.rs` field, `checker.rs:1489-1493/:1565`
>   collection, `inference/expressions.rs` and `inference/statements.rs` collection chain,
>   `environment.rs`'s `VariantCtorCall` struct; `infer_variant_ctor_call`'s type inference and all
>   diagnostics **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (keep in phase candidate name
>   table and `overload_resolutions`). All read/write points: written in phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550` decision point, written to `overload_resolutions` at `:4550` when
>   sole hit); the deleted one is the **zero-reader mirror** poured into `TypeCheckResult` at
>   `checker.rs:1558`. Naming fact still single-point expressed: candidate name table (in phase) +
>   span-keyed decision (cross-stage, `ir_gen.rs:380` constructs, `:7632/:7705` consumes);
>   definition-side mixed-compiled name is self-calculated by `method_def_ordinals` (`:1828`).
>
> Red list remaining after clearance ("non-silent" still borne by three points):
>
> | Field                     | Clearance Task (09-WBS) | "Zero Consumer" Evidence                                                                                             |
> | ------------------------- | ----------------------- | -------------------------------------------------------------------------------------------------------------------- |
> | `interface_impl_registry` | 3.4.9                   | Outside-typecheck tree sole hit `src/frontend/module/mod.rs:92` is doc comment (not code read); `src/lsp/` zero hits |
>
> Specification synchronized with clearance: obligation inventory 16 → **14** (`INVENTORY` /
> `Obligation::ALL` / this document §Obligation Field table); gate `--selftest` adds two more
> "inventory out of sync" counter-examples (code deleted, inventory not synchronized / inventory
> deleted, code still there), see `tests/baselines/obligations-gate/{stale-manifest,stale-struct}/`.
>
> **V. Static gate (9.2) actual criteria** (three more hard criteria than the one-line sketch)
>
> 1. **Three-point reconciliation**: `obligations.rs`'s `INVENTORY` ↔ `TypeCheckResult` field
>    definitions ↔ this document "§Obligation Field" table (including the table header count
>    declaration), any drift is red;
> 2. **Read evidence**: non-red-list obligations must have real reads in the **declared consumer
>    file** (`.field_name` member access; comments/tests/strings don't count)—this makes "delivery
>    mark ↔ downstream real read" a hard criterion (the main risk of mechanism A is exactly that the
>    two drift);
> 3. **Delivery mark**: non-red-list obligations must be marked at delivery point; red list
>    conversely must not be marked;
> 4. **Red list staleness detection**: red list fields must have zero real reads (a consumer growing
>    is red), and each carries a WBS id and evidence pointer; Additionally `--selftest`: comes with
>    counter/positive fixtures, counter must be red, positive must be green (proving the probe
>    doesn't idle).
>
> **VI. Item-by-item deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy field by field**: obligation
>    data is still held by `TypeCheckResult`, the ledger only records "inventory + delivery marks +
>    settlement conclusion". Reason: copying means two sources of truth (each field produced
>    separately by typecheck's checker and ownership layer), guaranteed to drift; cross-layer data
>    surface also shouldn't be replicated to orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, changed to `is_produced` emptiness
>    check + `DeliveryLedger` delivery mark. Reason: obligation and product are the same data,
>    `take()` will take away what downstream needs; and the 11 IrGeneration obligations are in the
>    form of "the entire `&TypeCheckResult` handed to ir_gen", cannot be taken field by field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: Conclusion written to `DriverOutcome.obligations`,
>    diagnostics are shunted by `ProgramKind` (Check/Lsp go through per-file collection channel,
>    others go through warning channel). Reason: returning `Vec<Diagnostic>` would lose form
>    shunting—check's diagnostics must belong per file.
> 4. Three-tier table first tier criterion changed from "obligation read by declared consumer" to
>    "has delivery mark"; "mark ↔ real read" goes to static gate (paragraph 5 criterion 2).
> 5. Three-tier table second tier's "`strict` mode promotes to E-level" **not implemented in this
>    round** (no strict switch): only W1090 is produced, W→E goes to WBS 4.3.2.
> 6. Static gate criterion upgraded from "field appears ≥2 times but no third file reads" to the
>    four in paragraph 5.
> 7. The two additional tiers not in the sketch (S1, skip exemption) are in paragraph 2.
> 8. The single test name `test_obligations_drained` mentioned in S1/S4 tables: actually landed as
>    **17 use cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turned
>    green; red state preserved: after removing the sole settlement point, `pairs: 0`, 6 failures).
>    S1/S4 two lines have been synchronized to actual files.
>
> **VII. 4.3.2 interface and to-do (gate reinforcement)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   emptied in this run, per-run available) and five counts (paragraph 2 conservation); combined
>   with audit surface `findings` can give the fact "whether this field has been consumed by any
>   entry".
> - **Level decision input**: paragraph 3's mechanism facts (severity and code category are
>   independent; exit code determined by `warning_count` / `--deny-warnings`; same code upgrade and
>   W1006-style command layer upgrade don't need to change this mechanism's structure).
> - **Gate reinforcement items** (t3 independent verification F1/F2 and t4 review R2/R3
>   registration; **not yet landed**, this note doesn't implement on behalf):
>   - F1/R2: Red list evidence is plain text pointer, when the alternative consumer is entirely
>     removed (e.g. `detect_variant_ctor_call` is renamed) CLI and gate are **double-blind** (t3 MG1
>     reproduction: after rename `check` exit 0, gate PASS). Reinforcement direction: gate parses
>     file:line in evidence, verifies that line still contains the named symbol (anchor existence),
>     or uses symbol anchors.
>   - F2/R3: Gate criterion 1 only has "inventory ⊆ struct fields" one-way inclusion, lacks reverse
>     containment—when a new field is added without registration, the gate PASS-es (t3 MG4
>     reproduction). Reinforcement direction: output "`TypeCheckResult` field set − `INVENTORY`
>     field set" difference.
> - **Information item F3**: `SkipReason::UpstreamFailed` exemption is unreachable in real entries
>   (units with typecheck failure don't retain `type_result`, obligations don't exist in the first
>   place); this tier semantics is pinned by synthetic `State` use cases, real end-to-end only
>   covers `ConfigDisabled` (t3 T06).

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
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
    /// Can only come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); it does
not accept callers passing arbitrary arrays**. This is the key constraint to prevent the `Program`
abstraction from degrading into "free parameters that accept anything", enforced by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one less
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, recorded for reference—
>
> 1. The `StageOutcome` three-state in §4 is not returned by stage arms, but expressed by `State`
>    failure mark + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, eliminating
>    per-arm boilerplate returns);
> 2. `Driver` has no `config` field—the sole source of config is `Program.config` (§5 "avoid Driver
>    holding mutable global state" ruling covers sketch field);
> 3. `Skipped` diagnostic is only recorded into `DriverOutcome.skipped` internal ledger in 4.1.3,
>    not output—S2 "deliberately fix no bug" zero-diff criterion requires; output comes with 4.3
>    obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: driver arm is the only new caller
>    (L1→L2 is allowed direction); orchestrator existing call points will be discussed after 4.2
>    migration.
> 5. Under multi-file form, ProofExecution is an independent stage, executing **after all**
>    typecheck (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof within
>    the per-file typecheck loop (file N's proof before file N+1's typecheck). Single-failure both
>    byte-for-byte identical; in the "file 1 proof failure + file 2 typecheck failure" multi-failure
>    scenario, the first report changes from proof error to typecheck error (pinned by test).
> 6. `DriverOutcome` carries products per ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every found file has an entry, clean file has empty Vec—check_project current state contract);
>    Discovery product extended to "file set + used_by edge set + shadowing events" triple into
>    State (MultiFile form has no downstream consumer, only recorded not output); check path
>    per-file parse reduced from 2 times to 1 time per C3 topology.
> 8. Lsp form landing (4.2.3, shunted by file source, 2026-10-09 user ruling): disk file parse
>    failure same as Check's Plan B (collect + exit compilation unit—both sides degradation
>    convergence); edited buffer parse failure retains residual AST and continues typecheck (editor
>    philosophy—mid-typing states shouldn't make semantic features disappear). Discovery arm for Lsp
>    retains buffer source overwriting disk stale content; Typecheck under Lsp only checks target
>    file (other units are only for registry signature); intra-file diagnostic order normalization
>    principle extends to LSP (typecheck diagnostic → W code warning → proof error). Old behavior
>    where LSP hard-aborted on unrelated disk file parse errors, with handler silently falling back
>    to single-file path quirk is fixed in this step.
> 9. Embedded form landing (4.2.4): `Program` adds `shared_registry` field—embedded std module is a
>    sub-compilation, registry is **input** not product (EMBEDDED_ STAGES has no Registry stage),
>    #94's SymbolTable sharing contract is thus from "caller remembers to pass the same registry"
>    explicit to program declaration. Error path text from `<std.test> (embedded std)` normalized to
>    unit virtual path `<std/test>` (only compiler internal error surface, no test pinned). Thus
>    orchestrator four entries all migrate into Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): Check form's
>     faithful carrier of single-file semantics for programs without project root—warning surface
>     only covers entry file (adjacent files only collect errors, rather miss than false-positive);
>     relative `use` resolved along importer directory (rustc single-file mod alignment). CHECK
>     stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full chain)
>     already runs IR generation, E3019/E1014/E1015 etc. only produced in ir_gen (runner gate
>     evidence). Check form's IrGeneration is pure check, doesn't consume IR; Script/Bin split
>     expressed by module_key's None/Some (E3023 existing switch, ir_gen.rs:1660). Monomorphization
>     not in CHECK (only produces resource overflow/internal errors, zero corpus dependency).
>     **[2026-10-10 withdrawn per 3.4.8 evidence]**—"zero corpus dependency" is actually evidence
>     blind spot for corpus having no pathological recursion fixture; E3005 is the only compile-time
>     defense for this class of program. See note #14. Additional fix for latent defect: Registry
>     arm repeatedly harvests registration for embedded std units overwrites native half-face that
>     with_std() has merged (E1043 false-positive)—embedded unit now skips duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence deleted, changed to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains residual AST and
>     continues typecheck (editor philosophy extended from 4.2.3 ruling; not marked as stage
>     failure, otherwise topology skip makes typecheck unreachable); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path now completes proof (E4018), W
>     code warnings (W1001–W1005, test pinned) and IR-level errors; lex failure now reports real
>     diagnostic (old synthetic "E0001 lex error" text disappears with sequence deletion—no
>     information loss with real diagnostic carrying precise span).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 required):
>     the two entries have long converged—mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, difference carried by
>     whether collected_errors is empty; statements.rs five collection points (function
>     body/use/for/block/while). During verification, the repeated diagnostic defect was confirmed
>     and fixed: collection point puts the first error into both collected_errors and the Err return
>     channel, pass-3's add_error on Err causes same code same span same message ×2 (nested can be
>     ×3); additionally annotation check signature parameters and overall annotation double visit
>     (E1003/E1103), ownership layer same point double-emit (E2014/E2018) two independent
>     mechanisms. Fix lands at module result boundary: deduplicate by (code, span,
>     message)—diagnostic is positional fact, repetition carries no information; 63 corpus entries
>     remove duplicate copies, run column and exit code zero drift (borrow_conflict_err's
>     "duplicate" is an artifact of baseline triple not including column number—two E2018 are
>     actually legitimate diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver—`Compiler` wrapper layer is no longer involved.
>     Error text byte-aligned with old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground and SingleFile
>     share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step has real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partially withdrawing note #10's mono exclusion**). Evidence overturns "mono only produces
>     resource limit noise": pathological generic recursion (`f(x)=f([x])`) under multi-file path
>     without mono compiler process stack overflows (0xc00000fd—interpreter recurses in Rust layer,
>     no graceful runtime error), mono depth gate is the only compile-time defense; standalone check
>     before 4.2.5 took pipeline full chain had mono, exclusion ruling caused silent coverage
>     regression ("zero corpus dependency" only proves corpus has no pathological fixture). "Type
>     erasure fallback" assumption voided by RFC-033 reflection ruling—`^^List(Int)` needs
>     instantiated real identity, monomorphization is reflection's foundation not optional
>     optimization. Four design points: ① typecheck on qualified call (`lib.f(x)`, FieldAccess form)
>     produces instantiation_request, generic_id uses qualified name, sharing namespace with merged
>     IR (currently only recognizes bare Var, expressions.rs:2455); ② `Stage::ALL` topology changed
>     to Linking before Monomorphization, MULTI_FILE stage table tail adds Monomorphization—
>     consumes merged_ir, aggregates all unit requests (deferred bucket's containing_fn
>     qualification), mono promoted from single-module pass to whole-program pass; single-file stage
>     table unchanged (no Linking, subsequence property preserved, IR snapshot zero drift); ③ CHECK
>     adds Linking (pure merge—E3020 entry check kept in RoleClassification, prevent double-report)
>     and Monomorphization (pure check not consuming, same as IrGeneration's Check form) two stages;
>     ④ resource protection covers cross-unit mutual recursion with whole-program BFS. Monomorphizer
>     body unchanged (contract of consuming ModuleIR + request set is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design, qualified call instantiation
>     collection landed: `ExpressionInferrer` injected module qualified key table (`use` three-arm
>     registration—normal / alias / item-wise alias and only SubModule type export),
>     `callee_generic_name` via `SymbolTable::qualify` concatenation and merged IR-same-source
>     qualified name; qualified call first-path arity criterion tightened (when parameter table
>     can't resolve, fall to second path taking actual arguments by signature, avoiding cross-module
>     same-name function mis-match false-positive E3018). During implementation, three asymmetric
>     defects on mono side were empirically confirmed and fixed in the same round: (1) Delete key
>     (generic name set) and rewrite key (containing_fn + name + span triple) granularity
>     mismatch—sites that can't resolve actual arguments degrade to symbol requests entering
>     deferred bucket, when container function is non-generic the bucket never drains, yet the
>     original has been deleted by name → dangling call (`list_ops.yx:53` runtime E6006 evidence);
>     fixed by restoring gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as single source of truth to determine coverage, uncovered sites'
>     original is preserved. (2) Restoration segment `HashSet` iteration order causes function table
>     order non-determinism (t2 review F1) → changed to `Vec` and sort by name, product byte-level
>     reproducible. (3) Non-rewrite-side form TailCall / MakeClosure originally looks up mapping key
>     as usual (comment claims conservative) → `call_form_is_rewritten` single-point determination,
>     non-rewrite form counted as uncovered. Each with pinning test (red state evidence before fix
>     for discriminative power). Corpus differential baseline regenerated with this commit, only one
>     line change (remove_at frame supplementary `(int64)`). **Commit strategy note**: when
>     typecheck collection and mono fix are split into two commits, the stash tree containing only
>     the former triggers `stdlib_docs` against HEAD existing defect red (`list.is_empty([])` same
>     family E6006)—the two halves are each other's green prerequisite, so merged into one atomic
>     commit.
> 16. 2026-10-10 five rulings registration (after R1 closure): (1) **D2 ruling A**—user module
>     export carries `Export.type_params` with `generic_fn_type_params_snapshot()`, isomorphic to
>     std path (`yx_sources.rs:62/108`): declared name becomes authoritative source of qualified
>     call monomorphization, signature back-inference degraded to fallback (merged into R2 scope).
>     (2) **HKT declaration side alive, call side broken** (inferring type arguments E1002 /
>     explicitly giving type arguments E1010) registered as WBS 3.5 (3 level-3 tasks). (3)
>     **deferred bucket under non-generic container never drains** merged into 3.4.8 ②'s acceptance
>     sub-item: bucket must drain or loudly report error, must not silently not specialize. (4)
>     **Closure variable by-name call runtime E6006** registered as WBS 3.6; semantics (i) support
>     indirect call / (ii) compile-time reject, **ruled (i)**—support indirect call: `CallDyn`
>     mechanism already exists (`ir_gen.rs:7945` closure expression path), broken is `Var` callee
>     downgraded to by-name `Call`, and typecheck already accepts this program. (5) **Trade-off
>     criterion solidified**—plan rejection reason can only be based on correctness and readability,
>     "large change surface / introduce new mechanism / high cost" doesn't constitute rejection
>     reason, lands in `coding-rules.md` second part and HOWTO self-check; user-required trade-offs
>     per `AGENTS.md` "When requesting user ruling (mandatory)" give all four items
>     (situation+evidence / per-item pros and cons / clear recommendation+cost / destructive
>     naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm **before taking each unit's IR** aggregates whole-program requests and qualifies
>     `containing_fn` (otherwise deferred bucket key and merged IR different source, bucket never
>     drains). **① closure (D2 ruling A)**: user module export carries `Export::type_params` with
>     declared-stage type parameter snapshot, isomorphic to std path (`yx_sources.rs:62/108`).
>     **Existing defects empirically confirmed and fixed in the process**: (a)
>     **F1**——`qualify_names` only rewrites function body, `ir.init` is never accessed, under
>     multi-file function definition already qualified but module top-level call still bare name →
>     runtime E6006 (**reproduces even for non-generic**, unrelated to generics); fixed by init
>     reusing same `rename`/`aliases` and same `rewrite_call_names`. (b) mono side "whole-program
>     unique suffix" heuristic and miss-silent `continue` → delete heuristic, miss changed to **loud
>     E3018** (ambiguous silent mis-match worse than not parsing). **Regression caught and fixed in
>     independent verification**: t6 found named import `use lib.{pick}` then bare-name call
>     rejected by false E3018 (IR instruction name is alias qualified name, request name is bare
>     name, aggregation side only qualifies by unit key) → fixed by aggregation side **directly
>     calling ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as pub single
>     point, one production one consumption), and matrix pin test covers named import two forms.
>     **Filing**: init segment `DefId` blank empirically confirmed as "consistency blank" not live
>     defect (codegen name fallback resolvable, four init calls all tested pass), registered as WBS
>     3.7.4 to close the "function body takes def, init relies on name" fork in specification.
> 18. init segment DefId specification fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     extracts `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure),
>     function body and `ir.init` share the same implementation and the same `resolve` closure; 208
>     IR snapshots regenerated, only 309 pairs `(def=_)`→`(def=dN)` change (other-lines 0; whole
>     tree init segment 0 residual `(def=_)`). codegen DefId main path and name fallback two states'
>     translation results and execution values item-by-item consistent (four init calls tested),
>     corpus baseline zero drift; probe flipped per new specification and strengthened (identity +
>     precise index + execution value, fallback init covered immediately red). **Errata
>     (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase`—original
>     b554497b/e60dcc87/b7ac7396 correspond in order to post-rebase 375789d2/d922a551/38bd4b9e;
>     pushed dev history is governed by new numbers (old numbers were local temporary numbers, not
>     entering remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (Design basis of ③④ see #14; ②
>     landing see #17, this is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), entry check block left to `RoleClassification`;
>     `ir_generation_check` generated then retained `us.ir` (memory only); mono failure in Check
>     enters collection channel, in MultiFile keeps `fail_with`; `into_outcome` first
>     `take_check_diagnostics` then judge failure (structural failure no longer clears
>     already-collected diagnostics). **Verification**: four rounds of independent verification
>     (t3/t4/t10 + t5 cross-form matrix) and one round of review (t6) all pass; corpus differential
>     340/340 line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps
>     discovered and fixed in the same round**: F1 (rootless multi-file check's request name and IR
>     definition key different source → silently not specialize, see WBS 3.4.8 row's landing
>     note)——fix is aggregation side exact reverse-lookup by real definition key (non-heuristic),
>     first version fix is aggregation side exact reverse-lookup by real definition key
>     (`flat_defs`); **same-day erratum**: the bridge has been deleted in regression fix, root cause
>     resolved by `module_key` one-duty two-use decoupling (`script_mode`), three same source as
>     `{unit key}.{name}`. This regression (CI docs example gate 258 blocks 10 fail → 0 fail)
>     positioning and fix see WBS 3.4.8 row's landing note. **Filing (existing, not this round's
>     regression)**: E3005 help text doesn't match resource protection semantics; multi-file E3005's
>     span label misalignment (HEAD binary reproducible); request collection surface's type
>     parameter position dependency (same surface goes to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution layer guard reachable)**: interpretive
>     call changed to **explicit stack-driven**——`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` after pushing frame returns control to outer `step_one`, frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, on return writes back to caller's slot, multi-call
>     instructions continue with frame's `pending_calls`. Baseline measured each layer interpretive
>     call consumes native stack **2544 B**, crash threshold at `count_down(394)` (Windows main
>     thread 1 MiB); after fix 20 000 layers of recursion can run on 512 KiB stack thread (each
>     layer about 0). Guard (`push_frame`) is thus reachable at any entry, any `max_stack_depth`
>     value; `ExecutorError::StackOverflow` changed to carry real upper limit (original
>     `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (deep stack echo truncation) per
>     user ruling merged into this task** (same file, cannot split by file): echo changed to first
>     16 frames + count ellipsis + last 8 frames, shallow stack frame-by-frame unchanged. **3.10.1
>     (compile-time ungated native recursion, #447)**: first locate then fix——crash point is
>     `MonoType::type_name` recursion (**328 B/layer**, SP probe + per-operation threshold
>     determined), same path also has derived `Clone` (~400 B/layer), `substitute` deep copy inside,
>     `type_name_hash` (164 B/layer), `collect_generic_type_refs`; after four-place change to work
>     list and removing one whole-request deep copy per layer, `mono.max_depth=5000`'s pathological
>     generic recursion produces **E3005 (with real upper limit)**, no longer stack overflows.
>     **3.11.1 (task boundary error code fidelity, #448)**: task result error side changed from
>     value to **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), cross-boundary by-value restores `ExecutorError`;
>     task-internal E6001/E6003/ E6004/E6008 in Standard mode and Embedded message verbatim
>     consistent. Two **pub API payload form changes** have been written in the commit
>     `BREAKING CHANGE`. **Commit strategy note**: 3.9.1 and 3.11.1 **cannot be split at file
>     granularity**——`interpreter/executor/executor.rs` simultaneously carries trampoline and error
>     channel's production/consumption points, `TaskFailure` defined in `runtime/engine.rs`, only
>     stashing either side won't compile (pre-commit uses **stash subset** to compile, actually
>     refused twice), so per #15 same specification merged into **one atomic commit**. **Known
>     boundaries and follow-up**: Drop derive (~75 B/layer) ⇒ `mono.max_depth ≳ 13000` may still
>     crash; E4012 (constant evaluation) gate carried on native recursion (each layer ~3 696 B,
>     default 256 layers ≈ 924 KiB), currently zero call point, production unreachable, future
>     wiring must be closed in same family; Standard mode task interpreter per default
>     `ExecutorConfig` constructs ⇒ its E6004 upper limit always 1024 (config propagation registered
>     as WBS 3.12, user ruling B: separate team); test runtime budget separate issue #449
>     (pathological fixture 76s, clippy dual target, commit runs full), not in this RFC construction
>     surface. **Corpus directory layering (same round, 8605a081b)**: `02-type-system` (78) and
>     `06-compile-errors` (134) total 212 fixtures split into 23 category-divided subdirectories, 88
>     IR snapshots moved by mirror path; corpus baseline regenerated with this commit (342 lines,
>     independently recomputed line-by-line comparison: only `file` field and 2 new change), 41
>     snapshots regenerated due to file header path lengthening, line-by-line check **only span
>     offset change** (zero difference after de-offset normalization). **Transformation for the ten
>     entries (function by function)**:

| Entry                                                      | After Transformation                                                                                                                                                                                          |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                       |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                   |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                     |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to one Driver call + result filtering to target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                    |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transitively calls `run_project` or `SingleFile`)                                                                                                                                                  |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transitively calls Driver)                                                                                                                                                                         |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changed to `Program { kind: Lsp, aggregation: CollectAll }`; single-file branch's **manual lex→parse→… sequence deleted**, changed to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                 |

**`check_project` and `compile_project` overlap degree (about 40% shared / 60% forked)**. The shared
part: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
resolution loop, per-file checker assembly, result handling. The forked part: items
2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton is exactly the `Driver`'s
value range**—the forked 60% is all "different stage sets" or "different aggregation modes", which
is exactly what `Program`'s two fields can completely express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure caused this stage to skip" is not a stage return value.**

Reason: skipping is determined by **topology**, not by the stage itself. If each stage returns
`Skipped`, the "why I didn't run" information is scattered across 12 stages and cannot be audited
centrally. Changed to `dispatch` making the decision at the beginning of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip",
producing no diagnostic, just because the obligation happens to be empty. After the rule is
established, any "because X wasn't done so Y didn't run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text Direction                                                                                                           |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Upstream `Abort` caused                                 | "Stage Y did not execute because upstream stage X failed"                                                                |
| Pre-condition obligation empty                          | "Stage Y did not execute because no pending obligations (normal)"—**this item severity = Info, not counted in warnings** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because configuration not enabled"                                                              |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                                 | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)                   | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` doesn't early-exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a `Program` field, not a Driver global setting**—LSP serves both in-project
files and single files in the same process, while CLI's `run` and `check` are two independent calls;
putting them on Program can avoid Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early-exit or not"
difference—they are two different checker entries. After unification, it should be driven by the
`Aggregation` parameter for the same implementation, not retaining two functions (**this is an
additional item to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                              |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation, no touching `MonoType` / `PolyType` / `ir::Type`                                                         |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to language type system                                                                                         |

**This document does not introduce a fourth type representation.** The convergence of three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type ownership and layering**: what migrates into the ledger is **settlement
responsibility**, not type's owning module. `ReleasePlan` is still defined in `layers/ownership.rs`,
`ProofFunctionCall` is still in `proof/`, other field types stay in their original places;
`driver/obligations.rs` only holds aggregate containers and `assert_drained()`. L3 consumers (e.g.
`ir_gen` reading `release_plan`) continue receiving specific field types through parameters, **must
not `use crate::driver`**—this is consistent with the driver cannot reverse-depend red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is considered a violation, intercepted by
`scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                       |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                              |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was stop-gap connected in P3; **warning surface maintained not added** (decision B1, 2026-10-09: dead code family warnings don't enter compile path—pool semantics being corrected by 4.9 as defect, normalize warning surface deferred until after 4.9; this row's draft "add new warning output" voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Stop-gap connected in P3                                                                                                                                                                                                                                                                                                   |
| LSP (in-project)                    | No proof_execution                       | Stop-gap connected in P3                                                                                                                                                                                                                                                                                                   |
| Z3 not installed + single file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                 |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                     |

**The only intentional behavior breakage** is `predicate.rs:35`'s `.expect()` changed to return
diagnostic. This aligns with RFC-027 §8 "not bound to specific solver" positioning, and is also the
contract `backend.rs:60` has already declared ("caller should conservatively degrade")—the current
implementation is opposite to its own contract document.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single file / N for multi file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                 |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                        |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to take Driver                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                     |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to driven by `Aggregation` parameter                                                                                                                                                                                                    |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                             |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                       |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently via `Compiler::compile_with_source` implicitly falls to Path 1)                                                                                                                                                                        |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent drop point**)                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment inconsistent with `1320`                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "lower layer failure means upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                         |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: new process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access; three consumer points in `predicate.rs`/`checker.rs`/`ownership.rs` changed to use shared port. `&'static` raw reference form infeasible (`dyn Solver` not `Sync`), original text governed by D58 |

**Deletion**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s manual lex/parse sequence.

**Not touched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic itself
is correct—**the problem is on the consumer side not the producer side**, changing the producer side
would mask architectural defects); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch (wasm
branch reachability decision goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                               | Disposition                                                                                                                                          |
| -------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                     | **Breaking**: previously silently passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Breaking**: programs that compiled now produce warnings                   | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), don't change exit code                                  |
| `assert_drained()` first run produces W-level diagnostic | **Breaking**: diagnostic set adds                                           | W first then E, S2/S4 two steps                                                                                                                      |
| `predicate.rs` panic changed to diagnostic               | **Improvement**: no longer crashes                                          | No breaking                                                                                                                                          |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                               | These two paths don't go through proof_execution                                                                                                     |
| `TypeCheckResult` field migration to `Obligations`       | **Internal refactoring**                                                    | If `pub` API surface has external dependencies, need to sync; in-repo consumers are all listed in change list                                        |

---

## Implementation Points

This document corresponds to **P3 (fix correctness holes)** and **P4 (stage contract and unified
Driver)** in RFC-039's global stage sequence. S1-S5 below is the implementation order within this
document, **each step's acceptance criteria references the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration change: each entry's
**diagnostic set** identical + corpus behavior identical).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failures first.

| Deliverable                                                                                                                                                    | Acceptance                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red**. If accidentally green, this document's vulnerability analysis needs re-review          |
| Multi-file corpus layer: create new `tests/yaoxiang-multifile/` (resolution D48—not mixed with single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file two versions of corpus diagnostic sets can be compared                       |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                                 | Mark `#[ignore]`, S4 turns green (red state preserved: after removing sole settlement point `pairs: 0`) |

**Rollback point**: no code change, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-for-byte identical**                    |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately fixes no bug**—it only moves existing behavior into Driver. Acceptance
criteria is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored.

### S3: Merge Four Orchestrator Entries + Fix Vulnerability

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entry diagnostic sets normalized by `Aggregation` are identical              |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in and out of project                       |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give identical diagnostic set for same file                           |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in Z3-unavailable environment                                    |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If
structural merge has issues, only structural commit can be rolled back, vulnerability fix commit
retained—at this time `test_multifile_proof_obligation_not_dropped` stays green; the reverse (retain
structure, roll back fix) will turn red, is unacceptable intermediate state, forbidden to merge.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                           |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 17 use cases turn green (implementation form see §2 revision note) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                             |
| Obligation diagnostic W → E upgrade          | Manual review of every new E after full corpus differential                                          |

**Rollback point**: `assert_drained()`'s severity can be controlled by config item, W/E switch
doesn't need code structure change.

### S5: Proof Layer and wasm Cleanup

| Deliverable                                                                                                                                                                                                                     | Acceptance                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                          | Documentation and `checker.rs` call points correspond item by item                                                                                                  |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown doesn't enter cache**—otherwise one timeout cached at process level solidifies across compilations and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call points go to zero (tests only retain); cache hit rate observable across three consumer points (counter landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                 | Has diagnostic when no `body_checker`                                                                                                                               |
| `orchestrator.rs` 12 wasm attributes reachability decision                                                                                                                                                                      | Conclusion goes to `06-cleanup-inventory.md`, this document only registers decision need                                                                            |

**Note**: fixing layer order will change diagnostic set, may expose a large number of previously
silenced `Unproven`. **This item should go independently of S1-S4**, don't mix with entry merge in
the same PR.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                                                                                                                                                                                          | Reason                                                                                                                                                                                                                                                                                                                                             |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration rejected**                                                                                                                                                                                                                                           | Exhaustive `match` is zero-cost compile-time enforcement: when adding variants `dispatch` won't compile. This is isomorphic to the mechanism already verified by the opcode table in RFC-039 routing table B. `StageScope` further makes "per module or project level" a type fact, eliminating call count problems requiring human reasoning      |
| **Obligation mechanism** | `Obligations` + `assert_drained()` does **runtime** settlement, used together with `scripts/ci/check-obligations.py`'s **static** gate; severity W first then E (**implementation form see §2 revision note**: ledger doesn't copy field structure; red list fields—1 after clearance—don't enter user-visible diagnostic stream) | Fixing is a whole class of bugs, not one bug: currently verified same kind of hidden dangers at least 2 (`proof_calls`, `checker.rs:1313`), all 16 span-keyed fields in range. W first then E is to make each step's C2 criterion usable—one step to E would make `yaoxiang check` suddenly have a large number of previously silenced diagnostics |
| **Fix scope**            | Fix **consumer side** (orchestration layer), **don't touch** `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branch producer-side logic                                                                                                                                                                                 | Problem is on consumer side not producer side. Changing producer side would mask architectural defects as "logic fixed", but the logic of these three branches is itself correct                                                                                                                                                                   |

This design additionally eliminates two classes of cracks:

- **The "comment promise" and "code behavior" crack**. `checker.rs:5165`'s "silent pass must not be
  allowed to revive" and `layers/README.md:3`'s "lower layer failure means upper layer doesn't run"
  are both **comment-level contracts**—the former is fulfilled by `assert_drained()`, the latter is
  exposed by topology-driven `Skipped` diagnostics.
- **"Forget to append to one array"**. Currently five functions each hand-write call sequences;
  after unification it's the unique stage table, and missing a new stage connection will error at
  compile time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—of the 11 inconsistencies, 7
  (two dead code analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type connection**, are configuration problems of "whether to
  run some analysis"; it also can't express `Aggregation`, only moving the `check_module` /
  `check_module_collect_all` choice from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when adding stages `dispatch` no longer fails to compile, equivalent to changing
  "five functions each hand-write calls" to "forget to append to one array", which is exactly the
  cause of the current 11 inconsistencies. This repo has same-form counter-example:
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` all zero-implementation.
- **Only fix bugs without touching structure**—can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` still scattered across 5 functions, the next fork point will continue
  to grow from here. **It must be done first** (part of S1/S3), because structural transformation
  needs a known red test to prove criterion is valid.
- **Make `proof_calls` `pub` and add `debug_assert`**—`check_module` is a generic entry, it doesn't
  know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]` only
  warns when field is **entirely** discarded. This is finding bugs at the wrong layer: the bug is in
  the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize wiring" refactoring.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silenced problems"**. After the fix, a batch of "new error" user reports may
  appear—they are real bugs, just never reported. Must be clearly stated in CHANGELOG.
- **`assert_drained()`'s severity switch needs per-field manual judgment**. In obligation fields,
  some (e.g. `module_namespaces`) "no consumer" may be designed to not need consumption, shouldn't
  warn. S4 needs to go field by field, can't be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entry's stage set is long-term unstable,
  `stages()` will degrade to "each call passes a different array" free parameter, contract
  constraint falls through. **Mitigation is `test_program_stage_coverage` asserts `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criterion depends on the
  [equivalence criteria document](07-equivalence-oracle.md)'s vulnerability test being red first.
  **If that document doesn't first establish multi-file corpus layer, S1 cannot be
  accepted**—because existing 293 corpora all take single-file path, zero coverage for this class of
  defect. This document doesn't involve IR verifier (`verify_loose`), that part of prerequisite work
  is not in this document's scope.
- **Layer order fix will expose previously silenced `Unproven`** (`equivalence` not in pipeline at
  all, `termination` and `ownership` order opposite to declaration). This is diagnostic set change
  risk, should go independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19`
  already says `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after changing
  to a singleton shared across compilation units, need to confirm all access paths go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelism will **mask order dependency defects** (such as termination and ownership's actual
  order dependency). Should be opened after equivalence criteria is stable.

> **The open questions originally listed in this section have all been ruled on.** Each decision is
> at [RFC-039 Decision Registration](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` transformation (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Phase 2.5 proof function execution design source
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  `build.rs:19-55` gate reference code table
- `src/frontend/core/typecheck/checker.rs:5165-5169` — "silent pass must not be allowed to revive"
  explicit declaration
- `src/frontend/core/typecheck/layers/README.md:3` — "lower layer failure means upper layer doesn't
  run" explicit declaration (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — SMT initialization hard-failure explicit
  reason
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — "caller should conservatively degrade"
  contract declaration (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-73` — 14 obligation fields that downstream must consume
  (after 3.4.4/3.4.5 clearance)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Playground's fourth compilation caller (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate not main crate
- `build.rs:19-55` — Error code build-time gate, example of this project's enforcement mechanism
