# Stage Contracts and Obligation Ledger

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage order are in the main body of
> RFC-039; the positioning of each subsidiary document is in [the directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** implementation blueprint for RFC-039. It addresses
one problem: **"Which stages ran in this compilation, which didn't, and why"** is currently
scattered across six paths and ten entry functions with hand-wired connections, and nowhere can
answer this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` reads "Unproven → compile error, no
degradation, no silent pass…… silent pass must not be revived." But this invariant only holds on the
**single-file path**: refinement constraints like `x: Sorted(3)` in the proof function form
**silently pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths; the proof
function is never executed, and no diagnostics are produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-function改造 plan for the ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, implementation
  order.

### Not Covered

- Four-layer model, dependency direction conventions, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → main body of RFC-039

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order** to do it; this document gives L1's
**specific form**, **per-file change list**, and **internal implementation stages** (corresponding
to RFC-039's global sequence P3 "fix correctness holes" and P4 "stage contract and unified Driver").
Where this document conflicts with RFC-039, RFC-039 takes precedence.

Equivalence criteria are executed under the **C2 (orchestration changes)** category of the
[equivalence criteria document](07-equivalence-oracle.md): diagnostic sets from each entry point are
the same + corpus behavior is the same.

## Current State

> All items in this section are **verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundary is the Only "One Mistake, Whole Repo Silent" Structural Defect

It differs in nature from the other two categories of defects (module boundary, test wiring):

| Defect                | Typical Manifestation               | Signal Present       |
| --------------------- | ----------------------------------- | -------------------- |
| Lexical/syntax error  | Source code is wrong                | Has diagnostic       |
| Type mismatch         | Type is wrong                       | Has diagnostic       |
| Stage mis-wiring      | Some stage was never called         | **No signal at all** |
| Obligation unconsumed | Field was filled but no one read it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
was written, not **what wasn't written**. This is precisely the root cause diagnosed by RFC-039:
"This project treats 'design' as a documentation convention rather than an executable constraint."

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms—they just haven't been extended to the
stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subjected to
  **build-time hard gating** by `build.rs:19-55` through `tools/code-tables`, comparing each against
  the RFC-013 code table, with any mismatch directly `panic!`-ing and refusing compilation.
- `src/package/` (**76 files / 13,012 lines**, of which the `tests/` directory accounts for 6,227
  lines / 47.9%) has high test density, with every module's test subtree declared and wired—this is
  the most completely-wired block in the repo for testing, and can serve as a formal reference for
  stage-layer gating.

**Design capability is sufficient. What's missing is "putting a same-level gate in the orchestration
layer."**

(Line count metric: `(Get-Content).Count`, see the "Line Count Metric" section in
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                   |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | --------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages, direct call                               |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                    |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking              |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics        |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                  |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR            |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1                   |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                       |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes 3c, otherwise hand-rolled lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                       |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1            |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below gives evidence cell by cell. **A blank does not mean "this entry doesn't do this
thing", but rather "no code in this entry does this thing"**—this is the very essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                       | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                  | **No**                                                       | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                  | Yes `356-377` (role-aware, no config gate)                   | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                  | Yes `336-348`                                                | **No**                             | **No**                                 |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never output**                          | Yes `350`                                                    | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                  | Yes (same as 2)                                              | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                  | N/A                                                          | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **first error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)  | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                  | Yes (`321-328`)                                              | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                     | **No**                                                       | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                         | Yes (`152-236`)                                         | **No**                                                       | **No**                             | Yes (independent ModuleIR then merged) |

> **Review Note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is the diagnostic
> snapshot at the `9e02e4db` baseline, preserved unchanged. Status differences after P3 landing:
>
> - **Row 1 has been fixed**: proof_execution is shared across five entries via the same
>   implementation in `frontend/proof_execution.rs` (pipeline + four orchestrator entries), and the
>   sole consumer defect is gone.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not output W1003, does not run the
>   dead code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization is single-file exclusive) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unproven defect).
> - **Row 7** check_module / check_module_collect_all dual entry → assigned to 4.2.7 (Aggregation
>   parameter-driven).
> - The code-side reference to "ruling #434" had no prior docs record—now back-registered as RFC-039
>   **D57**.
> - **Embedded std included in asymmetry (new fact outside the table, back-registered 2026-10-09)**:
>   The single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch), while multi-file `discover` previously only recognized
>   explicit `use`—in-project for-loops compile successfully but produce runtime E6006 (probe
>   evidence). Fixed and WBS 4.10.2 cancelled; the same-family "multi-file missing a single-file
>   step" parse hard-abort oddity is filed as WBS 4.10.1—**also fixed** (lands immediately after
>   4.2.2; Check path downgrades to per-file collection; from the perspective of this audit's "stage
>   coverage / field consumption" dimension, this is a compilation unit membership difference—a
>   missed dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   The Registry arm duplicates registration for embedded std units via `extract_module_info`, which
>   overwrites the native half registered by `with_std()` in the "native + yx surface merge"
>   (`result.is_err` etc. lost → std.test false-positive E1043). The third missed dimension of this
>   audit: "the merge semantics of two registration sources for the same module key"—the single-file
>   path's registration table is only formed once via `with_std()`, while multi-file/Check's
>   Registry arm inserts per-unit before any coverage is formed.

**Two places need precise expression, otherwise things will be written wrong:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  in `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function **has no read site for `result.warnings` anywhere in its
  body**; the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion is also from different sources**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "does a manifest exist"). The two functions give different answers to "what files must
  define `main`."

### Correctness Vulnerability: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — Obligation production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** location in the repository
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

The semantics: "the predicate argument of this refinement constraint is a compile-time literal; the
proof function must actually be executed to determine it."

**Step 2 — Consumption point is inside the checker, but only emits, doesn't consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                       | Behavior                |
| -------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → pushes hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call site argument check   | `5306` → pushes hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → pushes hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original text of `checker.rs:5165-5169`:

