# Compile Stage Contract and Obligations Ledger

> **Auxiliary design document**. This document is an auxiliary to
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, graded acceptance criteria, and stage execution order are in the main RFC-039;
> the role of each auxiliary document is described in the
> [table of contents for this directory](index.md).

## Position and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"which stages ran in this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each wired by hand, with no single place that can answer
this question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself have been broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` states verbatim: "Unproven → compile
error, no degradation, no silent pass... silent pass must not be revived." But this invariant only
holds on the **single-file path**: refinement constraints of the form `x: Sorted(3)` silently pass
on the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP — proof functions are
never executed and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, no runtime registration) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-function refactor plan for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of scope

- Four-layer model, dependency direction conventions, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order P1–P10 and G1–G10 acceptance gates → main RFC-039

### Division of labor with RFC-039

RFC-039 provides the **why** for the refactor and the **order** in which to do it; this document
provides the L1 **concrete form**, **per-file change list**, and the **internal implementation
stages** (corresponding to P3 "fix correctness vulnerabilities" and P4 "stage contract and unified
Driver" in the RFC-039 global sequence). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed under the **C2 (orchestration changes)** category of the
[equivalence criteria document](07-equivalence-oracle.md): same diagnostic set across entries + same
corpus behavior.

## Current State

> All items in this section are **verified facts**, each with a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage boundaries are the only "write once, silently wrong everywhere" structural defect

It differs in nature from the other two defect classes (module boundaries, test wiring):

| Defect                 | Typical manifestation          | Any signal?          |
| ---------------------- | ------------------------------ | -------------------- |
| Lexical / syntax error | Source code error              | Has diagnostic       |
| Type mismatch          | Type error                     | Has diagnostic       |
| Stage mis-wiring       | A stage is never called        | **No signal at all** |
| Unconsumed obligation  | Field filled in but never read | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
fixed by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what wasn't written**. This is exactly the root cause diagnosed by RFC-039: "this
project treats 'design' as documentation convention, not as an executable constraint."

### Existing enforcement mechanisms in this repository

The repository already has mature enforcement mechanisms — they just haven't been extended to the
stage layer:

- The **145 error codes** (137 E + 8 W) under `src/util/diagnostic/codes/` are gated at build time
  by `build.rs:19-55` through `tools/code-tables`, which does a hard comparison against the RFC-013
  code table; any inconsistency triggers `panic!` and refuses compilation.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines / 47.9% are under `tests/`) has
  high test density, with each module's test subtree declared and wired — this is the most complete
  test wiring in the repository, and can serve as the form reference for the stage-layer gate.

**Design capability is sufficient. What is missing is "a gate at the same level placed at the
orchestration layer."**

(Line count metric: `(Get-Content).Count`, see the "Line count metric" section of
`06-cleanup-inventory.md`.)

### Six compile paths, ten entry functions

| #   | Path                       | Entry function                                        | Location                                      | Which compile path                            |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | --------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages, direct call                         |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                              |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking        |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics   |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)            |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + standalone IR       |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1             |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                 |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project via 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                 |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1      |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call point    | Implementation                                                |
| ------------------- | ------------- | ------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                      |
| parsing             | `161`         | `run_parsing`                                                 |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                   |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5         |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined** (`381-389`) |

### 11 places of stage-coverage inconsistency

The table below gives evidence cell by cell. **An empty cell does not mean that entry "doesn't do
this thing" — it means "there is no code in that entry that does this thing"** — which is the very
nature of the defect.

| #   | Stage / behavior                                             | `pipeline` (single-file)                            | `compile_project`                                          | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | ------------------------------------------------------------ | --------------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                          | **yes** `187-203`                                   | **no**                                                     | **no**                                                      | **no**                             | **no**                               |
| 2   | dead-code analysis                                           | yes `275-278` (gated by `config.dead_code.enabled`) | **no**                                                     | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                               |
| 3   | W1006 local module shadowing                                 | **no**                                              | **no**                                                     | yes `336-348`                                               | **no**                             | **no**                               |
| 4   | W1003 unused import                                          | yes (`277` collects `type_result.warnings`)         | **collected but never output**                             | yes `350`                                                   | **no**                             | **no**                               |
| 5   | W1001/W1002 dead-code family                                 | yes (same as 2)                                     | **no**                                                     | yes (same as 2)                                             | **no**                             | **no**                               |
| 6   | **monomorphization**                                         | yes `381-389` (gated by `config.mono.enabled`)      | **no**                                                     | N/A                                                         | N/A                                | **no**                               |
| 7   | typecheck branch                                             | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | file discovery                                               | N/A                                                 | `discover` (`101`, drops `used_by` / `shadow_events`)      | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                  |
| 9   | role context (`surfaces` / `test_rules` / `roles::classify`) | **no**                                              | **no**                                                     | yes (`321-328`)                                             | **no**                             | **no**                               |
| 10  | global slot allocation                                       | N/A                                                 | yes `allocate_global_slots` (`147`)                        | **no**                                                      | **no**                             | **no**                               |
| 11  | IR generation + qualified-name rewrite + linking             | yes (`205`)                                         | yes (`152-236`)                                            | **no**                                                      | **no**                             | yes (standalone ModuleIR then merge) |

> **Review note (WBS 3.4.3, cell-by-cell check on 2026-10-07)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline, preserved unchanged. Status differences after P3 lands:
>
> - **Row 1 is fixed**: the five entries for proof_execution share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the single-consumer defect
>   no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still doesn't output W1003, still doesn't run
>   the dead-code family) → assigned to WBS 3.4.6 (prereq 4.2.1).
> - **Row 6** (monomorphization is single-file exclusive) → assigned to WBS 3.4.8 (prereq 4.1.3,
>   potential risk, unproven defect).
> - **Row 7** check_module / check_module_collect_all double entry → assigned to 4.2.7 (Aggregation
>   parameter driven).
> - The code-side reference to "Adjudication #434" had no prior docs entry — now back-registered as
>   RFC-039 **D57**.
> - **Embedded std included in the asymmetry (off-table new fact, back-registered 2026-10-09)**: the
>   single-file path unconditionally injects `std.list` via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch in #117), whereas the multi-file `discover` previously only
>   recognized explicit `use` — for-loops inside the project compile, but at runtime produce E6006
>   (probed and verified). Fixed and WBS 4.10.2 ticked off; the same-family "multi-file missing one
>   single-file step" parse hard-abort quirk logged as WBS 4.10.1 — **also fixed** (immediately
>   after 4.2.2, the Check path downgrades to per-file collection; from the perspective of this
>   audit's method of looking at "stage coverage / field consumption" dimensions, this is a
>   compile-unit membership difference, an escape dimension).
>
> - **Embedded std registration surface coverage (found during 4.2.5, fixed)**: The Registry arm for
>   embedded std units uses `extract_module_info` to repeatedly harvest registrations, which
>   overwrites the native half-surface already registered by `with_std()` through the "native + yx
>   surface merge" path (loss of `result.is_err` etc. → std.test falsely reports E1043). The third
>   escape dimension of this audit: the merge semantics of "two registration sources for the same
>   module key" — the single-file path's registry is shaped only by one `with_std()` call, whereas
>   the multi-file/Check Registry arm inserts per unit, hence the coverage exists.

**Two cells require precise wording, otherwise the analysis will be wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138` — `type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but the function **has no `result.warnings` read site at all**; the only downstream
  use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` criterion is also sourced differently**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "has manifest"). The two functions give different answers to "which files must define
  `main`."

