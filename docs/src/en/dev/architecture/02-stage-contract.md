# Compile Stage Contract and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution phase order are in the main body of
> RFC-039; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which didn't, and why" is currently scattered
across six paths and ten entry functions, each manually wired, and no single place can answer this
question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture, not by logic.**

The original comment at `src/frontend/core/typecheck/checker.rs:5165` says "Unproven → compile
error, no degradation, no silent pass... no resurrection of silent pass." But this invariant only
holds on the **single-file path**: proof function constraints shaped like `x: Sorted(3)` pass
**silently** under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the proof
function is never executed, and no diagnostic is generated.

### Coverage

- `Stage` enum (exhaustive, non-runtime registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and per-entry function transformation for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of Scope

- Four-layer model, dependency direction conventions, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA-ization →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 main body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactor and the **order** to do it; this document gives the L1
**specific form**, the **per-file change list**, and the **implementation phases within this
document** (corresponding to P3 "Fix Correctness Vulnerabilities" and P4 "Stage Contract and Unified
Driver" in RFC-039's global sequence). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed per the **C2 (orchestration changes)** category of the
[equivalence criteria document](07-equivalence-oracle.md): the diagnostic sets of each entry are
identical + corpus behavior is identical.

## Current State

> All items in this section are **verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect of "One Place Written Wrong, Whole Repository Silent"

It is different in nature from the other two defect classes (module boundaries, test wiring):

| Defect                  | Typical Manifestation        | Is There a Signal?   |
| ----------------------- | ---------------------------- | -------------------- |
| Lexical/syntactic error | Source code written wrong    | Has diagnostic       |
| Type mismatch           | Type written wrong           | Has diagnostic       |
| Stage missed in wiring  | A stage is not called at all | **No signal at all** |
| Obligation not consumed | Field filled but never read  | **No signal at all** |

The common feature of the latter two: **failure produces no error**. Therefore, they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentation convention, not as an executable constraint."

### Enforcement Mechanisms Already Existing in the Repository

The same repository already has mature enforcement mechanisms, just not extended to the stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subject to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, comparing each against the
  RFC-013 code table, and any inconsistency directly `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` directory has 6227 lines, accounting
  for 47.9%) has high test density, with each module's test subtree being declared-wired—this is the
  most complete test wiring in the repository and can serve as a form reference for stage-layer
  gates.

**Design capability is sufficient. What's missing is "a gate of the same level in the orchestration
layer too."**

(Line count caliber: `(Get-Content).Count`, see the "Line Count Caliber" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                   |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | --------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                                |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                    |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking              |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics         |
| 3c  | **LSP within project**     | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                  |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR            |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → Path 1                   |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (Path 3a)                       |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | Within project: Path 3c; otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                       |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → Path 1            |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Inconsistencies in Stage Coverage

The table below provides evidence for each cell. **A blank does not mean that entry "does not do
this thing", but rather "there is no code in that entry that does this thing"**—this is the essence
of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gating)                | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                 |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never output**                            | Yes `350`                                                   | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (merge after independent ModuleIR) |

> **Review Note (WBS 3.4.3, verified cell-by-cell on 2026-10-07)**: This table is a diagnostic
> snapshot at the `9e02e4db` baseline, preserved unchanged. State differences after P3 lands:
>
> - **Row 1 has been fixed**: proof_execution is shared by all five entry points via the same
>   implementation in `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the sole
>   consumer defect no longer exists.
> - **Rows 2/4/5 state unchanged** (multi-file `run` still doesn't output W1003, still doesn't run
>   the dead code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   unverified potential risk defect).
> - **Row 7** check_module / check_module_collect_all dual entry → assigned to 4.2.7 (Aggregation
>   parameter driven).
> - The "Ruling #434" referenced in the code side had no prior docs registration—now registered as
>   RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new fact outside the table, registered 2026-10-09)**: The
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch #117), while multi-file `discover` previously only recognized
>   explicit `use`—for-loops in projects compile through but fail at runtime with E6006 (empirical
>   evidence). Fixed and WBS 4.10.2 struck off; the same family of "multi-file missing one
>   single-file step", namely parse hard-abort quirk, registered as WBS 4.10.1—**also fixed**
>   (landing immediately after 4.2.2, Check path degrades to per-file collection; from this audit
>   method's "stage coverage/field consumption" dimension, this is a compilation unit membership
>   difference, a missed-net dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm for embedded std units uses `extract_module_info` to repeatedly harvest
>   registrations, which overwrites the native half-surface already merged by `with_std()`
>   (`result.is_err` and similar lost → std.test false-positives E1043). This audit's third
>   missed-net dimension: "merge semantics for two registration sources of the same module key"—the
>   single-file path's registry is formed only once via `with_std()`; only the multi-file/Check
>   Registry arm's per-unit insert has coverage overlap.

**Two points require precise expression, otherwise one will write them wrong:**

- **The precise conclusion of rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function has **no `result.warnings` read site throughout**, and
  the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion also comes from different sources**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "is there a manifest"). The two functions give different answers to "what files must define
  `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation generation point is unique.**
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

The semantics is "the predicate arguments of this refinement constraint are compile-time literals,
and the proof function must actually be executed to decide".

**Step 2 — The consumption point is inside the checker, but only emits without consuming.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch thus becomes a true defense line, not forward-looking collateral: silent pass must not be resurrected.`

That is: the author knows "no diagnostic produced" is a defect, explicitly categorizes it as a
**known, place-nowhere intermediate state**, and design assumes "someone must come to read
`proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares local
`proof_calls` and collects them; `checker.rs:1439` writes to `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — The repository has only one read point.** `TypeCheckResult.proof_calls` (`types.rs:29`)
in production code is **only read at `src/frontend/pipeline.rs:187`** (passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the repository fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four entries of the orchestrator bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) of
`src/frontend/module/orchestrator.rs` **all do not call `pipeline.rs`**, calling
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they cannot even reach the
only read point at `pipeline.rs:187`.

**Step 6 — Consequence: Standard library's own refinement obligations also take the discard path.**
`compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for compiling
embedded std. This means that the **proof obligations of the embedded std module itself are also not
executed**.

**Step 7 — Consequence: Constraints pass silently.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) under the multi-file / check / LSP three paths **compiles
through, runs through, no diagnostics**. The proof function was never called.

