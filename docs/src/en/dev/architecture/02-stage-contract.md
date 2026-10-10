# Compilation Stage Contracts and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039: Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, the grading of acceptance criteria, and the order of execution phases appear in
> the RFC-039 body; the positioning of each subsidiary document appears in the
> [directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint of RFC-039. It addresses one
question: **"which phases did this compilation run, which did it not, and why" is currently
distributed across six paths and ten entry functions with manual wiring in each, and there is no
single place that can answer it.**

The current state can be summarized in a single sentence:

> **An invariant that the code itself declares is being violated by the architecture rather than by
> the logic.**

The original comment at `src/frontend/core/typecheck/checker.rs:5165` reads: "Unproven → compile
error, no degradation, no silent pass……the silent pass must not be revived." But this invariant only
holds on the **single-file path**: refinement constraints of the form `x: Sorted(3)` silently pass
on the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP, the proof function is
never executed, and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- The `Obligations` ledger and the `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry-function refactor plan for all ten
  entry functions;
- Phase failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of Scope

- The four-layer model, dependency direction specification, anti-regression gate → `01-routing.md`
- Equivalence criteria (C1–C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1–P10 global execution order and G1–G10 acceptance gates → RFC-039 body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and **in what order** to do it; this document gives the
**specific form of L1**, the **per-file change checklist**, and the **implementation phases within
this document** (corresponding to RFC-039's global sequence P3 "Fix correctness vulnerabilities" and
P4 "Stage contracts and unified Driver"). Where this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are executed per the **C2 (orchestration change)** category of the
[Equivalence Criteria Document](07-equivalence-oracle.md): the diagnostic set per entry is the
same + corpus behavior is the same.

## Current State

> This section is **all verified facts**, each with a file path + line number. Line numbers are
> based on `9e02e4db`.

### Phase Boundaries Are the Only Structural Defect of "One Miswrite, Whole Repo Silent"

It is qualitatively different from the other two types of defects (module boundaries, test wiring):

| Defect                      | Typical Manifestation                    | Has a Signal?            |
| --------------------------- | ---------------------------------------- | ------------------------ |
| Lexical/syntactic error     | Source code is wrong                     | Has diagnostic           |
| Type mismatch               | Type is wrong                            | Has diagnostic           |
| **Phase not wired**         | A phase is simply never called           | **No signal whatsoever** |
| **Obligation not consumed** | A field is filled in but no one reads it | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentary convention rather than an executable constraint."

### Mandatory Mechanisms Already Present in This Repository

The same repository already has mature mandatory mechanisms; they just have not been extended to the
phase layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, which compares each entry
  against the RFC-013 code table, and any inconsistency directly `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13,012 lines**, of which `tests/` contains 6,227 lines / 47.9%) has
  high test density; every module's test subtree is declaratively wired — this is the block with the
  most complete test wiring in the entire repo, and can serve as a formal reference for the
  phase-layer gate.

**The design capability is sufficient. What is missing is "placing an equally strong gate at the
orchestration layer as well."**

(Line-count convention: `(Get-Content).Count`; see the "Line-count convention" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation It Runs                       |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ----------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                            |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking          |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics     |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)              |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + standalone IR         |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1               |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                   |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project takes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                   |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1        |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                              |
| ------------------- | ------------- | --------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                    |
| parsing             | `161`         | `run_parsing`                                                               |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                                 |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                       |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization is embedded within it** (`381-389`) |

### 11 Phase-Coverage Inconsistencies