### Correctness vulnerability: proof obligations silently dropped (full evidence chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — There is only one production point for obligations.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the repository
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

The semantics are: "the predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed to decide."

**Step 2 — There is a consumption point inside the checker, but it only produces, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                            | Location                                                                                                     | Behavior                |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Formal-parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **emits no diagnostic** |
| Call-site argument check          | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |
| Return-position obligation        | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |

The comment at `checker.rs:5165-5169` reads verbatim:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a true defense line, not a forward-looking cushion: silent pass must not be revived.`

That is: the author knows "no diagnostic produced" is a defect, and explicitly classifies it as a
**known, with-nowhere-to-go intermediate state**, designed under the assumption that "someone will
definitely read `proof_calls`."

**Step 3 — The field does in fact get filled into the result.** `checker.rs:1234-1235` declares and
accumulates a local `proof_calls`; `checker.rs:1439` writes it into `TypeCheckResult` with the
comment `proof_calls, // Phase 2.5 pre-register proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — There is exactly one read site in the entire repository.** In production code,
`TypeCheckResult.proof_calls` (`types.rs:29`) is read **only at `src/frontend/pipeline.rs:187`**
(the `189` argument-passing site):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the repository fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-name field on `ProofResult`; the `tests/rfc027_*.rs` files read
`ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer entirely.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
in `src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**; they call
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they never even reach the
sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: even the standard library's own refinement obligations are dropped.**
`compile_embedded_module` (`1374`, the `check_module` call at `1386`) is responsible for compiling
the embedded std. This means **the proof obligations of the embedded std module itself are not
executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) compiles, runs, and **emits no diagnostic** on the multi-file
/ check / LSP paths. The proof function is never called.

**Step 8 (additional verification) — There is a second silent-drop point in the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership check layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and
no ledger entry.** This directly contradicts the promise at `5164` that "silent pass must not be
revived," and is **independent of the orchestrator problem** — even if the entry layer were
completely fixed, this spot would still be silent. (Handling timing: should be processed in the same
batch as the obligations ledger, because it is the same kind of problem.)

### Why tests didn't catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty — for example `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists
in `result.proof_calls`. But they all go through `check_source` → `checker.check_module(&module)`
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping exactly
before `pipeline.rs`**.

The test file's own documentation comment already says this. From
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases live in the .yx layer.`

**This is exactly the shape of the bug: the test verifies "the obligation is filled in," while the
bug is "the consumer doesn't read it."** A test that only tests the producer side, not the consumer
side, is naturally immune to this class of defect.

### A stronger finding: the primary corpus of the equivalence criteria is the single-file path

The [equivalence criteria document](07-equivalence-oracle.md) uses end-to-end differential of the
293 `.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion**, the main acceptance
tool for C2 stage changes. But in practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero hits on a directory-wide glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files take the single-file path, all of them cover
  proof_execution, none of them cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is direct evidence of
