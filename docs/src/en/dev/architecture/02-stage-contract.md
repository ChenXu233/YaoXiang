# Compilation-Stage Contracts and the Obligation Ledger

> **Companion design document.** This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the RFC-039 body for the four-layer model, acceptance-criteria tiers, and execution-stage order;
> see the [directory index](index.md) for the role of each companion document.

## Position and Scope

This document is the implementation blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“Which stages ran in this compilation, which ones did not, and why?” is currently
wired manually across six paths and ten entry functions, with no single place able to answer that
question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code are being broken by the architecture, not by the logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` reads (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile-time error, no fallback, no silent
pass……must not allow silent pass to return.” However, this invariant holds only on the **single-file
path**: proof-function constraints such as `x: Sorted(3)` pass silently on the multi-file
`yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed, and no
diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, with no runtime registration) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (a single `dispatch`) and the individual migration path for each of the ten
  entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- Every affected file and line number, compatibility impact, and implementation order.

### Not Covered

- The four-layer model, dependency-direction rules, and bounce-back gates → `01-routing.md`
- Equivalence criteria (C1–C6 tiers and three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order for P1–P10 and G1–G10 acceptance gates → RFC-039 body

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactor is needed and **in what order** to perform it; this document
defines L1's **concrete shape**, a **file-by-file change list**, and the **implementation stages
within this document** (corresponding to P3 “Fix correctness defects” and P4 “Stage contracts and
unified Driver” in RFC-039's global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

Equivalence criteria follow the **C2 (orchestration changes)** category in the
[equivalence-criteria document](07-equivalence-oracle.md): the same diagnostic sets for every entry
point + the same corpus behavior.

## Current State

> Every item in this section is a **verified fact**, with a file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect in which one incorrect wiring point makes the entire repository fail silently

This differs in nature from the other two defect classes (module boundaries and test wiring):

| Defect                | Typical manifestation               | Signal                   |
| --------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntax error  | Incorrect source code               | Diagnostic               |
| Type mismatch         | Incorrect type                      | Diagnostic               |
| Missing stage wiring  | A stage is never invoked            | **No signal whatsoever** |
| Unconsumed obligation | A field is populated but never read | **No signal whatsoever** |

The latter two share a defining characteristic: **failure produces no error**. They therefore cannot
be solved by “writing the code more carefully” or “reviewing more strictly”—code review can show
what was written, but not what was **omitted**. This is precisely the root cause identified by
RFC-039: “This project treats ‘design’ as a documentation convention rather than an executable
constraint.”

### The repository already has enforcement mechanisms

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes + 8 W codes) are subject to a
  **compile-time hard gate** through `build.rs:19-55` and `tools/code-tables`. Every code is
  compared against the RFC-013 code table; any discrepancy causes an immediate `panic!` that rejects
  the build.
- `src/package/` has a high test density (**76 files / 13,012 lines**, including 6,227 lines under
  `tests/`, or 47.9%). Every module's test subtree is explicitly wired. This is the most completely
  wired test area in the repository and can serve as the structural model for stage-layer gates.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(Line-count convention: `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path taken                                                             |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages called directly                                                        |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project uses 3c; otherwise manual lex→parse                                     |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages of the single-file path (`pipeline.rs`):

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`         | `run_ir_generation`, **with monomorphization embedded in it** (`381-389`) |

### Eleven stage-coverage inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point “does not do this”; it means that “nothing in that entry point does this”**—which is precisely
the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single-file)                            | `compile_project`                                          | `check_project`                                                | `check_source_in_project` (LSP)    | `compile_embedded_module`               |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------- | --------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                         | **No**                             | **No**                                  |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, no configuration gate)              | **No**                             | **No**                                  |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                  | **No**                             | **No**                                  |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                            | Yes `350`                                                      | **No**                             | **No**                                  |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                | **No**                             | **No**                                  |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | N/A                                                            | N/A                                | **No**                                  |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                 |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                   | `discover` (`454`)                 | N/A                                     |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                | **No**                             | **No**                                  |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                        | **No**                                                         | **No**                             | **No**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                         | **No**                             | Yes (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. The current-state differences after
> P3 are as follows:
>
> - **Row 1 has been fixed**: the five entry points share the same proof-execution implementation,
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entry points); the sole-consumer
>   defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and the dead-code-family warnings, and the W-code sets of four paths—single-file
>   run / multi-file run / single-file check / multi-file check—have been normalized. The single
>   source of truth is the shared implementation on the `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to single-file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3; potential unproven defect).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “ruling #434” previously had no documentation record; it has been
>   registered as RFC-039 **D57**.
> - **Asymmetry in embedded std (new fact outside the table, recorded on 2026-10-09)**: the
>   single-file path unconditionally injects `std.list` through `merge_embedded_std_ir` (required
>   for desugaring for-loops; #117 hard switch), whereas multi-file `discover` previously recognized
>   only explicit `use` statements. Project-local for-loops compiled successfully but failed at
>   runtime with E6006 (confirmed by a probe). This has been fixed and WBS 4.10.2 has been closed.
>   The same-family “multi-file lacks a single-file step” parse hard-abort peculiarity was
>   registered as WBS 4.10.1—**it has also been fixed** (landed immediately after 4.2.2; the Check
>   path was downgraded to per-file collection). This audit method examines “stage coverage/field
>   consumption”; this case concerns differences in compilation-unit membership, so it falls outside
>   that dimension.
>
> - **Embedded std registration-surface coverage (discovered during 4.2.5 implementation, fixed)**:
>   the Registry arm repeatedly harvests registrations for embedded std units through
>   `extract_module_info`, displacing the native half registered through `with_std()` according to
>   the “native + yx surface merge” rule (`result.is_err` and others are lost → std.test falsely
>   reports E1043). This is the audit's third missed dimension: the merge semantics of “two
>   registration sources for the same module key.” The single-file path's registry is formed exactly
>   once through `with_std()`, while the multi-file/Check Registry arm gains coverage only by
>   inserting each unit individually.

**Two points require precise wording; otherwise the conclusion would be incorrect:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
  (`run_file` → `Pipeline::run`), it does. The reason lies in `compile_project`: it packs the
  complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function has **no
  `result.warnings` read site anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 also comes from a different source**: `check_project:385-395`
  uses `surfaces.bins` (the manifest-declared surface), whereas `compile_project` uses `is_bin_role`
  (`250-252`, determining only whether a manifest exists). The two functions give different answers
  to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for obligations.**

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

The semantics are: “The predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed before the constraint can be decided.”

**Step 2 — The consumer is inside the checker, but it only produces and never consumes.**

`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                          | Behavior                |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → pushes a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Pushes a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Pushes a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real defensive line rather than a forward-looking fallback: silent pass must not be allowed to return.`

In other words, the author knew that “emitting no diagnostic” was a defect and explicitly classified
it as a **known intermediate state with nowhere to go**. The design assumed that “someone would
eventually come to read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.**

`check_module_impl` declares a local `proof_calls` and collects into it (originally
`checker.rs:1234-1235`), then writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof-function obligations` (originally `:1439`). The
field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.**

`TypeCheckResult.proof_calls` (`types.rs:29`) has **only one read site in production code**:
`src/frontend/pipeline.rs:187` (passed as an argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(The other repository-wide occurrences of the `proof_calls` identifier fall into three categories,
none of which consumes this field: the three production-side collection sites in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); `verdict.rs:61` is a same-named field of
`ProofResult`; and `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points passes through that layer.**

The four entry points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module`
(`1374`)—**all call `TypeChecker::check_module` directly** (`124` / `493` / `1386`) and do not call
`pipeline.rs`. Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself follow the discard
path.**

`compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible for
compiling embedded std. This means that **the embedded std module's own proof obligations are not
executed either**.

**Step 7 — Consequence: constraints pass silently.**

`y: Sorted(3) = 5` (`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs
successfully, and produces no diagnostic** on the multi-file, check, and LSP paths. The proof
function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point inside the layer.**

When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`):

```rust
// Originally checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the promise at the `5164` branch that “silent pass must not
be allowed to return,” and is **independent of the orchestrator problem**—even if the entry-point
layer is completely fixed, this location will still remain silent. (Timing: it should be handled in
the same batch as the obligation ledger because it is the same class of problem.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero occurrences of the
three keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that the `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop immediately before `pipeline.rs`**.

The test file's own documentation comment already states this.
`rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases belong at the .yx layer.`

**This is exactly the shape of the problem: the tests verify that “the obligation was populated,”
whereas the bug is that “the consumer never reads it.”** A test that covers only the producer and
not the consumer is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria is the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and the
principal acceptance mechanism for C2 stages. In practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero matches for the whole-directory
  glob).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this. The file is annotated with `// expect: compile-error E4018` (line 15), and the
comment in its header says on line 7 that its status is “❌ should be rejected at compile time”—**it
passes precisely because it takes the only path that executes the proof function**.

**Conclusion: the third criterion layer needs an additional multi-file corpus layer; otherwise, it
cannot serve as the acceptance tool for this document.** See “Implementation Highlights” S1.

### Obligation fields: 13 fields that are “production implies contract,” with no mechanical guarantee

Of the 13 fields in `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, **13 fields**
have documentation comments that explicitly state “produced by stage X → consumed by stage Y”; in
other words, they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention:** definition line = the field's **declaration line** in `types.rs` (the
> `pub` / `pub(super)` line), not the first line of its doc comment—the comment block for one field
> may span multiple lines, and only the declaration line provides a unique anchor. The **struct
> range** in the sentence above (`src/frontend/core/typecheck/types.rs:16-76`) moves with the
> struct's line number and is mechanically reconciled by gate **F5** (an error immediately turns it
> red); definition lines are reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by `PlanId`**:
> read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); completion marker `:1095`) | | `escaped_refs` | `40` | ownership → IR
> generation | | `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`)
> | | `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46`
> | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced by `inference/expressions.rs:4550`,
> consumed by `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); completion marker `:1097`) |
> | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | | `match_scrutinee_types` |
> `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` | `72` | typecheck → IR
> generation |

