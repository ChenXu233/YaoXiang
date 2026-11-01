# Compilation-Stage Contract and Obligation Ledger

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, acceptance-criteria tiers, and execution-stage ordering are covered in the
> main body of RFC-039; see the [index for this directory](index.md) for the positioning of each
> companion document.

## Positioning and Scope

This document is the construction blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“Which compilation stages ran, which did not, and why?” is currently distributed
across six paths and ten entry functions, each manually wired, with no single place able to answer
the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself were broken by the architecture, not by the logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` states (originally at
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compilation error, no degradation, no silent
pass……silent pass must not be resurrected.” However, this invariant holds only on the **single-file
path**: a proof-function constraint such as `x: Sorted(3)` **passes silently** on the multi-file
`yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed, and no
diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustively enumerable, with no runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- A unified `Driver` (a single `dispatch`) and the individual refactoring approach for each of the
  ten entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by these changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency-direction rules, and anti-bounce gates → `01-routing.md`
- Equivalence criteria (C1–C6 tiers, three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- Global execution order for P1–P10 and G1–G10 acceptance gates → main body of RFC-039

### Division of Labor with RFC-039

RFC-039 explains **why** the restructuring is needed and **in what order** to perform it; this
document defines L1's **concrete shape**, **per-file change list**, and **implementation stages
within this document** (corresponding to P3 “Fix correctness defects” and P4 “Stage contract and
unified Driver” in RFC-039's global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

Equivalence criteria are applied under the **C2 (orchestration changes)** category of the
[equivalence-criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> This section consists entirely of **verified facts**, each with a file path and line number. Line
> numbers are based on `9e02e4db`.

### Stage boundaries are the only structural defect where “one wrong connection silences the entire repository”

This differs in nature from the other two classes of defects (module boundaries and test wiring):

| Defect                  | Typical manifestation               | Is there a signal?       |
| ----------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntax error    | Source code is wrong                | Yes, a diagnostic        |
| Type mismatch           | Type is wrong                       | Yes, a diagnostic        |
| Stage omitted           | Some stage is never called          | **No signal whatsoever** |
| Obligation not consumed | A field is populated but never read | **No signal whatsoever** |

The last two share a common characteristic: **failure does not produce an error**. They therefore
cannot be solved by “writing the code more carefully” or “reviewing more strictly”—code review can
show what was written, but not **what was not written**. This is precisely the root cause identified
by RFC-039: “This project treats ‘design’ as a documentation convention rather than an executable
constraint.”

### Enforcement mechanisms already present in this repository

The same repository already has mature enforcement mechanisms; they simply have not been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** through `tools/code-tables` in `build.rs:19-55`. Every code is compared
  against the RFC-013 code table; any mismatch directly triggers `panic!` and rejects the build.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines—47.9%—are under `tests/`) has
  high test density, and every module's test subtree is explicitly wired. This is the most complete
  test wiring in the repository and can serve as a formal reference for stage-layer gates.

**The design capability is sufficient. What is missing is “an equivalent gate at the orchestration
layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path used                                                              |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct invocation of 5 stages                                                      |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multiple files**     | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c in a project; otherwise manually lex→parse                                 |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages on the single-file path (`pipeline.rs`):

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`         | `run_ir_generation`, with **monomorphization embedded in it** (`381-389`) |

### 11 stage-coverage inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point “does not do this”; it means that “no code in this entry point does this”**—which is precisely
the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                          | `check_project`                                                | `check_source_in_project` (LSP)    | `compile_embedded_module`               |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------- | --------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                         | **No**                             | **No**                                  |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, no configuration gate)              | **No**                             | **No**                                  |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                  | **No**                             | **No**                                  |
| 4   | W1003 unused import                                      | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                            | Yes `350`                                                      | **No**                             | **No**                                  |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                | **No**                             | **No**                                  |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | Not applicable                                                 | Not applicable                     | **No**                                  |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                 |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                   | `discover` (`454`)                 | Not applicable                          |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                | **No**                             | **No**                                  |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                        | **No**                                                         | **No**                             | **No**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                         | **No**                             | Yes (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Differences in the current state
> after P3 lands are as follows:
>
> - **Row 1 has been fixed**: all five entry points share the same `frontend/proof_execution.rs`
>   implementation (pipeline + four orchestrator entry points), so the sole-consumer defect no
>   longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and dead-code-family warnings, and the W-code sets of the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) are normalized—the
>   single source of truth lies in the shared implementation of the `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk is an unproven defect).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to the “#434 ruling” previously had no documentation registration—now
>   registered as RFC-039 **D57**.
> - **Asymmetry in embedded-std inclusion (new fact outside the table, added on 2026-10-09)**: the
>   single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch), whereas multi-file `discover` previously recognized only
>   explicit `use` statements—project-level for loops compiled successfully and failed at runtime
>   with E6006 (confirmed by a probe). This was fixed and WBS 4.10.2 was closed. The related
>   “multi-file parse hard-abort quirk missing from the single-file path” was registered as WBS
>   4.10.1 and **has also been fixed** (landed immediately after 4.2.2; the Check path was degraded
>   to per-file collection; this audit method examines the dimensions of “stage coverage and field
>   consumption,” so this is a compilation-unit membership difference and belongs to the previously
>   unobserved category).
>
> - **Embedded-std registration-surface coverage (construction finding in 4.2.5, fixed)**: the
>   Registry arm repeatedly harvested registrations for embedded std units with
>   `extract_module_info`, which replaced the native half registered through `with_std()`
>   (`result.is_err`, etc. were lost → std.test falsely reported E1043). This exposed a third
>   previously unobserved audit dimension: **the merge semantics of two registration sources under
>   the same module key**. The registration table on the single-file path is formed only once
>   through `with_std()`, whereas the Registry arm on the multi-file/Check paths inserts per unit
>   and therefore has broader coverage.

**Two points must be stated precisely, or the description will be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` **never reports W1001/W1002/W1003 on
  the multi-file path** (`run_project` → `compile_project`); it does report them on the
  **single-file path** (`run_file` → `Pipeline::run`). The reason is `compile_project`: it packs the
  complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function **has no
  `result.warnings` read site anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 also has a different source**: `check_project:385-395` uses
  `surfaces.bins` (the manifest-declared surface), whereas `compile_project` uses `is_bin_role`
  (`250-252`, determining only whether a manifest exists). The two functions give different answers
  to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is a single source of proof obligations.**
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
the proof function must actually be executed to decide the constraint.”

**Step 2 — The consumer is inside the checker, but it only produces and does not consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                         | Behavior                |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → infer a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Infers a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Infers a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) stated:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real defense line rather than an advance concession: silent pass must not be resurrected.`

In other words, the author knew that “emitting no diagnostic” was a defect and explicitly classified
it as a **known intermediate state with nowhere to go**, on the design assumption that “someone
would eventually come to read `proof_calls`.”

**Step 3 — The field is indeed written into the result.** Inside `check_module_impl`, a local
`proof_calls` is declared and collected (originally `checker.rs:1234-1235`), then written into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registration of proof-function obligations`
(originally `:1439`). The field is defined at `types.rs:29`.

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

(The other occurrences of the `proof_calls` identifier fall into three categories and are not
consumers of this field: the producer-side collections in `checker.rs` and `checker/refinement.rs`
(originally `:1234/4564/5494`); `verdict.rs:61`, which is a same-named field of `ProofResult`; and
`tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points pass through that layer.** The
`compile_project` (`99`), `check_project` (`273`), `check_source_in_project` (`450`), and
`compile_embedded_module` (`1374`) functions in `src/frontend/module/orchestrator.rs` **do not call
`pipeline.rs`**; they call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also take the discard
path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible
for compiling embedded std. This means that **the embedded std module's own proof obligations are
likewise never executed**.

**Step 7 — Consequence: constraints pass silently.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostic** on the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent discard point inside the layer.**
When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`):

```rust
// 原 checker.rs:1313（现 check_module_impl 内）
ProofResult::Unproven { .. } => {}
```

**The ownership-checking layer's `Unproven` is swallowed by an empty match arm, with no diagnostic
and no accounting.** This directly contradicts the promise at the `5164` branch that “silent pass
must not be resurrected,” and it is **independent of the orchestrator problem**—even if the
entry-point layer is completely fixed, this location will still remain silent. (Processing timing:
it should be handled in the same batch as the obligation ledger because it is the same class of
problem.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test file's own documentation comment already states this.
`rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are handled at the .yx layer.`

**This is exactly the shape of the problem: the tests verify that “the obligation was populated,”
while the bug is that “the consumer did not read it.”** A test that covers only the producer, not
the consumer, is inherently immune to this class of defect.

### Stronger finding: the main corpus for the equivalence criteria uses the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) treats end-to-end differential testing
of the 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and the
primary acceptance mechanism for C2 stages. However, measurement shows:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero glob matches across
  the entire directory).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files use the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this. The file is annotated with `// expect: compile-error E4018` (line 15), while its
header comment on line 7 says “Status: ❌ should be rejected at compile time”—**it passes precisely
because it uses the only path that executes the proof function**.

**Conclusion: the third criterion layer needs an additional multi-file corpus layer; otherwise it
cannot serve as the acceptance tool for this document.** See “Implementation Points” S1.

### Obligation fields: 13 “production implies contract” fields with zero mechanical guarantees