this. The file is tagged `// expect: compile-error E4018` (line 15), and its header comment at line
7 says "Status: ❌ should be rejected at compile time" — **it passes precisely because it takes the
only path that executes the proof function**.

**Conclusion: the third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation
essentials" S1.

### Obligation fields: 16 "produce-as-contract" fields, zero mechanism guarantee

Of the `TypeCheckResult` defined at `src/frontend/core/typecheck/types.rs:16-70`, **16 fields** have
doc comments that explicitly say "produced by stage X → consumed by stage Y" — they are essentially
**cross-stage obligations**:

| Field                      | Defined at | Producer → Consumer                                                  |
| -------------------------- | ---------- | -------------------------------------------------------------------- |
| `proof_calls`              | `29`       | typecheck → **proof_execution**                                      |
| `release_plan`             | `31`       | ownership → IR generation (read at `ir_gen.rs:341`, used at `:1942`) |
| `escaped_refs`             | `33`       | ownership → IR generation                                            |
| `instantiation_requests`   | `35`       | typecheck → monomorphization (`pipeline.rs:381`)                     |
| `existential_coercions`    | `37`       | typecheck → IR generation                                            |
| `implementation_proofs`    | `39`       | typecheck → IR generation                                            |
| `interface_impl_registry`  | `42`       | typecheck → operator lookup / constraint solving / LSP               |
| `sum_types`                | `44`       | typecheck → IR generation                                            |
| `sum_type_param_names`     | `47`       | typecheck → IR generation                                            |
| `variant_ctor_calls`       | `49`       | typecheck → IR generation (span-keyed)                               |
| `operator_dispatches`      | `51`       | typecheck → IR generation (span-keyed)                               |
| `method_overload_ir_names` | `53`       | typecheck → IR generation                                            |
| `overload_resolutions`     | `56`       | typecheck → IR generation (span-keyed)                               |
| `try_expr_impls`           | `59`       | typecheck → IR generation (span-keyed)                               |
| `match_scrutinee_types`    | `63`       | typecheck → IR generation (span-keyed)                               |
| `module_namespaces`        | `66`       | typecheck → IR generation                                            |

**All of these fields are passed across layers keyed by `Span` or name tables.** The equivalence
criteria document already notes that a mismatch in the span-keyed contract will "silently fail, with
no error" (`ReleasePlan` is typical: any inconsistency in the way spans are computed on either side
→ `Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), is inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the
  consumer is in the downstream module, and the downstream module is on the necessary path of every
  entry.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, in **another entry
  implementation**. The four orchestrator entries don't pass through `pipeline.rs` at all.

**Verified fact**: there is currently **no mechanism** in the repository that guarantees any of
these 16 fields are consumed. The only "protection" is that `ReleasePlan` happens to piggyback on IR
generation.

### Four additional contract defects in the proof layer

The following all come from the proof layer analysis. They are independent of the entry-divergence
problem, but are all of the type "stage contract not enforced."

**(a) The declared layer order contradicts the actual execution order, and the `equivalence` layer
is not in the pipeline at all.**

The layer order declared by `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Depends on    |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README states: "executed in layer order; if a lower layer fails, the upper layers do
not run." The actual call sites inside `TypeChecker::check_module`:

| Actual order | Call site                                         | Declared layer |
| ------------ | ------------------------------------------------- | -------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2        |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1        |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3        |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0        |

That is: the order of `termination` and `ownership` is **the opposite** of the declared order; the
declared Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all**
(it is used only as a utility `is_subtype` function by `inference/assignment.rs:17`, unrelated to
`ProofResult`). At the same time, **there is no short-circuit** — `checker.rs:1271-1276`
`add_error`s each termination error individually and continues; the README's promise of "if a lower
layer fails, the upper layers do not run" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic has been removed by P3 task 3.3.1 (SOLVER slot
> made `Option`, missing backends degrade conservatively to `SMTResult::Unknown`); "silently skipped
> (not injected)" is covered by W1081 added in 3.4.2. Unification of the three forms
> (singletonization) is executed per **RFC-039 D58**, with the actual fix in `proof/smt/backend.rs`
> (new process-level shared singleton + `with_shared_solver` closure access point); the description
> "default_solver() → &'static" in the 02 change list is superseded by D58 (`&'static` raw reference
> is not viable: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition strategy                                     | When the solver is unavailable                             |
| ------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                   | global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected via `checker.rs:1265-1268`) | construction-time injection `with_solver_owned`          | `None` → **silently skipped** (not injected)               |
| `ownership.rs:627` (back-edge cut-off decision)        | **per call** `default_solver()`                          | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chose hard-fail: "Initialization failure remains
**hard-fail**: softening would mistake 'Z3 not installed' for 'constraint beyond kernel
capability'." But the contract documented at `backend.rs:60` says "`None`: backend unavailable...
**the caller should degrade conservatively**." **Two philosophies coexist in the same codebase, and
the hard-fail path panics instead of returning an error.**