**Step 8 (Supplementary Verification) — There's a second silent discard point within the layer.**
`checker.rs:1303-1314` when handling the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership check layer is swallowed by an empty match arm, with no diagnostic
and no bookkeeping.** This directly contradicts the promise of `5164`'s "silent pass must not be
resurrected", and is **independent of the orchestrator problem**—even if the entry layer is
completely fixed, this point remains silent. (Processing timing: should be handled in the same batch
as the obligations ledger, because it is the same class of problem as the obligations ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has hit count 0 for the three keywords
`Sorted` / `proof` / `refin`.** Proof obligations on the multi-file path have zero coverage.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping exactly before `pipeline.rs`**.

The test file's own doc comment makes this explicit. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so cases of refinement violation are at the .yx layer`

**This is exactly the shape of the problem: tests verify "the obligation is filled in", but the bug
is "the consumer doesn't read".** A test that only tests the producer and not the consumer is
inherently immune to this class of defect.

### A Stronger Finding: The Subject Corpus of the Equivalence Criterion Is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) uses the end-to-end differential of
293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion**, as the main acceptance
means for the C2 stage. But actual testing:

- Under the `tests/` directory **there is no `yaoxiang.toml`** (zero hits in directory-wide glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and none cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical evidence
of this. That file is labeled `// expect: compile-error E4018` (line 15), and its header comment
line 7 says "Status: ❌ Should be rejected at compile time"—**it passes precisely because it goes
through the only path that executes the proof function**.

**Conclusion: The third-layer criterion needs to supplement the multi-file corpus layer, otherwise
it cannot serve as the acceptance tool for this document.** See implementation point S1.

### Obligation Fields: 16 "Produce-and-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields** whose
doc comments explicitly state "produced by stage X → consumed by stage Y", meaning they are
**cross-stage obligations** in nature:

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

**All these fields are passed across layers keyed by `Span` or name tables.** The equivalence
criteria document has already pointed out that mismatch in span-keyed contracts will "fail silently
with no error" (`ReleasePlan` is typical: any inconsistency in span computation on either side →
`Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` die? The difference is **structural, not
accidental**:

- The consumer of `release_plan` is at `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`) inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and the downstream module is on the must-pass path of all entries.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, belonging to **another
  entry implementation**. The four entries of the orchestrator don't even pass through the
  `pipeline.rs` layer.

**Verified fact**: currently the repository has **no mechanism at all** to guarantee that these 16
fields are consumed. The only "protection" is that `ReleasePlan` happens to piggyback on IR
generation.

### Four Additional Contract Defects in the Proof Layer

The following all come from the proof layer analysis, independent of the entry fork problem, but all
belong to the "stage contract is not enforced" category.

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependency    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "Execute in layer order, lower-layer failure upper-layer doesn't run". The actual
call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear in the `check_module` stage sequence at all** (it
is only used by `inference/assignment.rs:17` as a utility function `is_subtype`, unrelated to
`ProofResult`). At the same time **there is no short-circuit at all**—`checker.rs:1271-1276`
`add_error`s each termination error and continues, so the README's promise of "lower-layer failure
upper-layer doesn't run" does not hold.

**(b) The SMT backend has three acquisition strategies, two failure philosophies.**

> **Review Note (2026-10-07)**: The hard-fail panic has been eliminated by 3.3.1 of P3 (SOLVER slot
> Option-ized, missing downgrades conservatively to `SMTResult::Unknown`); "silent skip (not
> injected)" supplemented with W1081 signal by 3.4.2. Unification of the three forms
> (singularization) executed per **RFC-039 D58**, real fix location `proof/smt/backend.rs` (new
> process-level shared singleton + `with_shared_solver` closure port), 02 change list's
> `default_solver() → &'static` statement per D58 (raw `&'static` reference is infeasible:
> `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Is Unavailable                                |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Construction-time injection `with_solver_owned`          | `None` → **silent skip** (not injected)                   |
| `ownership.rs:627` (back-edge cut decision)            | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservative no-cut      |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "Initialization failure keeps
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'". But `backend.rs:60`'s contract doc says "`None`: backend unavailable……**caller should
conservatively degrade**". **Two philosophies coexist in the same code, and the hard-failure one
panics instead of returning an error.**

**(c) Each back-edge decision creates a new Z3 context, cache is in name only.** `ownership.rs:627`
calls `default_solver()` on the hot path of back-edge cut decisions. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—each call does `Z3Backend::new()`, i.e., creates a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **intra-instance**, and the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits in this call pattern**.

(Comparison: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. Same backend, two lifetime
strategies.)

**(d) checker silent degradation + comment doesn't match reality.** `checker.rs:1283-1287` and
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
table** and runs as usual—ownership analysis degrades to "nothing conflicts", **with no warning of
any kind**. (Suggested treatment: should be recorded as a warning-level diagnostic, or at least left
a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type check and before constraint
solving". Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at `1256`,
ownership is after `1296`, i.e., the "before" the comment says is actually "after two layers"**.

### wasm Status: Shim Crate Carrier, 27 File Branches Are Alive

The wasm target **is built and built in CI**, with `cdylib` not in the main crate but in an
independent shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                  |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from fixed URL, degrades to warning when missing)                                                                                                                                                              |
| **27 files** contain the literal `wasm32`         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) only mention in docs                                                                                                                                            |
| `orchestrator.rs` 20 places                       | 12 attributes + 8 comments                                                                                                                                                                                                                                                       |
| `lib.rs` 11 places                                | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm
target—`lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` entirely (these three
all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**single-file path**. Therefore:

| Paths Consuming `proof_calls`                  | Does the Proof Function Execute? |
| ---------------------------------------------- | -------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)        | **Yes**                          |
| wasm playground (`wasm/src/lib.rs:48`)         | **Yes**                          |
| `build_bytecode` (`lib.rs:171`)                | **Yes**                          |
| Multi-file `run` (`compile_project`)           | **No**                           |
| `check` (`check_project`)                      | **No**                           |
| LSP within project (`check_source_in_project`) | **No**                           |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" 3), otherwise when unifying the Driver, this path will
be missed.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario—is assigned to `06-cleanup-inventory.md`. The open question in RFC-039 "build
wasm target, or delete these branches" **has expired**: target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-Runtime Registered)

**Core constraint: `Stage` is a compile-time exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can force at compile time that "a new stage must be
handled by the orchestration layer"—this is exactly the mechanism the opcode table has proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lex + syntax               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type check (including embedded proof layer)   PerModule
    DeadCodeAnalysis,   // Dead code family analysis              Project
    ProofExecution,     // Proof function compile-time execution         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs
once at the project level) turn "which stages must run per module, which must run once at the
project level" into a type-level fact. A `Project` scoped stage is structurally guaranteed to run
only once in `dispatch`—this eliminates the kind of problem in current `orchestrator.rs` about
"should `allocate_global_slots` be called once per file", which requires human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on order.

> **Revision Note (P4 Implementation, 2026-10-07)**: Two deviations from the initial draft corrected
> per implementation evidence—
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), the initial draft order contradicts the
>    data flow.
> 2. The Check variant's stage table is normalized to dead code **before** proof per `Stage::ALL`
>    topological order: current state of `check_project` is that proof runs before dead code, the
>    two have no data dependency, the diagnostic sets are identical (C2 set semantics), only the
>    within-file diagnostic order is normalized. The single-file path (dead code embedded in
>    typecheck, before proof) already follows ALL order, byte-by-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about data flow; and multi-file parse thereby drops from 2 times to 1 time. Side-fact honest
>    registration: multi-file path parse error was a hard-abort (`build_registry_from`'s `?`
>    propagation, tension with CollectAll semantics registered as WBS 4.10.1)—**4.10.1 is fixed**
>    (2026-10-09 user ruling of rust-style collection semantics + Plan B): Check path parse failure
>    degrades to per-file diagnostic collection, sick files exit compilation unit (don't enter
>    registry, importers report E5001); MultiFile path preserves hard-abort (under FailFast bad
>    files can't produce IR, semantically correct, pinned-board long-term valid).
> 4. Check variant landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data
>    dependency edges 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the initial draft missed these
>    (these two products were produced by `discover_with_used` in the orchestrator era, wrapped and
>    discarded by `discover`, lost the data flow before it entered the table); b. Within-file
>    diagnostic order normalization extended to the complete form of note #2: E3020 entry check
>    moved to RoleClassification arm (Check has no Linking stage, the arm holding surfaces and AST
>    is responsible for validation), within-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); diagnostic set unchanged (C2 set semantics), only
>    stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but never consumed" from undetectable to a fact assertable at
compile time or test time. RFC-039 has listed this as "the highest-value item".

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
    // …other span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                   | Criterion                                              | Disposition                                                                  |
| ------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------------- |
| Obligation is read by declared consumer     | The field in `Obligations` is `take()`/marked consumed | Pass                                                                         |
| Obligation is non-empty but has no consumer | Field is non-empty and not consumed                    | **Produce diagnostic** (W level, default; `strict` mode promotes to E level) |
| Obligation is empty                         | Field is empty                                         | Pass (no need to consume)                                                    |

**Why W level first, not E level**: fixing obligation discarding will **change the diagnostic set**.
The C2 criterion requires "the diagnostic sets of each entry are identical"; if promoting to E level
in one step, `yaoxiang check` will suddenly have a large number of `Unproven` diagnostics that were
previously silent. Splitting into two steps (W observation first, then promote to E) makes each
step's criteria usable. See implementation point S4.

**The single call point of `assert_drained()`**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third file reads → fail). `assert_drained()` is the **runtime** gate, the script is the
**static** gate, they are complementary, both needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP within project, result filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be the above 6 predefined combinations (one for each `ProgramKind`), does not
accept the caller freely passing an array**. This is the key constraint preventing the `Program`
abstraction from degenerating into "anything can be passed as a free parameter", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topological decision on whether to skip, not determined by stage return value
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
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision Note (P4 Implementation, 2026-10-09, 4.1.3 Landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for future reference—
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, expressed by `State` failure
>    flag + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saving per-arm
>    boilerplate return);
> 2. `Driver` has no `config` field—config's only source is `Program.config` (§5 "Avoid Driver
>    holding mutable global state" ruling covers sketch field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded in `DriverOutcome.skipped` internal ledger, not
>    emitted externally—S2 "deliberately fix no bug" zero-diff criterion requires; external emission
>    comes with 4.3 obligations ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: driver arm is the only new caller
>    (L1→L2 is allowed direction); orchestrator existing call sites move away with 4.2 and will be
>    discussed after return.
> 5. Under the multi-file variant ProofExecution is an independent stage, executed **after all**
>    typecheck (A1, 2026-10-09 user ruling): the original `compile_project` interleaved proof in the
>    per-file typecheck loop (file N's proof before file N+1's typecheck). For single failures the
>    two are byte-by-byte identical; in the "file 1 proof failure + file 2 typecheck failure"
>    multi-failure scenario, the first report changes from proof error to typecheck error (test
>    pinboard).
> 6. `DriverOutcome` carries products per ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each discovered file has an entry, clean file is empty Vec—check_project current contract);
>    Discovery product extended to "file set + used_by edge set + shadowing events" triple into
>    State (MultiFile variant has no downstream consumer, only recorded not emitted); check path
>    per-file parse drops from 2 to 1 with C3 topology.
> 8. Lsp variant landing (4.2.3, ruling split by file source, 2026-10-09 user finalization): disk
>    file parse failure same as Check's Plan B (collect + exit compilation unit—two ends degradation
>    converges); edited buffer parse failure preserves incomplete AST and continues typecheck
>    (editor philosophy—mid-typing state should not make semantic features disappear). Discovery arm
>    for Lsp preserves buffer source overwriting disk stale content; Typecheck under Lsp only checks
>    target file (other units only supply signatures for registry); within-file diagnostic order
>    normalization extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old
>    behavior where LSP hard-aborted on unrelated disk file parse errors, handler silently fell back
>    to single-file path quirk, is fixed with this step.
> 9. Embedded variant landing (4.2.4): `Program` adds new `shared_registry` field—embedded std
>    module is sub-compilation, the registry is an **input** not a product (EMBEDDED_ STAGES has no
>    Registry stage), #94's SymbolTable sharing contract is thus from "caller remembers to pass the
>    same registry" to explicit program declaration. Error path text normalized from
>    `<std.test> (embedded std)` to unit virtual path `<std/test>` (only compiler internal error
>    surface, no test pinning). Thus orchestrator's four entries all migrate into Driver.
> 10. standalone check unification (4.2.5, Ruling A + IR stage ruling, 2026-10-09): Check variant's
>     faithful carrier of single-file semantics for no-project-root programs—warning surface only
>     covers entry file (adjacent files only receive errors, rather miss than false-positive);
>     relative `use` resolved along importer's directory (aligned with rustc single-file mod). CHECK
>     stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full chain)
>     already runs IR generation, E3019/E1014/E1015 etc. only produced in ir_gen (runner gate
>     empirical). IrGeneration's Check variant is pure check does not consume IR; Script/Bin split
>     expressed by module_key's None/Some (E3023's existing switch, ir_gen.rs:1660).
>     Monomorphization not in CHECK (only produces resource exhaustion/internal error, zero corpus
>     dependency). **〔2026-10-10 withdrawn by 3.4.8 empirical evidence〕**—"zero corpus dependency"
>     is actually evidence blind zone of corpus having no pathological recursion fixture; E3005 is
>     the only compile-time defense for this class of programs. See note #14. Side fix latent
>     defect: Registry arm's repeated harvest registration for embedded std unit would overwrite
>     native half-surface already merged by with_std() (E1043 false positive)—embedded unit now
>     skips duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence deleted, goes to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll variant: Parsing collects all parse errors, preserves incomplete AST and
>     continues typecheck (editor philosophy extended from 4.2.3 ruling; don't mark stage failure,
>     otherwise topology skip makes typecheck never reachable); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from here on supplements proof
>     (E4018), W-code warnings (W1001–W1005, test pinboard) and IR-level errors; lexical failure
>     changes to reporting real diagnostic (old synthetic "E0001 lexical error" text disappears with
>     sequence deletion—real diagnostic has precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` per-line verification (4.2.7, C5 must-do item):
>     the two entries had already each converged—mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, difference carried by
>     whether collected_errors is empty; statements.rs five collection points (function
>     body/use/for/block/while). Verification confirmed and fixed duplicate diagnostic defect:
>     collection point puts first error in both collected_errors and Err return channel, pass-3's
>     add_error to Err causes same code same span same message ×2 (nested ×3); plus annotation
>     validation signature formal parameter and overall annotation double-visit (E1003/E1103),
>     ownership layer same-point double-emit (E2014/E2018) two independent mechanisms. Fix landed at
>     module result boundary: dedup by (code, span, message)—diagnostic is position fact,
>     duplication carries no information; 63 corpus entries had duplicates removed, run column and
>     exit code zero-drift (borrow_conflict_err's "duplicate" is a baseline triple format not
>     including column number artifact—two E2018 are actually legal diagnostics in different
>     columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver—`Compiler` wrapper layer no longer goes through.
>     Error text byte-by-byte aligned with old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground and SingleFile
>     share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step onward has real
>     producers).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirical evidence overturns "mono only
>     produces resource limit noise": pathological generic recursion (`f(x)=f([x])`) causes compiler
>     process stack overflow (0xc00000fd—the interpreter recurses in Rust layer, no graceful runtime
>     error), mono depth gate is the only compile-time defense; standalone check before 4.2.5 took
>     pipeline full chain and had mono, exclusion ruling caused silent coverage regression ("zero
>     corpus dependency" only proves corpus has no pathological fixture). "Type erasure fallback"
>     assumption voided per RFC-033 reflection ruling—`^^List(Int)` needs real identity of
>     instantiation, monomorphization is reflection's foundation not optional optimization. Design
>     four points: ①typecheck for qualified calls (`lib.f(x)`, FieldAccess form) produces
>     instantiation_request, generic_id uses qualified name, same namespace as merged IR (currently
>     only recognizes bare Var, expressions.rs:2455); ②`Stage::ALL` topology changes to Linking
>     before Monomorphization, MULTI_FILE stage table tail adds Monomorphization— consumes
>     merged_ir, aggregates all-unit requests (deferred bucket's containing_fn qualified), mono
>     upgrades from single-module pass to whole-program pass; single-file stage table unchanged (no
>     Linking, subsequence property preserved, IR snapshot zero-drift); ③CHECK adds Linking (pure
>     merge—E3020 entry check left in RoleClassification, prevents double-report) and
>     Monomorphization (pure check does not consume, same as IrGeneration's Check variant) two
>     stages; ④resource protection follows whole-program BFS covering cross-unit mutual recursion.
>     Monomorphizer body unchanged (consuming ModuleIR + request set contract happens to be merged
>     IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design lands qualified call instantiation
>     collection: `ExpressionInferrer` injects module qualified key table (`use` three arms
>     registered—normal / alias / item-wise alias and only SubModule kind exports),
>     `callee_generic_name` via `SymbolTable::qualify` concatenation merged-IR-same-source qualified
>     name; qualified call first path arity criterion tightened (when parameter table can't resolve
>     falls to second path takes actual args by signature, avoiding cross-module same-name function
>     mismatching false positive E3018). Implementation empirically found three asymmetric defects
>     on the mono side and fixed in the same round: (1) Delete key (generic name set) and rewrite
>     key (containing_fn + name + span triple) granularity mismatch—sites that can't resolve actual
>     args degrade to symbol request into deferred bucket, when container function is non-generic
>     bucket never drains, but the original has been deleted by name → dangling call
>     (`list_ops.yx:53` runtime E6006 empirical); fix is restore gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as single source of
>     truth for coverage judgment, original of uncovered sites preserved. (2) Restore segment
>     `HashSet` iteration order causes function table order non-deterministic (t2 review F1)→ change
>     to `Vec` and sort by name, product byte-level reproducible. (3) Non-rewrite surface form
>     TailCall / MakeClosure originally looks up mapping key (comment claims conservative)→
>     `call_form_is_rewritten` single-point judgment, non-rewrite form all counted as uncovered.
>     Each with pinning test (red state pre-fix empirical discriminating power). Corpus differential
>     baseline same commit regeneration exactly one line (remove_at frame adds `(int64)`). **Commit
>     strategy record**: typecheck collection and mono fix split into two commits, only containing
>     the former's temp tree triggered `stdlib_docs` against HEAD existing defect red
>     (`list.is_empty([])` same family E6006)—the two halves are each other's green light
>     prerequisite, so merged as one atomic commit.
> 16. 2026-10-10 Five Rulings Registration (After R1 Closure): (1) **D2 Ruling Pass**—user module
>     export carries `Export.type_params` with `generic_fn_type_params_snapshot()`, same shape as
>     std path (`yx_sources.rs:62/108`): declared name becomes authoritative source for qualified
>     call monomorphization, signature reverse-inference degrades to fallback (merged into R2
>     scope). (2) **HKT Declaration Side Alive, Call Side Broken** (inferring passing type argument
>     E1002 / explicit type argument E1010) registered as WBS 3.5 (3 third-level tasks). (3)
>     **deferred bucket under non-generic container never drains** merged into 3.4.8 ② acceptance
>     subitem: bucket must drain or loudly report error, no silent non-specialization. (4) **Closure
>     variable called by name runtime E6006** registered as WBS 3.6; semantics (i) support indirect
>     call / (ii) compile-time reject, **ruled (i)**—support indirect call: `CallDyn` mechanism
>     already in place (`ir_gen.rs:7945` closure expression path), broken is `Var` callee being
>     downgraded to by-name `Call`, and typecheck already accepts this program. (5) **Trade-off
>     Criterion Solidified**—plan rejection reasons can only be based on correctness and
>     readability, "large change surface / new mechanism / high cost" does not constitute rejection
>     reason, falls into `coding-rules.md` second part and HOWTO self-check list; trade-offs needing
>     user decision per `AGENTS.md` "When Requesting User Ruling (Mandatory)" give all four items
>     (situation+evidence / per-item pros and cons / clear recommendation+cost / destructive
>     naming).

**Ten entry transformation methods (per-function specification)**:

| Entry                                                      | After Transformation                                                                                                                                                                                               |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                     |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                      |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                        |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtered to target file                                                                                                                                                         |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                          |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfer to `run_project` or `SingleFile`)                                                                                                                                                              |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfer to Driver)                                                                                                                                                                                     |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | Within project change to `Program { kind: Lsp, aggregation: CollectAll }`; **delete the manual lex→parse→… sequence** in the single-file branch, change to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                      |

**The duplication degree of `check_project` and `compile_project` (about 40% shared / 60% forked)**.
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
parse loop, per-file checker assembly, result handling. Forked: items 2/3/4/5/7/8/9/10/11 in the
table above. **This 40% shared skeleton is exactly the value range of `Driver`**—the forked 60% are
all "different stage sets" or "different aggregation patterns", and these are exactly the two fields
of `Program` that can completely express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking problem, continue
}
```

**Key design decision: "upstream failure causes this stage to skip" does not belong to the stage
return value.**

Reason: skip is **topologically** determined, not by the stage itself. If each stage returns
`Skipped` itself, then the "why I didn't run" information is scattered across 12 stages and cannot
be centrally audited. Changed to `dispatch` making topological judgment at the beginning of the
loop:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability to be fixed by
this document—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip", it
produces no diagnostic, just because the obligation happens to be empty. After the rule is
established, any "didn't run Y because X wasn't done" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                    | Text Direction                                                                                                          |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` causes                                   | "Stage Y did not execute due to upstream stage X failure"                                                               |
| Pre-obligation empty causes                               | "Stage Y did not execute due to no pending obligations (normal)"—**this severity = Info, not counted in warning count** |
| Conditions not met (e.g., `config.mono.enabled == false`) | "Stage Y did not execute due to configuration not enabled"                                                              |

The second category is key: it lets "normal skip" and "abnormal skip" be distinguishable in the
diagnostic flow, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Pattern: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                              | Current Correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early returns)              | Exists                 |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                 |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
project-internal files and single files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference of the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early return or
not"—they are two different checker entries. After unification, it should be driven by the
`Aggregation` parameter for the same implementation, not keeping two functions (**this is an
additional item to verify in this document, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Does not introduce new type representations, does not touch `MonoType` / `PolyType` / `ir::Type`                                           |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **New types**, unrelated to language type system                                                                                                     |

**This document does not introduce a fourth type representation.** The convergence of three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type ownership and layering**: what migrates into the ledger is the **settlement
responsibility**, not the type ownership module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`**—this is consistent with the driver non-reverse-dependency red line
below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**. A
`Driver` appearing in `TypeChecker`'s import is treated as a violation, intercepted by RFC-039's
proposed `scripts/ci/check-module-boundary.py`.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                                    |
| ----------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                           |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already stopped bleeding and integrated in P3; **warning surface not expanded** (decision B1, 2026-10-09: dead code family warnings don't enter compile path—pool semantics being corrected as defect by 4.9, normalizing warning surface waits until after 4.9; initial draft "new warning output" in this row voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already stopped bleeding and integrated in P3                                                                                                                                                                                                                                                                                           |
| LSP (within project)                | No proof_execution                       | Already stopped bleeding and integrated in P3                                                                                                                                                                                                                                                                                           |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                               |
| Z3 not installed + multi-file       | Silent skip                              | Same as above, unified                                                                                                                                                                                                                                                                                                                  |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract `backend.rs:60` has declared ("caller should conservatively degrade")—the current
implementation is contrary to its own contract doc.

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                 |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                        |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, change to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **preserved**, moved into `driver` and as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, change to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slim down to Program constructor                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slim down to Program constructor                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` change to Driver                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` change to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to be driven by `Aggregation` parameter                                                                                                                                                                                                 |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, change to Driver call                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                       |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` change to construct `Program { kind: WasmPlayground }` (currently implicitly falls to Path 1 via `Compiler::compile_with_source`)                                                                                                                                                                         |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` change to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm change to produce diagnostic (**fix the second silent discard point**)                                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path add warning diagnostic                                                                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment inconsistent with `1320`                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table change to **actual** execution order; delete "lower-layer failure upper-layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs`/`checker.rs`/`ownership.rs` three consumer points change to shared port. `&'static` raw reference form is infeasible (`dyn Solver` is not `Sync`), original statement per D58 |

**Deleted**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s manual lex/parse sequence.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is on the consumer side, not the producer side**, changing the
producer side would mask architectural defects); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]`
branch (reachability judgment of wasm branches assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                               | Disposition                                                                                                                                 |
| --------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously silently-passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; batch release, expected diagnostic set change goes to CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compiled through start producing warnings       | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), does not change exit code                      |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set adds                                           | First W then E, S2/S4 two steps                                                                                                             |
| `predicate.rs` panic changes to diagnostic                | **Improvement**: no longer crashes                                          | No breakage                                                                                                                                 |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                               | These two paths don't go through proof_execution                                                                                            |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactoring**                                                    | If `pub` API surface has external dependencies need sync; in-repository consumers listed in change list                                     |

---

## Implementation Points

This document corresponds to **P3 (Fix Correctness Vulnerabilities)** and **P4 (Stage Contract and
Unified Driver)** in RFC-039's global stage sequence. The following S1-S5 are the implementation
order within this document, **each step's acceptance criterion references the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration change: each entry's
**diagnostic set** identical + corpus behavior identical).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must first be written as failing.

| Delivery                                                                                                                                                 | Acceptance                                                                                     |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                            | **Must be red**. If accidentally green, this document's vulnerability analysis needs re-review |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Decision D48—not mixed into single-file corpus tree; with `yaoxiang.toml` project fixtures) | Single-file and multi-file two corpus versions' diagnostic sets comparable                     |
| `test_obligations_drained` skeleton                                                                                                                      | Marked `#[ignore]`, turns green at S4                                                          |

