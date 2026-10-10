# Compilation Stage Contract and Obligations Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, the grading of acceptance criteria, and the order of execution phases are in the
> RFC-039 main text; the positioning of each subsidiary document is in
> [this directory's index](index.md).

## Positioning and Scope

This document is the **construction drawing for the L1 orchestration layer** of RFC-039. It
addresses one problem: **"which stages ran in this compilation, which didn't, and why" is currently
scattered across six paths and ten entry functions, each hand-wired locally, with no single place
that can answer this question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by the architecture rather than by logic.**

The comment in `src/frontend/core/typecheck/checker.rs:5165` reads literally "Unproven → compile
error, no fallback, no silent pass... silent pass must not be resurrected." But this invariant only
holds on the **single-file path**: a proof-function constraint like `x: Sorted(3)` is **silently
passed** under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP. The proof
function is never executed, and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, not registered at runtime) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactoring approach for the ten
  entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency direction specification, anti-rebound gating → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gating → RFC-039 main text

### Division of Labor with RFC-039

RFC-039 provides **why** the refactor is needed and **in what order** to do it; this document
provides the L1 **specific form**, the **per-file change list**, and the **internal implementation
phases** of this document (corresponding to P3 "Fix Correctness Vulnerabilities" and P4 "Stage
Contracts and Unified Driver" in the RFC-039 global sequence). In case of any conflict between this
document and RFC-039, RFC-039 prevails.

The equivalence criteria are executed under the **C2 (Orchestration Changes)** category of the
[Equivalence Oracle document](07-equivalence-oracle.md): the diagnostic set for each entry is
identical + the corpus behavior is identical.

## Current State

> All facts in this section have been **verified**, each with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only "Write Wrong in One Place, Whole Repo Goes Silent" Structural Defect

It is qualitatively different from the other two defects (module boundaries, test wiring):

| Defect                  | Typical Manifestation                  | Is There a Signal?   |
| ----------------------- | -------------------------------------- | -------------------- |
| Lexical/syntax error    | Source code is wrong                   | Has diagnostics      |
| Type mismatch           | Type is wrong                          | Has diagnostics      |
| Stage not wired         | Some stage is never called at all      | **No signal at all** |
| Obligation not consumed | Field is filled in but no one reads it | **No signal at all** |

The common feature of the latter two: **failure produces no error**. Therefore they cannot be solved
by "writing code more carefully" or "reviewing more strictly" — code review can only see what is
written, not **what is not written**. This is exactly the root cause diagnosed by RFC-039: "this
project treats 'design' as a documentation convention rather than an executable constraint."

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms, they just have not been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subjected to a
  **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, comparing each entry
  against the RFC-013 code table; any inconsistency immediately `panic!`s and refuses to compile.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` accounts for 6227 lines / 47.9%) has
  high test density, with each module's test subtree being declared-wired — this is the most
  complete test wiring in the entire repo, and can serve as the formal reference for the stage-layer
  gate.

**Design capacity is sufficient. What is missing is "putting a gate of the same level at the
orchestration layer."**

(Line count basis: `(Get-Content).Count`, see the "Line Count Basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation Path                            |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5-stage direct call                               |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                  |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking            |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics       |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR          |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1                 |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                     |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes to 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                     |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1          |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                |
| ------------------- | ------------- | ------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                      |
| parsing             | `161`         | `run_parsing`                                                 |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                   |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5         |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined** (`381-389`) |

### 11 Inconsistencies in Stage Coverage

The table below provides cell-by-cell evidence. **A blank does not mean "this entry does not do
this", but "no code in this entry does this"** — this is the very nature of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                    | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                    | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                           | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

> **Review notes (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, preserved without modification. Current state differences
> after P3 lands:
>
> - **Row 1 has been fixed**: proof_execution is shared by all five entries through the same
>   implementation in `frontend/proof_execution.rs` (pipeline + four orchestrator entries); the
>   single-consumer defect no longer exists.
> - **Rows 2/4/5 are unchanged** (multi-file `run` still does not emit W1003, still does not run the
>   dead code family) → assigned to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization is exclusive to single-file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3, potential risk unverified defect).
> - **Row 7** `check_module` / `check_module_collect_all` dual entry → assigned to 4.2.7
>   (Aggregation parameter-driven).
> - The "Ruling #434" referenced in the code had no prior docs registration — it has been
>   retroactively registered as RFC-039 **D57**.
> - **Embedded std integration asymmetry (new fact outside the table, 2026-10-09 supplemental
>   registration)**: The single-file path unconditionally injects std.list through
>   `merge_embedded_std_ir` (required for for-loop desugaring, #117 hard switch), while the
>   multi-file `discover` previously only recognized explicit `use` — in-project for loops compiled
>   successfully but produced runtime E6006 (probe empirical evidence). Fixed and WBS 4.10.2 checked
>   off; the same family of "multi-file missing the single-file step" parse hard-abort quirk is
>   registered as WBS 4.10.1 — **also fixed** (landed immediately after 4.2.2, the Check path
>   degrades to per-file collection; from this audit's perspective of "stage coverage / field
>   consumption" dimension, this is a compilation unit membership difference, a missed dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 implementation, fixed)**:
>   The Registry arm repeatedly harvested registrations for embedded std units through
>   `extract_module_info`, which would overwrite the native half registered via `with_std()`
>   ("native + yx surface merged" registration) — `result.is_err` etc. lost → std.test falsely
>   reported E1043. This audit's third missed dimension: "the merge semantics of two registration
>   sources for the same module key" — the single-file path's registry is shaped only once through
>   `with_std()`; multi-file/Check's Registry arm inserts per unit to have coverage.

**Two points need precise expression, otherwise they will be written incorrectly:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` under the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; under the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is in
  `compile_project:138` — `type_results.push(result)` stores the full `TypeCheckResult` (including
  `warnings`), but this function has **no `result.warnings` read site throughout**, and the only
  downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's entry `main` criterion also comes from a different source**: `check_project:385-395`
  uses `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`,
  only checks "is there a manifest"). The two functions give different answers to "which files must
  define `main`."