**(c) A new Z3 context is created on every back-edge decision, making the cache a dead letter.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut-off decisions. But at
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call does `Z3Backend::new()`, i.e. creates a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`." Not shared across calls ⇒
**this cache never hits under this calling mode**.

(For contrast: the `LazyLock` at `predicate.rs:34-36` is genuinely a singleton. Same backend, two
lifecycle strategies.)

**(d) The checker silently degrades, and the comment doesn't match reality.** `checker.rs:1283-1287`
and `1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check receives an **empty type ledger and empty call
table** and runs as if everything were fine — ownership analysis degrades to "nothing conflicts,"
**with no warning whatsoever**. (Suggested handling: record as a warning-level diagnostic, or at
least leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving." In reality `self.env.solver().solve()` is at `checker.rs:1320` — **termination
is at `1256`, ownership at `1296`, so the "before" referred to by the comment is actually "two
layers later"**.

### wasm current state: carried by shim crate, the 27-file branch is alive

The wasm target **is built and built in CI**; the `cdylib` is in a separate shim crate, not the main
crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has a wasm target dependency section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                   |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, downgrades to warning if missing)                                                                                                                                                            |
| **`wasm32` literal appears in 27 files**          | of which **25** carry actual `#[cfg(...)]` attributes; the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in documentation                                                                                                                             |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | all attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call in a wasm target —
`lib.rs:139/153/170` gate off `run_file` / `run_project` / `build_bytecode` entirely (all three need
`std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — the **single-file path**.
Therefore:

| Path that consumes `proof_calls`           | Whether it executes proof functions |
| ------------------------------------------ | ----------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **yes**                             |
| wasm playground (`wasm/src/lib.rs:48`)     | **yes**                             |
| `build_bytecode` (`lib.rs:171`)            | **yes**                             |
| Multi-file `run` (`compile_project`)       | **no**                              |
| `check` (`check_project`)                  | **no**                              |
| LSP in-project (`check_source_in_project`) | **no**                              |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "it has
three,"** but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target design" §3), otherwise the unified Driver will miss this path.

(The cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is owned by `06-cleanup-inventory.md`. The open question in RFC-039 "build a
wasm target, or delete these branches" **is now stale**: the target is already built.)

---

## Target Design

### 1. Stage model: `Stage` enum (exhaustive, no runtime registration)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "a new stage must be
handled by the orchestration layer" — this is exactly the mechanism that has already been proven
effective for the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check              Project
    Discovery,          // File discovery                       Project
    Parsing,            // Lexing + parsing                     PerModule
    Registry,           // Module registry construction         Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type checking (with inlined proof)   PerModule
    DeadCodeAnalysis,   // Dead-code family analysis            Project
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

**The purpose of `StageScope`**: `PerModule` (runs once per compile unit) and `Project` (runs once
at the project level) make "which stages must run per module and which must run once at the project
level" a type-level fact. Stages with `Project` scope are structurally guaranteed to run only once
inside `dispatch` — this eliminates questions like "should `allocate_global_slots` be called once
per file?" that currently require human reasoning in `orchestrator.rs`.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the original draft,
> corrected by implementation evidence —
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR output
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original draft order contradicts
>    the data flow.
> 2. The Check-form stage table is normalized to "dead code **before** proof" per the `Stage::ALL`
>    topological order: `check_project` currently runs proof before dead code; the two have no data
>    dependency, the diagnostic set is the same (C2 set semantics), and only the per-file diagnostic
>    order is normalized. The single-file path (dead code inlined in typecheck, before proof)
>    already matches the ALL order, so per-byte acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, user ruling 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST output — the
>    original draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; and multi-file parse thus drops from 2 runs to 1. Current-state
>    disclosure: multi-file parse errors used to be hard-abort (`build_registry_from`'s `?`
>    propagation, the tension with CollectAll semantics logged as WBS 4.10.1) — **4.10.1 is fixed**
>    (user ruling 2026-10-09, Rust-style collection semantics + plan B): the Check path's parse
>    failure downgrades to per-file diagnostic collection, the affected file leaves the compile unit
>    (does not enter the registry, importers report E5001); the MultiFile path keeps hard-abort (a
>    bad file under FailFast cannot produce IR, which is correct semantics, and the pinboard is
>    valid long-term).
> 4. Check-form landing (4.2.2, user ruling 2026-10-09) adds two entries: a. Data-dependency edges
>    14 → 16: `RoleClassification` consumes Discovery's `used_by` edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadowing events — the original draft missed these (these outputs
>    were produced by `discover_with_used` in the orchestrator era and dropped by the `discover`
>    wrapper; the data flow was lost as soon as it wasn't put in a table); b. Per-file diagnostic
>    order normalization extended to the full form of note #2 above: the E3020 entry check is moved
>    into the RoleClassification arm (Check has no Linking stage, so the arm holding `surfaces` and
>    AST is responsible for the check); per-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set semantics),
>    only the stderr entry order changes.

### 2. Obligations ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but no consumer" from undetectable into a fact assertable at
compile time or test time. RFC-039 has listed this as "the single highest-value item."

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
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or
    /// a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three levels)**:

| Situation                                | Criterion                                               | Disposition                                                           |
| ---------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------------------- |
| Obligation read by its declared consumer | field in `Obligations` is `take()`-en / marked consumed | Pass                                                                  |
| Obligation non-empty with no consumer    | field non-empty and not consumed                        | **produce diagnostic** (W-level by default; E-level in `strict` mode) |
| Obligation is empty                      | field is empty                                          | Pass (no consumer needed)                                             |

**Why W-level first, not E-level**: fixing obligation drops will **change the diagnostic set**. The
C2 criterion requires "the same diagnostic set across entries"; if we jump straight to E-level,
`yaoxiang check` will suddenly produce a large batch of `Unproven` diagnostics that were previously
silent. Two steps (W-level observation first, then upgrade to E) keep the criterion usable at every
step. See "Implementation essentials" S4.

**`assert_drained()` is called from exactly one place**: `Driver::run` at the tail of the stage
table, before producing the `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (field
appears ≥2 times but no third file reads it → fail). `assert_drained()` is the **runtime** gate;
that script is the **static** gate; the two are complementary, and both are needed.

### 3. Unified Driver: single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode (lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); it does
not accept a caller-supplied free-form array.** This is the key constraint preventing the `Program`
abstraction from degenerating into "a free parameter that accepts anything," and is asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Whether to skip is decided by topology, not by the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce a diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The single settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the sketch above, semantically equivalent, recorded for traceability —
>
> 1. The `StageOutcome` three-state in §4 is not returned by stage arms; it is expressed via a
>    `State` failure flag + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, saving
>    the per-arm boilerplate return);
> 2. `Driver` has no `config` field — the only source of configuration is `Program.config` (§5
>    "Avoid Driver holding mutable global state" supersedes the sketch field);
> 3. `Skipped` diagnostics are recorded only into the internal `DriverOutcome.skipped` ledger in
>    4.1.3, not emitted externally — required by S2's "deliberately fix no bugs" zero-diff
>    criterion; external emission follows in 4.3 with the obligations ledger;
> 4. `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is the allowed direction); the existing orchestrator call sites move out with
>    4.2, then revisit placement.
> 5. Under the multi-file form, ProofExecution is an independent stage that runs **after all**
>    typecheck passes (A1, user ruling 2026-10-09): `compile_project` originally interleaved proof
>    inside the per-file typecheck loop (file N's proof before file N+1's typecheck). For single
>    failures the two are byte-identical; for the multi-failure scenario "file 1's proof fails +
>    file 2's typecheck fails", the first reported error changes from a proof error to a typecheck
>    error (test pinboard).
> 6. `DriverOutcome` carries outputs in per-`ProgramKind` channels: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — the external error contracts of each
>    entry (`PipelineError` / `OrchestratorError`) are not a type surface the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec — `check_project`'s current
>    contract); the Discovery output is extended to the triple "file set + used_by edge set + shadow
>    events" entered into State (the MultiFile form has no downstream consumer, so it is only
>    recorded, not emitted externally); Check-path per-file parse drops from 2 runs to 1 along with
>    the C3 topology.
> 8. Lsp-form landing (4.2.3, source-of-file split ruling, user decision 2026-10-09): disk-file
>    parse failure follows Check's plan B (collect + leave compile unit — both ends converge in the
>    same downgrade); the edited buffer's parse failure keeps a partial AST and continues typecheck
>    (editor philosophy — the typing intermediate state shouldn't make semantic features disappear).
>    The Discovery arm covers the buffer source over the stale disk content for Lsp; Typecheck under
>    Lsp only checks the target file (other units only supply the registry with their signatures);
>    the per-file diagnostic order normalization extends to LSP (typecheck diagnostics → W-code
>    warnings → proof errors). The old behavior of LSP hard-aborting on unrelated disk-file parse
>    errors and the handler silently falling back to the single-file path is fixed by this step.
> 9. Embedded-form landing (4.2.4): `Program` gains a `shared_registry` field — the embedded std
>    module is a sub-compile, and the registry is an **input**, not an output (EMBEDDED_STAGES has
>    no Registry stage), so the #94 SymbolTable-sharing contract moves from "the caller remembers to
>    pass the same registry" to an explicit program declaration. The error-path text is normalized
>    from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only the compiler's
>    internal error surface, no test pins it). At this point all four orchestrator entries are moved
>    into the Driver.
> 10. standalone check unification (4.2.5, ruling A + IR-stage ruling, 2026-10-09): Check form for a
>     no-project-root program is a faithful embodiment of single-file semantics — the warning
>     surface only covers the entry file (neighboring files only get errors, better to miss than to
>     misreport); relative `use` resolves along the importer's directory (aligned with rustc
>     single-file mod). The CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone's old
>     path (full pipeline) already ran IR generation, and E3019/E1014/E1015 etc. are only produced
>     in `ir_gen` (verified by the runner gate). IrGeneration in Check form is pure check and does
>     not consume IR; the Script/Bin split is expressed via `module_key`'s None/Some (existing
>     switch for E3023, `ir_gen.rs:1660`). Monomorphization is not in CHECK (only produces
>     resource-limit / internal errors, zero corpus dependency). Side fix for a latent defect: the
>     Registry arm's repeated registration harvest for embedded std units would overwrite the native
>     half-surface already merged by `with_std()` (E1043 false positive) — embedded units now skip
>     the duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→`check_module_collect_all`
>     sequence in `run_diagnostics` is deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     Under the SingleFile+CollectAll form: Parsing collects all parse errors, keeps a partial AST
>     and continues typecheck (editor philosophy extends from the 4.2.3 ruling; the stage is not
>     marked failed, otherwise the topology skip would keep typecheck from ever running); Typecheck
>     dispatches the `check_module_collect_all` free function. From here on, the LSP single-file
>     path catches up on proof (E4018), W-code warnings (W1001–W1005, test pinboard), and IR-level
>     errors; lex failures now report real diagnostics (the old synthetic "E0001 lexical error" text
>     disappears with the sequence deletion — real diagnostics carry precise spans, with no
>     information loss).

**Refactor plan for the ten entries (per-function specification)**:

| Entry                                                      | After refactor                                                                                                                                                                                                          |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                 |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                         |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                           |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a single Driver call + filtering results to the target file                                                                                                                                                   |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                               |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                                  |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (delegates to Driver)                                                                                                                                                                                         |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project: switch to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence in the single-file branch is deleted**, replaced with `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | The `standalone` branch (`614-616` → `check_single_file` `623-661`) is deleted, unified to `Program { kind: Check }`                                                                                                    |