**Rollback point**: no code change, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Delivery                                                 | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This phase deliberately fixes no bug**—it only moves existing behavior into Driver. Acceptance
criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` single commit, `pipeline.rs` restored to original.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Delivery                                                                                                                       | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all change to Program constructors | Four entries' diagnostic sets unified by `Aggregation` identical                  |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                      | `yaoxiang check` behavior consistent inside and outside project                   |
| LSP manual stage sequence deleted                                                                                              | LSP and CLI give same diagnostic set for same file                                |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                  | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fix                                                                                          | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic |
| `predicate.rs:34-36` `.expect()` change to diagnostic                                                                          | No longer panic without Z3 environment                                            |

**Rollback point**: vulnerability fix and structural merge **split into two commits**. If structural
merge has issues, can only roll back structural commit, preserve vulnerability fix commit—then
`test_multifile_proof_obligation_not_dropped` stays green; reverse (preserve structure, roll back
fix) will turn red, is unacceptable intermediate state, forbidden to merge.

### S4: Enable Obligations Ledger

| Delivery                                     | Acceptance                                             |
| -------------------------------------------- | ------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                 |
| `scripts/ci/check-obligations.py`            | Static gate effective                                  |
| Obligation diagnostic W → E promotion        | Full corpus differential then manual review each new E |

**Rollback point**: severity of `assert_drained()` can be controlled by config item, W/E switch
doesn't need code structure change.

### S5: Proof Layer and wasm Wrap-up

| Delivery                                                                                                                                                                                                                       | Acceptance                                                                                                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order change to actual order                                                                                                                                                                          | Document and `checker.rs` call sites correspond one by one                                                                                                                        |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter cache**—otherwise one timeout固化 across compilations through process-level cache, and pollutes cargo test cross-thread sharing) | Production path `default_solver()` call sites reduced to zero (only tests preserve); cache hit rate observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation add warning                                                                                                                                                                                 | Diagnostic when no `body_checker`                                                                                                                                                 |
| Reachability judgment of 12 wasm attributes in `orchestrator.rs`                                                                                                                                                               | Conclusion goes to `06-cleanup-inventory.md`, this document only registers judgment need                                                                                          |

**Note**: correcting layer order will change diagnostic set, may expose large amounts of
previously-silent `Unproven`. **This item should go independently from S1-S4**, don't mix with entry
merge in the same PR.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                   | Reason                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustive enum, **refuses runtime registration**                                                                     | Exhaustive `match` is zero-cost compile-time enforcement: when adding a variant `dispatch` fails to compile. This is isomorphic to the mechanism the opcode table has been verified in RFC-039 routing table B. `StageScope` further turns "per-module or project-level" into a type fact, eliminating the call count problem that needs human reasoning    |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, paired with `scripts/ci/check-obligations.py`'s **static** gate; severity first W then E    | Fixes a whole class of bug, not one bug: currently verified same-class hidden danger at least 2 places (`proof_calls`, `checker.rs:1313`), 16 span-keyed fields all in range. First W then E is to make each step's C2 criterion usable—promoting to E in one step will make `yaoxiang check` suddenly have a large amount of previously-silent diagnostics |
| **Fix scope**            | Fix **consumer side** (orchestration layer), **don't touch** the producer-side logic of `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branches | Problem is on consumer side, not producer side. Changing producer side would mask architectural defects as "logic fixed", and the logic of these three branches is itself correct                                                                                                                                                                           |