### Correctness Vulnerability: Proof Obligations Silently Dropped (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation's production site is unique.**
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

The semantics are "the predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed to determine the result."

**Step 2 — The consumption site is inside the checker, but it only emits, never consumes.** There
are **three isomorphic branches** in `src/frontend/core/typecheck/checker.rs` handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The comment at `checker.rs:5165-5169` reads literally:

> `RFC-027 §4/§9: Unproven → compile error, no fallback, no silent pass.` ...
> `This branch is therefore a real line of defense, not a forward-looking placeholder: silent pass must not be resurrected.`

That is: the author knew "not producing diagnostics" was a defect, and explicitly classified it as a
**known, homeless intermediate state**, with the design assumption that "someone will definitely
come to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it, and `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof function obligations`. The field is defined in
`types.rs:29`.

**Step 4 — There is only one read site in the entire repo.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is read in production code at **only one place, `src/frontend/pipeline.rs:187`**
(passed as argument at `189`):

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
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries skip that layer entirely.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
in `src/frontend/module/orchestrator.rs` **all do not call `pipeline.rs`**, instead directly
invoking `TypeChecker::check_module` (`124` / `493` / `1386`). Therefore they cannot even reach the
only read site at `pipeline.rs:187`.

**Step 6 — Consequence: the standard library's own refinement obligations also take the discarded
path.** `compile_embedded_module` (`1374`, the `check_module` call is at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std modules
themselves are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) under the multi-file / check / LSP three paths **compiles
successfully, runs successfully, with no diagnostics at all**. The proof function is never called.

**Step 8 (supplemental verification) — There is a second silent discard point inside the layer.**
When `checker.rs:1303-1314` handles the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostics, no
bookkeeping.** This directly contradicts the `5164` branch's promise of "silent pass must not be
resurrected," and is **independent of the orchestrator issue** — even if the entry layer were
completely fixed, this one would still remain silent. (Handling timing: should be processed in the
same batch as the obligations ledger, since it is the same kind of problem as the obligations
ledger.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits on the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three single-tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example, `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call
exists in `result.proof_calls`. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping just before `pipeline.rs`**.

The test file's own documentation comment already states this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation is filled", but the
bug is "the consumer doesn't read it".** A test that only tests the production side and not the
consumption side is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Oracle's Primary Corpus is the Single-File Path

The [Equivalence Oracle document](07-equivalence-oracle.md) takes the end-to-end differential of 293
`.yx` corpus files in `tests/yaoxiang/` as the **third-layer criterion**, the main acceptance method
for C2 stages. But empirical:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero hits in a directory-wide glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) → `check_single_file` (`623-661`) → `Compiler::compile_with_source` (`635`) →
  `Pipeline::run` → **proof_execution executes**, for every corpus file.
- In other words, **all 293 corpus files take the single-file path, all of them cover
  proof_execution, none of them cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx` is the empirical
evidence of this point. That file is annotated `// expect: compile-error E4018` (line 15), and its
header comment at line 7 says "Status: ❌ Should be rejected at compile time" — **it passes,
precisely because it takes the only path that actually executes the proof function.**

**Conclusion: The third-layer criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as the acceptance tool for this document.** See "Implementation Points"
S1.

### Obligation Fields: 16 "Output is Contract" Fields, Zero Mechanism Guarantee

Among `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-70`, there are **16 fields**
whose documentation comments explicitly state "produced by stage X → consumed by stage Y", i.e.,
they are essentially **cross-stage obligations**:

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

**All these fields are passed across layers keyed by `Span` or name table.** The Equivalence Oracle
document already points out that a mismatch in the span-keyed contract will "silently fail with no
error" (`ReleasePlan` is typical: any inconsistency in how spans are computed on either side →
`Drop` instructions silently disappear).

