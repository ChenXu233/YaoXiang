# Compiler Stage Contracts and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039: Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage order are in the RFC-039 body;
> the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which didn't, and why"** is currently distributed
across six paths and ten entry functions, each manually wired, with no single place that can answer
this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by architecture rather than logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally reads "Unproven → compile
error, no degradation, no silent pass...... silent pass must not be resurrected." But this invariant
only holds on the **single-file path**: proof function constraints of the form `x: Sorted(3)`
silently pass through on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths, the proof
function is never executed, and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-entry transformation plan for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-regression gates → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA-ization →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order** in which to do it; this document
gives L1's **specific form**, **per-file change list**, and **this document's internal
implementation stages** (corresponding to P3 "Fix correctness vulnerabilities" and P4 "Stage
contracts and unified Driver" in RFC-039's global sequence). Where this document conflicts with
RFC-039, RFC-039 prevails.

Equivalence criteria are executed per the [Equivalence Oracle document](07-equivalence-oracle.md)
**C2 (orchestration changes)** category: the diagnostic sets of each entry are identical + corpus
behavior is identical.

## Current State

> This section consists entirely of **verified facts**, each with file path + line numbers. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries are the Only "Write It Wrong Once, Whole Repo Silent" Structural Defect

It differs in nature from the other two types of defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation             | Is There a Signal?   |
| ----------------------- | --------------------------------- | -------------------- |
| Lexical/syntactic error | Source code written wrong         | Has diagnostic       |
| Type mismatch           | Type written wrong                | Has diagnostic       |
| Stage skipped           | Some stage is never called at all | **No signal at all** |
| Obligation not consumed | Field filled in but never read    | **No signal at all** |

The common feature of the latter two: **failure produces no error**. Therefore they cannot be solved
by "writing code more carefully" or "reviewing more strictly" — code review can only see what was
written, not what wasn't written. This is exactly the root cause diagnosed in RFC-039: "This project
treats 'design' as a documentation convention rather than an executable constraint."

### Existing Enforcement Mechanisms in This Repo

The same repository already has mature enforcement mechanisms; they simply haven't been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subjected to a
  **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, comparing each entry
  against the RFC-013 code table, with any inconsistency directly `panic!`-ing to refuse
  compilation.
- `src/package/` (**76 files / 13,012 lines**, of which the `tests/` subdirectory accounts for 6,227
  lines / 47.9%) has high test density, and each module's test subtree is declared wired — this is
  the most complete test wiring in the repo, and can serve as a formal reference for stage-layer
  gates.

**The design capability is sufficient. What's missing is "putting an equivalent-level gate at the
orchestration layer too."**

(Line count standard: `(Get-Content).Count`, see the "Line count standard" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                              |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ---------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5-stage direct call                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                               |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + link            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics   |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)             |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + standalone IR        |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → Path 1              |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (Path 3a)                  |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                  |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → Path 1       |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Point    | Implementation                                                |
| ------------------- | ------------- | ------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                      |
| parsing             | `161`         | `run_parsing`                                                 |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                   |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5         |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below gives evidence cell by cell. **A blank does not mean "this entry doesn't do this
thing", but "there is no code in this entry doing this thing"** — which is the essence of the
defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                         | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | -------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                      | **yes** `187-203`                                | **none**                                                  | **none**                                                     | **none**                           | **none**                             |
| 2   | Dead code analysis                                       | yes `275-278` (`config.dead_code.enabled` gated) | **none**                                                  | yes `356-377` (role-aware, no config gating)                 | **none**                           | **none**                             |
| 3   | W1006 local module shadowing                             | **none**                                         | **none**                                                  | yes `336-348`                                                | **none**                           | **none**                             |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)      | **collected but never output**                            | yes `350`                                                    | **none**                           | **none**                             |
| 5   | W1001/W1002 dead code family                             | yes (same as 2)                                  | **none**                                                  | yes (same as 2)                                              | **none**                           | **none**                             |
| 6   | **Monomorphization**                                     | yes `381-389` (`config.mono.enabled` gated)      | **none**                                                  | not applicable                                               | not applicable                     | **none**                             |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | File discovery                                           | not applicable                                   | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | not applicable                       |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **none**                                         | **none**                                                  | yes (`321-328`)                                              | **none**                           | **none**                             |
| 10  | Global slot allocation                                   | not applicable                                   | yes `allocate_global_slots` (`147`)                       | **none**                                                     | **none**                           | **none**                             |
| 11  | IR generation + qualified name rewrite + linking         | yes (`205`)                                      | yes (`152-236`)                                           | **none**                                                     | **none**                           | yes (standalone ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, retained unchanged. Current state differences after P3 lands:
>
> - **Row 1 fixed**: proof_execution's five entries share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + orchestrator four entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 unchanged** (multi-file `run` still doesn't output W1003, doesn't run dead code
>   family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single-file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unverified defect).
> - **Row 7** check_module / check_module_collect_all dual entry → assigned to 4.2.7 (Aggregation
>   parameter-driven).
> - Code-side referenced "#434 ruling" was previously unrecorded in docs — now filed as RFC-039
>   **D57**.
> - **Embedded std incorporation asymmetry (new off-table fact, filed 2026-10-09)**: the single-file
>   path unconditionally injects std.list via `merge_embedded_std_ir` (required for for-loop
>   desugaring, #117 hard switch); the multi-file `discover` previously only recognized explicit
>   `use` — in-project for-loops compile successfully, runtime E6006 (empirical probe). Fixed and
>   closed WBS 4.10.2; same-family "multi-file lacks single-file step" parse hard-abort quirk filed
>   as WBS 4.10.1 — **also fixed** (landed immediately after 4.2.2, Check path degraded to per-file
>   collection; this audit method's "stage coverage / field consumption" dimension makes this a
>   compilation unit membership difference, an overlooked dimension).
>
> - **Embedded std registration surface coverage (4.2.5 construction discovery, fixed)**: the
>   Registry arm repeats registration harvest on embedded std units via `extract_module_info`, which
>   clobbers the native half-face already merged-registered by `with_std()` (loss of `result.is_err`
>   etc. → std.test false E1043). Third overlooked dimension of this audit: "merge semantics of two
>   registration sources for the same module key" — the single-file path's registration table is
>   formed only once via `with_std()`, while multi-file/Check's Registry arm does per-unit insert to
>   achieve coverage.
>
> - **multifile `use` parse dropped by hand-built path (2026-10-10 filed)**: `run_diagnostics`
>   projects-internal parse 错误改报真实诊断序列删除消失——真实诊断带精确 span，无信息损失）.

**Two places need precise expression, otherwise the writing will be wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function **has no `result.warnings` read point anywhere**, and
  the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion for the entry also comes from a different source**:
  `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
  `is_bin_role` (`250-252`, only checks "is there a manifest"). The two functions give different
  answers for "what file must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. The following eight steps are all verifiable.**

**Step 1 — Obligation is produced at exactly one place.**
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

The semantics is: "The predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed to judge."

**Step 2 — Consumption points are inside the checker, but they only emit, never consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **No diagnostic emitted** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |

The comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ......
> `This branch therefore becomes a real front line rather than forward-looking cushion: silent pass must not be resurrected.`

That is: the author knew "producing no diagnostic" is a defect, and explicitly categorized it as a
**known, homeless intermediate state**, with the design assuming "someone will definitely come to
read `proof_calls`".

**Step 3 — The field does get filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects, `checker.rs:1439` writes it as
`proof_calls, // Phase 2.5 pre-registered proof function obligation` into `TypeCheckResult`. The
field is defined at `types.rs:29`.

**Step 4 — There is only one read point in the entire repo.** `TypeCheckResult.proof_calls`
(`types.rs:29`) in production code is **read only at `src/frontend/pipeline.rs:187`**, passed as
argument at `189`:

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the whole repo fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is the same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — The orchestrator's four entries all bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
in `src/frontend/module/orchestrator.rs` **all do not call `pipeline.rs`**, instead directly calling
`TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they cannot even reach that one read
point at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also go through the drop
path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std modules
themselves are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP three paths **compiles, runs,
and has no diagnostics**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent drop point within the layer.**
`checker.rs:1303-1314` when handling ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic, no
bookkeeping.** This directly contradicts the "silent pass must not be resurrected" promise at the
`5164` branch, and is **independent of the orchestrator issue** — even if the entry layer is
completely fixed, this one will still be silent. (Processing timing: should be handled in the same
batch as the obligations ledger, because it's the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call
exists in `result.proof_calls`. But they **all go via `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping just before `pipeline.rs`**.

The test file's own doc comment already states this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is the shape of the problem: the test verifies "the obligation is filled in", and the bug is
"the consumer didn't read it".** A test that only tests the producer and not the consumer is
naturally immune to this class of defect.

### Stronger Finding: The Equivalence Oracle's Primary Corpus is the Single-File Path

The [Equivalence Oracle document](07-equivalence-oracle.md) takes the end-to-end diffing of 293
`.yx` corpora in `tests/yaoxiang/` as the **third-tier criterion**, as the primary acceptance means
for the C2 stage. But empirically:

- There is **no `yaoxiang.toml`** anywhere in the `tests/` directory (zero glob hits in the entire
  directory).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, all
  do not cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical proof of
this. The file is annotated `// expect: compile-error E4018` (line 15), and the comment in its
header at line 7 reads "Status: ❌ should be rejected at compile time" — **it passes precisely
because it takes the only path that executes the proof function**.

**Conclusion: The third-tier criterion needs a multi-file corpus layer added, otherwise it cannot
serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 16 "Production = Contract" Fields, Zero Mechanism Guarantee

In the `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields**
whose doc comments explicitly say "produced by stage X → consumed by stage Y", i.e., they are
essentially **cross-stage obligations**:

| Field                      | Defined at | Producer → Consumer                                             |
| -------------------------- | ---------- | --------------------------------------------------------------- |
| `proof_calls`              | `29`       | typecheck → **proof_execution**                                 |
| `release_plan`             | `31`       | ownership → IR generation (`ir_gen.rs:341` reads, `:1942` uses) |
| `escaped_refs`             | `33`       | ownership → IR generation                                       |
| `instantiation_requests`   | `35`       | typecheck → monomorphization (`pipeline.rs:381`)                |
| `existential_coercions`    | `37`       | typecheck → IR generation                                       |
| `implementation_proofs`    | `39`       | typecheck → IR generation                                       |
| `interface_impl_registry`  | `42`       | typecheck → operator query / constraint solving / LSP           |
| `sum_types`                | `44`       | typecheck → IR generation                                       |
| `sum_type_param_names`     | `47`       | typecheck → IR generation                                       |
| `variant_ctor_calls`       | `49`       | typecheck → IR generation (span-keyed)                          |
| `operator_dispatches`      | `51`       | typecheck → IR generation (span-keyed)                          |
| `method_overload_ir_names` | `53`       | typecheck → IR generation                                       |
| `overload_resolutions`     | `56`       | typecheck → IR generation (span-keyed)                          |
| `try_expr_impls`           | `59`       | typecheck → IR generation (span-keyed)                          |
| `match_scrutinee_types`    | `63`       | typecheck → IR generation (span-keyed)                          |
| `module_namespaces`        | `66`       | typecheck → IR generation                                       |

**All these fields are passed across layers keyed by `Span` or name tables.** The Equivalence Oracle
document has already pointed out that span-keyed contract mismatches will "silently fail with no
error at all" (`ReleasePlan` is typical: any inconsistency in how the two sides compute spans →
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer at `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`) is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the consumer
  is in a downstream module, and that downstream module is on the necessary path of every entry.
- `proof_calls`'s consumer is at the **top level** of `pipeline.rs`, which is **another entry's
  implementation**. The orchestrator's four entries don't go through `pipeline.rs` at all.

**Verified fact**: Currently the entire repo **has no mechanism** to ensure these 16 fields are
consumed. The only "protection" is that `ReleasePlan` happens to catch a ride on IR generation.

### Four Additional Contract Defects in the Proof Layer

The following all come from proof layer analysis, are independent of the entry-fork issue, but all
belong to the category of "stage contract not enforced".

**(a) The layer order declaration contradicts the actual execution order, and the `equivalence`
layer isn't in the pipeline at all.**

`src/frontend/core/typecheck/layers/README.md:5-11` declares the layer order:

| Layer | File             | Depends on    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "Execute in layer order, upper layer does not run if lower layer fails". Actual
call points in `TypeChecker::check_module`:

| Actual Order | Call Point                                        | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: `termination` and `ownership`'s order is **reversed** from the declaration; the declared
Layer 0 `equivalence` **doesn't appear in `check_module`'s stage sequence at all** (it's only used
as an `is_subtype` utility function at `inference/assignment.rs:17`, unrelated to `ProofResult`).
Meanwhile, **no short-circuit exists** — `checker.rs:1271-1276` adds termination errors one by one
via `add_error` and continues downward, so README's promise of "upper layer doesn't run if lower
layer fails" doesn't hold.

**(b) The SMT backend has three acquisition strategies, two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (SOLVER slot
> Option-ized, missing falls back to `SMTResult::Unknown` conservatively); "silently skipped (not
> injected)" gets a W1081 signal from 3.4.2. Unification of the three forms (singleton-ization) is
> executed per **RFC-039 D58**, real fix location `proof/smt/backend.rs` (new process-level shared
> singleton + `with_shared_solver` closure port), the 02 change list's `default_solver() → &'static`
> statement follows D58 (bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumption Point                                      | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction via `with_solver_owned`         | `None` → **silently skip** (not injected)                 |
| `ownership.rs:627` (back-edge cut decision)            | **Per call** `default_solver()`                          | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chose hard fail: "Initialization failure preserves
**hard fail**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". And `backend.rs:60`'s contract documentation says "`None`: backend
unavailable......**caller should degrade conservatively**". **Two philosophies coexist in the same
code, and the hard-fail one panics instead of returning an error.**

**(c) Each back-edge decision creates a new Z3 context, cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut decisions. And
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
a new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(For comparison: `predicate.rs:34-36`'s `LazyLock` is genuinely a singleton. Same backend, two
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
table** and runs as normal — ownership analysis degrades to "nothing conflicts", **without any
warning**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least leave
a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type check, before constraint
solving". Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at `1256`,
ownership at `1296`, i.e., the "before" in the comment is actually "after two layers"**.

### wasm Current State: shim crate carries, 27-file branch is alive

The wasm target **is built and built in CI**; the `cdylib` is in a separate shim crate, not the main
crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                        |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate only `rlib`                            | `Cargo.toml:29-31`                                                                                                                                                                                                                                                              |
| **shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                              |
| shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                     |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                 |
| shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                        |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from fixed URL, degrade to warning if missing)                                                                                                                                                                |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) only mention it in docs                                                                                                                                        |
| `orchestrator.rs` 20 occurrences                  | 12 attribute + 8 comment                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | All attribute (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs in the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target — `lib.rs:139/153/170` gates `run_file` / `run_project` / `build_bytecode` entirely
(these three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: `wasm/src/lib.rs:48`'s playground
entry calls `compiler.compile_with_source(...)` — **the single-file path**. Therefore:

| Path consuming `proof_calls`               | Does it execute proof functions? |
| ------------------------------------------ | -------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **yes**                          |
| wasm playground (`wasm/src/lib.rs:48`)     | **yes**                          |
| `build_bytecode` (`lib.rs:171`)            | **yes**                          |
| Multi-file `run` (`compile_project`)       | **no**                           |
| `check` (`check_project`)                  | **no**                           |
| LSP in-project (`check_source_in_project`) | **no**                           |

That is: **The existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — goes to `06-cleanup-inventory.md`. The open question in RFC-039 of "build wasm
target, or delete these branches" **has expired**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer" — this is the mechanism that the opcode table has already proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexical + syntactic               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type check (with embedded proof layer)   PerModule
    DeadCodeAnalysis,   // Dead code family analysis              Project
    ProofExecution,     // Proof function compile-time execution         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When adding new variants, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at the project level) make "which stages must run per-module and which must run once at project
level" a type-level fact. The `Project`-scoped stages are structurally guaranteed to run only once
in `dispatch` — this eliminates questions like the current `orchestrator.rs` one of "should
`allocate_global_slots` be called per file or not" that require human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the draft corrected per
> implementation evidence —
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), the draft order contradicts the data
>    flow.
> 2. The Check form's stage table is normalized to dead code **before** proof per `Stage::ALL`
>    topological order: `check_project`'s current state is proof before dead code, the two have no
>    data dependency, the diagnostic set is identical (C2 set semantics), only the in-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to the ALL order, byte-by-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    draft order would force the Registry arm to "hide parse", making the stage table lie about data
>    flow; and multi-file parse therefore drops from 2 to 1 time. An honest record of the current
>    state follows: the multi-file path's parse error was previously a hard-abort
>    (`build_registry_from`'s `?` propagation, the tension with CollectAll semantics filed as WBS
>    4.10.1) — **4.10.1 is fixed** (2026-10-09 user ruling on rust-style collection semantics + Plan
>    B): Check path parse failure degrades to per-file diagnostic collection, diseased files exit
>    the compilation unit (don't enter registry, importer reports E5001); MultiFile path preserves
>    hard-abort (a bad file under FailFast cannot produce IR, which is correct semantics, the pin
>    board is long-term valid).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two records: a. Data dependency edges
>    14→16: `RoleClassification` consumes Discovery's used_by edge set, `DeadCodeAnalysis` consumes
>    Discovery's W1006 shadowing events — the draft omitted these (these two products in the
>    orchestrator era were produced by `discover_with_used`, discarded by the `discover` wrapper,
>    the data flow was lost when it didn't enter the table); b. In-file diagnostic order
>    normalization extends to the full form of note #2 above: E3020 entry validation moves into the
>    RoleClassification arm (Check has no Linking stage, the arm holding surfaces and the AST is
>    responsible for validation), in-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); diagnostic set unchanged (C2 set semantics), only
>    the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: Make "field produced but not consumed" go from undetectable to a fact assertable at
compile time or test time. RFC-039 has already listed this as "the single most valuable item".

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
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                | Criterion                                                    | Disposition                                                                            |
| ---------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| Obligation read by its declared consumer | That field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                                   |
| Obligation non-empty but no consumer     | Field is non-empty and not consumed                          | **Produce diagnostic** (`W` level by default; in `strict` mode, escalate to `E` level) |
| Obligation is empty                      | Field is empty                                               | Pass (no need to consume)                                                              |

**Why W-level first, not E-level**: Fixing obligation drops will **change the diagnostic set**. The
C2 criterion requires "the diagnostic sets of each entry are identical"; if we jump to E-level in
one step, `yaoxiang check` will suddenly get a lot of `Unproven` diagnostics that were previously
silent. Splitting into two steps (observe W first, then escalate to E) makes the criterion usable at
each step. See "Implementation Points" S4.

**`assert_drained()` is called at exactly one place**: `Driver::run`, at the tail of the stage
table, before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third file reads it → fail). `assert_drained()` is the **runtime** gate, that script is
the **static** gate; the two complement each other and both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project（lib.rs:154）
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP in-project, results filtered to target file）
    Embedded,          // compile_embedded_module（embedded std）
    WasmPlayground,    // wasm/src/lib.rs:48 —— fourth caller of single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of the 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); it does
not accept callers freely passing an array**. This is the key constraint preventing the `Program`
abstraction from degrading into "free parameters that accept anything", asserted by
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
                // … 12 arms, not one can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, recorded for future reference —
>
> 1. The `StageOutcome` three-state in §4 is not returned by stage arms, but is expressed by `State`
>    failure flag + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, eliminating
>    the per-arm boilerplate return);
> 2. `Driver` has no `config` field — the sole source of configuration is `Program.config` (§5
>    "Avoid Driver holding mutable global state" ruling overrides the sketch field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded into the internal ledger
>    `DriverOutcome.skipped`, not emitted — S2 "Deliberately fix no bugs" zero-diff criterion
>    requires; emission follows in 4.3 obligations ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is allowed direction); orchestrator's existing call sites move away in 4.2 and
>    the question is revisited later.
> 5. In multi-file form, ProofExecution is an independent stage, executed after **all** typechecking
>    (A1, 2026-10-09 user ruling): `compile_project` originally interspersed proof in the per-file
>    typecheck loop (file N's proof before file N+1's typecheck). Single failure both byte-by-byte
>    identical; "file 1 proof fails + file 2 typecheck fails" multi-failure scenario, the first
>    report changes from proof error to typecheck error (test pin board).
> 6. `DriverOutcome` carries products by channel per ProgramKind: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each found file has an entry, clean files have empty Vec — `check_project` current contract);
>    Discovery product extends to "file set + used_by edge set + shadowing events" triple into State
>    (MultiFile form has no downstream consumer, only records without emitting); check path per-file
>    parse drops from 2 to 1 time with the C3 topology.
> 8. Lsp form landing (4.2.3, split by file source ruling, 2026-10-09 user decision): Disk file
>    parse failure same as Check's Plan B (collect + exit compilation unit — both ends degrade to
>    convergence); edited buffer parse failure retains residual AST to continue typecheck (editor
>    philosophy — typing intermediate state shouldn't make semantic features disappear). Discovery
>    arm keeps buffer source code overwriting stale disk content for Lsp; Typecheck in Lsp only
>    checks target file (other units only provide signatures for registry); in-file diagnostic order
>    normalization principle extends to LSP (typecheck diagnostic → W-code warning → proof error).
>    The old behavior where LSP hard-aborts on unrelated disk file parse error, handler silently
>    falling back to single-file path quirk is fixed in this step.
> 9. Embedded form landing (4.2.4): `Program` adds new `shared_registry` field — the embedded std
>    module is a sub-compilation, the registry is the **input** rather than product (EMBEDDED_
>    STAGES has no Registry stage), #94's SymbolTable sharing contract is thereby made explicit from
>    "caller remembers to pass the same registry" to program declaration. The error path text is
>    normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only
>    compiler internal error surface, no test pinning). Thus all four orchestrator entries migrate
>    into Driver.
> 10. standalone check unification (4.2.5, Ruling A + IR stage ruling, 2026-10-09): Check form is
>     the faithful carrier of single-file semantics for program without project root — warning
>     surface only covers entry file (neighboring files only collect errors, better to miss than
>     mis-attribute); relative `use` resolves along importer's directory (rustc single-file mod
>     alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path
>     (pipeline full chain) already runs IR generation, E3019/E1014/E1015 etc. only produced in
>     ir_gen (runner gate empirical). IrGeneration's Check form is pure check, not consuming IR;
>     Script/Bin variants expressed by module_key's None/Some (existing switch for E3023,
>     ir_gen.rs:1660). Monomorphization not in CHECK (only produces resource limit/internal error,
>     zero corpus dependency). **〔2026-10-10 withdrawn via 3.4.8 empirical verification〕** — "zero
>     corpus dependency" turns out to be evidence blind spot that the corpus has no pathological
>     recursive fixture; E3005 is the only compile-time defense for this kind of program. See note
>     #14. Side fix for latent defect: Registry arm's repeated registration harvest on embedded std
>     units would clobber the native half-face already merged by with_std() (E1043 false positive) —
>     embedded units now skip repeated registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, changed to
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains residual AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; doesn't mark stage failure,
>     otherwise topology skip would make typecheck never run); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path now completes proof (E4018),
>     W-code warnings (W1001–W1005, test pin board) and IR-level errors; lexical failure now reports
>     real diagnostics (old synthesized "E0001 lexical error" text disappears with the sequence
>     deletion — real diagnostics have precise spans, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries had already converged individually — mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, difference carried by
>     whether collected_errors is empty; statements.rs five collection points (function
>     body/use/for/block/while). Verification confirmed and fixed duplicate diagnostic defect:
>     collection point puts first error into both collected_errors and Err return channel, pass-3's
>     add_error on Err causes same code same span same message ×2 (nested can be ×3); plus
>     annotation validation signature parameters and overall annotation double-visit (E1003/E1103),
>     ownership layer same point double-emit (E2014/E2018) two independent mechanisms. Fix landed at
>     module result boundary: deduplicate by (code, span, message) — diagnostic is positional fact,
>     duplicate carries no information; 63 corpus entries have duplicate copies removed, run column
>     and exit code zero drift (`borrow_conflict_err`'s "duplicate" is baseline triple format not
>     including column number artifact — the two E2018 are actually legal diagnostics with different
>     columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver — `Compiler` wrapper layer no longer
>     involved. Error text byte-by-byte aligned with old `CompileError` Display prefix (Parse
>     error:/Type error:/Internal error:), playground UI zero visible difference. WasmPlayground and
>     SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, now has a real
>     producer from this step on).
> 14. Multi-file monomorphization and CHECK stage table fix (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirical refutation of "mono only
>     produces resource limit noise": pathological generic recursion (`f(x)=f([x])`) under
>     multi-file path without mono compiler process stack overflows (0xc00000fd — interpreter
>     recurses in Rust layer, no graceful runtime error), mono depth gate is the only compile-time
>     defense; standalone check before 4.2.5 takes pipeline full chain and has mono, the exclusion
>     ruling caused silent coverage regression ("zero corpus dependency" only proves corpus has no
>     pathological fixture). "Type erasure fallback" assumption invalidated by RFC-033 reflection
>     ruling — `^^List(Int)` needs real identity of instantiation, monomorphization is the
>     foundation of reflection rather than optional optimization. Four design points: ①typecheck for
>     qualified call (`lib.f(x)`, FieldAccess form) produces instantiation_request, generic_id uses
>     qualified name, with merged IR same namespace (currently only recognizes bare Var,
>     expressions.rs:2455); ②`Stage::ALL` topology changed to Linking before Monomorphization,
>     MULTI_FILE stage table tail adds Monomorphization — consumes merged_ir, aggregates all-unit
>     requests (containing_fn for deferred bucket qualification), mono upgraded from single-module
>     pass to whole-program pass; single-file stage table unchanged (no Linking, subsequence
>     property preserved, IR snapshot zero drift); ③CHECK adds Linking (pure merge — E3020 entry
>     validation stays in RoleClassification, prevents double report) and Monomorphization (pure
>     check, not consuming, same as IrGeneration's Check form) two stages; ④resource protection
>     follows whole-program BFS covering cross-unit mutual recursion. Monomorphizer itself unchanged
>     (consumes ModuleIR + request set contract happens to be merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design landing qualified call
>     instantiation collection: `ExpressionInferrer` injects module qualified key table (`use` three
>     arms registered — normal / alias / per-item alias and only SubModule kind export),
>     `callee_generic_name` goes through `SymbolTable::qualify` to splice the qualified name with
>     merged IR same source; qualified call first path arity criterion tightened (when parameter
>     table can't be solved, fall to second path to take real arguments by signature, avoiding
>     cross-module same-name function mismatch false E3018). During implementation empirically found
>     three asymmetric defects in mono side and fixed in the same round: (1) Delete key (generic
>     name set) and rewrite key (containing_fn + name + span triple) granularity mismatch — sites
>     that can't solve real arguments degrade to symbol requests entering deferred bucket, when
>     container function is non-generic bucket never drains, but original is already deleted by name
>     → dangling call (`list_ops.yx:53` runtime E6006 empirical); fix is to restore gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the single source
>     of fact to determine coverage, original at uncovered sites is preserved. (2) Restore section
>     `HashSet` iteration order causes function table order uncertainty (t2 review F1) → change to
>     `Vec` and sort by name, output byte-level reproducible. (3) Non-rewrite surface forms TailCall
>     / MakeClosure originally look up mapping key (comment claims conservative) →
>     `call_form_is_rewritten` single-point determination, non-rewrite form uniformly counted as
>     uncovered. All three are paired with pin tests (red state empirical discriminant power before
>     fix). Corpus diff baseline regenerated in same commit with exactly one line (remove_at frame
>     adds `(int64)`). **Commit strategy note**: typecheck collection and mono fix split into two
>     commits, only containing the former temporary tree triggers `stdlib_docs` red on HEAD existing
>     defect (same-family `list.is_empty([])` E6006) — the two halves are each other's green
>     prerequisite, so merged into one atomic commit.
> 16. 2026-10-10 five rulings recorded (after R1 closure): (1) **D2 ruling A** — user module export
>     carries `generic_fn_type_params_snapshot()` with `Export.type_params`, isomorphic to std path
>     (`yx_sources.rs:62/108`): declared name becomes authoritative source of qualified call
>     monomorphization, signature back-inference degrades to fallback (merged into R2 scope). (2)
>     **HKT declaration side alive, call side broken** (inferred type argument E1002 / explicit type
>     argument E1010) filed as WBS 3.5 (3 three-level tasks). (3) **deferred bucket under
>     non-generic container never drains** merged into 3.4.8 ② acceptance sub-item: bucket must
>     drain or loudly report error, no silent non-specialization. (4) **closure variable by name
>     call runtime E6006** filed as WBS 3.6; semantics (i) support indirect call / (ii) compile-time
>     reject, **ruled (i)** — support indirect call: `CallDyn` mechanism is in place
>     (`ir_gen.rs:7945` closure expression path), the break is that `Var` callee is downgraded to
>     by-name `Call`, and typecheck already accepts this program. (5) **Trade-off criterion
>     solidified** — scheme veto reasons can only be based on correctness and readability, "large
>     change surface / new mechanism / high cost" does not constitute veto reason, lands in
>     `coding-rules.md` second part and HOWTO self-check list; trade-offs requiring user decision
>     follow `AGENTS.md` "When requesting user ruling (mandatory)" with all four (situation+evidence
>     / per-item pros and cons / explicit recommendation+cost / destructiveness naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm aggregates all-program requests and qualifies `containing_fn` **before taking each
>     unit's IR** (otherwise deferred bucket key and merged IR different source, bucket never
>     drains). **① closure (D2 ruling A)**: user module export carries declared type parameter
>     snapshot with `Export::type_params`, isomorphic to std path (`yx_sources.rs:62/108`).
>     **Existing defects empirically found and fixed during process**: (a) **F1** — `qualify_names`
>     only rewrites function body, `ir.init` is never visited, multi-file function definition is
>     qualified but module top-level call still bare name → runtime E6006 (**non-generic
>     reproduces**, not related to generics); fix is for init to reuse the same `rename`/`aliases`
>     and the same `rewrite_call_names`. (b) mono side "whole-program unique suffix" heuristic and
>     missing hit silent `continue` → delete heuristic, missing hit changes to **loud E3018**
>     (ambiguous silent mismatch is worse than not parsing). **Independent verification caught and
>     fixed regression**: t6 found named import `use lib.{pick}` after bare name call rejected by
>     false E3018 (IR instruction name is alias qualified name, request name is bare name,
>     aggregation side only qualifies by unit key) → fix is aggregation side **directly calling
>     ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as pub single point, one
>     place produces one place consumes), and matrix pin test covers named import two forms.
>     **Record**: init section `DefId` blank empirically confirmed as "consistency blank" rather
>     than live defect (codegen name fallback can resolve, four types of init calls empirically
>     pass), filed as WBS 3.7.4 to close "function body goes through def, init relies on name"
>     caliber fork.
> 18. init section DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operand + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; 208 IR
>     snapshots regenerated with only 309 pairs of `(def=_)`→`(def=dN)` changes (other-lines 0;
>     whole tree init section 0 residual `(def=_)`). codegen DefId main path and name fallback two
>     states produce identical translation and execution values (four types of init calls
>     empirically tested), corpus baseline zero drift; probe flips per new caliber and strengthens
>     (identity + precise index + execution value, fallback init coverage causes red). **Errata
>     (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase` — original
>     b554497b/e60dcc87/b7ac7396 correspond to rebase's 375789d2/d922a551/38bd4b9e in turn; the
>     pushed dev history takes the new number as standard (old numbers are local temporary numbers,
>     not in remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④'s design basis see #14; ②'s
>     landing see #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), entry validation whole block stays in
>     `RoleClassification`; `ir_generation_check` generates and retains `us.ir` (in memory only);
>     mono failure in Check enters collection channel, in MultiFile keeps `fail_with`;
>     `into_outcome` first `take_check_diagnostics` then determines failure (structural failure no
>     longer empties already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-form matrix) and one review (t6) all pass; corpus diff
>     340/340 line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps
>     found and fixed in same round**: F1 (rootless multi-file check request name and IR definition
>     key different source → silently not specialized, see WBS 3.4.8 row landing note) — fix is
>     aggregation side precise reverse lookup by real definition key (non-heuristic), and file
>     `flat_defs` bridge to be deleted in 4.10.3. **Records (existing, not this round regression)**:
>     E3005 help text inconsistent with resource protection semantics; multi-file E3005 span label
>     misplacement (HEAD binary reproducible); request collection surface type parameter position
>     dependency (same surface goes to 3.5).

**Transformation method for the ten entries (function-by-function specified)**:

| Entry                                                      | After Transformation                                                                                                                                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                           |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already stateless wrapper)                                                                                                                                                                    |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                            |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                              |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filter to target file                                                                                                                                                 |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                   |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                          |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; single-file branch's **manual lex→parse→… sequence deleted**, changes to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                            |

**`check_project` and `compile_project` have about 40% shared / 60% fork**. The shared part: vendor
consistency check, file discovery, `build_registry_from`, `all_method_bindings`, parse loop,
per-file checker assembly, result processing. The fork part: items 2/3/4/5/7/8/9/10/11 in the table
above. **The 40% shared skeleton is exactly the value zone of `Driver`** — the forked 60% is all
"different stage set" or "different aggregation mode", and that happens to be fully expressible by
two fields of `Program`.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "Upstream failure caused this stage to skip" is not part of the stage's
return value.**

Reason: Skipping is **topology**-determined, not stage-determined. If we let each stage return
`Skipped` itself, then the "why didn't I run" information is scattered across 12 stages, unable to
be centrally audited. Instead, let `dispatch` judge by dependency at the start of the loop:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip", which
doesn't produce any diagnostic, just because the obligation happens to be empty. After the rule is
established, any "because X wasn't done, so Y didn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text Direction                                                                                                              |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                              | "Stage Y not executed due to upstream stage X failure"                                                                      |
| Caused by empty prerequisite obligation                 | "Stage Y not executed because no pending obligation (normal)" — **this item severity = Info, not counted in warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y not executed because configuration not enabled"                                                                    |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic flow, without polluting `yaoxiang check`'s `warning_count` (which
`diagnostic/mod.rs:637-641` depends on for the non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent invocations; placing them on Program can avoid Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a "early exit or not"
difference — they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, rather than retaining two functions (**this is an
item this document needs additional verification, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Doesn't introduce new type representations, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                             |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: What migrates into the ledger is **settlement
responsibility**, not the type's ownership module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
were; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3
consumers (e.g. `ir_gen` reading `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`** — this is consistent with the driver
not-reversely-dependable red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse-`use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is a violation, intercepted by
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                          |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                 |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already connected in P3 stop-gap; **warning surface remains not added** (Decision B1, 2026-10-09: dead code family warnings do not enter compile path — pool semantics being corrected by 4.9 defect, normalized warning surface deferred to after 4.9; this line's draft "add new warning output" is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                              |
| LSP (in-project)                    | No proof_execution                       | Already connected in P3 stop-gap                                                                                                                                                                                                                                                                                              |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                    |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                        |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract already declared by `backend.rs:60` ("caller should degrade conservatively") — the
current implementation is opposite to its own contract documentation.

### Compiler Change List

**New Files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single-file 1 / multi-file N)                                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified Files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                       |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                              |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **preserved**, moved into `driver` as implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to Driver                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or changed to be driven by `Aggregation` parameter                                                                                                                                                                                                        |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to Driver call                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                         |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                             |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to Path 1 via `Compiler::compile_with_source`)                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent drop point**)                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "upper layer doesn't run if lower layer fails" (no short-circuit exists)                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs`/`checker.rs`/`ownership.rs` three consumption points changed to shared port. Bare `&'static` reference form infeasible (`dyn Solver` is not `Sync`), original statement follows D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic itself is
correct — **the problem is on the consumer side, not the producer side**, changing the producer side
will mask architectural defects); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch
(reachability determination of wasm branches goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                          | Disposition                                                                                                                                           |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set change goes through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compiled successfully now produce warnings | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                                 |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set adds new items                            | W first, then E, S2/S4 in two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                     | No breaking                                                                                                                                           |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                          | These two paths don't go through proof_execution                                                                                                      |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactoring**                                               | If `pub` API surface has external dependencies, need to sync; in-repo consumers fully listed in change list                                           |

---

## Implementation Points

This document corresponds to RFC-039 global stage sequence's **P3 (Fix correctness
vulnerabilities)** and **P4 (Stage contracts and unified Driver)**. The following S1-S5 is this
document's internal implementation order; **each step's acceptance criterion references the C2
category of the [Equivalence Oracle document](07-equivalence-oracle.md)** (orchestration changes:
the **diagnostic set** of each entry is identical + corpus behavior is identical).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written failing first.

| Deliverable                                                                                                                                                     | Acceptance                                                                                           |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                   | **Must be red**. If accidentally green, this document's vulnerability analysis needs re-verification |
| Multi-file corpus layer: create new `tests/yaoxiang-multifile/` (Resolution D48 — not mixed into single-file corpus tree; project fixture with `yaoxiang.toml`) | Single-file and multi-file two versions of corpus diagnostic sets are comparable                     |
| `test_obligations_drained` skeleton                                                                                                                             | Marked `#[ignore]`, turn green in S4                                                                 |

**Rollback point**: No code change, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately fixes no bugs** — it only moves existing behavior into Driver. Acceptance
standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` single commit, `pipeline.rs` reverts to original.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entries diagnostic sets normalized by `Aggregation` are identical            |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behavior consistent in and out of project                        |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give identical diagnostic sets for the same file                      |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics when Z3 environment is missing                                   |

**Rollback point**: Vulnerability fix and structure merge **in two separate commits**. If structure
merge has problems, can only rollback the structure commit, keep vulnerability fix commit — at this
time `test_multifile_proof_obligation_not_dropped` stays green; reverse (keep structure, rollback
fix) will turn red, an unacceptable intermediate state, forbidden to merge.

### S4: Enable Obligations Ledger

| Deliverable                                  | Acceptance                                       |
| -------------------------------------------- | ------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green           |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                         |
| Obligation diagnostic W → E upgrade          | After full corpus diff, manual review each new E |

**Rollback point**: The severity of `assert_drained()` can be controlled by configuration item; W/E
switch doesn't require code structure change.

### S5: Proof Layer and wasm Cleanup

| Deliverable                                                                                                                                                                                                                 | Acceptance                                                                                                                                                                      |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                      | Documentation and `checker.rs` call points correspond one by one                                                                                                                |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache** — otherwise one timeout solidifies across compilations via process-level cache, and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call point goes to zero (only tests retain); cache hit rate observable across three consumption points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                             | No `body_checker` has diagnostic                                                                                                                                                |
| `orchestrator.rs` 12 wasm attributes reachability determination                                                                                                                                                             | Conclusion goes to `06-cleanup-inventory.md`, this document only records determination requirement                                                                              |

**Note**: Correcting the layer order will change the diagnostic set, may expose large amounts of
previously-silent `Unproven`. **This item should be separate from S1-S4**, not mixed with entry
merge in the same PR.

## Key Decisions and Rationale

| Decision                 | Decision                                                                                                                                                     | Rationale                                                                                                                                                                                                                                                                                                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage Model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **rejects runtime registration**                                                                       | Exhaustive `match` is zero-cost compile-time enforcement: when adding new variants, `dispatch` won't compile. This is isomorphic to the mechanism already validated by the opcode table in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the need to reason about call count                                 |
| **Obligation Mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, with `scripts/ci/check-obligations.py`'s **static** gate in parallel; severity W first, then E | What is being fixed is an entire class of bugs, not a single bug: current verified same-class latent issues at least 2 (`proof_calls`, `checker.rs:1313`), 16 span-keyed fields all in range. W first, then E, is to make the C2 criterion usable at each step — jumping to E in one step will make `yaoxiang check` suddenly have a lot of previously-silent diagnostics |
| **Fix Scope**            | Fix **consumer side** (orchestration layer), **don't touch** the six `Unproven` branch producer-side logic at `checker.rs:5164/5179/5306/5318/5420/5448`     | The problem is on the consumer side, not the producer side. Changing the producer side will mask the architectural defect as "logic is fixed", while the logic of these three branches itself is correct                                                                                                                                                                  |

This design additionally eliminates two classes of cracks:

- **The "comment promise" vs "code behavior" crack**. `checker.rs:5165`'s "silent pass must not be
  resurrected" and `layers/README.md:3`'s "upper layer doesn't run if lower layer fails" are both
  **comment-level contracts** — the former is fulfilled by `assert_drained()`, the latter is exposed
  by topology-driven `Skipped` diagnostic.
- **"One array forgot to append"**. Currently five functions each manually write the call sequence;
  after unification, it's the only stage table, and new stage miss-wiring will error at compile
  time.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of 11 inconsistencies
  (dead code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type connection**, are configuration problems of "should we
  run a certain analysis"; it also can't express `Aggregation`, only moves the `check_module` /
  `check_module_collect_all` either-or from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding new stages, `dispatch` no longer fails to compile, which is equivalent
  to changing "five functions each manually write the call" to "one array forgot to append", which
  is exactly the cause of the current 11 inconsistencies. This repo has the same-form
  counterexample: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs, don't touch structure** — can fix the known 2 bugs, but only covers 1 of the 16
  obligation fields, `Stage` is still scattered in 5 functions, the next fork point will continue to
  grow from here. **It must be done first** (part of S1/S3), because structure transformation needs
  a known red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` can't hold; `#[must_use]`
  only warns when the field is **entirely** dropped. This is looking for the bug at the wrong level:
  the bug is at the orchestration layer, detection must be at the orchestration layer.

## Known Limitations and Risks

- **The orchestrator.rs slimming will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is a common cost of all "concentrate the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously-silent problems"**. After the fix, there may be a batch of "new errors" user reports —
  they are real bugs, just never reported. Must be clearly stated in CHANGELOG.
- **The severity switch of `assert_drained()` needs manual judgment field by field**. Of the 16
  fields, some (e.g. `module_namespaces`) "no consumer" may be designed to not need to be consumed,
  and should not report warning. S4 needs to go through field by field, can't be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entry's stage set is unstable for a long time,
  `stages()` will degrade to "different array passed each call" free parameter, contract constraint
  fails. **Mitigation is `test_program_stage_coverage` asserting `stages()` can only come from 6
  predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criterion depends on
  [Equivalence Oracle document](07-equivalence-oracle.md)'s vulnerability test being red first. **If
  that document doesn't first establish multi-file corpus layer, S1 cannot be accepted** — because
  the existing 293 corpora all take single-file path, zero coverage of this class of defect. This
  document doesn't involve IR verifier (`verify_loose`), that part of prerequisite work is not in
  this document's scope.
- **Layer order correction will expose previously-silent `Unproven`** (`equivalence` not in pipeline
  at all, `termination` and `ownership` order opposite to declaration). This is a diagnostic set
  change risk, should go independently.
- **Thread safety after SMT backend changes to singleton is undetermined**. `backend.rs:13-19`
  already explains `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to a singleton shared across compilation units, need to confirm all access paths go
  through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask sequential dependency defects** (such as the actual sequential
  dependency between termination and ownership). Should be opened after equivalence criteria
  stabilize.

> **The open questions originally listed in this section are all ruled.** Per-item decisions see
> [RFC-039 Decision Log](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This document
> has no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing table A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` transformation (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Comparison code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit statement of "silent pass must not
  be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit statement of "upper layer doesn't run
  if lower layer fails" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for SMT
  initialization hard fail
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract statement of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that must be consumed
  downstream
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test self-admits
  "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — Corpus
  self-admits "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — Fourth compilation caller of playground (single-file path, hence executes
  proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — Error code build-time gate, an example of the project's enforcement mechanism
