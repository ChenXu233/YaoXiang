# Compile-Stage Contract and Obligation Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, acceptance-criterion tiers, and execution-stage
> ordering; see the [directory index](index.md) for the positioning of each companion document.

## Positioning and Scope

This document is the construction blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“which compilation stages ran, which did not, and why” is currently spread across six
paths and ten entry functions, each manually wired, with no single place that can answer the
question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being violated by the architecture rather than by the
> logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` states (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile error, no degradation, no silent
pass……must not allow silent pass to return.” However, this invariant holds only on the **single-file
path**: proof-function constraints such as `x: Sorted(3)` silently pass on the multi-file
`yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed, and no
diagnostic is produced.

### Covered

- The `Stage` enum (exhaustively enumerable, with no runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- The unified `Driver` (a single `dispatch`) and the specific changes required for each of the ten
  entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers affected by the changes above, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1–C6 tiers and the three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- The global execution order of P1–P10 and the G1–G10 acceptance gates → the main body of RFC-039

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** to perform it. This
document defines L1's **concrete form**, its **file-by-file change list**, and the **implementation
stages within this document** (corresponding to P3 “Fix correctness defects” and P4 “Stage contract
and unified Driver” in RFC-039's global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

The equivalence criteria are applied under category **C2 (orchestration changes)** from the
[equivalence-criteria document](07-equivalence-oracle.md): the diagnostic sets of all entry points
are identical + corpus behavior is identical.

## Current State

> Every item in this section is a **verified fact**, with a file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect in which “one mistaken location silently affects the entire repository”

This differs in nature from the other two classes of defects (module boundaries and test wiring):

| Defect                  | Typical manifestation               | Is there a signal?       |
| ----------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntactic error | Incorrect source code               | Yes, a diagnostic        |
| Type mismatch           | Incorrect type                      | Yes, a diagnostic        |
| Missing stage wiring    | A stage is never invoked            | **No signal whatsoever** |
| Unconsumed obligation   | A field is populated but never read | **No signal whatsoever** |

The latter two share a common characteristic: **failure does not produce an error**. They therefore
cannot be solved by “writing the code more carefully” or “conducting a stricter review”—code review
can reveal what was written, but not **what was not written**. This is precisely the root cause
identified by RFC-039: “This project treats ‘design’ as a documentation convention rather than an
executable constraint.”

### Enforced mechanisms already present in this repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`, which compares every
  entry against the RFC-013 code table. Any discrepancy directly triggers `panic!` and rejects the
  build.
- `src/package/` (**76 files / 13,012 lines**, including 6,227 lines under `tests/`, or 47.9%) has a
  high test density, and every module's test subtree is declared and wired. This is the repository's
  most complete test-wiring area and can serve as a formal reference for stage-layer gates.

**The design capability is sufficient. What is missing is “an equally strong gate at the
orchestration layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path used                                                              |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct invocation of 5 stages                                                      |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **LSP within a project**   | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c within a project; otherwise manually lex→parse                             |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages of the single-file path (`pipeline.rs`) are:

| Stage               | Invocation point | Implementation                                                            |
| ------------------- | ---------------- | ------------------------------------------------------------------------- |
| lexing              | `149`            | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`            | `run_parsing`                                                             |
| typecheck           | `173`            | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`**    | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`            | `run_ir_generation`, with **monomorphization embedded in it** (`381-389`) |

### Eleven stage-coverage inconsistencies

The table below provides evidence cell by cell. **An empty cell does not mean that the entry point
“does not do this thing”; it means that “no code in that entry point does this thing”**—which is
precisely the essence of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                          | `check_project`                                                | `check_source_in_project` (LSP)    | `compile_embedded_module`                      |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                         | **No**                             | **No**                                         |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, no configuration gate)              | **No**                             | **No**                                         |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                  | **No**                             | **No**                                         |
| 4   | W1003 unused imports                                     | Yes (collects `type_result.warnings` at `277`)      | **Collected but never emitted**                            | Yes `350`                                                      | **No**                             | **No**                                         |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                | **No**                             | **No**                                         |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | Not applicable                                                 | Not applicable                     | **No**                                         |
| 7   | Typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                        |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                   | `discover` (`454`)                 | Not applicable                                 |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                | **No**                             | **No**                                         |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                        | **No**                                                         | **No**                             | **No**                                         |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                         | **No**                             | Yes (independent ModuleIR followed by merging) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** this table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Differences in the current state
> after P3 lands are as follows:
>
> - **Row 1 has been fixed**: proof_execution's five entry points use the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entry points), so the
>   unique-consumer defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and the dead-code-family warnings, and the W-code sets of the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) have been
>   normalized—the criterion has a single source in the shared implementation of the
>   `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk is an unproven defect).
> - **Row 7**, the dual `check_module` / `check_module_collect_all` entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “decision #434” previously had no registration in the docs—it has now
>   been registered as RFC-039 **D57**.
> - **Asymmetry in embedded std inclusion (new out-of-table fact, registered on 2026-10-09)**: the
>   single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required for
>   desugaring for loops, with #117 mandating the hard switch), whereas multi-file `discover`
>   previously recognized only explicit `use` statements—project for loops compiled successfully but
>   failed at runtime with E6006 (confirmed by a probe). This has been fixed and WBS 4.10.2 closed.
>   The related “multi-file lacks the single-file parse hard-abort quirk” was registered as WBS
>   4.10.1—**it has also been fixed** (immediately after 4.2.2; the Check path was degraded to
>   per-file collection; this audit examines the “stage coverage/field consumption” dimension, and
>   this case is a compilation-unit membership difference, so it fell outside that scope).
>
> - **Embedded std registration-surface coverage (found during 4.2.5 implementation, fixed)**: the
>   Registry arm repeatedly harvested registrations for embedded std units through
>   `extract_module_info`, replacing the native half registered through `with_std()`
>   (`result.is_err`, etc. were lost → std.test falsely reported E1043). This is the audit's third
>   missed dimension: the merge semantics of “two registration sources for the same module key”—the
>   single-file path forms its registry only once through `with_std()`, whereas the multi-file/Check
>   Registry arm inserts per unit, which is what provides coverage.

**Two points require precise wording; otherwise the conclusion would be incorrect:**

- **The precise conclusion for rows 4/5 is:** `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
  (`run_file` → `Pipeline::run`), it does. The reason is `compile_project`: it places the complete
  `TypeCheckResult` (including `warnings`) into `type_results`, but the function **has no read site
  for `result.warnings` anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The entry criterion for `main` in row 11 also comes from different sources**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, which checks only whether a manifest exists). The
  two functions provide different answers to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of the document. All eight steps below are independently verifiable.**

**Step 1 — There is a unique production point for obligations.**
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

Its meaning is: “The predicate arguments of this refinement constraint are compile-time literals, so
the proof function must actually be executed to determine the result.”

**Step 2 — The consumer is inside the checker, but it only produces and never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                          | Behavior                |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → infers a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Infers a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Infers a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real defense line rather than a prospective fallback: silent pass must not be allowed to return.`

In other words, the author knew that “emitting no diagnostic” was a defect and explicitly classified
it as a **known intermediate state with nowhere to go**, assuming by design that “someone will
eventually read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.** Inside `check_module_impl`, a local
`proof_calls` is declared and collected (originally `checker.rs:1234-1235`), then written into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registration of proof-function obligations`
(originally `:1439`). The field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** In production code,
`TypeCheckResult.proof_calls` (`types.rs:29`) is read in **only one place**:
`src/frontend/pipeline.rs:187`, which passes it as an argument at `189`:

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other occurrences of the `proof_calls` identifier fall into three categories, none of which
consumes this field: the three production-side collection sites in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); `verdict.rs:61`, which is a same-named field
of `ProofResult`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points passes through that layer.** The four entry
points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)—**all bypass
`pipeline.rs`** and directly invoke `TypeChecker::check_module` (`124` / `493` / `1386`).
Consequently, none of them can even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also take the discard
path.** `compile_embedded_module` (`1374`, with the `check_module` invocation at `1386`) is
responsible for compiling embedded std. This means that **the embedded std module's own proof
obligations are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostic whatsoever** on the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layer.**
When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`):

```rust
// Originally checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the promise at `5164` that “silent pass must not be allowed
to return,” and it is **independent of the orchestrator issue**—even if the entry-point layer is
completely fixed, this location will still remain silent. (As for timing: it should be handled in
the same batch as the obligation ledger because it belongs to the same class of issue.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), which means they **stop exactly before `pipeline.rs`**.

The test files' own documentation comments already state this point.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer`

**This is exactly the shape of the problem: the tests verify that “the obligation was populated,”
whereas the bug is that “the consumer never read it.”** A test that exercises only the producer and
not the consumer is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria is the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` fixtures in `tests/yaoxiang/` as its **third-layer criterion** and the primary
acceptance mechanism for C2. In practice, however:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero hits for the entire
  directory glob).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  (`614-616`) for every fixture → `check_single_file` (`623-661`) → `Compiler::compile_with_source`
  (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 fixture files take the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this. The file is annotated with `// expect: compile-error E4018` (line 15), and line 7
of its header says “Status: ❌ should be rejected at compile time”—**it can pass precisely because
it takes the only path that executes proof functions**.

**Conclusion: the third-layer criterion needs an additional multi-file fixture layer; otherwise it
cannot serve as the acceptance tool for this document.** See “Implementation Highlights” S1.

### Obligation fields: 13 “production implies contract” fields with no mechanical guarantee

Among the fields of `TypeCheckResult` in `src/frontend/core/typecheck/types.rs:16-76`, **13 fields**
have documentation comments explicitly stating “produced by stage X → consumed by stage Y”; in other
words, they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: the definition line is the field's **declaration line** in `types.rs`
> (the line containing `pub` / `pub(super)`), not the first line of the doc comment—the comment
> block for the same field may span multiple lines, and only the declaration line provides a unique
> anchor. The **struct range** in the sentence above (`src/frontend/core/typecheck/types.rs:16-76`)
> moves with the struct's line number and is mechanically reconciled by gate **F5** (a mismatch
> turns the gate red); the definition-line column is reconciled by **F4**. | `proof_calls` | `36` |
> typecheck → **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by
> `PlanId`**: read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); completion marker at `:1095`) | | `escaped_refs` | `40` | ownership → IR
> generation | | `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`)
> | | `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46`
> | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); completion marker at `:1097`)
> | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | | `match_scrutinee_types` |
> `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` | `72` | typecheck → IR
> generation |

