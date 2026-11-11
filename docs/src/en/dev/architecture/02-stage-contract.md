# Compilation Stage Contract and Obligation Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criterion tiers, and execution-phase order are covered in the main
> RFC-039 text; see the [directory index](index.md) for the role of each companion document.

## Purpose and Scope

This document is the construction blueprint for RFC-039’s **L1 orchestration layer**. It addresses
one problem: **“Which stages did this compilation run, which did it not run, and why?” is currently
wired manually across six paths and ten entry functions, with no single place able to answer the
question.**

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are broken by the architecture, not by the logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` states (originally
`checker.rs:5165`, relocated by P5 5.2.1) that “Unproven → compile error, no fallback, no silent
pass……silent pass must not be allowed to return.” However, this invariant holds only on the
**single-file path**: proof-function constraints such as `x: Sorted(3)` silently pass on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed,
and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustively enumerable, with no runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, and the required changes to each of the ten entry
  functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- Every affected file and line range, compatibility impact, and implementation order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-bounce gate → `01-routing.md`
- Equivalence criteria (C1–C6 tiers and three levels of criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch-reachability cleanup → `06-cleanup-inventory.md`
- The global P1–P10 execution order and G1–G10 acceptance gates → main RFC-039 text

### Division of Labor with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should proceed. This
document defines the L1 layer’s **concrete shape**, **per-file change list**, and **implementation
phases within this document** (corresponding to P3 “Fix correctness vulnerabilities” and P4 “Stage
contract and unified Driver” in RFC-039’s global sequence). If this document conflicts with RFC-039,
RFC-039 takes precedence.

Equivalence criteria are applied under **C2 (orchestration changes)** in the
[equivalence-criteria document](07-equivalence-oracle.md): the diagnostic sets of all entry points
are identical + corpus behavior is identical.

## Current State

> This section contains **verified facts**, each backed by a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect Where “One Mistake Silences the Entire Repository”

This differs in nature from the other two classes of defects, namely module boundaries and test
wiring:

| Defect                | Typical manifestation                     | Is there a signal?       |
| --------------------- | ----------------------------------------- | ------------------------ |
| Lexical/syntax error  | The source code is wrong                  | Yes, a diagnostic        |
| Type mismatch         | The type is wrong                         | Yes, a diagnostic        |
| Stage omission        | A stage is never invoked                  | **No signal whatsoever** |
| Unconsumed obligation | A field is populated, but nobody reads it | **No signal whatsoever** |

The latter two share a common characteristic: **failure does not produce an error**. They therefore
cannot be addressed by “writing the code more carefully” or “performing a stricter review”—code
review can reveal what was written, but not **what was omitted**. This is precisely the root cause
identified by RFC-039: “This project treats ‘design’ as a documentation convention rather than an
executable constraint.”

### Enforcement Mechanisms Already Present in This Repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes + 8 W codes) are subject to a
  **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`, which compares them one
  by one against the RFC-013 code table. Any mismatch directly triggers `panic!` and rejects the
  build.
- `src/package/` (**76 files / 13,012 lines**, including 6,227 lines under `tests/`, or 47.9%) has
  high test density, and every module’s test subtree is wired explicitly. It has the repository’s
  most complete test wiring and can serve as a structural reference for a stage-layer gate.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths and Ten Entry Functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path used                                                              |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages invoked directly                                                       |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **LSP within a project**   | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multiple files**     | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c within a project; otherwise manual lex→parse                               |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages on the single-file path (`pipeline.rs`) are:

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`         | `run_ir_generation`, with **monomorphization embedded in it** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point “does not do this”; it means that “there is no code in that entry point that does
this”**—which is precisely the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                              | `check_project`                                                            | `check_source_in_project` (LSP)    | `compile_embedded_module`              |
| --- | -------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------- | -------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                         | **No**                                                                     | **No**                             | **No**                                 |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                         | Yes `356-377` (role-aware, no configuration gate)                          | **No**                             | **No**                                 |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                         | Yes `336-348`                                                              | **No**                             | **No**                                 |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                                | Yes `350`                                                                  | **No**                             | **No**                                 |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                         | Yes (same as 2)                                                            | **No**                             | **No**                                 |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                         | Not applicable                                                             | Not applicable                     | **No**                                 |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on the first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all diagnostics | `check_module_collect_all` (`493`) | `check_module` (`1386`)                |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)         | `discover_with_used` (`275`)                                               | `discover` (`454`)                 | Not applicable                         |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                         | Yes (`321-328`)                                                            | **No**                             | **No**                                 |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                            | **No**                                                                     | **No**                             | **No**                                 |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                                | **No**                                                                     | **No**                             | Yes (merge after independent ModuleIR) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is a diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Current differences after P3 landed
> are as follows:
>
> - **Row 1 has been fixed**: all five entry points share the same `frontend/proof_execution.rs`
>   implementation (pipeline + four orchestrator entry points), so the sole-consumer defect no
>   longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and the dead-code-family warnings. The W-code sets of four
>   paths—single-file run / multi-file run / single-file check / multi-file check—have been
>   normalized; the single source of the criterion is the shared implementation on the
>   `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential-risk defect not yet proven).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry point → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “decision #434” previously had no registration in the docs; it has
>   now been registered as RFC-039 **D57**.
> - **Asymmetry in embedded std inclusion (a new fact outside the table, added on 2026-10-09):** the
>   single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required for
>   for-loop desugaring; #117 is a hard switch), whereas multi-file `discover` previously recognized
>   only explicit `use` statements—project-local for loops compiled successfully and failed at
>   runtime with E6006, as confirmed by a probe. This has been fixed and closed under WBS 4.10.2.
>   The related “multi-file lacks a single-file step” parse hard-abort quirk was registered as WBS
>   4.10.1 and **has also been fixed** (immediately after 4.2.2; the Check path was degraded to
>   per-file collection. This audit method examines “stage coverage and field consumption”; this
>   case is a difference in compilation-unit membership and therefore falls into a missed
>   dimension).
>
> - **Embedded std registration-surface coverage (construction finding in 4.2.5, already fixed):**
>   the Registry arm repeatedly harvested registrations for the embedded std unit via
>   `extract_module_info`, displacing the native half registered through `with_std()`
>   (`result.is_err`, etc. were lost → std.test falsely reported E1043). This is the audit’s third
>   missed dimension: the merge semantics of “two registration sources for the same module key.” The
>   single-file path forms its registry only once through `with_std()`, whereas the multi-file/Check
>   Registry arm inserts unit by unit and therefore achieves coverage.

**Two points require precise wording; otherwise the description will be incorrect:**

- **The exact conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path (`run_project`
  → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path (`run_file` →
  `Pipeline::run`), it does. The cause is `compile_project`: it packs the complete `TypeCheckResult`
  (including `warnings`) into `type_results`, but the function has **no `result.warnings` read site
  anywhere**; the only downstream use of `result` is `generate_ir_with_context`.
- **The entry-point `main` criterion for row 11 also has a different source**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, which checks only whether a manifest exists). The
  two functions give different answers to “which files must define `main`?”

### Correctness Vulnerability: Proof Obligations Are Silently Discarded (Complete Evidence Chain)

**This is the core of the document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for obligations.**

`src/frontend/core/sema/layers/predicate.rs:232-239` is the only location in the entire repository
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

Its semantics are: “The predicate arguments of this refinement constraint are compile-time literals,
so the proof function must actually be executed to determine the result.”

**Step 2 — The consumption point is inside the checker, but it only emits and does not consume.**

`src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                           | Behavior                |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → derives a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Derives a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Derives a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no fallback, no silent pass.` ……
> `This branch therefore becomes a real line of defense rather than a forward-looking placeholder: silent pass must not be allowed to return.`

In other words, the author knew that “not producing a diagnostic” was a defect and explicitly
classified it as a **known intermediate state with nowhere to go**, assuming by design that “someone
would eventually come to read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.**

Inside `check_module_impl`, a local `proof_calls` is declared and collected (originally
`checker.rs:1234-1235`), then written into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof-function obligations` (originally `:1439`). The
field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.**

In production code, `TypeCheckResult.proof_calls` (`types.rs:29`) is read in exactly one place:
`src/frontend/pipeline.rs:187` (passed as an argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(The other repository-wide matches for the `proof_calls` identifier fall into three categories and
are not consumers of this field: the three production-side collection sites in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); the identically named field on `ProofResult`
in `verdict.rs:61`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator’s four entry points passes through that layer.**

The four entry points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module`
(`1374`)—**do not call `pipeline.rs`**. They invoke `TypeChecker::check_module` directly (`124` /
`493` / `1386`). Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discard path.**

`compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible for
compiling the embedded std. This means that **the embedded std module’s own proof obligations are
likewise never executed**.

**Step 7 — Consequence: constraints silently pass.**

`y: Sorted(3) = 5` (`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs
successfully, and produces no diagnostic** on the multi-file, check, and LSP paths. The proof
function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layer.**

When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`), it
contains:

```rust
// Formerly checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer’s `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the `5164` branch’s promise that “silent pass must not be
allowed to return,” and it is **independent of the orchestrator problem**—even if the entry-point
layer is completely fixed, this point will remain silent. It should be handled in the same batch as
the obligation ledger because it is the same class of problem.

### Why the Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero matches for the
three keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

RFC-027’s three unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop immediately before `pipeline.rs`**.

The test file’s own doc comment already states this. `rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so cases that violate refinement are at the .yx layer`

**This is precisely the shape of the problem: the tests verify that “the obligation has been
populated,” while the bug is that “the consumer never reads it.”** A test that covers only the
producer and not the consumer is inherently immune to this class of defect.

### Stronger Finding: The Subject Corpus of the Equivalence Criteria Uses Only the Single-File Path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
of the 293 `.yx` corpus files under `tests/yaoxiang/` as its **third-level criterion** and as the
primary acceptance mechanism for C2 stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero glob matches).
- Consequently, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone`
  branch (`614-616`) for every corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` provides
direct evidence of this fact. The file is marked `// expect: compile-error E4018` (line 15), and its
header comment states on line 7, “Status: ❌ Should be rejected at compile time”—**it passes
precisely because it takes the only path that executes the proof function**.

**Conclusion: The third-level criterion needs an additional multi-file corpus layer; otherwise, it
cannot serve as the acceptance tool for this document.** See “Implementation Highlights” S1.

### Obligation Fields: 13 Fields That “Production Implies Contract,” with No Mechanism to Guarantee It