**Reclassification record (3.4.9, user ruling A on 2026-10-13):** the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design,
it is consumed within the typecheck phase and has zero cross-stage reads, so it is not an
obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed in this round).
- **Disposition**: delete the documentation claim that it is “mirrored for consumption by the middle
  side/LSP”; retain the field and its in-phase consumption. The field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP genuinely needs the
  table in the future, register it under 4.6.x; **do not invent a consumer in this round**.

**All these fields cross layers keyed by `Span` or by name tables.** The equivalence-criteria
document already points out that a mismatch in a span-keyed contract can “fail silently, with no
error whatsoever” (`ReleasePlan` is a typical example: if the two sides compute spans differently in
any one place → the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**. The consumer
  is in a downstream module, and that downstream module is on the mandatory path of every entry
  point.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to a
  **different entry-point implementation**. The orchestrator's four entry points do not pass through
  the `pipeline.rs` layer at all.

**Verified fact**: the repository currently has **no mechanism whatsoever** that guarantees that
these 13 fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride
with IR generation.

### Registration of non-obligation fields: channel / in-phase / in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation fields”
table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                                |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit-identity metadata (0 matches in `src/middle`)                                                                                                                |
| `diagnostics`             | Error-diagnostic channel (existing channel, not a “production-implies-contract” relation)                                                                         |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its documentation comment does not declare a cross-stage contract ⇒ not incorporated                           |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                              |
| `semantic_db`             | Consumer is in LSP (externally delivered through the `DriverOutcome.semantic_db` channel from 4.6.1, `src/lsp/server.rs:482`); not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`; documentation comment does not declare a contract                                                                    |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                       |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; see the reclassification record in “§Obligation fields”)                                                                       |

**Why this is not “wiring up a consumer”**: creating a downstream consumer for these fields would
invent spurious obligations and disguise fields that “do not require cross-stage consumption by
design” as contract defects. The correct closure mechanism is **reverse containment**—the set of
`TypeCheckResult` fields = the obligation inventory ∪ the non-obligation registry. This is
hard-enforced by criterion 1 of `scripts/ci/check-obligations.py`: adding a field without
registering it, deleting a field without synchronizing the registry, omitting a reason for a
non-obligation, or overlap between the two tables all immediately turn the gate red.

### Four additional contract defects in the proof layer

All of the following come from analysis of the proof layer. They are independent of the entry-point
divergence problem, but all belong to the same class: “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation, with no mechanical enforcement; the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (locations are anchored by function name/driver point, with line
numbers taken from the current tree):

| Actual order | Call site                                                                                                                                                  | Corresponding layer |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (the three sites `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                            | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                               | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not in `layers/`)                                                                                          | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                     | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comment and do not correspond to the call order; the README has no “number
→ call timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds termination errors one by one with `add_error` and then
continues downward; “an upper layer does not run when a lower layer fails” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** hard-failure panics were removed by 3.3.1 in P3 (the SOLVER slot was
> made optional; absence conservatively degrades to `SMTResult::Unknown`); “silent skip (no
> injection)” was given a W1081 signal by 3.4.2. Unification of the three forms (singletonization)
> is to be performed according to **RFC-039 D58**, with the real fix at `proof/smt/backend.rs`
> (adding a process-wide shared singleton + a `with_shared_solver` closure-based access point). The
> `default_solver() → &'static` formulation in the change list in section 02 is subject to D58 (a
> bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                   | Acquisition strategy                                      | When the solver is unavailable                             |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>`  | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected through `check_module_impl`, originally `checker.rs:1265-1268`) | Injected at construction time through `with_solver_owned` | `None` → **silent skip** (no injection)                    |
| `ownership.rs:627` (back-edge-cut decision)                                                | **New instance on every call** through `default_solver()` | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure must
remain a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint
exceeds the kernel's capabilities’.” By contrast, the contract documentation at `backend.rs:60`
says: “`None`: backend unavailable……**callers should degrade conservatively**.” **Two philosophies
coexist in the same codebase, and the hard-failure path panics rather than returning an error.**

**(c) A new Z3 context is created for every back-edge decision, making the cache effectively
useless.**

`ownership.rs:627` calls `default_solver()` on the hot path of back-edge-cut decisions. Meanwhile,
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72` states:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call invokes `Z3Backend::new()`, creating a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the documentation comment at
`z3_backend.rs:17` says that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache never hits in this invocation mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. The same backend has
two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match actual behavior.**

The two `body_checker` fallbacks in `check_module_impl` (originally `checker.rs:1283-1287` and
`1289-1293`):

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
whatsoever**. (Proposed handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The `check_module_impl` comment says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—the “before” in the
comment is actually **two layers later**.

### Current wasm state: carried by a shim crate; the branches in 29 files are load-bearing

The wasm target **has been built and is built in CI**. The `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                       | Evidence                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Main crate is `rlib` only                                  | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                         |
| **The shim crate provides `cdylib` + `wasm-bindgen`**      | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                         |
| The shim depends on the main crate (`rlib`) as its library | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| Main crate has a wasm-target dependency section            | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                           |
| The shim directory is excluded from the main workspace     | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI contains four wasm builds**                           | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten       | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL; a missing file degrades to a warning)                                                                                                                                                                 |
| **29 files** contain the literal `wasm32`                  | Of these, **22** have actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (the inventory and per-entry verdicts are in the “F2 verdict” section of `06-cleanup-inventory.md`)                                                                              |
| `orchestrator.rs`: 16 occurrences                          | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                       |
| `src/lib.rs`: 11 occurrences                               | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                                 |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`), while the shim takes a different path.

**This directly affects this document**: the playground entry point `compile_playground`
(`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and passes it to `Driver::run`
(the JS-side entry point is `run_code`, `:122`)—the **single-file path**. Therefore:

| Path consuming `proof_calls`                              | Executes the proof function? |
| --------------------------------------------------------- | ---------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                      |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                      |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                      |
| Multi-file `run` (`compile_project`)                      | **No**                       |
| `check` (`check_project`)                                 | **No**                       |
| In-project LSP (`check_source_in_project`)                | **No**                       |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** During unification of the Driver, this path
is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`; it shares the same
single-file stage table as `SingleFile`, `:135`); the implementation record appears in the **4.2.8**
row of `09-execution-wbs.md` (`wasm` now uses `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 verdict” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time; the six `not(wasm32)` arms are load-bearing in the native three-stage tables; there is
no directly removable low-risk item. The RFC-039 open question “build a wasm target, or delete these
branches” is now **obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustive, with no runtime registration)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
Rust's exhaustive `match` can enforce at compile time that every new stage must be handled by the
orchestration layer—the same mechanism already proven effective by the opcode table (RFC-039 routing
table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check                 Project
    Discovery,          // File discovery                           Project
    Parsing,            // Lexing + parsing                         PerModule
    Registry,           // Module-registry construction             Project
    RoleClassification, // Role classification (Script/Bin/…)        Project
    Typecheck,          // Type checking (including proof layer)    PerModule
    DeadCodeAnalysis,   // Dead-code-family analysis                Project
    ProofExecution,     // Compile-time proof-function execution    PerModule
    GlobalSlotAlloc,    // Global slot allocation                   Project
    IrGeneration,       // AST → ModuleIR                           PerModule
    Monomorphization,   // Monomorphization                         Project
    Linking,            // Cross-module linking / IR merging        Project
}

impl Stage {
    /// All stages. When a variant is added, this array and the exhaustive match in `dispatch` both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module and which must run once per project” into a
type-level fact. Stages in the `Project` scope are structurally guaranteed by `dispatch` to run only
once. This eliminates questions such as whether `allocate_global_slots` should be called once per
file in the current `orchestrator.rs`.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07):** two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to the topological order of `Stage::ALL`, with dead
>    code **before** proof. The current `check_project` runs proof before dead code; the two have no
>    data dependency, and the diagnostic sets are identical (C2 set semantics). Only the within-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck and
>    before proof) already conforms to ALL order, so byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved **before** `Registry`/`RoleClassification` (C3, user ruling on 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products. The
>    original order would force the Registry arm to “hide parse,” making the stage table
>    misrepresent the data flow; multi-file parsing consequently fell from two passes to one. The
>    current state is also recorded faithfully: multi-file parse errors used to be hard aborts (the
>    `?` propagation from `build_registry_from`, in tension with CollectAll semantics, registered as
>    WBS 4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling: Rust-style collection
>    semantics + option B): the Check path degrades parse failures to per-file diagnostic collection
>    and removes the affected file from the compilation unit (it does not enter the registry, and
>    importers report E5001); the MultiFile path retains hard aborts (under FailFast, a bad file
>    cannot produce IR, which is correct semantics and remains permanently pinned).
> 4. Check-form implementation (4.2.2, user ruling on 2026-10-09) added two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events. The initial draft omitted these
>    (these products were produced by `discover_with_used` in the orchestrator era but discarded by
>    the `discover` wrapper, so the data flow was lost before it reached the table); b. The
>    within-file diagnostic-order normalization was expanded to the full form of note #2: E3020
>    entry validation moved into the RoleClassification arm (Check has no Linking stage, so the arm
>    that owns `surfaces` and the AST performs validation), and the within-file diagnostic order =
>    stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof). The diagnostic set
>    remains unchanged (C2 set semantics); only the order of stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design goal**: turn “a field is produced but nobody consumes it” from an undetectable condition
into a fact assertable at compile time or test time. RFC-039 already identifies this as “the single
highest-value item.”

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
    /// Each unconsumed field produces one W-level diagnostic (by default) or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers):**

| Case                                           | Criterion                                               | Handling                                                                                |
| ---------------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| An obligation is read by its declared consumer | The field in `Obligations` is `take()`n/marked consumed | Pass                                                                                    |
| An obligation is non-empty but has no consumer | The field is non-empty and not consumed                 | **Produce a diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| An obligation is empty                         | The field is empty                                      | Pass (nothing to consume)                                                               |

**Why start with W level rather than E level**: fixing obligation discards will **change the
diagnostic set**. C2 requires that “the diagnostic sets of all entry points be identical.” Moving
directly to E level would suddenly add a large number of previously silent `Unproven` diagnostics to
`yaoxiang check`. Splitting the work into two steps—first W-level observation, then promotion to
E—keeps every step's criteria usable. See “Implementation Highlights” S4.

**`assert_drained()` has exactly one call site**: `Driver::run`, at the end of the stage table and
before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing at
least twice but with no third file reading it → failure). `assert_drained()` is the **runtime**
gate, while the script is the **static** gate; the two complement each other, and both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed):** this note records the implemented
> shape, measured numbers, and 4.3.2 interface of the sketch and three-tier table above, with a
> one-to-one correspondence to code file:line. The landing commit is `1941cc47a` (verification tree
> `99079bc52`). It is also the **third** of the three places that enforce the red list's “no
> silence” rule (the other two are the audit surface and the static gate).
>
> **I. Implemented artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (`:201`, field
>   name/consumer stage/red-list status), **non-obligation field registry `NON_OBLIGATION_FIELDS`
>   (`:150`, 8 entries with reasons)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   function `settle_obligations` (`:577`), external emission `emit_obligation_diagnostics`
>   (`:654`), non-consumer gating `consumer_visible_unit` (`:538`), skip criterion
>   `consumer_stage_skipped` (`:549`), IR-generation delivery set `IR_GENERATION_DELIVERED`
>   (`:285`), and W1090 code constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: the end of `Driver::run`'s stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one makes the runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only external read path for this
>   obligation on `TypeCheckResult`, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—all four use the same
>   delivery point: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct request-table read) / `:1221`
>   (MultiFile·Check: aggregated by the Linking arm, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is written above the entry at `:26-31`—the settlement
>   granularity is “unit × obligation,” with no user-identifiable source location, so no dummy span
>   is inserted); six-language text is in `locales/*.json`. **Emission severity**: from 4.3.2 onward
>   it is **error** (the same code promoted to E; see the third section)—determined by the explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code itself remains in
>   the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the table above                   | Implemented criterion                                                                           | Code                   |
> | ---------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------- |
> | An obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ real read reconciled by the static gate; see section V)  | obligations.rs:618-623 |
> | An obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**; see the third section) | obligations.rs:624-630 |
> | An obligation is empty                         | `Obligation::is_produced` performs the per-field emptiness check → pass                         | obligations.rs:587-591 |
>
> Two **additional tiers** (2026-10-13 user ruling; not in the original sketch):
>
> 1. **S1**: the consumer stage is absent from the current `ProgramKind`'s stage table → **not
>    treated as an outstanding obligation** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (`:121-126`).
> 2. **Skip exemption**: the consumer stage is skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not treated as outstanding (obligations.rs:598-602). `NoObligations` does
>    **not** qualify for an exemption—a non-empty obligation coupled with a stage that claims it has
>    none precisely means the ledger is unbalanced and must be reported.
>
> **III. External-delivery policy (ruling “C”) and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism externally emits only **W level** (W1090); there is
> **no strict switch**. **4.3.2 (final state, user ruling A on 2026-10-13)**: **promote the same
> code to E**—the `W1090` code remains unchanged, but the emission severity is raised to `error`
> (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code is
> added** (one fact with two codes would create parallel representations). **Promotion changes only
> the emission severity, not the emission conditions.** This is a **D-class breaking change under
> D59** (an externally observable behavior change), recorded faithfully: healthy programs emit
> nothing (all obligations have delivery markers, so nothing is reported), and therefore existing
> corpora and user programs are unaffected (342/342 “difference: empty”); however, whenever a run
> actually encounters an undelivered obligation, the diagnostic **counts toward `error_count`,
> renders as `error[W1090]`, and produces a nonzero exit code**—unlike the previous “W level does
> not block compilation” behavior. The decoupling of code family from emission severity has been
> registered in RFC-013's “emission channel” section (the code prefix determines only numbering
> ownership; severity is determined explicitly at the construction site). Mechanism facts
> (measured): severity comes from explicit `.severity()`; otherwise, “code starts with `W` →
> Warning” (`codes/builder.rs:186-203`), and the registry's `ErrorCategory` does **not** participate
> in severity determination. Exit codes are determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`); **W1090 follows
> `error_count` because of its explicit error severity (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's ruling C on
> 2026-10-13; the current red list is empty—see section IV), for a reason that can be recomputed:
>
> - Ruling A (externally emit red-list entries at W level): measured cost: `interface_impl_registry`
>   is populated by the typecheck pre-registration table and is **always non-empty**, so
>   per-compilation external delivery is a **constant signal** with no discriminating power, yet it
>   breaks three existing external contracts—“clean programs emit no diagnostics / LSP emits no
>   diagnostics / `warning_count == 0`.” `cargo test --lib --all-features` measured **12 existing
>   test cases turning red**, of which **8 are on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 are in `src/driver/tests/`). The corpus
>   baseline changed in 24/342 fixtures, and the only viable modification would be “filter W1090 out
>   of zero-diagnostic assertions,” which weakens existing criteria.
> - Ruling B (downgrade to Info level) would still break 9/12 list-style assertions, and Info is not
>   “emitted according to the ruling's level.”
> - Ruling C's measurement: **zero changes to existing test cases and zero changes to the corpus
>   baseline** (342/342 “difference: empty, behavior equivalent”). Runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains completely silent throughout,
>   and removing any delivery marker makes it fire (each of the three delivery points was tested by
>   removing its marker; the CLI `check` path emitted two `warning [W1090]` entries when the
>   Check-arm marker was removed).
>
> The red list's “no silence” property is provided by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (carried on every run, including WBS IDs
> and evidence pointers); ② `check-obligations.py`, which prints the red list on every run
> (currently empty) + performs stale detection; ③ this note.
>
> **IV. Current red-list state (ledger closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three closure tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place form detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST form
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`); the call site at
>   `:7496` replaces the entire Call with `CreateVariant`. The function's doc explicitly states that
>   “span keys are unreliable across systems (the two sides assign different semantics to the span
>   of a Call node), so detection is performed in place”—**wiring a consumer would reintroduce the
>   already-fixed span-mismatch defect**. Removal surface: the `types.rs` field; collection at
>   `checker.rs:1489-1493/:1565`; collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`; and the `VariantCtorCall` structure in `environment.rs`. The type
>   inference of `infer_variant_ctor_call` and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550`, the decision point; written to `overload_resolutions` only on the
>   unique match at `:4550`). What was deleted is the **zero-reader mirror** injected into
>   `TypeCheckResult` at `checker.rs:1558`. The naming fact still has a single expression: the
>   candidate-name table (in phase) + span-keyed decision (cross-stage; constructed at
>   `ir_gen.rs:380`, consumed at `:7632/:7705`). The definition-side mixed name is computed
>   independently by `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in this document's §Obligation fields table). What was deleted is the
>   documentation claim that it is “mirrored for consumption by the middle side/LSP”
>   (`types.rs:47-48`); the field and its in-phase consumption are retained
>   (`operator_interfaces.rs:623`, operator lookup; seven constraint-solving lookups in
>   `inference/expressions.rs`; the read-only accessor at `checker.rs:214-217`). Direct proof of
>   zero cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not
>   wire a consumer**: the table's contents are an environment mirror populated by the typecheck
>   pre-registration table (always non-empty). If LSP genuinely needs it, it must be newly
>   registered under 4.6.x; creating a consumer for a field that “does not require cross-stage
>   consumption by design” would introduce a spurious obligation into the ledger.
>
> After ledger closure, the **field census is closed by “obligation inventory ∪ non-obligation
> registry”** (`NON_OBLIGATION_FIELDS`, with reasons)—the reverse-containment endpoint of gate
> criterion 1 protects this end to end (adding a field without registration / deleting a field
> without synchronization immediately turns the gate red). The red-list **mechanism** is retained:
> gate criterion 4's stale detection and the `--selftest` `stale-red-list` fixture continue to cover
> its detection surface; any future re-registration requires an explicit ruling.
>
> Alongside ledger closure, the inventory changed from 16 → **13** (`INVENTORY` / `Obligation::ALL`
> / this document's §Obligation fields table). The gate's `--selftest` now covers seven fixture
> categories (adding the “unregistered field,” “stale anchor,” and “empty red list” forms), under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Actual criteria of the static gate (9.2)** (three additional hard criteria beyond the
> sketch's single sentence)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document's “§Obligation
>    fields” table (including the table-header count declaration); any drift immediately turns the
>    gate red. In addition, the set of `TypeCheckResult` fields must be **exactly equal** to
>    “obligation inventory ∪ non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding
>    a field without registration, deleting a field without synchronization, omitting a reason for a
>    non-obligation, or overlap between the two tables all immediately turn the gate red.
> 2. **Read evidence**: every non-red-list obligation must have a real read in the **declared
>    consumer file** (`.field_name` member access; comments/tests/strings do not count)—this makes
>    “delivery marker ↔ actual downstream read” a hard criterion (the primary risk of mechanism A is
>    precisely that the two drift apart).
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    red-list entries, conversely, must not be marked.
> 4. **Red-list stale detection**: a red-list field must have zero real reads (the emergence of a
>    consumer turns the gate red), and every entry must carry a WBS ID and evidence pointer. There
>    is also `--selftest`: built-in negative/positive fixtures, with negatives always red and
>    positives always green (demonstrating that the detector is not idle).
>
> **VI. Every deviation from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains owned by `TypeCheckResult`, and the ledger records only “inventory +
>    delivery markers + settlement conclusions.” Reason: copying them would create two sources of
>    truth (the individual fields are produced by different places—the checker's typecheck phase and
>    the ownership layer)—and they would inevitably drift; the cross-layer data plane should not be
>    duplicated into the orchestration layer.
> 2. The sketch's `take()` / “mark consumed” was **not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. Reason: an obligation and its product are
>    the same data, and `take()` would remove what downstream code needs; moreover, 11 IrGeneration
>    obligations take the form of “handing the complete `&TypeCheckResult` to ir_gen,” so they
>    cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (from 4.3.2 onward, W1090 is E level)—the single-file path
>    uses `PipelineError::TypeCheck` (the external-emission arm in obligations.rs:654), while
>    multi-file/embedded paths record an `OrchestratorError` failure (otherwise `result.errors`
>    would have no consumer). Reason: returning `Vec<Diagnostic>` would lose form-based
>    routing—Check diagnostics must remain attributed to individual files.
> 4. The first tier of the three-tier table, originally “the obligation is read by its declared
>    consumer,” became “a delivery marker exists”; “marker ↔ actual read” is delegated to the static
>    gate (criterion 2 of section V).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**; it
>    became **promotion of the same code to E in 4.3.2** (promoted even without a strict switch,
>    because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no
>    third-file read” to the four criteria in section V.
> 7. The two additional tiers not present in the sketch (S1 and skip exemptions) appear in the
>    second section.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table actually became
>    **20 test cases** in `src/driver/tests/obligations.rs` (first an `#[ignore]` skeleton, then
>    made green; the red-state record: removing the sole settlement point produces `pairs: 0` and
>    six failures). The S1/S4 rows have been updated to the actual file.
>
> **VII. 4.3.2 implementation record (severity + gate hardening)**
>
> - **Inputs for the field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation
>   kinds actually drained in the run, available per run) and the five counters (the conservation
>   identity in the second section); together with the audit surface `findings`, these provide the
>   fact of “whether this field has been consumed by any entry point.”
> - **Severity: promotion of the same code to E has landed** (2026-10-13 user ruling A; see the
>   final-state registration and D59 D-class breaking-change declaration in the third section).
>   Implementation: explicit `.severity(Severity::Error)` in `to_diagnostic`; external-emission arms
>   route by severity. Mutation evidence: removing any delivery marker causes that form of
>   compilation to fail, and `check` exits nonzero (`error [W1090]`); after restoration, the healthy
>   state emits nothing (corpus 342/342, zero changes). Locale text now states in all six languages
>   that the code belongs to the W family while its emission severity is error.
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses the `file:line` anchors in **non-obligation
>     registry reasons and red-list evidence**, validating that “the file is locatable (uniquely) +
>     the line number is in range + the symbol immediately following the anchor actually appears in
>     the anchored file”; an anchor pointing to a nonexistent symbol turns the gate red. It includes
>     the negative fixtures `tests/baselines/obligations-gate/stale-anchor/` (one nonexistent
>     symbol, one out-of-range line). Note: the red list is currently empty, so the actual landing
>     point of this criterion is the **non-obligation registry** and future evidence pointers (the
>     same code path).
>   - **F2/R3 reverse containment**: the gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registry count = difference”; a nonzero difference turns it red. It
>     includes the negative fixture `unregistered-field/` (a new struct field not registered).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable at real
>   entry points (a unit whose typecheck fails does not retain its `type_result`, so the obligations
>   do not exist in the first place); this tier is pinned by a synthetic `State` test case, while
>   the real end-to-end path covers only `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 second-level task closure (red-list empty + final E-level obligation diagnostics)**
>
> **8.1 Three-round implementation timeline** (each item corresponds to a code file:line; this round
> only records conclusions and changes no code)
>
> | Round                  | Conclusion                                                                                                                                                                                       | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
> | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | 4.3.1 Ledger goes live | `State::assert_drained()` sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                             | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; external emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                    |
> | 3.4.4 Ledger closure   | `variant_ctor_calls` field deleted—its declared responsibility is equivalently fulfilled by in-place form detection on the IR side                                                               | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.5 Ledger closure   | `method_overload_ir_names` cross-stage mirror deleted (retained in the in-phase candidate-name table + `overload_resolutions`)                                                                   | Decision consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                               |
> | 3.4.9 Ledger closure   | `interface_impl_registry` reclassified as an **in-tree fact**: the “mirrored for middle/LSP consumption” declaration deleted; in-phase consumption retained; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; doc comment at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven sites in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor `:214-217`); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Severity         | **Promote the same code to E**: explicit `.severity(Severity::Error)`; the code remains W1090; external emission is routed by severity (first failure takes priority)                            | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level arm); code-family/severity decoupling is registered at `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                          |
>
> **8.2 Closure conclusions**
>
> - **Red list is empty**: the number of entries in `INVENTORY` (`src/driver/obligations.rs:201`)
>   whose `red_list` is `Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs`
>   → 0). The mechanism is **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate
>   criterion 4's stale detection, and the `stale-red-list` fixture; the gate prints “red list: 0
>   entries—ledger closure complete.”
> - **Final E-level obligation diagnostics**: the **code and W-family membership of `W1090` remain
>   unchanged**, while the emission severity is `error` (final D5 state, user ruling A on
>   2026-10-13). This is a **D-class breaking change under D59** (an externally observable behavior
>   change), registered in both the third section of this note and RFC-013's “emission channel”
>   section; **zero emission in the healthy state ⇒ zero impact on existing corpora and user
>   programs** (corpus 342/342 “difference: empty,” zero IR-snapshot drift; independently verified
>   by t11 and reviewed by t12).
> - **Closed field census**: the `TypeCheckResult` field set = obligation inventory ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a measured consumption-side
>   reason); gate F2 explicitly prints the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: all hardening items recorded in this note have **landed**
>   (F1/F2/F4/F5; see sections V, VII, and VIII). The only two remaining open items are 8.4's
>   **known boundary** and t7 F5, which has been moved to an issue; neither is an unimplemented
>   criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/review round**
>
> | Source        | Finding                                                                                                                                                                        | Status                                                                                                                                                                                                                                                            |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is a plain-text pointer and can silently become stale when a replacement consumer is removed                                                                 | **Landed**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`                                                           |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, so a new field that was not registered silently passes                                                                             | **Landed**: F2 explicitly prints the difference (`:577`) + F4 declaration-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                          |
> | t4 R4         | The W-only versus promoted-to-E policy was not registered                                                                                                                      | **Landed**: the third/seventh sections of this note + RFC-013 “emission channel” + W1090 help text and `error_output` examples in the six `locales/*.json` files                                                                                                  |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (legal overloads are falsely reported as `W1005`)                                                                | **Moved to issue `#450`**, assigned to WBS **4.9 correct reference-pool implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237`)—outside this second-level task                                                                                     |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                              | **Known boundary** (see 8.4)                                                                                                                                                                                                                                      |
> | t11 F8        | Validator-side note: mutation-driven testing once wrote to the main repository (cleared immediately with `git restore`; the affected round's output was invalidated and rerun) | **Informational item** (not a deliverable defect)                                                                                                                                                                                                                 |
> | t11 F9        | An intermediate main-repository state (unstaged files causing this document's table header to differ from `INVENTORY`) triggered a red gate                                    | **Informational item / resolved**: the frozen commit is self-consistent and passes; “directly committing an intermediate state turns the gate red” is precisely F2/F4's intended behavior                                                                         |
> | t12 F12-1     | This document gave the struct range as `16-73`, while the actual range is `16-76` (three comment lines were added at t9 without mechanical reconciliation)                     | **Landed + mechanized**: range corrected to `16-76`, and gate **F5** added (range anchor; an error immediately turns it red); related numbers were grep-aligned (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                               |
> | t12 F12-2     | Settlement in the multi-file form once overwrote the first failure (the type error in `zzz.yx` was masked by W1090)                                                            | **Fixed** (captain amended the commit message): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression is caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered entries**—the
>   reasons in the non-obligation registry (`NON_OBLIGATION_FIELDS`) and red-list evidence.
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If such an anchor promise is added to a module header
>   in the future, it must either be covered by **manual review** or F1's scanning surface must be
>   expanded—only one of these two options is permitted; it must not be left unmanaged by default.
> - **The gate's static boundary**: set-level validation of delivery markers is static (criterion
>   3), while removal of an individual arm is covered by **runtime settlement**—the static + runtime
>   two-layer structure is the intended design, not a gap (t11's M11A/M11B/M11C mutation tests
>   verify both directions).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed):**
> of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) in accordance
> with RFC-039 **D20**. This note records the implemented shape, criteria, classification basis, and
> known boundaries; the definition lines are still reconciled by gate **F4** (the line numbers in
> the two rows above are current code facts).
>
> **I. Implemented shape (carrier choice A: `PlanId`)**
>
> The key is a **structural identity** carried by the AST node, rather than relying on the
> positional coincidence of “each side computing a span once”: `PlanId` (`src/util/plan_id.rs`, a
> monotonic `u32`, unique within a compilation unit, with `UNPLANNED = u32::MAX` marking synthetic
> nodes outside the parser) is issued by the **sole allocator**, `ParserState::alloc_plan_id`. It
> has two carriers: `Stmt.plan_id` (statement identity → ownership of the release plan and its
> consumption) and `Expr::Call.plan_id` (call-site identity → production and consumption of overload
> decisions). `Expr::call` is the repository's only call-expression constructor; synthetic calls
> outside the parser are always recorded as `UNPLANNED` (**not consumable**, see the second
> section).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry is `ReleasePlan::consume(stmt)`—**the read and the accounting occur in
>   the same call**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is provided by
>   `walk_stmt`'s set/restore operations + the CFG-node-to-statement-identity table; the Drop of a
>   scope exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one object**),
>   produced at `inference/expressions.rs:4550` (written by call-site `PlanId`; dispatch passes it
>   through from `Expr::Call { plan_id }`), consumed at `ir_gen.rs:7697` (with bindings) / `:7772`
>   (without bindings) through `OverloadResolutions::get` (a hit is recorded).
>
> **II. Consumption-required settlement (reusing the 4.3 ledger rather than creating a second
> one)**: the consumption ledger has a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each of the two tables. When
> `generate_module_ir` returns successfully, it marks both ledgers as having “completed one full
> pass” at the **same point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload
> decisions). The three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point connects one line for each of the two
> obligations—a nonempty set difference produces an **E-level W1090** (promotion of the same code to
> E; policy in the third section). The exemption criterion is attached to the **ledger's own
> completion marker** (not to whether `unit.ir` exists), so the Check form's consumption of the IR
> product does not constitute an exemption.
>
> **III. D59 classification basis (class C: red before green)**: the form
> `Type.method(positional argument, name = value)` previously **silently returned the wrong answer**
> (measured: `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the
> selected candidate was discarded, and execution fell back to the last binding in definition
> order). After the change, the same program produces the compile-time diagnostic
> `error [W1090] ... produced key was not consumed`. The fix takes the **loud** route rather than
> “put the resolved name into a call with named arguments”: the latter would lose the named argument
> and merely replace one form of silence with another (final semantics in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition, `Drop` instructions are no-ops in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty diff
> across 342 corpus entries is direct evidence of this.
>
> **IV. Criteria and evidence**: D41's same-form criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the set difference must be empty + an evidence
> floor**: the number of files that produced a decision `≥ 1`, the number of truly consumed entries
> `≥ 2`, with no allowlist). The nine pinned decision-form cases are in
> `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return / positional argument
> / binding in type body / `?` operand / named only / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation testing: shifting a consumption key by one position
> ⇒ the criterion turns red + a real program emits `error [W1090]` + the corpus gate reports a
> difference; restoring it turns green again. Gate figures: `check-obligations.py` PASS, corpus diff
> of 342 entries empty, `cargo test --lib` 2712 passed / 0 failed (zero IR-snapshot drift).
>
> **V. Known boundary (registered item, not an unimplemented promise)**: the **no-bindings
> consumption site** at `ir_gen.rs:7766-7790` is unreachable under the current corpus and test
> surface (method declarations uniformly go through `register_type_bindings` to the with-bindings
> consumption site), so the key correctness of this branch has **no observable red state**. The
> reason for retaining it is that it is the symmetric branch of the with-bindings path; deleting it
> would rewrite the “method call without `type_bindings`” form into silence. This is recorded by
> independent verification (t5 F-1) and review (t6 F2), and **no new task is opened**.

### 3. Unified Driver: a single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (in-project LSP, results filtered to the target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Must be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers cannot pass an arbitrary array.** This is the key constraint preventing the `Program`
abstraction from degenerating into a “free-form parameter that can accept anything”;
`test_program_stage_coverage` asserts it.

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
                // … 12 arms, not one may be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed):** four implementation deviations
> from the sketch above are semantically equivalent and recorded for reference:
>
> 1. The three-state `StageOutcome` in §4 is not returned by stage arms. It is expressed by `State`
>    failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are unchanged,
>    eliminating per-arm boilerplate returns).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” decision in §5 overrides the sketch's field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not externally emitted—the zero-diff criterion of S2, “deliberately fix no
>    bugs,” requires this; external emission follows with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module is relaxed to `pub(crate)`: the driver arm is
>    the only new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their original visibility after migrating in 4.2.
> 5. In the multi-file form, ProofExecution is an independent stage that runs after **all**
>    typechecking (A1, user ruling on 2026-10-09): the original `compile_project` interleaved proof
>    execution with the per-file typecheck loop (file N's proof ran before file N+1's typecheck).
>    For a single failure, both are byte-for-byte identical; in the multi-failure scenario “file 1
>    proof failure + file 2 typecheck failure,” the first reported error changes from the proof
>    error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries products through separate channels by ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    the entry points (`PipelineError` / `OrchestratorError`) are not part of the type surface the
>    Driver can unify.
> 7. The Check channel landed in 4.2.2: `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; a clean file has an empty Vec—the current
>    `check_project` contract). The Discovery product is extended to a triple—“file set + `used_by`
>    edge set + shadowing events”—stored in State (the MultiFile form has no downstream consumer, so
>    it is recorded but not externally emitted); the Check path now parses each file once rather
>    than twice, following the C3 topology.
> 8. The Lsp form landed in 4.2.3 (ruling by file source, user decision on 2026-10-09): parse
>    failure for a file on disk follows Check's option B (collect the diagnostic + remove the unit
>    from the compilation—both sides degrade consistently); parse failure for an edited buffer
>    retains the partial AST and continues typechecking (the editor philosophy—semantic
>    functionality should not disappear while the user is typing). The Discovery arm preserves
>    buffer source over stale on-disk content for Lsp. Typecheck checks only the target file under
>    Lsp (other units exist solely to provide signatures to the registry). The within-file
>    diagnostic-order normalization principle extends to LSP (typecheck diagnostics → W-code
>    warnings → proof errors). The old LSP behavior of hard-aborting on parse errors in unrelated
>    on-disk files and silently falling back to the single-file path in the handler is fixed by this
>    step.
> 9. The Embedded form landed in 4.2.4: `Program` gains a `shared_registry` field—the embedded std
>    module is a child compilation, and the registry is an **input**, not a product (EMBEDDED_STAGES
>    has no Registry stage). The #94 SymbolTable-sharing contract is thereby made explicit as a
>    program declaration rather than a requirement that “the caller remembers to pass the same
>    registry.” Error-path text is normalized from `<std.test> (embedded std)` to the unit virtual
>    path `<std/test>` (compiler-internal error surface only; no test pins it). At this point, all
>    four orchestrator entry points have moved into the Driver.
> 10. Standalone check is unified (4.2.5, ruling A + IR-stage ruling, 2026-10-09): the Check form
>     faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive errors only; missing a warning
>     is preferable to a false warning). Relative `use` paths are resolved relative to the
>     importer's directory (aligned with rustc's single-file mod behavior). The CHECK stage table
>     adds GlobalSlotAlloc + IrGeneration: the old standalone path (the full pipeline) already ran
>     IR generation, and errors such as E3019/E1014/E1015 are produced only by ir_gen (verified by
>     the runner gate). The Check form of IrGeneration performs checking without consuming the IR;
>     the Script/Bin distinction is represented by `module_key` being None/Some (the existing E3023
>     switch, ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces only
>     resource-limit/internal errors, with no corpus dependency). **〔Withdrawn on 2026-10-10
>     following empirical verification in 3.4.8〕**—the claim of “zero corpus dependency” was based
>     on the evidence blind spot that the corpus contains no pathological-recursion fixture; E3005
>     is the only compile-time defense for this class of programs. See note #14. Incidental fix for
>     a latent defect: the Registry arm's repeated harvesting of registrations for embedded std
>     units displaced the native half already merged by `with_std()` (false E1043)—embedded units
>     now skip duplicate registration.
> 11. The LSP single-file fallback is unified (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is removed and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     The SingleFile+CollectAll form collects all parse errors, retains the partial AST, and
>     continues typechecking (the editor philosophy extends from the 4.2.3 ruling; the stage is not
>     marked failed, because topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now gains proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and IR-level errors;
>     lexical failures now report real diagnostics (the old synthetic “E0001 lexical error” text
>     disappears with the removed sequence—the real diagnostics carry precise spans, with no
>     information loss).
> 12. `check_module` / `check_module_collect_all` are verified line by line (4.2.7, mandatory C5
>     item): the two entry points had already converged separately—at the mod.rs layer,
>     `check_module_inner(ast, env, collect_all)` uses a single Boolean; at the checker layer,
>     `check_module_impl(module, collect_all)` does the same. The only fork is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`; both pass-3 and drain modes run,
>     and the difference is carried by whether `collected_errors` is empty; there are five
>     collection points in statements.rs (function body/use/for/block/while). Verification exposed a
>     duplicate-diagnostic defect, which was fixed: collection points put the first error into both
>     `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err` produced
>     the same code/span/message twice (or three times when nested). Two independent mechanisms were
>     also found: duplicate validation of annotation parameters and the whole annotation
>     (E1003/E1103), and duplicate emission at the same ownership-layer point (E2014/E2018). The fix
>     is at the module-result boundary: deduplication by `(code, span, message)`—diagnostics are
>     positional facts, and duplicates carry no information. Duplicate copies were removed from 63
>     corpus entries, with zero drift in run lists and exit codes (the apparent “duplicate” for
>     `borrow_conflict_err` is an artifact of the baseline tuple format not containing a column
>     number—the two E2018 entries are legal diagnostics in different columns).
> 13. The wasm playground lands in 4.2.8: `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`; the `Compiler`
>     wrapper layer no longer participates. Error text is aligned byte for byte with the old
>     `CompileError` Display prefix (`Parse error:`/`Type error:`/`Internal error:`), with no
>     visible difference in the playground UI. WasmPlayground shares SINGLE_FILE_STAGES + FailFast
>     with SingleFile (the 4.1 declaration now has a real production caller).
> 14. Multi-file monomorphization and the CHECK stage table are corrected (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing the mono exclusion in note #10**). Empirical evidence
>     disproves the claim that “mono produces only resource-limit noise”: under the multi-file path,
>     pathological generic recursion (`f(x)=f([x])`) causes the compiler process to overflow its
>     stack (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime error), and the
>     mono depth gate is the only compile-time defense. Excluding mono from standalone check before
>     4.2.5 caused a silent coverage regression (“zero corpus dependency” merely proves that the
>     corpus contains no pathological fixture). The “type-erasure fallback” assumption was
>     invalidated by the RFC-033 reflection ruling—`^^List(Int)` requires the real identity of an
>     instantiation, and monomorphization is the foundation of reflection, not an optional
>     optimization. Four design points: ① typecheck produces an instantiation_request for a
>     qualified call (`lib.f(x)`, FieldAccess form), with generic_id using the qualified name, in
>     the same namespace as the merged IR (currently only bare Var is recognized,
>     expressions.rs:2455); ② the `Stage::ALL` topology is changed so Linking precedes
>     Monomorphization, and MULTI_FILE gains Monomorphization at the end—it consumes merged_ir and
>     aggregates requests from all units (qualifying the deferred bucket's containing_fn), elevating
>     mono from a per-module pass to a whole-program pass. The single-file stage table is unchanged
>     (no Linking; the subsequence property is preserved, with zero IR-snapshot drift); ③ CHECK
>     gains Linking (pure merge—E3020 entry validation remains in RoleClassification to prevent
>     duplicate reporting) and Monomorphization (pure checking without consumption, like the Check
>     form of IrGeneration); ④ resource protection covers cross-unit mutual recursion through
>     whole-program BFS. The `Monomorphizer` itself is unchanged (its ModuleIR + request-set
>     contract already matches the merged-IR form).
> 15. 3.4.8 R1 (①) lands (2026-10-10, a9438007). Qualified-call instantiation collection is
>     implemented according to the #14 design: `ExpressionInferrer` receives the module-qualified
>     key table (three `use` arms register it—ordinary / alias / item-by-item alias, with only
>     SubModule exports eligible), and `callee_generic_name` is assembled by `SymbolTable::qualify`
>     using the same qualified-name source as the merged IR; the qualified-call first path tightens
>     the arity criterion (when the parameter table cannot be resolved, it takes the second path and
>     uses the signature to obtain actual arguments, avoiding false E3018 reports caused by
>     mismatched same-name functions across modules). Three asymmetric defects on the mono side were
>     empirically exposed and fixed in the same round: (1) the deletion key (a set of generic names)
>     and rewrite key (containing_fn + name + span triple) differ in granularity—a site whose actual
>     arguments cannot be resolved degrades to a symbol request in the deferred bucket, and when the
>     container function is non-generic the bucket never drains, while the original item has already
>     been deleted by name ⇒ dangling call (demonstrated by the runtime E6006 in `list_ops.yx:53`).
>     The fix restores the gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as the single source of truth to determine coverage and retaining
>     originals for uncovered sites. (2) Nondeterministic function-table order caused by `HashSet`
>     iteration order in the restoration phase (t2 review F1) ⇒ changed to `Vec`, sorted by name,
>     making the product byte-for-byte reproducible. (3) TailCall / MakeClosure, which are not
>     rewrite surfaces, still looked up mapping keys unchanged (despite the comment claiming
>     conservatism) ⇒ `call_form_is_rewritten` provides a single decision point, and all non-rewrite
>     forms are counted as uncovered. Each has a pinned test (with red-state evidence demonstrating
>     discriminating power before the fix). The corpus-diff baseline was regenerated in the same
>     commit, producing exactly one line (a `remove_at` frame gains `(int64)`). **Commit-strategy
>     note**: when typecheck collection and the mono fix were split into two commits, the staging
>     tree containing only the former triggered a red `stdlib_docs` job for an existing HEAD defect
>     (`list.is_empty([])`, the same E6006 family); the two halves are each other's green-light
>     prerequisite, so they were combined into one atomic commit.
> 16. Five rulings registered on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—user-module
>     exports carry `generic_fn_type_params_snapshot()` through `Export.type_params`, structurally
>     identical to the std path (`yx_sources.rs:62/108`): the declaration name becomes the
>     authoritative source for qualified-call monomorphization, and signature-based reverse
>     inference is demoted to a fallback (incorporated into R2's scope). (2) **HKT is live on the
>     declaration side but broken on the call side** (inference passes type arguments E1002 /
>     explicit type arguments E1010), registered as WBS 3.5 (three third-level tasks). (3) **The
>     deferred bucket never drains in a non-generic container** is incorporated into acceptance
>     subcriterion 3.4.8 ②: the bucket must drain or fail loudly; it must not silently remain
>     unspecialized. (4) **Calling a closure variable by name causes runtime E6006** is registered
>     as WBS 3.6; the semantics (i) support indirect calls / (ii) reject at compile time, and **(i)
>     has been chosen**—support for indirect calls: the `CallDyn` mechanism already exists (the
>     closure-expression path at `ir_gen.rs:7945`); the break is that a `Var` callee is degraded to
>     a name-based `Call`, even though typecheck already accepts the program. (5) The **decision
>     criterion is fixed**—a rejected option may be rejected only on correctness and readability
>     grounds; “large change surface / introduces a new mechanism / high cost” does not constitute a
>     rejection reason, and is recorded in the second part of `coding-rules.md` and the HOWTO
>     self-check table. Decisions requiring a user must follow all four items in `AGENTS.md` “When
>     requesting a user decision (mandatory)” (situation + evidence / item-by-item pros and cons /
>     explicit recommendation + cost / explicit breaking-change identification).
> 17. 3.4.8 ② lands and the F1 root cause is identified (2026-10-11, 375789d2 + d922a551). ② is
>     implemented according to the #14 design: `Stage::ALL` is changed so Linking precedes
>     Monomorphization, and the MULTI_FILE table gains mono at the end. The Linking arm aggregates
>     whole-program requests and qualifies `containing_fn` **before taking each unit's IR**
>     (otherwise the deferred-bucket keys differ from the merged IR's keys, and the bucket never
>     drains). **① is closed (D2 ruling A)**: user-module exports carry a snapshot of the
>     declaration-time type parameters through `Export::type_params`, structurally identical to the
>     std path (`yx_sources.rs:62/108`). **Existing defects exposed and fixed during the work**: (a)
>     **F1**—`qualify_names` rewrites only function bodies and never accesses `ir.init`, so under
>     multi-file compilation function definitions were qualified while module-level calls remained
>     bare names ⇒ runtime E6006 (**reproduces even for non-generic code**, independently of
>     generics). The fix makes init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono side's “whole-program unique suffix” heuristic and its
>     silent `continue` on a miss ⇒ the heuristic is removed, and a miss now produces a **loud
>     E3018** (silent ambiguity mismatch is worse than failing to resolve). **An independently
>     performed verification caught and fixed a regression**: t6 found that a named import
>     (`use lib.{pick}`) followed by a bare-name call was falsely rejected with E3018 (the IR
>     instruction name is the alias's qualified name, while the request name is bare, and the
>     aggregation side qualified only by unit key) ⇒ the aggregation side now **calls ir_gen's**
>     `import_aliases_from` and `resolve_call_target` directly (extracted as a public single point,
>     one producer and one consumer), with a matrix of pinned tests covering both forms of named
>     import. **Record**: the blank `DefId` in the init section was empirically shown to be a
>     “consistency blank” rather than a live defect (the codegen name fallback resolves it, and all
>     four kinds of init call were tested successfully), and is registered in WBS 3.7.4 to close the
>     “function bodies use def, init relies on names” policy divergence.
> 18. The init-section DefId policy divergence is closed (3.7.4, 2026-10-11, 38bd4b9e):
>     `assign_defs` is refactored to extract the single point `resolve_static_targets`
>     (Call/TailCall string operands + MakeClosure), and function bodies and `ir.init` share both
>     the same implementation and the same `resolve` closure; after 208 IR snapshots are
>     regenerated, only 309 `(def=_)`→`(def=dN)` pairs change (other-lines 0; zero residual
>     `(def=_)` entries in the init sections across the tree). The codegen DefId primary path and
>     the name-fallback path yield identical translation results and execution values for all four
>     kinds of init call; the corpus baseline has zero drift. The probe flips to the new policy and
>     is strengthened (identity + exact index + execution value; removing init coverage turns it
>     red). **Erratum (2026-10-11)**: the commit IDs above were rewritten by `git pull --rebase`—the
>     original b554497b/e60dcc87/b7ac7396 correspond to 375789d2/d922a551/38bd4b9e after the rebase.
>     The pushed dev history uses the new IDs (the old IDs were local temporary IDs and never
>     entered remote history).
> 19. 3.4.8 ③④ land and are verified (2026-10-11, 8ecf56f93). (The design basis for ③④ is in #14; ②
>     is implemented in #17, and this item closes it.) **Implementation**: the end of `CHECK_STAGES`
>     is `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs only a pure merge
>     (aggregating requests + `link_module_irs`), and the entire entry-validation block remains in
>     `RoleClassification`; after `ir_generation_check` generates the IR, `us.ir` is retained
>     (memory only); mono failure enters the collection channel under Check, while MultiFile retains
>     `fail_with`; `into_outcome` first calls `take_check_diagnostics` and then checks for failure
>     (structural failures no longer clear already-collected diagnostics). **Verification**: four
>     rounds of independent verification (t3/t4/t10 + a t5 cross-form matrix) and one review (t6)
>     all pass; the corpus diff is 340/340 line-for-line IDENTICAL, IR snapshots have zero drift,
>     and the test count rises from 2643 to 2656. **Existing gaps discovered and fixed in the same
>     round**: F1 (in a rootless multi-file check, request names and IR definition keys do not share
>     a source ⇒ silent failure to specialize, as recorded in the WBS 3.4.8 row's implementation
>     note)—the aggregation side is fixed by an exact reverse lookup against the real definition
>     keys (the first version used `flat_defs`); **same-day erratum**: that bridge was later removed
>     after the regression fix, and the root cause is instead resolved by decoupling the dual role
>     of `module_key` (`script_mode`), with all three sources using `{unit key}.{name}`. The
>     regression (CI documentation-example gate: 258 blocks, 10 failures → 0 failures) is located
>     and fixed in the WBS 3.4.8 row's implementation note. **Record (existing, not a regression
>     from this round)**: the E3005 help text does not match the resource-protection semantics; the
>     multi-file E3005 span label is misplaced (reproducible with the HEAD binary); the
>     type-parameter position dependency on the request-collection side remains (assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 land and the diagnostic surface is closed (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is reachable)**:
>     interpreted calls are changed to **explicitly use a stack-driven
>     mechanism**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push a frame and then
>     return control to the outer `step_one`; the frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, and on return it writes the result back to the
>     caller's slot. Multiple calls in one instruction continue through `pending_calls` within the
>     frame. Baseline measurement: each interpreted call level consumed 2544 B of native stack, with
>     the crash threshold at `count_down(394)` on the 1 MiB Windows main thread. After the fix,
>     20,000 levels of recursion run successfully on a 512 KiB stack thread (approximately 0 B per
>     level). The guard (`push_frame`) is therefore reachable from every entry point and every
>     `max_stack_depth` setting; `ExecutorError::StackOverflow` now carries the real limit
>     (previously `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (truncating
>     deep-stack echo) was merged into this task by user ruling** (same location in the same file
>     and inseparable at file granularity): the echo now shows the first 16 frames + an
>     omitted-frame count + the last 8 frames; shallow-stack output remains unchanged frame by
>     frame. **3.10.1 (no compile-time gate for native recursion, #447)**: locate before
>     changing—the crash point is recursion in `MonoType::type_name` (**328 B/level**, SP probe +
>     precise per-operation threshold), with another derived `Clone` on the same path (~400
>     B/level), a deep copy in `substitute`, `type_name_hash` (164 B/level), and
>     `collect_generic_type_refs`. After changing all four to work lists and removing one
>     whole-request deep copy per level, pathological generic recursion with `mono.max_depth=5000`
>     produces **E3005 (with the real limit)** and no longer overflows the stack. **3.11.1 (preserve
>     task-boundary error codes, #448)**: the task result's error side changes from a value to a
>     **structured value** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), restored by value across the boundary;
>     E6001/E6003/E6004/E6008 within tasks are word-for-word identical in Standard and Embedded
>     modes. Two **pub API payload-shape changes** include `BREAKING CHANGE` in the commit.
>     **Commit-strategy note**: 3.9.1 and 3.11.1 are **inseparable at file
>     granularity**—`interpreter/executor/executor.rs` carries both the trampoline and the
>     error-channel producer/consumer, while `TaskFailure` is defined in `runtime/engine.rs`;
>     staging only one side does not compile (the pre-commit partial-staging compilation rejected it
>     twice), so—under the same policy as #15—the two are combined into **one atomic commit**.
>     **Known boundaries and follow-ups**: the derived Drop implementation (~75 B/level) means
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant-evaluation) gate is carried by
>     native recursion (3,696 B/level, with the default limit of 256 levels ≈ 924 KiB), currently
>     has zero call sites and is unreachable in production, and future wiring must close the same
>     family. The Standard-mode task interpreter is constructed with the default `ExecutorConfig` ⇒
>     its E6004 limit is always 1024 (configuration propagation registered as WBS 3.12, user ruling
>     B: a separate team). The test-runtime budget is moved to issue #449 (a pathological fixture
>     takes 76 s, clippy has two targets, and commits run the full suite); it is outside this RFC's
>     implementation scope. **Corpus directory stratification (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134), totaling 212 fixtures, are split into 23
>     category-based subdirectories; 88 IR snapshots move to mirrored paths. The corpus baseline is
>     regenerated in the same commit (342 lines, independently recomputed line by line: only the
>     `file` field and two new changes), and 41 snapshots are regenerated because their header paths
>     become longer; line-by-line review shows **only span-offset changes** (zero differences after
>     offset normalization).
> 21. The LSP semantic-data pipeline is unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references carry `resolves_to`
>     pointing to the real definition. The old form triggered **two separate compilations** on
>     didOpen/didChange: diagnostics went through the unified Driver pipeline, while semantic data
>     went through server.rs's manual lex→parse→single-file typecheck (with no module registry,
>     project-local `use` resolution failed, no reference events were produced, and cross-file
>     navigation/rename/reference lookup silently failed—the silent failure registered as D55 is
>     closed by this task). **Implementation**: (1) `Export` carries `definition_span` (harvested
>     from AST identifier nodes by `extract_module_info`, sharing the same source and value as the
>     typecheck definition event's `def_span`; native std always has None), and `ModuleInfo` carries
>     `definition_file` (filled by the Registry arm from DiscoveredFile); (2) when the checker
>     drains reference events, `import_def_locations` redirects cross-file import items'
>     `resolves_to` from the use line to the real location in the definition file (std imports have
>     no registration and continue to resolve to the use line); (3) the checker's module identity
>     changes from a uniform `"<module>"` to the unit file path (this value is consumed only by the
>     semantic-data surface and has zero diagnostic consumers—verified across all four
>     `env.module_name` consumers); (4) `DriverOutcome` gains a `semantic_db` channel—populated by
>     CollectAll forms (Lsp target file regardless of diagnostic success + SingleFile fallback); the
>     Lsp arm injects the top-level export definition surface of non-target files from the registry
>     at its end (`external_definitions`, enabling navigation even for unopened files;
>     type/submodule exports are not injected because typecheck-side type registration does not
>     produce definition events, preserving a consistent absence on both sides); (5) orchestrator
>     gains `analyze_source_in_project` (`check_source_in_project` delegates to it and takes only
>     the diagnostic half); (6) the LSP-side `analysis.rs` single-compilation entry point replaces
>     the manual pipeline (`server.rs`'s `update_semantic_db` is removed), and the World key is
>     unified as a normalized file URI (`uri.rs`: `path_to_uri(uri_to_path(u))` is idempotently
>     normalized; drive-letter case and `%XX` encoding differences normalize to the same key).
>     **Incidental fix (D59 routing class B—old behavior silently did nothing and did not change any
>     externally visible behavior of previously successful programs)**: cross-file
>     navigation/rename/reference lookup in project files no longer silently fails and now resolves
>     to the real definition; on lexical errors, the old manual pipeline's deletion of semantic
>     information is replaced by retaining semantic information over the partial AST (the 4.2.6
>     editor-philosophy extension); when the target file has type errors, semantic data is no longer
>     discarded along with `type_result` (coloring/navigation does not disappear while the user is
>     typing). **Pins**: six driver-level tests (target-file path key / cross-file resolves_to +
>     external-definition-surface round trip / std-import use-line fallback / error-file retention /
>     single-file fallback path key + error retention) and one LSP e2e test (red before green—the
>     test returned None before the fix); four URI-normalization tests.

**Migration path for the ten entry functions (each function specified individually):**

| Entry point                                                | After migration                                                                                                                                                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to the Driver                                                                                                                      |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                         |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                           |
| `check_source_in_project` (`orchestrator.rs:450`)          | Replace with one Driver call + filter the result to the target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Replace with `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                         |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                                  |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project path uses `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                               |

**`check_project` and `compile_project` are about 40% shared / 60% divergent.** What they share:
vendor-consistency checking, file discovery, `build_registry_from`, `all_method_bindings`, the
parsing loop, per-file checker setup, and result processing. What diverges: items
2/3/4/5/7/8/9/10/11 in the 11-item table above. **This shared 40% skeleton is exactly the Driver's
value range**—the divergent 60% consists entirely of “different stage sets” or “different
aggregation modes,” precisely the two things that the `Program` fields can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage has non-blocking problems; continue
}
```

**Key design decision: “an upstream failure caused this stage to be skipped” is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If every stage returned
`Skipped` on its own, the information about “why did I not run?” would be scattered across 12 stages
and could not be centrally audited. Instead, `dispatch` determines this at the start of the loop
according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect this document is
intended to fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”
that produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every “Y did not run because X was not done” must be explainable.

Diagnostic text must distinguish three skip reasons:

| Reason                                                              | Text direction                                                                                                              |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                       | “Stage Y was not executed because upstream stage X failed”                                                                  |
| Caused by an empty prerequisite obligation                          | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; not counted as a warning** |
| Preconditions not met (for example, `config.mono.enabled == false`) | “Stage Y was not executed because it was disabled by configuration”                                                         |

The second category is critical: it distinguishes “normal skip” from “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract
depends on that count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                               | Current equivalent |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (four early returns at `pipeline.rs:150/162/174/193`)     | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
project-local and single-file files within the same process, while the CLI's `run` and `check` are
two independent calls. Placing it on Program avoids having the Driver hold mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether to
return early”: they are two different checker entry points. After unification, a single
implementation should be driven by the `Aggregation` parameter rather than retaining two functions
(**this is an additional item that this document must verify; see “Known Limitations and Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None.** It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types are unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                              |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the type definitions. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain in
their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers (for example, `ir_gen` reading `release_plan`) continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
driver non-reversal red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not `use crate::driver` in
reverse**. A `Driver` appearing in a `TypeChecker` import is considered a violation and is blocked
by the `scripts/ci/check-module-boundary.py` gate proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single-file) | Five stages                              | Same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone programs—it now uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven by the rootless path. This row now describes only the `lib::run_file` / `Compiler::compile` library paths)                                                                                                                                                                      |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” with Check / Lsp / MultiFile sharing one implementation and no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for external emission by that arm) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| LSP (in-project)                    | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

**The only intentional breaking behavior** is replacing the `.expect()` at `predicate.rs:35` with a
returned diagnostic. This is consistent with RFC-027 §8's positioning of the compiler as “not bound
to a specific solver” and with the contract already declared at `backend.rs:60` (“callers should
degrade conservently”)—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (one for single-file / N for multi-file)                       |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                   |
| `src/lib.rs`                                       | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                           |
| `src/lib.rs`                                       | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `141-227`                                      | Replace the `Pipeline::run` body with `Program` construction + a Driver call                                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `275-278`                                      | Move dead-code analysis out; use the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                         | `306-311`                                      | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                         | `381-389`                                      | Move monomorphization out; use the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `450`                                          | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                      | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                        |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics`; call the Driver                                                                                                                                                                                                                                                                                                  |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                      | Unify `check_files_with_diagnostics` on `Program { kind: Check }`                                                                                                                                                                                                                                                                                                       |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                          |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently reaching path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “lower-layer failure prevents upper-layer execution” (there is no short-circuit)                                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                        | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure-based access point; change the three consumer sites in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared access point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 governs the wording |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`checker/refinement.rs`'s `revalidate_refined` / `check_call_arg_refinements` /
`check_return_refinement` and the ownership-result-handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is on the consumer side, not the production side**, and changing the production side
would conceal the architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch
(wasm-branch reachability verdicts belong to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Handling                                                                                                                                                           |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches, with the expected diagnostic-set change recorded in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that compiled successfully now produce warnings     | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                     |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                     | Move from W to E in two steps, S2/S4                                                                                                                               |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no longer crashes                                         | Non-breaking                                                                                                                                                       |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not pass through proof_execution                                                                                                                |
| Migrate `TypeCheckResult` fields into `Obligations`              | **Internal refactor**                                                      | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                    |

---

## Implementation Highlights

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contracts and unified
Driver)** in RFC-039's global stage sequence. S1–S5 below are the implementation order within this
document. **Each step's acceptance criteria reference category C2 (orchestration changes: identical
diagnostic sets across entry points + identical corpus behavior) in the
[equivalence-criteria document](07-equivalence-oracle.md).**

### S1: Establish the Criteria (Red First)

**A prerequisite that cannot be skipped.** The defect criteria must first be written so that they
fail.

| Deliverable                                                                                                                                                    | Acceptance                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red.** If it unexpectedly passes, the defect analysis in this document must be re-reviewed              |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                             |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                                 | Marked `#[ignore]`; turned green in S4 (red-state record: removing the sole settlement point produces `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` Without Changing Behavior

| Deliverable                                              | Acceptance (C2)                                                                                  |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array  |
| Change `pipeline.rs:141-227` to call the Driver          | **The single-file path's diagnostic set and exit code are byte-for-byte identical**              |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver.
Acceptance is at the C1/C2 zero-diff level.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Merge the Orchestrator's Four Entry Points + Fix the Defects

| Deliverable                                                                                                                       | Acceptance (C2)                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Change all of `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` to Program constructors | The four entry points' diagnostic sets are normalized according to `Aggregation`         |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                          | `yaoxiang check` behaves consistently inside and outside projects                        |
| Delete the LSP manual stage sequence                                                                                              | LSP and CLI produce the same diagnostic set for the same file                            |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                     | The defect is fixed                                                                      |
| Fix the empty match arm at `checker.rs:1313`                                                                                      | `test_no_silent_pass_on_unproven`: `Unproven` always produces a diagnostic in every mode |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                        | Compilation no longer panics when Z3 is absent                                           |

**Rollback point**: the defect fixes and the structural merge must be **two separate commits**. If
the structural merge causes problems, only the structural commit can be rolled back while retaining
the defect-fix commit; at that point `test_multifile_proof_obligation_not_dropped` remains green.
The reverse—retaining the structure while rolling back the fix—turns it red and is an unacceptable
intermediate state that must not be merged.

### S4: Enable the Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                                            |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 test cases in `src/driver/tests/obligations.rs` turn green (implemented form in the §2 revision note; the count in that file is authoritative) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                          |
| Promote obligation diagnostics from W → E    | After a full-corpus diff, manually review every new E                                                                                                 |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration option;
switching between W and E does not require structural changes.

### S5: Finish the Proof Layer and wasm

| Deliverable                                                                                                                                                                                                                                         | Acceptance                                                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                    | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                                   |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes sharing between cargo test threads) | Production-path calls to `default_solver()` fall to zero (tests only); cache-hit rate across the three consumer points is observable (the counter already landed in 89576fafd) |
| Add a warning to the `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                                          | A diagnostic is produced when there is no `body_checker`                                                                                                                       |
| Reachability verdict for the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                      | The conclusion is delivered to the “F2 verdict” section of `06-cleanup-inventory.md`; this document records only the need for a verdict                                        |

**Caution**: correcting the layer order will change the diagnostic set and may expose a large number
of previously silent `Unproven` results. **This item should proceed independently of S1–S4** and
must not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustive enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                            | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant makes `dispatch` fail to compile. This is structurally identical to the opcode-table mechanism already validated in RFC-039 routing table B. `StageScope` further turns “per-module or project-level” into a type-level fact, eliminating the need to reason about invocation counts                                                                                                            |
| **Obligation mechanism** | `Obligations` + `assert_drained()` perform **runtime** settlement, used together with the **static** gate `scripts/ci/check-obligations.py`; **final severity is E level** (2026-10-13 user ruling A: promote the same code to E, code unchanged; **implemented form in the §2 revision note**: the ledger does not copy the field structure; the red list—empty after closure—retains the mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not a single bug: at least two instances of the same class have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. The W→E two-step approach keeps every step's C2 criteria usable—now that the inventory has been closed at 13 entries and the red list is empty, and the healthy state emits nothing, the final state directly uses E (see sections VII and VIII of the §2 revision note) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer); do **not** change the production-side logic of the `Unproven` branches (now three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                         | The problem is on the consumer side, not the production side. Changing the production side would disguise the architectural defect as “the logic has been fixed,” even though the logic of those branches is itself correct                                                                                                                                                                                                                                                   |

This design additionally eliminates two classes of cracks:

- **The crack between “comment promises” and “code behavior.”** The “silent pass must not be allowed
  to return” promise at `checker/refinement.rs` (originally `checker.rs:5165`) is a **comment-level
  contract**, fulfilled by `assert_drained()`.
- **The crack of “an array that someone forgot to append to.”** The current implementation has five
  functions each manually writing a call sequence. After unification, there is one stage table, and
  a missing stage fails at compile time.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, and IR
  generation/linking) **do not involve type continuity**; they are configuration questions of
  “should this analysis run?” This approach also cannot express `Aggregation`; it merely moves the
  `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer makes `dispatch` fail to compile, replacing
  “five functions each manually writing a call sequence” with “an array someone forgot to append
  to,” which is precisely the cause of the current 11 inconsistencies. This repository already
  contains a counterexample of the same form: the `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader` described under `docs/src/dev/design/check/` have zero
  implementation.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields; `Stage` remains scattered across five functions, and the next
  divergence will continue to grow from there. **It must be done first** (as part of S1/S3), because
  the structural refactor needs a known red test to prove that the criteria work.
- **Make `proof_calls` public and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold.
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must occur in the
  orchestration layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactor that “centralizes
  wiring.”
- **S3 significantly changes the diagnostic set, and the direction of change is “exposing previously
  silent problems.”** After the fix, users may report a batch of “new errors”—these are real bugs
  that were never reported before. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires field-by-field human judgment.** Some
  obligation fields, such as `module_namespaces`, may legitimately have “no consumer” by design and
  should not trigger a warning. S4 must review every field individually rather than applying one
  policy to all of them.
- **The `Program` abstraction may be premature.** If the stage sets of some entry points remain
  unstable for a long time, `stages()` may degenerate into a free-form parameter that accepts a
  different array on every call, and the contract constraint will fail. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can only come from the six predefined
  combinations.**
- **Dependency on criteria**: the defect-fix criteria for S1/S3 depend on the defect tests in the
  [equivalence-criteria document](07-equivalence-oracle.md) being established first and turning red.
  **If that document has not first established a multi-file corpus layer, S1 cannot be
  accepted**—because all 293 existing corpus files take the single-file path and provide zero
  coverage for this class of defect. This document does not cover the IR verifier (`verify_loose`);
  that prerequisite work is outside its scope.
- **Correcting the layer order will expose previously silent `Unproven` results** (`equivalence` is
  not in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse
  of the declared order). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after making the SMT backend a singleton is undecided.** `backend.rs:13-19`
  already states that `Solver` is only `Send`, not `Sync`, and `Z3Backend`'s cache is a `RefCell`;
  after changing to a singleton shared across compilation units, all access paths must be confirmed
  to pass through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependent defects**, such as the actual order
  dependency between termination and ownership. It should be considered only after the equivalence
  criteria have stabilized.

> **All open questions originally listed in this section have been decided.** See
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **This document retains no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (implementation steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three criterion layers, the
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker/refinement.rs` — Explicit declaration that “silent pass must
  not be allowed to return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/typecheck/layers/README.md` — Layer-order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for hard-failing SMT
  initialization
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration that “callers should
  degrade conservably” (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream code must
  consume (after 3.4.4/3.4.5/3.4.9 ledger closure)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's own
  admission that it “stops before the pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus's own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground's
  fourth compilation caller (the single-file path, so proof_execution executes)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Compile-time error-code gate and an example of this project's enforcement
  mechanism