The table below gives evidence cell by cell. **A blank does not mean "this entry does not do this
thing," but rather "there is no code in this entry that does this thing"** — that is the essence of
the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                               |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                               |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                               |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never output**                            | Yes `350`                                                   | **No**                             | **No**                               |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                               |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                               |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                  |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                               |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                               |
| 11  | IR generation + qualified-name rewrite + linking         | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (standalone ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell check)**: this table is the diagnostic snapshot
> of the `9e02e4db` baseline; it is kept unchanged. Differences in current state after P3 lands:
>
> - **Row 1 has been fixed**: proof_execution now shares the same implementation
>   `frontend/proof_execution.rs` across five entries (pipeline + four orchestrator entries); the
>   sole-consumer defect is gone.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not output W1003 and does not run
>   the dead-code family) → attributed to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization exclusive to single-file) → attributed to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk not proven as a defect).
> - **Row 7** `check_module` / `check_module_collect_all` dual entry → attributed to 4.2.7 (driven
>   by Aggregation parameter).
> - The "Ruling #434" referenced in the code had zero prior registration in docs — it has now been
>   back-registered as RFC-039 **D57**.
> - **Embedded std incorporated into the asymmetry (new fact outside the table, back-registered
>   2026-10-09)**: the single-file path unconditionally injects std.list via `merge_embedded_std_ir`
>   (required for for-loop desugaring, hard switch at #117); the multi-file `discover` previously
>   only recognized explicit `use` — for loops inside a project compiled fine but emitted runtime
>   E6006 (probed). This has been fixed and WBS 4.10.2 struck off; the same-family "multi-file is
>   missing the single-file step" parse hard-abort quirk is filed under WBS 4.10.1 — **also fixed**
>   (landed right after 4.2.2; the Check path degrades to per-file collection; from this audit's
>   "phase coverage/field consumption" perspective, this is a compile-unit membership difference and
>   is a missed dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   the Registry arm repeatedly harvests registration for embedded std units via
>   `extract_module_info`, clobbering the native half-face that `with_std()` had already merged
>   under the "native + yx surface merge" rule (`result.is_err` etc. lost → std.test falsely reports
>   E1043). This audit's third missed dimension: the merge semantics of "two registration sources
>   for the same module key" — the single-file path's registration table is shaped only once via
>   `with_std()`; the multi-file/Check Registry arm inserts unit by unit, which is what gives it the
>   surface coverage.

**Two cells require precise wording, otherwise they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report them. The
  reason is at `compile_project:138` — `type_results.push(result)` stores the complete
  `TypeCheckResult` (including `warnings`), but this function **has no read site for
  `result.warnings` anywhere in its body**; the sole downstream use of `result` is
  `generate_ir_with_context` (`154-156`).
- **Row 11's entry `main` criterion is also non-cosourced**: `check_project:385-395` uses
  `surfaces.bins` (the manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`,
  only checking "is there a manifest"). The two functions give different answers to "which file must
  define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is a unique obligation generation point.**
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

The semantics: "the predicate argument of this refinement constraint is a compile-time literal; this
proof function must actually be executed to decide it."

**Step 2 — The consumption site is inside the checker, but it only produces and does not consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                            | Location                                                                                                     | Behavior                  |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------- |
| Formal-parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **No diagnostic emitted** |
| Call-site argument check          | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |
| Return-position obligation        | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real defensive line rather than a forward-looking fallback: the silent pass must not be revived.`

That is: the author knows that "producing no diagnostic" is a defect, and explicitly classifies it
as a **known, nowhere-to-go intermediate state**, by design assuming that "someone will come and
read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects into it; `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligations`. The field is defined at
`types.rs:29`.

**Step 4 — The repository has only one read site.** `TypeCheckResult.proof_calls` (`types.rs:29`) is
read in production code at **only one place: `src/frontend/pipeline.rs:187`** (passed as an argument
at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other occurrences of the `proof_calls` identifier across the repository fall into three categories,
none of which is a consumer of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-name field on `ProofResult`; `tests/rfc027_*.rs` read `ProofResult`.)

**Step 5 — The four orchestrator entries do not pass through that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`) **do not call
`pipeline.rs`**; they directly call `TypeChecker::check_module` (`124` / `493` / `1386`). Therefore
they do not even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also go through
the discarded path.** `compile_embedded_module` (`1374`, the `check_module` call is at `1386`) is
responsible for compiling the embedded std. This means that **the proof obligations of the embedded
std module itself are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) on the three paths of multi-file / check / LSP **compiles,
runs, and produces no diagnostic**. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layer.**
`checker.rs:1303-1314` handles ownership-layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership-check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the "the silent pass must not be revived"
promise of the `5164` branch, and is **independent of the orchestrator problem** — even if the entry
layer were completely fixed, this point would still be silent. (Handling timing: should be processed
in the same batch as the obligations ledger, because it is the same class of problem.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the keywords
`Sorted` / `proof` / `refin`.** The multi-file path's proof obligations have zero coverage.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty — for example, `rfc027_return_refinement.rs:350-359` asserts that `SumUpTo` is present in
`result.proof_calls`. But they **all go through `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), and **happen to stop
right before `pipeline.rs`**.

The test file's own documentation comment has already stated this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. The proof call (E4018) is executed by pipeline.rs after check_module, so the refinement-violation cases are in .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation was filled in", and
the bug is "the consumer end did not read it".** A test that only tests the producer side and does
not test the consumer side is naturally immune to this class of defect.

### A Stronger Finding: The Main Corpus of the Equivalence Criteria Is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) takes the end-to-end differential of
293 `.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion**, the main acceptance
tool for the C2 phase. But empirical observation:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero matches by full-directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is exactly the
empirical evidence for this. That file is annotated `// expect: compile-error E4018` (line 15), and
its header comment on line 7 says "Status: ❌ should be rejected at compile time" — **it passes,
precisely because it takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation Points"
S1.

### Obligation Fields: 16 "Produce-as-Contract" Fields, Zero Mechanism Guarantees Them

Among the 16 fields in `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, the
documentation comments **explicitly write "produced by stage X → consumed by stage Y"**, meaning
they are essentially **cross-stage obligations**:

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

**All of these fields are passed across layers keyed by `Span` or name tables.** The Equivalence
Criteria Document has already pointed out that a span-keyed contract mismatch will "silently fail
with no error whatsoever" (`ReleasePlan` is typical: any inconsistency in how the two sides compute
spans → `Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), is inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the
  consumer is in a downstream module, and the downstream module is on the must-pass path of every
  entry.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry implementation**. The four orchestrator entries do not pass through the `pipeline.rs` layer
  at all.

**Verified fact**: the repository currently has **no mechanism whatsoever** that ensures these 16
fields are consumed. The only "protection" is that `ReleasePlan` happens to ride along with IR
generation.

### Four Additional Contract Defects in the Proof Layer

All of the following come from analysis of the proof layer, are independent of the entry-fork
problem, but all belong to the category "phase contracts are not enforced."

**(a) The layer-order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared by `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "executed in layer order, the upper layer does not run when a lower layer fails."
The actual call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **reversed** relative to the declaration; the
declared Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all**
(it is only used by `inference/assignment.rs:17` as the `is_subtype` utility function, unrelated to
`ProofResult`). At the same time, **no short-circuit exists** — `checker.rs:1271-1276` adds
termination errors one by one via `add_error` and then continues, so the README's promise of "the
upper layer does not run when a lower layer fails" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-failure panic has been eliminated by P3's 3.3.1 (the SOLVER
> slot made `Option`, missing cases degrade conservatively to `SMTResult::Unknown`); "silently
> skipped (not injected)" has been supplemented with the W1081 signal by 3.4.2. The unification of
> the three forms (singleton-ization) is executed per **RFC-039 D58**, with the actual fix location
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure port).
> The `default_solver() → &'static` wording in the 02 change list is governed by D58 (the bare
> `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | Solver Unavailable                                          |
| ------------------------------------------------------ | -------------------------------------------------------- | ----------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**   |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Injected at construction via `with_solver_owned`         | `None` → **silently skipped** (not injected)                |
| `ownership.rs:627` (back-edge cutoff decision)         | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservatively not cut off |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "initialization failure stays
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'". But the contract documentation at `backend.rs:60` says "`None`: backend
unavailable……**the caller should degrade conservatively**". **Two philosophies coexist in the same
codebase, and the hard-failure one panics instead of returning an error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff decision. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
// backend.rs:67-72
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call does `Z3Backend::new()`, i.e., creates
a new Z3 context. The cache field of `Z3Backend` (`cache: RefCell<HashMap<u64, SMTResult>>` at
`proof/smt/z3_backend.rs:20`) is **per-instance**, while the documentation comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(Comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. The same backend, two
lifecycle strategies.)

**(d) Checker silently degrades + the comment does not match the actual behavior.**
`checker.rs:1283-1287` and `1289-1293`:

```rust
// checker.rs:1283-1293
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership analysis receives an **empty type ledger and an empty call
table** and runs as normal — ownership analysis degenerates to "nothing conflicts", **with no
warning whatsoever**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at
least leave a trace in the obligations ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving". In fact, `self.env.solver().solve()` is at `checker.rs:1320` — **termination is
at `1256`, ownership is after `1296`, i.e., the "before" the comment speaks of is actually "two
layers after"**.

### Wasm Current State: Carried by the Shim Crate, 27 Files with Active Branches

The wasm target **has been built and is built in CI**; the `cdylib` is in a separate shim crate, not
the main crate.

| Fact                                               | Evidence                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                          | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                    |
| **Shim crate provides `cdylib` + `wasm-bindgen`**  | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                    |
| Shim depends on the main crate (rlib) as a library | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                           |
| Main crate has the wasm target dependency section  | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                       |
| Shim directory is excluded from the main workspace | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                              |
| **CI has 4 wasm builds**                           | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten   | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from a fixed URL; downgrade to warning when missing)                                                                                                                                                                |
| **27 files** contain the literal `wasm32`          | Of which **25** carry an actual `#[cfg(...)]` attribute; the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are mentioned only in documentation                                                                                                                               |
| `orchestrator.rs` 20 occurrences                   | 12 attributes + 8 comments                                                                                                                                                                                                                                                            |
| `lib.rs` 11 occurrences                            | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                            |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call in the wasm target
— `lib.rs:139/153/170` wholly gate `run_file` / `run_project` / `build_bytecode` (all three need
`std::fs`), while the shim takes a different path.

