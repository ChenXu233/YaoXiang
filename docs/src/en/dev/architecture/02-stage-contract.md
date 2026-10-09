# Compilation Stage Contracts and Obligation Ledger

> **Subsidiary Design Document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage ordering are in the RFC-039
> main text; the positioning of each subsidiary document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which did not, and why"** is currently distributed
across six paths and ten entry functions, each manually wired, with no single place able to answer
this question.

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are broken by the architecture, not the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` reads literally "Unproven → compilation
error, no degradation, no silent pass……must not let the silent pass come back to life." But this
invariant only holds on the **single-file path**: refinement constraints like `x: Sorted(3)` for
proof functions **silently pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths;
the proof functions never execute, and no diagnostics are produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-entry-function refactoring approach for all ten
  entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-rebound gating → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1–P10 global execution order and G1–G10 acceptance gates → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 gives the **why** for the refactor and **in what order** to do it; this document gives L1's
**specific form**, **per-file change list**, and **internal implementation stages** (corresponding
to RFC-039's global sequence P3 "Fix Correctness Vulnerabilities" and P4 "Stage Contracts and
Unified Driver"). Wherever this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are executed per the **C2 (orchestration changes)** category of the
[Equivalence Criteria document](07-equivalence-oracle.md): same diagnostic set at each entry + same
corpus behavior.

## Current State

> All facts in this section are **verified**, each with file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage Boundaries Are the Only "One Place Wrong, Whole Repo Silent" Structural Defect

It differs in nature from the other two defect classes (module boundaries, test wiring):

| Defect                | Typical Manifestation          | Has Signal?              |
| --------------------- | ------------------------------ | ------------------------ |
| Lexical/syntax error  | Source code written wrong      | Has diagnostic           |
| Type mismatch         | Type written wrong             | Has diagnostic           |
| Stage missed wiring   | Some stage never called at all | **No signal whatsoever** |
| Obligation unconsumed | Field filled but never read    | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"This project treats 'design' as a documentation convention, not as an executable constraint."

### Mandatory Mechanisms Already in the Repo

The same repo already has mature mandatory mechanisms — they just haven't been extended to the stage
layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are hard-gated at build time
  by `build.rs:19-55` via `tools/code-tables`, which compares each against the RFC-013 code table
  line by line; any inconsistency causes a direct `panic!` rejecting compilation.
- `src/package/` (**76 files / 13,012 lines**, of which `tests/` directory has 6,227 lines / 47.9%)
  has high test density; the test subtree of each module is declaratively wired — this is the most
  complete test wiring in the entire repo, and can serve as a form reference for stage-layer gating.

**Design capability is sufficient. The gap is "placing an equivalent gate in the orchestration layer
too."**

(Line count metric: `(Get-Content).Count`, see the "Line count metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                             |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | --------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5-stage direct call                           |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                              |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking        |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collects all diagnostics  |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)            |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR      |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1             |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                 |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project via 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                 |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1      |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean "this entry doesn't
do this thing", but rather "no code in this entry does this thing"** — this is the very nature of
the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                       | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                  | **No**                                                       | **No**                             | **No**                                 |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                  | Yes `356-377` (role-aware, no config gate)                   | **No**                             | **No**                                 |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                  | Yes `336-348`                                                | **No**                             | **No**                                 |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                          | Yes `350`                                                    | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                  | Yes (same as 2)                                              | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                  | N/A                                                          | N/A                                | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first-error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, drops `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | N/A                                    |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                  | Yes (`321-328`)                                              | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                     | **No**                                                       | **No**                             | **No**                                 |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                      | Yes (`152-236`)                                         | **No**                                                       | **No**                             | Yes (independent ModuleIR then merged) |