Why does `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:341`
  (`release_plan: type_result.release_plan.drops.clone()`), is inside `generate_ir_with_context`,
  and `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it** — the
  consumer is in a downstream module, and the downstream module is on the necessary path of all
  entries.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry's implementation**. The orchestrator's four entries don't pass through `pipeline.rs` at all.

**Verified fact**: Currently there is **no mechanism in the entire repo** that guarantees these 16
fields are consumed. The only "protection" is that `ReleasePlan` happens to hitch a ride on IR
generation.

### Four Additional Contract Defects in the Proof Layer

All from proof layer analysis, independent of the entry-fork problem, but all belong to the category
"stage contracts are not enforced."

**(a) The layer-order declaration contradicts the actual execution order, and the `equivalence`
layer is not in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

Line 3 of the README claims "executed in layer order, the upper layer does not run if a lower layer
fails." The actual call sites in `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **opposite** to the declaration; the declared
Layer 0 `equivalence` **does not appear in the stage sequence of `check_module` at all** (it is only
used as an `is_subtype` utility function by `inference/assignment.rs:17`, unrelated to
`ProofResult`). At the same time **there is no short-circuit** — `checker.rs:1271-1276` adds the
termination errors one by one via `add_error` and then continues, so the README's promise of "the
upper layer does not run if a lower layer fails" does not hold.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review notes (2026-10-07)**: The hard-fail panic has been eliminated by P3's 3.3.1 (the SOLVER
> slot made Optional, missing falls back conservatively as `SMTResult::Unknown`); "silently skip
> (don't inject)" has been patched with a W1081 signal by 3.4.2. Unification of the three forms
> (singleton-ization) is executed per **RFC-039 D58**; the actual fix location is
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure port),
> the `default_solver() → &'static` wording in the 02 change list follows D58 (a bare `&'static`
> reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver is Unavailable                                 |
| ------------------------------------------------------ | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Construction-time injection `with_solver_owned`          | `None` → **silently skip** (don't inject)                  |
| `ownership.rs:627` (back-edge cutting decision)        | **New per call** `default_solver()`                      | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "initialization failure remains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond kernel
capability'." Yet the contract document at `backend.rs:60` says "`None`: backend unavailable...
**caller should degrade conservatively**." **Two philosophies coexist in the same code, and the
hard-failure one panics instead of returning an error.**

**(c) A new Z3 context is created on every back-edge decision; the cache is useless in form.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge cutting decision. And
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
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the documentation comment
at `z3_backend.rs:17` claims "SMT query results are cached in `cache`." Not shared across calls ⇒
**this cache never hits under this calling pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. The same backend, two
lifetime strategies.)

**(d) The checker silently degrades + the comment doesn't match reality.** `checker.rs:1283-1287`
and `1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call ownership table
```

When `body_checker` is `None`, the ownership check gets an **empty type ledger and an empty call
table** and runs as usual — ownership analysis degenerates to "nothing conflicts", **without any
warning**. (Suggestion: should be recorded as a warning-level diagnostic, or at least left as a
trace in the obligations ledger.)

The comment at `checker.rs:1244` claims that termination check "runs after type checking and before
constraint solving." The actual `self.env.solver().solve()` is at `checker.rs:1320` — **termination
is at `1256`, ownership is at `1296` after, i.e., what the comment says "before" is actually "two
layers after"**.

### Current wasm State: Borne by a Shim Crate, the 27-File Branch is Live

The wasm target **is already built and built in CI**, with `cdylib` not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has a wasm target dependency section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                   |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacks into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning when missing)                                                                                                                                                            |
| **27 files** mention the `wasm32` literal         | Of which **25** carry an actual `#[cfg(...)]` attribute, and the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) are only mentioned in documentation                                                                                                                       |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm
target — `lib.rs:139/153/170` gate the entirety of `run_file` / `run_project` / `build_bytecode`
(all three need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: The playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path Consuming `proof_calls`               | Does it Execute Proof Functions? |
| ------------------------------------------ | -------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                          |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                          |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                          |
| Multi-file `run` (`compile_project`)       | **No**                           |
| `check` (`check_project`)                  | **No**                           |
| LSP in-project (`check_source_in_project`) | **No**                           |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add a
new `WasmPlayground` variant (see "Target Design" 3), otherwise this path will be missed when
unifying the Driver.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — is assigned to `06-cleanup-inventory.md`. The open question in RFC-039 of
"build a wasm target, or delete these branches" **is now moot**: the target is already built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Registered at Runtime)

**Core constraint: `Stage` is a compile-time-exhaustive enum, runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time "a new stage must be
handled by the orchestration layer" — this is exactly the mechanism that has already proven
effective for the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                    Project
    Parsing,            // Lexical + syntactic               PerModule
    Registry,           // Module registry construction      Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type check (includes proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis         Project
    ProofExecution,     // Proof function compile-time exec  PerModule
    GlobalSlotAlloc,    // Global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                    PerModule
    Monomorphization,   // Monomorphization                  Project
    Linking,            // Cross-module linking / IR merge  Project
}

impl Stage {
    /// All stages. When a new variant is added, this array and the exhaustive match in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) vs `Project` (run once
at the project level) turns "which stages must run per module, which must run once at the project
level" into a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once
in `dispatch` — this eliminates current questions in `orchestrator.rs` like "should
`allocate_global_slots` be called per file" that require human reasoning.

**The stage table is arranged in topological order, not alphabetical order**, because failure
propagation depends on the order.