**But this brings a fact directly related to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — the **single-file path**.
Therefore:

| Path Consuming `proof_calls`               | Does It Execute the Proof Function? |
| ------------------------------------------ | ----------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                             |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                             |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                             |
| Multi-file `run` (`compile_project`)       | **No**                              |
| `check` (`check_project`)                  | **No**                              |
| LSP in-project (`check_source_in_project`) | **No**                              |

That is: **the existence of `wasm/` changes "proof_execution has one consumer" into "it has
three"**, but those three are all on the single-file-path side. `Driver`'s `ProgramKind` must add a
new `WasmPlayground` variant (see "Target Design" §3), otherwise unifying the Driver will miss this
path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is attributed to `06-cleanup-inventory.md`. The RFC-039 open question "build
wasm target, or delete these branches" **is now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time exhaustive enum, and runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can enforce at compile time "a new stage
must be handled by the orchestration layer" — exactly the mechanism that the opcode table has
already proven effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check        Project
    Discovery,          // File discovery                   Project
    Parsing,            // Lexical + syntactic              PerModule
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…)  Project
    Typecheck,          // Type check (with embedded proof layer)  PerModule
    DeadCodeAnalysis,   // Dead-code family analysis        Project
    ProofExecution,     // Proof function compile-time execution  PerModule
    GlobalSlotAlloc,    // Global slot allocation           Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Monomorphization,   // Monomorphization                 Project
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When a new variant is added, this array and the
    /// exhaustive `match` of `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compile unit) versus `Project` (run once
at the project level) makes "which stages must run per module, and which must run once at the
project level" a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only
once in `dispatch` — this eliminates the kind of "should `allocate_global_slots` be called once per
file or not" question that currently requires human reasoning in `orchestrator.rs`.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on the order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the first draft corrected
> per implementation evidence —
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs); the order in the first draft contradicts
>    the data flow.
> 2. The Check variant's stage table, per the `Stage::ALL` topological order, is normalized to dead
>    code **before** proof: `check_project` currently runs proof before dead code, the two have no
>    data dependency, the diagnostic set is the same (C2 set semantics); only the in-file diagnostic
>    order is normalized. The single-file path (dead code embedded in typecheck, before proof)
>    already conforms to the ALL order, and byte-level acceptance is unaffected.
> 3. `Parsing` is moved **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    first-draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; and multi-file parse thus drops from 2 to 1 time. The current state is
>    also truthfully recorded as a side note: the multi-file path's parse error used to be a hard
>    abort (the `?` propagation of `build_registry_from`, the tension with CollectAll semantics is
>    filed as WBS 4.10.1) — **4.10.1 has been fixed** (2026-10-09 user ruling on Rust-style
>    collection semantics + Plan B): Check path parse failure degrades to per-file diagnostic
>    collection, the sick file exits the compile unit (does not enter the registry, the importer
>    reports E5001); MultiFile path retains hard abort (under FailFast a bad file cannot produce IR,
>    which is correct semantics, long-term pinning is valid).
> 4. Check variant landing (4.2.2, 2026-10-09 user ruling) adds two more registrations: a.
>    Data-dependency edges 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadow events — the first draft missed these (in
>    the orchestrator era these two products were produced by `discover_with_used`, discarded by the
>    `discover` wrapper; the data flow was lost when not entered into the table); b. The in-file
>    diagnostic order normalization is extended to the full form of note #2: the E3020
>    entry-validation is moved into the RoleClassification arm (Check has no Linking stage, the arm
>    holding surfaces and AST is responsible for validation), the in-file diagnostic order = stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is
>    unchanged (C2 set semantics), only the order of stderr entries changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but not consumed" from undetectable into a compile-time or
test-time-assertable fact. RFC-039 has already listed this as "the single most valuable item."

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
    // … remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                | Criterion                                                   | Disposition                                                               |