Of the `TypeCheckResult` fields in `src/frontend/core/typecheck/types.rs:16-76`, **13 fields** have
documentation comments that explicitly say “produced by stage X → consumed by stage Y”; in other
words, they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: definition line = the **declaration line** of the field in `types.rs`
> (the line containing `pub` / `pub(super)`), not the first line of its documentation comment—a
> field's comment block may span multiple lines, and only the declaration line provides a unique
> anchor. The **struct range** in the sentence above (`src/frontend/core/typecheck/types.rs:16-76`)
> moves with the struct's line number and is mechanically reconciled by gate **F5** (a mismatch
> turns the gate red); the definition-line column is reconciled by **F4**. | `proof_calls` | `36` |
> typecheck → **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by
> `PlanId`**: read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-completion marker `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Reclassification record (3.4.9, user ruling A on 2026-10-13)**: the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design it
is consumed within a phase by typecheck and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed during this round).
- **Disposition**: delete the documentation comment's **claim** that it is “mirrored for consumption
  by the middle side/LSP”; retain the field and its in-phase consumption. The field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP later genuinely
  needs the table, register it through 4.6.x; **do not create a consumer in this round**.

**All these fields cross layers keyed by `Span` or name tables.** The equivalence-criteria document
already points out that mismatches in span-keyed contracts “fail silently, with no error”
(`ReleasePlan` is a typical example: any discrepancy in how either side computes the span → `Drop`
instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, while
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module is on the mandatory path of every entry
  point.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator's four entry points do not pass through the
  `pipeline.rs` layer at all.

**Verified fact**: the repository currently has **no mechanism whatsoever** that guarantees that
these 13 fields are consumed. The only “protection” is that `ReleasePlan` happens to ride along with
IR generation.

### Non-obligation field registry: channel / in-phase / in-tree fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation fields”
table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                                 |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (0 hits in `src/middle`)                                                                                                      |
| `diagnostics`             | Error-diagnostic channel (existing channel, not “production implies contract”)                                                                                   |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its documentation comment declares no cross-stage contract ⇒ not adopted                                      |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                             |
| `semantic_db`             | Consumer is in LSP (since 4.6.1, surfaced externally through the `DriverOutcome.semantic_db` channel, src/lsp/server.rs:482), not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`; documentation comment declares no contract                                                                          |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                      |
| `interface_impl_registry` | In-tree fact (user ruling A in 3.4.9; see the in-phase consumption sites in the “§Obligation fields” reclassification record)                                    |

**Why not simply “wire up the consumer side”**: creating downstream consumers for these fields would
invent spurious obligations and disguise fields that “do not require cross-stage consumption by
design” as contract defects. The correct closure mechanism is **reverse containment**—the
`TypeCheckResult` field set = obligation list ∪ non-obligation registry (hard-checked by criterion 1
of `scripts/ci/check-obligations.py`: adding a field without registering it, deleting a field
without synchronizing the registry, a non-obligation entry without a reason, or overlap between the
two tables all turn the gate red immediately).

### Four additional contract defects in the proof layer

The following all come from proof-layer analysis and are independent of the entry-point divergence,
but all belong to the class of “stage contracts not being enforced.”

**(a) Layer ordering is declared only in documentation, with no mechanical enforcement; the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual invocation order of
`checker.rs::check_module_impl` (positioned by function name/driver point, with line numbers taken
from the current tree):

| Actual order | Call site                                                                                                                                               | Corresponding layer |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                         | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                            | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                    | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                  | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comment and do not correspond to the invocation order; the README has no
“number → invocation timing” mapping.