> **Revision notes (P4 implementation, 2026-10-07)**: Two deviations from the initial draft are
> corrected per implementation evidence —
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the initial draft's order contradicts
>    the data flow.
> 2. The Check form's stage table is normalized to dead code **before** proof per the `Stage::ALL`
>    topological order: `check_project` currently runs proof before dead code, the two have no data
>    dependency, the diagnostic set is identical (C2 set semantics), only the in-file diagnostic
>    order is normalized. The single-file path (dead code inlined into typecheck, before proof)
>    already follows the ALL order, and the byte-level acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    initial draft order would force the Registry arm to "hide parse", making the stage table lie
>    about the data flow; and multi-file parse therefore drops from 2 times to 1 time. Current state
>    honestly registered as a side note: the multi-file path's parse error was previously a hard
>    abort (the `?` propagation of `build_registry_from`, the tension with CollectAll semantics
>    registered as WBS 4.10.1) — **4.10.1 has been fixed** (2026-10-09 user ruling rust-style
>    collection semantics + Plan B): the Check path's parse failure degrades to per-file diagnostic
>    collection, sick files exit the compilation unit (don't enter the registry, importers report
>    E5001); the MultiFile path retains hard abort (under FailFast a bad file cannot produce IR,
>    which is correct semantics, the pinboard is valid in the long term).
> 4. The Check form lands (4.2.2, 2026-10-09 user ruling) and two more notes are registered: a. The
>    data dependency edges go from 14 to 16: `RoleClassification` consumes Discovery's `used_by`
>    edge set, and `DeadCodeAnalysis` consumes Discovery's W1006 shadowing event — the initial draft
>    missed these (these two products were produced by `discover_with_used` in the orchestrator era
>    and discarded by the `discover` wrapper, the data flow was lost before entering the table); b.
>    The in-file diagnostic order normalization extends to the full form of note #2 above: E3020
>    entry validation moves into the RoleClassification arm (the Check form has no Linking stage,
>    the arm holding `surfaces` and AST is responsible for validation), the in-file diagnostic order
>    = stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic
>    set is unchanged (C2 set semantics), only the stderr entry order changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: Turn "a field is produced but no one consumes it" from undetectable into a
compile-time or test-time assertable fact. RFC-039 has already listed this as "the most valuable
item."

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

**Settlement semantics (in three tiers)**:

| Situation                                   | Criterion                                                   | Disposition                                                                           |
| ------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Obligation is read by its declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                                  |
| Obligation is non-empty but has no consumer | The field is non-empty and not consumed                     | **Produce diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| Obligation is empty                         | The field is empty                                          | Pass (no consumption needed)                                                          |

**Why W level first, not E level**: Fixing the obligation discard will **change the diagnostic
set**. The C2 criterion requires "the diagnostic set for each entry is identical"; if we go straight
to E level, `yaoxiang check` will suddenly produce a large number of previously silenced `Unproven`
diagnostics. Two steps (observe as W first, then promote to E) make the criterion usable at every
step. See "Implementation Points" S4 for details.

**`assert_drained()` is called at one place only**: `Driver::run` at the end of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appears ≥ 2 times but no third file reads it → failure). `assert_drained()` is a **runtime** gate,
and that script is a **static** gate — they complement each other, and both are needed.

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
    /// Can only be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one for each `ProgramKind`); it