**The duplication between `check_project` and `compile_project` (about 40% shared / 60%
divergent).** What's shared: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, the resolution loop, per-file checker assembly, result handling. What
diverges: items 2/3/4/5/7/8/9/10/11 in the 11-row table above. **That 40% shared skeleton is exactly
where the value of `Driver` lives** — the 60% divergence is all "different stage sets" or "different
aggregation modes," which is precisely what the two fields of `Program` can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to be skipped" is not a stage return
value.**

Reason: skipping is decided by **topology**, not by the stage itself. If we let each stage return
`Skipped`, the "why didn't I run" information would be scattered across 12 stages and impossible to
audit centrally. Instead, `dispatch` decides at the top of the loop based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent "skip":
it produces no diagnostic, only because the obligation happens to be empty. With the rule in place,
any "because X wasn't done, Y didn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text direction                                                                                                                                |
| ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` caused it                              | "Stage Y was not executed because upstream stage X failed"                                                                                    |
| Pre-obligation empty caused it                          | "Stage Y was not executed because there were no pending obligations (normal)" — **this item's severity = Info, not counted in warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y was not executed because the configuration is not enabled"                                                                           |

The second class is key: it lets "normal skip" be distinguished from "abnormal skip" in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count` (the
`diagnostic/mod.rs:637-641` non-blocking contract depends on this counter).