Among the fields of `TypeCheckResult` in `src/frontend/core/sema/types.rs:16-76`, **13 fields** have
doc comments explicitly stating that they are “produced by stage X → consumed by stage Y.” In other
words, they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: the definition line means the **declaration line** of the field in
> `types.rs` (the `pub` / `pub(super)` line), not the first line of its doc comment—the comment
> block for a given field may span multiple lines, and only the declaration line provides a unique
> anchor. The **struct range** in the sentence above this table
> (`src/frontend/core/sema/types.rs:16-76`) moves with the struct’s line numbers and is mechanically
> reconciled by gate **F5** (an incorrect value turns the gate red); definition-line numbers are
> reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan`
> | `38` | ownership → IR generation (**`PlanId`-keyed**: read at `ir_gen.rs:360`
> (`type_result.release_plan.clone()`), used at `:3103` (`ReleasePlan::consume`);
> settlement-completion marker at `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> at `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Reclassification record (3.4.9, user decision A, 2026-10-13):** The original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**. By design,
it is consumed within the phase by typecheck and has zero cross-stage reads; it is not an
obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), and the read-only accessor
  at `checker.rs:214-217`, with injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger’s own registration and documentation remain
  (removed in this round).
- **Disposition**: delete the **declaration** in the doc comment that says it is a “mirror for
  consumption by the middle side/LSP”; retain the field and its in-phase consumption. The field is
  registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs`, with its rationale. If LSP
  genuinely needs the table in the future, it should be registered anew under 4.6.x; **this round
  must not invent a consumer**.

**All these fields are passed across layers keyed by `Span` or name tables.** The
equivalence-criteria document already points out that a mismatch in a span-keyed contract can “fail
silently, with no error whatsoever” (`ReleasePlan` is a typical example: if the two sides calculate
spans differently in any respect, the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module lies on every entry point’s mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator’s four entry points do not pass through the
  `pipeline.rs` layer at all.

**Verified fact:** The repository currently has **no mechanism whatsoever** that guarantees that
these 13 fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride
with IR generation.

### Non-Obligation Field Registry: Channel / In-Phase / In-Tree-Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)** individually registers every `TypeCheckResult` field that is
not a cross-stage obligation (field name + rationale). It is complementary to the “§ Obligation
Fields” table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (zero matches in `src/middle`)                                                                                             |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not “production implies contract”)                                                                             |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but its doc comment does not declare a cross-stage contract ⇒ not brought under the ledger                       |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings`—no declared contract                                                            |
| `semantic_db`             | Consumer is LSP (from 4.6.1, externally delivered through the `DriverOutcome.semantic_db` channel, src/lsp/server.rs:482); not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at ir_gen.rs:4336/:6867, but its doc comment does not declare a contract                                                                    |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                   |
| `interface_impl_registry` | In-tree fact (3.4.9 user decision A; see the reclassification record in “§ Obligation Fields”)                                                                |

**Why not “wire up a consumer”**: Creating downstream consumers for these fields would invent false
obligations and disguise fields that do not require cross-stage consumption by design as contract
defects. The correct closure mechanism is **reverse containment**—the `TypeCheckResult` field set =
obligation inventory ∪ non-obligation registry (field names and rationales included). The criterion
1 of `scripts/ci/check-obligations.py` mechanically enforces this: adding a field without
registering it, deleting a field without synchronizing the registry, omitting a rationale from the
non-obligation registry, or overlapping the two tables all turn the gate red immediately.

### Four Additional Contract Defects in the Proof Layer

The following findings come from an analysis of the proof layer. They are independent of the
entry-point divergence, but all belong to the same class: “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation; the `equivalence` layer is not in the
pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual invocation order of
`checker.rs::check_module_impl` (positions are anchored by function names/driver points; line
numbers follow the current tree):

| Actual order | Call site                                                                                                                                                                                                                               | Corresponding layer |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` carries the E2030 semantics itself; `judge_refined_obligation` is the sole decision unit shared by call-site and return-position obligations) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                         | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                            | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                                    | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                                                                                                  | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
the header comments of the respective modules and do not correspond to the invocation order. The
README has no mapping from “number → invocation point.”

`equivalence` is used only as an `is_subtype` utility in `inference/assignment.rs:17` and has
nothing to do with `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds each termination error and continues downward. “A lower layer
failing means upper layers do not run” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** The hard-failure panic was eliminated by P3’s 3.3.1 (the SOLVER slot
> was made optional, with absence conservatively degraded to `SMTResult::Unknown`); “silent skipping
> (no injection)” was given a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is to be performed under **RFC-039 D58**, at the actual fix site
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> entry point). The `default_solver() → &'static` wording in the 02 change list is superseded by D58
> (a bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                                    | Acquisition strategy                                     | When the solver is unavailable                             |
| ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                                                                        | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected at construction time via `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection with `with_solver_owned`        | `None` → **silent skip** (no injection)                    |
| `ownership.rs:627` (back-edge-cut decision)                                                                 | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver kernel’s capabilities’.” By contrast, the contract documentation at `backend.rs:60` says
“`None`: the backend is unavailable……**the caller should degrade conservatively**.” **Two
philosophies coexist in the same codebase, and the hard-failure path panics instead of returning an
error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**

`ownership.rs:627` calls `default_solver()` on the hot path of back-edge-cut decisions. Meanwhile,
`src/frontend/core/sema/proof/smt/backend.rs:67-72` contains:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call invokes `Z3Backend::new()`, creating a
new Z3 context. The `Z3Backend` cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the doc comment at
`z3_backend.rs:17` states that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache never hits in this invocation mode**.

(For comparison, the `LazyLock` at `predicate.rs:34-36` is genuinely a singleton. The same backend
has two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match actual behavior.**

The two `body_checker` fallbacks in `check_module_impl` (originally `checker.rs:1283-1287` and
`1289-1293`) are:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership checking receives an **empty type ledger and empty call
table** and proceeds normally—ownership analysis degrades to “nothing conflicts,” **with no warning
whatsoever**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says that termination checking runs “after type checking but
before constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally at `:1256`, ownership at `:1296`)—**the “before” in the
comment is actually “after two layers.”**

### Current wasm State: Carried by a Shim Crate, with 29 Files Containing Live Branches

The wasm target **has been built and is built in CI**. The `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                       | Evidence                                                                                                                                                                                                                                                                                |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate contains only an `rlib`                     | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                      |
| **The shim crate provides the `cdylib` + `wasm-bindgen`**  | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                      |
| The shim depends on the main crate (`rlib`) as its library | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                             |
| The main crate has a wasm-target dependency section        | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                         |
| The shim directory is excluded from the main workspace     | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                |
| **CI has four wasm builds**                                | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, namely the playground), `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten       | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL; if missing, degrades to a warning)                                                                                                                                                             |
| **29 files** contain the literal `wasm32`                  | **22** have actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (see the “F2 determination” section of `06-cleanup-inventory.md` for the inventory and item-by-item determination)                                                                      |
| `orchestrator.rs` has 16 occurrences                       | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                    |
| `src/lib.rs` has 11 occurrences                            | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                          |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`), while the shim takes a different path.

**This also yields a fact directly relevant to this document:** the wasm playground entry point
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and passes
it to `Driver::run` (the JS-side entry point `run_code`, `:122`)—the **single-file path**.
Therefore:

| Path consuming `proof_calls`                              | Executes the proof function? |
| --------------------------------------------------------- | ---------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                      |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                      |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                      |
| Multi-file `run` (`compile_project`)                      | **No**                       |
| `check` (`check_project`)                                 | **No**                       |
| LSP within a project (`check_source_in_project`)          | **No**                       |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** When the Driver is unified, this path is
already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the same
single-file stage table as `SingleFile`, `:135`); see row **4.2.8** of `09-execution-wbs.md`
(`:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 determination” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time, while six `not(wasm32)` arms are load-bearing in the three native stage tables; there
are no low-risk items that can be deleted directly. RFC-039’s open question “create a wasm target,
or delete these branches?” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustively Enumerable, No Runtime Registration)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum; runtime registration is
prohibited.** The reason is that Rust’s exhaustive `match` can enforce at compile time that “a new
stage must be handled by the orchestration layer”—exactly the mechanism already proven effective by
the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency verification          Project
    Discovery,          // File discovery                           Project
    Parsing,            // Lexical + syntax analysis               PerModule
    Registry,           // Module-registry construction             Project
    RoleClassification, // Role classification (Script/Bin/…)      Project
    Typecheck,          // Type checking (including proof layer)   PerModule
    DeadCodeAnalysis,   // Dead-code-family analysis               Project
    ProofExecution,     // Compile-time proof-function execution    PerModule
    GlobalSlotAlloc,    // Global slot allocation                  Project
    IrGeneration,       // AST → ModuleIR                          PerModule
    Monomorphization,   // Monomorphization                         Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When a variant is added, both this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`:** `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module, and which must run once at project scope” into a
type-level fact. Stages with `Project` scope are structurally guaranteed by `dispatch` to run
exactly once. This eliminates questions such as “should `allocate_global_slots` be called once per
file?” that currently require human reasoning in `orchestrator.rs`.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07):** Two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved **after** `IrGeneration`: monomorphization consumes the IR
>    artifact (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the draft order contradicted
>    the data flow.
> 2. The Check-form stage table was normalized to the `Stage::ALL` topological order, with dead-code
>    analysis **before** proof: `check_project` currently runs proof before dead-code analysis, and
>    the two have no data dependency, so their diagnostic sets are identical (C2 set semantics).
>    Only the within-file diagnostic order is normalized. The single-file path (dead-code analysis
>    embedded in typecheck, before proof) already conforms to the ALL order and is unaffected by
>    byte-for-byte acceptance.
> 3. `Parsing` was moved **before** `Registry`/`RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    artifacts. The draft order would force the Registry arm to “hide parsing,” making the stage
>    table lie about the data flow; it also reduced multi-file parsing from two passes to one. The
>    current state is registered faithfully as well: multi-file parse errors used to cause a hard
>    abort (the `?` propagation in `build_registry_from`, whose tension with CollectAll semantics
>    was registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 during
>    zero-caller cleanup, and the historical account is retained here)—**4.10.1 has been fixed**
>    (user decision on 2026-10-09: Rust-style collection semantics + plan B): on the Check path,
>    parse failures degrade to per-file diagnostic collection, and the defective file exits the
>    compilation unit (it does not enter the registry, and importers report E5001). The MultiFile
>    path retains hard abort behavior (under FailFast, a defective file cannot produce IR; this is
>    correct semantics and is pinned as a long-term invariant).
> 4. The Check form’s implementation (4.2.2, user decision on 2026-10-09) added two registrations:
>    a. Data-dependency edge 14→16: `RoleClassification` consumes Discovery’s `used_by` edge set,
>    and `DeadCodeAnalysis` consumes Discovery’s W1006 shadowing events. The initial draft omitted
>    these (in the orchestrator era, these artifacts were produced by `discover_with_used` but
>    discarded by the `discover` wrapper, so they disappeared from the data-flow table); b. The
>    normalization of within-file diagnostic order was extended to the full form in revision note
>    #2: E3020 entry validation was moved into the RoleClassification arm (Check has no Linking
>    stage, so the arm holding `surfaces` and the AST performs validation). The within-file
>    diagnostic order = stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof).
>    The diagnostic set is unchanged (C2 set semantics); only the order of stderr entries changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal:** Turn “a field was produced but nobody consumed it” from an undetectable condition
into a fact that can be asserted at compile time or test time. RFC-039 already lists this as “the
single highest-value item.”