> `RFC-027 §4/§9：Unproven → 编译错误，无降级、无 silent pass。` ……
> `本分支因此成为真防线而非前瞻兑底：不得让 silent pass 复活。`

That is: the author knows "producing no diagnostic" is a defect, classifies it explicitly as a
**known, nowhere-to-go intermediate state**, and assumes in the design that "someone will definitely
come to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it; `checker.rs:1439` writes to `TypeCheckResult` with
`proof_calls, // Phase 2.5 预登记证明函数义务`. The field is defined at `types.rs:29`.

**Step 4 — The entire repository has only one read site.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code is **only read at one place: `src/frontend/pipeline.rs:187`**
(passed as argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier throughout the repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four entries in the orchestrator skip that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**, calling
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they don't even reach the
sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: proof obligations of the standard library itself also go through the drop
path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of embedded std modules themselves
are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, produces no
diagnostics** on the multi-file / check / LSP three paths. The proof function is never called.

**Step 8 (Supplementary Verification) — There's a second silent-drop point within the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership-check layer's `Unproven` is swallowed by the empty match arm, with no diagnostic, no
ledger entry.** This directly contradicts the `5164` branch's "silent pass must not be revived"
promise, and is **independent of the orchestrator issue**—even if the entry layer is completely
fixed, this spot remains silent. (Handling timing: should be processed in the same batch as the
obligation ledger, as it is the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Multi-file path has zero coverage for proof obligations.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `SumUpTo` calls exist in
`result.proof_calls`. But they all **go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping just
before `pipeline.rs`**.

The test file's own documentation comment has already stated this.
`rfc027_refined_transparency.rs:13-15`:

> `本文件只断言 check_module 能看到的东西。证明调用（E4018）由 pipeline.rs 在 check_module 之后执行，故精化违反的用例在 .yx 层`

**This is the shape of the problem: the test verifies "obligations are filled in", while the bug is
"the consumer didn't read them".** A test that only tests the producer side, not the consumer side,
is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Main Corpus is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) uses the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-tier criterion** and the main acceptance
method for the C2 stage. But the actual test:

- **No `yaoxiang.toml` anywhere under the `tests/` directory** (zero hits from full-directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files go through the single-file path, all of them cover
  proof_execution, none of them cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is the
empirical evidence for this point. The file is annotated with `// expect: compile-error E4018` (line
15), and its header comment line 7 reads "Status: ❌ should be rejected at compile time"—**it passes
precisely because it goes through the only path that executes the proof function**.

**Conclusion: The third-tier criterion needs to add a multi-file corpus layer, otherwise it cannot
serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 16 "Produce-on-Contract" Fields, Zero Mechanism Guarantees

Of the `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, **16 fields** have
documentation comments explicitly stating "produced by stage X → consumed by stage Y", i.e., they
are **cross-stage obligations** by nature:

| Field                      | Definition Line | Producer → Consumer                                                  |
| -------------------------- | --------------- | -------------------------------------------------------------------- |
| `proof_calls`              | `29`            | typecheck → **proof_execution**                                      |
| `release_plan`             | `31`            | ownership → IR generation (read at `ir_gen.rs:341`, used at `:1942`) |
| `escaped_refs`             | `33`            | ownership → IR generation                                            |
| `instantiation_requests`   | `35`            | typecheck → monomorphization (`pipeline.rs:381`)                     |
| `existential_coercions`    | `37`            | typecheck → IR generation                                            |
| `implementation_proofs`    | `39`            | typecheck → IR generation                                            |
| `interface_impl_registry`  | `42`            | typecheck → operator query / constraint solver / LSP                 |
| `sum_types`                | `44`            | typecheck → IR generation                                            |
| `sum_type_param_names`     | `47`            | typecheck → IR generation                                            |
| `variant_ctor_calls`       | `49`            | typecheck → IR generation (span-keyed)                               |
| `operator_dispatches`      | `51`            | typecheck → IR generation (span-keyed)                               |
| `method_overload_ir_names` | `53`            | typecheck → IR generation                                            |
| `overload_resolutions`     | `56`            | typecheck → IR generation (span-keyed)                               |
| `try_expr_impls`           | `59`            | typecheck → IR generation (span-keyed)                               |
| `match_scrutinee_types`    | `63`            | typecheck → IR generation (span-keyed)                               |
| `module_namespaces`        | `66`            | typecheck → IR generation                                            |

**All of these fields are passed across layers keyed by `Span` or name tables.** The equivalence
criteria document has already pointed out that mismatches in the span-keyed contract will "silently
fail with no error" (`ReleasePlan` is typical: any inconsistency in how spans are computed on either
side → `Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:341` (`release_plan: type_result.release_plan.drops.clone()`)
  is inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in the downstream module, and the downstream module is
  on the must-pass path of all entry points.
- `proof_calls`'s consumer is at the **top level** of `pipeline.rs`, which belongs to **another
  entry implementation**. The orchestrator's four entries don't even pass through the `pipeline.rs`
  layer.

**Verified fact**: Currently the repository has **no mechanism** to ensure these 16 fields are
consumed. The only "protection" is that `ReleasePlan` happens to hitch a ride with IR generation.

### Four Additional Contract Defects in the Proof Layer

The following are all from proof layer analysis, independent of the entry-point divergence issue,
but all belong to the category of "stage contract not enforced."

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer isn't
in the pipeline at all.**

`src/frontend/core/typecheck/layers/README.md:5-11` declares the layer order:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "executed in layer order, lower-layer failure skips upper layers." Actual call
sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **opposite** to the declaration; the declared
Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all** (it's only
used as an `is_subtype` utility function by `inference/assignment.rs:17`, unrelated to
`ProofResult`). And **there is no short-circuit at all**—`checker.rs:1271-1276` adds each
termination error via `add_error` and continues; the README's promise of "lower-layer failure skips
upper layers" doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review Note (2026-10-07)**: The hard-failure panic was eliminated by P3's 3.3.1 (SOLVER slot
> Option-ified, missing → conservative degradation to `SMTResult::Unknown`); the "silent skip (don't
> inject)" was supplemented with W1081 signal by 3.4.2. The unification of the three forms
> (singularization) is executed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (added process-level shared singleton + `with_shared_solver` closure port);
> the `default_solver() → &'static` formulation in the 02 change list is governed by D58 (the bare
> `&'static` reference form is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver is Unavailable                                |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction time via `with_solver_owned`    | `None` → **silently skipped** (not injected)              |
| `ownership.rs:627` (back-edge cut determination)       | **Each call** to `default_solver()`                      | `None => return false` (`629`) → conservative no-cut      |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure maintains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'." Yet `backend.rs:60`'s contract documentation says "`None`: backend unavailable……**the
caller should conservatively degrade**." **Two philosophies coexist in the same code, and the
hard-failure one panics instead of returning an error.**

**(c) Each back-edge decision creates a new Z3 context, rendering the cache useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut determination. And
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
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while `z3_backend.rs:17`'s
documentation comment claims "SMT query results are cached in `cache`." Not shared across calls ⇒
**this cache never hits in this call pattern**.

(Comparison: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. Same backend, two lifetime
strategies.)

**(d) Checker silent degradation + comment doesn't match reality.** `checker.rs:1283-1287` and
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
table** and runs as usual—ownership analysis degenerates to "nothing conflicts", **without any
warning**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least left
a trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking, before constraint
solving." Actually `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at `1256`,
ownership after, i.e., the "before" the comment mentions is actually "after two layers"**.

### Wasm Current State: shim crate carrier, 27-file branch is live

The wasm target **has been built and is in CI**, with `cdylib` not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                           |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                 |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                 |
| shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                        |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                    |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                           |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from a fixed URL, downgrades to warning on missing)                                                                                                                                                           |
| **27 files** contain `wasm32` literal             | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are mentioned in documentation only                                                                                                                               |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                         |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                         |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target—`lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` entirely (these
three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: `wasm/src/lib.rs:48`'s playground
entry calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Executes proof function? |
| ------------------------------------------ | ------------------------ |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                  |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                  |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                  |
| Multi-file `run` (`compile_project`)       | **No**                   |
| `check` (`check_project`)                  | **No**                   |
| LSP in-project (`check_source_in_project`) | **No**                   |

That is: **the existence of `wasm/` turns "proof_execution has one consumer" into "it has three"**,
but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope—which of the 12 wasm attributes in `orchestrator.rs` are unreachable in playground
scenarios—goes to `06-cleanup-inventory.md`. The open question in RFC-039 "build wasm target, or
delete these branches" is **already obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (exhaustive, not runtime-registered)

**Core constraint: `Stage` is a compile-time exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can force at compile time that "a new stage must be
handled by the orchestration layer"—this is the mechanism already proven effective by the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexing + syntax               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type checking (with embedded proof layer)   PerModule
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

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once at
the project level) make "which stages must be run per module and which must be run once at the
project level" a type-level fact. The `Project`-scoped stage is structurally guaranteed by
`dispatch` to run only once—this eliminates the need for human reasoning about questions like
"should `allocate_global_slots` be called for every file" in the current `orchestrator.rs`.

**The stage table is in topological order, not alphabetical**, because failure propagation depends
on order.

> **Revision Note (P4 implementation, 2026-10-07)**: Two deviations from the original draft,
> corrected per implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the original order contradicts data flow.
> 2. The Check-form stage table is normalized to "dead code **before** proof" per the `Stage::ALL`
>    topological order: The current `check_project` runs proof before dead code; the two have no
>    data dependency, and the diagnostic set is the same (C2 set semantics), only normalizing the
>    within-file diagnostic order. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to the ALL order; byte-level acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    original order would force the Registry arm to "hide parse", making the stage table lie about
>    data flow; and multi-file parse thus goes from 2 times to 1 time. Side-effect honest
>    registration: multi-file path parse error was previously a hard abort (the `?` propagation in
>    `build_registry_from`, tension with CollectAll semantics registered as WBS 4.10.1)—**4.10.1 has
>    been fixed** (2026-10-09 user ruling: rust-style collection semantics + plan B): the Check
>    path's parse failure downgrades to per-file diagnostic collection, the infected file exits the
>    compilation unit (doesn't enter registry, importer reports E5001); the MultiFile path retains
>    hard abort (under FailFast a bad file can't produce IR, which is correct semantics, pinned for
>    long-term validity).
> 4. Check-form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edge count 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the original draft missed these
>    (these two products in the orchestrator era were produced by `discover_with_used`, discarded by
>    the `discover` wrapper, the data flow was lost by not entering the table); b. Within-file
>    diagnostic order normalization extended to the full form of note #2: E3020 entry validation
>    moved into the RoleClassification arm (Check has no Linking stage, the arm holding surfaces and
>    AST is responsible for validation), within-file diagnostic order = stage topological order
>    (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set
>    semantics), only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no consumer" from undetectable to a fact assertable at
compile time or test time. RFC-039 has already listed this as "the highest-value item."

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

**Settlement semantics (three levels)**:

| Situation                                | Criterion                                               | Disposition                                                             |
| ---------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------- |
| Obligation read by its declared consumer | Field in `Obligations` is `take()`-en / marked consumed | Pass                                                                    |
| Obligation non-empty but no consumer     | Field non-empty and not consumed                        | **Produce diagnostic** (`W` level by default; `E` level in strict mode) |
| Obligation empty                         | Field is empty                                          | Pass (no consumption needed)                                            |

**Why start with W level instead of E level**: fixing the obligation drop will **change the
diagnostic set**. C2 requires "diagnostic sets from each entry point are the same"; if we jump to E
level in one step, `yaoxiang check` will suddenly produce a large number of `Unproven` diagnostics
that were previously silent. Two steps (W observation first, then upgrade to E) make each step's
criterion usable. See "Implementation Points" S4.

**`assert_drained()`'s sole call site**: `Driver::run` at the end of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (field
appears ≥2 times but no third file reads it → fail). `assert_drained()` is the **runtime** gate,
that script is the **static** gate; the two complement each other, both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project（lib.rs:154）
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP in-project, filter results to target file）
    Embedded,          // compile_embedded_module（embedded std）
    WasmPlayground,    // wasm/src/lib.rs:48 —— the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Must come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations (one per `ProgramKind`), not freely
passed as an array by the caller.** This is the key constraint preventing the `Program` abstraction
from degrading into a "free parameter that takes anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not the stage's return value
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

> **Revision Note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for record—
>
> 1. The §4 `StageOutcome` tri-state is not returned by the stage arm, but expressed via `State`'s
>    failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saving
>    per-arm boilerplate returns);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" determination covers the sketch field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded in the internal `DriverOutcome.skipped`
>    ledger, not emitted outward—S2's "intentionally not fix any bug" zero-diff criterion requires
>    this; outward emission comes with 4.3's obligation ledger;
> 4. `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the sole new
>    caller (L1→L2 is an allowed direction); orchestrator existing call sites will be relocated by
>    4.2, then discussed.
> 5. In the multi-file form, ProofExecution is an independent stage, executed after **all**
>    typecheck (A1, 2026-10-09 user ruling): the original `compile_project` interleaves proof within
>    the per-file typecheck loop (file N's proof before file N+1's typecheck). Single failure: the
>    two are byte-for-byte identical; in the "file1 proof failure + file2 typecheck failure"
>    multi-failure scenario, the first report changes from proof error to typecheck error (test pin
>    board).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every found file has an entry, clean files have empty Vec—current check_project contract); the
>    Discovery product is extended to "file set + used_by edge set + shadowing events" triple into
>    State (MultiFile form has no downstream consumer, just recorded not emitted); check path's
>    per-file parse goes from 2 times to 1 time per the C3 topology.
> 8. Lsp form landing (4.2.3, decided by file source分流 ruling, 2026-10-09 user final): Disk file
>    parse failure uses Check's plan B (collect + exit compilation unit—two ends degrade
>    convergence); edited buffer parse failure retains incomplete AST to continue typecheck (editor
>    philosophy—mid-typing states shouldn't make semantic features disappear). Discovery arm for Lsp
>    reserves buffer source code to overwrite stale disk content; Typecheck in Lsp only checks the
>    target file (other units only provide registry for signatures); within-file diagnostic order
>    normalization principle extends to LSP (typecheck diagnostic → W-code warning → proof error).
>    Old behavior: LSP's hard abort on irrelevant disk file parse errors, handler silently falling
>    back to single-file path, that oddity is fixed in this step.
> 9. Embedded form landing (4.2.4): `Program` adds a `shared_registry` field—embedded std modules
>    are sub-compilations, the registry is **input** rather than product (EMBEDDED_ STAGES has no
>    Registry stage), #94's SymbolTable sharing contract is thus "the caller remembers to pass the
>    same registry" made explicit as a program declaration. Error path text from
>    `<std.test> (embedded std)` normalized to unit virtual path `<std/test>` (only internal
>    compiler error surface, no test pinned). At this point all four orchestrator entries migrate to
>    the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): The Check form's
>     faithful carrying of single-file semantics for programs without a project root—warning surface
>     only covers the entry file (neighboring files only receive errors, missing is preferred over
>     false positives); relative `use` resolved along the importer's directory (rustc single-file
>     mod alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: the standalone old path
>     (pipeline full chain) originally ran IR generation, E3019/E1014/E1015 etc. only produced in
>     ir_gen (runner gate evidence). Check form of IrGeneration is pure check, doesn't consume IR;
>     Script/Bin form expressed by module_key's None/Some (existing switch for E3023,
>     ir_gen.rs:1660). Monomorphization doesn't enter CHECK (only produces resource limit / internal
>     errors, zero corpus dependency). **〔2026-10-10 withdrawn by 3.4.8 evidence〕**—"zero corpus
>     dependency" is actually evidence blind spot for corpora without pathological recursive
>     fixtures; E3005 is the sole compile-time defense for this class of programs. See note #14.
>     Side-effect fixing of latent defect: Registry arm's duplicate harvesting of registrations for
>     embedded std units would overwrite the native half-merged by with_std() (E1043 false
>     positive)—embedded units now skip duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the hand-rolled
>     lex→parse→check_module_collect_all sequence in `run_diagnostics` is deleted, going through
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share one Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains incomplete AST to
>     continue typecheck (editor philosophy extends from 4.2.3 ruling; not marked as stage failure,
>     otherwise topology skipping makes typecheck never reachable); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from here on completes proof
>     (E4018), W-code warnings (W1001–W1005, test pin board) and IR-level errors; lexical failure
>     now reports real diagnostics (the old synthesized "E0001 词法错误" text disappears with the
>     sequence deletion—real diagnostics have precise spans, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries had already converged—mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain both run, difference carried by whether
>     collected_errors is empty; five collection points in statements.rs (function
>     body/use/for/block/while). During verification, duplicate diagnostic defect was confirmed and
>     fixed: the collection point puts the first error into both collected_errors and the Err return
>     channel, pass-3's add_error for Err causes the same code same span same message ×2 (nested can
>     be ×3); two additional independent mechanisms: annotation validation signature form parameters
>     and overall annotation double visit (E1003/E1103), ownership layer same point double-emit
>     (E2014/E2018). Fix lands at module result boundary: deduplicate by (code, span,
>     message)—diagnostics are positional facts, duplication carries no information; 63 corpus
>     entries remove duplicate copies, run column and exit code zero drift (the "duplication" in
>     borrow_conflict_err is an artifact of the baseline triple not including column number—two
>     E2018 are actually legal diagnostics at different columns).
> 13. Wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver—`Compiler` wrapper layer is no longer involved.
>     Error text byte-by-byte aligned with old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground and SingleFile
>     share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step makes a real
>     producer).
> 14. Multi-file monomorphization and CHECK stage table fix (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Evidence overturns "mono only produces
>     resource limit noise": pathological generic recursion (`f(x)=f([x])`) on the multi-file path
>     without mono causes the compiler process to stack overflow (0xc00000fd—the interpreter
>     recurses in the Rust layer, no graceful runtime error), mono depth gate is the sole
>     compile-time defense; standalone check before 4.2.5 goes through the pipeline full chain and
>     has mono, the exclusion ruling causes silent coverage regression ("zero corpus dependency"
>     only proves the corpus has no pathological fixtures). The "type erasure fallback" assumption
>     is invalidated by RFC-033 reflection ruling—`^^List(Int)` needs the real identity of
>     instantiation, monomorphization is the foundation of reflection rather than an optional
>     optimization. Four design points: ①typecheck produces instantiation_request for qualified
>     calls (`lib.f(x)`, FieldAccess form), generic_id uses the qualified name, sharing namespace
>     with merged IR (currently only recognizes bare Var, expressions.rs:2455); ②`Stage::ALL`
>     topology changed to Linking before Monomorphization, MULTI_FILE stage table tail adds
>     Monomorphization—consumes merged_ir, aggregates all-unit requests (containing_fn qualification
>     for deferred bucket), mono elevated from single-module pass to whole-program pass; single-file
>     stage table unchanged (no Linking, subsequence property preserved, IR snapshot zero drift);
>     ③CHECK adds Linking (pure merge—E3020 entry validation stays in RoleClassification, preventing
>     double reporting) and Monomorphization (pure check doesn't consume, same as Check form of
>     IrGeneration) two stages; ④resource protection covers cross-unit mutual recursion with
>     whole-program BFS. Monomorphizer body unchanged (consuming ModuleIR + request set contract is
>     exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design landing qualified call
>     instantiation collection: `ExpressionInferrer` injects module qualified key table (`use`
>     three-arm registration— normal / alias / item-by-item alias and only SubModule type exports),
>     `callee_generic_name` via `SymbolTable::qualify` concatenates qualified name from same source
>     as merged IR; qualified call first-path arity criterion tightened (when parameter table can't
>     be resolved, fall to second path to take actual arguments by signature, avoiding cross-module
>     same-name function mismatch false E3018). During implementation, three asymmetric defects on
>     the mono side were empirically found and fixed in the same round: (1) Delete key (generic name
>     set) and rewrite key (containing_fn + name + span triple) granularity mismatch—sites that
>     can't resolve actual arguments degrade to symbol request entering deferred bucket, when
>     container function is non-generic the bucket never drains, but original item has been deleted
>     by name → dangling call (empirical `list_ops.yx:53` runtime E6006); fix is recovery gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as single source of
>     fact to determine coverage, original items for uncovered sites are preserved. (2) Recovery
>     segment `HashSet` iteration order causes function table order non-deterministic (t2 review F1)
>     → change to `Vec` and sort by name, product byte-level reproducible. (3) Non-rewrite surface
>     forms TailCall / MakeClosure look up mapping keys as-is (comment claims conservative) →
>     `call_form_is_rewritten` single-point determination, non-rewrite forms counted as uncovered.
>     Each of the three has pinning tests (red state before fix empirically discriminative). Corpus
>     differential baseline same commit regenerated exactly one line (remove_at frame added
>     `(int64)`). **Commit Strategy Note**: typecheck collection and mono fix split into two
>     commits, the temporary tree containing only the former triggered `stdlib_docs` red against
>     HEAD existing defect (`list.is_empty([])` same family E6006)—two halves are each other's green
>     premise, so combined into one atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **Ruling D2 First**—user module
>     export carries `generic_fn_type_params_snapshot()` via `Export.type_params`, isomorphic to the
>     std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified call monomorphization, signature reverse derivation degraded to fallback (merged
>     into R2 scope). (2) **HKT declaration side live, call side broken** (infer pass type argument
>     E1002 / explicit pass type argument E1010) registered as WBS 3.5 (3 level-3 tasks). (3)
>     **deferred bucket never drains under non-generic container** merged into 3.4.8 ② acceptance
>     sub-item: bucket must drain or loudly report error, no silent non-specialization. (4)
>     **Closure variable by-name call runtime E6006** registered as WBS 3.6; semantics (i) support
>     indirect call / (ii) compile-time reject, **ruling (i)**—support indirect call: `CallDyn`
>     mechanism already exists (`ir_gen.rs:7945` closure expression path), broken is `Var` callee
>     being downgraded to by-name `Call`, and typecheck already accepts this program. (5)
>     **Trade-off criterion fixed**—the reasons for rejecting a solution can only be based on
>     correctness and readability, "large change surface / introduce new mechanism / high cost" does
>     not constitute a rejection reason, falls into `coding-rules.md` second part and HOWTO
>     self-check list; trade-offs requiring user decision per `AGENTS.md` "When requesting user
>     ruling (mandatory)" provide all four items (situation+evidence / per-item pros and cons /
>     clear recommendation+cost / destructive naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE tail adds mono; Linking
>     arm **before taking each unit's IR** aggregates whole-program requests and qualifies
>     `containing_fn` (otherwise deferred bucket keys are from a different source than merged IR,
>     bucket never drains). **① closure (Ruling D2 First)**: user module export carries declared
>     type parameter snapshot via `Export::type_params`, isomorphic to std path
>     (`yx_sources.rs:62/108`). **Empirically found and fixed existing defects during process**: (a)
>     **F1**—`qualify_names` only rewrites function body, `ir.init` is never visited, multi-file
>     function definitions are already qualified while module top-level calls remain bare names →
>     runtime E6006 (**reproduces with non-generic**, unrelated to generics); fix is init reusing
>     same `rename`/`aliases` and same `rewrite_call_names`. (b) mono side "whole-program unique
>     suffix" heuristic and miss-silent `continue` → delete heuristic, miss change to **loud E3018**
>     (ambiguous silent mismatch is worse than not parsing). **Independently verified and fixed
>     regression**: t6 found that after named import `use lib.{pick}` bare-name calls are falsely
>     rejected by E3018 (IR instruction name is alias qualified name, request name is bare name,
>     aggregation side only qualifies by unit key) → fix is aggregation side **directly calling
>     ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as pub single point, one
>     place produces one place consumes), with matrix pin test covering two forms of named imports.
>     **Record keeping**: init segment `DefId` blank was empirically a "consistency blank" not an
>     active defect (codegen name fallback resolvable, four types of init calls tested pass),
>     registered as WBS 3.7.4 to close the "function body via def, init via name" caliber fork.
> 18. Init segment DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), function
>     body and `ir.init` share the same implementation and same `resolve` closure; 208 IR snapshots
>     regenerated with only 309 pairs of `(def=_)`→`(def=dN)` change (other-lines 0; whole tree init
>     segment 0 residual `(def=_)`). codegen's DefId main path and name fallback two-state
>     translation result and execution value item-by-item consistent (four types of init calls
>     tested), corpus baseline zero drift; probe flipped per new caliber and strengthened
>     (identity + precise index + execution value, fallback init coverage means red). **Errata
>     (2026-10-11)**: The above commit numbers were rewritten via `git pull --rebase`—original
>     b554497b/e60dcc87/b7ac7396 correspond to post-rebase 375789d2/d922a551/38bd4b9e in order;
>     pushed dev history uses new numbers (old numbers are local temporary numbers, never entered
>     remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (Design basis of ③④ see #14; ②
>     landing see #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), entry validation block stays in
>     `RoleClassification`; `ir_generation_check` generated then retained `us.ir` (in-memory only);
>     mono failure in Check goes to collection channel, in MultiFile maintains `fail_with`;
>     `into_outcome` first `take_check_diagnostics` then determine fault (structural fault no longer
>     clears collected diagnostics). **Verification**: four rounds of independent verification
>     (t3/t4/t10 + t5 cross-form matrix) and one review (t6) all pass; corpus differential 340/340
>     line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps found
>     and fixed in same round**: F1 (rootless multi-file check's request name and IR definition key
>     from different sources → silent non-specialization, see WBS 3.4.8 row landing note) — fix is
>     aggregation side precise reverse lookup by actual definition key (not heuristic), first
>     version of fix is aggregation side precise reverse lookup by actual definition key
>     (`flat_defs`); **same-day errata**: this bridge was already deleted in the regression fix,
>     root cause now solved by `module_key` dual-role decoupling (`script_mode`), all three from the
>     same source `{unit key}.{name}`. The regression (CI docs example gate 258 blocks 10 fail → 0
>     fail) localization and fix see WBS 3.4.8 row landing note. **Record keeping (existing, not
>     this round regression)**: E3005 help text inconsistent with resource protection semantics;
>     multi-file E3005 span label misalignment (HEAD binary reproducible); request collection
>     surface type parameter position dependency (same surface goes to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (Execution-layer guard reachable)**: Interpretive
>     calls changed to **explicit stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` push frame then hand control back to outer `step_one`, frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, on return write back to caller slot, multi-call
>     instructions use frame-internal `pending_calls` to continue running. Baseline measurement per
>     layer of interpretive call consumes native stack **2544 B**, crash threshold at
>     `count_down(394)` (Windows main thread 1 MiB); after fix 20,000 layers of recursion can run on
>     512 KiB stack thread (each layer approximately 0). Guard (`push_frame`) is thus reachable at
>     any entry, any `max_stack_depth` value; `ExecutorError::StackOverflow` changed to carry real
>     upper limit (original `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (deep stack
>     backtrace truncation) per user ruling merged into this task** (same file, cannot be split by
>     file): backtrace changed to first 16 frames + count ellipsis + last 8 frames, shallow stack
>     frame-by-frame unchanged. **3.10.1 (Compile-time ungated native recursion, #447)**: Locate
>     first then change—crash point is `MonoType::type_name` recursion (**328 B/layer**, SP probe +
>     per-operation threshold determination), same path has derivative `Clone` (~400 B/layer),
>     `substitute` deep copy inside, `type_name_hash` (164 B/layer), `collect_generic_type_refs`;
>     after changing all four to work lists and removing per-layer entire request deep copy,
>     pathological generic recursion with `mono.max_depth=5000` produces **E3005 (with real upper
>     limit)**, no longer stack overflows. **3.11.1 (Task boundary error code fidelity, #448)**:
>     Task result error side changed from value to **structured**
>     (`TaskResult = Result<SyncValue, TaskFailure>`、`TaskOutcome::Err(TaskFailure)`),
>     cross-boundary restore `ExecutorError` by value; within-task E6001/E6003/ E6004/E6008 in
>     Standard mode and Embedded message character-by-character consistent. Two **pub API payload
>     form changes** marked with `BREAKING CHANGE` in the commit. **Commit Strategy Note**: 3.9.1
>     and 3.11.1 **are indivisible at file granularity**—`interpreter/executor/executor.rs`
>     simultaneously carries trampoline and error channel production/consumption points,
>     `TaskFailure` defined in `runtime/engine.rs`, only staging either side fails to compile
>     (pre-commit **staging subset** compilation, empirically two rejections), so per #15 same
>     caliber combined into **one atomic commit**. **Known boundaries and follow-up**: Drop derive
>     (~75 B/layer) ⇒ `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate is
>     carried on native recursion (each layer ~3,696 B, default 256 layers ≈ 924 KiB), currently
>     zero call sites, production unreachable, future wiring must close the same family; Standard
>     mode task interpreter constructs by default `ExecutorConfig` ⇒ its E6004 upper limit always
>     1024 (config propagation registered as WBS 3.12, user ruling B: separate team); test runtime
>     budget opens another issue #449 (pathological fixture 76s, clippy dual target, commit runs
>     full), not in this RFC construction surface. **Corpus directory layering (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split
>     into 23 subdirectories by category, 88 IR snapshots relocated per mirror path; corpus baseline
>     same commit regenerated (342 lines, independent recalculation line-by-line comparison: only
>     `file` field and 2 new additions changed), 41 snapshots regenerated due to file header path
>     lengthening, line-by-line check **only span offset changed** (zero difference after offset
>     removal normalization). **Ten entry改造 methods (function-by-function specification)**:

| Entry                                                      | After Transformation                                                                                                                                                                                                |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                      |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                             |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                       |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                         |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter results to target file                                                                                                                                                           |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                           |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfer to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfer to Driver)                                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; **hand-rolled lex→parse→… sequence deleted** in single-file branch, changed to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                       |