### 5. Diagnostic aggregation mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop at first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                             | Current counterpart |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` returns `OrchestratorError::TypeCheck` on first error), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early returns)           | Already exists      |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists      |

**`Aggregation` must be a field of `Program`, not a global setting on the Driver** — LSP serves both
in-project and single-file in the same process, while CLI's `run` and `check` are two independent
invocations; putting it on Program avoids the Driver holding mutable global state.

**Note the current semantic difference between the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a "return early or not"
difference — they are two different checker entry points. After unification, the `Aggregation`
parameter should drive a single implementation, not keep two functions (**this is an item this
document needs additional verification on, see "Known limits and risks"**).

---

## Detailed Design

### Type system impact

| Change                                      | Type-level impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation, does not touch `MonoType` / `PolyType` / `ir::Type`                                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not type ownership. `ReleasePlan` is still defined in `layers/ownership.rs`,
`ProofFunctionCall` is still in `proof/`, and the other field types stay in their original places;
`driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(such as `ir_gen` reading `release_plan`) keep receiving the concrete field type via parameters, and
**must not `use crate::driver`** — consistent with the no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not `use crate::driver` in
reverse**. A `Driver` appearing in `TypeChecker`'s import list is a violation, caught by the
`scripts/ci/check-module-boundary.py` proposed in RFC-039.

### Runtime behavior

| Scenario                            | Before refactor                          | After refactor                                                                                                                                                                                                                                                                                                                |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                 |
| `yaoxiang run` (multi file)         | no proof_execution, no W1001/W1002/W1003 | proof_execution was plugged in by P3; **warning surface stays unchanged** (decision B1, 2026-10-09: dead-code-family warnings do not enter the compile path — pool semantics is being fixed by defect judgment 4.9, warning surface normalization left to after 4.9; the original "new warning output" in this row is voided) |
| `yaoxiang check` (multi file)       | no proof_execution                       | Plugged in by P3                                                                                                                                                                                                                                                                                                              |
| LSP (in-project)                    | no proof_execution                       | Plugged in by P3                                                                                                                                                                                                                                                                                                              |
| Z3 not installed + single file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                     |
| Z3 not installed + multi file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                        |

**The only intentional behavior break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver," and with
the contract already declared at `backend.rs:60` ("the caller should degrade conservatively") — the
current implementation contradicts its own contract document.

### Compiler change list

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

| File                                               | Line range                         | Change                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                                   |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                          |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead-code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` switched to Driver                                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` is driven by the `Aggregation` parameter                                                                                                                                                                                                                                                         |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | The manual stage sequence in `run_diagnostics` is deleted, replaced with Driver call                                                                                                                                                                                                                                                                                                    |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                                         |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently falls through to path 1 implicitly via `Compiler::compile_with_source`)                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce a diagnostic (**fix second silent-drop point**)                                                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | Add warning diagnostic to the `unwrap_or_default()` downgrade path                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Change the layer table to the **actual** execution order; delete "if a lower layer fails, the upper layers do not run" (no short-circuit exists)                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add a process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access point; the three consumer sites in `predicate.rs` / `checker.rs` / `ownership.rs` are switched to the shared access point. The `&'static` raw-reference form is not viable (`dyn Solver` is not `Sync`); the original wording is superseded by D58 |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is on the consumer side, not the producer side**; changing
the producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (reachability of the wasm branches is owned by
`06-cleanup-inventory.md`).