This design additionally eliminates two classes of cracks:

- **The crack between "comment promise" and "code behavior"**. `checker.rs:5165`'s "silent pass must
  not be resurrected" and `layers/README.md:3`'s "lower-layer failure upper-layer doesn't run" are
  both **comment-level contracts**—the former is honored by `assert_drained()`, the latter is
  exposed by topology-driven `Skipped` diagnostics.
- **"One array forgot to append"**. Currently five functions each hand-write the call sequence;
  after unification there's only one stage table, and missing wiring for a new stage will error at
  compile time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—of 11 inconsistencies, 7 (dead
  code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, are "should we run some analysis"
  configuration problems; it also can't express `Aggregation`, can only move `check_module` /
  `check_module_collect_all`'s two-choice from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when adding a stage `dispatch` no longer fails to compile, equals changing "five
  functions each hand-write call" to "one array forgot to append", which is exactly the cause of the
  current 11 inconsistencies. The repository already has same-form counterexamples:
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs, don't touch structure**—can fix known 2 bugs, but only covers 1 of 16 obligation
  fields, `Stage` still scattered in 5 functions, next fork point will continue to grow from here.
  **It must be done first** (is part of S1/S3), because structural transformation needs a known red
  test to prove criteria are valid.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is finding bugs at the wrong level: bug
  is in orchestration layer, detection must be in orchestration layer.

## Known Limitations and Risks

- **The slim-down of `orchestrator.rs` will make it harder to read in the short term**. After
  139-line `compile_project` is split into "Program constructor + several driver arms", readers need
  to cross two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent problems"**. After the fix, a batch of "new error" user reports may
  appear—they're real bugs that just hadn't been reported. Must be clearly stated in CHANGELOG.
- **The severity switch of `assert_drained()` needs per-field manual judgment**. Of 16 fields, some
  (e.g., `module_namespaces`) "no consumer" may be design-intended not needing consumption,
  shouldn't report. S4 needs to go through each field, can't be one-size-fits-all.
- **The `Program` abstraction may be premature**. If some entries' stage sets are long-term
  unstable, `stages()` will degenerate into "passing different arrays each call" free parameter,
  contract constraints fall through. **Mitigation is `test_program_stage_coverage` asserting
  `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criterion depends on
  [equivalence criteria document](07-equivalence-oracle.md)'s vulnerability test being red first.
  **If that document doesn't first establish the multi-file corpus layer, S1 cannot be
  accepted**—because the existing 293 corpora all go through single-file path, zero coverage for
  this class of defect. This document doesn't involve the IR verifier (`verify_loose`), that
  prerequisite work is not in this document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order opposite to declaration). This is diagnostic
  set change risk, should go independently.
- **Thread safety after SMT backend changes to singleton is undetermined**. `backend.rs:13-19`
  already states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to cross-compilation-unit shared singleton, need to confirm all access paths go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order dependency defects** (e.g., the actual order dependency of
  termination and ownership). Should be opened after the equivalence criteria stabilize.

> **All open questions originally listed in this section have been ruled.** Per-item decisions see
> [RFC-039 Decisions Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing table A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction conventions,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactoring (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "silent pass must not
  be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower-layer failure
  upper-layer doesn't run" (no short-circuit in reality)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that must be consumed
  downstream
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of playground (single-file path, so executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate not main crate
- `build.rs:19-55` — Error code build-time gate, example of project's enforcement mechanism