```rust
// src/driver/obligations.rs
pub struct Obligations {
    pub proof_calls:             Vec<ProofFunctionCall>,     // types.rs:29
    pub release_plan:            ReleasePlan,                // types.rs:38
    pub escaped_refs:            HashSet<String>,            // types.rs:40
    pub instantiation_requests:  Vec<InstantiationRequest>,  // types.rs:42
    pub existential_coercions:   Vec<ExistentialCoercion>,   // types.rs:44
    pub overload_resolutions:    OverloadResolutions,         // types.rs:62
    pub try_expr_impls:          Vec<(Span, String)>,        // types.rs:59
    // …remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces one W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers):**

| Case                                            | Criterion                                                               | Handling                                                                                |
| ----------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The corresponding field in `Obligations` is `take()`n / marked consumed | Pass                                                                                    |
| The obligation is non-empty and has no consumer | The field is non-empty and not marked consumed                          | **Produce a diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| The obligation is empty                         | The field is empty                                                      | Pass (nothing to consume)                                                               |

**Why W level first rather than E level**: Fixing obligation discarding will **change the diagnostic
set**. C2 requires that “the diagnostic sets of all entry points are identical.” If the severity
were raised directly to E level, `yaoxiang check` would suddenly produce a large number of
previously silent `Unproven` diagnostics. Splitting the work into two steps—first W-level
observation, then promotion to E—keeps every step verifiable against the criteria. See
“Implementation Highlights” S4.

**The call site of `assert_drained()` is unique:** `Driver::run` invokes it at the end of the stage
table, before producing `CompilationResult`.

**Companion static gate:** RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appearing at least twice but with no read in a third file causes failure). `assert_drained()` is a
**runtime** gate, while the script is a **static** gate. The two are complementary, and both are
required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed):** This note registers the
> implementation form of the sketch and three-tier table above, the measured figures, and the 4.3.2
> interface, item by item against code file:line. The landing commit is `1941cc47a` (verification
> tree `99079bc52`). It is also the **third** of the three places that enforce the red-list’s “no
> silence” rule (the other two are the audit surface and the static gate).
>
> **1. Landed artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201, field
>   name/consumer stage/red-list status), **non-obligation field registry `NON_OBLIGATION_FIELDS`
>   (:150, eight entries with rationales)**, delivery-marker ledger `DeliveryLedger`, pure
>   settlement function `settle_obligations` (:577), external emission `emit_obligation_diagnostics`
>   (:654), non-consumer gating `consumer_visible_unit` (:538), skip criterion
>   `consumer_stage_skipped` (:549), IR-generation delivery set `IR_GENERATION_DELIVERED` (:285),
>   and W1090 code-string constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of `Driver::run`’s stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one makes the runtime W1090 fire):
>   ProofExecution at `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration at `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—all four use the
>   same delivery point: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization at `arms.rs:936` (single-file direct
>   request-table read) / `:1221` (MultiFile·Check: aggregation in the Linking arm, with
>   `driver/helpers.rs:295` reading each unit’s `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, with the rationale in the preceding lines :26-31—the settlement granularity
>   is “unit × obligation,” with no source location to which the user can point, so no dummy span is
>   inserted); text for six languages is in `locales/*.json`. **Emission severity**: from 4.3.2
>   onward it is **error** (same code promoted to E, as described in the third section)—determined
>   by the explicit `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code
>   number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two added tiers**
>
> | Case in the three-tier table above              | Implementation criterion                                                                                                              | Code                   |
> | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (the correspondence between markers and actual reads is reconciled by the static gate; see section five) | obligations.rs:618-623 |
> | The obligation is non-empty and has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see the third section)                                       | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                                                      | obligations.rs:587-591 |
>
> Two **added tiers** (user decision on 2026-10-13, not included in the sketch):
>
> 1. **S1**: The obligation’s consumer stage is not in this `ProgramKind`’s stage table → **not
>    judged as under-settled** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: The consumer stage was skipped due to `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not judged as under-settled (obligations.rs:598-602). `NoObligations` does
>    **not** qualify for an exemption—if an obligation is non-empty while the stage declares that it
>    has no obligations, the ledger is imbalanced and must be reported.
>
> **3. External-emission policy (decision “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: This mechanism externally emits **W level only** (W1090); there is
> **no strict switch**. **4.3.2 (final state, user decision A, 2026-10-13)**: **The same code is
> promoted to E**—the `W1090` code number remains unchanged, but the emission severity is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (two codes for the same fact would constitute parallel representations). **The
> promotion changes only the emission severity, not the emission conditions**. This is a **D-class
> breaking item under D59** (a change in externally observable behavior), registered faithfully:
> healthy programs emit nothing (no diagnostics are emitted when all obligations have delivery
> markers), so existing corpora and user programs are unaffected (342/342 “difference: empty”);
> however, when a run actually encounters an undelivered obligation, the diagnostic **counts toward
> `error_count`**, is rendered as `error[W1090]`, and produces a nonzero exit code—unlike the
> previous behavior in which “a W-level diagnostic did not block compilation.” The decoupling
> between code family and emission severity is registered in RFC-013’s “Emission Channel” section (a
> code prefix determines only numbering ownership; severity is determined explicitly by the
> construction site’s severity). Mechanism facts (measured): the severity comes from an explicit
> `.severity()`; otherwise, “a code beginning with `W` → Warning” (`codes/builder.rs:186-203`), and
> the registry’s `ErrorCategory` **does not participate** in severity. Exit status is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`); **because W1090 explicitly uses error severity, it follows the
> `error_count` path from 4.3.2 onward**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain’s decision C on
> 2026-10-13; the current red list has reached zero, as described in section four), for a
> reproducible reason:
>
> - Decision A (externally emit the red list at W level): measured cost: `interface_impl_registry`
>   is populated from typecheck’s prebuilt table and is **always non-empty**. Emitting it per
>   compilation would produce a **constant signal** with no discriminating power and would break
>   three existing external contracts—“zero diagnostics for healthy programs / zero diagnostics for
>   LSP / `warning_count == 0`.” A `cargo test --lib --all-features` run found **12 existing test
>   cases turning red**, **8 of them on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other four were under `src/driver/tests/`). The
>   corpus baseline changed in 24/342 fixtures, and the only viable modification was “filter W1090
>   out of the zero-diagnostic assertions,” which weakens an existing criterion.
> - Decision B (downgrade to Info level) would still break 9/12 list-style assertions, and Info is
>   not “emitted according to the decided severity.”
> - Decision C, as measured: **zero changes to existing test cases** and **zero changes to the
>   corpus baseline** (342/342 “difference: empty, behavior equivalent”). Runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains silent throughout, and removing
>   any delivery marker makes it fire (a marker-removal mutation was performed at each of the three
>   delivery points; the CLI `check` emitted two `warning [W1090]` diagnostics when a marker was
>   removed in the Check arm).
>
> The red list’s “no silence” requirement is enforced in three places: ①
> `DriverOutcome.obligations.findings`, the audit surface (emitted on every run, including WBS IDs
> and evidence pointers); ② `check-obligations.py`, which prints the red list on every run
> (currently empty) and performs stale-entry detection; ③ this note.
>
> **4. Current red-list state (cleanup and closure) and landing records for 3.4.4/3.4.5/3.4.9**
>
> The red list went from 3 → **0**; all three cleanup tasks landed on 2026-10-13:
>
> Note: The line anchors below record the file state **as of 2026-10-22**. After P5 (2026-10-26)
> split `checker.rs` into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, line
> numbers in `checker.rs` shifted as a whole. The anchors below no longer correspond to the current
> file and are retained only as historical citations.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place AST-form detection on the IR
>   side: `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST form
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`), with the call site
>   at `:7496` replacing the entire Call with `CreateVariant`; the function’s doc comment explicitly
>   states that “span keys are unreliable across systems (the two sides interpret the span of a Call
>   node differently), so detection is performed in place”—**wiring up a consumer would reintroduce
>   the already-fixed span-mismatch defect**. Deletion surface: the `types.rs` field, collection at
>   `checker.rs:1489-1493/:1565`, the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`; type
>   inference and all diagnostics of `infer_variant_ctor_call` remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550`, the decision point; on the unique match, `:4550` writes
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** written into
>   `TypeCheckResult` at `checker.rs:1558`. The naming fact remains expressed in a single place: the
>   candidate-name table (in phase) + span-keyed resolution (cross-stage, constructed at
>   `ir_gen.rs:380`, consumed at `:7632/:7705`); mixed definition-side names are self-computed by
>   `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user decision A, detailed
>   in the § Obligation Fields table above). What was deleted is the **declaration** in the doc
>   comment saying it is a “mirror for consumption by middle/LSP” (`types.rs:47-48`); the field and
>   its in-phase consumption are retained (`operator_interfaces.rs:623` operator lookup, seven
>   constraint-solving lookups in `inference/expressions.rs`, `checker.rs:214-217` read-only
>   accessor); direct proof of zero cross-stage reads:
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the table’s contents are an environment mirror populated from typecheck’s prebuilt table (always
>   non-empty). If LSP genuinely needs it, it should be newly registered under 4.6.x; creating a
>   consumer for a field that does not require cross-stage consumption by design would introduce a
>   false obligation into the ledger.
>
> After the cleanup, the **field census is closed by “obligation inventory ∪ non-obligation field
> registry (`NON_OBLIGATION_FIELDS`, with rationales)”**—gate criterion 1’s reverse containment
> protects this end to end (adding a field without registration / deleting a field without
> synchronization both turn the gate red immediately). The red-list **mechanism** is retained:
> stale-entry detection under gate criterion 4 and the `--selftest` `stale-red-list` fixture
> continue to cover its detection surface; any future registration must go through an explicit
> decision.
>
> In sync with the cleanup, the obligation count changed from 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / the § Obligation Fields table in this document); the gate’s `--selftest` now
> covers seven fixture categories (adding “unregistered field,” “stale anchor,” and “empty red
> list”), as seen under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2)—three hard criteria beyond the sketch’s one
> sentence**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ the “§ Obligation Fields” table
>    in this document (including the table header’s count declaration); any drift turns the gate
>    red. The `TypeCheckResult` field set must also be **exactly equal** to “obligation inventory ∪
>    non-obligation registry (`NON_OBLIGATION_FIELDS`, with rationales)”—adding a field without
>    registration, deleting a field without synchronization, omitting a non-obligation rationale, or
>    overlapping the two tables all turn the gate red immediately.
> 2. **Read evidence**: Every non-red-list obligation must have a real read in its declared consumer
>    file (`.fieldname` member access; comments/tests/strings do not count)—this makes “delivery
>    marker ↔ actual downstream read” a hard criterion (the main risk of mechanism A is precisely
>    drift between the two).
> 3. **Delivery markers**: Every non-red-list obligation must be marked at a delivery point;
>    conversely, a red-list entry must not be marked.
> 4. **Red-list stale-entry detection**: A red-list field must have zero real reads (growing a
>    consumer turns the gate red), and every entry must carry a WBS ID and evidence pointer. There
>    is also `--selftest`, with built-in negative/positive fixtures: negatives must fail and
>    positives must pass, proving that the detector is not vacuous.
>
> **6. Deviations from the sketch above**
>
> 1. The sketch’s `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains owned by `TypeCheckResult`, and the ledger records only “inventory +
>    delivery markers + settlement conclusions.” The reason is that copying them creates two sources
>    of truth (the fields originate separately from the checker and ownership layers in typecheck),
>    which will inevitably drift; the orchestration layer should not duplicate the cross-layer data
>    plane either.
> 2. The sketch’s `take()` / “mark consumed” **is not used**. It is replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The reason is that an obligation and its
>    artifact are the same data; `take()` would remove what downstream code needs. Moreover, 11
>    IrGeneration obligations have the form of “handing the complete `&TypeCheckResult` to ir_gen,”
>    so they cannot be taken field by field.
> 3. The sketch’s signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; all
>    other forms are routed **by severity** (from 4.3.2 onward, W1090 is E level)—the single-file
>    path uses `PipelineError::TypeCheck` (the external-emission arm at obligations.rs:654), while
>    multi-file/embedded record an `OrchestratorError` fault (otherwise `result.errors` would have
>    no consumer). The reason is that returning `Vec<Diagnostic>` loses form-based routing—check
>    diagnostics must retain their per-file attribution.
> 4. The first tier of the three-tier table changed from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is delegated to the static gate
>    (criterion 2 in section five).
> 5. The second tier’s “promote to E level in `strict` mode” **does not use a strict switch**. It is
>    replaced by **promoting the same code to E in 4.3.2** (it is promoted to E even without a
>    strict switch because zero emission in the healthy state is already sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but is not read in
>    a third file” to the four criteria in section five.
> 7. The two added tiers omitted from the sketch (S1, skip exemption) are described in section two.
> 8. The unit-test name `test_obligations_drained` mentioned in the S1/S4 table actually landed as
>    **20 cases** in `src/driver/tests/obligations.rs` (first an `#[ignore]` skeleton, then turned
>    green; red-state archival record: removing the sole settlement point yields `pairs: 0` and six
>    failures). The S1/S4 rows have been synchronized to the actual file.
>
> **7. Landing record for 4.3.2 (severity assignment + gate hardening)**
>
> - **Per-field review inputs**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   drained in a given run, available per run) and the five counters (the conservation identity in
>   section two); together with the audit-surface `findings`, these establish the fact of “whether
>   this field has ever been consumed by any entry point.”
> - **Severity: same-code promotion to E has landed** (2026-10-13, user decision A; see the
>   final-state registration in section three). Implementation: `to_diagnostic` explicitly calls
>   `.severity(Severity::Error)`; external-emission arms route by severity to the error channel.
>   Mutation evidence: removing any delivery marker causes compilation to fail in that form, and
>   `check` to exit nonzero (`error [W1090]`); restoring it returns to zero emission in the healthy
>   state (342/342 corpus unchanged). The locale text explicitly states in six languages that “the
>   code belongs to the W family, but the emission severity is error.”
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: The gate now parses `file:line` anchors in non-obligation
>     registration rationales and red-list evidence and validates that “the file is locatable
>     (uniquely) + the line number is registered + the symbol immediately following the anchor
>     actually appears in the anchored file.” An anchor pointing to a nonexistent symbol turns the
>     gate red. It includes negative fixtures `tests/baselines/obligations-gate/stale-anchor/` (one
>     nonexistent symbol and one out-of-range line). Note: The red list is currently empty, so the
>     criterion’s actual landing points are **non-obligation registrations** and future evidence
>     pointers (the same code path).
>   - **F2/R3 reverse containment**: The gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registry count = difference”; a nonzero difference turns the gate red.
>     It includes the negative fixture `unregistered-field/` (a new struct field not registered).
> - **Informational item F3**: The `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain a `type_result`, so the obligation
>   does not exist in the first place). This tier’s semantics are pinned by a synthetic `State`
>   case, and the real end-to-end path covers only `ConfigDisabled` (t3 T06).
>
> **8. Closure of the 4.3 secondary task (red-list zero + final E-level obligation diagnostics)**
>
> **8.1 Three-round landing timeline** (each item corresponds to code file:line; this round only
> records conclusions and changes no code)
>
> | Round                     | Conclusion                                                                                                                                                                                  | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
> | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger launch       | `State::assert_drained()` as the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                 | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; external emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                    |
> | 3.4.4 Cleanup             | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place AST-form detection on the IR side                                                      | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.5 Cleanup             | `method_overload_ir_names` cross-stage mirror deleted (in-phase candidate-name table + `overload_resolutions` retained)                                                                     | Resolution consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                             |
> | 3.4.9 Cleanup             | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirror for middle/LSP consumption” declaration, retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under decision A); in-phase consumers `src/frontend/core/sema/operator_interfaces.rs:623`, seven sites in `src/frontend/core/sema/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor at :214-217); registry `src/driver/obligations.rs:150` |
> | 4.3.2 Severity assignment | **Same-code promotion to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; external emission routes by severity to the error channel (first failure takes priority)  | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/severity decoupling registered at `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                          |
>
> **8.2 Closure conclusions**
>
> - **Red list reduced to zero**: The number of entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) whose `red_list` is `Some` = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The **mechanism is
>   retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, stale-entry detection under gate
>   criterion 4, and the `stale-red-list` fixture; the gate prints “red list: 0 entries—cleanup
>   complete.”
> - **Final obligation-diagnostic severity is E level**: The **code number and W-family membership
>   of `W1090` remain unchanged**, while the emission severity is `error` (D5 final state,
>   2026-10-13, user decision A). This **is a D-class breaking item under D59** (a change in
>   externally observable behavior), registered both in section three of this note and in RFC-013’s
>   “Emission Channel” section; **zero emission in the healthy state ⇒ no impact on existing corpora
>   or user programs** (342/342 “difference: empty,” zero IR snapshot drift; t11 independently
>   verified and t12 reviewed).
> - **Field census is closed**: The `TypeCheckResult` field set = obligation inventory ∪
>   non-obligation registry (eight entries, `src/driver/obligations.rs:150`, each with a measured
>   consumption rationale); gate F2 explicitly prints the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises are left**: All hardening items registered in this note have
>   **landed** (F1/F2/F4/F5, see sections five, seven, and eight). The only two remaining open items
>   are the **known boundary** in 8.4 and t7 F5, which was converted to an issue; neither is an
>   unimplemented criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/review round**
>
> | Source        | Finding                                                                                                                                                                                    | Status                                                                                                                                                                                                                                                                            |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence consists of plain-text pointers and can become silently stale when the replacement consumer is removed                                                                   | **Landed**: gate F1 anchor validation (file locatable + line registered + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                                |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, so adding a field without registration silently passes                                                                                         | **Landed**: explicit F2 difference output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                                 |
> | t4 R4         | W-only versus promotion to E was not registered                                                                                                                                            | **Landed**: sections three and seven of this note + RFC-013 “Emission Channel” + W1090 help and error_output examples in the six `locales/*.json` files                                                                                                                           |
> | t7 F5         | Dead-code analysis incorrectly counts references to overloaded methods, falsely reporting legal overloads as `W1005`                                                                       | **Converted to issue `#450`**, assigned to WBS **4.9 correct reference-pool implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—outside this secondary task                                                                                      |
> | t11 F7        | Under the gate’s `--root` mode, F1 does not cover anchor commitments in module-header prose comments                                                                                       | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                      |
> | t11 F8        | Verifier-side self-record: mutation-driven validation once wrote to the main repository (immediately cleared with `git restore`; output from the affected round was invalidated and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                                      |
> | t11 F9        | An intermediate main-repository state (unstaged files causing this document’s table header to diverge from `INVENTORY`) triggered a red gate                                               | **Informational / resolved**: the frozen commit is self-consistent and passes; “committing an intermediate state directly would turn the gate red” is precisely F2/F4’s intended behavior                                                                                         |
> | t12 F12-1     | This document gave the struct range as `16-73`, while the actual range was `16-76` (three comment lines added in t9 without mechanical reconciliation)                                     | **Landed and mechanized**: range corrected to `16-76`, and new gate **F5** added (range anchor; an incorrect value turns the gate red); related figures were aligned by grep (`17 → 20 cases`, `16 span-keyed fields` → 13)                                                       |
> | t12 F12-2     | Settlement in the multi-file form once displaced the first failure (the type error in `zzz.yx` was overwritten by W1090)                                                                   | **Fixed** (the captain amended the commit message to explain this): `failure.is_none()` guard at `src/driver/obligations.rs:686`; the regression was caught by the **existing pinned case** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1’s scanning surface**: Anchor validation covers only text in **registered
>   entries**—non-obligation registration rationales (`NON_OBLIGATION_FIELDS`) and red-list
>   evidence. **`file:line` commitments in module-header prose comments are outside the scanning
>   surface** (no registered entry is currently affected). If such an anchor is added to a module
>   header in the future, it must either **be covered by human review** or **have F1’s scanning
>   surface expanded**—one or the other; it must not be left unowned by default.
> - **The gate’s static boundary**: Collection-level validation of delivery markers is static
>   (criterion 3), while removing a marker from a single arm is covered by **runtime
>   settlement**—the static + runtime two-layer design is intentional, not a gap (t11’s
>   M11A/M11B/M11C mutation verification in both directions).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed):**
> Of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been converted to **structural identity keys** (`PlanId`) under
> RFC-039 **D20**. This note registers the implementation form, criteria, classification basis, and
> known boundaries; definition lines are still reconciled by gate **F4** (the line numbers in the
> two corresponding rows are current code facts).
>
> **1. Implementation form (carrier choice A: `PlanId`)**
>
> The key is a **structural identity** that travels with the AST node, rather than relying on the
> positional coincidence of “both sides calculating a span independently”: `PlanId`
> (`src/util/plan_id.rs`, monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → ownership of the release plan and its consumption) and `Expr::Call.plan_id` (call-site
> identity → production and consumption of overload resolution). `Expr::call` is the repository’s
> only call-expression constructor; all synthetic calls outside the parser are recorded as
> `UNPLANNED` (**not consumable**, as described in section two).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**reading and accounting occur in
>   the same invocation**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity comes
>   from `walk_stmt`’s set/restore operations + the CFG-node-to-statement-identity table; a
>   scope-exit `Drop` belongs to the scope’s final statement).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in the same
>   object**), produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with
>   dispatch transparently passing through from `Expr::Call { plan_id }`), consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) via `OverloadResolutions::get` (a
>   hit is recorded).
>
> **2. Consumption-required settlement (reusing the 4.3 ledger rather than creating a second one)**:
> The consumption ledger has a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each of the two tables. When
> `generate_module_ir` returns successfully, both ledgers are marked as having “completed one full
> pass” **at the same point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload
> resolution); the three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point connects one row for each obligation—a
> nonempty difference is **E-level W1090** (same-code promotion to E, as described in section
> three). The exemption criterion is attached to the **ledger’s own completion marker**, not to
> whether `unit.ir` exists; consequently, the `check` form’s removal of the IR artifact does not
> constitute an exemption.
>
> **3. D59 classification basis (class C, red before green)**: Before the change,
> `Type.method(positional argument, name = value)` **silently produced the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> candidate was discarded and resolution fell back to the last definition-order binding). After the
> change, the same program produces a compile-time
> `error [W1090] ... produced key was not consumed`. The chosen fix is to be **loud** rather than to
> “put the resolved name into the call with named arguments”: the latter would lose the named
> argument and merely replace one form of silence with another (final-state semantics are documented
> in [09](./09-execution-wbs.md) §3.14 / issue #452). Moreover, `Drop` instructions are a no-op in
> the interpreter (the wildcard arm at `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating release-plan keys does not change runtime behavior—the 342-case corpus
> differential being empty is empirical proof of this.
>
> **4. Criteria and evidence**: The D41-equivalent criterion `test_release_plan_spans_consumed`
> (settled by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + an evidence floor**:
> number of files that produced resolutions `≥ 1`, number of genuinely consumed entries `≥ 2`, no
> allowlist). The nine pinned resolution forms are documented in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> in-type binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven verification: shifting a consumption key by
> one position ⇒ the criterion fails + a real program reports `error [W1090]` + the corpus gate
> differs; restoring it turns the criterion green again. Gate figures: `check-obligations.py` PASS,
> 342-case corpus differential empty, `cargo test --lib` 2712 passed / 0 failed (zero IR snapshot
> drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: The **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable in the current corpus and test surface** (method
> declarations uniformly go through `register_type_bindings` and therefore reach the bound
> consumption point), so the correctness of this branch’s keying has **no red state with which it
> can be falsified**. The reason for retaining it is that it is the symmetric counterpart of the
> bound path; deleting it would rewrite the “method call without `type_bindings`” form into silence.
> This is registered by independent verification (t5 F-1) and review (t6 F2), and **no new task is
> opened**.

### 3. Unified Driver: A Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP within a project; results filtered to the target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// May only be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above** (one per `ProgramKind`);
callers cannot freely pass an array. This is the key constraint that prevents the `Program`
abstraction from degenerating into a “free-form parameter that can accept anything,” and it is
asserted by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not the stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce a diagnostic; see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one may be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed):** Four implementation deviations
> from the sketch above are semantically equivalent and are recorded for reference:
>
> 1. The three-state `StageOutcome` from §4 is not returned by stage arms. It is expressed through
>    `State` failure flags + `Aggregation` gating (the Continue / Warn / Abort semantics are
>    unchanged, eliminating repetitive return boilerplate in every arm).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” determination in §5 covers the sketch’s field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-diff criterion in S2, “deliberately fix no
>    bugs,” requires this; external emission comes with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is
>    the only new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their proper location after the 4.2 migration and will be reconsidered then.
> 5. In the multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user decision on 2026-10-09): the original `compile_project` interleaved
>    proof with the per-file typecheck loop (file N’s proof ran before file N+1’s typecheck). With a
>    single failure, both orders are byte-for-byte identical. In the multiple-failure scenario “file
>    1 proof fails + file 2 typecheck fails,” the first reported error changes from a proof error to
>    a typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through separate channels according to `ProgramKind`:
>    `result` (the pipeline contract) / `module` + `failure` (the orchestrator contract, 4.2.1)—the
>    external error contracts of the entry points (`PipelineError` / `OrchestratorError`) are
>    outside the type surface that the Driver can unify.
> 7. The Check channel landed in 4.2.2: `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry, while a clean file has an empty Vec—the
>    existing `check_project` contract). Discovery’s artifacts are extended to a triple—“file set +
>    `used_by` edge set + shadowing events”—stored in State (the MultiFile form has no downstream
>    consumer; they are recorded but not emitted externally). On the check path, per-file parsing
>    drops from two passes to one following the C3 topology.
> 8. The Lsp form landed in 4.2.3, with a user decision on 2026-10-09 to route by file source: parse
>    failures for files on disk use Check’s plan B (collect + exit the compilation unit—both sides
>    degrade consistently), while parse failures in edited buffers retain a partial AST and continue
>    to typecheck (the editor philosophy—an intermediate typing state should not make semantic
>    functionality disappear). The Discovery arm retains buffer source over stale on-disk content
>    for Lsp. Typecheck under Lsp checks only the target file (other units exist only to provide
>    signatures to the registry). The principle of normalizing within-file diagnostic order extends
>    to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior in which LSP
>    hard-aborted on parse errors in unrelated disk files and the handler silently fell back to the
>    single-file path is fixed in this step.
> 9. The Embedded form landed in 4.2.4: `Program` gains a `shared_registry` field—the embedded std
>    module is a subcompilation, and its registry is an **input**, not an artifact
>    (`EMBEDDED_STAGES` has no Registry stage). The #94 SymbolTable-sharing contract is thereby made
>    explicit as a program declaration rather than the convention that “the caller must remember to
>    pass the same registry.” Error-path text is normalized from `<std.test> (embedded std)` to the
>    unit virtual path `<std/test>` (compiler-internal error surface only; no test pins it). At this
>    point, all four orchestrator entry points have migrated to the Driver.
> 10. Standalone check was unified in 4.2.5 (decision A + IR-stage decision, 2026-10-09): the Check
>     form faithfully carries single-file semantics for a program without a project root—the warning
>     surface covers only the entry file (neighboring files receive errors only, preferring omission
>     over false positives); relative `use` paths are resolved relative to the importer’s directory,
>     aligned with rustc’s single-file `mod` behavior. The CHECK stage table gains GlobalSlotAlloc +
>     IrGeneration: the old standalone path (the full pipeline) already ran IR generation, and
>     errors such as E3019/E1014/E1015 are produced only by ir_gen (confirmed by runner-gate
>     evidence). The Check form of IrGeneration performs checking without consuming IR; the
>     Script/Bin split is expressed through `module_key` being None/Some (the existing E3023 switch,
>     ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces only
>     resource-limit/internal errors, with no corpus dependency). **〔Withdrawn on 2026-10-10 after
>     3.4.8 empirical verification〕**—“zero corpus dependency” was merely evidence from the absence
>     of pathological-recursion fixtures in the corpus; E3005 is the sole compile-time defense for
>     this class of program. See revision note #14. An incidental latent defect was also fixed:
>     repeated registration harvesting by the Registry arm for embedded std units displaced the
>     native half already merged through `with_std()` (a false E1043)—embedded units now skip
>     repeated registration.
> 11. The LSP single-file fallback was unified in 4.2.6: the manual
>     lex→parse→check_module_collect_all sequence in `run_diagnostics` was removed and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     In the SingleFile+CollectAll form: Parsing collects all parse errors and retains a partial AST
>     to continue typechecking (the editor philosophy introduced by the 4.2.3 decision; the stage is
>     not marked failed, or topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now includes proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and IR-level
>     errors; lexical failures are now reported as real diagnostics (the old synthetic “E0001
>     lexical error” text disappeared with removal of the sequence—the real diagnostic has an exact
>     span, with no information loss).
> 12. `check_module` / `check_module_collect_all` were verified line by line in 4.2.7 (mandatory C5
>     item): the two entry points had already converged independently—the `mod.rs` layer uses a
>     single `check_module_inner(ast, env, collect_all)` boolean, while the checker layer uses
>     `check_module_impl(module, collect_all)`. The only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`; pass 3 and drain run in both
>     modes, with the difference carried by whether `collected_errors` is empty. There are five
>     collection points in `statements.rs` (function body/use/for/block/while). Verification also
>     exposed and fixed a duplicate-diagnostic defect: collection sites placed the first error in
>     both `collected_errors` and the `Err` return channel, and pass 3’s `add_error` for `Err`
>     produced two diagnostics with the same code, span, and message (up to three for nested cases).
>     Two independent mechanisms were also found: duplicate validation of annotation signature
>     parameters versus the complete annotation (E1003/E1103), and duplicate emission from the
>     ownership layer at the same point (E2014/E2018). The fix is at the module-result boundary:
>     deduplication by `(code, span, message)`—a diagnostic is a positional fact, and duplicates
>     carry no information. Duplicate copies were removed from 63 corpus entries, with zero drift in
>     run columns and exit codes (the apparent “duplicate” for borrow_conflict_err is an artifact of
>     the baseline triple format not including column numbers—the two E2018 diagnostics are
>     legitimately at different columns).
> 13. The wasm playground landed in 4.2.8: `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper layer no longer participates. Error text is aligned byte-for-byte with the old
>     `CompileError` Display prefix (`Parse error:/Type error:/Internal error:`), producing no
>     visible difference in the playground UI. WasmPlayground shares SINGLE_FILE_STAGES + FailFast
>     with SingleFile (the existing 4.1 declaration now has a real production caller).
> 14. Multi-file monomorphization and the CHECK stage table were corrected in 3.4.8 (user decision
>     on 2026-10-10, **partially withdrawing revision note #10’s mono exclusion**). Empirical
>     evidence disproved the claim that “mono only produces resource-limit noise”: under
>     pathological generic recursion (`f(x)=f([x])`), the multi-file path without monomorphization
>     overflows the compiler process stack (0xc00000fd—the interpreter recurses in Rust, with no
>     graceful runtime error), and the mono depth gate is the sole compile-time defense. The
>     “type-erasure fallback” assumption was invalidated by the RFC-033 reflection
>     decision—`^^List(Int)` requires the real identity of an instantiated type, and
>     monomorphization is the foundation of reflection rather than an optional optimization. Four
>     design points: ① typecheck produces an instantiation_request for qualified calls (`lib.f(x)`,
>     the FieldAccess form), with generic_id using the qualified name, in the same namespace as the
>     merged IR (currently only bare Vars are recognized, expressions.rs:2455); ② the `Stage::ALL`
>     topology changes to place Linking before Monomorphization, and the MULTI_FILE stage table
>     gains Monomorphization at its end—it consumes merged_ir and aggregates requests from all units
>     (qualifying the deferred bucket’s containing_fn), elevating mono from a per-module pass to a
>     whole-program pass. The single-file stage table remains unchanged (with no Linking, the
>     subsequence property is preserved and IR snapshots do not drift); ③ CHECK gains Linking (pure
>     merging—E3020 entry validation remains in RoleClassification to avoid duplicate reporting) and
>     Monomorphization (pure checking without consumption, like the Check form of IrGeneration); ④
>     resource protection covers cross-unit mutual recursion through whole-program BFS. The
>     `Monomorphizer` itself is unchanged (its ModuleIR + request-set consumption contract exactly
>     matches the merged-IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the design in #14: `ExpressionInferrer` receives a module-qualified
>     key table (registered by the three `use` arms—ordinary / alias / item-by-item alias, with only
>     `SubModule`-class exports included), and `callee_generic_name` is formed through
>     `SymbolTable::qualify`, using the same qualified names as the merged IR. The qualified-call
>     first-path arity criterion was tightened (when the parameter table cannot be resolved, fall
>     back to the second path and use the signature to obtain actual arguments, avoiding false E3018
>     reports caused by mismatched same-name functions across modules). Implementation empirically
>     exposed and fixed **three asymmetric defects** on the mono side in the same round: (1) The
>     deletion-key set (generic-name set) and rewrite-key set (containing_fn + name + span triple)
>     had different granularity—sites where actual arguments could not be resolved degraded into
>     symbolic requests in the deferred bucket, but the bucket never drained when the containing
>     function was non-generic, while the original item had already been deleted by name → dangling
>     call (empirically confirmed by runtime E6006 at `list_ops.yx:53`). The fix restores the gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the sole source
>     of truth to determine coverage and retaining the original item for uncovered sites. (2)
>     `HashSet` iteration order in the restoration section made function-table order
>     nondeterministic (t2 review F1) → changed to `Vec`, sorted by name, making artifacts
>     byte-for-byte reproducible. (3) Non-rewrite forms such as TailCall / MakeClosure still looked
>     up mapping keys even though the comments claimed they were handled conservatively → a
>     single-point `call_form_is_rewritten` determination now treats all non-rewrite forms as
>     uncovered. Each defect has a pinned test (red-state empirical evidence before the fix). The
>     corpus differential baseline was regenerated on the same commit, producing exactly one changed
>     line (adding `(int64)` to a remove_at frame). **Commit-strategy note**: when typecheck
>     collection and the mono fix were split into two commits, the staging tree containing only the
>     former triggered a pre-existing `stdlib_docs` defect on HEAD (`list.is_empty([])`, same E6006
>     family)—the two halves are each other’s green-light prerequisite, so they were combined into
>     one atomic commit.
> 16. Five decisions were registered on 2026-10-10 (after R1 closure): (1) **D2 decision
>     A**—user-module exports carry `generic_fn_type_params_snapshot()` through
>     `Export.type_params`, isomorphic to the std path (`yx_sources.rs:62/108`): the declared name
>     becomes the authoritative source for qualified-call monomorphization, while signature
>     reverse-inference is only a fallback (included in R2 scope). (2) **HKT declaration side live,
>     call side broken** (inferring type arguments E1002 / explicitly supplying type arguments
>     E1010) is registered as WBS 3.5 (three third-level tasks). (3) **The deferred bucket never
>     drains under a non-generic container** is included as an acceptance subitem of 3.4.8 ②: the
>     bucket must drain or fail loudly and must not silently fail to specialize. (4) Closure
>     variables called by name, causing runtime E6006, are registered as WBS 3.6; semantics (i)
>     support indirect calls / (ii) reject at compile time, with **(i) decided**—indirect calls are
>     now supported: the `CallDyn` mechanism already exists (the closure-expression path at
>     `ir_gen.rs:7945`); the break is that a `Var` callee is lowered to a name-based `Call`, and
>     typecheck already accepts such programs. (5) **The tradeoff criterion is fixed**—a rejection
>     rationale may be based only on correctness and readability; “large change surface / introduces
>     a new mechanism / high cost” is not grounds for rejection and is written into the second part
>     of `coding-rules.md` and the HOWTO self-check table. Tradeoffs requiring a user decision must
>     follow all four items in `AGENTS.md` under “Requesting a user decision (mandatory)”
>     (situation + evidence / pros and cons per option / explicit recommendation + cost / explicit
>     naming of breaking changes).
> 17. 3.4.8 ② landed and the F1 root cause was identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the design in #14: `Stage::ALL` was changed to place Linking before
>     Monomorphization, and the MULTI_FILE table gained mono at its end. The Linking arm aggregates
>     whole-program requests and qualifies `containing_fn` **before taking each unit’s IR**
>     (otherwise deferred-bucket keys and merged IR are not derived from the same source, and the
>     bucket never drains). **① was closed (D2 decision A)**: user-module exports carry the
>     declaration-time type-parameter snapshot through `Export::type_params`, isomorphic to the std
>     path (`yx_sources.rs:62/108`). **Existing defects exposed and fixed during implementation**:
>     (a) **F1**—`qualify_names` rewrote only function bodies; `ir.init` was never visited, so in
>     multi-file mode function definitions were qualified while module-level calls remained bare
>     names → runtime E6006 (**reproduces with non-generic code**, unrelated to generics). The fix
>     makes init reuse the same `rename`/`aliases` and the same `rewrite_call_names`. (b) The mono
>     side’s “whole-program unique suffix” heuristic and unmatched silent `continue` → the heuristic
>     was removed, and unmatched calls now produce a **loud E3018** (ambiguous silent mismatch is
>     worse than failure to resolve). **Independent verification caught and fixed a regression**: t6
>     found that a bare-name call after `use lib.{pick}` was falsely rejected with E3018 (the IR
>     instruction name is the alias-qualified name, while the requested name is bare, and the
>     aggregation side qualified only by unit key) → the aggregation side now **directly invokes
>     ir_gen’s** `import_aliases_from` and `resolve_call_target` (extracted into a public single
>     point, one producer and one consumer), with a matrix-pinned test covering both named-import
>     forms. **Archival note**: the blank `DefId` in the init section was empirically determined to
>     be a “consistency blank” rather than a live defect (the codegen name fallback can resolve it,
>     and all four classes of init calls were tested successfully), and was registered under WBS
>     3.7.4 to close the “function bodies use def, init uses names” policy divergence.
> 18. The init-section DefId policy divergence was closed (3.7.4, 2026-10-11, 38bd4b9e):
>     `assign_defs` was extracted into the single-point `resolve_static_targets` (string operands
>     for Call/TailCall + MakeClosure), and function bodies and `ir.init` share the same
>     implementation and the same `resolve` closure. After 208 IR snapshots were regenerated, only
>     309 `(def=_)`→`(def=dN)` pairs changed (other-lines 0; zero residual `(def=_)` in the init
>     sections across the tree). The codegen DefId main path and the name-fallback state produce
>     identical translation results and execution values item by item (all four classes of init
>     calls tested), and the corpus baseline does not drift; the probe flips under the new policy
>     and is strengthened (identity + exact index + execution value; falling back to init coverage
>     turns it red). **Erratum (2026-10-11)**: the commit IDs above were rewritten by
>     `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396 correspond respectively to
>     375789d2/d922a551/38bd4b9e after rebasing; the pushed dev history uses the new IDs (the old
>     IDs were local temporary IDs and never entered remote history).
> 19. 3.4.8 ③④ landed and were verified (2026-10-11, 8ecf56f93). (③④’s design basis is #14; ②’s
>     implementation is #17, and this entry closes it.) **Implementation**: the end of
>     `CHECK_STAGES` = `…IrGeneration → Linking → Monomorphization`; Check’s `Linking` performs only
>     pure merging (aggregating requests + `link_module_irs`), leaving entry validation entirely to
>     `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (in memory only);
>     mono failures enter the collection channel in Check and retain `fail_with` in MultiFile;
>     `into_outcome` first takes `check_diagnostics` and then checks for faults (structural faults
>     no longer clear already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + the t5 cross-form matrix) and one review round (t6) all passed; the
>     340/340-case corpus differential was line-by-line IDENTICAL, IR snapshots did not drift, and
>     the test count increased from 2643 to 2656. **Existing gaps found and fixed in the same
>     round**: F1 (in rootless multi-file check, request names and IR definition keys were not
>     derived from the same source → silent failure to specialize; see the landing note in WBS row
>     3.4.8)—the initial fix used an exact reverse lookup by actual definition key (`flat_defs`);
>     **same-day erratum**: that bridge was deleted during regression fixing, and the root cause was
>     instead resolved by decoupling the dual role of `module_key` (`script_mode`), so all three
>     sources are `{unit key}.{name}`. The regression’s location and fix (the CI
>     documentation-example gate went from 258 blocks / 10 failures to 0 failures) are documented in
>     the landing note in WBS row 3.4.8. **Archival note (existing, not a regression from this
>     round)**: the E3005 help text does not match resource-protection semantics; the span label for
>     multi-file E3005 is misplaced (reproducible with the HEAD binary); the type-argument position
>     dependency on the collection surface remains in the same category as 3.5.
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and the diagnostic surface was closed (2026-10-12, 809b9b936 +
>     8605a081b + subsequent commits). **3.9.1 (execution-layer guard is reachable)**: interpreted
>     calls were changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frames and return
>     control to the outer `step_one`; frames carry `ReturnTarget{Host|CallerSlot|Discard}`, and on
>     return write back to the caller’s slot; multiple calls in one instruction continue through the
>     frame’s `pending_calls`. Baseline measurements showed that each interpreted call consumed
>     **2544 B** of native stack, with the crash threshold at `count_down(394)` on the 1 MiB Windows
>     main thread. After the fix, 20,000 levels of recursion run on a 512 KiB stack thread
>     (approximately zero per level). The guard (`push_frame`) is therefore reachable from every
>     entry point under every `max_stack_depth` setting; `ExecutorError::StackOverflow` now carries
>     the real limit (previously `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2
>     (deep-stack echo truncation) was merged into this task by user decision** (same location in
>     the same file, inseparable at file granularity): echoes now show the first 16 frames + an
>     omitted-frame count + the last 8 frames, while shallow-stack frames remain unchanged. **3.10.1
>     (no compile-time guard for native recursion, #447)**: localization preceded the fix—the crash
>     point was recursive `MonoType::type_name` (**328 B/level**, determined with an SP probe +
>     iterative identification of each critical point). The same path also derives `Clone` (~400
>     B/level), performs deep copies in `substitute`, and includes `type_name_hash` (164 B/level)
>     and `collect_generic_type_refs`. After converting these four locations to work lists and
>     removing one deep copy of the entire request per level, pathological generic recursion under
>     `mono.max_depth=5000` produces **E3005 (with the real limit)** instead of overflowing the
>     stack. **3.11.1 (task-boundary error-code fidelity, #448)**: the task result’s error side
>     changed from a value to a **structured form** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), with values restored by value across the boundary;
>     E6001/E6003/E6004/E6008 inside tasks are verbatim identical in Standard and Embedded messages.
>     Two **pub API payload-shape changes** are marked `BREAKING CHANGE` in the commit.
>     **Commit-strategy note**: 3.9.1 and 3.11.1 are **inseparable at file
>     granularity**—`interpreter/executor/executor.rs` carries both the trampoline and the
>     error-channel producer/consumer, while `TaskFailure` is defined in `runtime/engine.rs`;
>     staging only either side fails to compile (the pre-commit staged-subset compilation rejected
>     it twice), so under the same policy as #15 they were combined into **one atomic commit**.
>     **Known boundaries and follow-up**: Drop derivation (~75 B/level) means
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation) gate currently bears
>     the native-recursion burden (each level uses ~3696 B; the default 256 levels ≈ 924 KiB), has
>     zero current call sites, and is unreachable in production; wiring it in the future requires
>     closure of the same family. The Standard-mode task interpreter constructs with the default
>     `ExecutorConfig`, so its E6004 limit is always 1024 (configuration propagation registered as
>     WBS 3.12, user decision B: create a separate team); the test-runtime budget is tracked in
>     separate issue #449 (a pathological fixture takes 76 s, runs clippy for both targets, and
>     requires a full commit run), and is outside this RFC’s construction scope. **Corpus directory
>     stratification (same round, 8605a081b)**: the 212 fixtures in `02-type-system` (78) and
>     `06-compile-errors` (134) were split into 23 category-based subdirectories; 88 IR snapshots
>     were moved in mirror-path order; the corpus baseline was regenerated on the same commit (342
>     lines, independently recalculated and compared line by line: only the `file` field and two new
>     changes), and 41 snapshots were regenerated because the file-header paths became longer;
>     line-by-line verification showed **only span-offset changes** (zero differences after
>     normalizing offsets).
> 21. The LSP semantic-data pipeline was unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references within a project carry
>     `resolves_to` pointing to the real definition. The old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics used the unified Driver pipeline, while
>     semantic data used a manual lex→parse→single-file typecheck in server.rs (with no module
>     registry, project-local `use` resolution failed, no reference events were produced, and
>     cross-file navigation/rename/reference lookup silently returned nothing—the silent failure
>     registered as D55 is closed by this task). **Implementation**: (1) `Export` carries
>     `definition_span` (harvested by `extract_module_info` from AST identifier nodes, with the same
>     source and value as the checker’s definition-event `def_span`; native std always has None),
>     and `ModuleInfo` carries `definition_file` (filled by the Registry arm from `DiscoveredFile`);
>     (2) when the checker drains reference events, it uses `import_def_locations` to redirect
>     cross-file imported items’ `resolves_to` from the `use` line to the real location in the
>     definition file (std imports have no registration and continue to resolve to the `use` line);
>     (3) the checker’s module identity changes from the unified `"<module>"` to the unit file path
>     (this value is consumed only by the semantic-data surface, with zero diagnostic
>     consumption—empirically, all four consumers of `env.module_name`); (4) `DriverOutcome` gains a
>     `semantic_db` channel—aggregated by CollectAll forms (the Lsp target file regardless of
>     diagnostic success + the SingleFile fallback), with the Lsp arm injecting the top-level export
>     definition surface for non-target files from the registry (`external_definitions`, enabling
>     navigation even for unopened files; type/submodule exports are not injected because
>     typecheck-side type registration does not produce definition events, and both sides retain the
>     same omission); (5) the orchestrator gains `analyze_source_in_project`
>     (`check_source_in_project` delegates to it and takes only the diagnostic half); (6) the
>     LSP-side `analysis.rs` single-compilation entry point replaces the manual pipeline
>     (`update_semantic_db` in server.rs was removed), and World keys are unified as normalized file
>     URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is idempotently normalized, with drive-letter
>     case and %XX encoding differences normalized to the same key). **Incidental fix (D59 routing
>     class B—old behavior did nothing silently and did not change the externally visible behavior
>     of any previously successful program)**: project-local cross-file navigation/rename/reference
>     lookup changed from silently returning nothing to hitting the real definition; when lexical
>     errors occur, the old manual pipeline discarded semantic information, whereas it now retains a
>     partial AST with coloring (the 4.2.6 editor philosophy extended); when the target file has
>     type errors, semantic data is no longer discarded along with `type_result`
>     (coloring/navigation do not disappear during an intermediate typing state). **Pinned cases**:
>     six at the driver level (target-file path key / cross-file resolves_to +
>     external-definition-surface round trip / std-import fallback to the `use` line / preservation
>     for an erroneous file / single-file fallback path key + error preservation), and one LSP
>     end-to-end case (red before green, empirically demonstrating that navigation returned None
>     before the fix), plus four URI-normalization cases.

**Required changes to each of the ten entry functions:**

| Entry point                                                | After the change                                                                                                                                                                                                        |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to the Driver                                                                                                                         |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                 |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a constructor for `Program { kind: MultiFile, aggregation: FailFast }`                                                                                                                                        |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a constructor for `Program { kind: Check, aggregation: CollectAll }`                                                                                                                                          |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver invocation + filter the result to the target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                               |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                                  |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                                     |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | Within a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and invoke `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                                  |

**The degree of duplication between `check_project` and `compile_project` (approximately 40% shared
/ 60% divergent).** What they share is: vendor-consistency verification, file discovery,
`build_registry_from` (deleted on 2026-10-26 during zero-caller cleanup; the historical account is
retained here), `all_method_bindings`, the parsing loop, per-file checker assembly, and result
handling. What differs is items 2/3/4/5/7/8/9/10/11 from the 11-item table above. **This 40% shared
skeleton is precisely the Driver’s value range**—all 60% of the divergence consists of “different
stage sets” or “different aggregation modes,” and those are exactly what the two `Program` fields
can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage has a non-blocking problem; continue
}
```

**Key design decision: “an upstream failure caused this stage to be skipped” is not a stage return
value.**

The reason is that skipping is determined by **topology**, not by the stage itself. If every stage
returned `Skipped`, then the information about “why did it not run?” would be scattered across 12
stages and could not be centrally audited. Instead, `dispatch` determines it at the beginning of the
loop according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**A `Skipped` stage must produce a diagnostic.** This rule directly targets the vulnerability
addressed by this document: the current `pipeline.rs:187` `if !proof_calls.is_empty()` is a silent
“skip.” It produces no diagnostic, merely because the obligation happens to be empty. Once this rule
is established, every case where “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                           | Text direction                                                                                                              |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                    | “Stage Y was not executed because upstream stage X failed”                                                                  |
| Caused by an empty prerequisite obligation                       | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; not counted as a warning** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y was not executed because the configuration was not enabled”                                                        |

The second category is important: it distinguishes a “normal skip” from an “abnormal skip” in the
diagnostic stream without polluting the `warning_count` of `yaoxiang check` (the non-blocking
contract on which `diagnostic/mod.rs:637-641` depends).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                               | Current equivalent |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)        | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
project-local files and single files within the same process, while CLI `run` and `check` are
independent invocations. Placing it on Program avoids having the Driver hold mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ in more than “whether they
return early”—they are two different checker entry points. After unification, the same
implementation should be driven by the `Aggregation` parameter rather than preserving two functions
(**this is an additional item requiring verification under “Known Limitations and Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types are unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                              |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) belongs to `03-type-unification.md`
(RFC-039 routing table C).

**Ownership and layering of obligation types**: What moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the types of the remaining
fields stay in their original locations. `driver/obligations.rs` holds only the aggregate container
and `assert_drained()`. L3 consumers, such as `ir_gen` reading `release_plan`, continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
Driver no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2), `middle` (L3), and `backends` (L4); **L2/L3/L4 must not `use crate::driver` in
reverse**. The presence of `Driver` in a `TypeChecker` import is a violation and is blocked by the
proposed `scripts/ci/check-module-boundary.py` in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single file) | Five stages                                 | Same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone files—it uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven by rootless mode; this row now describes only the `lib::run_file` / `Compiler::compile` library paths)                                                                                                                                                                                 |
| `yaoxiang run` (multiple files)     | No proof_execution and no W1001/W1002/W1003 | proof_execution was already connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” with Check / Lsp / MultiFile sharing the same implementation and no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for that arm to emit) |
| `yaoxiang check` (multiple files)   | No proof_execution                          | Already connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| LSP (within a project)              | No proof_execution                          | Already connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Z3 not installed + single file      | `predicate.rs:35` **panics**                | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see Compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Z3 not installed + multiple files   | Silently skipped                            | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

The **only intentional behavioral break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This is consistent with RFC-027 §8’s positioning as “not bound to a specific solver” and
with the contract already declared at `backend.rs:60` (“the caller should degrade
conservatively”)—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (one for a single file / N for multiple files)                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                          | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib.rs`                                  | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                  | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                            |
| `src/lib.rs`                                  | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                          |
| `src/frontend/pipeline.rs`                    | `141-227`                                      | Replace the `Pipeline::run` body with `Program` construction + Driver invocation                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                    | `275-278`                                      | Move dead-code analysis out and into the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                    | `306-311`                                      | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                    | `381-389`                                      | Move monomorphization out and into the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                     |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`         | `450`                                          | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`         | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                      | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                         |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics` and invoke the Driver                                                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                      | Uniformly use `Program { kind: Check }` in `check_files_with_diagnostics`                                                                                                                                                                                                                                                                                                |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                           |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                      |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                            |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` fallback path                                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/sema/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “a lower layer failing means upper layers do not run” (no short-circuit exists)                                                                                                                                                                                                                   |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                        | **Under D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure entry point; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared entry point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 supersedes the original wording |

**Deleted:** `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged:** The production logic of the `Unproven` branches (now distributed among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is at the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch (wasm
branch-reachability determination belongs to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Handling                                                                                                                                                           |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix it; stage releases separately, and document the diagnostic-set change in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that previously compiled now produce warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                     |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                     | First W, then E, in two steps under S2/S4                                                                                                                          |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no more crashes                                           | No break                                                                                                                                                           |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not pass through proof_execution                                                                                                                |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                    |

---

## Implementation Highlights

This document corresponds to **P3 (Fix correctness vulnerabilities)** and **P4 (Stage contract and
unified Driver)** in RFC-039’s global phase sequence. S1–S5 below are the implementation order
within this document. **The acceptance criterion for every step cites category **C2** (orchestration
changes) in the [equivalence-criteria document](07-equivalence-oracle.md)** (the **diagnostic sets**
of all entry points are identical + corpus behavior is identical).

### S1: Establish the Criteria (Red First)

**A prerequisite that cannot be skipped.** The vulnerability criterion must first be written as a
failing test.

| Deliverable                                                                                                                                                            | Acceptance                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                          | **Must be red**. If it unexpectedly passes, the vulnerability analysis in this document must be reviewed again           |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures containing `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions can be compared                                           |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                                     | Marked `#[ignore]`; turned green in S4 (red-state archival record: removing the sole settlement point yields `pairs: 0`) |

**Rollback point:** No code changes; additions only.

### S2: Introduce `Stage` + `Program` Without Changing Behavior

| Deliverable                                              | Acceptance (C2)                                                                                        |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array        |
| Change `pipeline.rs:141-227` to invoke the Driver        | **The single-file path’s diagnostic set and exit code are byte-for-byte identical**                    |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic lists (sorted by `(code, file, line)`), exit codes, and stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance criterion is C1/C2-level zero-diff.

**Rollback point:** `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Consolidate the Four Orchestrator Entry Points + Fix the Vulnerabilities

| Deliverable                                                                                                                         | Acceptance (C2)                                                                           |
| ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Change all of `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The four entry points’ diagnostic sets are identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                            | `yaoxiang check` behaves consistently inside and outside a project                        |
| Delete the LSP manual stage sequence                                                                                                | LSP and CLI produce the same diagnostic set for the same file                             |
| **Turn `test_multifile_proof_obligation_not_dropped` green**                                                                        | The vulnerability has been fixed                                                          |
| Fix the ownership-layer `Unproven` empty match arm (originally `checker.rs:1313`)                                                   | `test_no_silent_pass_on_unproven`: `Unproven` always produces a diagnostic in every mode  |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                          | No panic when Z3 is absent                                                                |

**Rollback point:** The vulnerability fix and the structural consolidation must be **two separate
commits**. If the structural consolidation causes problems, only the structural commit can be rolled
back while retaining the vulnerability-fix commit—in that state,
`test_multifile_proof_obligation_not_dropped` remains green. The reverse—retaining the structural
change while rolling back the fix—turns it red and is an unacceptable intermediate state that must
not be merged.

### S4: Enable the Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                                                    |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implementation form described in the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                                  |
| Promote obligation diagnostics from W to E   | After the full-corpus differential, manually review every new E diagnostic                                                                                    |

**Rollback point:** The severity of `assert_drained()` can be controlled through a configuration
option; switching between W and E does not require structural changes.

### S5: Finish the Proof Layer and wasm

| Deliverable                                                                                                                                                                                                                                                 | Acceptance                                                                                                                                                         |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                            | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                       |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout would become fixed across compilations through the process-wide cache and contaminate sharing between cargo test threads) | Production-path `default_solver()` call sites drop to zero (tests only); cache hit rate is observable across the three consumers (the counter landed in 89576fafd) |
| Add a warning to the `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`) for the degraded path                                                                                            | A diagnostic is produced when there is no `body_checker`                                                                                                           |
| Determine the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                         | The conclusion is delivered to the “F2 determination” section of `06-cleanup-inventory.md`; this document records only the need for that determination             |

**Note:** Correcting the layer order changes the diagnostic set and may expose many previously
silent `Unproven` results. **This item should proceed independently of S1–S4** and must not be mixed
into the same PR as the entry-point consolidation.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustively-enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                                | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is isomorphic to the opcode-table mechanism already proven in RFC-039 routing table B. `StageScope` further turns “per module or project-wide” into a type-level fact, eliminating the need to reason about invocation counts                                                                                                                            |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement, together with the **static** `scripts/ci/check-obligations.py` gate; **final severity is E level** (2026-10-13, user decision A: promote the same code to E without changing the code number; **implementation form described in the §2 revision note**: the ledger does not duplicate the field structures; the red list—zero after cleanup—retains the mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not a single bug: at least two related hazards have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. The W→E split was intended to keep every step’s C2 criterion usable—now that the obligation inventory has been cleaned up to 13 entries and the red list is zero, healthy-state zero emission makes the final severity directly E (see sections seven and eight of the §2 revision note) |
| **Fix scope**            | Fix the **consumer** (orchestration layer); do not change the production logic of the `Unproven` branches (now three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                                                        | The problem is at the consumer, not the producer. Changing the producer would conceal the architectural defect as “the logic has been fixed,” whereas the logic of these branches is itself correct                                                                                                                                                                                                                                                                               |

This design additionally eliminates two classes of cracks:

- **The crack between “comment promises” and “code behavior.”** The “silent pass must not be allowed
  to return” statement in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, enforced through `assert_drained()`.
- **The crack of “one array entry was forgotten.”** Currently, five functions each manually
  implement a call sequence. After unification, there is one stage table, and omission of a new
  stage causes a compile-time error.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—seven of the 11
  inconsistencies (two dead-code-analysis implementations, W1006, W1003, monomorphization, role
  context, and IR generation/linking) **do not involve type transitions**; they are configuration
  questions of “should this analysis run?” This approach also cannot express `Aggregation` and
  merely moves the `check_module` / `check_module_collect_all` choice from the function layer to the
  type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this eliminates
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions each manually invoking stages” with “one array entry was forgotten,”
  which is precisely the cause of the current 11 inconsistencies. This repository already contains a
  counterexample of the same form: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache`
  / `HotReloader` described in `docs/src/dev/design/check/` are all unimplemented.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields. `Stage` remains scattered across five functions, and the next
  divergence will continue to grow from there. **It must be done first** (as part of S1/S3), because
  the structural refactoring needs a known red test that demonstrates the criterion’s validity.
- **Make `proof_calls` public and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must be in the orchestration
  layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several Driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactor that “centralizes
  wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”—these are real bugs that had
  simply never been reported. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual, field-by-field judgment.** Some
  obligation fields, such as `module_namespaces`, may have no consumer by design and should not
  produce warnings. S4 requires reviewing every field rather than applying a one-size-fits-all
  policy.
- **The `Program` abstraction may be premature.** If the stage sets of certain entry points remain
  unstable for a long time, `stages()` may degenerate into a free-form parameter that accepts a
  different array on every invocation, and the contract constraint will fail to hold.
  **Mitigation:** `test_program_stage_coverage` asserts that `stages()` can only come from the six
  predefined combinations.**
- **Criterion dependency:** The vulnerability-fix criteria in S1/S3 depend on the vulnerability
  tests in the [equivalence-criteria document](07-equivalence-oracle.md) being established first and
  turning red. **If that document has not first created a multi-file corpus layer, S1 cannot be
  accepted**, because all 293 existing corpus files take the single-file path and provide zero
  coverage for this class of defect. This document does not cover the IR validator (`verify_loose`);
  that prerequisite work is outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is
  absent from the pipeline, and the `termination` and `ownership` order is the reverse of the
  declared order). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undetermined.**
  `backend.rs:13-19` already states that `Solver` is only `Send`, not `Sync`, and the `Z3Backend`
  cache uses `RefCell`; after changing it to a singleton shared across compilation units, all access
  paths must be confirmed to pass through a `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **hide order-dependency defects**, such as the actual order
  dependency between termination and ownership. It should be considered only after the equivalence
  criteria have stabilized.

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) for the item-by-item
> resolutions (D1–D50). **No unresolved item remains in this document.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch-reachability cleanup; `include!`
  refactoring of `checker/semantic_tokens.rs` (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three levels of criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — Explicit statement that “silent pass must not be
  allowed to return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — Layer-order invocation table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — Explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — Contract declaration that “the caller should
  degrade conservently” (contradicting `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-76` — 13 obligation fields that downstream code must consume
  (after the 3.4.4/3.4.5/3.4.9 cleanups)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — The unit test’s own
  admission that it “stops before the pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus’s own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground’s
  fourth compilation caller (the single-file path, so proof_execution executes)
- `wasm/Cargo.toml:10-18` — The cdylib + wasm-bindgen live in the shim crate rather than the main
  crate
- `build.rs:19-55` — The compile-time error-code gate, an example of this project’s enforcement
  mechanisms