does not accept the caller freely passing an array.** This is the key constraint to prevent the
`Program` abstraction from degenerating into "a free parameter that can pass anything," asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // The topology decides whether to skip, not the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one can be missed
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision notes (P4 implementation, 2026-10-09, 4.1.3 landed)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for reference —
>
> 1. The three-state `StageOutcome` in §4 is not returned by the stage arm, but expressed via
>    `State` failure markers + `Aggregation` gating (Continue / Warn / Abort semantics unchanged,
>    the boilerplate return per arm is saved);
> 2. `Driver` has no `config` field — the only source of configuration is `Program.config` (the §5
>    "avoid Driver holding mutable global state" determination covers the sketch field);
> 3. The `Skipped` diagnostic in 4.1.3 is only recorded into the `DriverOutcome.skipped` internal
>    ledger, not emitted externally — the S2 "intentionally fix no bug" zero-diff criterion requires
>    this; external emission goes with the 4.3 obligations ledger;
> 4. The `proof_execution` module's visibility is relaxed to `pub(crate)`: the driver arm is the
>    only new caller (L1→L2 is an allowed direction); the orchestrator's existing call sites will be
>    relocated after 4.2 migration before being reconsidered.
> 5. Under the multi-file form, ProofExecution is an independent stage, executed **after all**
>    typechecks (A1, 2026-10-09 user ruling): `compile_project` originally interleaved proof in the
>    per-file typecheck loop (file N's proof precedes file N+1's typecheck). Single-failure
>    scenarios are byte-identical for the two; in the multi-failure scenario "file 1 proof failure +
>    file 2 typecheck failure", the first report changes from a proof error to a typecheck error
>    (test pinboard).
> 6. `DriverOutcome` carries results in channels per `ProgramKind`: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (`PipelineError` / `OrchestratorError`) is not part of the types the Driver can unify.
> 7. Check channel lands (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have an empty Vec — the existing
>    `check_project` contract); the Discovery product is extended to a triple of "file set + used_by
>    edge set + shadowing event" into State (the MultiFile form has no downstream consumer, only
>    records without external emission); the Check path's per-file parse drops from 2 times to 1
>    time per the C3 topology.
> 8. Lsp form lands (4.2.3, split-routing by file source ruling, 2026-10-09 user decision): disk
>    file parse failure follows Check's Plan B (collect + exit compilation unit — degradation
>    converges on both ends); edited buffer parse failure retains a partial AST and continues
>    typecheck (editor philosophy — intermediate typing states should not make semantic features
>    disappear). The Discovery arm for Lsp retains the buffer source overwriting stale disk content;
>    Typecheck under Lsp only checks the target file (other units only supply signatures for the
>    registry); the in-file diagnostic order normalization principle extends to LSP (typecheck
>    diagnostics → W-code warnings → proof errors). The old behavior of LSP hard-aborting on
>    unrelated disk file parse errors and the handler silently falling back to the single-file path
>    is fixed with this step.
> 9. Embedded form lands (4.2.4): `Program` adds a `shared_registry` field — the embedded std module
>    is a sub-compilation, and the registry is **input** rather than product (EMBEDDED_STAGES has no
>    Registry stage), and the SymbolTable sharing contract from #94 is thereby made explicit as a
>    program declaration, away from the "caller remembers to pass the same registry" arrangement.
>    The error path text is normalized from `<std.test> (embedded std)` to the unit virtual path
>    `<std/test>` (only inside the compiler error surface, no test pinning). At this point all four
>    orchestrator entries have been migrated into the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): The Check form
>     is the faithful bearer of single-file semantics for programs without a project root — the
>     warning surface only covers the entry file (neighboring files only get errors, prefer omission
>     over false positive); relative `use` is resolved along the importer's directory (rustc
>     single-file mod alignment). The CHECK stage table adds `GlobalSlotAlloc` + `IrGeneration`: the
>     standalone old path (full pipeline chain) already ran IR generation, with E3019/E1014/E1015
>     only generated in `ir_gen` (runner gate empirical evidence). The IrGeneration Check form is
>     pure-check, not consuming IR; the Script/Bin split is expressed as `module_key`'s None/Some
>     (existing switch for E3023, `ir_gen.rs:1660`). Monomorphization is not in CHECK (only produces
>     resource over-limit / internal errors, zero corpus dependency). **〔2026-10-10 partially
>     retracted after 3.4.8 empirical evidence〕** — "zero corpus dependency" is actually
>     evidence-blindness from the corpus having no pathological recursive fixture; E3005 is the only
>     compile-time line of defense for such programs. See note #14. Also fixes a latent defect: the
>     Registry arm's repeated registration harvesting for embedded std units overwrites the native
>     half already merged by `with_std()` (false positive E1043) — the embedded unit now skips
>     duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual `lex→parse→check_module_collect_all`
>     sequence in `run_diagnostics` is deleted, replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains a partial AST and
>     continues typecheck (editor philosophy extended from 4.2.3 ruling; not marking stage failure,
>     otherwise topological skipping prevents typecheck from ever running); Typecheck dispatches the
>     `check_module_collect_all` free function. The LSP single-file path now fills in proof (E4018),
>     W-code warnings (W1001–W1005, test pinboard), and IR-level errors; lexical failure now reports
>     real diagnostics (the old synthetic "E0001 lexical error" text disappears with the sequence
>     deletion — real diagnostics carry precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory):
>     the two entries have long since converged on their own — the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, with both pass-3 and drain modes running in parallel, the difference
>     carried by whether `collected_errors` is empty; five collection points in `statements.rs`
>     (function body / use / for / block / while). During verification, the duplicate diagnostic
>     defect is confirmed and fixed: the collection points put the first error into both
>     `collected_errors` and the Err return channel, and pass-3's `add_error` on Err causes same
>     code / same span / same message × 2 (nested × 3); plus two independent mechanisms: the
>     annotation validation signature's parameter vs whole annotation double-visit (E1003/E1103),
>     and the ownership layer's same-point double-emit (E2014/E2018). The fix lands at the module
>     result boundary: dedup by (code, span, message) — diagnostics are positional facts, duplicates
>     carry no information; 63 corpus entries have duplicate copies removed, the run column and exit
>     code drift zero (the "duplicate" in `borrow_conflict_err` is an artifact of the baseline
>     triple format not containing column numbers — the two E2018 are actually valid diagnostics
>     from different columns).
> 13. wasm playground lands (4.2.8): `run_code` / `test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver — the `Compiler` wrapper layer is no
>     longer involved. The error text is byte-aligned with the old `CompileError` Display prefixes
>     (Parse error: / Type error: / Internal error:), zero visible difference in the playground UI.
>     WasmPlayground and SingleFile share `SINGLE_FILE_STAGES` + `FailFast` (4.1's existing
>     declaration, this step gives it real producers).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partially retracts the mono exclusion of note #10**). Empirical evidence overturns "mono
>     only produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) in the
>     multi-file path without mono causes the compiler process to stack overflow (0xc00000fd — the
>     interpreter recurses at the Rust layer, no graceful runtime error), and mono's depth gate is
>     the only compile-time line of defense; standalone check, before 4.2.5, went through the full
>     pipeline chain and had mono, and the exclusion ruling caused a silent coverage regression
>     ("zero corpus dependency" only proves the corpus has no pathological fixtures). The "type
>     erasure fallback" assumption is invalidated by the RFC-033 reflection ruling — `^^List(Int)`
>     needs the real identity of instantiation, and monomorphization is the foundation of
>     reflection, not an optional optimization. Four design points: ① typecheck produces an
>     `instantiation_request` for qualified calls (`lib.f(x)`, FieldAccess form), with `generic_id`
>     using the qualified name, in the same namespace as the merged IR (currently only recognizes
>     bare `Var`, `expressions.rs:2455`); ② the `Stage::ALL` topology is changed so that Linking
>     precedes Monomorphization, and the MULTI_FILE stage table adds Monomorphization at the tail —
>     consumes `merged_ir`, aggregates all-units requests (the `containing_fn` qualification of the
>     deferred bucket), mono upgrades from a single-module pass to a whole-program pass; the
>     single-file stage table is unchanged (no Linking, the subsequence property is preserved, the
>     IR snapshot has zero drift); ③ CHECK adds Linking (pure merge — E3020 entry validation stays
>     in RoleClassification, to prevent double-report) and Monomorphization (pure check, not
>     consuming IR, same as the IrGeneration Check form); ④ resource protection covers cross-unit
>     mutual recursion with whole-program BFS. The Monomorphizer body is unchanged (the contract of
>     consuming ModuleIR + request set is exactly the merged IR form).
> 15. 3.4.8 R1 (①) lands (2026-10-10, a9438007). Per the #14 design, the qualified-call
>     instantiation collection lands: `ExpressionInferrer` injects a module-qualified key table (the
>     three arms of `use` are registered — normal / alias / per-item alias, and only SubModule-class
>     exports), `callee_generic_name` is concatenated via `SymbolTable::qualify` into a qualified
>     name sharing the source with merged IR; the qualified-call first-path arity criterion is
>     tightened (when the parameter table cannot be resolved, fall to the second path taking actual
>     arguments from the signature, avoiding cross-module same-named function mismatch and
>     false-positive E3018). During implementation, three **asymmetry defects** on the mono side are
>     empirically found and fixed in the same round: (1) the delete key (generic name set) and
>     rewrite key (`containing_fn` + name + span triple) have granularity mismatch — call sites that
>     cannot resolve actual arguments degrade to symbol requests entering the deferred bucket, and
>     when the containing function is non-generic the bucket never drains, while the original has
>     already been deleted by name → dangling calls (`list_ops.yx:53` runtime E6006 empirical
>     evidence); the fix is the recovery gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as the single source of truth for coverage determination, with the
>     original of uncovered sites preserved. (2) The recovery segment's `HashSet` iteration order
>     causes the function table order to be non-deterministic (t2 review F1) → changed to `Vec` and
>     sorted by name, byte-level reproducible products. (3) The non-rewrite form `TailCall` /
>     `MakeClosure` originally looked up map keys the same way (though the comment claimed it was
>     conservative) → single-point determination via `call_form_is_rewritten`, non-rewrite form is
>     counted as uncovered. Each is paired with a pin test (red state before the fix demonstrates
>     discriminative power). Corpus differential baseline regenerated on the same commit, exactly
>     one line drift (`remove_at` frame adds `(int64)`). **Commit strategy note**: when the
>     typecheck collection and mono fix were split into two commits, the stashed tree containing
>     only the former triggered `stdlib_docs`'s red for an existing HEAD defect (same-family E6006
>     for `list.is_empty([])`) — the two halves are each other's green prerequisite, so they are
>     merged into a single atomic commit.

**Refactoring approach for the ten entries (per function)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                           |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                      |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                              |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a single Driver call + result filtering to the target file                                                                                                                                         |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                    |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                       |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                              |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project goes to `Program { kind: Lsp, aggregation: CollectAll }`; **delete the manual lex→parse→… sequence in the single-file branch**, switch to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | The `standalone` branch (`614-616` → `check_single_file` `623-661`) is deleted, unified to `Program { kind: Check }`                                                                                         |

**The degree of duplication between `check_project` and `compile_project` (about 40% shared / 60%
forked)**. Shared: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parsing loop, per-file checker assembly, result processing. Forked: items
2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton is exactly the value zone
of the `Driver`** — the forked 60% is all "different stage set" or "different aggregation mode",
which is exactly what the two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design determination: "upstream failure caused this stage to be skipped" is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If each stage were to
return `Skipped` on its own, then the "why I didn't run" information would be scattered across 12
stages and could not be audited centrally. Change to having `dispatch` determine it at the beginning
of the loop based on dependency relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability to be fixed by
this document — the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip"; it
produces no diagnostic, just because the obligation happens to be empty. Once the rule is
established, any "Y didn't run because X wasn't done" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                               |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` caused                                  | "Stage Y did not execute due to upstream stage X failure"                                                                                    |
| Pre-obligation empty caused                              | "Stage Y did not execute because there are no pending obligations (normal)" — **severity = Info for this one, not counted in warning count** |
| Conditions not met (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because the configuration is not enabled"                                                                           |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` relies on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop on the first error (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                                  | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early exits)                    | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` does not early-exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; placing them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just different in "early-exit
or not" — they are two different checker entries. After unification, the same implementation should
be driven by the `Aggregation` parameter, rather than keeping two functions (**this is an additional
item to be verified in this document, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation introduced, no touch on `MonoType` / `PolyType` / `ir::Type`                                                    |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type attribution and layering**: What is migrated into the ledger is **settlement
responsibility**, not the attribution module of the type. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay in their
original places; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`.
L3 consumers (e.g. `ir_gen` reads `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`** — this is consistent with the no-reverse-dependency red
line for driver below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse-`use`
`crate::driver`**. A `Driver` appearing in `TypeChecker`'s import is considered a violation,
intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Refactoring                       | After Refactoring                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement                                                                                                                                                                                                                                                                                                                |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already plugged in by P3; **warning surface remains not newly added** (decision B1, 2026-10-09: dead code family warnings do not enter the compile path — the pool semantics are being fixed as a defect by 4.9, normalized warning surface deferred until after 4.9; the original "newly added warning output" wording in this row is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                                                     |
| LSP (in-project)                    | No proof_execution                       | Already plugged in by P3                                                                                                                                                                                                                                                                                                                                     |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                   |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                                                       |

**The only intentional behavior break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This aligns with RFC-027 §8 "not bound to a specific solver," and is also the contract
already declared at `backend.rs:60` ("caller should degrade conservatively") — the current
implementation is contrary to its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for single-file / N for multi-file)                         |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------------------- | ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **preserved**, moved into `driver` and used as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to a Program constructor                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` switched to Driver                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | The `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` is now driven by the `Aggregation` parameter                                                                                                                                                                                                                               |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | Manual stage sequence in `run_diagnostics` deleted, switched to Driver                                                                                                                                                                                                                                                                                            |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                               |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                   |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently goes through `Compiler::compile_with_source` implicitly to path 1)                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix the second silent discard point**)                                                                                                                                                                                                                                            |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                            |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct the comment that doesn't match `1320`                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Change the layer order table to the **actual** execution order; remove "lower layer failure means upper layer does not run" (no short-circuit exists)                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add a process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; the three consumer points `predicate.rs` / `checker.rs` / `ownership.rs` go through the shared port. The bare `&'static` reference form is not feasible (`dyn Solver` is not `Sync`), the original wording follows D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual `lex`/`parse`
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Untouched**: the six `Unproven` branch logics at `checker.rs:5164/5179/5306/5318/5420/5448` (these
are themselves **correct — the problem is at the consumer end, not at the producer end**, changing
the producer would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (the reachability determination of the wasm branch is
assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                               | Disposition                                                                                                                                           |
| --------------------------------------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously silently passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first, then fix**; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compiled successfully now produce warnings      | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), exit code unchanged                                      |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set adds                                           | W first then E, S2/S4 two steps                                                                                                                       |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                          | No break                                                                                                                                              |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                               | These two paths don't go through proof_execution                                                                                                      |
| `TypeCheckResult` field migration to `Obligations`        | **Internal refactor**                                                       | The `pub` API surface needs to be synchronized if there are external dependencies; in-repo consumers are all listed in the change list                |

---

## Implementation Points

This document corresponds to **P3 (Fix Correctness Vulnerabilities)** and **P4 (Stage Contracts and
Unified Driver)** in the RFC-039 global stage sequence. S1-S5 below are the implementation order
within this document, and the acceptance criterion for each step references the **C2 category** of
the [Equivalence Oracle document](07-equivalence-oracle.md) (orchestration change: **diagnostic
set** for each entry is identical + corpus behavior is identical).

### S1: Establish the Criterion (Red First)

**Prerequisite, cannot be skipped.** The vulnerability criterion must be written as failing first.

| Deliverable                                                                                                                                                     | Acceptance                                                                                                        |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                   | **Must be red**. If it is accidentally green, the vulnerability analysis of this document needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (resolution D48 — don't mix into the single-file corpus tree; project fixture with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions of the corpus are comparable                       |
| `test_obligations_drained` skeleton                                                                                                                             | Mark `#[ignore]`, turn green in S4                                                                                |

