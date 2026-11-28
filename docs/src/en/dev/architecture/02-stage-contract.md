# Compilation-Stage Contract and Obligation Ledger

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criterion tiers, and execution-stage order are defined in the main
> RFC-039 text; the role of each companion document is described in the [directory index](index.md).

## Positioning and Scope

This document is the implementation blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **the current answer to “which compilation stages ran, which did not, and why?” is
manually wired across six paths and ten entry functions, with no single place able to provide that
answer.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being broken by the architecture rather than by the
> logic.**

The original comment in `revalidate_refined` in `checker/refinement.rs` states (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile error, no degradation, no silent
pass… the silent pass must not be resurrected.” However, this invariant holds only on the
**single-file path**: proof-function constraints of the form `x: Sorted(3)` silently pass on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed,
and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustive, not registered at runtime) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, plus the specific conversion of each of the ten entry
  functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- Every affected file and line range, compatibility impact, and implementation order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-bounce-back gates → `01-routing.md`
- Equivalence criteria (C1–C6 tiers and three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- The global P1–P10 execution order and G1–G10 acceptance gates → main RFC-039 text

### Division of Responsibility with RFC-039

RFC-039 defines **why** to refactor and **in what order**; this document defines the concrete shape
of L1, the file-by-file change list, and the implementation stages within this document
(corresponding to P3 “Fix correctness gaps” and P4 “Stage contract and unified Driver” in RFC-039's
global sequence). If this document conflicts with RFC-039, RFC-039 takes precedence.

The equivalence criteria are applied under **C2 (orchestration changes)** in the
[equivalence criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> This section contains **verified facts only**, each with a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect where one mistake silently affects the entire repository

This differs in nature from the other two defect classes, module boundaries and test wiring:

| Defect                  | Typical manifestation                    | Is there a signal?   |
| ----------------------- | ---------------------------------------- | -------------------- |
| Lexical/syntactic error | Incorrect source code                    | Yes, a diagnostic    |
| Type mismatch           | Incorrect type                           | Yes, a diagnostic    |
| Missing stage wiring    | A stage is never invoked                 | **No signal at all** |
| Unconsumed obligation   | A field is populated but nobody reads it | **No signal at all** |

The last two share a common characteristic: **failure does not produce an error**. They therefore
cannot be solved by “writing the code more carefully” or “reviewing more rigorously”—code review can
reveal what was written, but not **what was omitted**. This is precisely the root cause identified
by RFC-039: “This project treats ‘design’ as a documentation convention rather than as an executable
constraint.”

### Existing enforcement mechanisms in this repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`. Every entry is compared
  against the RFC-013 code table, and any discrepancy directly triggers `panic!` and rejects the
  build.
- `src/package/` has a high test density (**76 files / 13,012 lines**, of which 6,227 lines, or
  47.9%, are under `tests/`). Every module's test subtree is explicitly wired, making this the most
  completely wired test area in the repository and a useful model for a stage-layer gate.

**The design capability already exists. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation route                                                                  |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages invoked directly                                                       |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file type checking, collecting all diagnostics                                 |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project uses 3c; otherwise manually lex→parse                                   |
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

### 11 stage-coverage inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point does not perform the activity; it means that no code in that entry point performs it**—which
is the essence of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                          | `check_project`                                                       | `check_source_in_project` (LSP)    | `compile_embedded_module`               |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------- | --------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                                | **No**                             | **No**                                  |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, no configuration gate)                     | **No**                             | **No**                                  |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                         | **No**                             | **No**                                  |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                            | Yes `350`                                                             | **No**                             | **No**                                  |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                       | **No**                             | **No**                                  |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | Not applicable                                                        | Not applicable                     | **No**                                  |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting everything | `check_module_collect_all` (`493`) | `check_module` (`1386`)                 |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                          | `discover` (`454`)                 | Not applicable                          |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                       | **No**                             | **No**                                  |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                        | **No**                                                                | **No**                             | **No**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                                | **No**                             | Yes (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. The current differences after P3
> are:
>
> - **Row 1 has been fixed**: all five proof-execution entry points use the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entry points); the unique-consumer
>   defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and the dead-code-family warnings. The W-code sets of the four
>   paths—single-file run, multi-file run, single-file check, and multi-file check—are normalized
>   around the shared implementation on the `dead_code_analysis` arm as the single source of truth.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk not yet proven to be a defect).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “decision #434” previously had no registration in the docs; it has
>   now been registered as RFC-039 **D57**.
> - **Asymmetry in embedded std inclusion** (a new fact outside the table, added on 2026-10-09): the
>   single-file path unconditionally injects `std.list` through `merge_embedded_std_ir` (required
>   for desugaring `for` loops; hard switch under #117), whereas multi-file `discover` previously
>   recognized only explicit `use` statements. A `for` loop in a project therefore compiled
>   successfully but failed at runtime with E6006 (confirmed by a probe). This has been fixed and
>   WBS 4.10.2 closed. The related “multi-file lacks a single-file step” parse hard-abort quirk was
>   registered as WBS 4.10.1 and **has also been fixed** (immediately after 4.2.2: the Check path
>   degraded to per-file collection; this audit method examines the dimension of “stage coverage and
>   field consumption,” so this difference in compilation-unit membership is a separate, previously
>   uncaught dimension).
>
> - **Embedded std registration-surface coverage** (discovered during 4.2.5 implementation, now
>   fixed): the Registry arm repeatedly harvested registrations for embedded std units using
>   `extract_module_info`, overwriting the native half registered by `with_std()` through the
>   “native + yx surface merge” (causing `result.is_err` and similar entries to disappear and
>   `std.test` to report a false E1043). This audit exposed a third previously uncaught dimension:
>   merge semantics for two registration sources under the same module key. The single-file path
>   forms its registry exclusively through one `with_std()` call, while the multi-file/Check
>   Registry arm inserts unit by unit and therefore has broader coverage.

**Two points require precise wording; otherwise the conclusion will be wrong:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` never reports W1001/W1002/W1003 on the
  **multi-file** path (`run_project` → `compile_project`); it does report them on the
  **single-file** path (`run_file` → `Pipeline::run`). The reason lies in `compile_project`: it
  places the complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function
  has **no read site for `result.warnings` anywhere**. The only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 also comes from different sources**: `check_project:385-395`
  uses `surfaces.bins` (the manifest-declared surface), whereas `compile_project` uses `is_bin_role`
  (`250-252`, determining only whether a manifest exists). The two functions give different answers
  to “which files must define `main`?”

### Correctness gap: proof obligations are silently discarded (complete evidence chain)

**This is the core of the document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for obligations.**
`src/frontend/core/sema/layers/predicate.rs:232-239` is the **only** place in the entire repository
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

The meaning is: “The predicate arguments of this refinement constraint are compile-time literals, so
the proof function must actually be executed to determine the result.”

**Step 2 — The consumption point is inside the checker, but it only produces obligations and never
consumes them.** `src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                          | Behavior                |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → infers a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Infers a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Infers a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Emits no diagnostic** |

The original comment in `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` …
> `This branch therefore becomes a real line of defense rather than a forward-looking fallback: the silent pass must not be resurrected.`

In other words, the author knew that “emitting no diagnostic” was a defect and explicitly classified
it as a **known intermediate state with nowhere to go**, assuming that “someone would eventually
read `proof_calls`.”

**Step 3 — The field is indeed written into the result.** `check_module_impl` declares a local
`proof_calls` and collects into it (originally `checker.rs:1234-1235`), then writes it into
`TypeCheckResult` with `proof_calls, // Phase 2.5 proof-function obligation pre-registration`
(originally `:1439`). The field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** In production code,
`TypeCheckResult.proof_calls` (`types.rs:29`) is read in exactly one place:
`src/frontend/pipeline.rs:187`, which passes it at `189`:

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other occurrences of the `proof_calls` identifier fall into three categories and are not consumers
of this field: the production-side collection sites in `checker.rs` and `checker/refinement.rs`
(originally `:1234/4564/5494`); the same-named field of `ProofResult` in `verdict.rs:61`; and
`tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points passes through that layer.** The four entry
points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)—**all invoke
`TypeChecker::check_module` directly** (`124` / `493` / `1386`) and never call `pipeline.rs`. They
therefore cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also take the discard
path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible
for compiling embedded std. This means that **the embedded std module's own proof obligations are
likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostic** on the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (additional verification) — There is a second silent-discard point within the layer.** When
`check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`):

```rust
// Former checker.rs:1313 (now within check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` result is swallowed by an empty match arm, with no diagnostic and
no accounting.** This directly contradicts the promise at the `5164` branch that “the silent pass
must not be resurrected,” and is **independent of the orchestrator problem**—even if the entry-point
layer is completely fixed, this point will remain silent. (Its handling should occur in the same
batch as the obligation ledger because it belongs to the same class of issue.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have no
coverage.

RFC-027's three unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty. For example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test files' own documentation comments already state this.
`rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer.`

**This is the exact shape of the problem: the tests verify that “the obligation was populated,”
while the bug is “the consumer did not read it.”** A test that exercises only the producer and not
the consumer is inherently immune to this class of defect.

### Stronger finding: the primary corpus for the equivalence criteria uses the single-file path

The [equivalence criteria document](07-equivalence-oracle.md) defines end-to-end differential
testing over the 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and
the primary acceptance mechanism for C2 stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero glob matches).
- Consequently, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone`
  branch for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof execution**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx`
demonstrates this directly. The file is marked `// expect: compile-error E4018` on line 15, and its
header comment on line 7 says “Status: ❌ should be rejected at compile time”—**it passes precisely
because it takes the only path that executes proof functions**.

**Conclusion: the third criterion layer needs a multi-file corpus layer; otherwise it cannot serve
as the acceptance mechanism for this document.** See implementation point S1.

### Obligation fields: 15 “production implies contract” fields with no enforcement mechanism

`TypeCheckResult` in `src/frontend/core/sema/types.rs:16-98` contains **15 fields** whose
documentation comments explicitly state “produced by stage X → consumed by stage Y”; in other words,
they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: “Definition line” means the field's **declaration line** in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of its documentation comment—the comment block
> may span multiple lines, and only the declaration line provides a unique anchor. The **struct
> range** in the sentence above (`src/frontend/core/sema/types.rs:16-98`) moves with the struct's
> line number and is mechanically reconciled by gate **F5** (an incorrect value turns the gate red);
> the definition-line column is reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by `PlanId`**:
> read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-completed marker `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completed marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation | | `record_models` | `79` | typecheck (once at declaration) → IR
> generation (quantifier propagation; from B.2 onward, witness field `TypeDeclField.quantity` and
> constructor-argument/default-value population both read this table) | | `expr_result_types` | `97`
> | typecheck → IR generation (**keyed by `PlanId`**—call-site identity: produced in the Call arm of
> `inference/expressions/expr.rs`, consumed by `lower/query.rs::expr_value_type`; export points
> converge, and unconverged entries are not exported) |

**Reclassification record (3.4.9, user decision A on 2026-10-13):** Original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design it
is consumed within a phase and has no cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (active)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; the `src/driver` side contains only the ledger's own registration and documentation
  (removed in this round).
- **Disposition**: delete the documentation comment's claim that the field is a “mirror for
  consumption by the middle layer/LSP,” but retain the field and its in-phase consumption. The field
  is registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP
  later genuinely needs the table, it must be registered through a new 4.6.x entry; **no consumer
  should be invented in this round**.

**These fields cross layers keyed by `PlanId` (statement/call-site structural identity), `Span`, or
name tables.** The equivalence criteria document already points out that mismatches in span-keyed
contracts “fail silently, with no error” (`ReleasePlan` is a typical example: if the two sides
compute spans differently in any respect, the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, while
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**. The consumer
  is in a downstream module, and that downstream module lies on every entry point's required path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to a
  **different entry-point implementation**. The orchestrator's four entry points never pass through
  the `pipeline.rs` layer.

**Verified fact:** The repository currently has **no mechanism whatsoever** that guarantees these 15
fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride with IR
generation.

### Registration of non-obligation fields: channel, in-phase, and in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (9 entries):** Every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation fields”
table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                                                                                                                                                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (zero hits in `src/middle`)                                                                                                                                                                                                                                                                                                                        |
| `diagnostics`             | Error diagnostic channel (an existing channel, not “production implies contract”)                                                                                                                                                                                                                                                                                         |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but the documentation comment does not declare a cross-stage contract ⇒ not consolidated                                                                                                                                                                                                                                     |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings`—no declared contract                                                                                                                                                                                                                                                                        |
| `semantic_db`             | Consumer is LSP (from 4.6.1, emitted through the `DriverOutcome.semantic_db` channel at src/lsp/server.rs:482), not a compilation-stage obligation                                                                                                                                                                                                                        |
| `trait_table`             | Cross-stage reads at ir_gen.rs:4336/:6867, but the documentation comment declares no contract                                                                                                                                                                                                                                                                             |
| `warnings`                | Warning diagnostic channel (same category as `diagnostics`)                                                                                                                                                                                                                                                                                                               |
| `interface_impl_registry` | In-tree fact (3.4.9 user decision A; in-phase consumption sites are listed in the “§Obligation fields” reclassification record)                                                                                                                                                                                                                                           |
| `method_def_ir_names`     | Cross-stage mapping (typecheck produces mixed names keyed by the method-declaration statement's `PlanId` → IR generation reads only the names, WBS 7c.3). A miss fails loudly with E3005, with no silent surface; consumption granularity is per declaration rather than a non-empty unit, so the ledger's “unit produced means it must be delivered” rule does not apply |

**Why not simply “wire up the consumer”**: Giving these fields downstream consumers would invent
spurious obligations and disguise fields that “do not require cross-stage consumption by design” as
contract defects. The correct closure mechanism is **reverse containment**—the `TypeCheckResult`
field set = the obligation inventory ∪ the non-obligation registrations. Gate criterion 1 in
`scripts/ci/check-obligations.py` enforces this mechanically: adding a field without registering it,
removing a field without synchronizing the registries, omitting a reason for a non-obligation
registration, or overlapping the two tables all turn the gate red immediately.

### Four additional contract defects in the proof layer

The following findings come from an analysis of the proof layer. They are independent of the
entry-point split, but all belong to the class of “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation; there is no enforcement mechanism, and the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual invocation order of
`checker.rs::check_module_impl` (locations are anchored by function name/driver site, with line
numbers taken from the current tree):

| Actual order | Call site                                                                                                                                                                                                                                    | Corresponding layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` has its own E2030 semantics; `judge_refined_obligation` is the sole decision unit shared by call-site obligations and return-position obligations) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                              | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                                 | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                                         | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                                                                                                       | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comment and do not correspond to the invocation order. The README has no
“number → invocation timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds termination errors one by one and continues downward, so “a
lower layer failing means an upper layer does not run” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** The hard-failure panic was removed by P3's 3.3.1 (the SOLVER slot
> was made optional; absence conservatively degrades to `SMTResult::Unknown`); “silent skip (no
> injection)” was given signal W1081 by 3.4.2. Unification of the three forms (singletonization) is
> to be performed under **RFC-039 D58**, at the actual fix location `proof/smt/backend.rs` (adding a
> process-wide shared singleton + a `with_shared_solver` closure entry point). The wording in the
> change list for 02, `default_solver() → &'static`, is subject to D58 (a bare `&'static` reference
> is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                   | Acquisition strategy                                     | When the solver is unavailable                               |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected through `check_module_impl`, originally `checker.rs:1265-1268`) | Injected at construction time with `with_solver_owned`   | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver's capabilities’.” In contrast, the contract documentation at `backend.rs:60` says:
“`None`: backend unavailable… **the caller should degrade conservatively**.” **Two philosophies
coexist in the same codebase, and the hard-failure path panics rather than returning an error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge-cut decisions. Meanwhile,
`src/frontend/core/sema/proof/smt/backend.rs:67-72`:

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
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the documentation comment at
`z3_backend.rs:17` states that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache can never be hit in this invocation mode**.

(For comparison, the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. It is the same
backend with two lifetime strategies.)

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
table** and proceeds normally—ownership analysis degrades to “nothing conflicts,” **without any
warning**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm status: the shim crate carries the target, and branches in 29 files are active

The wasm target **has already been built and is built in CI**. The `cdylib` is not in the main crate
but in a separate shim crate.

| Fact                                                       | Evidence                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The main crate contains only `rlib`                        | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                         |
| **The shim crate provides `cdylib` + `wasm-bindgen`**      | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                         |
| The shim depends on the main crate (`rlib`) as its library | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| The main crate has a wasm-target dependency section        | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                           |
| The shim directory is excluded from the main workspace     | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI has four wasm builds**                                | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten       | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL; degrades to a warning if missing)                                                                                                                                                                 |
| **29 files** contain the literal `wasm32`                  | **22** contain actual `#[cfg(...)]` attributes; the other 7 mention it only in documentation (see the “F2 determination” section of `06-cleanup-inventory.md`)                                                                                                                             |
| `orchestrator.rs` contains 16 occurrences                  | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                       |
| `src/lib.rs` contains 11 occurrences                       | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                             |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`)—while the shim follows a different path.

**This directly yields another fact relevant to this document:** the playground entry point
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and passes
it to `Driver::run` (JS-side entry point `run_code`, `:122`)—the **single-file path**. Therefore:

| Path consuming `proof_calls`                              | Does it execute proof functions? |
| --------------------------------------------------------- | -------------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                          |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                          |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                          |
| Multi-file `run` (`compile_project`)                      | **No**                           |
| `check` (`check_project`)                                 | **No**                           |
| In-project LSP (`check_source_in_project`)                | **No**                           |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three remain on the single-file side.** When the unified Driver is implemented,
this path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`); the implementation record appears in the
**4.2.8** row of `09-execution-wbs.md` (`wasm` now uses `ProgramKind::WasmPlayground`, `:274`).

(The cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)
are unreachable in playground scenarios—is assigned to the “F2 determination” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time, while six `not(wasm32)` arms are load-bearing in the native three-stage tables; there
are no low-risk items that can be deleted directly. The RFC-039 open question “build a wasm target,
or remove these branches” is **obsolete**: the target has already been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustive, not registered at runtime)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that “a new stage must be
handled by the orchestration layer”—exactly the mechanism already proven effective by the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // vendor consistency check          Project
    Discovery,          // file discovery                   Project
    Parsing,            // lexical + syntactic analysis     PerModule
    Registry,           // module registry construction     Project
    RoleClassification, // role classification (Script/Bin/…) Project
    Typecheck,          // type checking (incl. proof layer) PerModule
    DeadCodeAnalysis,   // dead-code-family analysis         Project
    ProofExecution,     // compile-time proof execution      PerModule
    GlobalSlotAlloc,    // global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                   PerModule
    Monomorphization,   // monomorphization                 Project
    Linking,            // cross-module linking / IR merge   Project
}