### Backward compatibility

| Change                                                    | Compatibility                                                                   | Disposition                                                                                                                                          |
| --------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: constraints that previously passed silently will now report E4018 | `test_multifile_proof_obligation_not_dropped` **written red first** then fixed; phased release, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compile cleanly now produce warnings                | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`); exit code does not change                            |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set grows                                              | W first, then E; two steps via S2/S4                                                                                                                 |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                              | No break                                                                                                                                             |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                                   | These two paths do not go through proof_execution                                                                                                    |
| `TypeCheckResult` fields migrate to `Obligations`         | **Internal refactor**                                                           | If `pub` API has external dependents, sync them; in-repo consumers are all listed in the change list                                                 |

---

## Implementation Essentials

This document corresponds to **P3 (fix correctness vulnerabilities)** and **P4 (stage contract and
unified Driver)** of the RFC-039 global stage sequence. S1–S5 below are the internal implementation
order, and **each step's acceptance criterion references the C2 category (orchestration change: same
diagnostic set across entries + same corpus behavior) of the
[equivalence criteria document](07-equivalence-oracle.md)**.

### S1: Build the criterion (red first)

**Prerequisite, not skippable.** The vulnerability criterion must be written as failing first.

| Deliverable                                                                                                                                                       | Acceptance                                                                                                      |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                     | **Must be red**. If it's green by accident, the vulnerability analysis in this document needs to be re-examined |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (per ruling D48 — do not mix into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions of the corpus can be compared                    |
| `test_obligations_drained` skeleton                                                                                                                               | Mark `#[ignore]`, flipped to green in S4                                                                        |

**Rollback point**: no code change, only new tests.

### S2: Introduce `Stage` + `Program`, no behavior change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code are byte-identical**                            |
| Differential across the full corpus (293 `.yx`)          | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bugs** — it only moves existing behavior into the Driver.
Acceptance is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, restore `pipeline.rs` to its original state.

### S3: Merge the four orchestrator entries + fix the vulnerability

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The four entries' diagnostic sets are the same after normalization by `Aggregation` |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves the same inside and outside the project                    |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` flipped to green**                                                              | Vulnerability fixed                                                                 |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` produces a diagnostic in every mode   |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in a Z3-less environment                                           |

**Rollback point**: vulnerability fix and structural merge are in **two separate commits**. If the
structural merge has problems, only the structural commit can be rolled back while keeping the
vulnerability fix commit — in this case `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keep structure, roll back fix) flips to red, which is an unacceptable intermediate
state and is forbidden from merging.

### S4: Enable the obligations ledger

| Deliverable                                  | Acceptance                                                            |
| -------------------------------------------- | --------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` flipped to green                           |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                              |
| Obligation diagnostic W → E upgrade          | After differential across the full corpus, manually review each new E |

**Rollback point**: the severity of `assert_drained()` is controllable by a config item, so W/E
switching does not require code-structure change.

### S5: Proof layer and wasm wrap-up

| Deliverable                                                                                                                                                                                                                                    | Acceptance                                                                                                                                                                  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                         | Documentation matches each `checker.rs` call site one by one                                                                                                                |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: `Unknown` must not enter the cache** — otherwise a single timeout would固化 across compilations via the process-level cache and pollute cargo test thread-shared state) | Production-path `default_solver()` call sites reach zero (only tests retain it); cache hit rate is observable across the three consumer sites (counter landed in 89576fafd) |
| `checker.rs:1283-1293` downgrade adds a warning                                                                                                                                                                                                | Diagnostic present when `body_checker` is absent                                                                                                                            |
| Reachability determination for the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                     | Conclusion handed to `06-cleanup-inventory.md`; this document only logs the need                                                                                            |