**Rollback point**: no code change, purely new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code are byte-for-byte identical**                   |
| Full corpus (293 `.yx`) differential                     | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage intentionally fixes no bug** — it just moves the existing behavior into the Driver. The
acceptance criterion is zero-diff at the C1/C2 level.

**Rollback point**: `git revert` a single commit, restore `pipeline.rs` to its original state.

### S3: Merge Four Orchestrator Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The diagnostic sets of the four entries are normalized by `Aggregation` and are identical |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project                      |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                                |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                                       |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` produces a diagnostic in any mode           |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in the Z3-absent environment                                             |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has issues, only the structural commit can be rolled back, and the vulnerability
fix commit retained — at this point `test_multifile_proof_obligation_not_dropped` remains green; the
reverse (retain structure, roll back fix) would turn red, which is an unacceptable intermediate
state and forbidden to merge.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                |
| -------------------------------------------- | --------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `test_obligations_drained` turns green                    |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                  |
| Obligation diagnostic W → E promotion        | Manually review each new E after full corpus differential |

**Rollback point**: the severity of `assert_drained()` is controllable by a configuration item, the
W/E switch does not require code structure changes.

### S5: Proof Layer and wasm Wrap-Up

| Deliverable                                                                                                                                                                                                                                             | Acceptance                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to the actual order                                                                                                                                                                                              | Documentation corresponds entry by entry to `checker.rs` call sites                                                                                                                 |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter the cache** — otherwise one timeout will be固化d across compilations through the process-level cache, and will pollute cargo test's cross-thread sharing) | Production path `default_solver()` call sites drop to zero (only tests retain); cache hit rate is observable across the three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds a warning                                                                                                                                                                                                       | Diagnostic when `body_checker` is absent                                                                                                                                            |
| Reachability determination of the 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                               | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination need                                                                                   |

