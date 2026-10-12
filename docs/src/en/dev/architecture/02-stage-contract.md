# Compilation Stage Contracts and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, the tiered acceptance criteria, and the stage execution order are in the body of
> RFC-039; the positioning of each subsidiary document is in [this directory's index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses a
single problem: **"which stages ran in this compilation, which didn't, and why" is currently
scattered across six paths and ten entry functions, each wired up by hand, with no single place that
can answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being broken by the architecture, not the logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally reads "Unproven → compile
error, no degradation, no silent pass... must not let silent pass be resurrected." But this
invariant only holds on the **single-file path**: proof-function constraints of the form
`x: Sorted(3)` **silently pass** under the three paths of multi-file `yaoxiang run`,
`yaoxiang check`, and LSP — the proof function is never executed, and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and how to refactor each of the ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction rules, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 tiers, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order of P1-P10 and G1-G10 acceptance gates → body of RFC-039

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactor and the **order** to do it in; this document gives L1's
**concrete form**, the **per-file change list**, and this document's **internal implementation
stages** (corresponding to the global sequence's P3 "fix correctness holes" and P4 "stage contracts
and unified Driver"). Where this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are applied under category **C2 (orchestration changes)** of the
[equivalence-criteria document](07-equivalence-oracle.md): the diagnostic set of each entry point is
identical + corpus behavior is identical.

## Current State

> **All items in this section are verified facts**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect of the "one place wrong, entire repo silently broken" kind

It differs in nature from the other two categories of defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation                  | Has a Signal?        |
| ----------------------- | -------------------------------------- | -------------------- |
| Lexical/syntax errors   | Source code is wrong                   | Has diagnostic       |
| Type mismatch           | Types are wrong                        | Has diagnostic       |
| Stage miss-wired        | Some stage is simply not called        | **No signal at all** |
| Obligation not consumed | Field is filled in but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentation convention, not as an executable constraint."

### Existing enforcement mechanisms in this repository

The same repository already has mature enforcement mechanisms — they just haven't been extended to
the stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subject to a
  **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, which compares each
  against the RFC-013 code table and any inconsistency causes a direct `panic!` refusing
  compilation.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` is 6227 lines / 47.9%) has a high
  test density, and the test subtree of every module is declared and wired up — this is the most
  complete test wiring in the entire repo, and can serve as a formal reference for the stage-layer
  gate.

**The design capability is sufficient. What is missing is "putting a same-tier gate in the
orchestration layer as well."**

(Line count basis: `(Get-Content).Count`, see the "Line count basis" section in
`06-cleanup-inventory.md`.)

### Six compilation paths, ten entry functions

| #   | Path                       | Entry function                                        | Location                                      | Which compilation                               |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages, direct calls                          |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + link             |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics     |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + separate IR           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project takes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1        |

Five stages of the single-file path (`pipeline.rs`):

| Stage               | Call site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded inside** (`381-389`) |

### 11 stage-coverage inconsistencies

The following table provides cell-by-cell evidence. **A blank cell does not mean "this entry point
does not do this thing" — it means "there is no code in this entry point that does this thing"** —
which is exactly the nature of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                      | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`                  |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------------ |
| 1   | **proof_execution**                                      | **yes** `187-203`                                | **no**                                                 | **no**                                                      | **no**                             | **no**                                     |
| 2   | Dead-code analysis                                       | yes `275-278` (`config.dead_code.enabled` gated) | **no**                                                 | yes `356-377` (role-aware, no config gate)                  | **no**                             | **no**                                     |
| 3   | W1006 local module shadowing                             | **no**                                           | **no**                                                 | yes `336-348`                                               | **no**                             | **no**                                     |
| 4   | W1003 unused import                                      | yes (`277` collects `type_result.warnings`)      | **collected but never emitted**                        | yes `350`                                                   | **no**                             | **no**                                     |
| 5   | W1001/W1002 dead-code family                             | yes (same as 2)                                  | **no**                                                 | yes (same as 2)                                             | **no**                             | **no**                                     |
| 6   | **Monomorphization**                                     | yes `381-389` (`config.mono.enabled` gated)      | **no**                                                 | not applicable                                              | not applicable                     | **no**                                     |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first-error return** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                    |
| 8   | File discovery                                           | not applicable                                   | `discover` (`101`, discards `used_by`/`shadow_events`) | `discover_with_used` (`275`)                                | `discover` (`454`)                 | not applicable                             |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **no**                                           | **no**                                                 | yes (`321-328`)                                             | **no**                             | **no**                                     |
| 10  | Global slot allocation                                   | not applicable                                   | yes `allocate_global_slots` (`147`)                    | **no**                                                      | **no**                             | **no**                                     |
| 11  | IR generation + qualified-name rewrite + linking         | yes (`205`)                                      | yes (`152-236`)                                        | **no**                                                      | **no**                             | yes (after independent ModuleIR is merged) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell check)**: This table is the diagnostic snapshot
> of the `9e02e4db` baseline and is preserved unchanged. Differences from current state after P3
> lands:
>
> - **Row 1 has been fixed**: proof_execution now shares the same implementation
>   `frontend/proof_execution.rs` across all five entry points (pipeline + four orchestrator
>   entries); the single-consumer defect no longer exists.
> - **Rows 2/4/5 remain unchanged** (multi-file `run` still does not emit W1003, does not run the
>   dead-code family) → filed under WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single-file) → filed under WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unproven).
> - **Row 7** `check_module` / `check_module_collect_all` dual entry → filed under 4.2.7
>   (Aggregation parameter-driven).
> - The "#434 ruling" cited by the code side had zero prior docs registration — now registered as
>   RFC-039 **D57**.
> - **Embedded std included in asymmetry (new fact outside the table, registered 2026-10-09)**: the
>   single-file path unconditionally injects `std.list` via `merge_embedded_std_ir` (required for
>   for-loop desugaring, hard switch at #117), but multi-file `discover` previously only recognized
>   explicit `use` — in-project for-loops compiled but raised E6006 at runtime (probe-based
>   evidence). This has been fixed and WBS 4.10.2 marked off; the same-family "multi-file missing a
>   single-file step" parse hard-abort quirk is filed under WBS 4.10.1 — **also already fixed**
>   (landed right after 4.2.2; the Check path degrades to per-file collection; from this audit
>   method's "stage coverage / field consumption" view, this is a compilation-unit membership
>   difference, a slipping-through dimension).
>
> - **Embedded std registration surface coverage (construction-time discovery at 4.2.5, already
>   fixed)**: the Registry arm repeatedly harvests registrations for embedded std units via
>   `extract_module_info`, which overwrites the native half already merged-registered by
>   `with_std()` (`result.is_err` etc. is lost → std.test falsely reports E1043). This audit's third
>   slipping-through dimension: "merge semantics of two registration sources for the same module
>   key" — the single-file path's registry is shaped only once through `with_std()`;
>   multi-file/Check's Registry arm inserts unit by unit and is the only one that has this coverage
>   surface.

**Two places require precise wording, otherwise they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138` — `type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but the function **has no read site of `result.warnings` anywhere in its body**; the
  only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **The `main` criteria for the row 11 entry also come from different sources**:
  `check_project:385-395` uses `surfaces.bins` (the surface declared by the manifest);
  `compile_project` uses `is_bin_role` (`250-252`, only checking "is there a manifest"). The two
  functions give different answers to "which files must define `main`".

### Correctness hole: proof obligations silently dropped (full evidence chain)

**This is the core of this document. All eight steps below can be verified.**

**Step 1 — The single production point of obligations.**
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

The semantics is "the predicate argument of this refinement constraint is a compile-time literal,
and this proof function must actually be executed to decide it."

**Step 2 — The consumption point is inside the checker, but it only emits, never consumes.** There
are **three isomorphic branches** in `src/frontend/core/typecheck/checker.rs` handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter-refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **emits no diagnostic** |

The comment at `checker.rs:5165-5169` reads verbatim:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real line of defense, not forward-looking makeshift: must not let silent pass be resurrected.`

That is: the author knows that "producing no diagnostic" is a defect, classifies it explicitly as a
**known, homeless intermediate state**, and assumes by design that "someone will definitely come to
read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it, and `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is **read in only one place in production code: `src/frontend/pipeline.rs:187`**
(passed at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier across the repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are the production-side collection;
`verdict.rs:61` is a same-named field on `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entry points bypass that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), `compile_embedded_module` (`1374`) in
`src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**; they call
`TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore they cannot even reach the
sole read point at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also take the drop
path.** `compile_embedded_module` (`1374`, the `check_module` call is at `1386`) is responsible for
compiling the embedded std. This means the **proof obligations of the embedded std modules
themselves are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and produces no diagnostic** under the
multi-file / check / LSP three paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-drop point inside the layer.** When
`checker.rs:1303-1314` handles the ownership layer's result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the `5164` branch's promise of "must not let
silent pass be resurrected", and is **independent of the orchestrator problem** — even if the entry
layer is completely fixed, this point will still remain silent. (Handling timing: should be
processed in the same batch as the obligations ledger, because it is the same kind of problem as the
obligations ledger.)

### Why the tests didn't catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three unit tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example `rfc027_return_refinement.rs:350-359` asserts the existence of a
`SumUpTo` call in `result.proof_calls`. But they **all go via `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping precisely before `pipeline.rs`**.

The test file's own doc-comment already states this. From `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so cases of refinement violation live in the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation is filled in", while
the bug is "the consumer never reads it".** A test that only tests the producer and not the consumer
is naturally immune to this kind of defect.

### A stronger finding: the primary corpus of the equivalence criteria is the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) takes the end-to-end differential over
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion**, the main acceptance
means for category C2 stages. But empirical testing shows:

- Under `tests/` there is **no `yaoxiang.toml`** (zero hits via directory-wide glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files go through the single-file path, all of them cover
  proof_execution, and none of them cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is precisely the
empirical evidence for this. The file is annotated `// expect: compile-error E4018` (line 15), and
its header comment at line 7 says "Status: ❌ should be rejected at compile time" — **it passes
precisely because it goes through the one path that actually runs the proof function**.

**Conclusion: the third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation Points"
S1.

### Obligation fields: 16 "produced-as-contract" fields, zero mechanism guarantees them

Among `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, **16 fields** have
doc-comments that explicitly say "produced by stage X → consumed by stage Y", i.e. they are in
essence **cross-stage obligations**:

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

**All these fields are passed across layers keyed by `Span` or name table.** The
equivalence-criteria document has already pointed out that any mismatch in the span-keyed contract
"silently fails, with no error whatsoever" (`ReleasePlan` is typical: any inconsistency in the way
the two sides compute spans → `Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan` is `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), located inside
  `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it** — the consumer lives in a downstream module, and that downstream module
  sits on the unavoidable path of every entry point.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **a
  different entry implementation**. The four orchestrator entry points never go through this
  `pipeline.rs` layer.

**Verified fact**: the entire repository currently has **no mechanism whatsoever** that guarantees
these 16 fields are consumed. The only "protection" is that `ReleasePlan` happened to piggyback on
the IR generation.

### Four additional contract defects of the proof layer

The following all come from the proof-layer analysis, are independent of the entry-fanout problem,
but all fall under "stage contracts are not enforced".

**(a) The declared layer order contradicts the actual execution order, and the `equivalence` layer
is not in the pipeline at all.**

The layer order declared by `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "executed in layer order, lower-layer failure prevents upper-layer from running".
The actual call sites in `TypeChecker::check_module`:

| Actual order | Call site                                         | Corresponding declared layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** relative to the declaration; the
declared Layer 0 `equivalence` **does not appear at all in the stage sequence of `check_module`**
(it is only used as an `is_subtype` helper by `inference/assignment.rs:17`, unrelated to
`ProofResult`). Meanwhile **there is no short-circuit at all** — `checker.rs:1271-1276` `add_error`s
every termination error and keeps going, so the README's promise of "lower-layer failure prevents
upper-layer from running" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic has been eliminated by 3.3.1 of P3 (SOLVER
> slot made Option, missing gracefully degraded as `SMTResult::Unknown`); "silently skipped (not
> injected)" is supplemented with a W1081 signal by 3.4.2. Unification of the three forms
> (singleton-ization) is performed per **RFC-039 D58**, with the actual fix location at
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure access).
> The `default_solver() → &'static` formulation in the 02 change list is governed by D58 (`&'static`
> raw reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition strategy                                     | When solver is unavailable                                |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction via `with_solver_owned`         | `None` → **silently skipped** (not injected)              |
| `ownership.rs:627` (back-edge cutoff decision)         | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "initialization failure remains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". And the contract doc at `backend.rs:60` says "`None`: backend unavailable……**caller
should conservatively degrade**". **Two philosophies coexist in the same code, and the hard-failure
one panics instead of returning an error.**

**(c) Every back-edge decision creates a new Z3 context, the cache is non-functional.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff decisions. And at
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
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc-comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifetime strategies.)