| ---------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------- |
| Obligation read by its declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                      |
| Obligation non-empty but no consumer     | Field is non-empty and not consumed                         | **Produce diagnostic** (`W` level by default; `E` level in `strict` mode) |
| Obligation is empty                      | Field is empty                                              | Pass (no need to consume)                                                 |

**Why W level first, not E level**: fixing obligation discard will **change the diagnostic set**. C2
criterion requires "the diagnostic set per entry is the same"; if we jump straight to E level,
`yaoxiang check` will suddenly emit a large amount of previously silenced `Unproven` diagnostics.
Splitting into two steps (observe W first, then promote to E) lets every step's criterion be usable.
See "Implementation Points" S4.

**`assert_drained()` has a single call site**: `Driver::run`, at the tail of the stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (a field appears ≥
2 times but no third file reads it → fail). `assert_drained()` is the **runtime** gate, the script
is the **static** gate; they are complementary, and both are needed.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, filter result to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
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

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); the
caller is not allowed to pass an array freely.** This is the key constraint that prevents the
`Program` abstraction from degenerating into "anything goes" free parameters, asserted by
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
                state.record_skipped(stage);   // must produce a diagnostic, see below
                continue;
            }
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // the single settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: four implementation deviations
> from the sketch above, semantically equivalent, registered for reference —
>
> 1. The three-state `StageOutcome` of §4 is not returned by the stage arm, but expressed by the
>    `State` failure mark + `Aggregation` gate (the `Continue` / `Warn` / `Abort` semantics are
>    unchanged, removing the boilerplate return per arm);
> 2. `Driver` has no `config` field — the sole source of configuration is `Program.config` (§5
>    "Avoid Driver holding mutable global state" ruling covers the sketch field);
> 3. The `Skipped` diagnostic in 4.1.3 is only recorded in the internal `DriverOutcome.skipped`
>    ledger, not emitted outward — the zero-diff criterion of S2 "deliberately not fixing any bug"
>    requires this; emission is deferred to 4.3 with the obligations ledger;
> 4. The visibility of the `proof_execution` module is widened to `pub(crate)`: the driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the orchestrator's existing call sites
>    move with 4.2 and will be reassessed after that.
> 5. In the multi-file variant, ProofExecution is an independent stage, executed **after all**
>    typecheck (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof inside
>    the per-file typecheck loop (file N's proof before file N+1's typecheck). Under single failure
>    the two are byte-for-byte identical; under the multi-failure scenario "file 1 proof failure +
>    file 2 typecheck failure", the first-reported error changes from a proof error to a typecheck
>    error (pinned by tests).
> 6. `DriverOutcome` carries products through different channels per `ProgramKind`: `result`
>    (pipeline contract) / `module` + `failure` (orchestrator contract, 4.2.1) — each entry's
>    external error contract (`PipelineError` / `OrchestratorError`) is not a type surface that
>    Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry; clean files have an empty Vec — the current
>    `check_project` contract); the Discovery product is extended to a triple of "file set + used_by
>    edge set + shadow events" into State (the MultiFile variant has no downstream consumer, only
>    recorded not emitted); per-file parse in the check path drops from 2 to 1 time with the C3
>    topology.
> 8. Lsp variant landing (4.2.3, divided by file source, 2026-10-09 user ruling): disk file parse
>    failure follows Check's Plan B (collect + exit compile unit — both ends degrade and converge);
>    the edited buffer's parse failure retains a residual AST and continues typecheck (editor
>    philosophy — mid-typing states should not make semantic features disappear). The Discovery arm
>    overrides disk-stale content with the buffer source for Lsp; Typecheck under Lsp only checks
>    the target file (other units are only for registry signature supply); the in-file diagnostic
>    order normalization extends to LSP (typecheck diagnostic → W-code warnings → proof errors). The
>    old behavior's quirk in which LSP hard-aborted on unrelated disk-file parse failures and the
>    handler silently fell back to the single-file path is fixed by this step.
> 9. Embedded variant landing (4.2.4): a new `shared_registry` field is added to `Program` — the
>    embedded std module is a sub-compilation, the registry is an **input** rather than a product
>    (EMBEDDED_STAGES has no Registry stage), so the SymbolTable-sharing contract of #94 is now
>    expressed as a program declaration rather than "the caller remembers to pass the same
>    registry". Error path text is normalized from `<std.test> (embedded std)` to the unit virtual
>    path `<std/test>` (only the compiler's internal error surface, no test pinning this). At this
>    point all four orchestrator entries have been migrated into Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): the Check
>     variant faithfully carries the single-file semantics for programs without a project root — the
>     warning surface only covers the entry file (neighboring files only receive errors, better to
>     miss than to misreport); relative `use` resolves along the importer's directory (aligned with
>     rustc single-file mod). The CHECK stage table adds `GlobalSlotAlloc` + `IrGeneration`: the old
>     standalone path (the full pipeline) already runs IR generation, and E3019/E1014/E1015 etc. are
>     only produced in ir_gen (runner-gate empirical). The Check variant of `IrGeneration` only
>     checks and does not consume IR; Script/Bin is expressed by the `None`/`Some` of `module_key`
>     (the existing E3023 switch, `ir_gen.rs:1660`). `Monomorphization` does not enter CHECK (only
>     produces resource-exceeded/internal errors, zero corpus dependency). **〔2026-10-10 withdrawn
>     after empirical evidence at 3.4.8〕** — the "zero corpus dependency" is actually an evidence
>     blind spot in the corpus's lack of pathological recursion fixtures; E3005 is the unique
>     compile-time defense for that class of programs. See note #14. Side fix for a latent defect:
>     the Registry arm's repeated harvest registration for embedded std units clobbers the native
>     half-face that `with_std()` had merged (E1043 false positive) — embedded units now skip
>     duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted and replaced by
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     SingleFile+CollectAll variant: Parsing collects all parse errors, retains a residual AST to
>     continue typecheck (editor philosophy extended from the 4.2.3 ruling; not marking stage
>     failure, otherwise the topology skip would make typecheck unreachable); Typecheck dispatches
>     the `check_module_collect_all` free function. The LSP single-file path now includes proof
>     (E4018), W-code warnings (W1001–W1005, pinned by tests) and IR-level errors; lexical failure
>     now reports the true diagnostic (the old synthetic "E0001 lexical error" text disappears with
>     the deleted sequence — true diagnostics carry precise spans, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory
>     item): the two entries had already each converged — the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` with a single bool, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`; pass-3 and drain run in both modes, the difference carried by
>     whether `collected_errors` is empty; the five collection points in statements.rs (function
>     body/use/for/block/while). Verification confirmed a duplicate-diagnostic defect and fixed it:
>     the collection point put the first error into both `collected_errors` and the `Err` return
>     channel, and pass-3's `add_error` for `Err` caused the same code, same span, same message ×2
>     (×3 when nested); plus two independent mechanisms: the annotation- validation signature
>     parameter and the whole annotation are visited twice (E1003/E1103), and the ownership layer
>     double-emits at the same point (E2014/E2018). The fix lands at the module-result boundary:
>     deduplicate by (code, span, message) — diagnostics are positional facts, duplicates carry no
>     information; 63 corpus entries have duplicate copies removed, the `run` column and exit code
>     have zero drift (the "duplicate" of `borrow_conflict_err` is an artifact of the baseline
>     triple lacking a column number — the two E2018s are actually legitimate diagnostics at
>     different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver — the `Compiler` wrapper layer no longer
>     handles anything. The error text is byte-aligned with the old `CompileError` Display prefix
>     (Parse error:/Type error:/Internal error:), the playground UI has zero visible difference.
>     WasmPlayground shares `SINGLE_FILE_STAGES` + `FailFast` with SingleFile (declared in 4.1, from
>     this step on it has a real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partially withdraws the mono exclusion in note #10**). Empirical evidence overturns "mono
>     only produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) causes the
>     compiler process to stack-overflow (0xc00000fd — the interpreter recurses in the Rust layer,
>     no graceful runtime error) on the multi-file path without mono, and the mono depth gate is the
>     only compile-time defense; standalone check, before 4.2.5, went through the full pipeline and
>     had mono, and the exclusion ruling caused a silent coverage regression ("zero corpus
>     dependency" only proves the corpus has no pathological fixture). The "type-erasure fallback"
>     assumption is voided by RFC-033's reflection ruling — `^^List(Int)` requires a real identity
>     for instantiation, and monomorphization is the foundation of reflection, not an optional
>     optimization. Four design points: ①typecheck emits an `instantiation_request` for a qualified
>     call (`lib.f(x)`, FieldAccess form), the `generic_id` uses the qualified name, sharing the
>     namespace with the merged IR (currently only bare Var is recognized, `expressions.rs:2455`);
>     ②the topology of `Stage::ALL` is changed to Linking before Monomorphization, and
>     Monomorphization is added to the tail of the MULTI_FILE stage table — it consumes `merged_ir`,
>     aggregates per-unit requests (the containing_fn of the deferred bucket is qualified), and mono
>     is upgraded from a per-module pass to a whole-program pass; the single-file stage table is
>     unchanged (no Linking, the sub-sequence property holds, the IR snapshot has zero drift);
>     ③CHECK adds both Linking (pure merge — E3020 entry-validation stays in RoleClassification, to
>     prevent double reporting) and Monomorphization (pure check, does not consume, same as the
>     Check variant of IrGeneration); ④resource protection covers cross-unit mutual recursion with
>     the whole-program BFS. The Monomorphizer body is unchanged (the contract of consuming
>     ModuleIR + request sets is exactly the merged-IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per design #14, the qualified-call instantiation
>     collection was landed: `ExpressionInferrer` injects a module-qualified key table (three arms
>     of `use` registered — normal / alias / per-item alias, and only SubModule-type exports),
>     `callee_generic_name` is concatenated via `SymbolTable::qualify` to form a qualified name with
>     the same source as the merged IR; the qualified-call first-path arity criterion is tightened
>     (when the parameter list cannot be resolved, fall back to the second path and take the actual
>     argument by signature, to avoid cross-module same-name function mismatch falsely reporting
>     E3018). Three asymmetric defects on the mono side were empirically discovered and fixed in the
>     same round: (1) granularity mismatch between the deletion key (generic-name set) and the
>     rewrite key (`containing_fn` + name + span triple) — sites where the actual argument cannot be
>     resolved degrade to a symbol request that enters the deferred bucket; when the container
>     function is non-generic, the bucket never drains, while the original item has already been
>     deleted by name → dangling call (empirically `list_ops.yx:53` runtime E6006); the fix is the
>     restore-gate `restore_generics_with_uncovered_call_sites`, which uses `build_call_site_map` as
>     the single source of truth to determine coverage, and the originals at uncovered sites are
>     preserved. (2) The restore segment's `HashSet` iteration order causes the function-table order
>     to be non-deterministic (t2 review F1) → switched to `Vec` and sorted by name, the product is
>     byte-level reproducible. (3) The non-rewritten-surface form TailCall / MakeClosure originally
>     looked up the mapping key (the comment claims to be conservative) → `call_form_is_rewritten`
>     is the single point of decision, the non-rewritten form is always counted as uncovered. Each
>     of the three has a paired pin test (red-state empirical discriminating power before the fix).
>     The corpus differential baseline regenerated exactly one row against the same commit (the
>     `remove_at` frame supplements `(int64)`). **Commit-strategy correction**: when the typecheck
>     collection and the mono fix were split into two commits, the staging tree containing only the
>     former triggered `stdlib_docs` red on a pre-existing defect in HEAD (the `list.is_empty([])`
>     same-family E6006) — the two halves are each other's green-light prerequisite, so they are
>     merged into one atomic commit.
> 16. 2026-10-10 five-ruling registration (after R1 closure): (1) **D2 ruling A** — user-module
>     exports carry `generic_fn_type_params_snapshot()` via `Export.type_params`, isomorphic to the
>     std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified-call monomorphization, and signature inference is downgraded to a fallback (folded
>     into R2 scope). (2) **HKT declaration side alive, call side broken** (inferring type argument
>     E1002 / explicitly giving type argument E1010) is filed as WBS 3.5 (3 level-3 tasks). (3)
>     **the deferred bucket never drains under a non-generic container** is folded into acceptance
>     sub-item 3.4.8 ②: the bucket must drain or report loudly, silent non-specialization is not
>     allowed. (4) **closure variable by-name call runtime E6006** filed as WBS 3.6; semantics (i)
>     support indirect call / (ii) compile-time reject pending user ruling. (5) **Trade-off
>     criterion formalized** — a plan can only be rejected on grounds of correctness and
>     readability, "large change surface / introducing a new mechanism / high cost" is not a
>     rejection reason, this goes into part two of `coding-rules.md` and the HOWTO self-checklist;
>     trade-offs that need the user's decision go through `AGENTS.md` "When requesting user ruling
>     (mandatory)" with all four items (situation + evidence / per-option pros and cons / explicit
>     recommendation + cost / destructive naming).

**Ten Entry Function Refactor Plan (function-by-function specification)**:

| Entry                                                      | After Refactor                                                                                                                                                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body, change to constructing `Program { kind: SingleFile, … }` and handing to Driver                                                                                                                  |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                   |
| `compile_project` (`orchestrator.rs:99-237`)               | Slimmed to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                             |
| `check_project` (`orchestrator.rs:273-399`)                | Slimmed to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                               |
| `check_source_in_project` (`orchestrator.rs:450`)          | Changed to one Driver call + filter the result to the target file                                                                                                                                                         |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Changed to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                                |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                                    |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                                           |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project: changed to `Program { kind: Lsp, aggregation: CollectAll }`; the **manual lex→parse→… sequence in the single-file branch is deleted** and replaced by `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | The `standalone` branch (`614-616` → `check_single_file` `623-661`) is deleted and unified to `Program { kind: Check }`                                                                                                   |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% forked)**. The
shared part: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
the resolution loop, per-file checker assembly, result handling. The forked part: items
2/3/4/5/7/8/9/10/11 of the 11-item table above. **This 40% shared skeleton is exactly the value
range of `Driver`** — the forked 60% is all "different stage sets" or "different aggregation modes",
which is exactly what two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // stage succeeded, continue
    Abort,     // stage failed, terminate this compilation
    Warn,      // stage has non-blocking issues, continue
}
```

**Key design ruling: "an upstream failure causes this stage to be skipped" is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let every stage
return `Skipped` itself, then the "why didn't I run" information is scattered across the 12 stages
and cannot be centrally audited. Instead, the dispatch loop determines it at the beginning of the
loop based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
is fixing — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" that
produces no diagnostic, just because the obligation happens to be empty. Once the rule is
established, any "because X was not done, Y was not run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                  | Text Direction                                                                                                                             |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Caused by upstream `Abort`                              | "Stage Y did not run because upstream stage X failed"                                                                                      |
| Caused by empty prerequisite obligation                 | "Stage Y did not run because there was no pending obligation (normal)" — **this item's severity = Info, not counted in the warning count** |
| Condition not met (e.g. `config.mono.enabled == false`) | "Stage Y did not run because the configuration is not enabled"                                                                             |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // abort on first error (compile path)
    CollectAll,  // collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                                   | Current Correspondence |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` returns `OrchestratorError::TypeCheck` on first error), `Pipeline::run` (early return at `pipeline.rs:150/162/174/193`)                    | Already exists         |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` does not early-return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists         |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single-file files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early return or not" —
they are two different checker entries. After unification, the same implementation should be driven
by the `Aggregation` parameter, rather than retaining two functions (**this is an item that this
document needs to additionally verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation; does not touch `MonoType` / `PolyType` / `ir::Type`                                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what is migrated into the ledger is the **settlement
responsibility**, not the type-ownership module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the other field types each stay
where they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`.
L3 consumers (e.g. `ir_gen` reading `release_plan`) continue to receive concrete field types through
parameters, and **must not `use crate::driver`** — this aligns with the no-reverse-dependency red
line of driver below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. A `Driver` appearing in `TypeChecker`'s imports is a violation, intercepted
by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Refactor                          | After Refactor                                                                                                                                                                                                                                                                                                                                                                  |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                                                                   |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution has been stop-bled and plugged in by P3; **the warning surface does not gain new items** (ruling B1, 2026-10-09: dead-code-family warnings do not enter the compile path — the pool semantics are being corrected as a defect by 4.9, normalizing the warning surface is left until after 4.9; the original draft's "new warning output" of this row is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already stop-bled and plugged in by P3                                                                                                                                                                                                                                                                                                                                          |
| LSP (in-project)                    | No proof_execution                       | Already stop-bled and plugged in by P3                                                                                                                                                                                                                                                                                                                                          |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                                      |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                                                                          |

**The only intentional behavior break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver", and is
the contract already declared at `backend.rs:60` ("the caller should degrade conservatively") — the
current implementation is the opposite of its own contract documentation.

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead-code analysis moved out, changed to the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                          |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **retained**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` changed to go through Driver                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The two-way choice between `check_module` and `check_module_collect_all` in `typecheck_with_registry_in` is driven by the `Aggregation` parameter                                                                                                                                                                                                                   |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | The manual stage sequence in `run_diagnostics` is deleted and replaced by a Driver call                                                                                                                                                                                                                                                                             |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                                 |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                     |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently goes through `Compiler::compile_with_source` and implicitly lands on path 1)                                                                                                                                                                                         |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | The empty `ProofResult::Unproven { .. } => {}` arm changed to produce a diagnostic (**fix for the second silent-discard point**)                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | The `unwrap_or_default()` degradation path adds a warning diagnostic                                                                                                                                                                                                                                                                                                |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix the comment that contradicts `1320`                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | The layer-order table is changed to the **actual** execution order; remove "the upper layer does not run when a lower layer fails" (no short-circuit exists)                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add a process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumers in `predicate.rs` / `checker.rs` / `ownership.rs` go through the shared port. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); the original wording is governed by D58 |

**Deletion**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is on the consumer side, not the producer side**; changing
the producer side would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (reachability determination of wasm branches is attributed
to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                       | Compatibility                                                                   | Disposition                                                                                                                                              |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New proof_execution on the multi-file path                   | **Breaking**: constraints that previously silently passed will now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first** and then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| New W1001/W1002/W1003 on the multi-file path                 | **Breaking**: programs that previously compiled will now produce warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code does not change                                |
| First run of `assert_drained()` produces W-level diagnostics | **Breaking**: new diagnostic set                                                | W first, then E; S2/S4 split into two steps                                                                                                              |
| `predicate.rs` panic changed to diagnostic                   | **Improvement**: no longer crashes                                              | No break                                                                                                                                                 |
| `build` / `dump_bytecode` subcommands                        | **No impact**                                                                   | These two paths do not go through proof_execution                                                                                                        |
| `TypeCheckResult` field migration to `Obligations`           | **Internal refactor**                                                           | If `pub` API surface has external dependencies, sync is needed; intra-repo consumers are fully listed in the change list                                 |

---

## Implementation Points

This document corresponds to the **P3 (Fix correctness vulnerabilities)** and **P4 (Stage contracts
and unified Driver)** of the RFC-039 global phase sequence. The following S1–S5 are the
implementation order within this document, **each step's acceptance criterion refers to the **C2
category** of the [Equivalence Criteria Document](07-equivalence-oracle.md) (orchestration change:
per-entry **diagnostic set** is the same + corpus behavior is the same).

### S1: Establish Criteria (Red First)

**A prerequisite, cannot be skipped.** The vulnerability criterion must be written to fail first.

| Deliverable                                                                                                                                                  | Acceptance                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                | **Must be red**. If it is green by accident, the vulnerability analysis of this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48 — do not mix into the single-file corpus tree; project fixture with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                            |
| `test_obligations_drained` skeleton                                                                                                                          | Marked `#[ignore]`, turned green in S4                                                                           |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to a Driver call           | **Single-file path diagnostic set and exit code are byte-for-byte identical**                   |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately does not fix any bug** — it only moves the existing behavior into Driver.
The acceptance criterion is zero-diff at the C1/C2 level.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restored to its original state is
sufficient.

### S3: Merge the Four Orchestrator Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                     |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The four entries' diagnostic sets are the same after normalization by `Aggregation` |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently in and out of the project                     |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                          |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                                 |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce a diagnostic |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics when Z3 is not installed                                           |

**Rollback point**: vulnerability fix and structural merge are **split into two commits**. If the
structural merge has a problem, only the structural commit can be rolled back while keeping the
vulnerability-fix commit — at this time `test_multifile_proof_obligation_not_dropped` stays green;
the reverse (keep structure, roll back fix) will turn red, which is an unacceptable intermediate
state and is forbidden to merge.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                                  |
| -------------------------------------------- | --------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                                      |
| `scripts/ci/check-obligations.py`            | Static gate effective                                                       |
| Obligation diagnostic W → E upgrade          | Every newly added E is manually reviewed after the full corpus differential |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration item;
the W/E switch does not require changing the code structure.

### S5: Proof Layer and Wasm Wrap-Up

| Deliverable                                                                                                                                                                                                                                  | Acceptance                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                       | Documentation corresponds to `checker.rs` call sites one by one                                                                                                       |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown must not enter the cache** — otherwise a single timeout solidifies across compilations via the process-level cache and pollutes the cargo-test thread-shared) | Production path `default_solver()` call sites drop to zero (tests retain only); cache hit rate is observable across the three consumers (counter landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds a warning                                                                                                                                                                                            | A diagnostic is produced when `body_checker` is missing                                                                                                               |
| Reachability determination for the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                   | Conclusion delivered to `06-cleanup-inventory.md`, this document only records the determination requirement                                                           |

**Note**: correcting the layer order will change the diagnostic set and may expose a large amount of
previously silenced `Unproven`. **This item should proceed independently of S1–S4**, not mixed in
the same PR as the entry merge.

## Key Decisions and Rationale

| Decision                 | Decision                                                                                                                                                          | Rationale                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustive enum, **runtime registration is rejected**                                                                        | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism already validated in RFC-039 routing table B for the opcode table. `StageScope` further makes "per-module or project-level" a type fact, eliminating the call-count question that requires human reasoning             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, in parallel with the **static** gate of `scripts/ci/check-obligations.py`; severity W first, then E | What is fixed is a whole class of bugs, not one: at least 2 such hidden dangers have been verified currently (`proof_calls`, `checker.rs:1313`), and all 16 span-keyed fields are in range. W first, then E, is to make every step's C2 criterion usable — going straight to E would cause `yaoxiang check` to suddenly emit a large amount of previously silenced diagnostics |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** the six `Unproven` branch producer logics at `checker.rs:5164/5179/5306/5318/5420/5448`         | The problem is on the consumer side, not the producer side. Changing the producer side would mask the architectural defect as "the logic is fixed", while the logic of these three branches itself is correct                                                                                                                                                                  |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment promise" and "code behavior"**. The "the silent pass must not be
  revived" at `checker.rs:5165` and the "the upper layer does not run when a lower layer fails" at
  `layers/README.md:3` are both **comment-level contracts** — the former is fulfilled by
  `assert_drained()`, the latter is exposed by the topology-driven `Skipped` diagnostic.
- **"Forgetting to append to one array"**. Currently, five functions each hand-write their own call
  sequence; after unification, there is a single stage table, and the new stage not being wired will
  fail at compile time.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — of the 11 inconsistencies, 7
  (the two implementations of dead-code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type interfaces**; they are configuration questions of
  "whether to run a certain analysis"; it also cannot express `Aggregation`, and can only move the
  two-way choice of `check_module` / `check_module_collect_all` from the function layer to the type
  layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, which is
  equivalent to replacing "five functions each hand-write their own call sequence" with "one array
  forgot to append", exactly the cause of the current 11 inconsistencies. The same-shape
  anti-example already exists in this repository: the `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader` described in `docs/src/dev/design/check/` are all
  zero-implementation.
- **Only fix the bug without touching the structure** — it can fix the 2 known bugs, but only covers
  1 of the 16 obligation fields, and `Stage` is still scattered across 5 functions; the next fork
  point will keep growing from here. **It must be done first** (it is part of S1/S3), because the
  structural refactor needs a known-red test to prove the criterion is valid.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it does
  not know who the caller is, and `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **wholly** discarded. This is looking for the bug at the wrong level:
  the bug is in the orchestration layer, and the detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into a "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is the common cost of all refactorings that
  "centralize the wiring."
- **S3 will significantly change the diagnostic set, and the direction of the change is "expose
  previously silenced problems"**. After the fix, a batch of "new error" user reports may appear —
  they are real bugs that just hadn't been reported. This must be clearly stated in the CHANGELOG.
- **The severity switch of `assert_drained()` requires per-field manual judgment**. Among the 16
  fields, some (e.g. `module_namespaces`) may not need to be consumed by design and should not be
  alarmed. S4 needs to go through them one by one, not cut them all with one stroke.
- **The `Program` abstraction may be premature**. If some entry's stage set is unstable in the long
  term, `stages()` will degenerate into "a different array each time" free parameters, and the
  contract constraint will be void. **The mitigation is `test_program_stage_coverage` asserting that
  `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: the vulnerability-fix criterion of S1/S3 depends on the
  [Equivalence Criteria Document](07-equivalence-oracle.md) establishing the multi-file corpus layer
  first. **If that document does not first establish the multi-file corpus layer, S1 cannot be
  accepted** — because the existing 293 corpus files all take the single-file path, with zero
  coverage for this class of defect. This document does not involve the IR verifier
  (`verify_loose`); that prerequisite is out of scope.
- **Correcting the layer order will expose previously silenced `Unproven`** (`equivalence` is not in
  the pipeline at all; the order of `termination` and `ownership` is the opposite of the
  declaration). This is a diagnostic set change risk and should proceed independently.
- **Thread safety after the SMT backend becomes a singleton is undetermined**. `backend.rs:13-19`
  has stated that `Solver` is only `Send` not `Sync`, and `Z3Backend`'s cache is `RefCell`; after
  changing to a cross-compile-unit shared singleton, all access paths need to be confirmed to go
  through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelism will **mask order-dependence defects** (such as the actual order dependence between
  `termination` and `ownership`). This should be opened after the equivalence criteria are stable.

> **The open questions originally listed in this section have all been ruled on.** See the per-item
> decisions in [RFC-039 Decisions Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactor (construction steps attributed to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 grading, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — The design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — The
  reference code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "the silent pass
  must not be revived"
- `src/frontend/core/typecheck/layers/README.md:3` — The explicit declaration of "the upper layer
  does not run when a lower layer fails" (no short-circuit in reality)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit reason for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "the caller
  should degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's own
  statement of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — The corpus's
  own statement of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, so it
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — The error-code build-time gate, the example of this project's mandatory
  mechanism