**Note**: fixing the layer order will change the diagnostic set and may surface a large batch of
previously-silent `Unproven`. **This item should be done independently of S1–S4**, not mixed with
entry merging in the same PR.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                               | Reason                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration refused**                                                                                 | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism already validated for the opcode table in RFC-039 routing table B. `StageScope` further makes "per module or project level" a type fact, eliminating call-count questions that need human reasoning   |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, paired with the **static** gate `scripts/ci/check-obligations.py`; severity W first, then E             | What we're fixing is a whole class of bugs, not one: at least 2 same-class hazards have been verified (`proof_calls`, `checker.rs:1313`), with all 16 span-keyed fields in range. W first then E keeps the C2 criterion usable at every step — a single jump to E would make `yaoxiang check` suddenly produce a large batch of previously-silent diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the producer-side logic of the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed," while the logic in those three branches is itself correct                                                                                                                                                        |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment promise" and "code behavior"**. "Silent pass must not be revived" at
  `checker.rs:5165` and "if a lower layer fails, the upper layers do not run" at
  `layers/README.md:3` are both **comment-level contracts** — the former is fulfilled by
  `assert_drained()`, the latter is exposed by the topology-driven `Skipped` diagnostic.
- **"Forgot to append to an array"**. Currently five functions each hand-write their own call
  sequence; after unification there's a single stage table, and missing a stage causes a compile
  error.

### Directions not adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — of the 11 inconsistencies, 7
  (the two implementations of dead-code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type bridging**; they are configuration questions of "whether
  to run a particular analysis." It also can't express `Aggregation`; it can only move the
  `check_module` / `check_module_collect_all` binary choice from the function layer to the type
  layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is
  equivalent to trading "each of five functions hand-writes its calls" for "an array with a missing
  append" — exactly the cause of the current 11 inconsistencies. There's already an in-repo example
  of this form: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all unimplemented.
- **Only fix the bug, don't change the structure** — can fix the 2 known bugs, but only covers 1 of
  the 16 obligation fields, with `Stage` still scattered across 5 functions, and the next divergence
  point will keep growing from here. **It must be done first** (it's part of S1/S3) because the
  structural refactor needs a known red test to prove the criterion works.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is dropped **as a whole**. This is looking for the bug in
  the wrong layer: the bug is in the orchestration layer, and the detection must be in the
  orchestration layer.

## Known Limits and Risks

- **The slim-down of `orchestrator.rs` will make it harder to read in the short term.** The 139-line
  `compile_project`, once split into "Program constructor + several driver arms", forces the reader
  to span two files to understand the flow. This is the common cost of any "centralize the wiring"
  refactor.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously-silent problems."** After the fix, there may be a batch of "new error" user reports —
  they are real, existing bugs that simply weren't being reported. This must be clearly stated in
  the CHANGELOG.
- **The severity switch of `assert_drained()` needs to be judged per field by hand.** Of the 16
  fields, some (such as `module_namespaces`) may have "no consumer" by design, and should not be
  reported. S4 needs to walk through each field; a one-size-fits-all approach is not acceptable.
- **The `Program` abstraction may be premature.** If the stage set of some entries is unstable
  long-term, `stages()` will degenerate into "pass a different array every call", and the contract
  constraint will be a dead letter. **The mitigation is `test_program_stage_coverage` asserting that
  `stages()` can only come from the 6 predefined combinations.**
- **Criterion dependency**: the vulnerability-fix criteria of S1/S3 depend on the vulnerability
  tests of the [equivalence criteria document](07-equivalence-oracle.md) going red first. **If that
  document does not first establish the multi-file corpus layer, S1 cannot be accepted** — because
  the existing 293 corpus files all take the single-file path and have zero coverage for this class
  of defect. This document does not cover the IR validator (`verify_loose`); that prerequisite work
  is out of scope.
- **Fixing the layer order will surface previously-silent `Unproven`** (the `equivalence` layer is
  not in the pipeline at all; `termination` and `ownership` are in the opposite of the declared
  order). This is a diagnostic-set change risk and should be done independently.
- **Thread safety after the SMT backend becomes a singleton is undetermined.** `backend.rs:13-19`
  already states that `Solver` is only `Send` and not `Sync`, and `Z3Backend`'s cache is `RefCell`;
  after switching to a cross-compile-unit shared singleton, all access paths must be confirmed to go
  through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typecheck is naturally
  parallelizable, but parallelization would **mask order-dependence defects** (such as the actual
  order dependence between termination and ownership). This should be opened only after the
  equivalence criteria are stable.

> **All open questions originally listed in this section have been ruled on.** See the per-item
> decision in [RFC-039 decision log](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction conventions,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (implementation steps owned by
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-layer criteria, the
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of the Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Cross-reference code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "silent pass must not
  be revived"
- `src/frontend/core/typecheck/layers/README.md:3` — Explicit declaration of "if a lower layer
  fails, the upper layers do not run" (no short-circuit in practice)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for the SMT
  initialization hard-fail
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "the caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream stages must
  consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's
  self-declaration of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — The corpus's
  self-declaration of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compile caller of the playground (single-file path, therefore
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate, not the main crate
- `build.rs:19-55` — Error code build-time gate, exemplar of this project's enforcement mechanism