**Note**: correcting the layer order will change the diagnostic set, and may expose a large number
of previously silenced `Unproven` instances. **This item should proceed independently of S1-S4**,
and should not be mixed into the same PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Decision                                                                                                                                                              | Reason                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **rejects runtime registration**                                                                                | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism that has already been validated in RFC-039 routing table B for the opcode table. `StageScope` further turns "per module or project level" into a type-level fact, eliminating the need for human reasoning about call counts |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used in parallel with the **static** gate of `scripts/ci/check-obligations.py`; severity W first then E | This fixes a whole class of bugs, not one: there are at least 2 known same-class latent issues (`proof_calls`, `checker.rs:1313`), with all 16 span-keyed fields in range. W first then E is to make the C2 criterion usable at every step — going straight to E would cause `yaoxiang check` to suddenly produce a large number of previously silenced diagnostics                  |
| **Fix scope**            | Fix the **consumer end** (orchestration layer), **do not touch** the six `Unproven` branch producer-end logics at `checker.rs:5164/5179/5306/5318/5420/5448`          | The problem is at the consumer end, not at the producer end. Changing the producer would mask the architectural defect as "logic fixed", and the logics of these three branches are themselves correct                                                                                                                                                                               |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment promise" and "code behavior"**. The "silent pass must not be
  resurrected" at `checker.rs:5165` and the "lower layer failure means upper layer does not run" at
  `layers/README.md:3` are both **comment-level contracts** — the former is redeemed by
  `assert_drained()`, the latter is exposed by the topology-driven `Skipped` diagnostic.