> **Review Note (WBS 3.4.3, cell-by-cell check on 2026-10-07)**: This table is a diagnostic snapshot
> of the `9e02e4db` baseline, preserved unchanged. Status differences after P3 lands:
>
> - **Row 1 fixed**: proof_execution is now shared by five entries via the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not output W1003, still does not
>   run the dead code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   unproven latent risk).
> - **Row 7** `check_module` / `check_module_collect_all` dual entry → assigned to 4.2.7
>   (Aggregation parameter-driven).
> - The "Ruling #434" referenced by code has zero prior docs registration — now registered as
>   RFC-039 **D57**.
> - **Embedded std asymmetry (new out-of-table fact, registered 2026-10-09)**: The single-file path
>   unconditionally injects `std.list` via `merge_embedded_std_ir` (required for for-loop
>   desugaring, #117 hard switch); multi-file `discover` previously only recognized explicit `use` —
>   in-project for loops compiled successfully but produced runtime E6006 (probe empirical). Fixed
>   and WBS 4.10.2 closed; the sibling "multi-file lacks single-file step" parse hard-stop quirk
>   registered as WBS 4.10.1 — **also fixed** (4.2.2 landed immediately after; Check path degraded
>   to per-file collection; this audit methodology examines "stage coverage/field consumption"
>   dimensions, this line is a compilation-unit membership difference, a dimension that slipped
>   through).
> - The `main` predicate at the entry in **Row 11** has different sources: `check_project:385-395`
>   uses `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role`
>   (`250-252`, only judges "manifest presence"). The two functions give different answers to "what
>   file must define `main`".

**Two places need precise wording, otherwise the writeup will be wrong:**

- **The precise conclusion for Rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but **this function has no `result.warnings` read site anywhere**; the
  sole downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **The `main` predicate at the entry in Row 11 is also not single-sourced**:
  `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
  `is_bin_role` (`250-252`, only judges "manifest presence"). The two functions give different
  answers to "what file must define `main`".

### Correctness Vulnerability: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are reproducible.**

**Step 1 — The obligation production site is unique.**
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

The semantics: "the predicate argument of this refinement constraint is a compile-time literal; this
proof function must actually be executed to decide."

**Step 2 — The consumption site is inside the checker, but only produces, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The comment at `checker.rs:5165-5169` literally reads:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch therefore becomes a true defense line rather than a forward-looking fallback: must not let the silent pass come back to life.`

That is: the author knowingly that "not producing diagnostics" is a defect, explicitly classified it
as **a known intermediate state with nowhere to go**, design-assuming "someone will definitely come
to read `proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects; `checker.rs:1439` writes into `TypeCheckResult` with
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — The entire repo has only one read site.** `TypeCheckResult.proof_calls` (`types.rs:29`)
is **read in only one place in production code: `src/frontend/pipeline.rs:187`** (parameter passed
at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other repo-wide hits on the `proof_calls` identifier fall into three categories, none of which is a
consumer of this field: `checker.rs:1234/4564/5494` are production-side collection; `verdict.rs:61`
is a same-name field on `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**, calling
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they cannot even reach the
sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations of the standard library itself also take the dropped
path.** `compile_embedded_module` (`1374`, `check_module` called at `1386`) is responsible for
compiling embedded std. This means **the proof obligations of embedded std modules themselves are
also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and emits no diagnostic** on the multi-file
/ check / LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — A second silent-drop point exists within the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership-check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic, no
bookkeeping.** This directly contradicts the `5164` branch's promise of "must not let the silent
pass come back to life", and is **independent of the orchestrator issue** — even if the entry layer
is fully fixed, this spot remains silent. (Handling timing: should be processed in the same batch as
the obligation ledger, as it is the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits on the keywords `Sorted` /
`proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty — e.g., `rfc027_return_refinement.rs:350-359` asserts a `SumUpTo` call exists in
`result.proof_calls`. But they **all go via `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping exactly
before `pipeline.rs`**.

The test files' own doc-comments make this explicit. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer`

**This is exactly the shape of the problem: tests verify "obligation is filled", but the bug is
"consumer didn't read".** A test that only tests the production side and not the consumer side is
naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Criteria's Primary Corpus Is the Single-File Path

The [Equivalence Criteria document](07-equivalence-oracle.md) takes the end-to-end diff of 293 `.yx`
corpora in `tests/yaoxiang/` as the **third-tier criterion**, and as the primary acceptance method
for C2 stages. But empirically:

- The `tests/` directory has **no `yaoxiang.toml` at all** (zero hits repo-wide on glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, all
  do not cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is empirical proof of
this. That file is annotated `// expect: compile-error E4018` (line 15); its header comment on line
7 says "Status: ❌ should be rejected at compile time" — **it passes precisely because it takes the
only path that executes the proof function**.

**Conclusion: The third-tier criterion needs a supplementary multi-file corpus layer, otherwise it
cannot serve as the acceptance tool for this document.** See Implementation Note S1.

### Obligation Fields: 16 "Produce-on-Contract" Fields, Zero Mechanism Guaranteeing It

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, **16 fields** have
doc-comments explicitly stating "produced by stage X → consumed by stage Y" — that is, they are
essentially **cross-stage obligations**:

| Field                      | Defined At | Producer → Consumer                                                  |
| -------------------------- | ---------- | -------------------------------------------------------------------- |
| `proof_calls`              | `29`       | typecheck → **proof_execution**                                      |
| `release_plan`             | `31`       | ownership → IR generation (read at `ir_gen.rs:341`, used at `:1942`) |
| `escaped_refs`             | `33`       | ownership → IR generation                                            |
| `instantiation_requests`   | `35`       | typecheck → monomorphization (`pipeline.rs:381`)                     |
| `existential_coercions`    | `37`       | typecheck → IR generation                                            |
| `implementation_proofs`    | `39`       | typecheck → IR generation                                            |
| `interface_impl_registry`  | `42`       | typecheck → operator query / constraint solver / LSP                 |
| `sum_types`                | `44`       | typecheck → IR generation                                            |
| `sum_type_param_names`     | `47`       | typecheck → IR generation                                            |
| `variant_ctor_calls`       | `49`       | typecheck → IR generation (span-keyed)                               |
| `operator_dispatches`      | `51`       | typecheck → IR generation (span-keyed)                               |
| `method_overload_ir_names` | `53`       | typecheck → IR generation                                            |
| `overload_resolutions`     | `56`       | typecheck → IR generation (span-keyed)                               |
| `try_expr_impls`           | `59`       | typecheck → IR generation (span-keyed)                               |
| `match_scrutinee_types`    | `63`       | typecheck → IR generation (span-keyed)                               |
| `module_namespaces`        | `66`       | typecheck → IR generation                                            |

**All these fields are passed across layers keyed by `Span` or name tables.** The Equivalence
Criteria document has already pointed out that span-keyed contract mismatches "silently fail with no
error at all" (`ReleasePlan` is typical: any inconsistency in span computation between the two sides
→ `Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
coincidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), is inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the
  consumer is in a downstream module, and the downstream module is on the unavoidable path of every
  entry.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry's implementation**. The four orchestrator entries don't even go through the `pipeline.rs`
  layer.

**Verified fact**: currently the entire repo has **no mechanism** guaranteeing that these 16 fields
are consumed. The only "protection" is that `ReleasePlan` happens to catch a ride on IR generation.

### Four Additional Contract Defects in the Proof Layer

All below come from proof-layer analysis, independent of the entry-fork problem, but all belong to
the class "stage contract not enforced".

**(a) Layer-order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

Layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 states "executed in layer order; lower-layer failure means upper-layer doesn't run".
Actual call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Declared Layer |
| ------------ | ------------------------------------------------- | -------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2        |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1        |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3        |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0        |

That is: the order of `termination` and `ownership` is **reversed** from the declaration; the
declared Layer 0 `equivalence` **does not appear at all in the stage sequence of `check_module`**
(it's only used by `inference/assignment.rs:17` as an `is_subtype` utility, unrelated to
`ProofResult`). And there is **no short-circuit** — `checker.rs:1271-1276` pushes termination errors
one by one via `add_error` and continues, so the README's promised "lower-layer failure means
upper-layer doesn't run" doesn't hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review Note (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (SOLVER slot
> Option-ified; absence conservatively degrades to `SMTResult::Unknown`); "silently skip (no
> injection)" got W1081 signal from 3.4.2. Unification of the three forms (singleton-ization) is per
> **RFC-039 D58**, with the real fix location at `proof/smt/backend.rs` (new process-level shared
> singleton + `with_shared_solver` closure port); the `default_solver() → &'static` wording in the
> 02 change list is governed by D58 (bare `&'static` reference is not feasible: `dyn Solver` is not
> `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Construction-time injection `with_solver_owned`          | `None` → **silently skip** (no injection)                 |
| `ownership.rs:627` (back-edge cut judgment)            | **Every call** `default_solver()`                        | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "initialization failure keeps
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'". Whereas `backend.rs:60`'s contract document says "`None`: backend
unavailable……**caller should conservatively degrade**". **Two philosophies coexist in the same
codebase, and the hard-fail one panics instead of returning an error.**

**(c) Each back-edge judgment creates a new Z3 context, rendering caching effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut judgment. And
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
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while `z3_backend.rs:17`'s
doc-comment claims "SMT query results are cached in `cache`". Not shared across calls ⇒ **this cache
never hits under this call pattern**.

(Contrast: `predicate.rs:34-36`'s `LazyLock` is indeed a singleton. Same backend, two different
lifecycle strategies.)

**(d) Checker silently degrades + comment doesn't match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check receives an **empty type ledger and empty call
table** and runs as usual — ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Suggested handling: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking, before constraint
solving". Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at `1256`,
ownership is at `1296`, i.e., the comment's "before" is actually "two layers after"**.

### wasm Current State: Carried by Shim Crate, 27-File Branches Are Live

The wasm target **has been built and is built in CI**; `cdylib` is not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has wasm target dependency block       | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm block: tokio/ureq/tempfile)                                                                                                     |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacks into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning on absence)                                                                                                                                                              |
| **27 files** contain the `wasm32` literal         | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in docs                                                                                                                                       |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target — `lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` entirely
(these three all need `std::fs`), while the shim takes a different path.

**But this brings a fact directly related to this document**: `wasm/src/lib.rs:48`'s playground
entry calls `compiler.compile_with_source(...)` — the **single-file path**. Therefore:

| Path Consuming `proof_calls`               | Executes Proof Function? |
| ------------------------------------------ | ------------------------ |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                  |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                  |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                  |
| Multi-file `run` (`compile_project`)       | **No**                   |
| `check` (`check_project`)                  | **No**                   |
| LSP in-project (`check_source_in_project`) | **No**                   |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver would miss this
path.

(Cleanup scope — which of `orchestrator.rs`'s 12 wasm attributes are unreachable in playground
scenarios — goes to `06-cleanup-inventory.md`. The open issue in RFC-039 "build wasm target, or
delete these branches" is **already obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The rationale is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer" — this is the same mechanism that the opcode table has already
proven effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check        Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lex + syntax                    PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/...)  Project
    Typecheck,          // Type check (with embedded proof layers)  PerModule
    DeadCodeAnalysis,   // Dead-code family analysis       Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When a new variant is added, this array and the exhaustive
    /// `match` in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once at
project level) turn "which stages must run per-module and which must run once at the project level"
into a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in
`dispatch` — this eliminates current issues in `orchestrator.rs` like "should
`allocate_global_slots` be called for every file" that require human reasoning.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on the order.

> **Revision Note (P4 implementation, 2026-10-07)**: Two deviations from the initial draft are
> corrected per implementation evidence:
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR output
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the initial draft order contradicted the
>    data flow.
> 2. The Check form's stage table is normalized per `Stage::ALL` topological order: dead code
>    **before** proof — `check_project` currently executes proof before dead code, the two have no
>    data dependency, diagnostic set is identical (C2 set semantics), only intra-file diagnostic
>    order is normalized. The single-file path (dead code embedded in typecheck, before proof)
>    already matches the ALL order; byte-level acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST output — the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; also, multi-file parse is reduced from 2 passes to 1 pass. The accompanied
>    status is faithfully registered: multi-file path parse errors were once a hard stop
>    (`build_registry_from`'s `?` propagation, the tension with CollectAll semantics registered as
>    WBS 4.10.1) — **4.10.1 has been fixed** (2026-10-09 user ruling: Rust-style collection
>    semantics + Plan B): Check path parse failure degrades to per-file diagnostic collection, sick
>    files exit the compilation unit (don't enter registry, importers report E5001); the MultiFile
>    path keeps the hard stop (under FailFast a bad file can't produce IR, which is correct
>    semantics, and the policy is locked long-term).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data-dependency
>    edges grow from 14 to 16: `RoleClassification` consumes Discovery's `used_by` edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events — the initial draft missed these
>    (in the orchestrator era, these two outputs were produced by `discover_with_used` and dropped
>    by the `discover` wrapper, the data flow was lost when not entered into the table); b.
>    Intra-file diagnostic order normalization extended to the full form of note #2: E3020 entry
>    validation moved into the `RoleClassification` arm (Check has no Linking stage, the arm holding
>    surfaces and AST is responsible for validation), intra-file diagnostic order = stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); diagnostic set is
>    unchanged (C2 set semantics), only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no consumer" from undetectable into a compile-time or
test-time-assertable fact. RFC-039 has already listed this as "the single highest-value item."

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
    // … other span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three levels)**:

| Situation                                | Criterion                                                   | Disposition                                                                        |
| ---------------------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Obligation read by its declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                               |
| Obligation non-empty but no consumer     | Field non-empty and not consumed                            | **Produce diagnostic** (`W` level by default; `strict` mode upgrades to `E` level) |
| Obligation empty                         | Field is empty                                              | Pass (no consumption needed)                                                       |

**Why W level first instead of E level**: Fixing obligation drops **changes the diagnostic set**.
The C2 criterion requires "same diagnostic set at each entry"; jumping straight to E level,
`yaoxiang check` would suddenly have a flood of previously-silent `Unproven` diagnostics. Splitting
into two steps (W observe first, then upgrade to E) lets each step's criterion be usable. See
Implementation Note S4.

**The single call site of `assert_drained()`**: `Driver::run`, at the end of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third-file read → fail). `assert_drained()` is the **runtime** gate; the script is the
**static** gate; they are complementary, both needed.

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
    /// Must be one of 6 predefined combinations from `ProgramKind::stages()` only
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); the
caller may not freely pass an array.** This is the key constraint preventing the `Program`
abstraction from degrading into "free parameter passing", enforced by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides skipping, not stage return value
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

> **Revision Note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for the record:
>
> 1. The `StageOutcome` tri-state in §4 is not returned by the stage arm, but expressed via `State`
>    failure flag + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, eliminating
>    boilerplate return from each arm);
> 2. `Driver` has no `config` field — the only source of config is `Program.config` (§5 "Avoid
>    Driver Holding Mutable Global State" ruling overrides the sketch field);
> 3. `Skipped` diagnostic at 4.1.3 is only recorded into the internal `DriverOutcome.skipped`
>    ledger, not emitted — S2's "intentionally fix no bug" zero-diff criterion requires this;
>    emission comes with 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the sole new
>    caller (L1→L2 is an allowed direction); orchestrator legacy call sites will be discussed after
>    migration in 4.2.
> 5. In multi-file form, `ProofExecution` is an independent stage, executed after **all**
>    typechecking (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof
>    within the per-file typecheck loop (file N's proof before file N+1's typecheck). Under single
>    failure, the two are byte-identical; in the multi-failure case "file 1 proof failure + file 2
>    typecheck failure", the first report changes from proof error to typecheck error (test nailed).
> 6. `DriverOutcome` carries products via ProgramKind-channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a Driver-unifiable type surface.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec — `check_project`'s current
>    contract); Discovery output extended to "file set + `used_by` edge set + shadowing events"
>    triplet into State (MultiFile form has no downstream consumer, only recorded not emitted);
>    per-file parse on Check path drops from 2 to 1 pass per C3 topology.

**Refactoring approach for the ten entries (per-function specification)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                           |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                              |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                     |
| `compile_project` (`orchestrator.rs:99-237`)               | Thin down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                               |
| `check_project` (`orchestrator.rs:273-399`)                | Thin down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                 |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtering to target file                                                                                                                                                 |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                   |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                      |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                             |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project switches to `Program { kind: Lsp, aggregation: CollectAll }`; **manual lex→parse→… sequence in single-file branch deleted**, switched to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                               |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% divergent)**.
The shared parts: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, resolution loop, per-file checker assembly, result handling. The divergent
parts: items 2/3/4/5/7/8/9/10/11 in the 11-row table above. **This 40% shared skeleton is exactly
where the value of `Driver` lies** — the divergent 60% is entirely "different stage set" or
"different aggregation mode", and these are exactly what the two fields of `Program` can fully
express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure caused this stage to skip" is NOT a stage return value.**

The rationale: skipping is decided by **topology**, not by the stage itself. If we let every stage
return `Skipped`, then "why I didn't run" information is scattered across 12 stages and cannot be
centrally audited. Instead, `dispatch` decides at the top of the loop by dependency:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
aims to fix — `pipeline.rs:187`'s `if !proof_calls.is_empty()` is a silent "skip" today, producing
no diagnostic, just because the obligation happens to be empty. Once the rule is established, any
"because X didn't happen so Y didn't run" must be explainable.

Diagnostic text must distinguish three skip reasons:

| Reason                                                 | Text Direction                                                                                                           |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Upstream `Abort` caused                                | "Stage Y did not execute due to upstream stage X failure"                                                                |
| Prerequisite obligation empty caused                   | "Stage Y did not execute due to no pending obligations (normal)" — **severity = Info for this, not counted in warnings** |
| Condition unmet (e.g., `config.mono.enabled == false`) | "Stage Y did not execute due to configuration not enabled"                                                               |

The second category is key: it makes "normal skip" distinguishable from "abnormal skip" in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count` (the non-blocking
contract depended on by `diagnostic/mod.rs:637-641`).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop at first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                              | Current Corresponding |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (early-return at `pipeline.rs:150/162/174/193`)                 | Already exists        |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists        |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single files within the same process, while CLI's `run` and `check` are two
independent calls; placing them on Program avoids Driver holding mutable global state.

**Note the semantic difference of the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a difference of
"early-return or not" — they are two different checker entries. After unification, the same
implementation should be driven by the `Aggregation` parameter, not two functions retained (**this
is an item requiring additional verification, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-Level Impact                                                                                                                              |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation introduced; no touching `MonoType` / `PolyType` / `ir::Type`                                              |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` still `ownership::ReleasePlan`, `proof_calls` still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                     |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is **settlement
responsibility**, not the type ownership module. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`** — this is consistent with the driver non-reverse-dependency red
line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**. A
`Driver` appearing in `TypeChecker`'s import is considered a violation, intercepted by the
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                          |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                  |
| `yaoxiang run` (multi file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already plugged in by P3; **warning surface stays unchanged** (Decision B1, 2026-10-09: dead code family warnings don't enter compile path — pool semantics being corrected as a defect by 4.9, normalizing warning surface deferred until after 4.9; this row's initial draft "new warning output" is voided) |
| `yaoxiang check` (multi file)       | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                       |
| LSP (in-project)                    | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                       |
| Z3 not installed + single file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**behavior change, see compatibility**)                                                                                                                                                                                                                                               |
| Z3 not installed + multi file       | Silently skip                            | Same, unified                                                                                                                                                                                                                                                                                                                  |

**The only intentional behavior break** is `predicate.rs:35`'s `.expect()` changed to return a
diagnostic. This aligns with RFC-027 §8 "not bound to a specific solver", and with the contract
already declared by `backend.rs:60` ("caller should conservatively degrade") — the current
implementation is contrary to its own contract document.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single file / N for multi file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` thinned down to Program constructor                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` thinned down to Program constructor                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` switched to Driver                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` either-or driven by `Aggregation` parameter                                                                                                                                                                                                                              |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, switched to Driver                                                                                                                                                                                                                                                                               |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                     |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls into path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                    |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent drop**)                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer-order table changed to **actual** execution order; delete "lower-layer failure means upper-layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs` / `checker.rs` / `ownership.rs` three consumer sites switched to shared port. Bare `&'static` reference form is not feasible (`dyn Solver` is not `Sync`), original wording governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is at the consumer, not at the producer**, changing the
producer would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]`
branch (reachability determination of wasm branches goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                          | Disposition                                                                                                                                 |
| --------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously silently-passing constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; staged release, expected diagnostic set changes go to CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compiled cleanly now produce warnings      | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                            |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set increased                                 | W first then E, S2/S4 split into two steps                                                                                                  |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                     | No breakage                                                                                                                                 |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                          | These two paths don't go through proof_execution                                                                                            |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                  | If `pub` API surface has external dependencies, sync needed; in-repo consumers already fully listed in change list                          |

---

## Implementation Notes

This document corresponds to RFC-039 global stage sequence **P3 (Fix Correctness Vulnerabilities)**
and **P4 (Stage Contracts and Unified Driver)**. The following S1–S5 is the implementation order
**within** this document; **each step's acceptance criterion references the C2 category of the
[Equivalence Criteria document](07-equivalence-oracle.md)** (orchestration changes: same
**diagnostic set** at each entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** Vulnerability criteria must be written as failing first.

| Deliverable                                                                                                                                                 | Acceptance                                                                                                |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                               | **Must be red**. If accidentally green, the vulnerability analysis in this document needs re-verification |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Resolution D48 — not mixed into single-file corpus tree; project fixture with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file versions are comparable                                     |
| `test_obligations_drained` skeleton                                                                                                                         | Mark `#[ignore]`, switch to green in S4                                                                   |

**Rollback point**: no code changes, only new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                             |
| Full corpus (293 `.yx` files) diff                       | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage intentionally fixes no bug** — it only moves existing behavior into the Driver. The
acceptance criterion is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entries' diagnostic sets normalized per `Aggregation` are identical          |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in-project and out                          |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give same diagnostic set for same file                                |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in Z3-absent environment                                         |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has issues, only the structural commit can be rolled back, keeping the
vulnerability-fix commit — at that point `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keep structure, roll back fix) turns red, which is an unacceptable intermediate state
and is forbidden from merging.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                         |
| -------------------------------------------- | -------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green             |
| `scripts/ci/check-obligations.py`            | Static gate in effect                              |
| Obligation diagnostic W → E upgrade          | Full corpus diff, then manual review of each new E |

**Rollback point**: severity of `assert_drained()` is controllable by config; W/E switching doesn't
require code structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                        | Acceptance                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                             | Document and `checker.rs` call sites correspond one by one                                                                                                               |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache** — otherwise a single timeout gets cached process-wide and persists across compilations, and pollutes cargo test thread-shared state) | `default_solver()` call sites on production path zero (tests only retained); cache hit rate observable across three consumer sites (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                    | Diagnostic when no `body_checker`                                                                                                                                        |
| `orchestrator.rs` 12 wasm attribute reachability determination                                                                                                                                                                     | Conclusion goes to `06-cleanup-inventory.md`; this document only registers the determination need                                                                        |

**Note**: Fixing the layer order will change the diagnostic set and may expose a flood of
previously-silent `Unproven`. **This item should be taken independently of S1–S4**, not mixed with
entry merging in the same PR.

## Key Decisions and Rationale

| Decision                 | Determination                                                                                                                                                | Rationale                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration refused**                                                                       | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism the opcode table has verified in RFC-039 routing table B. `StageScope` further turns "per-module or project-level" into a type fact, eliminating the call-count issue that requires human reasoning |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, combined with `scripts/ci/check-obligations.py` **static** gate; severity W first then E      | This fixes an entire class of bugs, not one: at least 2 same-class hidden dangers verified ( `proof_calls`, `checker.rs:1313`), all 16 span-keyed fields are in range. W first then E is to make each step's C2 criterion usable — a one-step upgrade to E would make `yaoxiang check` suddenly flood with previously-silent diagnostics                    |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **leave untouched** the six `Unproven` branch producer logics at `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer would mask the architectural defect as "logic fixed", whereas the logic of these three branches is itself correct                                                                                                                                                         |

This design additionally eliminates two classes of cracks:

- **The crack between "comment promises" and "code behavior"**. `checker.rs:5165`'s "must not let
  the silent pass come back to life" and `layers/README.md:3`'s "lower-layer failure means
  upper-layer doesn't run" are both **comment-level contracts** — the former is honored by
  `assert_drained()`, the latter is exposed by topology-driven `Skipped` diagnostics.
- **"One array forgot an append"**. Currently five functions each write their own call sequence;
  after unification, there is one stage table, and a new stage missing wiring would be a compile
  error.

### Unadopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of 11 inconsistencies (two
  dead code analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type transitions**; they are configuration issues of "whether
  to run some analysis"; it also cannot express `Aggregation`, and can only move the `check_module`
  / `check_module_collect_all` either-or from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — removes compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is exactly
  equivalent to replacing "five functions each write their own call" with "one array forgot an
  append" — the cause of the 11 inconsistencies today. The same-shape anti-example already exists in
  this repo: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described
  in `docs/src/dev/design/check/` are all zero-implementation.
- **Only patch bugs, don't touch structure** — fixes the known 2 bugs, but only covers 1 of the 16
  obligation fields, `Stage` is still scattered across 5 functions, and the next fork point will
  continue to grow from here. **It must be done first** (part of S1/S3), because the structural
  refactor needs a known red test to prove the criterion is effective.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry; it
  doesn't know who the caller is, so `debug_assert!(<caller will handle>)` cannot stand;
  `#[must_use]` only warns when the field is dropped **as a whole**. This is looking for the bug at
  the wrong level: the bug is in the orchestration layer, detection must be in the orchestration
  layer.

## Known Limitations and Risks

- **`orchestrator.rs` thinning will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all refactorings that "centralize
  wiring".
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously-silent problems"**. After the fix, there may be a wave of user reports of "new errors"
  — they are real bugs, just never reported before. Must be clearly stated in CHANGELOG.
- **The severity switch of `assert_drained()` requires per-field human judgment**. Among the 16
  fields, for some (e.g., `module_namespaces`), "no consumer" may be by design and shouldn't trigger
  a warning. S4 needs to go through each field one by one, can't be one-size-fits-all.
- **`Program` abstraction may be premature**. If the stage set of some entries is unstable
  long-term, `stages()` would degrade to "different array passed on every call" free parameter,
  contract constraint is hollow. **Mitigation is `test_program_stage_coverage` asserting that
  `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3 vulnerability fix criterion depends on the
  [Equivalence Criteria document](07-equivalence-oracle.md)'s vulnerability test going red first.
  **If that document doesn't first establish a multi-file corpus layer, S1 cannot be accepted** —
  because the existing 293 corpora all take the single-file path, zero coverage of this class of
  defect. This document does not involve the IR verifier (`verify_loose`); that part of prerequisite
  work is outside this document's scope.
- **Layer-order fix will expose previously-silent `Unproven`** (`equivalence` not in the pipeline at
  all, `termination` and `ownership` order is reversed from the declaration). This is a
  diagnostic-set change risk; should be taken independently.
- **Thread safety of the SMT backend after singleton conversion is undetermined**.
  `backend.rs:13-19` already explains `Solver` is `Send` only, not `Sync`; `Z3Backend`'s cache is
  `RefCell`; after conversion to cross-compilation-unit shared singleton, the access path must be
  confirmed to go through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelism will **mask order-dependence defects** (e.g., the actual order dependence of
  termination and ownership). Should be opened after the equivalence criteria stabilize.

> **The open questions originally listed in this section have all been resolved**. Per-item
> decisions are in [RFC-039 Resolution Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactor (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design origin of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "must not let the
  silent pass come back to life"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "lower-layer failure
  means upper-layer doesn't run" (no short-circuit in reality)
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
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, so
  proof_execution runs)
- `wasm/Cargo.toml:10-18` — `cdylib` + `wasm-bindgen` in the shim crate, not the main crate
- `build.rs:19-55` — Error code build-time gate, example of this project's mandatory mechanism