**Reclassification record (3.4.9, user decision A on 2026-10-13):** the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design,
it is consumed within a phase by typecheck and has zero cross-stage reads, so it is not an
obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solving queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection of `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed in this round).
- **Disposition**: remove the doc comment's **claim** that it is “mirrored for consumption by the
  middle layer/LSP,” while retaining the field and its in-phase consumption. The field is registered
  in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a rationale). If LSP eventually
  needs the table, register it under 4.6.x; **do not create a consumer in this round**.

**All these fields cross layers keyed by `Span` or by a name table.** The equivalence-criteria
document already points out that mismatches in span-keyed contracts can “fail silently, without any
error” (`ReleasePlan` is a typical example: if the two sides calculate spans differently in any
respect → the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module is on the mandatory path of every entry
  point.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator's four entry points never pass through the
  `pipeline.rs` layer.

**Verified fact:** the repository currently has **no mechanism whatsoever** that guarantees that
these 13 fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride
with IR generation.

### Registration of non-obligation fields: channel / in-phase / in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries):** every field of `TypeCheckResult` that is not a cross-stage
obligation is individually registered (field name + rationale), complementing the “§Obligation
Fields” table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                                 |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Compilation-unit identity metadata (0 hits in `src/middle`)                                                                                                        |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not “production implies contract”)                                                                                  |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but the doc comment does not declare a cross-stage contract ⇒ not incorporated                                      |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                               |
| `semantic_db`             | Consumer is in LSP (externally delivered through the `DriverOutcome.semantic_db` channel since 4.6.1, `src/lsp/server.rs:482`); not a compilation-stage obligation |
| `trait_table`             | Has cross-stage reads at `ir_gen.rs:4336/:6867`; the doc comment does not declare a contract                                                                       |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                        |
| `interface_impl_registry` | In-tree fact (3.4.9 user decision A; see the “Reclassification record” in “§Obligation Fields”)                                                                    |

**Why this is not “wiring up consumers”**: creating downstream consumers for these fields would
fabricate obligations out of thin air and disguise fields that “do not require cross-stage
consumption by design” as contract defects. The correct closure mechanism is **reverse
containment**—the field set of `TypeCheckResult` = the obligation inventory ∪ the non-obligation
registration. This is hard-enforced by criterion 1 of `scripts/ci/check-obligations.py`: adding a
field without registering it, deleting a field without synchronizing the registration, omitting a
rationale from a non-obligation entry, or overlapping the two tables all turn the gate red
immediately.

### Four additional contract defects in the proof layer

All of the following come from analysis of the proof layer. They are independent of the entry-point
bifurcation issue, but all belong to the class of “stage contracts are not enforced.”

**(a) Layer ordering is declared only in documentation; there is no mechanical enforcement, and the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual invocation sequence of
`checker.rs::check_module_impl` (positions are anchored by function name/driver point; line numbers
are taken from the current tree):

| Actual order | Invocation point                                                                                                                                        | Corresponding layer |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                         | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                            | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not in `layers/`)                                                                                       | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                  | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
the header comments of the respective modules and do not correspond to the invocation order; the
README has no “number → invocation point” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuit at
all**—`check_module_impl` adds termination errors one by one and then continues, so “an upper layer
does not run after a lower layer fails” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** hard-failure panic was removed by P3's 3.3.1 (the SOLVER slot was
> made optional, and absence is conservatively degraded to `SMTResult::Unknown`); “silent skip (no
> injection)” was given a W1081 signal by 3.4.2. Unification of the three forms (singletonization)
> is to be performed according to **RFC-039 D58**, with the actual fix at `proof/smt/backend.rs`
> (adding a process-wide shared singleton + a `with_shared_solver` closure entry point). The wording
> in the 02 change list that says `default_solver() → &'static` is superseded by D58 (a bare
> `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                                       | Acquisition strategy                                     | When the solver is unavailable                               |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                           | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected during construction through `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection with `with_solver_owned`        | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge cutting decision)                                                                | **Calls** `default_solver()` each time                   | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver's capabilities’.” By contrast, the contract documentation at `backend.rs:60` says:
“`None`: backend unavailable……**the caller should degrade conservatively**.” **Two failure
philosophies coexist in the same codebase, and the hard-failure path panics instead of returning an
error.**

**(c) A new Z3 context is created for every back-edge decision, making the cache effectively
useless.** `ownership.rs:627` invokes `default_solver()` on the hot path of the back-edge cutting
decision. In `src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call creates a new Z3 context through
`Z3Backend::new()`. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the doc comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache never hits in this invocation mode**.

(By contrast, the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. It is the same backend
with two different lifetime strategies.)

**(d) The checker silently degrades, and its comments do not match reality.** The two `body_checker`
fallbacks in `check_module_impl` (originally `checker.rs:1283-1287` and `1289-1293`):

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership checking receives an **empty type ledger and empty call
table** and proceeds normally—ownership analysis degenerates into “nothing conflicts,” **with no
warning whatsoever**. (Recommended handling: record a warning-level diagnostic, or at least leave a
trace in the obligation ledger.)

The comment in `check_module_impl` says that termination checking runs “after type checking and
before constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm state: carried by a shim crate; branches in 29 files are live

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
an independent shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The main crate contains only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                         |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                         |
| The shim depends on the main crate (rlib) as a library    | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                           |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI contains 4 wasm builds**                             | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten      | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL; degradation to a warning if missing)                                                                                                                                                                  |
| **29 files** contain the literal `wasm32`                 | **22** have actual `#[cfg(...)]` attributes; the other 7 mention it only in documentation (see the “F2 Determination” subsection of `06-cleanup-inventory.md` for the inventory and item-by-item determination)                                                                            |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                       |
| `src/lib.rs` has 11 occurrences                           | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                             |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call under the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` as a whole
(all three require `std::fs`), while the shim takes a different path.

**This also yields a fact directly relevant to this document**: the playground entry point
`compile_playground` (`:84`) in `wasm/src/lib.rs` constructs `Program { kind: WasmPlayground }`,
hands it to `Driver::run` (the JS-side entry point is `run_code`, `:122`)—that is, the **single-file
path**. Therefore:

| Path consuming `proof_calls`                              | Executes proof functions? |
| --------------------------------------------------------- | ------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                   |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                   |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                   |
| Multi-file `run` (`compile_project`)                      | **No**                    |
| `check` (`check_project`)                                 | **No**                    |
| LSP within a project (`check_source_in_project`)          | **No**                    |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three remain on the single-file side.** When the unified Driver is implemented,
this path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`); the implementation record appears in row
**4.2.8** of `09-execution-wbs.md` (`wasm` now goes through `ProgramKind::WasmPlayground`, `:274`).

(The cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)
are unreachable in playground scenarios—is assigned to the “F2 Determination” subsection of
`06-cleanup-inventory.md`: 4 wasm arms are unreachable at playground runtime but load-bearing at
compile time, while 6 `not(wasm32)` arms are load-bearing across the three native stage tables;
there are no low-risk items that can be deleted directly. The RFC-039 open question “create a wasm
target, or delete these branches” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, with no runtime registration)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum, and runtime registration
is forbidden.** The reason is that Rust's exhaustive `match` can enforce at compile time that “a new
stage must be handled by the orchestration layer”—precisely the mechanism that the opcode table has
already proved effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexical + syntactic analysis    PerModule
    Registry,           // Module registry construction     Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type checking (incl. proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis          Project
    ProofExecution,     // Compile-time proof execution      PerModule
    GlobalSlotAlloc,    // Global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                     PerModule
    Monomorphization,   // Monomorphization                   Project
    Linking,            // Cross-module linking / IR merge    Project
}

impl Stage {
    /// All stages. When a variant is added, this array and the exhaustive `dispatch` match will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at project level) make “which stages must run per module and which must run once at project level” a
fact encoded in the type system. Stages with `Project` scope are structurally guaranteed by
`dispatch` to run only once—this eliminates questions that currently require human reasoning, such
as “should `allocate_global_slots` be called once per file or not?”

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on the order.

> **Revision note (P4 implementation, 2026-10-07):** two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the initial order
>    contradicted the data flow.
> 2. The Check form's stage table was normalized to the topological order of `Stage::ALL`, with
>    dead-code analysis **before** proof execution: the current `check_project` executes proof
>    before dead code, but the two have no data dependency, so their diagnostic sets are identical
>    (C2 set semantics), and only the within-file diagnostic order is normalized. The single-file
>    path (dead-code analysis embedded in typecheck and before proof execution) already conforms to
>    the ALL order, so byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products—the initial order would force the Registry arm to “hide parse,” making the stage table
>    lie about the data flow; it would also reduce multi-file parsing from 2 times to 1. The current
>    state is faithfully registered as an appendix: multi-file parse errors used to be a hard abort
>    (the `?` propagation in `build_registry_from`, in tension with CollectAll semantics, registered
>    as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 when its last caller was
>    removed, so the historical account is retained here)—**4.10.1 has been fixed** (user decision
>    on 2026-10-09: Rust-style collection semantics + option B): a parse failure in the Check path
>    is degraded to per-file diagnostic collection, and the affected file leaves the compilation
>    unit (does not enter the registry; importers report E5001); the MultiFile path retains the hard
>    abort (under FailFast, a bad file cannot produce IR, which is the correct semantics, and this
>    long-term invariant remains in force).
> 4. The Check form's implementation (4.2.2, user decision on 2026-10-09) adds two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the initial draft omitted these
>    (those products were produced by `discover_with_used` in the orchestrator era and discarded by
>    the `discover` wrapper, so they disappeared from the table because they never entered the data
>    flow); b. The within-file diagnostic-order normalization was expanded to the full form of note
>    #2 above: E3020 entry validation was moved into the RoleClassification arm (Check has no
>    Linking stage, so the arm holding `surfaces` and the AST performs the validation), and the
>    within-file diagnostic order = stage topological order (E3020 → typecheck → W1006/W1003/dead
>    code → proof); the diagnostic set is unchanged (C2 set semantics), and only the order of stderr
>    entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective**: turn “a field is produced but nobody consumes it” from an undetectable
condition into a fact assertable at compile time or test time. RFC-039 has already identified this
as “the single highest-value item.”

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
    /// Each unconsumed field produces a W-level diagnostic (by default) or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                      | Criterion                                                 | Disposition                                                                             |