- **"One array forgetting to append"**. Currently five functions each hand-write the call sequence;
  after unification, it is a single stage table, and a new stage being missed in wiring will cause a
  compile error.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type bridging**; they are configuration questions of "whether
  to run a certain analysis"; nor can it express `Aggregation`, and can only move the `check_module`
  / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each hand-writing the call" with "one array forgetting to append", which
  is exactly the cause of the current 11 inconsistencies. The same repo already has a counterexample
  of this form: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix the bug without changing the structure** — can fix the 2 known bugs, but only covers 1
  of the 16 obligation fields, and `Stage` is still scattered across 5 functions, and the next fork
  point will continue to grow from here. **It must be done first** (it is part of S1/S3), because
  structural refactoring needs a known red test to prove the criterion is effective.
- **Make `proof_calls` `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, and `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is **wholly** discarded. This is looking for a bug at the
  wrong level: the bug is at the orchestration layer, and detection must be at the orchestration
  layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silenced problems"**. After the fix, a batch of "new errors" user reports may appear —
  they are real bugs, they just hadn't been reported before. Must be clearly stated in the
  CHANGELOG.
- **The severity switching of `assert_drained()` requires per-field manual judgment**. Among the 16
  fields, some (e.g. `module_namespaces`) may be designed to not need consumption, and should not
  alarm. S4 needs to go through them one by one, cannot be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entry's stage set is unstable in the long
  term, `stages()` will degenerate into "passing a different array every call" free parameter, and
  the contract constraint fails. **The mitigation is `test_program_stage_coverage` asserting that
  `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: the vulnerability-fix criterion of S1/S3 depends on the
  [Equivalence Oracle document](07-equivalence-oracle.md) establishing the multi-file corpus layer
  first for the vulnerability test to go red. **If that document does not first establish the
  multi-file corpus layer, S1 cannot be accepted** — because the existing 293 corpus all take the
  single-file path and have zero coverage of this class of defect. This document does not involve
  the IR validator (`verify_loose`), and that prerequisite is not within the scope of this document.
- **Layer order correction will expose previously silenced `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` are in the opposite order to the declaration). This
  is a diagnostic-set-change risk and should proceed independently.
- **Thread safety after SMT backend becomes singleton is undetermined**. `backend.rs:13-19` already
  states that `Solver` is `Send` but not `Sync`, and `Z3Backend`'s cache is `RefCell`; after
  changing to a singleton shared across compilation units, it needs to be confirmed that all access
  paths go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization would **mask ordering-dependence defects** (such as the actual order dependence of
  termination and ownership). Should be opened after the equivalence criterion is stable.

> **The open questions originally listed in this section have all been ruled on.** See the
> [RFC-039 ruling register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **This document has no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-layer criterion,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Source of the Phase 2.5 proof function execution design
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Express declaration of "silent pass must not
  be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — Express declaration of "lower layer failure
  means upper layer does not run" (actually no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Express reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-70` — 16 obligation fields that downstream must consume
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The single test's
  self-acknowledgment of "stopping before pipeline"
- `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx:12-13` — The corpus's
  self-acknowledgment of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, so it
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — `cdylib` + `wasm-bindgen` are in the shim crate, not the main crate
- `build.rs:19-55` — The error code build-time gate, an example of the project's enforcement
  mechanism