**Duplication between `check_project` and `compile_project` (approximately 40% shared / 60%
divergent)**. The shared parts: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, resolution loop, per-file checker assembly, result processing. The divergent
parts: items 2/3/4/5/7/8/9/10/11 of the 11 items in the table above. **This 40% shared skeleton is
precisely the value range of `Driver`**—the divergent 60% is all "different stage sets" or
"different aggregation modes", which is exactly what the two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causing this stage to skip" is not a stage return value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` itself, then the information about "why I didn't run" would be scattered across 12 stages
with no central audit. Change to `dispatch` determining at the loop start based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the vulnerability this
document is fixing—`pipeline.rs:187`'s current `if !proof_calls.is_empty()` is a silent "skip",
producing no diagnostic, just because the obligation happens to be empty. After the rule is
established, any "because X wasn't done, Y didn't run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` caused                                  | "Stage Y did not execute due to upstream stage X failure"                                                                     |
| Prerequisite obligation empty caused                     | "Stage Y did not execute due to no pending obligations (normal)"——**this item severity = Info, not counted in warning count** |
| Condition not met (e.g., `config.mono.enabled == false`) | "Stage Y did not execute due to configuration not enabled"                                                                    |

The second category is key: it lets "normal skip" and "abnormal skip" be distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                             | Current State Correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (four early returns at `pipeline.rs:150/162/174/193`)           | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; placing them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not just in "early return or
not"—they are two different checker entries. After unification, the same implementation should be
driven by the `Aggregation` parameter, not retaining two functions (**this is an additional
verification item needed by this document, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-layer Impact                                                                                                                              |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representations, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                                      |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                     |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type ownership and layering**: what migrates to the ledger is **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` still in `proof/`, the rest of the field types each stay
in place; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3
consumers (e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`**—this is consistent with the red line below about the
driver not having reverse dependencies.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. `Driver` appearing in `TypeChecker`'s imports is considered a violation,
intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                              |
| ----------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                     |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already接入 in P3 stopgap; **warning surface maintained without new additions** (decision B1, 2026-10-09: dead code family warnings don't enter compile path—pool semantics being fixed by 4.9 defect correction, normalizing warning surface left for after 4.9; this line's draft "new warning output" is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already接入 in P3 stopgap                                                                                                                                                                                                                                                                                                         |
| LSP (in-project)                    | No proof_execution                       | Already接入 in P3 stopgap                                                                                                                                                                                                                                                                                                         |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                        |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                            |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract `backend.rs:60` has already declared ("the caller should conservatively
degrade")—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single-file 1 / multi-file N)                                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changes to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changes to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slim down to Program constructor                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slim down to Program constructor                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changes to Driver path                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changes to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                                       |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s hand-rolled stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                             |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                            |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` change to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changes to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changes to produce diagnostic (**fix the second silent-drop point**)                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changes to **actual** execution order; delete "lower-layer failure skips upper layers" (no short-circuit exists)                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: Add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumption points in `predicate.rs`/`checker.rs`/`ownership.rs` change to use the shared port. Bare `&'static` reference form not feasible (`dyn Solver` is not `Sync`), original text governed by D58 |

**Deleted**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s hand-rolled lex/parse sequence.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct——**the problem is on the consumer side, not the producer side**, changing the
producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (wasm branch reachability determination goes to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                             | Disposition                                                                                                                                 |
| --------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Break**: previously silently-passing constraints will now report E4018  | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; staged release, expected diagnostic set changes go to CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Break**: programs that previously compiled cleanly now produce warnings | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                    |
| `assert_drained()` first run produces W-level diagnostics | **Break**: diagnostic set adds                                            | W first then E, S2/S4 in two steps                                                                                                          |
| `predicate.rs` panic changes to diagnostic                | **Improvement**: no longer crashes                                        | No break                                                                                                                                    |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                             | These two paths don't go through proof_execution                                                                                            |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactoring**                                                  | `pub` API surface needs to be synced if there's external dependency; in-repo consumers are all listed in the change list                    |

---

## Implementation Points

This document corresponds to RFC-039's global stage sequence **P3 (fix correctness holes)** and **P4
(stage contract and unified Driver)**. The following S1-S5 are the internal implementation order of
this document; **each step's acceptance criterion references the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration change: **diagnostic
sets** from each entry are the same + corpus behavior is the same).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failing first.

| Deliverable                                                                                                                                                     | Acceptance                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                   | **Must be red**. If unexpectedly green, this document's vulnerability analysis needs re-review |
| Multi-file corpus layer: create new `tests/yaoxiang-multifile/` (resolution D48——not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file two corpus versions' diagnostic sets can be compared                |
| `test_obligations_drained` skeleton                                                                                                                             | Mark `#[ignore]`, turns green in S4                                                            |

**Rollback point**: No code changes, purely adding tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code byte-for-byte identical**                    |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage intentionally does not fix any bug**—it only moves existing behavior into the Driver.
Acceptance criteria are C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored to original is fine.

### S3: Merge Four Orchestrator Entries + Fix Vulnerabilities

| Deliverable                                                                                                                    | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all change to Program constructors | Four entries' diagnostic sets normalized by `Aggregation` are the same            |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                      | `yaoxiang check` behavior consistent inside and outside project                   |
| LSP hand-rolled stage sequence deleted                                                                                         | LSP and CLI produce the same diagnostic set for the same file                     |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                  | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                        | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic |
| `predicate.rs:34-36` `.expect()` changes to diagnostic                                                                         | No longer panics when Z3 environment removed                                      |

**Rollback point**: Vulnerability fix and structural merge are **two separate commits**. If
structural merge has issues, only the structural commit can be rolled back, keeping the
vulnerability fix commit—at this time `test_multifile_proof_obligation_not_dropped` stays green;
reverse (keeping structure, rolling back fix) will turn red, an unacceptable intermediate state,
prohibited from merging.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                  |
| -------------------------------------------- | ----------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                      |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                    |
| Obligation diagnostic W → E upgrade          | After full corpus differential, manual review of each new E |

**Rollback point**: `assert_drained()`'s severity level can be controlled by config item, W/E switch
doesn't require code structure change.

### S5: Proof Layer and Wasm Wrap-up

| Deliverable                                                                                                                                                                                                           | Acceptance                                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                | Documentation and `checker.rs` call sites correspond one by one                                                                                                                    |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown doesn't enter cache**——otherwise one timeout is固化 across compilation via process-level cache, and pollutes cargo test thread-shared) | Production path `default_solver()` call sites reduced to zero (only tests retain); cache hit rate observable across three consumption points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                       | Has diagnostic when `body_checker` is absent                                                                                                                                       |
| `orchestrator.rs` 12 wasm attributes reachability determination                                                                                                                                                       | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination need                                                                                  |

**Note**: Correcting the layer order will change the diagnostic set, possibly exposing many
previously-silent `Unproven`. **This item should go independently from S1-S4**, not mixed in the
same PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                        | Reason                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustive enum, **rejects runtime registration**                                                                          | Exhaustive `match` is zero-cost compile-time forcing: when adding a new variant, `dispatch` fails to compile. This is isomorphic to the mechanism already verified by the opcode table in RFC-039 routing table B. `StageScope` further turns "per module or project level" into a type fact, eliminating the need for human reasoning about call counts                |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used together with `scripts/ci/check-obligations.py`'s **static** gate; severity W first then E   | The fix is for an entire class of bugs, not one bug: at least 2 already-verified similar hidden dangers (`proof_calls`, `checker.rs:1313`), all 16 span-keyed fields are within range. W first then E is to make each step's C2 criterion usable——jumping to E in one step would make `yaoxiang check` suddenly produce a large number of previously-silent diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **don't touch** the six `Unproven` branch production-side logics at `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                  |

This design additionally eliminates two classes of cracks:

- **The crack between "comment promise" and "code behavior"**. `checker.rs:5165`'s "silent pass must
  not be revived" and `layers/README.md:3`'s "lower-layer failure skips upper layers" are both
  **comment-level contracts**——the former is fulfilled by `assert_drained()`, the latter is exposed
  by topology-driven `Skipped` diagnostics.
- **"One array forgot to append"**. Currently five functions each hand-write the call sequence;
  after unification there's only one stage table, and new stage漏接 will report error at compile
  time.

### Not Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**——7 of the 11 inconsistencies
  (two dead code analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, they're configuration questions of "whether
  to run some analysis"; it also can't express `Aggregation`, only moving the either-or of
  `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**——cancels compile-time
  exhaustiveness: when adding a new stage, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each hand-write the call" with "one array forgot to append", which is
  exactly the cause of the current 11 inconsistencies. This repo already has a same-form
  counterexample: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described by `docs/src/dev/design/check/` are all zero-implementation.
- **Only fix bugs without touching structure**——can fix the known 2 bugs, but only covers 1 of the
  16 obligation fields, `Stage` still scattered across 5 functions, the next divergence point will
  continue to grow from here. **It must be done first** (is part of S1/S3), because
  structural改造 needs a known-red test to prove the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`**——`check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is finding the bug at the wrong level:
  the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs` slim down will make it harder to read in the short term**. After 139 lines of
  `compile_project` are split into "Program constructor + several driver arms", readers need to
  cross two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent problems"**. After the fix, there may be a batch of "new error" user
  reports——they are real existing bugs, just never reported. Must be clearly explained in CHANGELOG.
- **`assert_drained()`'s severity level switch needs per-field manual judgment**. Of the 16 fields,
  some (e.g., `module_namespaces`) having "no consumer" may be designed to not need consumption,
  should not alarm. S4 needs to go through each field one by one, cannot do a one-size-fits-all.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable in the long
  term, `stages()` will degrade to "pass different arrays each time" free parameter, contract
  constraint fails. **Mitigation is `test_program_stage_coverage` asserting `stages()` can only come
  from 6 predefined combinations**.
- **Criterion dependency**: S1/S3's vulnerability fix criterion depends on the
  [equivalence criteria document](07-equivalence-oracle.md)'s vulnerability test being red first.
  **If that document doesn't first establish the multi-file corpus layer, S1 cannot pass**——because
  the existing 293 corpora all go through the single-file path, zero coverage for this class of
  defect. This document doesn't involve IR verifier (`verify_loose`), that part of prerequisite work
  is not in this document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order opposite to declaration). This is diagnostic
  set change risk, should go independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19`
  already states `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to a cross-compilation-unit shared singleton, need to confirm all access paths go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order dependency defects** (such as termination and ownership's actual
  order dependency). Should be opened after the equivalence criteria are stable.

> **All open questions originally listed in this section have been ruled on.** Per-item decisions
> see [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction conventions,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` transformation (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Phase 2.5 proof function execution design source
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "silent pass must not
  be revived"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower-layer failure
  skips upper layers" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of playground (single-file path, thus executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — Error code build-time gate, example of this project's enforcement mechanism