| ---------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| An obligation is read by its declared consumer | The field is `take()`n / marked consumed in `Obligations` | Pass                                                                                    |
| An obligation is non-empty but has no consumer | The field is non-empty and not consumed                   | **Produce a diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| An obligation is empty                         | The field is empty                                        | Pass (no consumption required)                                                          |

**Why start with W level rather than E level**: fixing obligation discarding will **change the
diagnostic set**. The C2 criterion requires that “the diagnostic sets of all entry points are
identical.” If the severity is raised directly to E level, `yaoxiang check` will suddenly gain a
large number of previously silent `Unproven` diagnostics. Splitting the work into two steps—first
observing with W, then promoting to E—keeps every step's criteria usable. See “Implementation
Highlights” S4.

**There is exactly one call site for `assert_drained()`**: `Driver::run`, at the end of the stage
table and before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appearing at least twice but not being read by a third file → failure). `assert_drained()` is a
**runtime** gate, while the script is a **static** gate. The two are complementary, and both are
required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed):** this note records the
> **implementation form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a corresponding code file:line for every item. Implementation commit `1941cc47a`
> (verification tree `99079bc52`). It is also the **third** of the three places that uphold the “no
> silence” red-list policy (the other two: the audit surface and the static gate).
>
> **1. Deliverables (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201; field
>   name/consumer stage/red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (:150, 8 entries with rationales)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   function `settle_obligations` (:577), external emission function `emit_obligation_diagnostics`
>   (:654), non-consumer gating `consumer_visible_unit` (:538), skip criterion
>   `consumer_stage_skipped` (:549), IrGeneration delivery set `IR_GENERATION_DELIVERED` (:285), and
>   W1090 code-string constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` also settles (the books
>   must balance).
> - **Seven delivery points** (one per arm; removing any one makes runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path for this
>   obligation from `TypeCheckResult`, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —all four use the same
>   delivery point: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (direct request-table read for a single file) / `:1221`
>   (MultiFile·Check: aggregation in the Linking arm, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the rationale appears above the entry at :26-31—the settlement granularity
>   is “unit × obligation,” with no source location that the user can point to, so no dummy span is
>   inserted); message text is present in six languages under `locales/*.json`. **Emission
>   severity**: since 4.3.2 it is **error** (same code promoted to E; see the third
>   section)—determined by the explicit `.severity(Severity::Error)` in `to_diagnostic`
>   (obligations.rs:444); the code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above        | Implementation criterion                                                                                      | Code                   |
> | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------- |
> | An obligation is read by its declared consumer | **A delivery marker exists** (the marker ↔ real read is reconciled by the static gate; see the fifth section) | obligations.rs:618-623 |
> | An obligation is non-empty but has no consumer | Non-empty + the consumer stage has run + no marker → W1090 (**error level**; see the third section)           | obligations.rs:624-630 |
> | An obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                              | obligations.rs:587-591 |
>
> Two **additional tiers** (user decision on 2026-10-13, not included in the sketch):
>
> 1. **S1**: the consumer stage of an obligation is not in this `ProgramKind`'s stage table → **do
>    not judge it under-consumed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: the consumer stage was skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → do not judge it under-consumed (obligations.rs:598-602). `NoObligations`
>    does **not** grant an exemption—if an obligation is non-empty while the stage claims it has no
>    obligations, the books do not balance and an error must be reported.
>
> **3. External-emission policy (decision “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: this mechanism externally emits **only W-level diagnostics**
> (W1090); there is **no strict switch**. **4.3.2 (terminal state, user decision A on 2026-10-13)**:
> **the same code is promoted to E**—the `W1090` code number remains unchanged, but the emission
> severity is raised to `error` (explicit `.severity(Severity::Error)`, `to_diagnostic`
> obligations.rs:444); **no new E code is added** (one fact with two code numbers would be parallel
> representations). **The promotion changes only the emission severity, not the emission
> conditions**. **This is a D-class breaking item under D59 (changes externally observable
> behavior), faithfully registered**: healthy programs emit nothing (no diagnostics when all
> obligations have delivery markers), so existing corpora and user programs are unaffected (342/342
> “difference: empty”); however, if a run actually encounters an undelivered obligation, that
> diagnostic **counts toward `error_count`, is rendered as `error[W1090]`, and causes a nonzero exit
> code**—unlike the previous “W level does not block compilation.” The code family and emission
> severity are decoupled; this policy is registered in RFC-013's “Emission channel” section (the
> code prefix determines only numbering ownership; severity is determined by the explicit severity
> at the construction site). Mechanism facts (measured): severity is taken from an explicit
> `.severity()`; otherwise, “a code beginning with `W` → Warning” (`codes/builder.rs:186-203`), and
> the registry's `ErrorCategory` does **not** participate in severity. The exit code is determined
> by `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`); **W1090 follows `error_count` because of its explicit error severity
> (since 4.3.2)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's decision C on
> 2026-10-13; the current red list is empty; see the fourth section). The reason is reproducible:
>
> - Decision A (emit the red list as W) has a measured cost: `interface_impl_registry` is populated
>   from typecheck's prebuilt table, is **always non-empty**, and is therefore a **constant signal**
>   with no discriminating power. It would nevertheless break three existing external
>   contracts—“clean programs emit no diagnostics / LSP emits no diagnostics /
>   `warning_count == 0`”: a measured `cargo test --lib --all-features` turned **12 existing cases
>   red**, of which **8 are on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs`
>   ×5, `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4
>   are in `src/driver/tests/`), and 24/342 fixture baselines changed. The only viable change would
>   be “filter W1090 out of zero-diagnostic assertions,” which relaxes an existing criterion.
> - Decision B (demote to Info level) would still break 9/12 of the list-style assertions, and Info
>   is not “emitted according to the adjudicated level.”
> - Decision C's measured result: **zero changes to existing cases**, **zero changes to the fixture
>   baseline** (342/342 “difference: empty, behaviorally equivalent”), and runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains silent throughout, and removing
>   any one delivery marker makes it fire (all three delivery points were tested by removing their
>   markers; the CLI `check` path emitted two `warning [W1090]` diagnostics when the marker was
>   removed from the Check arm).
>
> The red list's “no silence” policy is upheld in three places: ① the
> `DriverOutcome.obligations.findings` audit surface (emitted on every run, including WBS IDs and
> evidence pointers); ② `check-obligations.py`, which prints the red list on every run (currently
> empty) and detects stale entries; ③ this note.
>
> **4. Current red-list state (closure after clearing the accounts) and implementation records for
> 3.4.4/3.4.5/3.4.9**
>
> The red list has gone from 3 → **0**; all three cleanup tasks landed on 2026-10-13:
>
> Note: the line anchors below record the file state **as of 2026-10-22**. P5 (2026-10-26) split
> `checker.rs` into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting
> the line numbers in `checker.rs`; the anchors below no longer correspond to the current file and
> are retained solely as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order obtained from `sum_type_variants`), with the
>   call site at `:7496` replacing the entire Call with `CreateVariant`; the function's doc
>   explicitly states that “span-keyed cross-system matching is unreliable (the two sides interpret
>   a Call node's span differently), so detection is performed in place”—**wiring up a consumer
>   would reintroduce the already-fixed span-mismatch defect**. Deletion surface: the `types.rs`
>   field, collection at `checker.rs:1489-1493/:1565`, the collection chains in
>   `inference/expressions.rs` and `inference/statements.rs`, and the `VariantCtorCall` structure in
>   `environment.rs`; `infer_variant_ctor_call`'s type inference and all diagnostics **remain
>   unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550`, the decision point; the sole hit writes to `overload_resolutions`
>   at `:4550`); what is deleted is the **zero-reader mirror** injected by `checker.rs:1558` into
>   `TypeCheckResult`. The naming fact still has a single expression: the candidate-name table (in
>   phase) + span-keyed decisions (cross-stage, constructed at `ir_gen.rs:380`, consumed at
>   `:7632/:7705`); mixed names on the definition side are computed independently by
>   `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user decision A; see the
>   reclassification record in the §Obligation Fields table). What is deleted is the **claim** in
>   the doc comment that it is “mirrored for consumption by the middle layer/LSP”
>   (`types.rs:47-48`); the field and its in-phase consumption are retained
>   (`operator_interfaces.rs:623`, operator queries; seven constraint-solving queries in
>   `inference/expressions.rs`; the read-only accessor at `checker.rs:214-217`); direct proof of
>   zero cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not
>   wire up a consumer**: the table's content is an environment mirror populated from typecheck's
>   prebuilt table (always non-empty). If LSP genuinely needs it, register it under 4.6.x; creating
>   a consumer for a field that “does not require cross-stage consumption by design” would introduce
>   a false obligation.
>
> After the cleanup, the **field census is closed by “obligation inventory ∪ non-obligation field
> registration (`NON_OBLIGATION_FIELDS`, with rationales)”**—the reverse-containment endpoint of
> gate criterion 1 protects this end to end (adding a field without registering it / deleting a
> field without synchronizing it both turn the gate red immediately). The red-list **mechanism** is
> retained: stale-entry detection under gate criterion 4 and the `--selftest` `stale-red-list`
> fixture continue to cover its detection surface; any future registration must go through an
> explicit decision.
>
> In conjunction with the cleanup, the accounting policy changed from 16 obligations → **13**
> (`INVENTORY` / `Obligation::ALL` / the §Obligation Fields table in this document); the gate's
> `--selftest` now covers seven fixture categories (adding “unregistered field,” “stale anchor,” and
> “empty red list”), under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2) (three additional hard criteria beyond the sketch)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: the
>    `INVENTORY` in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ the “§Obligation
>    Fields” table in this document (including the table header's count declaration); any drift
>    turns the gate red; moreover, the field set of `TypeCheckResult` must be **exactly equal** to
>    “the obligation inventory ∪ the non-obligation registration (`NON_OBLIGATION_FIELDS`, with
>    rationales)”— adding a field without registering it, deleting a field without synchronizing it,
>    omitting a rationale from a non-obligation entry, or overlapping the two tables all turn the
>    gate red immediately;
> 2. **Read evidence**: every non-red-list obligation must have a real read in the **declared
>    consumer file** (`.field_name` member access; comments/tests/strings do not count)—this makes
>    “delivery marker ↔ actual downstream read” a hard criterion (the main risk of mechanism A is
>    precisely that the two drift);
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    conversely, red-list fields must not be marked;
> 4. **Red-list stale-entry detection**: red-list fields must have zero real reads (growing a
>    consumer turns the gate red), and each entry must carry a WBS ID and an evidence pointer; there
>    is also `--selftest`: built-in negative/positive fixtures, with negative fixtures required to
>    be red and positive fixtures required to be green (proving that the detector is not idle).
>
> **6. Deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains held by `TypeCheckResult`, and the ledger records only the “inventory +
>    delivery markers + settlement conclusions.” The reason: copying it would create two sources of
>    truth (each field is produced separately by the typecheck checker and the ownership layer) and
>    they would inevitably drift; the cross-layer data plane should not be duplicated into the
>    orchestration layer.
> 2. The sketch's `take()` / “mark consumed” was **not adopted**; it was replaced by `is_produced`
>    emptiness checks
>    - `DeliveryLedger` delivery markers. Reason: an obligation and its product are the same data,
>      and `take()` would remove what downstream needs; moreover, 11 IrGeneration obligations have
>      the shape “the entire `&TypeCheckResult` is handed to ir_gen,” so they cannot be taken field
>      by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (since 4.3.2, W1090 is E level)—the single-file path uses
>    `PipelineError::TypeCheck` (the external-emission arm at obligations.rs:654), while
>    multi-file/embedded record an `OrchestratorError` failure (otherwise `result.errors` would have
>    no consumer). Reason: returning `Vec<Diagnostic>` would lose form-based routing—check
>    diagnostics must retain per-file attribution.
> 4. The first tier of the three-tier table became “a delivery marker exists”; “marker ↔ real read”
>    is handled by the static gate (criterion 2 of the fifth section).
> 5. The second tier's “promote to E level in `strict` mode” **does not use a strict switch**; it
>    was changed to **promotion of the same code to E in 4.3.2** (it is promoted even without a
>    strict switch, because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appearing at least twice but not being
>    read by a third file” to the four criteria in the fifth section.
> 7. The two additional tiers not included in the sketch (S1 and the skip exemption) are described
>    in the second section.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table actually landed as
>    **20 test cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]` skeleton →
>    green; the red state is retained in the record: after removing the sole settlement point,
>    `pairs: 0` and 6 failures). The S1/S4 rows have been synchronized to the actual file.
>
> **7. Implementation record for 4.3.2 (severity + gate hardening)**
>
> - **Inputs for field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in this run, available per run) and five counters (the conservation identity in
>   the second section); together with the audit-surface `findings`, these provide the fact of
>   “whether this field has ever been consumed by any entry point.”
> - **Severity: promotion of the same code to E has landed** (2026-10-13, user decision A; see the
>   terminal-state registration in the third section). Implementation: `to_diagnostic` explicitly
>   calls `.severity(Severity::Error)`; the external-emission arm routes by severity into the error
>   channel. Mutation evidence: removing any one delivery marker → compilation fails in that form,
>   and the `check` exit code is nonzero (`error [W1090]`); after restoration, the healthy state
>   emits nothing (corpus 342/342 unchanged). Locale text states in six languages that “the code is
>   in the W family, but the emission severity is error.”
> - **Gate-hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration rationales and red-list evidence**, validating that “the file is locatable
>     (uniquely), the line number is in range, and the symbol immediately following the anchor
>     really appears in the anchored file”; an anchor pointing to a nonexistent symbol turns the
>     gate red. Built-in negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`
>     (one nonexistent symbol / one out-of-range line). Note: the red list is currently empty, so
>     the criterion's actual landing point is the **non-obligation registration** and future
>     evidence pointers (the same code path).
>   - **F2/R3 reverse containment**: the gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference”; a nonzero difference turns the gate
>     red; a built-in negative fixture is `unregistered-field/` (a new struct field with no
>     registration).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck fails does not retain its `type_result`, so no obligation
>   exists); the semantics of this tier are pinned by a synthetic `State` case, and real end-to-end
>   coverage only includes `ConfigDisabled` (t3 T06).
>
> **8. Closure of the 4.3 secondary task (red list cleared + terminal E-level obligation
> diagnostics)**
>
> **8.1 Three-round implementation timeline** (each item corresponds to a code file:line; this round
> only registers conclusions and does not change code)
>
> | Round                  | Conclusion                                                                                                                                                                                     | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
> | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger goes live | `State::assert_drained()` as the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                    | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; external emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                                  |
> | 3.4.4 Cleanup          | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                            | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                                       |
> | 3.4.5 Cleanup          | `method_overload_ir_names` cross-stage mirror deleted (retain the in-phase candidate-name table + `overload_resolutions`)                                                                      | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                                    |
> | 3.4.9 Cleanup          | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirrored for middle/LSP consumption” claim and retain in-phase consumption; obligations 14 → 13, red list 1 → 0     | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under decision A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven locations in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Severity         | **Promote the same code to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; external emission routes by severity into the error channel (first failure takes priority) | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level arm); code-family/severity decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                                           |
>
> **8.2 Closure conclusions**
>
> - **Red list cleared to zero**: the number of entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) with `red_list: Some` = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism is
>   **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the gate's stale-entry
>   detection under criterion 4, and the `stale-red-list` fixture; the gate prints “red list: 0
>   entries—cleared and closed.”
> - **Terminal E-level obligation diagnostics**: the **code number and W-family membership of
>   `W1090` remain unchanged**, while the emission severity is `error` (terminal D5 state, user
>   decision A on 2026-10-13). This is a **D-class breaking item under D59** (changes externally
>   observable behavior), registered in both the third section of this note and RFC-013's “Emission
>   channel” section; **zero emission in the healthy state ⇒ no impact on existing corpora or user
>   programs** (corpus 342/342 “difference: empty,” zero IR-snapshot drift; t11 independent
>   verification and t12 review each rechecked this).
> - **Field census closed**: the field set of `TypeCheckResult` = the obligation inventory ∪ the
>   non-obligation registration (8 entries, `src/driver/obligations.rs:150`, each with a measured
>   consumption-surface rationale); gate F2 explicitly prints the difference (currently
>   `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: all hardening items registered in this note have
>   **landed** (F1/F2/F4/F5; see the fifth, seventh, and eighth sections); the only two unresolved
>   items are the **known boundary** in 8.4 and t7 F5, which was converted into an issue; neither is
>   an unimplemented criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/review round**
>
> | Source        | Finding                                                                                                                                                                 | Status                                                                                                                                                                                                                                                                                |
> | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence consists of plain-text pointers and becomes silently stale if the replacement consumer is removed                                                     | **Landed**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`                                                                               |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, so adding a new field silently passes                                                                                       | **Landed**: F2 explicitly prints the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                               |
> | t4 R4         | The W-only versus E-promotion policy was not registered                                                                                                                 | **Landed**: the third/seventh sections of this note + RFC-013 “Emission channel” + W1090 help and error_output examples in the six languages under `locales/*.json`                                                                                                                   |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (legitimate overloads are still falsely reported as `W1005`)                                              | **Converted into issue `#450`**, assigned to WBS **4.9 Correct implementation of the reference pool** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—not part of this secondary task                                                                             |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in prose comments in module headers                                                                   | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                          |
> | t11 F8        | Verifier-side self-record: mutation-driven runs once wrote to the main repository (immediately cleared with `git restore`; affected-round output invalidated and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                                          |
> | t11 F9        | An intermediate state in the main repository (unstaged files made this document's table header inconsistent with `INVENTORY`) triggered a red gate                      | **Informational / resolved**: the frozen commit is self-consistent and passes; “directly committing an intermediate state turns the gate red” is precisely the intended behavior of F2/F4                                                                                             |
> | t12 F12-1     | This document states the struct range as `16-73`, but the actual range is `16-76` (three comment lines added in t9 without mechanical reconciliation)                   | **Landed + mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; a mismatch turns the gate red); related figures were aligned by grep as well (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                |
> | t12 F12-2     | Settlement in the multi-file form once superseded the first failure (the type error in `zzz.yx` was overwritten by W1090)                                               | **Fixed** (the captain amended the commit message to explain this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression is caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered
>   entries**—non-obligation registration rationales (`NON_OBLIGATION_FIELDS`) and red-list
>   evidence; **`file:line` promises in prose comments in module headers are outside the scanning
>   surface** (no currently registered entry is affected). If such an anchor promise is added to a
>   module header in the future, either **manual review must explicitly assume responsibility**, or
>   F1's scanning surface must be expanded—one of the two, with no default assumption that nobody
>   will handle it.
> - **The gate's static boundary**: set-level validation of delivery markers is static (criterion
>   3), and removal of a single arm is covered by **runtime settlement**— the static + runtime
>   two-layer structure is the intended design, not a gap (t11's M11A/M11B/M11C mutation
>   verification was bidirectional).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId conversion
> landed):** among the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been converted to **structural identity keys** (`PlanId`) in
> accordance with RFC-039 **D20**. This note registers the implementation form, criteria,
> classification basis, and known boundaries. The definition lines are still reconciled by gate
> **F4** (the line numbers in the two corresponding rows above are the current code facts).
>
> **1. Implementation form (carrier selected: `PlanId`)**
>
> The key is the **structural identity** that travels with the AST node, no longer relying on the
> positional coincidence of “both sides calculate the span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthesized nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → release-plan ownership and consumption) and `Expr::Call.plan_id` (call-site identity →
> production and consumption of overload resolutions); `Expr::call` is the repository's only
> call-expression constructor, and synthesized calls outside the parser all record `UNPLANNED`
> (**not consumable**; see the second section).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**reading and accounting occur in
>   the same invocation**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is
>   provided by `walk_stmt`'s set/restore + the CFG-node → statement-identity table; a scope-exit
>   Drop belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (the table and consumption ledger are **one
>   object**), with the production point at `inference/expressions.rs:4550` (written by call-site
>   `PlanId`, with dispatch passing it through from `Expr::Call { plan_id }`), and the consumption
>   points at `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) via
>   `OverloadResolutions::get` (a hit is recorded).
>
> **2. Consumption-completion settlement (reusing the 4.3 ledger rather than creating a second
> one)**: the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each of the two tables; when
> `generate_module_ir` successfully returns, the two ledgers are marked as having “completed a full
> pass” **at the same point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload
> resolutions); the three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point adds one line for each of the two
> obligations—a nonempty difference produces an **E-level W1090** (promotion of the same code to E;
> see the third section). The exemption criterion hangs off the **ledger's own completion marker**
> (not whether `unit.ir` exists), so the Check form's removal of the IR product does not constitute
> an exemption.
>
> **3. D59 classification basis (class C, red before green)**: the form
> `Type.method(positional argument, name = value)` previously **silently returned the wrong answer**
> (measured: `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the
> selected overload was discarded, and resolution fell back to the last binding in declaration
> order); after the change, the same program produces a compile-time
> `error [W1090] ... produced key was not consumed`. The fix chosen is **loud** rather than “put the
> resolved name into the call with named arguments”: the latter would lose the named argument and
> merely replace one form of silence with another (terminal semantics are described in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition: `Drop` instructions are a no-op in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty
> differential over 342 corpus entries is empirical proof of this.
>
> **4. Criteria and evidence**: the D41-style criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + an evidence-surface
> floor**: the number of files that produced decisions `≥ 1`, the number of genuinely consumed
> entries `≥ 2`, with no allowlist); the nine pinned decision forms are covered by
> `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return / positional argument
> / binding in a type body / `?` operand / named only / positional + named / ambiguous / variant
> construction / unassigned identity). Mutation-driven verification: shifting a consumption key by
> one position ⇒ criterion turns red + the real program produces `error [W1090]` + the corpus gate
> differs; after restoration, it turns green again. Gate figures: `check-obligations.py` PASS,
> corpus differential empty across 342 entries, `cargo test --lib` 2712 passed / 0 failed (zero
> IR-snapshot drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: the **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable** with the current corpus and test surface
> (method declarations consistently use `register_type_bindings` and therefore reach the bound
> consumption point), so the keying correctness of that branch has **no red state with which it can
> be falsified**; the reason for retaining it is that it is the symmetric branch of the bound path,
> and deleting it would rewrite the “method call without `type_bindings`” form into silence. This is
> registered by independent verification (t5 F-1) and review (t6 F2); **no new task is opened**.

### 3. Unified Driver: a single `dispatch`

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

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers cannot freely supply an array.** This is the key constraint that prevents the `Program`
abstraction from degenerating into “a free-form parameter that can accept anything,” and it is
asserted by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce a diagnostic; see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // …12 arms; not one may be omitted
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed):** the four implementation
> deviations from the sketch above are semantically equivalent and are registered for reference:
>
> 1. The three-state `StageOutcome` from §4 is not returned by stage arms; it is expressed through
>    `State`'s failure flags + the `Aggregation` gate (Continue / Warn / Abort semantics remain
>    unchanged, eliminating repetitive return boilerplate from every arm);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” determination in §5 covers the sketch field);
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not externally emitted—the zero-diff criterion of S2, “deliberately fix no
>    bugs”; external emission will follow with the 4.3 obligation ledger;
> 4. The visibility of the `proof_execution` module is relaxed to `pub(crate)`: the driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the existing orchestrator call sites will
>    be restored to their proper location after the 4.2 migration;
> 5. In the multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user decision on 2026-10-09): the original `compile_project` interleaved
>    proof execution within the per-file typecheck loop (file N's proof preceded file N+1's
>    typecheck). With a single failure, both are byte-for-byte identical; in the multiple-failure
>    scenario “file 1 proof failure + file 2 typecheck failure,” the first reported error changes
>    from the proof error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries products through channels partitioned by ProgramKind: `result`
>    (pipeline contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error
>    contracts of the entry points (PipelineError / OrchestratorError) are outside the type surface
>    that Driver can unify.
> 7. The Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry, while a clean file has an empty Vec—the
>    existing `check_project` contract); the Discovery product is extended to a three-part tuple of
>    “file set + used_by edge set + shadowing events” in State (the MultiFile form has no downstream
>    consumer, so it is only recorded and not externally emitted); parsing per file on the check
>    path is reduced from 2 times to 1 as the C3 topology changes.
> 8. The Lsp form landed (4.2.3, source-based routing decision, user ruling on 2026-10-09): a parse
>    failure in a disk file follows Check's option B (collect + leave the compilation unit—both
>    sides converge on degradation); a parse failure in an edited buffer retains the partial AST and
>    continues typechecking (editor philosophy—an intermediate state while typing should not make
>    semantic functionality disappear). The Discovery arm preserves the buffer source over stale
>    on-disk content for Lsp; Typecheck checks only the target file under Lsp (other units exist
>    only to provide signatures to the registry); the principle of normalizing within-file
>    diagnostic order extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The
>    old LSP behavior—hard-aborting on a parse error in an unrelated disk file and silently falling
>    back to the single-file path in the handler—is fixed by this step.
> 9. The Embedded form landed (4.2.4): the `Program` struct gains a `shared_registry` field—the
>    embedded std module is a child compilation, and the registry is an **input**, not a product
>    (EMBEDDED_STAGES has no Registry stage), so the #94 SymbolTable sharing contract is made
>    explicit as a program declaration rather than “the caller must remember to pass the same
>    registry.” The error-path text is normalized from `<std.test> (embedded std)` to the unit's
>    virtual path `<std/test>` (compiler-internal error surface only, with no pinned test). At this
>    point, all four orchestrator entry points have moved to the Driver.
> 10. Standalone check unification (4.2.5, decision A + IR-stage decision, 2026-10-09): the Check
>     form faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive errors only, preferring omission
>     over false positives); relative `use` is resolved relative to the importer's directory
>     (aligned with rustc's single-file mod behavior). The CHECK stage table gains GlobalSlotAlloc +
>     IrGeneration: the old standalone path (the full pipeline) already ran IR generation, and
>     E3019/E1014/E1015 are produced only by ir_gen (confirmed by the runner gate). The Check form
>     of IrGeneration is a pure check and does not consume IR; the Script/Bin distinction is
>     represented by None/Some of module_key (the existing E3023 switch, ir_gen.rs:1660).
>     Monomorphization is not included in CHECK (it produces only resource-limit/internal errors,
>     with no corpus dependency). **〔Withdrawn on 2026-10-10 after empirical verification in
>     3.4.8〕**—“zero corpus dependency” was actually an evidence blind spot caused by the corpus
>     lacking pathological-recursion fixtures; E3005 is the sole compile-time defense for this class
>     of program. See note #14. Incidental fix for a latent defect: the Registry arm's repeated
>     harvesting of registrations for embedded std units would replace the native half already
>     merged by with_std() (causing false E1043 reports)—embedded units now skip duplicate
>     registration.
> 11. LSP single-file fallback unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and the CLI now use the same
>     Driver. In the SingleFile+CollectAll form, Parsing collects all parse errors, retains the
>     partial AST, and continues typechecking (the editor philosophy from the 4.2.3 decision extends
>     here; the stage is not marked as failed, since otherwise topological skipping would prevent
>     typecheck from ever running); Typecheck dispatches the `check_module_collect_all` free
>     function. The LSP single-file path now covers proof (E4018), W-code warnings (W1001–W1005,
>     pinned by tests), and IR-level errors; lexical failures are now reported as real diagnostics
>     (the old synthetic “E0001 lexical error” text disappears with the deleted sequence—real
>     diagnostics carry precise spans, with no information loss).
> 12. `check_module` / `check_module_collect_all` verified line by line (4.2.7, mandatory C5 item):
>     the two entry points had already converged independently—the mod.rs layer uses
>     `check_module_inner(ast, env, collect_all)` with a single Boolean, and the checker layer uses
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, with both the pass-3 and drain
>     modes running, and differences carried by whether `collected_errors` is empty; five collection
>     points in statements.rs (function body/use/for/block/while). Verification also confirmed and
>     fixed a duplicate-diagnostic defect: the collection point placed the first error in both
>     `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err` produced
>     two identical diagnostics with the same code, span, and message (up to three for nested
>     cases); there were also two independent mechanisms for duplicate annotation validation (formal
>     parameters vs. whole annotations, E1003/E1103) and ownership layer double emission at the same
>     point (E2014/E2018). The fix is applied at the module-result boundary: deduplication by (code,
>     span, message)—a diagnostic is a positional fact, and duplicates carry no information; 63
>     corpus entries had duplicate copies removed, with zero drift in run columns or exit codes (the
>     “duplicate” in `borrow_conflict_err` is an artifact of the baseline triplet format lacking a
>     column number—the two E2018 diagnostics are legitimate diagnostics in different columns).
> 13. wasm playground landed (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper no longer
>     participates. Error text is aligned byte-for-byte with the old `CompileError` Display prefix
>     (Parse error:/Type error:/Internal error:), with no visible difference in the playground UI.
>     WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the existing 4.1
>     declaration now has a real producer).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user decision on
>     2026-10-10, **partially withdrawing the mono exclusion in note #10**). Empirical evidence
>     disproved the claim that “mono produces only resource-limit noise”: with pathological generic
>     recursion (`f(x)=f([x])`) and no mono on the multi-file path, the compiler process overflowed
>     its stack (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime error), and
>     the mono depth gate is the sole compile-time defense; standalone check before 4.2.5 ran the
>     full pipeline and therefore already had mono, so the exclusion decision introduced a silent
>     coverage regression (“zero corpus dependency” only proves that the corpus has no pathological
>     fixtures). The “type-erasure fallback” assumption was invalidated by the RFC-033 reflection
>     decision—`^^List(Int)` requires the real identity of an instantiation, and monomorphization is
>     the foundation of reflection rather than an optional optimization. Four design points: ①
>     typecheck produces an instantiation_request for qualified calls (`lib.f(x)`, the FieldAccess
>     form), with generic_id using the qualified name, sharing the merged IR's namespace (currently
>     only bare Var is recognized, expressions.rs:2455); ② the `Stage::ALL` topology changes to
>     Linking before Monomorphization, and the MULTI_FILE stage table gains Monomorphization at the
>     end— consuming merged_ir and aggregating requests from all units (qualifying the deferred
>     bucket's containing_fn), with mono promoted from a per-module pass to a whole-program pass;
>     the single-file stage table remains unchanged (no Linking, the subsequence property is
>     preserved, and IR snapshots have zero drift); ③ CHECK gains Linking (pure merge—E3020 entry
>     validation remains in RoleClassification to avoid duplicate reports) and Monomorphization
>     (pure check without consumption, like the Check form of IrGeneration); ④ resource protection
>     covers cross-unit mutual recursion through whole-program BFS. The Monomorphizer itself remains
>     unchanged (its contract—consume ModuleIR + a request set—exactly matches the merged IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the #14 design: `ExpressionInferrer` is injected with a
>     module-qualified key table (the three `use` arms register it—ordinary / alias / item-by-item
>     alias, with only SubModule classes exported), and `callee_generic_name` concatenates through
>     `SymbolTable::qualify` with a qualified name from the same source as the merged IR; the first
>     path for qualified calls tightens its arity criterion (when the parameter table cannot be
>     resolved, it falls back to the second path and obtains arguments by signature, avoiding false
>     E3018 reports caused by mismatched same-name functions across modules). During implementation,
>     three **asymmetrical defects** on the mono side were empirically confirmed and fixed in the
>     same round: (1) The granularity of the deletion key (the generic-name set) and the rewriting
>     key (containing_fn + name + span triplet) mismatched—a site whose arguments could not be
>     resolved degraded to a symbolic request in the deferred bucket, and when the containing
>     function was not generic, the bucket never drained, while the original had already been
>     deleted by name → dangling call (empirically confirmed by runtime E6006 in `list_ops.yx:53`);
>     the fix is the restoration gate `restore_generics_with_uncovered_call_sites`, which uses
>     `build_call_site_map` as the sole source of truth to determine coverage and retains the
>     original at uncovered sites. (2) The restoration segment's `HashSet` iteration order made
>     function-table order nondeterministic (t2 review F1) → changed to `Vec` and sorted by name,
>     making the output byte-for-byte reproducible. (3) The TailCall / MakeClosure forms in
>     non-rewriting areas still looked up mapping keys (although the comment claimed they were
>     conservative) → a single-point `call_form_is_rewritten` determination was introduced, and
>     non-rewriting forms are always counted as uncovered. Each of the three has a pinned test (the
>     red state before the fix empirically demonstrates discriminating power). The corpus
>     differential baseline is regenerated from the same commit with exactly one changed line (a
>     `(int64)` added to the `remove_at` frame). **Commit-strategy note**: when typecheck collection
>     and the mono fix were split into two commits, the staged tree containing only the former
>     triggered a `stdlib_docs` red result for a pre-existing HEAD defect (`list.is_empty([])` in
>     the same family as E6006)—the two halves are each half of the other's green-light
>     prerequisite, so they were combined into one atomic commit.
> 16. Registration of five decisions from 2026-10-10 (after R1 closure): (1) **D2 decision
>     A**—user-module exports carry `Export.type_params` together with
>     `generic_fn_type_param_snapshot()`, isomorphically to the std path (`yx_sources.rs:62/108`);
>     the declaration name becomes the authoritative source for qualified-call monomorphization, and
>     signature reverse-inference is demoted to a fallback (included in R2's scope). (2) **HKT
>     declaration side live, call side broken** (type-argument inference reports E1002 / explicit
>     type arguments report E1010) is registered as WBS 3.5 (3 tertiary tasks). (3) The **deferred
>     bucket never drains under a non-generic container** is included as an acceptance sub-item of
>     3.4.8 ②: the bucket must drain or fail loudly; silent failure to specialize is forbidden. (4)
>     **Calling a closure variable by name produces runtime E6006** is registered as WBS 3.6; the
>     semantics (i) support indirect calls / (ii) reject at compile time; **(i) has been
>     decided**—indirect calls are supported: the `CallDyn` mechanism already exists (the
>     closure-expression path at `ir_gen.rs:7945`), but the `Var` callee is demoted to a name-based
>     `Call`, even though typecheck already accepts the program. (5) The **trade-off criterion is
>     fixed**—a rejection rationale may be based only on correctness and readability; “large change
>     surface / new mechanism / high cost” does not constitute a rejection rationale, as recorded in
>     the second part of `coding-rules.md` and the HOWTO self-check list; trade-offs requiring a
>     user decision follow all four items in `AGENTS.md` “Requesting a user decision (mandatory)”
>     (situation + evidence / pros and cons for each item / explicit recommendation + cost /
>     explicit naming of breaking changes).
> 17. 3.4.8 ② landed and the F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the #14 design: `Stage::ALL` changed to Linking before
>     Monomorphization, and the MULTI_FILE table gained mono at the end; the Linking arm
>     **aggregates whole-program requests and qualifies `containing_fn` before taking each unit's
>     IR** (otherwise the deferred-bucket key differs from the merged IR's key, and the bucket never
>     drains). **① closure (D2 decision A)**: user-module exports carry a declaration-time
>     type-parameter snapshot through `Export::type_params`, isomorphically to the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically confirmed and fixed during the
>     work**: (a) **F1**—`qualify_names` rewrote only function bodies, and `ir.init` was never
>     visited, so in multi-file builds, function definitions were qualified while top-level module
>     calls remained bare names → runtime E6006 (**reproduces even without generics**; unrelated to
>     generics); the fix makes init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono side's “whole-program unique suffix” heuristic and its
>     silent `continue` on a miss → the heuristic was deleted, and misses now produce a **loud
>     E3018** (silent ambiguity is worse than failing to resolve). **Independent verification caught
>     and fixed a regression**: t6 found that a named import `use lib.{pick}` followed by a
>     bare-name call was falsely rejected with E3018 (the IR instruction name is the alias's
>     qualified name, while the request name is bare; the aggregation side only qualified by unit
>     key) → the fix makes the aggregation side **directly invoke ir_gen's** `import_aliases_from`
>     and `resolve_call_target` (extracted as a public single point, with one producer and one
>     consumer), and a matrix of pinned tests covers both forms of named import. **Retained
>     record**: the blank `DefId` in the init section was empirically determined to be “consistency
>     whitespace” rather than a live defect (the codegen name fallback can resolve it, and all four
>     forms of init call were tested successfully); this is registered as WBS 3.7.4 to close the
>     policy divergence between “function bodies use def, init relies on names.”
> 18. Closure of the init-section DefId policy divergence (3.7.4, 2026-10-11, 38bd4b9e):
>     `assign_defs` was extracted into the single point `resolve_static_targets` (Call/TailCall
>     string operands + MakeClosure), and function bodies and `ir.init` share both the same
>     implementation and the same `resolve` closure; after regenerating 208 IR snapshots, only 309
>     pairs of `(def=_)`→`(def=dN)` changed (other-lines 0; zero residual `(def=_)` entries in init
>     sections across the tree). The codegen's primary DefId path and the name-fallback path produce
>     identical translation results and execution values item by item (all four forms of init call
>     were tested), with zero drift in the corpus baseline; the probe flips and strengthens
>     according to the new policy (identity + precise index + execution value; falling back to init
>     coverage turns it red). **Erratum (2026-10-11)**: the commit numbers above were rewritten by
>     `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396 correspond respectively to the
>     rebased 375789d2/d922a551/38bd4b9e; the new numbers are authoritative in the dev history (the
>     old numbers were local temporary numbers and never entered the remote history).
> 19. 3.4.8 ③④ landed and verified (2026-10-11, 8ecf56f93). (The design basis for ③④ is in #14; ②
>     landed in #17, and this item closes it.) **Implementation**: the `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs only a pure merge
>     (aggregating requests + `link_module_irs`), with the entire entry validation left to
>     RoleClassification; `ir_generation_check` retains `us.ir` after generation (in memory only);
>     mono failures enter the collection channel in Check but continue to use `fail_with` in
>     MultiFile; `into_outcome` first calls `take_check_diagnostics` and then checks for failures
>     (structural failures no longer clear already collected diagnostics). **Verification**: four
>     rounds of independent verification (t3/t4/t10 + t5 cross-form matrix) and one review (t6) all
>     passed; the corpus differential was IDENTICAL across 340/340 entries, IR snapshots had zero
>     drift, and the test count increased from 2643 to 2656. **Existing gaps discovered and fixed in
>     the same round**: F1 (in rootless multi-file check, the request name and the IR definition key
>     came from different sources → silent failure to specialize; see the implementation note in the
>     3.4.8 row of the WBS) — the fix makes the aggregation side perform an exact reverse lookup by
>     the actual definition key (not a heuristic); the first implementation of the fix performed an
>     exact reverse lookup by the actual definition key (`flat_defs`); **same-day erratum**: that
>     bridge was deleted during the regression fix, and the root cause was instead solved by
>     decoupling `module_key`'s two responsibilities (`script_mode`), with all three using
>     `{unit key}.{name}` as the same source. The regression (CI documentation-example gate: 258
>     blocks, 10 failures → 0 failures) and its location/fix are described in the implementation
>     note in the 3.4.8 row of the WBS. **Retained record (existing, not a regression from this
>     round)**: the E3005 help text is inconsistent with the resource-protection semantics; the
>     multi-file E3005 span label is misaligned (reproducible with the HEAD binary); the
>     type-parameter position dependency on the request-collection surface (same front as 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and diagnostic surface closed (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is reachable)**:
>     interpreted calls are changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/ `SpawnFromList` push frames and return
>     control to the outer `step_one`; frames carry `ReturnTarget{Host|CallerSlot|Discard}`, and on
>     return they write the result back to the caller's slot, with multiple call instructions
>     continuing through frames' `pending_calls`. Baseline measurements showed that each interpreted
>     call consumed **2544 B** of native stack, with the crash threshold at `count_down(394)`
>     (Windows main thread, 1 MiB); after the fix, 20,000 levels of recursion run successfully on a
>     512 KiB stack thread (approximately 0 per level). The guard (`push_frame`) is thereby
>     reachable from every entry point under every `max_stack_depth` setting;
>     `ExecutorError::StackOverflow` is changed to carry the actual limit (previously
>     `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2 (deep-stack echo truncation)
>     was merged into this task by user decision** (same location in the same file; indivisible by
>     file): the echo now shows the first 16 frames + an omitted count + the last 8 frames, while
>     shallow stacks remain frame-for-frame unchanged. **3.10.1 (no compile-time guard against
>     native recursion, #447)**: first locate, then fix—the crash point is `MonoType::type_name`
>     recursion (**328 B/level** (SP probe + precise operation-level threshold determination)); the
>     same path also derives `Clone` (~400 B/level), deep-copies in `substitute`, and has
>     `type_name_hash` (164 B/level), `collect_generic_type_refs`; after all four were changed to
>     work-list processing and removal of the deep copy of the entire request on every level,
>     pathological generic recursion with `mono.max_depth=5000` now produces **E3005 (with the
>     actual limit)** instead of overflowing the stack. **3.11.1 (task-boundary error-code fidelity,
>     #448)**: the task-result error side is changed from a value to a **structured representation**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), and values
>     are restored by value across the boundary; E6001/E6003/ E6004/E6008 within tasks are now
>     word-for-word identical in Standard and Embedded messages. Two **pub API payload-shape
>     changes** include `BREAKING CHANGE` in the commit. **Commit-strategy note**: 3.9.1 and 3.11.1
>     are **indivisible at file granularity**—`interpreter/executor/executor.rs` contains both the
>     trampoline and the error-channel producer/consumer, and `TaskFailure` is defined in
>     `runtime/engine.rs`; staging only one side cannot compile (the pre-commit check compiles a
>     staged subset and rejected it twice), so they are combined under the same policy as #15 into
>     **one atomic commit**. **Known boundaries and follow-ups**: derived Drop (~75 B/level) ⇒
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation) gate carries the
>     native-recursion burden (about 3,696 B per level, with the default 256 levels ≈ 924 KiB); it
>     currently has zero call sites and is unreachable in production, and future wiring must close
>     the same family of issues; the Standard-mode task interpreter is constructed with the default
>     `ExecutorConfig` ⇒ its E6004 limit is always 1024 (configuration propagation is registered as
>     WBS 3.12, user decision B: start a separate team); the test-runtime budget is tracked under
>     separate issue #449 (pathological fixture 76s, clippy for both targets, full suite on commit),
>     which is outside this RFC's implementation surface. **Corpus directory stratification (same
>     round, 8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134), 212 fixtures in
>     total, are split into 23 category-based subdirectories, and the 88 IR snapshots are relocated
>     along mirrored paths; the corpus baseline is regenerated from the same commit (342 rows,
>     independently recalculated and compared row by row: only the `file` field and 2 new changes),
>     and 41 snapshots were regenerated because the file-header path became longer, with
>     line-by-line verification showing **only span-offset changes** (zero differences after offset
>     normalization).
> 21. LSP semantic-data pipeline unified (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, and cross-file references within a project carry
>     `resolves_to` pointing to the real definition—the old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics used the unified Driver pipeline, while
>     semantic data used a manual lex→parse→single-file typecheck in server.rs (no module registry,
>     project-level `use` resolution failed, reference events were not produced, and cross-file
>     jump/rename/find-references silently failed to resolve—the silent failure registered as D55 is
>     closed by this task). **Implementation**: (1) `Export` carries `definition_span` (harvested
>     from AST identifier nodes by extract_module_info, with the same source and value as
>     typecheck's definition event `def_span`; native std is always None), and `ModuleInfo` carries
>     `definition_file` (filled by the Registry arm from DiscoveredFile); (2) when the checker
>     drains reference events, it redirects cross-file imported items' `resolves_to` from the use
>     line to the real location in the definition file through `import_def_locations` (std imports
>     have no registration and continue to resolve to the use line); (3) the checker's module
>     identity changes from the unified `"<module>"` to the unit's file path (this value is consumed
>     only on the semantic-data surface, with zero diagnostic consumers—confirmed by all 4 consumers
>     of `env.module_name`); (4) `DriverOutcome` gains a `semantic_db` channel—fed by the CollectAll
>     forms (Lsp target file regardless of diagnostic success + SingleFile fallback); the Lsp arm's
>     tail injects top-level export definitions from non-target files (`external_definitions`,
>     allowing jumps even to files that have not been opened; type/submodule exports are not
>     injected—typecheck-side type registration does not produce definition events, so both sides
>     retain the same gap); (5) the orchestrator gains `analyze_source_in_project`
>     (`check_source_in_project` delegates to it and takes only the diagnostic half); (6) the LSP
>     side `analysis.rs` single-compilation entry point replaces the manual pipeline
>     (`update_semantic_db` in server.rs is deleted), and World keys are unified as normalized file
>     URIs (`uri.rs`: the idempotent normalization `path_to_uri(uri_to_path(u))` unifies differences
>     in drive-letter case and %XX encoding under the same key). **Incidental fix (D59 routing class
>     B—the old behavior did nothing silently and did not change the externally visible behavior of
>     any program that previously succeeded)**: cross-file jump/rename/find-references within a
>     project changes from silently failing to resolve to correctly reaching the real definition;
>     when a lexical error occurs, the old manual pipeline deleted semantic information, whereas it
>     now retains the partial AST and preserves coloring (an extension of the 4.2.6 editor
>     philosophy); when the target file has type errors, semantic data is no longer discarded along
>     with `type_result` (coloring/jump no longer disappears while typing). **Pinned tests**: 6 at
>     the driver level (target-file path key/cross-file resolves_to + external-definition-surface
>     round trip/std import fallback to the use line/error-file retention/single-file fallback path
>     key + error retention), and 1 LSP e2e test (red before green, empirically confirming that the
>     fix returned None before the change), plus 4 uri-normalization tests.

**How to transform the ten entry points (specified function by function)**:

| Entry point                                                | After the change                                                                                                                                                                                                        |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body and construct `Program { kind: SingleFile, … }` and hand it to the Driver                                                                                                                      |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                 |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                            |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                              |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver invocation + filter the result to the target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                               |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                                  |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                                     |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | Within a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and invoke `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                                  |

**The degree of duplication between `check_project` and `compile_project` (approximately 40% shared
/ 60% divergent)**. What they share is: vendor consistency checking, file discovery,
`build_registry_from` (deleted on 2026-10-26 when its last caller was removed; the historical
account is retained here), `all_method_bindings`, the parsing loop, per-file checker assembly, and
result handling. What they diverge on is items 2/3/4/5/7/8/9/10/11 in the 11-item table above.
**This shared 40% skeleton is precisely the Driver's value range**—all of the divergent 60% consists
of “a different set of stages” or “a different aggregation mode,” and these are exactly what the two
`Program` fields can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had a non-blocking issue; continue
}
```

**Key design decision: “an upstream failure causes this stage to be skipped” is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If every stage returned
`Skipped`, the information about “why did I not run?” would be scattered across 12 stages and could
not be centrally audited. Instead, `dispatch` determines this at the start of the loop according to
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect targeted by this
document—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”; it
produces no diagnostic, merely because the obligation happens to be empty. Once the rule is
established, every instance of “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                      | Text direction                                                                                                                                         |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Caused by an upstream `Abort`                               | “Stage Y was not executed because upstream stage X failed”                                                                                             |
| Caused by an empty prerequisite obligation                  | “Stage Y was not executed because there were no pending obligations (normal)”—**this item's severity = Info and is not included in the warning count** |
| Conditions not met (such as `config.mono.enabled == false`) | “Stage Y was not executed because it was not enabled by configuration”                                                                                 |

The second category is critical: it makes “normal skip” distinguishable from “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract
depends on this count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                               | Existing equivalent |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)       | Exists              |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists              |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both project
files and single files within the same process, while the CLI's `run` and `check` are two
independent invocations; placing these on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether to
return early”—they are two different checker entry points. After unification, the same
implementation should be driven by the `Aggregation` parameter rather than retaining two functions
(**this is an additional point that this document must verify; see “Known Limitations and Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                             |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                    |
| `TypeCheckResult` → `Obligations` migration | **Field types remain unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language's type system                                                                                               |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and stratification of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain
where they are; `driver/obligations.rs` holds only the aggregate container and `assert_drained()`.
L3 consumers (such as `ir_gen` reading `release_plan`) continue to receive the concrete field types
through parameters and **must not `use crate::driver`**—this is consistent with the
no-reverse-dependency red line for the driver described below.

**Dependency direction** (RFC-039's four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend back on
`crate::driver`**. The appearance of `Driver` in a `TypeChecker` import is considered a violation
and is intercepted by `scripts/ci/check-module-boundary.py`, as proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                    | The same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone files—it uniformly goes through `compile_project` (the MULTI_FILE stage table), with Script semantics driven by rootless mode; this row now describes only the `lib::run_file` / `Compiler::compile` library path)                                                                                                                                                                                                        |
| `yaoxiang run` (multi-file)         | No proof_execution and no W1001/W1002/W1003 | proof_execution was already connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” with Check / Lsp / MultiFile sharing the same implementation and no second copy of the criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` and hands them to that arm for emission) |
| `yaoxiang check` (multi-file)       | No proof_execution                          | Already connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| LSP (within a project)              | No proof_execution                          | Already connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Z3 not installed + single file      | `predicate.rs:35` **panics**                | Changed to `Abort` + an E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Z3 not installed + multi-file       | Silently skipped                            | Unified as above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

**The only intentional behavioral break** is changing the `.expect()` at `predicate.rs:35` to return
a diagnostic. This is consistent with RFC-027 §8's positioning as “not bound to a specific solver”
and with the contract already declared by `backend.rs:60` (“the caller should degrade
conservatively”)—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (1 for a single file / N for multiple files)                   |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line range                                      | Change                                                                                                                                                                                                                                                                                                                                                                          |
| -------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                           |
| `src/lib.rs`                                       | `140-147`                                       | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                   |
| `src/lib.rs`                                       | `154-167`                                       | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `141-227`                                       | Replace the `Pipeline::run` function body with Program construction + Driver invocation                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `275-278`                                       | Move dead-code analysis out and into the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                          |
| `src/frontend/pipeline.rs`                         | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `381-389`                                       | Move monomorphization out and into the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `450`                                           | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                       | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                                |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                       | Delete the manual stage sequence in `run_diagnostics` and invoke the Driver                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                       | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                       | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                                  |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they implicitly reach path 1 through `Compiler::compile_with_source`)                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                         | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                          | Correct the comment that conflicts with `1320`                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; delete “an upper layer does not run after a lower layer fails” (no short-circuit exists)                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure entry point; change the three consumers (`predicate.rs` / `checker.rs` / `ownership.rs`) to use the shared entry point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); the original wording is superseded by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is in the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; all `#[cfg(target_arch = "wasm32")]` branches (wasm branch
reachability determination is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Disposition                                                                                                                                                            |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | `test_multifile_proof_obligation_not_dropped` is **written red first**, then fixed; staged releases, with the expected diagnostic-set change recorded in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that previously compiled now produce warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                         |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                     | W first, then E; S2/S4 are split into two steps                                                                                                                        |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no more crash                                             | No break                                                                                                                                                               |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not go through proof_execution                                                                                                                      |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                        |

---

## Implementation Highlights

This document corresponds to RFC-039's global stage sequence **P3 (Fix correctness defects)** and
**P4 (Stage contract and unified Driver)**. S1–S5 below are the implementation order internal to
this document. **The acceptance criteria for every step refer to category C2 of the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration changes: the **diagnostic
sets** of all entry points are identical + corpus behavior is identical).

### S1: Establish the criteria (red first)

**A prerequisite that cannot be skipped.** The defect criterion must first be written as a failing
test.

| Deliverable                                                                                                                                                      | Acceptance                                                                                                                            |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red**. If it unexpectedly passes, the defect analysis in this document must be reviewed again                               |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions of the corpus can be compared                                          |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                               | Marked `#[ignore]`; S4 turns it green (the red state is retained in the record: after removing the sole settlement point, `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                      |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array      |
| Change `pipeline.rs:141-227` to invoke the Driver        | **The single-file path's diagnostic set and exit code are byte-for-byte identical**                  |
| Differential over the full corpus (293 `.yx` files)      | Diagnostic list (sorted by `(code, file, line)`), exit code, and stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance standard is a C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Merge the four orchestrator entry points + fix the defects

| Deliverable                                                                                                                         | Acceptance (C2)                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Change all of `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                            | `yaoxiang check` behaves consistently both inside and outside a project                         |
| Delete the LSP manual stage sequence                                                                                                | LSP and the CLI produce the same diagnostic set for the same file                               |
| **Turn `test_multifile_proof_obligation_not_dropped` green**                                                                        | The defect is fixed                                                                             |
| Fix the ownership layer's empty `Unproven` match arm (originally `checker.rs:1313`)                                                 | `test_no_silent_pass_on_unproven`: `Unproven` must always produce a diagnostic in every mode    |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                          | The compiler no longer panics in an environment without Z3                                      |

**Rollback point**: the defect fix and the structural merge must be **two separate commits**. If the
structural merge causes problems, only the structural commit can be rolled back while retaining the
defect-fix commit—in that state, `test_multifile_proof_obligation_not_dropped` remains green; the
reverse (retaining the structure while rolling back the fix) turns it red, which is an unacceptable
intermediate state and must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                                                       |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 test cases in `src/driver/tests/obligations.rs` turn green (implementation form described in the revision note to §2; the authoritative count is the number in that file) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                                                     |
| Promote obligation diagnostics from W to E   | After the full-corpus differential, manually review every new E                                                                                                                  |

**Rollback point**: the severity of `assert_drained()` can be controlled through a configuration
option, so switching between W and E does not require changing the code structure.

### S5: Finish the proof layer and wasm

| Deliverable                                                                                                                                                                                                                                            | Acceptance                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                       | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                               |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise a single timeout becomes固化 across compilations through the process-wide cache and pollutes cross-thread sharing in cargo test) | Production-path calls to `default_solver()` go to zero (tests only); cache hit rate is observable across all three consumers (the counter has already landed in 89576fafd) |
| Add a warning to the degradation paths in `checker.rs`'s `TypeChecker::ownership_inputs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                                        | A diagnostic is produced when there is no `body_checker`                                                                                                                   |
| Determine the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                    | The conclusion is handed to the “F2 Determination” subsection of `06-cleanup-inventory.md`; this document registers only the need for that determination                   |

**Note**: correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item should proceed independently of S1–S4** and must
not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustively-enumerable enum with 12 variants, and **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                                    | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is isomorphic to the mechanism already validated in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating the need to reason about invocation counts                                                                                                                                                                                        |
| **Obligation mechanism** | `Obligations` + `assert_drained()` perform **runtime** settlement, used together with the **static** gate `scripts/ci/check-obligations.py`; the **terminal severity is E** (user decision A on 2026-10-13: promote the same code to E without changing its number; **implementation form described in the revision note to §2**: the ledger does not copy the field structure; the red list—zero entries after cleanup—retains its mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not a single bug: at least 2 instances of the same class have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are in scope. Splitting W→E into two steps was intended to keep every step's C2 criterion usable—now that the obligation inventory has been cleaned down to 13 entries and the red list is empty, zero emission in the healthy state means the terminal state can directly use E (see the seventh and eighth sections of the revision note to §2) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), and **do not change** the production-side logic of the `Unproven` branches (now distributed among three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                            | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic has been fixed,” whereas the logic of these three branches is itself correct                                                                                                                                                                                                                                                                                                                      |

This design additionally eliminates two classes of cracks:

- **The crack between “promises in comments” and “code behavior.”** The “silent pass must not be
  allowed to return” statement in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, enforced by `assert_drained()`.
- **The crack of “one array entry was forgotten.”** The current implementation is five functions,
  each with a hand-written invocation sequence; after unification, there is one stage table, and a
  missing new stage causes a compile-time error.

### Approaches Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, and IR
  generation/linking) **do not involve type transitions**; they are configuration questions of
  “whether to run a particular analysis.” This approach also cannot express `Aggregation`; it would
  merely move the `check_module` / `check_module_collect_all` choice from the function layer to the
  type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this cancels
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions, each with a hand-written invocation sequence” with “one array entry was
  forgotten,” which is precisely the cause of the current 11 inconsistencies. This repository
  already contains a same-shaped counterexample: `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader`, described under `docs/src/dev/design/check/`, all have zero
  implementations.
- **Fix only the bugs without changing the structure**—this can fix the 2 known bugs, but it covers
  only 1 of the obligation fields, while `Stage` remains scattered across 5 functions and the next
  point of divergence will continue to grow from here. **It must be done first** (as part of S1/S3),
  because the structural refactoring needs a known red test to prove that the criterion is
  effective.
- **Make `proof_calls` `pub` and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, so detection must occur in the
  orchestration layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into “a Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of refactors that “centralize the
  wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing problems that were
  previously silent.”** After the fix, users may report a batch of “new errors”—these are real bugs
  that were simply never reported. This must be explicitly explained in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual judgment for each field.** For some
  obligation fields (such as `module_namespaces`), a lack of consumers may mean that consumption is
  not required by design and should not be reported. S4 must review every field individually rather
  than treating them uniformly.
- **The `Program` abstraction may be premature.** If the stage sets of certain entry points remain
  unstable for a long time, `stages()` may degrade into “a free-form parameter that accepts a
  different array on every invocation,” and the contract constraint will fail. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can only come from the 6 predefined
  combinations.**
- **Criteria dependency**: the defect-fix criteria in S1/S3 depend on the defect tests in the
  [equivalence-criteria document](07-equivalence-oracle.md) first turning red. **If that document
  has not first established the multi-file corpus layer, S1 cannot be accepted**—because all 293
  existing corpus files take the single-file path and provide zero coverage for this class of
  defect. This document does not cover the IR verifier (`verify_loose`); that prerequisite work is
  outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the order of `termination` and `ownership` is the reverse of their
  declaration). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is not yet settled.**
  `backend.rs:13-19` already explains that `Solver` is only `Send`, not `Sync`, and that the
  `Z3Backend` cache uses `RefCell`; after changing it to a singleton shared across compilation
  units, it must be confirmed that all access paths go through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependency defects** (such as the actual order
  dependency between termination and ownership). This should be opened only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item decisions. **This document leaves no unresolved items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage tables and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup; conversion
  of `checker/semantic_tokens.rs` to `include!` (implementation steps are assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker/refinement.rs` — Explicit declaration that “silent pass must
  not be allowed to return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/typecheck/layers/README.md` — Layer-order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for hard failure
  during SMT initialization
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration that “the caller
  should degrade conservably” (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream code must
  consume (after the 3.4.4/3.4.5/3.4.9 cleanups)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test's own
  statement that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus's own statement that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (invoked by `run_code`) — The playground's fourth
  compilation caller (the single-file path, so proof_execution is executed)
- `wasm/Cargo.toml:10-18` — The cdylib + wasm-bindgen reside in the shim crate rather than the main
  crate
- `build.rs:19-55` — The build-time error-code gate, a model of this project's enforcement
  mechanisms