impl Stage {
    /// All stages. Adding a variant makes this array and dispatch's exhaustive match fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`:** `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module and which must run once per project” into a
type-level fact. Stages with `Project` scope are structurally guaranteed by `dispatch` to run only
once. This eliminates questions such as “should `allocate_global_slots` be called once per file?”
that currently require human reasoning in `orchestrator.rs`.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07):** Two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved **after** `IrGeneration`: monomorphization consumes the IR
>    artifact (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to the `Stage::ALL` topological order, with dead-code
>    analysis **before** proof execution. Currently, `check_project` executes proof before dead-code
>    analysis; the two have no data dependency, and the diagnostic sets are identical (C2 set
>    semantics). Only the within-file diagnostic order changes. The single-file path (dead-code
>    analysis embedded in type checking and preceding proof) already follows the `ALL` order, so
>    byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved **before** `Registry` / `RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    artifacts. The original order would force the Registry arm to “hide parsing,” making the stage
>    table lie about the data flow; it also reduced multi-file parsing from twice to once. The
>    accompanying current-state fact is recorded faithfully: multi-file parse errors were previously
>    a hard abort (the `?` propagation in `build_registry_from`, in tension with CollectAll
>    semantics, registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 when its
>    last caller was removed, and the historical account is retained here)—**4.10.1 has been fixed**
>    (user decision on 2026-10-09: rust-style collection semantics + Plan B): on the Check path, a
>    parse failure degrades to per-file diagnostic collection, and the affected file exits the
>    compilation unit (does not enter the registry; importers report E5001). The MultiFile path
>    retains a hard abort (under FailFast, a broken file cannot produce IR; this is correct
>    semantics and the ruling remains in force long term).
> 4. Check-form implementation (4.2.2, user decision on 2026-10-09) added two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events. The initial draft omitted these
>    (in the orchestrator era, these artifacts were produced by `discover_with_used` and discarded
>    by the `discover` wrapper, so the data flow was lost when it did not appear in the table). b.
>    Within-file diagnostic-order normalization was extended to the complete form of note #2: the
>    E3020 entry validation moved into the RoleClassification arm (Check has no Linking stage, so
>    the arm holding `surfaces` and the AST performs validation), and the within-file diagnostic
>    order = stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof). The
>    diagnostic set is unchanged (C2 set semantics); only the order of stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design goal:** Turn “a field was produced but nobody consumed it” from an undetectable condition
into a fact assertable at compile time or test time. RFC-039 identifies this as “the single
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
    /// Each unconsumed field produces one W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers):**

| Situation                                       | Criterion                                                  | Disposition                                                             |
| ----------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The field is `take()`n or marked consumed in `Obligations` | Pass                                                                    |
| The obligation is non-empty but has no consumer | The field is non-empty and not consumed                    | **Produce a diagnostic** (W level by default; E level in `strict` mode) |
| The obligation is empty                         | The field is empty                                         | Pass (no consumption required)                                          |

**Why begin with W rather than E:** Fixing discarded obligations will **change the diagnostic set**.
C2 requires that “the diagnostic sets of all entry points be identical.” Moving directly to E would
cause `yaoxiang check` to suddenly emit a large number of previously silent `Unproven` diagnostics.
A two-step approach—first W for observation, then E—keeps every step independently testable against
the criteria. See implementation point S4.

**`assert_drained()` has exactly one call site**: `Driver::run`, at the end of the stage table and
before producing `CompilationResult`.

**Companion static gate:** RFC-039 proposed `scripts/ci/check-obligations.py` (fail when a field
occurs at least twice but has no read in a third file). `assert_drained()` is the **runtime** gate;
the script is the **static** gate. The two are complementary, and both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 implementation):** This note records the
> **implementation shape, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a direct correspondence to code file:line locations. The implementation commit is
> `1941cc47a` (validation tree `99079bc52`). It is also the **third** of the three places carrying
> the red list's “not silent” requirement (the other two are the audit surface and the static gate).
>
> **I. Implemented artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201; field
>   name/consumer stage/red-list status), **non-obligation field registration
>   `NON_OBLIGATION_FIELDS` (:175, 9 entries with reasons)**, delivery-marker ledger
>   `DeliveryLedger`, pure settlement function `settle_obligations` (:577), external emission
>   `emit_obligation_diagnostics` (:654), non-consumer gating `consumer_visible_unit` (:538), skip
>   criterion `consumer_stage_skipped` (:549), IrGeneration delivery set `IR_GENERATION_DELIVERED`
>   (:285), and W1090 code constant `OBLIGATION_UNDRAINED_CODE`.
> - **Only settlement point**: at the end of the stage table in `Driver::run`, before
>   `into_outcome()` produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path for `Aggregation::FailFast` settles as well (the
>   ledger must balance).
> - **Seven delivery points** (one per arm; removing any one makes the runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —all four use the
>   same delivery point: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single file reads the
>   request table directly) / `:1221` (MultiFile·Check: aggregation in the Linking arm,
>   `driver/helpers.rs:295` reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by tests.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is written immediately above the entry at :26-31—the settlement
>   granularity is “unit × obligation,” with no source location that the user can point to, so no
>   dummy span is inserted). Six locale texts are provided in `locales/*.json`. **Emission level**:
>   from 4.3.2 onward it is **error** (same code promoted to E, see section three)—determined by the
>   explicit `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code number
>   remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above         | Implementation criterion                                                                            | Code                   |
> | ----------------------------------------------- | --------------------------------------------------------------------------------------------------- | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ actual read reconciled by the static gate, see section five) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see section three)         | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                    | obligations.rs:587-591 |
>
> Two **additional tiers** (user decisions on 2026-10-13, not included in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in the current `ProgramKind`'s stage table →
>    **not considered outstanding** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: The consumer stage was skipped because of `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not considered outstanding (obligations.rs:598-602). `NoObligations` does
>    **not** grant an exemption—if an obligation is non-empty while that stage declares that it has
>    none, the ledger is unbalanced and must be reported.
>
> **III. External-emission policy (decision “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: this mechanism emits only **W-level** diagnostics (W1090); there
> is **no strict switch**. **4.3.2 (final state, user decision A on 2026-10-13)**: **the same code
> is promoted to E**—the `W1090` code number remains unchanged, but the emission level is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (one fact with two code numbers would create parallel representations). **The
> promotion changes only the emission level, not the emission conditions.** This is a **D-class
> breaking change under D59** (changes externally observable behavior), faithfully registered:
> healthy programs emit nothing (all obligations have delivery markers), so existing corpora and
> user programs are unaffected (342/342 “difference: empty”); however, when a run genuinely hits an
> undelivered obligation, that diagnostic **counts toward `error_count`, renders as `error[W1090]`,
> and produces a nonzero exit code**, unlike the previous “W-level diagnostics do not block
> compilation.” The policy decoupling code family from emission level is registered in RFC-013's
> “emission channel” section (the code prefix determines numbering ownership only; the construction
> site explicitly determines severity). Mechanism facts (measured): severity comes from explicit
> `.severity()`; otherwise “a code beginning with `W` → Warning” (`codes/builder.rs:186-203`). The
> registry's `ErrorCategory` does **not** participate in severity. The exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`); **because of its explicit error severity, W1090 follows `error_count`
> (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's decision C on
> 2026-10-13; the current red list is empty, see section four), for a reproducible reason:
>
> - Decision A (emit the red list at W level) has a measured cost: `interface_impl_registry` is
>   populated from typecheck's prebuilt table and is **always non-empty**, so emitting it per
>   compilation unit produces a **constant signal** with no discriminative value, while breaking
>   three existing external contracts: “clean programs emit no diagnostics / LSP emits no
>   diagnostics / `warning_count == 0`.” A measured `cargo test --lib --all-features` turned **12
>   existing test cases red**, of which **8 are on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 are under `src/driver/tests/`). The corpus
>   baseline changed in 24/342 fixtures, and the only viable modification is “filter W1090 out of
>   zero-diagnostic assertions,” which weakens an existing criterion.
> - Decision B (downgrade to Info) would still break 9/12 list-style assertions, and Info is not
>   emitted “at the level chosen by the ruling.”
> - Decision C has been measured: existing test cases require **zero changes**, and the corpus
>   baseline has **zero changes** (342/342 “difference: empty, behavior equivalent”). Runtime W1090
>   becomes a **discriminative regression detector**—the healthy tree remains silent throughout, and
>   removing any one delivery marker makes it fire (all three delivery-point classes were tested by
>   removing markers; removing the Check-arm marker made the CLI `check` command print two
>   `warning [W1090]` diagnostics).
>
> The red list's “not silent” requirement is carried by three mechanisms: ① the
> `DriverOutcome.obligations.findings` audit surface (emitted on every run, including WBS IDs and
> evidence pointers); ② `check-obligations.py`, which prints the red list on every run (currently
> empty) and performs stale-entry detection; ③ this note.
>
> **IV. Red-list status (closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three cleanup tasks were completed on 2026-10-13:
>
> Note: the line anchors below record the file state as of **2026-10-22**. After P5 (2026-10-26)
> split the checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, the
> line numbers in `checker.rs` shifted upward, so these anchors no longer correspond to the current
> file and are retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → field deleted**. Its declared responsibility (“ir_gen generates
>   CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`), and the call site at
>   `:7496` replaces the entire Call with `CreateVariant`. The function's documentation explicitly
>   states that “span keys are unreliable across systems (the two ends have different semantics for
>   a Call node's span), so detection is performed in place”—**wiring up a consumer would
>   reintroduce the already-fixed span-mismatch defect**. Removed surfaces: the `types.rs` field,
>   collection at `checker.rs:1489-1493/:1565`, the collection chain in `inference/expressions.rs`
>   and `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`;
>   `infer_variant_ctor_call` type inference and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → cross-stage mirror deleted** (the in-phase candidate-name
>   table and `overload_resolutions` remain). All read/write sites: written in phase
>   (`environment.rs:205-219`, registered alongside `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550`, decision point; the sole match is written to `overload_resolutions`
>   at `:4550`); what was deleted is the **zero-reader mirror** injected into `TypeCheckResult` at
>   `checker.rs:1558`. The naming fact still has a single expression: the candidate-name table (in
>   phase) + span-keyed decision (cross-stage, `ir_gen.rs:380` construction, `:7632/:7705`
>   consumption); the definition-side mixed name is computed by `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user decision A, detailed
>   in the §Obligation fields table's reclassification record). What was deleted is the **claim** in
>   the documentation comment that the field is a “mirror for consumption by the middle layer/LSP”
>   (`types.rs:47-48`); the field and its in-phase consumption remain (`operator_interfaces.rs:623`
>   operator lookup, seven constraint-solving lookups in `inference/expressions.rs`, and the
>   read-only accessor at `checker.rs:214-217`). Direct proof of zero cross-stage reads:
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the table's contents are an environment mirror populated from typecheck's prebuilt table (always
>   non-empty). If LSP genuinely needs it, it must follow a new 4.6.x registration; giving a
>   consumer to a field that “does not require cross-stage consumption by design” would introduce a
>   false obligation into the ledger.
>
> After cleanup, **the field inventory is closed by “obligation inventory ∪ non-obligation field
> registration (`NON_OBLIGATION_FIELDS`, with reasons)”**—the reverse-containment end of gate
> criterion 1 protects this end to end (adding a field without registration or removing a field
> without synchronization turns the gate red immediately). The red-list **mechanism** is retained:
> the gate's criterion 4 stale-entry detection and the `--selftest` `stale-red-list` fixture
> continue to cover its detection surface; any future registration must go through an explicit
> decision.
>
> In conjunction with cleanup, the stated count changed: obligation inventory 16 → **13**
> (`INVENTORY` / `Obligation::ALL` / this document's §Obligation fields table); the gate's
> `--selftest` now covers seven fixture categories (adding “unregistered field,” “stale anchor,” and
> “empty red list”), as shown in
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Actual criteria of the static gate (9.2)** (three additional hard criteria beyond the sketch)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document's “§Obligation
>    fields” table (including the table's declared count); any drift turns the gate red. The
>    `TypeCheckResult` field set must also be **exactly equal** to “obligation inventory ∪
>    non-obligation registrations (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field without
>    registering it, removing a field without synchronization, omitting a reason for a
>    non-obligation registration, or overlapping the two tables all turn the gate red immediately.
> 2. **Read evidence**: every non-red-list obligation must have a genuine read in the **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings do not count). This turns
>    “delivery marker ↔ actual downstream read” into a hard criterion (the main risk in mechanism A
>    is precisely drift between the two).
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    red-list entries, conversely, must not be marked.
> 4. **Red-list stale-entry detection**: red-list fields must have zero genuine reads (the
>    appearance of a consumer turns the gate red), and each entry must include a WBS ID and evidence
>    pointer; there is also `--selftest`: built-in counterexample/positive fixtures, with
>    counterexamples required to fail and positive cases required to pass (proving that the detector
>    is not a no-op).
>
> **VI. Deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields**: the obligation
>    data remains held by `TypeCheckResult`, and the ledger records only the “inventory + delivery
>    markers + settlement conclusion.” Copying the data would create two sources of truth (the
>    individual fields are produced by typecheck's checker and ownership layer, respectively) and
>    guarantee drift; the cross-layer data plane should not be duplicated into the orchestration
>    layer either.
> 2. The sketch's `take()` / “mark consumed” **was not adopted**. It was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The obligation and its artifact are the
>    same data, so `take()` would remove what downstream code needs; moreover, 11 IrGeneration
>    obligations take the form of “pass the entire `&TypeCheckResult` to ir_gen,” which cannot be
>    taken field by field.
> 3. The sketch's signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written to `DriverOutcome.obligations`,
>    and diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel;
>    other forms are routed **by severity** (from 4.3.2 onward W1090 is E-level)—the single-file
>    path uses `PipelineError::TypeCheck` (the obligations.rs:654 emission arm), while
>    multi-file/embedded forms record an `OrchestratorError` failure separately (otherwise
>    `result.errors` has no consumer). The reason is that returning `Vec<Diagnostic>` would lose
>    form-based routing—Check diagnostics must retain per-file attribution.
> 4. The first tier of the three-tier table changed from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is delegated to the static gate
>    (criterion 2 in section five).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**.
>    Instead, **4.3.2 promotes the same code to E** (promotion occurs even without a strict switch,
>    because zero emission in the healthy state is already sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field occurs at least twice but has no read in a
>    third file” to the four criteria in section five.
> 7. The two additional tiers not included in the sketch (S1, skip exemption) appear in section two.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table was actually
>    implemented as **20 test cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]`
>    skeleton → turned green; red-state record: after removing the sole settlement point, `pairs: 0`
>    and 6 failures). The S1/S4 rows have been synchronized with the actual file.
>
> **VII. 4.3.2 implementation record (severity + gate hardening)**
>
> - **Per-field review inputs**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   drained in this run, available per run) and the five counters (the conservation identity in
>   section two); combined with the audit surface's `findings`, these establish whether a field has
>   ever been consumed by any entry point.
> - **Severity: same-code promotion to E is implemented** (user decision A on 2026-10-13; see the
>   final-state registration in section three and the D59 D-class breaking-change declaration).
>   Implementation: `to_diagnostic` explicitly calls `.severity(Severity::Error)`; emission arms
>   route by severity to the error channel. Mutation evidence: removing any one delivery marker
>   causes that form's compilation to fail, and `check` returns a nonzero exit code
>   (`error [W1090]`); after restoration, the healthy state emits nothing (corpus 342/342
>   unchanged). The locale text now states in all six languages that “the code belongs to the W
>   family, but its emission level is error.”
> - **Gate hardening items (F1/F2, both implemented)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence**, validating “the file is locatable (uniquely) +
>     the line number is in range + the symbol immediately after the anchor actually appears in the
>     anchored file”; an anchor to a nonexistent symbol turns the gate red. Built-in counterexample
>     fixture `tests/baselines/obligations-gate/stale-anchor/` (one case each for a nonexistent
>     symbol and an out-of-range line). Note: the red list is currently empty, so the criterion's
>     effective landing point is **non-obligation registration** and future evidence pointers (the
>     same code path).
>   - **F2/R3 reverse containment**: the gate prints “number of `TypeCheckResult` fields − number of
>     obligations − number of non-obligation registrations = difference”; a nonzero difference turns
>     the gate red; built-in counterexample fixture `unregistered-field/` (a new struct field with
>     no registration).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable through
>   real entry points (a unit whose typecheck failed does not retain a `type_result`, so the
>   obligation does not exist); this tier's semantics are pinned by a synthetic `State` test, and
>   real end-to-end coverage only includes `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 secondary-task closure (red list zero + obligation diagnostics at final E severity)**
>
> **8.1 Three-round implementation timeline** (each item corresponds to code file:line; this round
> records conclusions only and does not change code)
>
> | Round               | Conclusion                                                                                                                                                                                  | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
> | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger launch | `State::assert_drained()` as the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                 | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                |
> | 3.4.4 cleanup       | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                         | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                            |
> | 3.4.5 cleanup       | `method_overload_ir_names` cross-stage mirror deleted (in-phase candidate-name table + `overload_resolutions` retained)                                                                     | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.9 cleanup       | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirror for middle/LSP consumption” declaration, retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten by decision A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, seven sites in `src/frontend/core/sema/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 severity      | **Same-code promotion to E**: explicit `.severity(Severity::Error)`, code remains `W1090`; emission routes by severity to the error channel (first failure takes priority)                  | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/severity decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                             |
>
> **8.2 Closure conclusions**
>
> - **Red list zero**: the number of entries in `INVENTORY` (`src/driver/obligations.rs:201`) whose
>   `red_list` is `Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0).
>   The mechanism is **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the gate's
>   criterion 4 stale-entry detection, and the `stale-red-list` fixture; the gate prints “red list 0
>   entries—cleanup complete.”
> - **Final obligation-diagnostic severity: E**: the **code number and W-family membership of
>   `W1090` remain unchanged**, but the emission level is `error` (final D5 state, user decision A
>   on 2026-10-13). This is a **D-class breaking change under D59** (changes externally observable
>   behavior), registered in both section three of this note and RFC-013's “emission channel”
>   section; **zero emission in the healthy state ⇒ no impact on existing corpora or user programs**
>   (corpus 342/342 “difference: empty,” no IR snapshot drift; t11 independent validation and t12
>   review each rechecked it).
> - **Closed field inventory**: the `TypeCheckResult` field set = obligation inventory ∪
>   non-obligation registration (8 entries, `src/driver/obligations.rs:150`, each with a measured
>   consumption-surface reason); gate F2 explicitly prints the difference (currently
>   `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: all hardening items registered in this note have been
>   **implemented** (F1/F2/F4/F5, see sections five, seven, and eight); the only two unclosed items
>   are the **known boundary** in 8.4 and t7 F5, which has been converted into an issue; neither is
>   an unimplemented criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/review round**
>
> | Source        | Finding                                                                                                                                                                    | Status                                                                                                                                                                                                                                                            |
> | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is a plain-text pointer, so it can become silently stale when the replacement consumer is removed                                                        | **Implemented**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); counterexample fixture `tests/baselines/obligations-gate/stale-anchor/`                                                       |
> | t3 F2 / t4 R3 | Gate criterion 1 lacked reverse containment, so adding a field without registration silently passed                                                                        | **Implemented**: F2 explicitly prints the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; counterexample fixture `unregistered-field/`                                                                                |
> | t4 R4         | The W-only versus promoted-to-E policy was not registered                                                                                                                  | **Implemented**: sections three and seven of this note + RFC-013 “emission channel” + W1090 help and `error_output` examples in six `locales/*.json` files                                                                                                        |
> | t7 F5         | Dead-code analysis miscounts overloaded-method references (a legal overload is falsely reported as `W1005`)                                                                | **Converted to issue `#450`**, assigned to WBS **4.9 Correct reference-pool implementation** (registration in the 4.9 row of `docs/src/dev/architecture/09-execution-wbs.md`)—not part of this secondary task                                                     |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor commitments in module-header prose comments                                                                       | **Known boundary** (see 8.4)                                                                                                                                                                                                                                      |
> | t11 F8        | Validator-side self-record: mutation-driven work once wrote to the main repository (immediately removed with `git restore`; affected round output was discarded and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                      |
> | t11 F9        | The main repository's intermediate state (unstaged files made this document's table header inconsistent with `INVENTORY`) triggered a red gate                             | **Informational / resolved**: the frozen commit passes self-consistently; “committing an intermediate state directly turns the gate red” is precisely the intent of F2/F4                                                                                         |
> | t12 F12-1     | This document gave the struct range as `16-73`, while the actual range was `16-76` (three comment lines added in t9 without mechanical reconciliation)                     | **Implemented + mechanized**: range corrected to `16-76`, and gate **F5** added (range anchor; an incorrect value turns the gate red); related figures were grep-aligned (`17` → 20 test cases, `16 span-keyed fields` → 13)                                      |
> | t12 F12-2     | Settlement in the multi-file form once overwrote the first failure (the type error in `zzz.yx` was masked by W1090)                                                        | **Fixed** (the captain amended the commit message): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; the regression is caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` commitments in module-header prose comments are outside the scanning surface** (no
>   current registered entry is affected). If an anchor commitment is added to a module header in
>   the future, it must either be **covered by human review** or the scanning surface of F1 must be
>   expanded; neither option may be left implicitly unowned.
> - **The gate's static boundary**: set-level validation of delivery markers remains static
>   (criterion 3), while removal of an individual arm is covered by **runtime settlement**— the
>   static + runtime two-layer structure is the intended design, not a gap (bidirectional validation
>   through t11's M11A/M11B/M11C mutations).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 implementation of the cross-layer PlanId
> contract):** Of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) were changed to **structural-identity keys** (`PlanId`) under RFC-039
> **D20**. This note records the implementation form, criteria, classification basis, and known
> boundaries; the definition lines are still reconciled by gate **F4** (the two rows above reflect
> the current code).
>
> **I. Implementation form (carrier chosen: A, `PlanId`)**
>
> The key is a **structural identity** carried with the AST node, no longer relying on the
> positional coincidence of “each side computes a span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → release-plan ownership and consumption) and `Expr::Call.plan_id` (call-site identity →
> production and consumption of overload decisions); `Expr::call` is the repository's only
> call-expression constructor, and synthetic calls outside the parser are all recorded as
> `UNPLANNED` (**not consumable**, see section two).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**the read and the accounting occur
>   in the same call**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is supplied
>   by `walk_stmt`'s set/restore operations + the CFG-node-to-statement-identity table; the Drop for
>   a scope exit is attributed to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (the table and the consumption ledger are **one
>   object**), production site `inference/expressions.rs:4550` (written by call-site `PlanId`, with
>   dispatch passing it through by destructuring `Expr::Call { plan_id }`), consumption sites
>   `ir_gen.rs:7697` (with binding) / `:7772` (without binding) via `OverloadResolutions::get` (a
>   hit is recorded).
>
> **II. Mandatory-consumption settlement (reuse the 4.3 ledger; do not create a second system)**:
> the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each table; when `generate_module_ir`
> returns successfully, the same point marks both ledgers as having “completed one full run”
> (`ir_gen.rs:1095` release plan / `:1097` overload decisions); there is a single tri-state mapping
> (`consumption_state` in `src/driver/obligations.rs`), and the sole settlement point connects one
> line for each of the two obligations—a nonempty difference produces an **E-level W1090**
> (same-code promotion to E; policy described in section three). The exemption criterion is attached
> to the **ledger's own completion marker**, not to whether `unit.ir` exists, so a Check form that
> takes the IR artifact does not constitute an exemption.
>
> **III. D59 classification basis (Class C, red before green)**: before the change, the form
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> decision was discarded and execution fell back to the last binding in definition order). After the
> change, the same program fails at compile time with
> `error [W1090] ... produced key was not consumed`. The fix deliberately takes the **loud** path
> rather than “put the resolved name into the call with named arguments”: the latter would lose
> named arguments and replace one silent failure with another (final semantics described in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition, `Drop` instructions are a no-op in
> the interpreter (the wildcard arm at `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty diff
> over 342 corpus files is empirical proof of this.
>
> **IV. Criteria and evidence**: the D41-equivalent criterion `test_release_plan_spans_consumed`
> (settled by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + the evidence floor**:
> number of files that produced decisions `≥ 1`, number of genuinely consumed entries `≥ 2`, no
> allowlist). Nine pinned decision forms are covered in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> type-body binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven verification: shifting a consumption key by
> one position ⇒ criterion turns red + the real program emits `error [W1090]` + the corpus gate
> differs; restoration returns it to green. Gate figures: `check-obligations.py` PASS, corpus diff
> over 342 files empty, `cargo test --lib` 2712 passed / 0 failed (no IR snapshot drift).
>
> **V. Known boundary (registered item, not an unimplemented promise)**: the **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable under the current corpus and test surface**
> (method declarations uniformly go through `register_type_bindings` and therefore use the bound
> consumption point), so the keying correctness of that branch **cannot be falsified in a red
> state**. The reason for retaining it is that it is the symmetric branch of the bound path;
> deleting it would turn the “method call with no `type_bindings`” form into a silent rewrite. This
> is recorded by independent validation (t5 F-1) and review (t6 F2), with **no new task opened**.

### 3. Unified Driver: a single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (in-project LSP, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Must be one of the 6 predefined combinations from ProgramKind::stages()
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers may not supply an arbitrary array.** This is the key constraint preventing the `Program`
abstraction from degrading into a “free parameter that can accept anything,” and is asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines skipping, not the stage's return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce a diagnostic, see below
                continue;
            }
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … all 12 arms; none may be omitted
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 implementation):** Four implementation
> deviations from the sketch above are semantically equivalent and recorded for reference:
>
> 1. §4's three-state `StageOutcome` is not returned by stage arms; it is expressed through `State`
>    failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are unchanged,
>    eliminating per-arm boilerplate).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” determination in §5 covers the sketch field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted—the zero-diff criterion in S2, “deliberately fix no bugs,” requires
>    this; external emission will follow with the 4.3 obligation ledger.
> 4. Visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is the
>    sole new caller (L1→L2 is an allowed direction); existing orchestrator call sites will be
>    restored after their migration in 4.2.
> 5. In the multi-file form, ProofExecution is an independent stage executed **after all type
>    checking** (A1, user decision on 2026-10-09): `compile_project` originally interleaved proof
>    execution within the per-file typecheck loop (file N's proof preceded file N+1's typecheck).
>    With a single failure, the two are byte-for-byte identical; in the multiple-failure scenario
>    “file 1 proof failure + file 2 typecheck failure,” the first reported error changes from the
>    proof error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through channels selected by ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    each entry point (PipelineError / OrchestratorError) are outside the type surface that Driver
>    can unify.
> 7. The Check channel is implemented (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (each discovered file has an entry; clean files have an empty Vec—the existing
>    `check_project` contract); the Discovery artifact is extended to a “file set + used_by edge
>    set + shadowing events” triple stored in State (the MultiFile form has no downstream consumer,
>    so it is recorded but not emitted); the Check path parses each file once rather than twice,
>    following the C3 topology.
> 8. The Lsp form is implemented (4.2.3, file-source routing decision, user ruling on 2026-10-09):
>    parse failures in on-disk files use the Check Plan B approach (collect + exit the compilation
>    unit—convergent degradation on both sides); parse failures in edited buffers retain a partial
>    AST and continue type checking (the editor philosophy—an intermediate typing state should not
>    make semantic functionality disappear). The Discovery arm preserves buffer source over stale
>    on-disk content for Lsp; Typecheck under Lsp checks only the target file (other units exist
>    only to provide signatures through the registry); within-file diagnostic-order normalization
>    extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior in
>    which LSP hard-aborted on an unrelated on-disk file's parse error and the handler silently fell
>    back to the single-file path is fixed by this step.
> 9. The Embedded form is implemented (4.2.4): `Program` gains a `shared_registry` field—the
>    embedded std module is a child compilation, and the registry is an **input**, not an artifact
>    (EMBEDDED_ STAGES has no Registry stage), making #94's SymbolTable sharing contract explicit in
>    the program declaration rather than dependent on “the caller remembers to pass the same
>    registry.” Error-path text changes from `<std.test> (embedded std)` to the unit virtual path
>    `<std/test>` (compiler-internal error surface only, with no test pinning it). At this point all
>    four orchestrator entry points have migrated to the Driver.
> 10. Standalone check is unified (4.2.5, Decision A + IR-stage decision, 2026-10-09): the Check
>     form faithfully carries single-file semantics for programs with no project root—the warning
>     surface covers only the entry file (neighboring files receive errors only; prefer omission
>     over false positives); relative `use` resolves along the importer's directory, matching
>     rustc's single-file mod behavior. The CHECK stage table gains GlobalSlotAlloc + IrGeneration:
>     the old standalone path (the full pipeline) already ran IR generation, and E3019/E1014/E1015
>     and similar errors are produced only by ir_gen (confirmed by a runner gate). The Check form of
>     IrGeneration performs checking only and does not consume the IR; Script/Bin forms are
>     represented by the None/Some value of module_key (the existing E3023 switch, ir_gen.rs:1660).
>     Monomorphization is excluded from CHECK (it only produces resource-limit/internal errors and
>     has no corpus dependency). **〔Withdrawn after empirical validation on 2026-10-10 through
>     3.4.8〕**—“zero corpus dependency” was merely evidence from a corpus with no
>     pathological-recursion fixture; E3005 is the only compile-time defense for this class of
>     program. See note #14. Incidental fix for a latent defect: the Registry arm's repeated
>     harvesting of registrations for embedded std units would overwrite the native half merged by
>     with_std() (false E1043); embedded units now skip duplicate registration.
> 11. The LSP single-file fallback is unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted, and execution moves to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     In the SingleFile+CollectAll form: Parsing collects all parse errors, retains a partial AST,
>     and continues type checking (the editor philosophy from the 4.2.3 decision; the stage is not
>     marked failed, because topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now covers proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and IR-level errors;
>     lexical failures now report real diagnostics (the old synthetic “E0001 lexical error” text
>     disappears with the sequence's deletion—real diagnostics carry precise spans, with no
>     information loss).
> 12. `check_module` / `check_module_collect_all` are verified line by line (4.2.7, mandatory C5
>     item): the two entry points had already converged—the mod.rs layer has
>     `check_module_inner(ast, env, collect_all)` with one Boolean, and the checker layer has
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, with pass-3 and drain modes both running and their difference
>     carried by whether `collected_errors` is empty; there are five collection points in
>     statements.rs (function body/use/for/block/while). Verification also proved and fixed a
>     duplicate-diagnostic defect: collection sites placed the first error in both
>     `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err` produced
>     the same code, span, and message twice (or three times when nested); two further independent
>     mechanisms were found: duplicate annotation validation (E1003/E1103) and duplicate
>     ownership-layer emission at the same point (E2014/E2018). The fix is at the module-result
>     boundary: deduplication by (code, span, message)—a diagnostic is a positional fact, and
>     duplicates carry no information; 63 corpus entries lose duplicate copies, with zero drift in
>     the run list and exit code (the apparent “duplicate” in borrow_conflict_err is an artifact of
>     the baseline triplet format omitting the column number—the two E2018 diagnostics are
>     legitimately in different columns).
> 13. The wasm playground is implemented (4.2.8): `run_code`/`test_compile` use `compile_playground`
>     to construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper layer no
>     longer participates. Error text is aligned byte-for-byte with the old `CompileError` Display
>     prefix (`Parse error:`/`Type error:`/`Internal error:`), with no visible difference in the
>     playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the existing
>     4.1 declaration, with a real producer beginning in this step).
> 14. Multi-file monomorphization and the CHECK stage table are corrected (3.4.8, user decision on
>     2026-10-10, **partially withdrawing the mono exclusion in note #10**). Empirical evidence
>     disproves the claim that “mono only produces resource-limit noise”: pathological generic
>     recursion (`f(x)=f([x])`) causes the compiler process to overflow the stack on the multi-file
>     path without mono (0xc00000fd—the interpreter recurses at the Rust level, with no graceful
>     runtime error), so the mono depth gate is the only compile-time defense; standalone check used
>     the full pipeline before 4.2.5 and therefore already had mono, and the exclusion decision
>     introduced a silent coverage regression (“zero corpus dependency” only proves that the corpus
>     has no pathological fixture). The “type-erasure fallback” assumption was invalidated by the
>     RFC-033 reflection decision—`^^List(Int)` requires the real identity of an instantiation, and
>     monomorphization is the foundation of reflection rather than an optional optimization. The
>     design has four points: ① typecheck produces an instantiation_request for qualified calls
>     (`lib.f(x)`, FieldAccess form), with generic_id using the qualified name, matching the merged
>     IR namespace (currently only bare Var is recognized, expressions.rs:2455); ② the `Stage::ALL`
>     topology changes to Linking before Monomorphization, and the MULTI_FILE stage table gains
>     Monomorphization at the end— consuming merged_ir and aggregating all-unit requests (qualifying
>     the deferred bucket's containing_fn), with mono promoted from a per-module pass to a
>     whole-program pass; the single-file stage table remains unchanged (no Linking, so subsequence
>     properties are preserved and IR snapshots do not drift); ③ CHECK gains Linking (pure
>     merge—E3020 entry validation remains in RoleClassification to avoid duplicate reporting) and
>     Monomorphization (checking only without consuming, like the Check form of IrGeneration)
>     stages; ④ resource protection covers cross-unit mutual recursion through whole-program BFS.
>     The `Monomorphizer` itself is unchanged (its ModuleIR + request-set contract happens to match
>     the merged IR form).
> 15. 3.4.8 R1 (①) is implemented (2026-10-10, a9438007). Qualified-call instantiation collection is
>     implemented according to the #14 design: `ExpressionInferrer` receives a module-qualified key
>     table (the three `use` arms register entries—plain / alias / per-item alias, with only
>     SubModule-class exports eligible), and `callee_generic_name` obtains a qualified name through
>     unified qualified-name construction, using the same source as the merged IR (construction
>     implementation: `frontend::module::symbol`); the qualified call's first path tightens the
>     arity criterion (when the parameter table cannot be resolved, it falls back to deriving
>     arguments from the signature to avoid mismatching same-named functions across modules and
>     falsely reporting E3018). Implementation empirically exposed and fixed three mono-side
>     **asymmetries** in the same round: (1) deletion-key (generic-name set) and rewrite-key
>     (containing_fn + name + span triplet) granularity mismatched—sites whose arguments could not
>     be resolved degraded into symbolic requests in the deferred bucket, and because the containing
>     function was not generic, the bucket never drained, yet the original had already been deleted
>     by name ⇒ dangling call (runtime E6006 confirmed at `list_ops.yx:53`); the fix adds the
>     restoration gate `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as
>     the single source of truth to determine coverage and retaining originals for uncovered sites.
>     (2) `HashSet` iteration order in the restoration section made function-table order
>     nondeterministic (t2 review F1) → changed to `Vec`, sorted by name, making artifacts
>     byte-for-byte reproducible. (3) TailCall / MakeClosure, which are not rewrite-surface forms,
>     still looked up mapping keys (despite the comment claiming conservatism) →
>     `call_form_is_rewritten` provides a single determination, and non-rewrite forms are all
>     counted as uncovered. Each issue has a pinned test (red-state empirical discrimination before
>     the fix). The corpus difference baseline was regenerated with the same commit, yielding
>     exactly one line (a `remove_at` frame gained `(int64)`). **Commit-strategy note**: when
>     typecheck collection and the mono fix were split into two commits, the tree containing only
>     the former triggered an existing `stdlib_docs` failure against a pre-existing HEAD defect
>     (`list.is_empty([])`, same E6006 family)—the two halves are each half of the other's
>     green-light precondition, so they were made one atomic commit.
> 16. Five decisions from 2026-10-10 are registered (after R1 closure): (1) **D2 Decision
>     A**—user-module exports carry `Export.type_params` with `generic_fn_type_params_snapshot()`,
>     structurally identical to the std path (`yx_sources.rs:62/108`): the declared name becomes the
>     authoritative source for qualified-call monomorphization, and signature reverse-inference is
>     demoted to a fallback (within R2's scope). (2) **HKT is active on the declaration side but
>     broken on the call side** (inference passing type arguments E1002 / explicitly supplying type
>     arguments E1010) is registered as WBS 3.5 (three level-three tasks). (3) **The deferred bucket
>     never drains under a non-generic container** is included as an acceptance subitem of 3.4.8 ②:
>     the bucket must drain or fail loudly; it must not silently remain unspecialized. (4) **Calling
>     a closure variable by name yields runtime E6006** is registered as WBS 3.6; the semantics are
>     (i) support indirect calls / (ii) reject at compile time; **(i) has been chosen**—support
>     indirect calls: the `CallDyn` mechanism already exists (the closure-expression path at
>     `ir_gen.rs:7945`), but `Var` callee is demoted to a name-based `Call`, even though typecheck
>     already accepts such programs. (5) The **decision criteria are fixed**—a rejection rationale
>     may only be based on correctness and readability; “large change surface / introduce a new
>     mechanism / high cost” does not constitute a rejection rationale. This is written into the
>     second part of `coding-rules.md` and the HOWTO self-check table; choices requiring a user
>     decision must follow all four items in `AGENTS.md` under “When requesting a user decision
>     (mandatory)” (situation + evidence / pros and cons for each option / explicit recommendation +
>     cost / explicit identification of breaking changes).
> 17. 3.4.8 ② is implemented and the F1 root cause is identified (2026-10-11, 375789d2 + d922a551).
>     ② is implemented according to the #14 design: `Stage::ALL` changes to Linking before
>     Monomorphization, and the MULTI_FILE table gains mono at the end; the Linking arm aggregates
>     whole-program requests **before taking each unit's IR** and qualifies `containing_fn`
>     (otherwise the deferred bucket's keys differ from the merged IR's source and the bucket never
>     drains). **① is closed (D2 Decision A)**: user-module exports carry a declaration-time
>     type-parameter snapshot via `Export::type_params`, structurally identical to the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically exposed and fixed during
>     implementation**: (a) **F1**—`qualify_names` rewrites only function bodies, while `ir.init` is
>     never visited, so in multi-file builds function definitions are qualified but top-level module
>     calls remain bare names ⇒ runtime E6006 (**reproduces even for non-generic code**, unrelated
>     to generics); the fix has init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono-side “whole-program unique suffix” heuristic and silent
>     `continue` on a miss → the heuristic is deleted, and a miss now produces a **loud E3018**
>     (silent ambiguity misresolution is worse than failing to resolve). **Independent validation
>     caught and fixed a regression**: t6 found that a named import `use lib.{pick}` followed by a
>     bare-name call was falsely rejected with E3018 (the IR instruction name is the alias-qualified
>     name, while the request name is bare; the aggregation side qualified only by unit key) → the
>     aggregation side now **directly calls ir_gen's** `import_aliases_from` and
>     `resolve_call_target` (extracted as a public single point, with one producer and one
>     consumer), and a matrix of pinned tests covers both forms of named import. **Record**: the
>     blank `DefId` in the init section was empirically a “consistency blank” rather than a live
>     defect (the codegen name fallback resolves, and all four classes of init call were empirically
>     verified to work); it is registered under WBS 3.7.4 to close the policy split between
>     “function bodies use def, init uses names.”
> 18. The init-section `DefId` policy split is closed (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     extracts the single-point `resolve_static_targets` (Call/TailCall string operands +
>     MakeClosure), and function bodies and `ir.init` share the same implementation and the same
>     `resolve` closure; after 208 IR snapshots were regenerated, only 309 `(def=_)`→`(def=dN)`
>     pairs changed (other-lines 0; zero residual `(def=_)` entries in the full tree's init
>     section). Under both codegen's primary `DefId` path and the name-fallback path, translated
>     results and execution values agree item by item (all four classes of init call empirically
>     verified), with zero corpus-baseline drift; the probe flips and strengthens under the new
>     policy (identity + exact index + execution value; a fallback init override turns it red).
>     **Correction (2026-10-11)**: the commit numbers above were rewritten by
>     `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396 correspond respectively to
>     375789d2/d922a551/38bd4b9e after the rebase; the pushed dev history uses the new numbers (the
>     old numbers were local temporary numbers and never entered remote history).
> 19. 3.4.8 ③④ is implemented and verified (2026-10-11, 8ecf56f93). (③④'s design basis is #14; ②'s
>     implementation is #17, which is its closure.) **Implementation**: the `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; the Check form of `Linking` performs only a pure
>     merge (aggregate requests + `link_module_irs`), with the entire entry validation left to
>     `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (memory only);
>     mono failures enter the collection channel in Check and retain `fail_with` in MultiFile;
>     `into_outcome` first calls `take_check_diagnostics` and then checks for failure (a structural
>     failure no longer clears already-collected diagnostics). **Verification**: four rounds of
>     independent validation (t3/t4/t10 + t5 cross-form matrix) and one review round (t6) all pass;
>     the corpus diff over 340/340 files is line-by-line IDENTICAL, IR snapshots do not drift, and
>     the test count changes from 2643 to 2656. **Existing gaps exposed and fixed in the same
>     round**: F1 (rootless multi-file check had request names and IR definition keys from different
>     sources ⇒ silent failure to specialize, see the implementation note in WBS 3.4.8's row)—the
>     fix is precise reverse lookup by the actual definition key (not a heuristic); the first
>     version of the fix used the actual definition key (flat_defs) for precise reverse lookup;
>     **same-day correction**: this bridge was deleted during the regression fix, and the root cause
>     was instead decoupled by having `module_key` serve two purposes (`script_mode`), with all
>     three now sharing the source `{unit key}.{name}`. The location and fix for that regression
>     (the CI documentation-example gate went from 258 blocks / 10 failures to 0 failures) are
>     described in the implementation note in WBS 3.4.8's row. **Record of an existing,
>     non-regression issue**: the E3005 help text does not match the resource-protection semantics;
>     the multi-file E3005 span label is misplaced (reproducible with the HEAD binary); the
>     type-parameter-position dependency on the collection surface remains (assigned to 3.5 on the
>     same surface).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 are implemented and the diagnostic surface is closed (2026-10-12,
>     809b9b936 + 8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is
>     reachable)**: interpreted calls are changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/ `SpawnFromList` push frames and return
>     control to the outer `step_one`; each frame carries `ReturnTarget{Host|CallerSlot|Discard}`,
>     and returns write the result back to the caller's slot, while multiple call instructions in
>     one frame continue through the frame's `pending_calls`. Baseline measurements show each
>     interpreted call level consumes **2544 B** of native stack, with the crash threshold at
>     `count_down(394)` (1 MiB main thread on Windows); after the fix, 20,000 levels of recursion
>     run successfully on a 512 KiB-stack thread (approximately 0 B per level). The guard
>     (`push_frame`) is therefore reachable through every entry point and every `max_stack_depth`
>     value; `ExecutorError::StackOverflow` now carries the true limit (previously
>     `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (deep-stack echo truncation) was
>     merged into this task by user decision** (same file, cannot be split): the echo now shows the
>     first 16 frames + an omission count + the last 8 frames; shallow-stack frames are unchanged.
>     **3.10.1 (no compile-time guard against native recursion, #447)**: locate first, then fix—the
>     crash point was recursive `MonoType::type_name` (**328 B/level**, SP probe + precise operation
>     boundary identification), with derived `Clone` (~400 B/level), deep copies in `substitute`,
>     and `type_name_hash` (164 B/level), plus `collect_generic_type_refs`; after converting all
>     four to worklists and removing one deep copy of the entire request per level, a pathological
>     generic recursion under `mono.max_depth=5000` produces **E3005 (with the true limit)** instead
>     of overflowing the stack. **3.11.1 (task-boundary error-code fidelity, #448)**: the task
>     result's error side changes from a value to a **structured form**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), restored by
>     value across boundaries; task-internal E6001/E6003/ E6004/E6008 are identical in wording in
>     Standard and Embedded modes. Two **pub API payload-shape changes** in the commit are marked
>     `BREAKING CHANGE`. **Commit-strategy note**: 3.9.1 and 3.11.1 are **inseparable at file
>     granularity**—`interpreter/executor/executor.rs` contains both the trampoline and the
>     error-channel producer/consumer, while `TaskFailure` is defined in `runtime/engine.rs`;
>     staging only either side will not compile (pre-commit compiles the staged subset and rejected
>     it twice in practice), so under the same policy as #15 they are made one atomic commit.
>     **Known boundaries and follow-up**: derived `Drop` (~75 B/level) ⇒ `mono.max_depth ≳ 13000`
>     may still crash; the E4012 (constant evaluation) gate rests on native recursion (each level
>     ~3,696 B, default 256 levels ≈ 924 KiB) and currently has zero call sites and is unreachable
>     in production; future wiring must close the same family of issues; the Standard-mode task
>     interpreter is constructed with the default `ExecutorConfig` ⇒ its E6004 limit is always 1024
>     (configuration propagation registered as WBS 3.12, user decision B: separate team); the
>     test-runtime budget is a separate issue #449 (pathological fixture 76 s, clippy dual target,
>     commit runs the full suite), and is outside this RFC's implementation surface. **Corpus
>     directory stratification (same round, 8605a081b)**: `02-type-system` (78) and
>     `06-compile-errors` (134), totaling 212 fixtures, are split into 23 category-based
>     subdirectories, and 88 IR snapshots move to mirrored paths; the corpus baseline is regenerated
>     with the same commit (342 lines, independently recomputed line by line: only the `file` field
>     and 2 new entries change), and 41 snapshots are regenerated because the file-header path
>     became longer, with line-by-line verification showing **only span-offset changes** (zero
>     differences after offset normalization).
> 21. The LSP semantic-data pipeline is unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references in projects carry
>     `resolves_to` pointing to the real definition— the old form triggered **two** compilations
>     through didOpen/didChange: diagnostics used the unified Driver pipeline, while semantic data
>     used server.rs's manual lex→parse→single-file typecheck (with no module registry, in-project
>     `use` resolution failed and no reference events were produced, so cross-file
>     navigation/rename/find-references silently failed to resolve—the silent failure registered as
>     D55 is closed by this task). **Implementation**: (1) `Export` carries `definition_span`
>     (harvested by `extract_module_info` from AST identifier nodes, with the same source and value
>     as typecheck's definition event `def_span`; native std always has None), and `ModuleInfo`
>     carries `definition_file` (filled by the Registry arm from `DiscoveredFile`); (2) when
>     draining reference events, the checker redirects the `resolves_to` of cross-file imported
>     items from the use line to the real location in the definition file through
>     `import_def_locations` (std imports have no registration and remain on the use line); (3) the
>     checker's module identity changes from the unified `"<module>"` to the unit file path (that
>     value is consumed only by the semantic-data surface, with zero diagnostic consumers—empirical
>     verification of all four `env.module_name` consumption sites); (4) `DriverOutcome` gains a
>     `semantic_db` channel—CollectAll forms (Lsp target file regardless of diagnostic success +
>     SingleFile fallback) feed into it, and the Lsp arm populates top-level definition surfaces for
>     non-target files from the registry at the end (`external_definitions`, allowing navigation to
>     files that have not been opened; type/ submodule exports are not injected—typecheck-side type
>     registration does not produce definition events, and both sides preserve the same absence);
>     (5) the orchestrator gains `analyze_source_in_project` (`check_source_in_project` delegates to
>     it and takes only the diagnostic half); (6) the LSP-side `analysis.rs` single-compilation
>     entry point replaces the manual pipeline (`update_semantic_db` in server.rs is deleted), and
>     World keys are unified as normalized file URIs (`uri.rs`: `path_to_uri(uri_to_path(u))`
>     provides idempotent normalization, unifying differences in drive-letter case and `%XX`
>     encoding under one key). **Incidental fix (D59 Class B—old behavior silently did nothing
>     without changing the externally visible behavior of any previously successful program)**:
>     cross-file navigation/rename/find-references for in-project files now resolve to the real
>     definition instead of silently failing; on lexical errors, the old manual pipeline discarded
>     semantic information, whereas the partial AST now retains coloring (an extension of the 4.2.6
>     editor philosophy); when the target file has type errors, semantic data is no longer discarded
>     along with `type_result` (coloring/navigation no longer disappears during an intermediate
>     typing state). **Pinned tests**: 6 driver-level tests (target-file path key / cross-file
>     resolves_to + external definition-surface round trip / std import falling back to the use line
>     / retaining an erroneous file / single-file fallback path key + error retention), 1 LSP
>     end-to-end test (red-before-green empirical proof—navigation returned None before the fix),
>     and 4 URI-normalization tests.

**Conversion of the ten entry functions (each function specified individually):**

| Entry point                                                | After conversion                                                                                                                                                                                             |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and invoke Driver                                                                                                                      |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                      |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                 |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                   |
| `check_source_in_project` (`orchestrator.rs:450`)          | Replace with one Driver invocation + filter results to the target file                                                                                                                                       |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Replace with `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                 |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                       |
| `lib::run_project`                                         | Unchanged (delegates to Driver)                                                                                                                                                                              |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project: use `Program { kind: Lsp, aggregation: CollectAll }`; delete the single-file branch's **manual lex→parse→… sequence** and invoke `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                       |

**`check_project` and `compile_project` have approximately 40% overlap / 60% divergence.** The
shared parts are vendor consistency checking, file discovery, `build_registry_from` (deleted on
2026-10-26 when its last caller was removed; the historical account is retained here),
`all_method_bindings`, the parsing loop, per-file checker assembly, and result processing. The
divergent parts are items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This shared 40% skeleton
is precisely Driver's value interval**—the divergent 60% consists entirely of “different stage sets”
or “different aggregation modes,” and those are exactly what the two `Program` fields can fully
express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // stage succeeded; continue
    Abort,     // stage failed; terminate this compilation
    Warn,      // stage had a non-blocking problem; continue
}
```

**Key design decision: “skipping a stage because an upstream stage failed” is not a stage return
value.**

The reason is that skipping is determined by **topology**, not by the stage itself. If every stage
returned `Skipped` independently, the reason for not running would be distributed across 12 stages
and could not be centrally audited. Instead, `dispatch` determines this at the beginning of each
iteration according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**A skip must produce a diagnostic.** This rule directly targets the gap addressed by this document:
the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip” that produces no
diagnostic, merely because the obligation happened to be empty. Once this rule is established, every
case where “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                           | Text direction                                                                                                              |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                    | “Stage Y did not execute because upstream stage X failed”                                                                   |
| Caused by an empty prerequisite obligation                       | “Stage Y did not execute because there was no pending obligation (normal)”—**severity = Info and not counted as a warning** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y did not execute because it was not enabled by configuration”                                                       |

The second category is critical: it distinguishes a “normal skip” from an “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (`diagnostic/mod.rs:637-641`
depends on that count for its non-blocking contract).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // abort on the first error (compile paths)
    CollectAll,  // collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                               | Existing equivalent |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)       | Existing            |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing            |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project and single-file files in the same process, while the CLI's `run` and `check` are
independent invocations. Placing it on Program avoids mutable global state in Driver.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether they
return early”—they are two different checker entry points. After unification, the same
implementation should be driven by the `Aggregation` parameter rather than retaining both functions
(**this is an additional item requiring verification under this document; see “Known Limitations and
Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types are unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                              |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain in
their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers, such as `ir_gen` reading `release_plan`, continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
driver no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend on `crate::driver` in
reverse**. The presence of `Driver` in a `TypeChecker` import is a violation and is to be
intercepted by `scripts/ci/check-module-boundary.py`, as proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                    | The same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone inputs—it uniformly uses `compile_project` with the MULTI_FILE stage table, and Script semantics are driven by rootless operation; this row now describes only the `lib::run_file` / `Compiler::compile` library paths)                                                                                                                                                           |
| `yaoxiang run` (multi-file)         | No proof_execution and no W1001/W1002/W1003 | proof_execution was connected by the P3 emergency fix; **dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” with Check / Lsp / MultiFile sharing one implementation and no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for this arm to emit) |
| `yaoxiang check` (multi-file)       | No proof_execution                          | Connected by the P3 emergency fix                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| LSP (in-project)                    | No proof_execution                          | Connected by the P3 emergency fix                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Z3 not installed + single file      | `predicate.rs:35` **panics**                | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Z3 not installed + multi-file       | Silently skipped                            | Unchanged above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |

**The only intentional behavior break** is replacing the `.expect()` at `predicate.rs:35` with a
returned diagnostic. This is consistent with RFC-027 §8's “not bound to a specific solver”
positioning and with the contract already declared at `backend.rs:60` (“the caller should degrade
conservatively”)—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Contents                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (one for a single file / N for multiple files)                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                          | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                                      |
| --------------------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                       |
| `src/lib.rs`                                  | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                  | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                    | `141-227`                                      | Replace the `Pipeline::run` body with `Program` construction + Driver invocation                                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                    | `275-278`                                      | Move dead-code analysis out and into a `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                    | `306-311`                                      | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                    | `381-389`                                      | Move monomorphization out and into a `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`         | `450`                                          | Change `check_source_in_project` to use Driver                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`         | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                      | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                            |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics` and invoke Driver                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                      | Unify `check_files_with_diagnostics` on `Program { kind: Check }`                                                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                              |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                         |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                               |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                      |
| `src/frontend/core/sema/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “a lower layer failing means an upper layer does not run” (there is no short-circuiting)                                                                                                                                                                                                             |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                        | **Under D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure entry point; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared entry point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); the original wording is subject to D58 |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production logic of the `Unproven` branches (now distributed across
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is at the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch (wasm branch
reachability determination is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Disposition                                                                                                                                           |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches and record the diagnostic-set change in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that previously compiled now produce warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                        |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: diagnostic set grows                                         | W first, then E, in two steps under S2/S4                                                                                                             |
| Replace the `predicate.rs` panic with a diagnostic               | **Improvement**: no longer crashes                                         | No break                                                                                                                                              |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These paths do not pass through proof_execution                                                                                                       |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | If the public API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                      |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness gaps)** and **P4 (Stage contract and unified
Driver)** in RFC-039's global stage sequence. S1–S5 below are the implementation order within this
document. **Each step's acceptance criteria reference category C2 (orchestration changes) in the
[equivalence criteria document](07-equivalence-oracle.md)** (identical **diagnostic sets** across
entry points + identical corpus behavior).

### S1: Establish the criteria (red first)

**Prerequisite; cannot be skipped.** The gap criteria must first be written as failing tests.

| Deliverable                                                                                                                                                      | Acceptance                                                                                                     |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red**. If it unexpectedly passes, the gap analysis in this document must be reviewed again           |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | Diagnostic sets of the single-file and multi-file corpus versions are comparable                               |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                               | Marked `#[ignore]`; turns green in S4 (red-state record: after removing the sole settlement point, `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                    |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array    |
| Change `pipeline.rs:141-227` to invoke Driver            | **Single-file diagnostic set and exit code are byte-for-byte identical**                           |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic lists (sorted by `(code, file, line)`), exit codes, stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. Acceptance is
at the C1/C2 zero-diff level.

**Rollback point**: `git revert` a single commit to restore `pipeline.rs`.

### S3: Merge the four orchestrator entry points + fix the gaps

| Deliverable                                                                                                                   | Acceptance (C2)                                                                       |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Convert `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | Normalize the diagnostic sets of the four entry points according to `Aggregation`     |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                      | `yaoxiang check` behaves consistently inside and outside a project                    |
| Delete the manual LSP stage sequence                                                                                          | LSP and CLI produce the same diagnostic set for the same file                         |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                 | The gap is fixed                                                                      |
| Fix the ownership layer's empty `Unproven` match arm (originally `checker.rs:1313`)                                           | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode |
| Replace `.expect()` at `predicate.rs:34-36` with a diagnostic                                                                 | No panic when Z3 is removed                                                           |

**Rollback point**: the gap fix and structural merge must be **two separate commits**. If the
structural merge causes problems, only the structural commit may be rolled back while retaining the
gap-fix commit; at that point `test_multifile_proof_obligation_not_dropped` remains green. The
reverse—retaining the structural change while rolling back the fix—turns the test red and is an
unacceptable intermediate state that must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                         |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implementation form described in the §2 revision note; use that file's actual count) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                                           |
| Promote obligation diagnostics from W to E   | Review every newly introduced E after the full-corpus differential                                                                                 |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration item,
and switching between W and E does not require structural changes.

### S5: Finish the proof layer and wasm work

| Deliverable                                                                                                                                                                                                                                            | Acceptance                                                                                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                       | The documentation corresponds point-by-point to the call sites in `checker.rs`                                                                                            |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes cross-thread sharing under cargo test) | Production call sites of `default_solver()` drop to zero (tests only); cache-hit rate is observable across all three consumers (counter already implemented in 89576fafd) |
| The `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`) adds a warning when degrading                                                                                                | A diagnostic is produced when there is no `body_checker`                                                                                                                  |
| Determine the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                    | Submit the conclusion to the “F2 determination” section of `06-cleanup-inventory.md`; this document records only the need for the determination                           |

**Note**: correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item should proceed independently of S1–S4** and must
not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustive enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                                       | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant makes `dispatch` fail to compile. This is structurally identical to the opcode-table mechanism already validated in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that require human reasoning                                                                                                                                    |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement, together with the **static** gate `scripts/ci/check-obligations.py`; **final severity is E** (user decision A on 2026-10-13: same-code promotion to E with the code number unchanged; **implementation form described in the §2 revision note**: the ledger does not copy the field structure; the red list—zero entries after cleanup—retains its mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not a single bug: at least 2 instances of the same hazard have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. The W→E two-step approach was intended to keep every step testable against C2—now that the obligation inventory has been cleaned up to 13 entries and the red list is zero, the healthy state emits nothing, so the final state directly uses E (see sections seven and eight of the §2 revision note) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer); do **not** change the production logic of the `Unproven` branches (now distributed across three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                      | The problem is at the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic has been fixed,” while the logic in these branches is itself correct                                                                                                                                                                                                                                                                                                      |

This design additionally eliminates two classes of cracks:

- The crack between **“commentary promises” and “code behavior.”** The “silent pass must not be
  resurrected” statement in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, now enforced by `assert_drained()`.
- The crack of **“one array was forgotten during an append.”** Currently, five functions each
  hand-write their call sequence. After unification, there is one stage table, and a missing stage
  fails at compile time.

### Rejected Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, and IR
  generation/linking) **do not involve type transitions**; they are configuration questions of
  “whether to run an analysis.” This form also cannot express `Aggregation`; it would merely move
  the `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer makes `dispatch` fail to compile, replacing
  “five functions each hand-writing their calls” with “one array forgotten during an append,” which
  is precisely the cause of the current 11 inconsistencies. The repository already contains a
  counterexample of the same form: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache`
  / `HotReloader` described in `docs/src/dev/design/check/` all have zero implementations.
- **Fix only the bugs without changing the structure**—this would fix the two known bugs but cover
  only 1 of the obligation fields; `Stage` would remain scattered across 5 functions, and the next
  divergence would continue to grow from there. **It must be done first** (as part of S1/S3),
  because the structural refactoring needs a known red test to prove the criterion is effective.
- **Make `proof_calls` public and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the **entire field** is discarded. This searches for the bug at the
  wrong layer: the bug is in the orchestration layer, so detection must be in the orchestration
  layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of refactoring that “centralizes the
  wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing problems that were
  previously silent.”** After the fix, users may report a batch of “new errors”—these are real bugs
  that were never reported. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires per-field human judgment.** Some
  obligation fields, such as `module_namespaces`, may have no consumer by design and should not be
  reported. S4 must review each field individually rather than applying one policy indiscriminately.
- **The `Program` abstraction may be premature.** If the stage sets of some entry points remain
  unstable for a long time, `stages()` may degrade into a free parameter through which “every
  invocation supplies a different array,” and the contract constraint will fail. **Mitigation:**
  `test_program_stage_coverage` asserts that `stages()` can only come from the 6 predefined
  combinations.
- **Criterion dependency**: the gap-fix criteria for S1/S3 depend on the
  [equivalence criteria document](07-equivalence-oracle.md) establishing the multi-file corpus layer
  first. **If that document has not first established a multi-file corpus layer, S1 cannot be
  accepted**—all 293 existing corpus files take the single-file path and provide zero coverage for
  this class of defect. This document does not cover the IR verifier (`verify_loose`); that
  prerequisite work is outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse of
  their declaration). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undecided.** `backend.rs:13-19`
  states that `Solver` is `Send` but not `Sync`, and the `Z3Backend` cache uses `RefCell`; after
  making it a singleton shared across compilation units, all access paths must be confirmed to pass
  through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file type checking is naturally
  parallelizable, but parallelism can **mask order-dependency defects**, such as the actual order
  dependency between termination and ownership. This should be opened only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been decided.** See
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **This document leaves no unresolved items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` conversion (implementation steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three criterion layers,
  `test_multifile_proof_obligation_not_dropped` gap criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — explicit statement that “the silent pass must not
  be resurrected” (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — layer-order invocation table and the true identity of
  `equivalence` (no short-circuiting)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — contract statement that “the caller should
  degrade conservably” (contradicts `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-98` — 15 obligation fields that downstream code must consume
  (after the 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — the unit test's own
  acknowledgement that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  the corpus's own acknowledgement that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (used by `run_code` for compilation)—the playground's
  fourth compilation caller (the single-file path, so it executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — compile-time error-code gate and an example of this project's enforcement
  mechanism