**(d) The checker silently degrades + the comment does not match reality.** `checker.rs:1283-1287`
and `1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check receives an **empty type ledger and an empty call
table** and runs as usual — ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` claims termination check "runs after type-checking and before
constraint solving". In reality `self.env.solver().solve()` is at `checker.rs:1320` — **termination
at `1256` and ownership at `1296` come after, i.e. the "before" in the comment is actually "after
two layers"**.

### wasm current state: carried by a shim crate, the 27-file branch is alive

The wasm target **is built and is built in CI**; the `cdylib` is not in the main crate but in an
independent shim crate.

| Fact                                               | Evidence                                                                                                                                                                                                                                                                             |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Main crate is `rlib` only                          | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                   |
| **Shim crate provides `cdylib` + `wasm-bindgen`**  | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                   |
| Shim depends on the main crate (rlib) as a library | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                          |
| Main crate has a wasm target dependency section    | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                      |
| Shim directory is excluded from the main workspace | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                             |
| **CI has 4 wasm builds**                           | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacks into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten   | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from a fixed URL, degrades to warning on miss)                                                                                                                                                                  |
| **27 files** contain the literal `wasm32`          | of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in docs                                                                                                                                          |
| `orchestrator.rs` has 20 occurrences               | 12 attributes + 8 comments                                                                                                                                                                                                                                                           |
| `lib.rs` has 11 occurrences                        | all attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                           |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs in the main crate can be called by `wasm/src/lib.rs` (73 lines) under the
wasm target — `lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` as a whole
(all three need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — the **single-file path**.
Therefore:

| Path that consumes `proof_calls`           | Runs the proof function |
| ------------------------------------------ | ----------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                 |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                 |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                 |
| Multi-file `run` (`compile_project`)       | **No**                  |
| `check` (`check_project`)                  | **No**                  |
| LSP in-project (`check_source_in_project`) | **No**                  |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file-path side. The `Driver`'s `ProgramKind` must add a
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — goes to `06-cleanup-inventory.md`. The "build wasm target or delete these
branches" open question in RFC-039 **is now expired**: the target has been built.)

---

## Target Design

### 1. Stage model: `Stage` enum (exhaustive, not runtime-registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can compile-time-enforce "a new stage must be handled
by the orchestration layer" — exactly the mechanism the opcode table has already proven effective
(RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check               Project
    Discovery,          // File discovery                        Project
    Parsing,            // Lexical + syntax                      PerModule
    Registry,           // Module-registry construction          Project
    RoleClassification, // Role classification (Script/Bin/...)   Project
    Typecheck,          // Type check (with embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis             Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation                Project
    IrGeneration,       // AST → ModuleIR                        PerModule
    Monomorphization,   // Monomorphization                      Project
    Linking,            // Cross-module linking / IR merge       Project
}

impl Stage {
    /// All stages. When adding a new variant, both this array and the exhaustive
    /// match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: making `PerModule` (run once per compilation unit) and `Project`
(run once at project level) into a type-level fact of "which stages must run per module and which
must run once at project level". A `Project`-scope stage is structurally guaranteed to run only once
in `dispatch` — this eliminates the kind of question that currently requires human reasoning in
`orchestrator.rs`, such as "should `allocate_global_slots` be called once per file".

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft,
> corrected per implementation evidence —
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the initial draft's order
>    contradicts the data flow.
> 2. The Check variant's stage table is normalized under `Stage::ALL`'s topological order to dead
>    code **before** proof: `check_project`'s current state is proof first then dead code; the two
>    have no data dependency and produce the same diagnostic set (C2 set semantics); only the
>    in-file diagnostic order is normalized. The single-file path (dead code embedded in typecheck,
>    before proof) already follows the ALL order; byte-level acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    both signature collection (`extract_module_info`) and `ast_has_main` consume AST products — the
>    initial draft's order would force the Registry arm to "hide parse", making the stage table lie
>    about data flow; and multi-file parse drops from 2 to 1. Attached is an honest current-state
>    registration: multi-file path parse errors used to be a hard abort (`build_registry_from`'s `?`
>    propagation, its tension with CollectAll semantics was registered as WBS 4.10.1) — **4.10.1 has
>    been fixed** (2026-10-09 user ruling: rust-style collection semantics + Plan B): Check path
>    parse failure degrades to per-file diagnostic collection, sick files exit the compilation unit
>    (don't enter the registry, importers report E5001); MultiFile path retains hard abort (under
>    FailFast a bad file cannot produce IR, which is correct semantics, the pin is permanently
>    valid).
> 4. Check variant landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a.
>    Data-dependency edges 14→16: `RoleClassification` consumes Discovery's `used_by` edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events — the initial draft omitted them
>    (these two products were produced by `discover_with_used` in the orchestrator era and discarded
>    by the `discover` wrapper; the data flow was lost before it entered the table); b. In-file
>    diagnostic order normalization extended to the full form of note #2: E3020 entry-point check
>    moves into the RoleClassification arm (Check has no Linking stage, the arm holding `surfaces`
>    and AST is responsible for the check); the in-file diagnostic order equals the stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is
>    unchanged (C2 set semantics); only the stderr-entry order changes.

### 2. Obligations ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field is produced but no one consumes it" from undetectable into a
compile-time or test-time-assertable fact. RFC-039 has already listed this as "the single most
valuable item".

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
    /// Each unconsumed field produces a W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                   | Criterion                                                   | Disposition                                                                       |
| ------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Obligation is read by its declared consumer | The field is `take()`-en / marked consumed in `Obligations` | Pass                                                                              |
| Obligation is non-empty but has no consumer | Field is non-empty and not consumed                         | **Produce diagnostic** (W-level by default; promoted to E-level in `strict` mode) |
| Obligation is empty                         | Field is empty                                              | Pass (no consumption needed)                                                      |

**Why W-level first, not E-level**: fixing obligation drops will **change the diagnostic set**. C2
requires "the diagnostic set of each entry point is identical"; if we jump straight to E-level,
`yaoxiang check` will suddenly surface a large amount of previously-silent `Unproven` diagnostics.
Splitting into two steps (W observation first, then E promotion) makes every step's criterion
usable. See "Implementation Points" S4.

**The sole call site of `assert_drained()`**: `Driver::run` at the tail of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appearing ≥2 times without a third-file reader → fail). `assert_drained()` is a **runtime** gate;
that script is a **static** gate; the two complement each other and both are needed.

### 3. Unified Driver: single `dispatch`

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
    /// Must come from one of the 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the above 6 predefined combinations (one per `ProgramKind`), and
does not accept the caller passing in a free array.** This is the key constraint that prevents the
`Program` abstraction from degenerating into "a free parameter that can be anything", enforced by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topologically decides whether to skip, not by stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one fewer
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: four implementation deviations
> from the above sketch, semantically equivalent, logged for reference —
>
> 1. The three-state `StageOutcome` of §4 is not returned by the stage arm; it is expressed by
>    `State`'s failure flag + `Aggregation` gating (Continue / Warn / Abort semantics unchanged,
>    removing the per-arm boilerplate return);
> 2. `Driver` has no `config` field — the sole source of config is `Program.config` (the §5 "avoid
>    Driver holding mutable global state" determination covers the sketch field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded in `DriverOutcome.skipped`'s internal ledger,
>    not externally emitted — S2 "deliberately fix no bug" zero-diff criterion requires this;
>    external emission comes with the 4.3 obligations ledger;
> 4. `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is an allowed direction); orchestrator's existing call sites will be relocated
>    after 4.2 and revisited.
> 5. Under the multi-file variant, ProofExecution is an independent stage, executed after **all**
>    typechecks (A1, 2026-10-09 user ruling): the original `compile_project` interleaves proof
>    within the per-file typecheck loop (file N's proof comes before file N+1's typecheck). For a
>    single failure, the two are byte-for-byte identical; in the multi-failure scenario of "file 1's
>    proof fails + file 2's typecheck fails", the first reported error changes from a proof error to
>    a typecheck error (test pinboard).
> 6. `DriverOutcome` carries products on separate channels by `ProgramKind`: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error
>    contract (`PipelineError` / `OrchestratorError`) is not a type surface the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec — this is `check_project`'s
>    current contract); Discovery's product is extended to a "file set + used_by edge set + shadow
>    event" triple into State (MultiFile variant has no downstream consumer, only recorded, not
>    externally emitted); the Check path's per-file parse drops from 2 to 1 along with the C3
>    topology.
> 8. Lsp variant landing (4.2.3, split by file source, 2026-10-09 user decision): disk-file parse
>    failure follows Check's Plan B (collect + exit compilation unit — the two ends converge in
>    degradation); edited-buffer parse failure retains a partial AST and continues to typecheck
>    (editor philosophy — a mid-typing state should not make semantic features disappear). The
>    Discovery arm reserves the buffer source for Lsp to cover stale disk content; Typecheck under
>    Lsp only checks the target file (other units only serve the registry for signatures); in-file
>    diagnostic order normalization extends to LSP (typecheck diagnostics → W-code warnings → proof
>    errors). The old LSP behavior of hard-aborting on irrelevant disk-file parse errors and
>    silently falling back to the single-file path in the handler is fixed by this step.
> 9. Embedded variant landing (4.2.4): `Program` adds a new `shared_registry` field — the embedded
>    std module is a sub-compilation; the registry is an **input**, not a product (EMBEDDED_STAGES
>    has no Registry stage); #94's SymbolTable-sharing contract is thereby made explicit as a
>    program declaration instead of "the caller remembers to pass the same registry". The error-path
>    text is normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only
>    the compiler's internal error surface, no test pinning). At this point, all four orchestrator
>    entries are migrated into the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): the Check
>     variant is the faithful bearer of single-file semantics for a program with no project root —
>     the warning surface only covers the entry file (neighboring files only receive errors, prefer
>     under-reporting to false-positives); relative `use` resolves along the importer's directory
>     (rustc single-file mod alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: the
>     standalone old path (the full pipeline chain) already runs IR generation, and only the ir_gen
>     stage produces E3019/E1014/E1015 (runner-gate evidence). The Check variant of IrGeneration is
>     pure-check, not consuming IR; Script/Bin variants are expressed by module_key's None/Some (the
>     existing switch of E3023, ir_gen.rs:1660). Monomorphization does not enter CHECK (it only
>     produces resource-exhaustion/internal errors, zero corpus dependency). **〔2026-10-10
>     withdrawn per 3.4.8 evidence〕** — "zero corpus dependency" turns out to be evidence-blindness
>     from the corpus having no pathological-recursion fixtures; E3005 is the sole compile-time
>     defense for this class of program. See note #14. Side-effect fix of a latent defect: the
>     Registry arm's repeated harvesting of registrations for embedded std units overwrites the
>     native half already merged-registered by `with_std()` (false-positive E1043) — embedded units
>     now skip the duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, replaced by
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     SingleFile+CollectAll variant: Parsing collects all parse errors, retains partial AST and
>     continues to typecheck (editor philosophy, extended from 4.2.3 ruling; not marking stage
>     failure, otherwise the topological skip would prevent typecheck from ever running); Typecheck
>     dispatches the `check_module_collect_all` free function. From this point on, the LSP
>     single-file path fills in proof (E4018), W-code warnings (W1001–W1005, test pinboard) and
>     IR-level errors; lexical failures switch to reporting real diagnostics (the old synthesized
>     "E0001 lexical error" text disappears with the deleted sequence — real diagnostics carry
>     precise spans, with no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries have long since converged internally — the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` (single bool), the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, with pass-3 and drain running in both modes, the difference being
>     carried by whether `collected_errors` is empty; statements.rs has five collection points
>     (function body/use/for/block/while). The verification confirmed and fixed a
>     duplicate-diagnostic defect: the collection point puts the first error in both
>     `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err` causes the
>     same code, same span, same message × 2 (nested × 3). There are also two independent
>     mechanisms: annotation-check signature parameters vs. whole-annotation double-visiting
>     (E1003/E1103), and the ownership layer emitting twice at the same point (E2014/E2018). The fix
>     landed at the module-result boundary: dedup by (code, span, message) — a diagnostic is a
>     positional fact, and duplicates carry no information; 63 corpus entries have duplicate copies
>     removed, the `run` column and exit code have zero drift (the "duplication" in
>     `borrow_conflict_err` is an artifact of the baseline triple format not including the column
>     number — the two E2018 are actually legitimate diagnostics of different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` constructs
>     `Program { kind: WasmPlayground }` + Driver via `compile_playground` — the `Compiler` wrapper
>     no longer passes through. Error text is byte-for-byte aligned with the old `CompileError`
>     Display prefixes (Parse error:/Type error:/Internal error:), with zero visible difference in
>     the playground UI. WasmPlayground and SingleFile share `SINGLE_FILE_STAGES` + FailFast (the
>     4.1 existing declaration, and from this step it has a real producer).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, 2026-10-10 user ruling,
>     **partially withdraws the mono exclusion of note #10**). Empirical refutation of "mono only
>     produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) makes the
>     compiler process stack-overflow (0xc00000fd — the interpreter recurses in the Rust layer, with
>     no graceful runtime error) on the multi-file path without mono; the mono depth gate is the
>     only compile-time defense; the standalone check, before 4.2.5, walked the full pipeline chain
>     and had mono, and the exclusion ruling caused a silent coverage regression ("zero corpus
>     dependency" only proves the corpus has no pathological fixtures). The "type-erasure fallback"
>     assumption is invalidated by the RFC-033 reflection ruling — `^^List(Int)` needs the real
>     identity of the instantiation, and monomorphization is the foundation of reflection, not an
>     optional optimization. Four design points: ① typecheck produces `instantiation_request` for
>     qualified calls (`lib.f(x)`, FieldAccess form); `generic_id` uses the qualified name, sharing
>     the namespace with merged IR (currently only bare Var is recognized, expressions.rs:2455); ②
>     `Stage::ALL` topology is changed to Linking before Monomorphization, and `Monomorphization` is
>     added to the tail of MULTI_FILE — it consumes merged_ir, aggregates all-units requests (the
>     `containing_fn` of the deferred bucket is qualified), and mono upgrades from a single-module
>     pass to a whole-program pass; the single-file stage table is unchanged (no Linking,
>     subsequence property preserved, IR snapshot zero-drift); ③ CHECK adds Linking (pure merge —
>     E3020 entry check stays in RoleClassification to prevent double-reporting) and
>     Monomorphization (pure-check not consuming, same as the Check variant of IrGeneration) as two
>     stages; ④ resource protection covers cross-unit mutual recursion along with the whole-program
>     BFS. The Monomorphizer body is unchanged (the contract of consuming ModuleIR + request set is
>     exactly the merged-IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per the #14 design, qualified-call instantiation
>     collection: `ExpressionInferrer` injects the module-qualified key table (`use` three-arm
>     registration — regular / alias / per-item alias, and only SubModule-class exports),
>     `callee_generic_name` is concatenated with the qualified name (same source as merged IR) via
>     `SymbolTable::qualify`; the first-path arity criterion of qualified calls is tightened (when
>     the parameter list cannot be resolved, fall back to the second path to take the real arguments
>     by signature, avoiding cross-module same-name function mismatch false-positive E3018). During
>     implementation, **three asymmetric defects** on the mono side were empirically found and fixed
>     in the same round: (1) The granularity of the delete key (set of generic names) and the
>     rewrite key (`containing_fn` + name + span triple) did not match — sites that cannot resolve
>     real arguments degrade to symbol requests entering the deferred bucket; when the containing
>     function is non-generic, the bucket never drains, while the original has already been deleted
>     by name → dangling call (empirically E6006 at runtime on `list_ops.yx:53`); the fix is the
>     restoration gate `restore_generics_with_uncovered_call_sites`, which uses
>     `build_call_site_map` as the single source of truth to determine coverage, and the originals
>     of uncovered sites are preserved. (2) The recovery segment's `HashSet` iteration order causes
>     the function table to be in non-deterministic order (t2 review F1) → changed to `Vec` and
>     sorted by name, with byte-level reproducible output. (3) The non-rewritten-surface forms
>     TailCall / MakeClosure originally looked up map keys as usual (the comment claimed
>     conservative) → `call_form_is_rewritten` single-point determination; non-rewritten forms are
>     counted as uncovered. Each is accompanied by a pin test (red state pre-fix proves
>     discriminatory power). Corpus differential baseline regenerates exactly one line with the same
>     submission (the `remove_at` frame adds `(int64)`). **Commit-strategy note**: when typecheck
>     collection and mono fix are split into two commits, the staging tree with only the former
>     triggers a red on `stdlib_docs`'s pre-existing HEAD defect (the same-family E6006 on
>     `list.is_empty([])`) — the two halves are each other's green-light prerequisite, so they are
>     combined into one atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling-α** — the user-module
>     export carries `generic_fn_type_params_snapshot()` along with `Export.type_params`, isomorphic
>     to the std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source
>     for qualified-call monomorphization, and signature-inference degrades to a fallback (folded
>     into the R2 scope). (2) **HKT declared-side alive, call-side broken** (inferring type
>     arguments E1002 / explicitly giving type arguments E1010) is registered as WBS 3.5 (3 level-3
>     tasks). (3) **The deferred bucket never drains under non-generic containers** is folded into
>     the acceptance sub-item of 3.4.8 ②: the bucket must drain or report loudly, never silently
>     fail to specialize. (4) **Closure variables called by name produce runtime E6006** is
>     registered as WBS 3.6; semantics (i) support indirect calls / (ii) reject at compile time,
>     **(i) is already ruled** — support indirect calls: the `CallDyn` mechanism already exists
>     (`ir_gen.rs:7945` closure-expression path); what is broken is that the `Var` callee is
>     degraded to a name-based `Call`, and typecheck already accepts this program. (5) **Trade-off
>     criteria solidified** — the reason for rejecting a plan can only be correctness and
>     readability; "large change surface / new mechanism / high cost" does not constitute a
>     rejection reason; this is filed in part 2 of `coding-rules.md` and the HOWTO self-check list;
>     trade-offs needing user decisions follow `AGENTS.md` "When requesting user ruling (mandatory)"
>     and provide all four items (situation + evidence / per-option pros-and-cons / clear
>     recommendation + cost / destructive-call-out).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② is landed per the #14
>     design: `Stage::ALL` is changed to Linking before Monomorphization, MULTI_FILE table tail
>     appends mono; the Linking arm **aggregates the whole-program request and qualifies
>     `containing_fn` before taking each unit's IR** (otherwise the deferred-bucket key and merged
>     IR are not from the same source, and the bucket never drains). **① closure (D2 ruling-α)**:
>     the user-module export carries the declared-period type-parameter snapshot along with
>     `Export::type_params`, isomorphic to the std path (`yx_sources.rs:62/108`). **Existing defects
>     empirically found and fixed during the process**: (a) **F1** — `qualify_names` only rewrites
>     function bodies, `ir.init` is never visited; under multi-file, function definitions are
>     qualified but module top-level calls remain bare names → runtime E6006 (**reproduces with
>     non-generic**, unrelated to generics); fixed by making init reuse the same `rename`/`aliases`
>     and the same `rewrite_call_names`. (b) The mono-side "whole-program unique suffix" heuristic
>     and unhit-silent `continue` → delete the heuristic, and the unhit becomes a **loud E3018**
>     (silent ambiguous mismatch is worse than not parsing). **Independently verified regression,
>     found and fixed**: t6 finds that after a named import `use lib.{pick}`, the bare-name call is
>     rejected by E3018 (IR instruction name is the alias-qualified name, the request name is the
>     bare name, and the aggregation side only qualifies by unit key) → fixed by having the
>     aggregation side **directly call ir_gen's** `import_aliases_from` and `resolve_call_target`
>     (extracted as a single pub point, one producer one consumer), with a matrix pin test covering
>     both forms of named import. **Filed away**: the empty `DefId` of the init segment is
>     empirically a "consistency blank" rather than a live defect (codegen name fallback is
>     resolvable, four kinds of init calls pass actual measurement), registered as WBS 3.7.4 to
>     close the divergence between "function body uses def, init relies on names".
> 18. init-segment DefId divergence closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     the `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), and
>     the function body and `ir.init` share the same implementation and the same `resolve` closure;
>     after regenerating 208 IR snapshots only 309 `(def=_)`→`(def=dN)` pairs change (other-lines 0;
>     entire tree init segment 0 residual `(def=_)`). The two states of codegen's DefId main path
>     and name fallback produce identical translation results and execution values item-by-item
>     (four kinds of init calls measured), the corpus baseline has zero drift; the probe is flipped
>     to the new criterion and strengthened (identity + precise index + execution value, falling
>     back to init coverage causes red). **Erratum (2026-10-11)**: the above commit numbers were
>     rewritten via `git pull --rebase` — the original b554497b/e60dcc87/b7ac7396 correspond to
>     post-rebase 375789d2/d922a551/38bd4b9e in sequence; the pushed dev history takes the new
>     numbers (the old numbers are local temporary, never entered the remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (Design basis of ③④ in #14; ②
>     landing in #17, this item is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregating requests + `link_module_irs`), the entire entry-point check is left to
>     `RoleClassification`; `ir_generation_check` generates and retains `us.ir` (memory only); mono
>     failure in Check enters the collection channel, in MultiFile retains `fail_with`;
>     `into_outcome` first does `take_check_diagnostics` then judges failure (structural failure no
>     longer clears already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-variant matrix) and one round of re-review (t6) all pass;
>     corpus differential 340/340 line-by-line IDENTICAL, IR snapshot zero-drift, test count
>     2643→2656. **Existing gaps found and fixed in the same round**: F1 (rootless multi-file check:
>     request name and IR definition key from different sources → silently fail to specialize, see
>     WBS 3.4.8 row's landing note) — fix is to have the aggregation side precisely look up by the
>     real definition key (non-heuristic), the first-version fix is to have the aggregation side
>     precisely look up by the real definition key (`flat_defs`); **same-day erratum**: the bridge
>     was deleted in the regression fix, the root cause is instead solved by decoupling
>     `module_key`'s dual use (`script_mode`), the three sources converge to `{unit-key}.{name}`.
>     The regression's location and fix (CI docs-example gate 258 blocks 10 failures → 0 failures)
>     are detailed in WBS 3.4.8 row's landing note. **Filed away (existing, not this round's
>     regression)**: E3005 help text does not match the resource-protection semantics; multi-file
>     E3005's span label is misaligned (HEAD binary reproducible); the position dependence of the
>     type-parameter in the request collection surface (same surface goes to 3.5).

**Refactor approach for the ten entry points (one function per row)**:

| Entry                                                      | After refactor                                                                                                                                                                                                     |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the body, change to constructing `Program { kind: SingleFile, … }` and hand off to Driver                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                            |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                    |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                      |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a single Driver call + result filter to target file                                                                                                                                                      |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                          |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                             |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                                    |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project goes to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence of the single-file branch is deleted**, replaced by `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | The `standalone` branch (`614-616` → `check_single_file` `623-661`) is deleted, unified to `Program { kind: Check }`                                                                                               |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% diverged)**.
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`, the
resolution loop, per-file checker assembly, result handling. Diverged: items 2/3/4/5/7/8/9/10/11 of
the 11-row table above. **This 40% shared skeleton is exactly the value range of `Driver`** — the
diverged 60% is entirely "different stage set" or "different aggregation mode", and these are
exactly what `Program`'s two fields can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causing this stage to be skipped" is not a stage return
value.**

Reason: a skip is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` on its own, the "why didn't I run" information is scattered across 12 stages and cannot be
centrally audited. Change to having `dispatch` decide at the head of the loop based on the
dependency relationship:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the hole this document is fixing
— the current `pipeline.rs:187`'s `if !proof_calls.is_empty()` is a silent "skip": it produces no
diagnostic at all, just because the obligation happens to be empty. Once the rule is established,
any "didn't run Y because X was not done" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text direction                                                                                                                |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort`                                        | "Stage Y not executed due to upstream stage X failure"                                                                        |
| Prerequisite obligation empty                           | "Stage Y not executed because no obligation pending (normal)" — **this item's severity = Info, not counted in warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y not executed because configuration is not enabled"                                                                   |

The second category is key: it makes "normal skip" distinguishable from "abnormal skip" in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on the non-blocking contract of this count).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                             | Current correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early returns)              | Exists                 |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                 |

**`Aggregation` must be a field of `Program`, not a global Driver setting** — LSP serves both
in-project and single-file files within the same process, while CLI's `run` and `check` are two
independent calls; placing them on Program avoids the Driver holding mutable global state.

**Note the current semantic difference between the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just a "whether to
early-return" difference — they are two different checker entry points. After unification, the same
implementation should be driven by the `Aggregation` parameter, rather than keeping two functions
(**this is an additional item this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-layer impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Does not introduce new type representations, does not touch `MonoType` / `PolyType` / `ir::Type`                                           |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what migrates into the ledger is the **settlement
responsibility**, not the type's home module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the rest of the field types each
stay where they are; `driver/obligations.rs` only holds the aggregate container and
`assert_drained()`. L3 consumers (e.g. `ir_gen` reading `release_plan`) continue to receive specific
field types via parameters, **and must not `use crate::driver`** — consistent with the
no-reverse-dependency red line of driver below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not `use crate::driver` in
reverse**. Any `Driver` appearing in `TypeChecker`'s import is considered a violation, blocked by
the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                                     |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already plugged in at P3; **warning surface does not expand** (Decision B1, 2026-10-09: dead-code-family warnings do not enter the compile path — pool semantics are being corrected as a defect by 4.9, normalize warning surface deferred until after 4.9; the original draft of this row "newly added warning output" is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in at P3                                                                                                                                                                                                                                                                                                                          |
| LSP (in-project)                    | No proof_execution                       | Already plugged in at P3                                                                                                                                                                                                                                                                                                                          |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**behavior change, see compatibility**)                                                                                                                                                                                                                                                                  |
| Z3 not installed + multi-file       | Silently skipped                         | Same, unified                                                                                                                                                                                                                                                                                                                                     |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8's positioning of "not bound to a specific solver", and is
also the contract already declared at `backend.rs:60` ("caller should conservatively degrade") — the
current implementation contradicts its own contract document.

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

| File                                               | Line range                         | Change                                                                                                                                                                                                                                                                                                                                                        |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                         |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead-code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` and serves as the `Stage::ProofExecution` implementation                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` rerouted through Driver                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The two-way choice between `check_module` and `check_module_collect_all` in `typecheck_with_registry_in` is driven by the `Aggregation` parameter                                                                                                                                                                                                             |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence is deleted, replaced by Driver call                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                               |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                                |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | The `ProofResult::Unproven { .. } => {}` empty arm changed to produce a diagnostic (**fix the second silent-drop point**)                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds a warning diagnostic                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct the comment that does not match `1320`                                                                                                                                                                                                                                                                                                                |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer-order table changed to **actual** execution order; delete "lower-layer failure prevents upper-layer from running" (no short-circuit exists)                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add a process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access; the three consumer points in `predicate.rs`/`checker.rs`/`ownership.rs` switch to the shared access. The `&'static` raw-reference form is infeasible (`dyn Solver` is not `Sync`); the original text is governed by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branches' logic itself
is correct — **the problem is on the consumer side, not the producer side**; changing the producer
side would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch
(reachability determination of wasm branches goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                               | Disposition                                                                                                                                               |
| --------------------------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously-silently-passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **written red first, then fixed**; release in batches, expected diagnostic-set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that previously compiled start producing warnings    | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code does not change                                 |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set newly added                                    | W first, then E, S2/S4 in two steps                                                                                                                       |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                          | No break                                                                                                                                                  |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                               | These two paths do not go through proof_execution                                                                                                         |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                       | If the `pub` API surface has external dependencies, they need to be synced; in-repo consumers are all listed in the change list                           |

---

## Implementation Points

This document corresponds to **P3 (fix correctness holes)** and **P4 (stage contracts and unified
Driver)** in RFC-039's global stage sequence. The following S1-S5 are the implementation order
internal to this document; **the acceptance criterion for each step cites category C2 of the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration changes: each entry's
**diagnostic set** is identical + corpus behavior is identical).

### S1: Establish the criterion (red first)

**Prerequisite, cannot be skipped.** The hole criterion must be written to fail first.

| Deliverable                                                                                                                                                      | Acceptance                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red**. If it accidentally is green, the hole analysis of this document needs to be re-verified |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (Resolution D48 — not mixed into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of single-file and multi-file versions of the corpus are comparable                      |
| `test_obligations_drained` skeleton                                                                                                                              | Marked `#[ignore]`, turned green in S4                                                                   |

**Rollback point**: no code changes, purely new tests.

### S2: Introduce `Stage` + `Program`, do not change behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-for-byte identical**                       |
| Full corpus (293 `.yx` files) differential               | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately does not fix any bug** — it only moves existing behavior into the Driver.
The acceptance standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, restore `pipeline.rs` to its original state.

### S3: Merge the four orchestrator entry points + fix the hole

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The four entry diagnostic sets are identical after normalization by `Aggregation`   |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in and out of a project                       |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` turned green**                                                                  | Hole is fixed                                                                       |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in a Z3-less environment                                           |

**Rollback point**: hole fix and structural merge are **in two separate commits**. If the structural
merge has issues, only the structural commit can be rolled back while keeping the hole-fix commit —
at this point `test_multifile_proof_obligation_not_dropped` stays green; the reverse (keeping the
structure, rolling back the fix) turns red, is an unacceptable intermediate state, and is forbidden
from merging.

### S4: Enable the obligations ledger

| Deliverable                                  | Acceptance                                                          |
| -------------------------------------------- | ------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turned green                             |
| `scripts/ci/check-obligations.py`            | Static gate is in effect                                            |
| Obligation diagnostic W → E upgrade          | Manual review of every newly added E after full corpus differential |

**Rollback point**: the severity of `assert_drained()` can be controlled by a config item; the W/E
switch does not require changing the code structure.

### S5: Proof layer and wasm wrap-up

| Deliverable                                                                                                                                                                                                                           | Acceptance                                                                                                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                | Doc and `checker.rs` call sites correspond item by item                                                                                                                           |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter the cache** — otherwise a single timeout is frozen across compilations via the process-level cache, polluting cargo-test thread-shared) | Production-path `default_solver()` call sites drop to zero (only tests retained); cache hit rate is observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                       | A diagnostic exists when there is no `body_checker`                                                                                                                               |
| Reachability determination of the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                             | Conclusion goes to `06-cleanup-inventory.md`; this document only registers the determination need                                                                                 |

**Note**: correcting the layer order will change the diagnostic set and may surface a large amount
of previously-silent `Unproven`. **This item should be taken independently of S1-S4**, not mixed
with entry-merge in the same PR.

## Key Decisions and Rationale

| Decision                 | Decision                                                                                                                                                            | Rationale                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **runtime registration rejected**                                                                             | An exhaustive `match` is a zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism already proven effective in RFC-039 routing table B. `StageScope` further turns "per-module or project-level" into a type fact, eliminating the call-count question that requires human reasoning                                      |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, used together with the **static** gate of `scripts/ci/check-obligations.py`; severity first W then E | What we are fixing is a whole class of bugs, not one: currently at least 2 of the same kind of latent issues have been verified (`proof_calls`, `checker.rs:1313`), and all 16 span-keyed fields are within range. W first then E is to make every step's C2 criterion usable — jumping to E in one step will suddenly surface a large amount of previously-silent diagnostics for `yaoxiang check` |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the six `Unproven` branches' producer logic at `checker.rs:5164/5179/5306/5318/5420/5448`         | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                              |

This design additionally closes two kinds of cracks:

- **The crack between "comment promises" and "code behavior"**. Both `checker.rs:5165`'s "must not
  let silent pass be resurrected" and `layers/README.md:3`'s "lower-layer failure prevents
  upper-layer from running" are **comment-level contracts** — the former is honored by
  `assert_drained()`, and the latter is exposed by topology-driven `Skipped` diagnostics.
- **"An array someone forgot to append to"**. Currently, five functions each hand-write their own
  call sequence; after unification there is a single stage table, and a new stage that fails to be
  wired up will error at compile time.

### Not-Adopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — Of the 11 inconsistencies, 7
  (two implementations of dead-code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type bridging**; they are configuration questions of "whether
  to run a particular analysis"; it also cannot express `Aggregation`, and can only move the
  `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — Cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is just
  "an array someone forgot to append to" replaced with "one array", exactly the cause of the current
  11 inconsistencies. The same repository already has a same-form counter-example: the
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs, do not touch the structure** — Fixes the known 2 bugs, but only covers 1 of the
  16 obligation fields, `Stage` is still scattered across 5 functions, and the next fork will
  continue to grow from here. **It must be done first** (it is part of S1/S3), because structural
  refactoring needs a known-red test to prove the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a general entry; it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **discarded as a whole**. This is looking for the bug at the wrong
  layer: the bug is in the orchestration layer, and detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slim-down will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to span
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously-silent issues"**. After the fix, there may be a batch of "new error" user reports —
  they are real bugs, just never reported. This must be clearly stated in CHANGELOG.
- **The severity switch of `assert_drained()` needs per-field manual judgment**. Among the 16
  fields, some (e.g. `module_namespaces`) "no consumer" may by design not need consumption, and
  should not warn. S4 needs to go through them one by one; a one-size-fits-all approach is not
  appropriate.
- **The `Program` abstraction may be premature**. If some entry's stage set remains unstable for a
  long time, `stages()` degenerates into "different arrays per call" as a free parameter, and the
  contract constraint falls through. **Mitigation is `test_program_stage_coverage` asserting that
  `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's hole-fix criterion depends on
  [equivalence-criteria document](07-equivalence-oracle.md)'s hole test being red first. **If that
  document does not first establish the multi-file corpus layer, S1 cannot be accepted** — because
  all 293 existing corpora go through the single-file path, with zero coverage for this kind of
  defect. This document does not involve the IR verifier (`verify_loose`); that part of the
  prerequisite work is not within this document's scope.
- **Layer-order correction will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` are in the opposite order from the declaration).
  This is a diagnostic-set change risk and should be taken independently.
- **Thread-safety after singleton-izing the SMT backend is undetermined**. `backend.rs:13-19`
  already states that `Solver` is only `Send` not `Sync`, and `Z3Backend`'s cache is `RefCell`;
  after changing to a cross-compilation-unit shared singleton, the access path must be confirmed to
  all go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). It should be opened after the equivalence criteria stabilize.

> **All open questions originally listed in this section have been ruled on.** Per-item decisions
> are in [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup; the
  `include!` refactor of `checker/semantic_tokens.rs` (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria, the
  `test_multifile_proof_obligation_not_dropped` hole criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — The
  cross-reference code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "must not let
  silent pass be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — The explicit declaration of "lower-layer
  failure prevents upper-layer from running" (in reality no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit reason for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's own
  admission of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — The corpus's
  own admission of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path,
  therefore runs proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen are in the shim crate, not the main crate
- `build.rs:19-55` — The error-code build-time gate, an example of forced mechanisms in this project