`equivalence` is used only as an `is_subtype` utility function by `inference/assignment.rs:17` and
is unrelated to `ProofResult`. At the same time, **there is no short-circuit
whatsoever**—`check_module_impl` adds termination errors one by one and then continues downward; “a
lower layer failing means upper layers do not run” is not true.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-failure panic was removed by P3's 3.3.1 (the SOLVER slot
> was made optional; absence is conservatively degraded to `SMTResult::Unknown`); “silent skipping
> (no injection)” was given a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is to be performed under **RFC-039 D58**, at the actual fix location
> `proof/smt/backend.rs` (adding a process-wide shared singleton + `with_shared_solver` closure
> interface). The wording in the 02 change list that `default_solver() → &'static` takes precedence
> under D58 (a bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                   | Acquisition strategy                                     | When the solver is unavailable                               |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected through `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection with `with_solver_owned`        | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                | **New instance on every call** via `default_solver()`    | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver's capabilities’.” By contrast, the contract documentation at `backend.rs:60` says:
“`None`: backend unavailable……**callers should degrade conservatively**.” **Two philosophies coexist
in the same codebase, and the hard-failure path panics instead of returning an error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge-cut decisions. Meanwhile,
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—each call invokes `Z3Backend::new()`, creating a new
Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the documentation comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Since it is not shared
across calls ⇒ **the cache can never be hit in this invocation mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. It is the same
backend with two different lifecycle strategies.)

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
table** and still runs—ownership analysis degenerates into “nothing conflicts,” **with no warning
whatsoever**. (Recommended handling: emit a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### wasm current state: hosted by a shim crate, with 29 files containing live branches

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate is `rlib` only                             | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                           |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                           |
| The shim depends on the main crate (rlib) as its library  | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                                 |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                             |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                    |
| **CI contains 4 wasm builds**                             | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, namely the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten      | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pull `libz3.a` from a fixed URL; degrade to a warning if missing)                                                                                                                                                                         |
| **29 files** contain the literal `wasm32`                 | **22** have actual `#[cfg(...)]` attributes; the other 7 are documentation-only mentions (see the “F2 determination” subsection of `06-cleanup-inventory.md` for the inventory and per-file determinations)                                                                                  |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                         |
| `src/lib.rs` has 11 occurrences                           | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                                   |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`), while the shim follows a different path.

**This brings us to a fact directly relevant to this document**: the wasm playground entry point
`compile_playground` (`:84`) in `wasm/src/lib.rs` constructs `Program { kind: WasmPlayground }` and
hands it to `Driver::run` (the JS-side entry point `run_code`, `:122`)—the **single-file path**.
Therefore:

| Path consuming `proof_calls`                              | Executes proof functions? |
| --------------------------------------------------------- | ------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                   |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                   |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                   |
| Multi-file `run` (`compile_project`)                      | **No**                    |
| `check` (`check_project`)                                 | **No**                    |
| In-project LSP (`check_source_in_project`)                | **No**                    |

In other words: **the existence of `wasm/` turns “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** When the unified Driver was implemented,
this path was already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`); the implementation record appears on row
**4.2.8** of `09-execution-wbs.md` (`wasm` moved to `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 determination” subsection of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time, while six `not(wasm32)` arms are load-bearing in the native three-stage tables; there
are no low-risk items that can be deleted directly. The RFC-039 open question “build a wasm target,
or delete these branches?” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: `Stage` enum (exhaustively enumerable, no runtime registration)

**Core constraint: `Stage` is a compile-time exhaustively enumerable enum; runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can force at compile time that “a new
stage must be handled by the orchestration layer”—exactly the mechanism already proven effective by
the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency verification          Project
    Discovery,          // File discovery                           Project
    Parsing,            // Lexical + syntactic analysis             PerModule
    Registry,           // Module-registry construction              Project
    RoleClassification, // Role classification (Script/Bin/…)        Project
    Typecheck,          // Type checking (including embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code-family analysis                Project
    ProofExecution,     // Compile-time execution of proof functions  PerModule
    GlobalSlotAlloc,    // Global slot allocation                   Project
    IrGeneration,       // AST → ModuleIR                            PerModule
    Monomorphization,   // Monomorphization                           Project
    Linking,            // Cross-module linking / IR merging         Project
}

impl Stage {
    /// All stages. When a variant is added, this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module and which must run once per project” into a fact
expressed by the type. Stages with `Project` scope are structurally guaranteed to run only once in
`dispatch`—this eliminates questions requiring human reasoning, such as “should
`allocate_global_slots` be called once for every file?” in the current `orchestrator.rs`.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on ordering.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    artifact (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the draft order contradicted
>    the data flow.
> 2. The Check-form stage table was normalized to the topological order of `Stage::ALL`, with
>    dead-code analysis **before** proof: currently `check_project` executes proof before dead-code
>    analysis; the two have no data dependency, and their diagnostic sets are identical (C2 set
>    semantics)—only the within-file diagnostic order is normalized. The single-file path (dead-code
>    analysis embedded in typecheck, before proof) already conforms to the ALL order, so
>    byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user ruling on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    artifacts—the draft order would force the Registry arm to “hide parse,” making the stage table
>    lie about the data flow; it also reduced parsing on the multi-file path from 2 times to 1 time.
>    The current state is faithfully registered as an appendix: multi-file parse errors used to be a
>    hard abort (the `?` propagation of `build_registry_from`, in tension with CollectAll semantics,
>    registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 when its zero callers
>    were cleaned up, with the historical account retained here)—**4.10.1 has been fixed** (user
>    ruling on 2026-10-09: Rust-style collection semantics + plan B): parse failures on the Check
>    path are degraded to per-file diagnostic collection, and the affected file leaves the
>    compilation unit (does not enter the registry; importers report E5001); the MultiFile path
>    retains its hard abort (under FailFast, a broken file cannot produce IR, which is correct
>    semantics and remains permanently pinned).
> 4. Check-form implementation (4.2.2, user ruling on 2026-10-09) added two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the draft omitted these (these
>    artifacts were produced by `discover_with_used` in the orchestrator era and discarded by the
>    `discover` wrapper, so they disappeared because the data flow was not represented in the
>    table); b. The within-file diagnostic-order normalization was extended to the complete form in
>    note #2 above: E3020 entry validation moved into the RoleClassification arm (Check has no
>    Linking stage, so the arm holding `surfaces` and the AST performs validation), and the
>    within-file diagnostic order = stage topological order (E3020 → typecheck → W1006/W1003/dead
>    code → proof); the diagnostic set is unchanged (C2 set semantics), and only the order of stderr
>    entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective**: turn “a field is produced but nobody consumes it” from an undetectable
condition into a fact assertable at compile time or test time. RFC-039 already identified this as
“the single highest-value item.”

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
    /// Every unconsumed field produces one W-level diagnostic (by default), or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                       | Criterion                                                 | Disposition                                                          |
| ----------------------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The field in `Obligations` is `take()`n / marked consumed | Pass                                                                 |
| The obligation is non-empty but has no consumer | The field is non-empty and not consumed                   | **Emit a diagnostic** (W level by default; E level in `strict` mode) |
| The obligation is empty                         | The field is empty                                        | Pass (nothing to consume)                                            |

**Why start with W rather than E**: fixing discarded obligations will **change diagnostic sets**.
The C2 criterion requires that “the diagnostic sets of all entry points are identical”; moving
directly to E level would cause `yaoxiang check` to suddenly produce a large number of previously
silent `Unproven` diagnostics. Splitting the work into two steps (first W for observation, then E)
keeps every step's criteria usable. See “Implementation Points” S4.

**The call site of `assert_drained()` is unique**: `Driver::run` invokes it at the end of the stage
table, before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing ≥2
times but not being read by a third file → failure). `assert_drained()` is the **runtime** gate; the
script is the **static** gate. The two complement each other, and both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: this note registers the
> **implementation form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a code file:line correspondence for every item. The implementation commit is
> `1941cc47a` (verified tree `99079bc52`). It is also the **third** of the three places that enforce
> the red list's “no silence” rule (the other two are the audit surface and the static gate).
>
> **1. Implementation artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201; field
>   name/consumer stage/red list), **non-obligation field registry `NON_OBLIGATION_FIELDS` (:150, 8
>   entries with reasons)**, delivery-marker ledger `DeliveryLedger`, pure settlement function
>   `settle_obligations` (:577), external emission `emit_obligation_diagnostics` (:654),
>   non-consumer gating `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped`
>   (:549), IrGeneration delivery set `IR_GENERATION_DELIVERED` (:285), and W1090 code constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one of them makes runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —all four use the
>   same delivery interface: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct
>   request-table read) / `:1221` (MultiFile·Check: aggregated by the Linking arm,
>   `driver/helpers.rs:295` reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is written above the entry at :26-31—the settlement granularity
>   is “unit × obligation,” with no user-identifiable source location, so no dummy span is
>   inserted); text for all six languages is in `locales/*.json`. **Emission severity**: since 4.3.2
>   it is **error** (the same code is elevated to E; see part three)—determined by the explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code number remains in
>   the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above         | Implementation criterion                                                                         | Code                   |
> | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ actual read reconciled by the static gate; see part five) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see part three)         | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                 | obligations.rs:587-591 |
>
> Two **additional tiers** (user ruling on 2026-10-13; not included in the sketch):
>
> 1. **S1**: the obligation's consumer stage is not in this `ProgramKind`'s stage table → **do not
>    judge it as under-delivered** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: the consumer stage was skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → do not judge it as under-delivered (obligations.rs:598-602). `NoObligations`
>    does **not** grant an exemption—if an obligation is non-empty while the stage itself claims
>    there are no obligations, the ledger is unbalanced and must be reported.
>
> **3. External-emission policy (ruling “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: this mechanism emits only **W-level** diagnostics (W1090); there
> is **no strict switch**. **4.3.2 (final state, user ruling A on 2026-10-13)**: **the same code is
> elevated to E**—the `W1090` code number is unchanged, but the emission severity is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (two codes for the same fact would be parallel representations). **The elevation
> changes only the emission severity, not the emission conditions.** This is a **D-class breaking
> item under D59** (changes externally observable behavior) and is faithfully registered: healthy
> programs emit nothing (all obligations have delivery markers), so existing corpora and user
> programs are unaffected (342/342 “difference: empty”); however, if a run really hits an
> undelivered obligation, that diagnostic **counts toward `error_count`, renders as `error[W1090]`,
> and produces a nonzero exit code**—unlike the previous “W level does not block compilation.” The
> rule that the code family and emission severity are decoupled is registered in RFC-013's “Emission
> channel” section (the code prefix determines only numbering ownership; severity is determined
> explicitly at the construction site). Mechanism facts (measured): the severity comes from an
> explicit `.severity()`; otherwise, “a code beginning with `W` → Warning”
> (`codes/builder.rs:186-203`), and the registry's `ErrorCategory` **does not participate** in
> severity. The exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`); **since 4.3.2,
> W1090 follows `error_count` because its severity is explicitly error**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain ruling “C” on
> 2026-10-13; the current red list has been reduced to zero; see part four), for a recomputable
> reason:
>
> - Ruling A (emit the red list as W) has a measured cost: `interface_impl_registry` is populated by
>   the typechecker's prebuilt table and is **always non-empty**, so the per-compilation emission is
>   a **constant signal** with no discriminating power, yet it would break three existing external
>   contracts—“clean programs produce zero diagnostics / LSP produces zero diagnostics /
>   `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing test cases
>   turning red**, of which **8 were on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 were under `src/driver/tests/`), and the
>   corpus baseline changed in 24/342 fixtures. The only viable modification would be “filter W1090
>   out of zero-diagnostic assertions,” which weakens an existing criterion.
> - Ruling B (demote to Info) would still break 9/12 list-based assertions, and Info is not “output
>   according to the ruling's severity.”
> - Ruling C's measurement: **zero changes to existing test cases**, **zero changes to the corpus
>   baseline** (342/342 “difference: empty, behavior equivalent”), and runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains silent throughout, and removing
>   any delivery marker makes it fire (all three delivery points were mutation-tested by removing
>   their marker; the CLI `check` emitted 2 `warning [W1090]` diagnostics when the Check arm's
>   marker was removed).
>
> The red list's “no silence” is enforced in three places: ① the
> `DriverOutcome.obligations.findings` audit surface (emitted on every run, including WBS IDs and
> evidence pointers); ② `check-obligations.py`, which prints the red list (currently empty) on every
> run and performs stale detection; ③ this note.
>
> **4. Current state of the red list (closure through cleanup) and implementation records for
> 3.4.4/3.4.5/3.4.9**
>
> The red list went from 3 → **0**—all three cleanup tasks landed on 2026-10-13:
>
> Note: the line anchors below record the file state **as of 2026-10-22**; after P5 (2026-10-26)
> split the checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, the
> line numbers in `checker.rs` shifted as a whole, so the anchors below no longer correspond to the
> current file and are retained only as historical citations.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order taken from `sum_type_variants`); call site
>   `:7496` replaces the entire Call with `CreateVariant`; the function's documentation explicitly
>   states that “span keys are unreliable across systems (the two ends interpret the span of a Call
>   node differently), so detection is performed in place”—**wiring a consumer would reintroduce the
>   already-fixed span-mismatch defect**. Deleted surfaces: the `types.rs` field, collection at
>   `checker.rs:1489-1493/:1565`, the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`;
>   `infer_variant_ctor_call` type inference and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550`, decision point; the unique hit at `:4550` writes
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** at `checker.rs:1558`
>   that was placed in `TypeCheckResult`. The naming fact remains expressed in one place: the
>   candidate-name table (in-phase) + the span-keyed decision (cross-stage, `ir_gen.rs:380`
>   construction, `:7632/:7705` consumption); the mixed naming definition side is computed
>   independently by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in this document's §Obligation fields table). What was deleted is the
>   **claim** in the documentation comment that it is “mirrored for consumption by middle/LSP”
>   (`types.rs:47-48`); the field and its in-phase consumption remain (`operator_interfaces.rs:623`
>   operator lookup, seven constraint-solving lookups in `inference/expressions.rs`,
>   `checker.rs:214-217` read-only accessor); direct proof of zero cross-stage reads:
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire a consumer**: the
>   table's content is an environment mirror populated by the typechecker's prebuilt table (always
>   non-empty); if LSP genuinely needs it, register it through 4.6.x. Creating a consumer for a
>   field that “does not require cross-stage consumption by design” would introduce a false
>   obligation into the ledger.
>
> After cleanup, the **field inventory is closed by “obligation list ∪ non-obligation field registry
> (`NON_OBLIGATION_FIELDS`, with reasons)”**—the reverse-containment end of gate criterion 1
> protects this end to end (adding a field without registering it / deleting a field without
> synchronizing it immediately turns the gate red). The red-list **mechanism** is retained: gate
> criterion 4's stale detection and the `--selftest` `stale-red-list` fixture continue to cover its
> detection surface; any future registration requires an explicit ruling.
>
> The cleanup also synchronized the accounting: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / this document's §Obligation fields table); the gate's `--selftest` now covers
> seven fixture categories (adding “unregistered field,” “stale anchor,” and “empty red list”), as
> seen under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2) (three hard criteria beyond the sketch's sentence)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`'s `INVENTORY` ↔ the `TypeCheckResult` field definitions ↔ this document's
>    “§Obligation fields” table (including the table header's count declaration); any drift turns
>    the gate red. In addition, the `TypeCheckResult` field set must **equal exactly** “obligation
>    list ∪ non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field without
>    registering it, deleting a field without synchronizing it, a non-obligation entry without a
>    reason, or overlap between the two tables all turn the gate red immediately;
> 2. **Read evidence**: every non-red-list obligation must have a real read (`.field_name` member
>    access) in its **declared consumer file** (comments/tests/strings do not count)—this makes
>    “delivery marker ↔ actual downstream read” a hard criterion (the primary risk of mechanism A is
>    precisely that the two drift apart);
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    conversely, red-list obligations must not be marked;
> 4. **Red-list stale detection**: red-list fields must have zero real reads (growing a consumer
>    turns the gate red), and every entry must carry a WBS ID and evidence pointer; there is also
>    `--selftest`: built-in negative/positive fixtures, with negative cases always red and positive
>    cases always green (proving the detector is not a no-op).
>
> **6. Point-by-point deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy each field**: the obligation
>    data remains owned by `TypeCheckResult`, and the ledger records only “inventory + delivery
>    markers + settlement conclusions.” The reason is that copying creates two sources of truth (the
>    fields originate separately in the checker's typecheck and ownership layers) and necessarily
>    drifts; the cross-layer data plane should also not be duplicated in the orchestration layer.
> 2. The sketch's `take()` / “mark consumed” was **not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The reason is that an obligation and its
>    artifact are the same data, and `take()` would remove what downstream needs; moreover, 11
>    IrGeneration obligations have the form “hand the entire `&TypeCheckResult` to ir_gen,” so they
>    cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (since 4.3.2, W1090 is E level)—the single-file path uses
>    `PipelineError::TypeCheck` (the emission arm in obligations.rs:654), while multi-file/embedded
>    forms record an `OrchestratorError` failure separately (otherwise `result.errors` has no
>    consumer). The reason is that returning `Vec<Diagnostic>` loses form-based routing—check
>    diagnostics must remain attributable to individual files.
> 4. The first tier of the three-tier table changes from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is handled by the static gate
>    (criterion 2 in part five).
> 5. The second tier's “promote to E level in `strict` mode” **does not use a strict switch**; it is
>    replaced by **the same-code elevation to E in 4.3.2** (it is elevated even without a strict
>    switch, because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion is upgraded from “a field appears ≥2 times but is not read by a third
>    file” to the four criteria in part five.
> 7. The two additional tiers (S1 and skip exemption) in the sketch are described in part two.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table was actually
>    implemented as **20 cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]`
>    skeleton → turned green; red-state record: removing the sole settlement point yields `pairs: 0`
>    and 6 failures). The S1/S4 rows have been synchronized to the actual file.
>
> **7. Implementation record for 4.3.2 (severity assignment + gate hardening)**
>
> - **Input for field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in a given run, available per run) and the five counters (the conservation
>   identity in part two); together with the audit surface's `findings`, these provide the fact of
>   “whether this field has ever been consumed by any entry point.”
> - **Severity: same-code elevation to E has landed** (2026-10-13, user ruling A; see the
>   final-state registration in part three and the D59 D-class breaking-item declaration).
>   Implementation: explicit `.severity(Severity::Error)`; the emission arm routes through the error
>   channel by severity. Mutation evidence: removing any delivery marker → compilation fails in that
>   form and `check` exits nonzero (`error [W1090]`); after restoration, the healthy state emits
>   nothing (corpus 342/342 unchanged). The locale text in all six languages states that it is “a
>   W-family code whose emission severity is error.”
> - **Gate-hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence**, and validates that “the file is locatable
>     (uniquely) + the line number exists + the symbol immediately following the anchor really
>     appears in the anchored file”; an anchor pointing to a nonexistent symbol turns the gate red.
>     Built-in negative fixtures: `tests/baselines/obligations-gate/stale-anchor/` (one nonexistent
>     symbol and one out-of-range line). Note: the red list is currently empty, so the criterion's
>     actual landing point is **non-obligation registration** and future evidence pointers (the same
>     code path).
>   - **F2/R3 reverse containment**: the gate outputs “the number of `TypeCheckResult` fields − the
>     number of obligations − the number of non-obligation registrations = difference,” and a
>     nonempty difference turns the gate red; built-in negative fixture: `unregistered-field/` (a
>     new struct field not registered).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain a `type_result`, so the obligation
>   does not exist); this tier's semantics are pinned by a synthetic `State` case, while real
>   end-to-end coverage only includes `ConfigDisabled` (t3 T06).
>
> **8. Closure of the 4.3 second-level task (red list reduced to zero + final E-level obligation
> diagnostics)**
>
> **8.1 Three-round implementation timeline** (each item corresponds to code file:line; this round
> only records conclusions and does not change code)
>
> | Round                     | Conclusion                                                                                                                                                                              | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
> | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger launched     | `State::assert_drained()` as the sole settlement point; 13-entry obligation list + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                  | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                              |
> | 3.4.4 cleanup             | `variant_ctor_calls` field deleted—its declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                     | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                          |
> | 3.4.5 cleanup             | `method_overload_ir_names` cross-stage mirror deleted (in-phase candidate-name table + `overload_resolutions` retained)                                                                 | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                       |
> | 3.4.9 cleanup             | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirrored for middle/LSP consumption” claim, retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; doc comment at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven sites in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 severity assignment | **Same-code elevation to E**: explicit `.severity(Severity::Error)`, code remains `W1090`; external routing through the error channel by severity (first failure takes priority)        | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); registration of code-family/severity decoupling in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                      |
>
> **8.2 Closure conclusions**
>
> - **Red list reduced to zero**: the number of entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) whose `red_list` is `Some` = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism is
>   **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's stale
>   detection, and the `stale-red-list` fixture; the gate outputs “red list: 0 entries—cleanup
>   complete.”
> - **Final E-level obligation diagnostics**: the **code number and W-family membership of `W1090`
>   remain unchanged**, while its emission severity is `error` (D5 final state, user ruling A on
>   2026-10-13). This is a **D-class breaking item under D59** (changes externally observable
>   behavior), registered both in part three of this note and in RFC-013's “Emission channel”
>   section; **zero emission in the healthy state ⇒ no impact on existing corpora or user programs**
>   (corpus 342/342 “difference: empty,” zero IR snapshot drift; independently verified by t11 and
>   reviewed by t12).
> - **Closed field inventory**: the `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a measured consumption-based
>   reason); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises left**: all hardening items registered in this note have **landed**
>   (F1/F2/F4/F5; see parts five, seven, and eight); the only two items not fully closed are the
>   **known boundary** in 8.4 and t7 F5, which has been moved to an issue; neither is an
>   unimplemented criterion of this mechanism.
>
> **8.3 Disposition status found by each round's acceptance/review** (each item identified)
>
> | Source        | Finding                                                                                                                                                                                      | Status                                                                                                                                                                                                                                                                               |
> | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | t3 F1 / t4 R2 | Red-list evidence consists of plain-text pointers, so removal of a replacement consumer could leave it silently stale                                                                        | **Landed**: gate F1 anchor validation (uniquely locatable file + registered line + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures `tests/baselines/obligations-gate/stale-anchor/`                                                                         |
> | t3 F2 / t4 R3 | Gate criterion 1 lacked reverse containment, so a newly added field that was not registered silently passed                                                                                  | **Landed**: F2 explicitly outputs the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                             |
> | t4 R4         | The W-only vs. elevated-to-E policy was not registered                                                                                                                                       | **Landed**: parts three/seven of this note + RFC-013 “Emission channel” + W1090 help and error_output examples in the six `locales/*.json` files                                                                                                                                     |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (valid overloads are falsely reported as `W1005`)                                                                              | **Moved to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237` registers it)—outside this second-level task                                                                                           |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                                            | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                         |
> | t11 F8        | Validator-side self-record: mutation-driven verification once wrote into the main repository (cleared immediately with `git restore`; the affected round's output was invalidated and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                                         |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document's table header differ from `INVENTORY`) triggered a red gate                                                        | **Informational / resolved**: the frozen commit was self-consistent and passed; “committing an intermediate state directly turns the gate red” is precisely F2/F4's intended behavior                                                                                                |
> | t12 F12-1     | This document states the struct range as `16-73`, but it is actually `16-76` (t9 added 3 comment lines without mechanical reconciliation)                                                    | **Landed + mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; a mismatch turns the gate red); related figures were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                       |
> | t12 F12-2     | Settlement in the multi-file form once displaced the first failure (the type error in `zzz.yx` was overwritten by W1090)                                                                     | **Fixed** (the captain amended the commit message to explain it): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If such an anchor promise is written in a module header
>   in the future, it must either be **covered by human review or have F1's scanning surface
>   extended**—one of the two is required; it must never be left unattended by default.
> - **The gate's static boundary**: collection-level validation of delivery markers is static
>   (criterion 3), while removal of an individual arm's marker is covered by **runtime settlement**—
>   static + runtime are the intended two-layer design, not a gap (bidirectional mutation
>   verification by t11's M11A/M11B/M11C).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId implementation)**:
> of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) were converted under RFC-039 **D20** into **structural-identity keys**
> (`PlanId`). This note registers the implementation form, criteria, classification basis, and known
> boundaries; definition lines are still reconciled by gate **F4** (the line numbers in the two rows
> above are current code facts).
>
> **1. Implementation form (carrier chosen: `PlanId`)**
>
> The key is a **structural identity** that travels with the AST node, no longer relying on the
> positional coincidence of “both sides computing a span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonic `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthesized nodes outside the parser) is issued by the **sole
> allocator** `ParserState::alloc_plan_id`. It has two carriers: `Stmt.plan_id` (statement identity
> → ownership of the release plan and its consumption) and `Expr::Call.plan_id` (call-site identity
> → production and consumption of overload resolutions); `Expr::call` is the repository's only
> call-expression constructor, and all synthesized calls outside the parser are recorded as
> `UNPLANNED` (**not consumable**; see part two).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), with the
>   sole consumption entry point `ReleasePlan::consume(stmt)`—**reading and accounting happen in the
>   same call**; read at `ir_gen.rs:360`, used at `:3103` (statement identity is supplied by
>   `walk_stmt`'s set/restore operations + the CFG-node → statement-identity table; a Drop at scope
>   exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (the table and consumption ledger are **one
>   object**), produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with
>   dispatch passing through the destructured `Expr::Call { plan_id }`), consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) through `OverloadResolutions::get`
>   (a hit is recorded).
>
> **2. Guaranteed consumption settlement (reuse the 4.3 ledger; do not create a second system)**:
> the consumption ledgers share one **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with each of the two tables holding an instance; when
> `generate_module_ir` returns successfully, both ledgers are marked at the **same point** as having
> “completed one full pass” (`ir_gen.rs:1095` release plan / `:1097` overload resolutions); the
> three-state mapping is unique (the `consumption_state` in `src/driver/obligations.rs`), and the
> sole settlement point connects one row for each of the two obligations—a nonempty difference means
> **E-level W1090** (same-code elevation to E; see part three). The exemption criterion is attached
> to the **ledger's own completion marker** (not to whether `unit.ir` exists), so the Check form's
> removal of the IR artifact does not constitute an exemption.
>
> **3. D59 classification basis (class C, red before green)**: before the change, the form
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` instead of `20`—the selected
> candidate was discarded and execution fell back to the last binding in definition order); after
> the change, the same program fails at compile time with
> `error [W1090] ... produced key was not consumed`. The chosen fix is **loud** rather than “put the
> resolved name into the call with a named argument”: the latter would lose the named argument and
> merely replace one form of silence with another (the final semantics are described in
> [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop` instructions are a no-op in the
> interpreter (the wildcard arm of `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty
> difference across 342 corpus entries is evidence of this.
>
> **4. Criteria and evidence**: the same-kind criterion D41 `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + an evidence-surface
> floor**: the number of files that produced decisions `≥ 1`, the number of genuinely consumed
> entries `≥ 2`, with no allowlist); nine pinned cases covering the decision forms are in
> `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return / positional argument
> / in-type body binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven verification: shifting a consumption key by
> one position ⇒ the criterion turns red + a real program produces `error [W1090]` + the corpus gate
> differs; restoring it turns green again. Gate figures: `check-obligations.py` PASS, a zero
> difference across 342 corpus entries, and `cargo test --lib` 2712 passed / 0 failed (zero IR
> snapshot drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: the **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test surface
> (method declarations uniformly use `register_type_bindings` and therefore reach the bound
> consumption point), so the correctness of this branch's keying has **no red state that could
> disprove it**; the reason for retaining it is that it is the symmetric branch of the bound path,
> and deleting it would rewrite the “method call without `type_bindings`” form into silence. This is
> recorded by independent verification (t5 F-1) and review (t6 F2); **no new task is opened**.

### 3. Unified Driver: a single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (in-project LSP; results filtered to the target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — the fourth caller on the single-file path
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
callers cannot freely supply an array.** This is the key constraint preventing the `Program`
abstraction from degenerating into “a free-form parameter that can accept anything,” and it is
asserted by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines skipping, not the stage's return value
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

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: the four implementation
> deviations from the sketch above are semantically equivalent and are recorded for reference:
>
> 1. §4's three-state `StageOutcome` is not returned by stage arms; it is expressed through `State`
>    failure markers + an `Aggregation` gate (Continue / Warn / Abort semantics remain unchanged,
>    removing per-arm boilerplate);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the §5
>    decision to “avoid mutable global state in Driver” overrides the sketch field);
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-diff criterion of S2 “deliberately fix no bugs”
>    requires this; external emission follows with the 4.3 obligation ledger;
> 4. The visibility of the `proof_execution` module is relaxed to `pub(crate)`: the driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their proper layer after the 4.2 migration, to be reconsidered later.
> 5. In the multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user ruling on 2026-10-09): `compile_project` originally interleaved proof
>    within the per-file typecheck loop (file N's proof preceded file N+1's typecheck). A single
>    failure produces byte-for-byte identical output; in a multi-failure scenario involving “file 1
>    proof failure + file 2 typecheck failure,” the first reported error changes from the proof
>    error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through separate channels by ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    the entry points (PipelineError / OrchestratorError) are outside the type surface that Driver
>    can unify.
> 7. The Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; a clean file has an empty Vec—the existing
>    `check_project` contract); the Discovery artifact is extended to a “file set + used_by edge
>    set + shadowing events” triple stored in State (the MultiFile form has no downstream consumer,
>    so it is recorded but not emitted externally); the Check path parses each file once rather than
>    twice following the C3 topological order.
> 8. The Lsp form landed (4.2.3, file-source routing ruling, user decision on 2026-10-09): parse
>    failures for files on disk use the same plan B as Check (collect + leave the compilation
>    unit—both sides converge through degradation); parse failures for the edited buffer retain a
>    partial AST and continue typechecking (the editor philosophy—an intermediate typing state
>    should not make semantic functionality disappear). The Discovery arm preserves buffer source
>    over stale on-disk content for Lsp; Typecheck under Lsp checks only the target file (other
>    units exist only to provide signatures to the registry); the within-file diagnostic-order
>    normalization principle extends to LSP (typecheck diagnostics → W-code warnings → proof
>    errors). The old behavior in which LSP hard-aborted on parse errors in unrelated disk files and
>    the handler silently fell back to the single-file path is fixed by this step.
> 9. The Embedded form landed (4.2.4): `Program` gains a `shared_registry` field—the embedded std
>    module is a child compilation, and its registry is an **input**, not an artifact (EMBEDDED_
>    STAGES has no Registry stage), making issue #94's SymbolTable-sharing contract explicit as a
>    program declaration rather than “the caller must remember to pass the same registry.”
>    Error-path text changes from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler-internal error surface only; no test pins it). At this point all four orchestrator
>    entry points have moved into the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR-stage ruling, 2026-10-09): The Check form
>     faithfully carries the single-file semantics for a program without a project root—the warning
>     surface covers only the entry file (neighboring files receive only errors; better to miss than
>     falsely report); relative `use` paths are resolved relative to the importer's directory
>     (aligned with rustc's single-file mod behavior). The CHECK stage table gains GlobalSlotAlloc +
>     IrGeneration: the old standalone path (the full pipeline) already ran IR generation, and
>     E3019/E1014/E1015 etc. are produced only by ir_gen (confirmed by the runner gate).
>     IrGeneration in the Check form is check-only and does not consume IR; the Script/Bin split is
>     represented by the None/Some value of module_key (the existing E3023 switch, ir_gen.rs:1660).
>     Monomorphization is not included in CHECK (it produces only resource-limit/internal errors and
>     has zero corpus dependency). **〔Withdrawn on 2026-10-10 based on 3.4.8 evidence〕**—“zero
>     corpus dependency” was merely evidence from the absence of pathological-recursion fixtures in
>     the corpus; E3005 is the only compile-time defense for this class of programs. See note #14.
>     Incidental fix for a latent defect: repeated registration harvesting by the Registry arm for
>     embedded std units would replace the native half already merged by with_std() (false E1043
>     reports)—embedded units now skip repeated registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI use the same Driver. In
>     the SingleFile+CollectAll form: Parsing collects all parse errors and retains a partial AST to
>     continue typechecking (the editor philosophy from the 4.2.3 ruling extends here; the stage is
>     not marked failed, otherwise topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now covers proof (E4018), W-code warnings (W1001–W1005, pinned by tests) and IR-level errors;
>     lexical failures now report real diagnostics (the old synthetic “E0001 lexical error” text
>     disappears with the deleted sequence—real diagnostics have precise spans, with no information
>     loss).
> 12. Line-by-line verification of `check_module` / `check_module_collect_all` (4.2.7, mandatory C5
>     item): the two entry points had already converged independently—the mod.rs layer has
>     `check_module_inner(ast, env, collect_all)` with a single Boolean, and the checker layer has
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, with pass-3 and drain both
>     running in both modes and the difference carried by whether `collected_errors` is empty; the
>     five collection points in statements.rs (function body/use/for/block/while). Verification also
>     confirmed and fixed a duplicate-diagnostic defect: collection sites put the first error into
>     both `collected_errors` and the Err return channel, and pass-3's `add_error` for Err produced
>     two copies of the same code, span, and message (up to ×3 when nested); there were also two
>     independent mechanisms for duplicate annotation-validation signatures and arguments
>     (E1003/E1103), and for duplicate ownership-layer emissions at the same site (E2014/E2018). The
>     fix was applied at the module-result boundary: deduplication by (code, span,
>     message)—diagnostics are positional facts, and duplicates carry no information; 63 corpus
>     entries lost duplicate copies, with zero drift in the run columns and exit codes (the apparent
>     “duplicate” in borrow_conflict_err is an artifact of the baseline triplet format not
>     containing column numbers—the two E2018 entries are legitimate diagnostics at different
>     columns).
> 13. The wasm playground landed (4.2.8): `run_code`/`test_compile` use `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper layer no longer
>     participates. Error text is aligned byte for byte with the old `CompileError` Display prefix
>     (`Parse error:/Type error:/Internal error:`), with zero visible difference in the playground
>     UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the existing 4.1
>     declaration now has real producers).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing note #10's mono exclusion**). Evidence disproved the claim
>     that “mono produces only resource-limit noise”: pathological generic recursion (`f(x)=f([x])`)
>     causes the compiler process to overflow the stack on the multi-file path without mono
>     (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime error), and the mono
>     depth gate is the only compile-time defense; standalone check used the full pipeline before
>     4.2.5 and therefore already had mono, so excluding it caused a silent coverage regression
>     (“zero corpus dependency” only proves the corpus lacks pathological fixtures). The
>     “type-erasure fallback” assumption was invalidated by the RFC-033 reflection
>     ruling—`^^List(Int)` requires a real identity to instantiate, and monomorphization is the
>     foundation of reflection rather than an optional optimization. Four design points: ① typecheck
>     produces an instantiation_request for qualified calls (`lib.f(x)`, FieldAccess form), with
>     generic_id using the qualified name, sharing the merged-IR namespace (currently only bare Var
>     is recognized, expressions.rs:2455); ② the `Stage::ALL` topology changes to Linking before
>     Monomorphization, and the MULTI_FILE stage table gains Monomorphization at the tail— consuming
>     merged_ir and aggregating all units' requests (qualifying the deferred bucket's
>     containing_fn), so mono is promoted from a single-module pass to a whole-program pass; the
>     single-file stage table remains unchanged (without Linking, subsequence properties are
>     preserved and IR snapshots do not drift); ③ CHECK gains Linking (pure merging—E3020 entry
>     validation remains in RoleClassification to prevent duplicate reports) and Monomorphization
>     (check-only, like IrGeneration in the Check form); ④ resource protection covers cross-unit
>     mutual recursion through whole-program BFS. The Monomorphizer itself is unchanged (its
>     contract for consuming ModuleIR + the request set exactly matches the merged IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the #14 design: `ExpressionInferrer` receives a module-qualified key
>     table (the `use` three-arm registration—ordinary / alias / item-by-item alias, with only
>     SubModule-class exports), and `callee_generic_name` uses `SymbolTable::qualify` to concatenate
>     a qualified name from the same source as merged IR; the qualified-call first-path arity
>     criterion is tightened (when the parameter table cannot be resolved, it falls back to a second
>     path using the signature to obtain the actual arguments, avoiding E3018 mismatches between
>     same-named functions across modules). Implementation empirically exposed and fixed **three
>     asymmetries** on the mono side in the same round: (1) the deletion-key (generic-name set) and
>     rewriting-key (containing_fn + name + span triplet) granularity mismatch—when actual arguments
>     could not be resolved at a site, it degraded to a symbol request in the deferred bucket, and
>     if the containing function was non-generic the bucket would never drain, while the original
>     artifact had already been deleted by name → dangling call (runtime E6006 evidence from
>     `list_ops.yx:53`); the fix restores the gate `restore_generics_with_uncovered_call_sites`,
>     using `build_call_site_map` as the sole source of truth to determine coverage and retaining
>     originals at uncovered sites. (2) `HashSet` iteration order in the restoration section made
>     function-table order nondeterministic (t2 review F1) → changed to `Vec`, sorted by name,
>     making the artifact byte-for-byte reproducible. (3) non-rewritten forms TailCall / MakeClosure
>     still looked up mapping keys as before (despite the comment claiming conservatism) → a single
>     `call_form_is_rewritten` determination was introduced, and non-rewritten forms are always
>     counted as uncovered. Each has a pinned test (red-state evidence of discriminating power
>     before the fix). The corpus differential baseline was regenerated at the same commit,
>     producing exactly one line (a `(int64)` inserted into a remove_at frame). **Commit-strategy
>     note**: when typecheck collection and the mono fix were split into two commits, the tree
>     containing only the former triggered `stdlib_docs` on a pre-existing HEAD defect
>     (`list.is_empty([])`, same E6006 family)—the two halves are each a prerequisite for the
>     other's green state, so they were committed atomically as one commit.
> 16. Five rulings registered on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—user-module
>     exports carry `Export.type_params` together with `generic_fn_type_params_snapshot()`, matching
>     the std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified-call monomorphization, and signature inference is demoted to a fallback (included in
>     R2's scope). (2) **HKT declaration side is live, call side is broken** (inference passing type
>     arguments E1002 / explicitly providing type arguments E1010) is registered as WBS 3.5 (3
>     third-level tasks). (3) **The deferred bucket never drains under a non-generic container** is
>     included as an acceptance subitem of 3.4.8 ②: the bucket must drain or fail loudly; silent
>     failure to specialize is forbidden. (4) **Closure variables called by name cause runtime
>     E6006** is registered as WBS 3.6; the semantics are (i) support indirect calls / (ii) reject
>     at compile time; **(i) was ruled**—support indirect calls: the `CallDyn` mechanism already
>     exists (`ir_gen.rs:7945`, closure-expression path); what is broken is that the `Var` callee is
>     demoted to a name-based `Call`, even though typecheck already accepts such programs. (5)
>     **Trade-off criteria are fixed**—a rejection rationale may be based only on correctness and
>     readability; “large change surface / introduces a new mechanism / high cost” is not a
>     rejection rationale and belongs in the second part of `coding-rules.md` and the HOWTO
>     self-check list; trade-offs requiring a user decision follow all four items in `AGENTS.md`
>     under “Requesting a user decision (mandatory)” (situation + evidence / item-by-item pros and
>     cons / explicit recommendation + cost / explicit naming of breaking changes).
> 17. 3.4.8 ② landed and F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the #14 design: `Stage::ALL` was changed to Linking before
>     Monomorphization, and the MULTI_FILE table gained mono at the tail; the Linking arm aggregates
>     whole-program requests **before taking each unit's IR** and qualifies `containing_fn`
>     (otherwise deferred-bucket keys and merged IR would not come from the same source and the
>     bucket would never drain). **① closure (D2 ruling A)**: user-module exports carry a
>     declaration-time type-parameter snapshot via `Export::type_params`, matching the std path
>     (`yx_sources.rs:62/108`). **Existing defects exposed and fixed during the work**: (a)
>     **F1**—`qualify_names` rewrites only function bodies, and `ir.init` is never visited, so in
>     multi-file builds function definitions are qualified while top-level module calls remain bare
>     names → runtime E6006 (**reproduces even without generics**, and is unrelated to generics);
>     the fix reuses the same `rename`/`aliases` and the same `rewrite_call_names` for init. (b) The
>     mono side's “whole-program unique suffix” heuristic and silent `continue` on a miss → the
>     heuristic was deleted and misses were changed to a **loud E3018** (ambiguous silent
>     mismatching is worse than failing to resolve). **Independent verification caught and fixed a
>     regression**: t6 found that a named import `use lib.{pick}` followed by a bare-name call was
>     falsely rejected with E3018 (the IR instruction name is the alias's qualified name, while the
>     request name is bare, and the aggregation side only qualified by unit key) → the fix makes the
>     aggregation side **directly invoke ir_gen's** `import_aliases_from` and `resolve_call_target`
>     (extracted into one public point, one producer and one consumer), with a pinned test matrix
>     covering both named-import forms. **Record**: the blank `DefId` in the init section was
>     empirically a “consistency blank” rather than a live defect (the codegen name fallback can
>     resolve it, and all four kinds of init call were measured to work), and is registered as WBS
>     3.7.4 to close the policy split between “function bodies use def” and “init uses names.”
> 18. The init-section DefId policy split was closed (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     was extracted into a single `resolve_static_targets` implementation (Call/TailCall string
>     operands + MakeClosure), with function bodies and `ir.init` sharing both the implementation
>     and the `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)`
>     pairs changed (other-lines 0; zero residual `(def=_)` entries in init sections across the
>     tree). The codegen DefId main path and the name-fallback path produced identical translation
>     results and execution values item by item (all four kinds of init call measured), with zero
>     corpus-baseline drift; the probe flipped under the new policy and was strengthened (identity +
>     exact index + execution value; removing init coverage now turns it red). **Erratum
>     (2026-10-11)**: the commit IDs above were rewritten by `git pull --rebase`—the original
>     b554497b/e60dcc87/b7ac7396 correspond in order to 375789d2/d922a551/38bd4b9e after the rebase;
>     the new IDs are authoritative in the pushed dev history (the old IDs were local temporary IDs
>     and never entered remote history).
> 19. 3.4.8 ③④ landed and verified (2026-10-11, 8ecf56f93). (③④'s design basis is in #14; ②'s
>     implementation is in #17, which this entry closes.) **Implementation**: the `CHECK_STAGES`
>     tail = `…IrGeneration → Linking → Monomorphization`; Linking in Check performs only pure
>     merging (aggregating requests + `link_module_irs`), and the entire entry-validation block
>     remains in `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (in
>     memory only); mono failures enter the collection channel in Check, while MultiFile retains
>     `fail_with`; `into_outcome` first calls `take_check_diagnostics` and then checks for failure
>     (structural failures no longer clear already-collected diagnostics). **Verification**: four
>     rounds of independent verification (t3/t4/t10 + the t5 cross-form matrix) and one round of
>     review (t6) all passed; the corpus differential was IDENTICAL line by line across 340/340
>     entries, IR snapshots did not drift, and the test count rose from 2643 to 2656. **An existing
>     gap discovered and fixed in the same round**: F1 (in rootless multi-file check, request names
>     and IR definition keys did not come from the same source → silent failure to specialize, as
>     recorded in the implementation note on row WBS 3.4.8)—the initial fix was for the aggregation
>     side to look up the exact real definition key rather than using a heuristic (first version:
>     exact reverse lookup using `flat_defs`); **same-day erratum**: that bridge was removed during
>     the regression fix, and the root cause was instead solved by decoupling the dual use of
>     `module_key` (`script_mode`), with all three sides using the same source `{unit key}.{name}`.
>     The location and fix for this regression (the CI documentation-example gate went from 258
>     blocks / 10 failures → 0 failures) are described in the implementation note on row WBS 3.4.8.
>     **Record (existing, not a regression from this round)**: the E3005 help text does not match
>     the resource-protection semantics; the span label for multi-file E3005 is misplaced
>     (reproducible with the HEAD binary); the type-parameter-position dependency on the
>     request-collection surface remains on the same side and is assigned to 3.5.
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and the diagnostic surface closed (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard reachable)**: interpreted
>     calls were changed to **explicit-stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` push frames and return control to the outer `step_one`; frames carry
>     `ReturnTarget{Host|CallerSlot|Discard}`, and on return they write back to the caller's slot;
>     multiple calls in one instruction continue through the frame's `pending_calls`. Baseline
>     measurement showed that each interpreted call level consumed **2544 B** of native stack, with
>     the crash threshold at `count_down(394)` (Windows main thread, 1 MiB); after the fix, 20,000
>     levels of recursion run successfully on a 512 KiB-stack thread (approximately 0 B per level).
>     The guard (`push_frame`) is therefore reachable from every entry point and every
>     `max_stack_depth` setting; `ExecutorError::StackOverflow` was changed to carry the actual
>     limit (previously `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2 (truncated
>     deep-stack echo) was merged into this task by user ruling** (same location in the same file;
>     cannot be split by file): the echo now shows the first 16 frames + an omitted count + the last
>     8 frames; shallow-stack output is unchanged frame by frame. **3.10.1 (no compile-time guard
>     for native recursion, #447)**: locate first, then fix—the crash point was recursive
>     `MonoType::type_name` (**328 B/level** (SP probe + precise threshold determination)); the same
>     path also has derived `Clone` (~400 B/level), deep copies inside `substitute`, and
>     `type_name_hash` (164 B/level), plus `collect_generic_type_refs`; after converting these four
>     locations to work lists and removing one full-request deep copy per level, a pathological
>     generic recursion with `mono.max_depth=5000` produces **E3005 (with the real limit)** and no
>     longer overflows the stack. **3.11.1 (structured task-boundary error preservation, #448)**:
>     the task-result error side changed from a value to a **structured form**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), with values
>     restored across the boundary; E6001/E6003/ E6004/E6008 within tasks are word-for-word
>     identical in Standard and Embedded messages. Two **pub API payload-shape changes** are
>     documented in the commit as `BREAKING CHANGE`. **Commit-strategy note**: 3.9.1 and 3.11.1
>     **cannot be split at file granularity**—`interpreter/executor/executor.rs` carries both the
>     trampoline and the error-channel producer/consumer, and `TaskFailure` is defined in
>     `runtime/engine.rs`; staging only one side does not compile (pre-commit compiles a staged
>     subset and rejected it twice), so under the same policy as #15 they were committed as **one
>     atomic commit**. **Known boundaries and follow-up**: the derived `Drop` (~75 B/level) means
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant-evaluation) gate carries native
>     recursion (approximately **3,696 B/level**, default 256 levels ≈ 924 KiB), currently has zero
>     call sites, and is unreachable in production; wiring it in future requires closing the same
>     family of issues; Standard-mode tasks construct the interpreter with the default
>     `ExecutorConfig`, so their E6004 limit is always 1024 (configuration propagation is registered
>     as WBS 3.12, user ruling B: a separate team); the test-runtime budget is a separate issue #449
>     (pathological fixture 76 s, clippy on both targets, full pre-commit run), and is outside this
>     RFC's construction surface. **Corpus directory stratification (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134), 212 fixtures in total, were split into 23
>     category-based subdirectories; 88 IR snapshots were moved to mirror paths; the corpus baseline
>     was regenerated at the same commit (342 lines, independently recomputed and compared line by
>     line: only the `file` field and 2 new changes), 41 snapshots were regenerated because their
>     file-header paths became longer, and a line-by-line review confirmed that **only span offsets
>     changed** (zero difference after normalizing offsets away).
> 21. The LSP semantic-data pipeline was unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references in a project carry
>     `resolves_to` pointing to the real definition— the old form triggered **two** compilation
>     passes through didOpen/didChange: diagnostics used the unified Driver pipeline, while semantic
>     data used a manual lex→parse→single-file typecheck in server.rs (with no module registry,
>     project-level `use` resolution failed and no reference events were produced, so cross-file
>     navigation/rename/reference finding silently failed—the silent failure registered as D55 was
>     closed by this task). **Implementation**: (1) `Export` carries `definition_span` (harvested by
>     extract_module_info from AST identifier nodes, from the same source and with the same value as
>     the typecheck definition event's `def_span`; native std is always None), and `ModuleInfo`
>     carries `definition_file` (filled by the Registry arm from DiscoveredFile ); (2) when the
>     checker drains reference events, it uses `import_def_locations` to redirect `resolves_to` for
>     cross-file imports from the use line to the real location in the definition file (std imports
>     have no registration and continue to resolve to the use line); (3) the checker's module
>     identity changed from the unified `"<module>"` to the unit file path (this value is consumed
>     only on the semantic-data surface, with zero diagnostic consumers—confirmed across all 4
>     consumption points of `env.module_name`); (4) `DriverOutcome` gains a `semantic_db`
>     channel—fed by CollectAll forms (Lsp target file regardless of diagnostic success + SingleFile
>     fallback); the Lsp arm's tail injects top-level export definitions for non-target files from
>     the registry (`external_definitions`; files that have not been opened can still be navigated
>     to; type/submodule exports are not injected—type registration on the typecheck side does not
>     produce definition events, so both sides consistently remain absent); (5) orchestrator gains
>     `analyze_source_in_project` (`check_source_in_project` delegates to it and takes only the
>     diagnostic half); (6) the LSP-side `analysis.rs` single-compilation entry point replaces the
>     manual pipeline (`update_semantic_db` in server.rs is deleted), and World keys are unified as
>     normalized file URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is idempotent normalization;
>     drive-letter case and %XX encoding differences are normalized to the same key). **Incidental
>     fix (D59 routing class B—old behavior did nothing silently and does not change the externally
>     visible behavior of any previously successful program)**: cross-file
>     navigation/rename/reference finding for project files, which previously silently failed, now
>     hits the real definition; when there is a lexical error, the old manual pipeline deleted
>     semantic information, whereas it now retains a partial AST with coloring (an extension of the
>     4.2.6 editor philosophy); when the target file has type errors, semantic data is no longer
>     discarded along with `type_result` (coloring/navigation no longer disappear during
>     intermediate typing). **Pinned tests**: 6 at the driver level (target-file path key/cross-file
>     resolves_to + external-definition round trip/std import fallback to use line/error-file
>     retention/single-file fallback path key + error retention), and 1 LSP end-to-end test (red
>     before green, empirically confirming that navigation previously returned None), plus 4
>     URI-normalization tests.

**How to refactor the ten entry points (one function at a time)**:

| Entry point                                                | After refactoring                                                                                                                                                                                                 |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body and replace it with construction of `Program { kind: SingleFile, … }` passed to Driver                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                           |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                      |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                        |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver invocation + filter the result to the target file                                                                                                                                            |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                         |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                            |
| `lib::run_project`                                         | Unchanged (delegates to Driver)                                                                                                                                                                                   |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                            |

**The degree of duplication between `check_project` and `compile_project` (approximately 40% shared
/ 60% divergent)**. What they share is: vendor consistency verification, file discovery,
`build_registry_from` (deleted on 2026-10-26 when its zero callers were cleaned up; the historical
account is retained here), `all_method_bindings`, the parsing loop, per-file checker assembly, and
result processing. What differs is items 2/3/4/5/7/8/9/10/11 of the 11 items above. **This 40%
shared skeleton is precisely Driver's value interval**—all 60% of the divergence is either “a
different set of stages” or “a different aggregation mode,” and these are exactly what the two
`Program` fields can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had non-blocking problems; continue
}
```

**Key design decision: “an upstream failure caused this stage to be skipped” is not a stage return
value.**

The reason is that skipping is determined by **topology**, not by the stage itself. If every stage
returned `Skipped` itself, the information about “why did I not run?” would be scattered across the
12 stages and could not be centrally audited. Instead, `dispatch` determines it at the beginning of
the loop according to dependency relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect this document seeks
to fix: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”—it produces
no diagnostic, merely because the obligation happened to be empty. Once this rule is established,
every “Y did not run because X was not done” must be explainable.

Diagnostic text must distinguish three skip reasons:

| Reason                                                           | Text direction                                                                                                                      |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                    | “Stage Y was not executed because upstream stage X failed”                                                                          |
| Caused by an empty prerequisite obligation                       | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; not counted in the warning total** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y was not executed because it was not enabled by configuration”                                                              |

The second category is critical: it distinguishes a “normal skip” from an “abnormal skip” in the
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
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)       | Existing            |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing            |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both project
files and single files in the same process, while CLI `run` and `check` are two independent
invocations; placing it on Program avoids Driver holding mutable global state.

**Note the semantic difference between the two current typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ in more than “early return
versus no early return”—they are two different checker entry points. After unification, one
implementation should be driven by the `Aggregation` parameter rather than retaining two functions
(**this is an additional point that must be verified in this document; see “Known Limitations and
Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types do not change**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                              |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the remaining field types stay
in their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers (for example, `ir_gen` reading `release_plan`) continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
driver no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend on `crate::driver` in
reverse**. The presence of `Driver` in a `TypeChecker` import is considered a violation and is
rejected by the proposed `scripts/ci/check-module-boundary.py` in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                    | Same 5 stages + `assert_drained()` settlement (**revision in 4.10.3, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone—it uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven by the rootless path; this row now describes only the `lib::run_file` / `Compiler::compile` library path)                                                                                                                                                                     |
| `yaoxiang run` (multiple files)     | No proof_execution and no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “not the pipeline channel,” and Check / Lsp / MultiFile share one implementation with no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for that arm to emit) |
| `yaoxiang check` (multiple files)   | No proof_execution                          | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| LSP (in-project)                    | No proof_execution                          | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Z3 not installed + single file      | `predicate.rs:35` **panics**                | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see Compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Z3 not installed + multiple files   | Silently skipped                            | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

The **only intentional behavior break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This conforms to RFC-027 §8's “do not bind to a specific solver” positioning and to the
contract already declared in `backend.rs:60` (“callers should degrade conservently”)—the current
implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
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
| `src/frontend/pipeline.rs`                         | `275-278`                                       | Move dead-code analysis out and make it the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                         | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                         | `381-389`                                       | Move monomorphization out and make it the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `450`                                           | Change `check_source_in_project` to use Driver                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                       | Drive the choice between `check_module` / `check_module_collect_all` in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                        |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                       | Delete the manual stage sequence in `run_diagnostics` and call Driver                                                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                       | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                       | Delete `check_single_file`                                                                                                                                                                                                                                                                                                                                                      |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                         | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent discard point**)                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                          | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; delete “a lower layer failing means upper layers do not run” (there is no short-circuit)                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                         | **Under D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure interface; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared interface. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 takes precedence over the original wording |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production logic of the `Unproven` branches (currently split among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem lies in the consumer side, not the producer side**, and changing the producer side
would conceal the architectural defect; `Cargo.toml`; all `#[cfg(target_arch = "wasm32")]` branches
(wasm-branch reachability determination is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Disposition                                                                                                                                                                 |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | `test_multifile_proof_obligation_not_dropped` is written **red first**, then fixed; release in batches, with the expected diagnostic-set change documented in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that compiled successfully now produce warnings     | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                              |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: diagnostic set grows                                         | First W, then E; split into two steps under S2/S4                                                                                                                           |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no longer crashes                                         | No break                                                                                                                                                                    |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not pass through proof_execution                                                                                                                         |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | Synchronize any external dependencies on the `pub` API surface; all in-repository consumers are listed in the change list                                                   |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contract and unified
Driver)** in RFC-039's global stage sequence. The following S1–S5 are the implementation order
within this document. **Every step's acceptance criteria reference category C2 of the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration changes: identical
**diagnostic sets** across entry points + identical corpus behavior).

### S1: Establish the criteria (red first)

**A prerequisite that cannot be skipped.** The defect criterion must first be written as a failing
test.

| Deliverable                                                                                                                                                      | Acceptance                                                                                                      |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red**. If it unexpectedly passes, the defect analysis in this document must be reviewed again         |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                           |
| Skeleton for `src/driver/tests/obligations.rs` (initially `#[ignore]`)                                                                                           | Marked `#[ignore]`; turned green in S4 (red-state record: removing the sole settlement point yields `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                                | Acceptance (C2)                                                                                  |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs`   | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array  |
| Change `pipeline.rs:141-227` to invoke Driver              | **The single-file path's diagnostic set and exit code are byte-for-byte identical**              |
| Differential test across the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. The
acceptance criterion is C1/C2-level zero-diff.

**Rollback point**: revert a single commit; `pipeline.rs` returns to its original state.

### S3: Merge the orchestrator's four entry points + fix the defects

| Deliverable                                                                                                                         | Acceptance (C2)                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Change all of `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are normalized according to `Aggregation`   |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                            | `yaoxiang check` behaves consistently inside and outside a project                       |
| Delete the LSP manual stage sequence                                                                                                | LSP and CLI produce the same diagnostic set for the same file                            |
| **Turn `test_multifile_proof_obligation_not_dropped` green**                                                                        | The defect is fixed                                                                      |
| Fix the ownership-layer `Unproven` empty match arm (originally `checker.rs:1313`)                                                   | `test_no_silent_pass_on_unproven`: `Unproven` always produces a diagnostic in every mode |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                          | Removing Z3 no longer causes a panic                                                     |

**Rollback point**: the defect fix and structural merge must be **two separate commits**. If the
structural merge causes problems, only the structural commit can be rolled back while retaining the
defect-fix commit—in that case, `test_multifile_proof_obligation_not_dropped` remains green; the
reverse (retaining the structure and rolling back the fix) turns it red and is an unacceptable
intermediate state that must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                          |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implementation form in the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                        |
| Upgrade obligation diagnostics from W to E   | After the full-corpus differential, manually review every new E                                                                                     |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration option;
switching between W/E does not require structural changes.

### S5: Finish the proof layer and wasm

| Deliverable                                                                                                                                                                                                                                            | Acceptance                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                       | The documentation corresponds one by one to the call sites in `checker.rs`                                                                                       |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes固化 across compilations through the process-wide cache and pollutes shared state between cargo test threads) | Production-path calls to `default_solver()` fall to zero (tests only); cache hit rate is observable across the three consumers (the counter landed in 89576fafd) |
| The `TypeChecker::ownership_inputs` criterion point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`) emits a warning when degraded                                                                                               | There is a diagnostic when there is no `body_checker`                                                                                                            |
| Reachability determination for the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                   | Submit the conclusion to the “F2 determination” subsection of `06-cleanup-inventory.md`; this document only registers the need for the determination             |

**Note**: correcting the layer order changes the diagnostic set and may expose many previously
silent `Unproven` results. **This item should proceed independently of S1–S4** and must not be mixed
into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time exhaustively enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                        | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is isomorphic to the opcode-table mechanism already proven in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that require human reasoning                                                                                                                                       |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement, together with the **static** gate `scripts/ci/check-obligations.py`; **final severity is E** (user ruling A on 2026-10-13: same-code elevation to E, code number unchanged; **implementation form in the §2 revision note**: the ledger does not duplicate the field structure; the red list—0 entries after cleanup—retains its mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not one bug: at least 2 instances of the same class of hidden risk have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. The two-step W→E process keeps every step's C2 criterion usable—now that the obligation list has been cleaned to 13 entries and the red list reduced to zero, healthy execution emits nothing, so the final state directly uses E (see parts seven and eight of the §2 revision note) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer) and do **not** change the production logic of the `Unproven` branches (currently three in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                    | The problem lies in the consumer side, not the producer side. Changing the producer side would disguise the architectural defect as “the logic has been fixed,” while the logic of these three branches is itself correct                                                                                                                                                                                                                                                                          |

This design additionally eliminates two classes of cracks:

- **The crack between “a documentation promise” and “code behavior.”** The “silent pass must not be
  resurrected” promise in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, fulfilled by `assert_drained()`.
- **The crack of “an array entry was forgotten.”** Currently five functions each manually write
  their own invocation sequence; after unification there is one stage table, and a missing new stage
  causes a compile-time error.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (the two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type continuity**; they are configuration questions of
  “should this analysis run?”; the chain also cannot express `Aggregation`, and would merely move
  the choice between `check_module` / `check_module_collect_all` from the function layer to the type
  layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this eliminates
  compile-time exhaustiveness: after adding a stage, `dispatch` no longer fails to compile,
  replacing “five functions each manually write their invocation sequence” with “an array entry was
  forgotten,” which is precisely the cause of the current 11 inconsistencies. This repository
  already contains a same-shaped counterexample: `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader`, described under `docs/src/dev/design/check/`, have zero
  implementations.
- **Fix only the bugs without changing the structure**—this can fix the 2 known bugs, but covers
  only 1 of the obligation fields; `Stage` remains scattered across 5 functions, and the next
  divergence will continue to grow from here. **It must be done first** (as part of S1/S3), because
  structural refactoring requires a known red test proving that the criterion is effective.
- **Make `proof_calls` public and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This looks for the bug at the
  wrong layer: the bug is in the orchestration layer, so detection must also occur there.

## Known Limitations and Risks

- **Slimming `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into “a Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of refactoring that “centralizes
  wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”—they are real bugs that
  simply were never reported. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual field-by-field judgment.** In the
  obligation fields, “no consumer” for some fields (such as `module_namespaces`) may mean that
  consumption is not needed by design and should not be reported. S4 requires a field-by-field
  review rather than a one-size-fits-all decision.
- **The `Program` abstraction may be premature.** If the stage sets of certain entry points remain
  unstable for a long time, `stages()` may degenerate into “a different array is passed on every
  invocation,” and the contract constraint will fail. **Mitigation: `test_program_stage_coverage`
  asserts that `stages()` can only come from the 6 predefined combinations.**
- **Criteria dependency**: the defect-fix criteria for S1/S3 depend on the defect test in the
  [equivalence-criteria document](07-equivalence-oracle.md) being red first. **If that document has
  not first established a multi-file corpus layer, S1 cannot be accepted**—because all 293 existing
  corpus entries use the single-file path and provide zero coverage for this class of defect. This
  document does not cover the IR verifier (`verify_loose`); that prerequisite work is outside its
  scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse of
  the declared order). This is a diagnostic-set-change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is not settled.** `backend.rs:13-19`
  already states that `Solver` is `Send` but not `Sync`, and `Z3Backend`'s cache is a `RefCell`;
  after changing it to a singleton shared across compilation units, all access paths must be
  confirmed to go through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependent defects** (such as the actual order
  dependency between termination and ownership). It should be considered only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been ruled on.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **No pending items remain in this document.**

## See Also

- [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three criterion layers, the
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker/refinement.rs` — Explicit declaration that “silent pass must
  not be resurrected” (originally `checker.rs:5165-5169`)
- `src/frontend/core/typecheck/layers/README.md` — Layer invocation table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for hard failure
  during SMT initialization
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration that “callers should
  degrade conservently” (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream code must
  consume (after the 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's own
  admission that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus's own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground's
  fourth compilation caller (the single-file path, so it executes proof_execution)
- `wasm/Cargo.toml:10-18` — the cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Error-code build-time gate, an example of this project's enforcement mechanism
