# Compilation-Stage Contracts and the Obligations Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, acceptance-criterion tiers, and execution-stage
> order; see the [index for this directory](index.md) for the purpose of each companion document.

## Purpose and Scope

This document is the construction blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **the question of “which compilation stages ran, which did not, and why” is currently
distributed across six paths and ten entry functions, each manually wired, with no single place able
to answer it.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being violated by the architecture rather than by the
> logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` reads as follows
(originally `checker.rs:5165`, relocated in P5 5.2.1): “Unproven → compilation error, no
degradation, no silent pass……silent pass must not be allowed to return.” However, this invariant
holds only on the **single-file path**: proof-function constraints such as `x: Sorted(3)` **silently
pass** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is
never executed, and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustively enumerable, not registered at runtime) and `StageScope`;
- The `Obligations` obligations ledger and `assert_drained()` settlement;
- A unified `Driver` (a single `dispatch`) and the individual changes required for each of the ten
  entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- All affected files and line numbers, compatibility impact, and implementation order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-bounce gates → `01-routing.md`
- Equivalence criteria (the C1–C6 tiers and three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch-reachability cleanup → `06-cleanup-inventory.md`
- The global execution order for P1–P10 and the G1–G10 acceptance gates → the main body of RFC-039

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should be performed.
This document defines the concrete shape of L1, a file-by-file change list, and the implementation
stages within this document, corresponding to P3 “Fix correctness vulnerabilities” and P4 “Stage
contracts and unified Driver” in RFC-039's global sequence. If this document conflicts with RFC-039,
RFC-039 takes precedence.

Equivalence criteria are applied under the **C2 (orchestration change)** category from the
[Equivalence Criteria Document](07-equivalence-oracle.md): the diagnostic sets of all entry points
are identical + corpus behavior is identical.

## Current State

> Every item in this section is a **verified fact**, with a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Sole Structural Defect Where “One Mistake Silences the Entire Repository”

This differs in nature from the other two classes of defects—module boundaries and test wiring:

| Defect                  | Typical manifestation                    | Is there a signal?       |
| ----------------------- | ---------------------------------------- | ------------------------ |
| Lexical/syntactic error | Source code is wrong                     | Yes, a diagnostic        |
| Type mismatch           | Type is wrong                            | Yes, a diagnostic        |
| Stage omitted           | A stage is never invoked                 | **No signal whatsoever** |
| Obligation not consumed | A field is populated but nobody reads it | **No signal whatsoever** |

The latter two classes share a defining characteristic: **failure does not produce an error**. They
therefore cannot be solved by “writing the code more carefully” or “performing a more rigorous
review”—code review can show what was written, but not what was **omitted**. This is precisely the
root cause identified by RFC-039: “This project treats ‘design’ as a documentation convention rather
than an executable constraint.”

### Enforcement Mechanisms Already Exist in This Repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes + 8 W codes) are subject to a
  **build-time hard gate** through `tools/code-tables` via `build.rs:19-55`. Every code is compared
  against the RFC-013 code table, and any discrepancy directly triggers `panic!` and rejects the
  build.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines, or 47.9%, are under `tests/`)
  has high test density, and every module's test subtree is explicitly wired up. This is the
  repository's most complete example of test wiring and can serve as a formal reference for a
  stage-layer gate.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(Line-count basis: `(Get-Content).Count`; see the “Line-Count Basis” section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths and Ten Entry Functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path used                                                              |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages invoked directly                                                       |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `lib::run_file` in `src/lib.rs`                  | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `lib::run_project` in `src/lib.rs`               | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c in a project; otherwise manually lex→parse                                 |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages on the single-file path (`pipeline.rs`) are:

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)—RFC-027 Phase 2.5                       |
| ir_generation       | `205`         | `run_ir_generation`, **with monomorphization embedded in it** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The table below provides evidence cell by cell. **A blank cell does not mean that the entry point
“does not do this”; it means that “no code in the entry point does this”**—which is precisely the
nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single-file)                             | `compile_project`                                              | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`               |
| --- | -------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | --------------------------------------- |
| 1   | **proof_execution**                                      | **Yes**, `187-203`                                   | **No**                                                         | **No**                                                       | **No**                             | **No**                                  |
| 2   | Dead-code analysis                                       | Yes, `275-278` (gated by `config.dead_code.enabled`) | **No**                                                         | Yes, `356-377` (role-aware, no configuration gate)           | **No**                             | **No**                                  |
| 3   | W1006 local-module shadowing                             | **No**                                               | **No**                                                         | Yes, `336-348`                                               | **No**                             | **No**                                  |
| 4   | W1003 unused imports                                     | Yes (collects `type_result.warnings` at `277`)       | **Collected but never emitted**                                | Yes, `350`                                                   | **No**                             | **No**                                  |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                      | **No**                                                         | Yes (same as 2)                                              | **No**                             | **No**                                  |
| 6   | **Monomorphization**                                     | Yes, `381-389` (gated by `config.mono.enabled`)      | **No**                                                         | Not applicable                                               | Not applicable                     | **No**                                  |
| 7   | typecheck branch                                         | `check_module`                                       | `check_module` (`124`), **returns on the first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                 |
| 8   | File discovery                                           | Not applicable                                       | `discover` (`101`, discards `used_by`/`shadow_events`)         | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | Not applicable                          |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                               | **No**                                                         | Yes (`321-328`)                                              | **No**                             | **No**                                  |
| 10  | Global slot allocation                                   | Not applicable                                       | Yes, `allocate_global_slots` (`147`)                           | **No**                                                       | **No**                             | **No**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                          | Yes (`152-236`)                                                | **No**                                                       | **No**                             | Yes (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Differences in the current state
> after P3 lands are as follows:
>
> - **Row 1 has been fixed**: the five entry points share the same `frontend/proof_execution.rs`
>   implementation (pipeline + four orchestrator entry points), so the sole-consumer defect no
>   longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and dead-code-family warnings, and the W-code sets of the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) have been
>   normalized—the criterion has a single source in the shared implementation on the
>   `dead_code_analysis` arm.
> - **Row 6** (single-file-only monomorphization) → assigned to WBS 3.4.8 (prerequisite 4.1.3; the
>   potential-risk defect has not been demonstrated).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (`Aggregation` parameter-driven).
> - The code-side reference to “decision #434” previously had no documentation entry—now recorded as
>   RFC-039 **D57**.
> - **Asymmetry in embedded-std inclusion (new fact outside the table, added on 2026-10-09)**: the
>   single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required for
>   for-loop desugaring; hard switch under #117), whereas multi-file `discover` previously
>   recognized only explicit `use`—project for loops compiled successfully and failed at runtime
>   with E6006 (confirmed by a probe). This has been fixed and WBS 4.10.2 closed. The related
>   “multi-file lacks the single-file step” parse hard-abort quirk was registered as WBS 4.10.1 and
>   **has also been fixed** (landed immediately after 4.2.2; the Check path was downgraded to
>   per-file collection. This audit method examines the dimensions of “stage coverage and field
>   consumption”; this case is a difference in compilation-unit membership and fell outside that
>   scope).
> - **Embedded-std registration surface coverage (construction finding in 4.2.5, fixed)**: the
>   Registry arm redundantly harvests registrations for embedded std modules through
>   `extract_module_info`, replacing the native half registered through `with_std()` (losing
>   `result.is_err`, etc., and causing std.test to report a false E1043). This exposed the audit
>   method's third missed dimension: merge semantics for **two registration sources under the same
>   module key**. The single-file path forms its registry only once through `with_std()`, whereas
>   the multi-file/Check Registry arm obtains coverage by inserting each compilation unit
>   individually.

**Two points require precise wording; otherwise the conclusion would be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` **never reports W1001/W1002/W1003 on
  the multi-file path** (`run_project` → `compile_project`); it **does report them on the
  single-file path** (`run_file` → `Pipeline::run`). The reason lies in `compile_project`: it packs
  the complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function **has
  no read site for `result.warnings` anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The criterion for the entry `main` in row 11 also has different sources**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, which checks only whether a manifest exists). The
  two functions give different answers to “which files must define `main`.”

### Correctness Vulnerability: Proof Obligations Are Silently Discarded

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1—There is exactly one production point for obligations.**
`src/frontend/core/sema/layers/predicate.rs:232-239` is the **only** location in the repository that
constructs a nonempty `proof_calls`:

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

Its semantics are: “The predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed before the constraint can be decided.”

**Step 2—The consumer is inside the checker, but the obligations are produced without being
consumed.** `src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                            | Behavior                   |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| Parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → produces a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Produces no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Produces a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Produces no diagnostic** |
| Return-position obligations (`check_return_refinement`, originally `checker.rs:5420`)      | Produces a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Produces no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real line of defense rather than a prospective fallback: silent pass must not be allowed to return.`

In other words, the author knew that “producing no diagnostic” was a defect, explicitly classified
it as a **known intermediate state with nowhere to go**, and assumed by design that “someone would
eventually come to read `proof_calls`.”

**Step 3—The field is indeed populated in the result.** `check_module_impl` declares a local
`proof_calls` and collects into it (originally `checker.rs:1234-1235`), then writes it into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registered proof-function obligations`
(originally `:1439`). The field is defined at `types.rs:29`.

**Step 4—There is only one read site in the entire repository.** In production code,
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

(Other repository-wide occurrences of the `proof_calls` identifier fall into three categories and
none consumes this field: the three producer-side collections in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); `verdict.rs:61`, which is a same-named field
of `ProofResult`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5—None of the orchestrator's four entry points passes through that layer.** The four entry
points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)—**all bypass
`pipeline.rs`** and call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, none of them can even reach the sole read site at `pipeline.rs:187`.

**Step 6—Consequence: refinement obligations in the standard library itself also follow the discard
path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible
for compiling embedded std. This means that **the embedded std module's own proof obligations are
not executed either**.

**Step 7—Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostics** on the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (supplementary verification)—There is a second silent discard point within the layers.**
When `check_module_impl` handles ownership-layer results (originally `checker.rs:1303-1314`):

```rust
// Originally checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership checking layer's `Unproven` is swallowed by an empty match arm, with no diagnostic
and no accounting.** This directly contradicts the promise at the `5164` branch that “silent pass
must not be allowed to return,” and it is **independent of the orchestrator problem**—even after the
entry layer is completely fixed, this point will still be silent. (Its handling timing should be the
same batch as the obligations ledger, because it is the same class of problem.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero occurrences of the
keywords `Sorted`, `proof`, or `refin`.** Proof obligations on the multi-file path have zero
coverage.

The three RFC-027 unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
nonempty—for example, `rfc027_return_refinement.rs:350-359` asserts that the `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test file's own documentation comment already states this point.
`rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer`

**This is the shape of the problem: the tests verify that “obligations are populated,” while the bug
is that “the consumer does not read them.”** A test that covers only the producer and not the
consumer is inherently immune to this class of defect.

### Stronger Finding: the Main Corpus for Equivalence Criteria Uses Only the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) uses end-to-end differential testing
over 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and the
principal acceptance mechanism for C2. However, measurements show:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero glob matches for the
  entire directory).
- Consequently, for every corpus file, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)
  takes the `standalone` branch (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this fact. The file is annotated `// expect: compile-error E4018` on line 15, and its
header comment on line 7 says “Status: ❌ Should be rejected at compile time”—**it passes precisely
because it takes the only path that executes the proof function**.

**Conclusion: the third criterion layer needs an additional multi-file corpus layer; otherwise it
cannot serve as the acceptance mechanism for this document.** See Implementation Point S1.

### Obligation Fields: 14 “Produced Means Contractual” Fields with No Mechanism Ensuring Consumption

The `TypeCheckResult` in `src/frontend/core/sema/types.rs:16-89` has **14 fields** whose
documentation comments explicitly state “produced by stage X → consumed by stage Y”; in other words,
they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: definition line = the **declaration line** of the field in `types.rs`
> (the line containing `pub` / `pub(super)`), not the first line of its documentation comment—the
> comment block for one field may span multiple lines, and only the declaration line is uniquely
> anchorable. The **struct range** in the sentence above this table
> (`src/frontend/core/sema/types.rs:16-89`) moves with the struct's line numbers and is mechanically
> reconciled by gate **F5** (an error immediately turns it red); definition lines are reconciled by
> **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan` | `38` |
> ownership → IR generation (**keyed by `PlanId`**: read at `ir_gen.rs:360`
> (`type_result.release_plan.clone()`), used at `:3103` (`ReleasePlan::consume`);
> settlement-completion marker at `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> at `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation | | `record_models` | `79` | typecheck (one pass at declaration
> time) → IR generation (quantifier propagation; from B.2 onward, witness-field
> `TypeDeclField.quantity` and constructor-parameter/default-value population both read this table)
> |

**Reclassification record (3.4.9, user decision A on 2026-10-13):** the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design it
is consumed in-phase by typecheck and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed during this round).
- **Disposition**: remove the documentation comment's **claim** that the field is “mirrored for
  consumption by the middle layer/LSP”; retain the field and its in-phase consumption. The field is
  registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP
  genuinely needs the table in the future, register it under 4.6.x; **do not create a consumer
  during this round**.

**All these fields cross layers keyed by `Span` or name tables.** The Equivalence Criteria Document
already points out that mismatches in span-keyed contracts can “fail silently, without any error”
(`ReleasePlan` is a typical example: if the two sides calculate spans in any different way, the
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module is on every entry point's mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator's four entry points never pass through the
  `pipeline.rs` layer.

**Verified fact**: the repository currently has **no mechanism whatsoever** ensuring that these 14
fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride with IR
generation.

### Non-Obligation Field Registry: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (9 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation Fields”
table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                                                                                                                                                                                                                                                         |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (zero occurrences in `src/middle`)                                                                                                                                                                                                                                                                                                                    |
| `diagnostics`             | Error-diagnostic channel (existing channel, not “produced means contractual”)                                                                                                                                                                                                                                                                                                            |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its documentation comment declares no cross-stage contract ⇒ not incorporated                                                                                                                                                                                                                                                         |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                                                                                                                                                                                                                                                     |
| `semantic_db`             | Consumer is LSP (from 4.6.1, emitted through the `DriverOutcome.semantic_db` channel at `src/lsp/server.rs:482`); not a compilation-stage obligation                                                                                                                                                                                                                                     |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`; documentation comment declares no contract                                                                                                                                                                                                                                                                                                  |
| `warnings`                | Warning-diagnostic channel (same class as `diagnostics`)                                                                                                                                                                                                                                                                                                                                 |
| `interface_impl_registry` | In-tree fact (user decision A in 3.4.9; see the “§Obligation Fields” reclassification record for in-phase consumption sites)                                                                                                                                                                                                                                                             |
| `method_def_ir_names`     | Cross-stage mapping (typecheck produces mixed names from method-declaration statements keyed by `PlanId` → IR generation reads only the names, WBS 7c.3); a miss produces a loud E3005 failure with no silent surface; consumption granularity is per declaration rather than per nonempty compilation unit, so the ledger's “unit produced means delivery required” rule does not apply |

**Why this is not “wiring up consumers”**: creating downstream consumers for these fields would
introduce spurious obligations and disguise “cross-stage consumption is not required by design” as a
contract defect. The correct closure mechanism is **reverse containment**—the field set of
`TypeCheckResult` = the obligation inventory ∪ the non-obligation registry. This is mechanically
enforced by criterion 1 of `scripts/ci/check-obligations.py`: adding a field without registering it,
deleting a field without synchronizing the registry, omitting a reason from the non-obligation
registry, or overlapping the two tables all immediately turn the gate red.

### Four Additional Contract Defects in the Proof Layer

The following findings all come from analysis of the proof layer. They are independent of the
entry-point divergence problem, but all belong to the same class: “stage contracts are not
enforced.”

**(a) Layer order is declared only in documentation and is not mechanically enforced; the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (locations are anchored by function name/driver point, with line
numbers taken from the current tree):

| Actual order | Call site                                                                                                                                                                                                                        | Corresponding layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` has its own E2030 semantics; `judge_refined_obligation` is the sole decision unit shared by call-site and return-position obligations) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                  | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                     | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                             | —                   |
| —            | `equivalence::check_type_equivalence`—**zero calls in `checker.rs`**                                                                                                                                                             | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comment and do not correspond to the call order. The README has no “number
→ call timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **no short-circuit exists**—`check_module_impl` adds
each termination error and continues; “a lower layer failing means the upper layer does not run” is
not the actual behavior.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** the hard-failure panic was removed by 3.3.1 in P3 (the SOLVER slot
> was made optional; absence conservatively degrades to `SMTResult::Unknown`); “silent skipping
> (without injection)” was supplemented with a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is to be performed according to **RFC-039 D58**, at the actual repair site
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> interface). The `default_solver() → &'static` formulation in the 02 change list is superseded by
> D58 (a bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumption site                                                                                               | Acquisition strategy                                     | When the solver is unavailable                               |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                           | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected during construction through `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection with `with_solver_owned`        | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                                    | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver's capabilities’.” In contrast, the contract documentation in `backend.rs:60` says:
“`None`: backend unavailable……**callers should conservatively degrade**.” **Two failure philosophies
coexist in the same codebase, and the hard-failure path panics rather than returning an error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path for back-edge-cut decisions. Meanwhile,
`src/frontend/core/sema/proof/smt/backend.rs:67-72` contains:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every call performs `Z3Backend::new()`, creating a
new Z3 context. The `Z3Backend` cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the documentation comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Since it is not shared
across calls ⇒ **the cache never hits in this calling mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. It is the same
backend with two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match actual behavior.** The two
`body_checker` fallbacks in `check_module_impl` (originally `checker.rs:1283-1287` and `1289-1293`):

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership checking receives an **empty type ledger and empty call
table** and still runs—ownership analysis degrades to “nothing conflicts,” **with no warning
whatsoever**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligations ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm State: A Shim Crate Carries the Implementation, and 29 Files Have Live Branches

The wasm target **has been implemented and is built in CI**. The `cdylib` is not in the main crate
but in a separate shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The main crate contains only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                         |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                         |
| The shim depends on the main crate's `rlib` as a library  | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]`; `Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile                                                                                                                               |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI has four wasm builds**                               | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten      | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL; missing files degrade to a warning)                                                                                                                                                                   |
| **29 files** contain the literal `wasm32`                 | Of these, **22** have actual `#[cfg(...)]` attributes, and the other 7 mention it only in documentation (the inventory and per-item determination are in the “F2 Determination” section of `06-cleanup-inventory.md`)                                                                      |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                       |
| `src/lib.rs` has 11 occurrences                           | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                                 |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call under the wasm
target—`src/lib.rs:151/170/187` gate out `run_file` / `run_project` / `build_bytecode` entirely (all
three require `std::fs`), while the shim follows a different path.

**This also establishes a fact directly relevant to this document**: the wasm playground entry point
`compile_playground` in `wasm/src/lib.rs` (`:84`) constructs `Program { kind: WasmPlayground }` and
passes it to `Driver::run` (the JS-side entry point is `run_code`, `:122`)—the **single-file path**.
Therefore:

| Path consuming `proof_calls`                              | Executes proof functions? |
| --------------------------------------------------------- | ------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                   |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                   |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                   |
| Multi-file `run` (`compile_project`)                      | **No**                    |
| `check` (`check_project`)                                 | **No**                    |
| In-project LSP (`check_source_in_project`)                | **No**                    |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** This path is already covered by the unified
Driver through `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the same
single-file stage table as `SingleFile`, `:135`); the implementation record is in the **4.2.8** row
of `09-execution-wbs.md` (`wasm` now uses `ProgramKind::WasmPlayground`, `:274`).

(The cleanup scope—which of the 10 wasm attributes in `orchestrator.rs`—4 positive + 6 negative—are
unreachable in playground scenarios—is assigned to the “F2 Determination” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time; six `not(wasm32)` arms are load-bearing in the three native stage tables; there are no
directly removable low-risk items. The RFC-039 open question “build a wasm target, or delete these
branches” **has expired**: the target has already been implemented.)

---

## Target Design

### 1. Stage Model: A `Stage` Enum That Is Exhaustively Enumerable and Not Registered at Runtime

**Core constraint: `Stage` is a compile-time exhaustively enumerable enum; runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can force at compile time that “a new
stage must be handled by the orchestration layer”—exactly the mechanism already proven effective by
the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency verification          Project
    Discovery,          // File discovery                            Project
    Parsing,            // Lexing + parsing                          PerModule
    Registry,           // Module registry construction              Project
    RoleClassification, // Role classification (Script/Bin/…)        Project
    Typecheck,          // Type checking (including embedded proof layers) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis                 Project
    ProofExecution,     // Compile-time execution of proof functions  PerModule
    GlobalSlotAlloc,    // Global slot allocation                    Project
    IrGeneration,       // AST → ModuleIR                            PerModule
    Monomorphization,   // Monomorphization                          Project
    Linking,            // Cross-module linking / IR merging         Project
}

impl Stage {
    /// All stages. Adding a variant makes this array and the exhaustive match in `dispatch` fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module and which must run once per project” into a
type-level fact. Stages with `Project` scope are structurally guaranteed by `dispatch` to run only
once, eliminating the need for human reasoning about questions such as “should
`allocate_global_slots` be called once per file?” currently present in `orchestrator.rs`.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on the order.

> **Revision note (P4 implementation, 2026-10-07):** two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the initial order contradicted
>    the data flow.
> 2. The Check-form stage table was normalized to the topological order of `Stage::ALL`, with
>    dead-code analysis **before** proof execution: in the current `check_project`, proof execution
>    precedes dead-code analysis; the two have no data dependency, the diagnostic sets are identical
>    (C2 set semantics), and only the within-file diagnostic order is normalized. The single-file
>    path already conforms to ALL order because dead-code analysis is embedded in typecheck and
>    precedes proof execution, so byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved **before** `Registry`/`RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products. The initial order would force the Registry arm to “hide parsing,” making the stage
>    table lie about the data flow; it would also reduce multi-file parsing from two passes to one.
>    The current state is also registered faithfully: multi-file parse errors used to be a hard
>    abort (the `?` propagation in `build_registry_from`, in tension with CollectAll semantics,
>    registered as WBS 4.10.1; `build_registry_from` was removed on 2026-10-26 after its last caller
>    was cleared, so this historical narrative is retained here)—**4.10.1 has been fixed** (user
>    decision on 2026-10-09: Rust-style collection semantics + plan B): on the Check path, parse
>    failures degrade to per-file diagnostic collection, and the affected file exits the compilation
>    unit (does not enter the registry; importers report E5001). The MultiFile path retains hard
>    abort (under FailFast, a bad file cannot produce IR, which is correct semantics and remains
>    pinned in the long term).
> 4. Check-form implementation (4.2.2, user decision on 2026-10-09) added two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events. The initial draft omitted these
>    edges; in the orchestrator era, these products were produced by `discover_with_used` but
>    discarded by the `discover` wrapper, so omitting them from the table lost the data flow. b.
>    Within-file diagnostic-order normalization was expanded to the full form of note 2 above: E3020
>    entry validation moved into the RoleClassification arm (Check has no Linking stage, so the arm
>    holding `surfaces` and the AST performs validation), and the within-file diagnostic order
>    follows the topological stage order (E3020 → typecheck → W1006/W1003/dead code → proof). The
>    diagnostic set is unchanged (C2 set semantics); only the order of stderr entries changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design objective**: turn “a field is produced but nobody consumes it” from an undetectable
condition into a fact that can be asserted at compile time or test time. RFC-039 already identifies
this as “the single highest-value item.”

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

**Settlement semantics (three tiers)**:

| Situation                                      | Criterion                                                                 | Handling                                                                            |
| ---------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| An obligation is read by its declared consumer | The corresponding field in `Obligations` is `take()`-n or marked consumed | Pass                                                                                |
| An obligation is nonempty but has no consumer  | The field is nonempty and not marked consumed                             | **Produce a diagnostic** (W level by default; promoted to E level in `strict` mode) |
| An obligation is empty                         | The field is empty                                                        | Pass (no consumption required)                                                      |

**Why start with W rather than E**: fixing obligation discarding **changes the diagnostic set**. C2
requires that “the diagnostic sets of all entry points be identical.” Promoting directly to E would
cause `yaoxiang check` to suddenly produce a large number of previously silent `Unproven`
diagnostics. Splitting the work into two steps—first W for observation, then E—keeps the criterion
usable at every step. See Implementation Point S4.

**There is exactly one call site for `assert_drained()`**: `Driver::run`, at the end of the stage
table and before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing at
least twice but having no read in a third file → failure). `assert_drained()` is the **runtime**
gate, and the script is the **static** gate; they complement each other, and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed):** this note records the
> **implemented form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a one-to-one correspondence to code file:line. The implementation commit is
> `1941cc47a` (verification tree `99079bc52`). It also implements the third of the three “not
> silent” enforcement points (the other two are the audit surface and the static gate).
>
> **1. Implemented artifacts (file:line)**
>
> - Obligations ledger `src/driver/obligations.rs`: a 13-entry obligation inventory `INVENTORY`
>   (:201; field name/consumer stage/red-list status), **non-obligation field registry
>   `NON_OBLIGATION_FIELDS` (:175, 9 entries with reasons)**, delivery-marker ledger
>   `DeliveryLedger`, pure settlement `settle_obligations` (:577), external emission
>   `emit_obligation_diagnostics` (:654), non-consumer gating `consumer_visible_unit` (:538), skip
>   criterion `consumer_stage_skipped` (:549), IrGeneration delivery set `IR_GENERATION_DELIVERED`
>   (:285), and the W1090 code-string constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path for `Aggregation::FailFast` also settles (“the books
>   must balance”).
> - **Seven delivery points** (one per arm; removing any one makes the runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path for this
>   obligation from `TypeCheckResult`, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —all four use the same
>   delivery interface: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct
>   request-table read) / `:1221` (MultiFile·Check: aggregated by the Linking arm, with
>   `driver/helpers.rs:295` reading each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, with the reason written above the entry at :26-31—the settlement
>   granularity is “compilation unit × obligation,” with no source location a user can point to, so
>   no dummy span is inserted); text for six languages is in `locales/*.json`. **Emission level**:
>   from 4.3.2 onward it is **error** (the same code is promoted to E; see the third
>   section)—determined by the explicit `.severity(Severity::Error)` in `to_diagnostic`
>   (obligations.rs:444); the code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI adds the `obligations` job
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the table above                   | Implemented criterion                                                                                  | Code                   |
> | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------- |
> | An obligation is read by its declared consumer | **A delivery marker exists** (markers ↔ actual reads are reconciled by the static gate; see section 5) | obligations.rs:618-623 |
> | An obligation is nonempty but has no consumer  | Nonempty + consumer stage has run + no marker → W1090 (**error level**, see section 3)                 | obligations.rs:624-630 |
> | An obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                       | obligations.rs:587-591 |
>
> Two **additional tiers** (user decision on 2026-10-13, absent from the sketch):
>
> 1. **S1**: the consumer stage is absent from the `ProgramKind`'s stage table → **not treated as an
>    outstanding obligation** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: the consumer stage is skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not treated as outstanding (obligations.rs:598-602). `NoObligations` **does
>    not** grant an exemption—an obligation is nonempty while the stage itself claims there is no
>    obligation, which means the books do not balance and must be reported.
>
> **3. External-emission policy (decision “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: this mechanism emitted only **W-level** diagnostics (W1090); there
> was **no strict switch**. **4.3.2 (final state, user decision A on 2026-10-13)**: **the same code
> is promoted to E**—the `W1090` code number is unchanged, but the emission level is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (the same fact with two codes would create parallel representations). **The
> promotion changes only the emission level, not the emission conditions.** This is a **D-class
> breaking item under D59** (changes externally observable behavior), registered faithfully: healthy
> programs emit nothing (all obligations have delivery markers), so existing corpora and user
> programs are unaffected (342/342 “difference: empty”); however, when a run genuinely hits an
> undelivered obligation, that diagnostic **counts toward `error_count`, renders as `error[W1090]`,
> and produces a nonzero exit code**, unlike the previous “W level does not block compilation”
> behavior. The coupling of code family and emission level is registered in the “Emission Channels”
> section of RFC-013 (a code prefix determines numbering ownership only; severity is determined by
> the explicit severity at the construction site). Mechanism facts (measured): severity comes from
> an explicit `.severity()`; otherwise, “a code beginning with `W` → Warning”
> (`codes/builder.rs:186-203`), and the registry's `ErrorCategory` **does not participate** in
> severity. Exit codes are determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`); **because of its
> explicit error severity, W1090 counts toward `error_count` (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's decision C on
> 2026-10-13; the current red list is now empty; see section 4), for a reproducible reason:
>
> - Decision A (emit red-list entries as W) has a measured cost: `interface_impl_registry` is
>   populated from typecheck's prebuilt table and is **always nonempty**, so the per-compilation
>   external signal is **constant** and has no discriminating power, yet it breaks three existing
>   external contracts—“clean programs produce no diagnostics / LSP produces no diagnostics /
>   `warning_count == 0`.” `cargo test --lib --all-features` measured **12 existing cases turning
>   red**, of which **8 are on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs`
>   ×5, `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4
>   are under `src/driver/tests/`). The corpus baseline changed in 24/342 fixtures, and the only
>   viable change was “filter W1090 out of the zero-diagnostic assertions,” which weakens an
>   existing criterion.
> - Decision B (downgrade to Info level) still breaks 9/12 of the list-style assertions, and Info is
>   not “emitted according to the decided level.”
> - Under decision C, measurements show **zero changes to existing cases** and **zero changes to the
>   corpus baseline** (342/342 “difference: empty, behavior equivalent”). Runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains completely silent, and removing
>   any delivery marker makes it fire (all three delivery points were tested by removing their
>   markers; CLI `check` emitted two `warning [W1090]` diagnostics when the Check-arm marker was
>   removed).
>
> The red list's “not silent” property is provided by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (included on every run, with WBS IDs and
> evidence pointers); ② `check-obligations.py`, which prints the red list on every run (currently
> empty) and performs stale detection; ③ this note.
>
> **4. Current red-list state (account closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three closure tasks landed on 2026-10-13:
>
> Note: the line anchors below record the file state **as of 2026-10-22**. P5 (2026-10-26) split the
> checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting the
> line numbers in `checker.rs`; the anchors below no longer correspond to the current file and are
> retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates `CreateVariant`”) is equivalently fulfilled by in-place shape detection on the IR
>   side: `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`), with the call site
>   at `:7496` replacing the entire Call with `CreateVariant`; the function's documentation
>   explicitly states that “span keys are unreliable across systems (the two sides interpret a Call
>   node's span differently), so detection is performed in place”—**wiring up a consumer would
>   reintroduce the already-fixed span-mismatch defect**. Removal surface: the `types.rs` field,
>   collection at `checker.rs:1489-1493/:1565`, the collection chain in `inference/expressions.rs`
>   and `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`; type
>   inference for `infer_variant_ctor_call` and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain it only in the
>   in-phase candidate-name table and in `overload_resolutions`). All read/write sites: written
>   in-phase (`environment.rs:205-219`, registered in the same order as `method_overloads` →
>   `checker.rs:972` → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read
>   in-phase (`expressions.rs:4490-4550`, the resolution point; the only hit, `:4550`, writes
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** at `checker.rs:1558`
>   injected into `TypeCheckResult`. The naming fact still has a single expression: the
>   candidate-name table (in-phase) + span-keyed resolution (cross-stage, constructed at
>   `ir_gen.rs:380`, consumed at `:7632/:7705`); mixed names on the definition side are computed
>   independently by `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassify as an in-tree fact** (user decision A; see the
>   reclassification record in the §Obligation Fields table of this document). What was deleted is
>   the documentation comment's **claim** that the field is “mirrored for consumption by the middle
>   layer/LSP” (`types.rs:47-48`); the field and its in-phase consumption are retained
>   (`operator_interfaces.rs:623` operator lookup, seven constraint-solving lookups in
>   `inference/expressions.rs`, and the read-only accessor at `checker.rs:214-217`); direct proof of
>   zero cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not
>   wire up a consumer**: this table is an environment mirror populated by typecheck's prebuilt
>   table and is always nonempty. If LSP genuinely needs it, it must be registered under 4.6.x;
>   creating a consumer for a field that by design does not require cross-stage consumption would
>   introduce a spurious obligation into the ledger.
>
> After closure, **the field census is closed by “obligation inventory ∪ non-obligation field
> registry (`NON_OBLIGATION_FIELDS`, with reasons)”**—the reverse-containment endpoint of gate
> criterion 1 protects this end to end (adding a field without registration / deleting a field
> without synchronization immediately turns the gate red). The red-list **mechanism** is retained:
> stale detection under gate criterion 4 and the `--selftest` `stale-red-list` fixture continue to
> cover its detection surface; future re-registration must go through an explicit decision.
>
> The closure also synchronizes the counts: obligation inventory 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / the §Obligation Fields table in this document). The gate's `--selftest` now
> covers seven fixture categories (adding “unregistered field,” “stale anchor,” and “empty red
> list”), as seen in
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2)** (three more hard criteria than the sketch states)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ the “§Obligation Fields” table
>    in this document (including the table header's count declaration); any drift turns the gate
>    red. The field set of `TypeCheckResult` must also be **exactly equal** to “the obligation
>    inventory ∪ the non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field
>    without registration, deleting a field without synchronization, omitting a reason from the
>    non-obligation registry, or overlapping the two tables all immediately turn the gate red;
> 2. **Read evidence**: every non-red-list obligation must have a real read in its **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings do not count)—this makes
>    “delivery marker ↔ actual downstream read” a hard criterion (the primary risk of mechanism A is
>    precisely drift between the two);
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    conversely, red-list entries must not be marked;
> 4. **Red-list stale detection**: red-list fields must have zero real reads (the emergence of a
>    consumer turns the gate red), and every entry must carry a WBS ID and an evidence pointer;
>    there is also `--selftest`, with built-in negative/positive fixtures: negatives must turn red
>    and positives must turn green (proving that the detector is not inert).
>
> **6. Deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields individually**:
>    the obligation data remains owned by `TypeCheckResult`, and the ledger records only the
>    “inventory + delivery markers + settlement result.” Reason: copying would create two sources of
>    truth for each field (produced respectively by typecheck's checker and the ownership layer),
>    which must inevitably drift; the cross-layer data plane should also not be duplicated into the
>    orchestration layer.
> 2. The sketch's `take()` / “mark consumed” was **not adopted**; it was replaced by emptiness
>    checks through `is_produced` + `DeliveryLedger` delivery markers. Reason: an obligation and its
>    product are the same data, and `take()` would remove what downstream code needs; moreover, 11
>    IrGeneration obligations take the form of “handing the entire `&TypeCheckResult` to ir_gen” and
>    cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: results are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; all
>    other forms route **by severity** (from 4.3.2 onward W1090 is E level)—the single-file path
>    uses `PipelineError::TypeCheck` (the obligations.rs:654 emission arm), while
>    multi-file/embedded forms record an `OrchestratorError` failure separately (otherwise
>    `result.errors` would have no consumer). Reason: returning `Vec<Diagnostic>` would discard
>    form-based routing—Check diagnostics must retain per-file attribution.
> 4. The first tier of the three-tier table changed from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is delegated to the static gate
>    (criterion 2 in section 5).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**; it was
>    replaced by **the same code being promoted to E in 4.3.2** (it is promoted even without a
>    strict switch, because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no read in
>    a third file” to the four criteria in section 5.
> 7. The two additional tiers absent from the sketch (S1 and skip exemptions) are in section 2.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 tables actually landed
>    as **20 test cases** in `src/driver/tests/obligations.rs` (first an `#[ignore]` skeleton, then
>    turned green; the red state is retained in the record: removing the sole settlement point
>    yields `pairs: 0` and six failures). The S1/S4 rows have been synchronized to the actual file.
>
> **7. Implementation record for 4.3.2 (severity + gate hardening)**
>
> - **Inputs for field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in this run, available per run) and the five counters (the conservation
>   identity in section 2); together with the audit surface `findings`, they provide the fact of
>   “whether this field has ever been consumed by any entry point.”
> - **Severity: promotion of the same code to E has landed** (user decision A on 2026-10-13; see the
>   final-state registration and D59 D-class breaking-item declaration in section 3).
>   Implementation: `to_diagnostic` explicitly calls `.severity(Severity::Error)`; the emission arm
>   routes to the error channel by severity. Mutation evidence: removing any delivery marker →
>   compilation fails for that form, and `check` returns a nonzero exit code (`error [W1090]`);
>   after restoration, the healthy state emits nothing (zero corpus changes across 342 cases).
>   Locale text states in all six languages that the code belongs to the W family but is emitted at
>   error level.
> - **Gate hardening items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registry reasons and red-list evidence**, validating that “the file is locatable and unique +
>     the line number is in range + the symbol immediately following the anchor really appears in
>     the anchored file”; an anchor pointing to a nonexistent symbol turns the gate red. The
>     built-in negative fixtures `tests/baselines/obligations-gate/stale-anchor/` contain one
>     nonexistent-symbol case and one out-of-range-line case. Note: the red list is currently empty,
>     so the criterion's actual landing point is the **non-obligation registry** and future evidence
>     pointers (the same code path).
>   - **F2/R3 reverse containment**: the gate outputs “`TypeCheckResult` field count − obligation
>     count − non-obligation registry count = difference”; a nonzero difference turns the gate red.
>     The built-in negative fixture `unregistered-field/` adds a struct field without registering
>     it.
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain a `type_result`, so the obligations
>   do not exist); the semantics of this tier are pinned by a synthetic `State` test, while real
>   end-to-end coverage includes only `ConfigDisabled` (t3 T06).
>
> **8. Closure of 4.3 secondary tasks (red list reaches zero + final E severity for obligation
> diagnostics)**
>
> **8.1 Three implementation rounds (each corresponding to code file:line; this round only records
> conclusions and does not change code)**
>
> | Round                  | Conclusion                                                                                                                                                                               | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
> | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger goes live | `State::assert_drained()` as the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`              | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                |
> | 3.4.4 closure          | Delete `variant_ctor_calls`—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                             | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                            |
> | 3.4.5 closure          | Delete the `method_overload_ir_names` cross-stage mirror (retain it in the in-phase candidate-name table + `overload_resolutions`)                                                       | Resolution consumption site `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                        |
> | 3.4.9 closure          | Reclassify `interface_impl_registry` as an **in-tree fact**: delete the “mirrored for middle/LSP consumption” claim and retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under decision A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, seven sites in `src/frontend/core/sema/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 severity         | **Same code promoted to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; emission routed to the error channel by severity (first failure takes priority)         | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level arm); decoupling of code family and severity registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                         |
>
> **8.2 Closure conclusions**
>
> - **Red list reaches zero**: the number of entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) whose `red_list` is `Some` = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism is
>   **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, stale detection under gate
>   criterion 4, and the `stale-red-list` fixture; the gate outputs “red list: 0 entries—cleared to
>   zero.”
> - **Final E severity for obligation diagnostics**: the **code number and W-family ownership of
>   `W1090` remain unchanged**, but the emission level is `error` (D5 final state, user decision A
>   on 2026-10-13). This is a **D-class breaking item under D59** (changes externally observable
>   behavior), registered in both section 3 of this note and the “Emission Channels” section of
>   RFC-013; **zero emission in the healthy state ⇒ no impact on existing corpora or user programs**
>   (342/342 corpus cases with an “empty difference,” zero IR snapshot drift; t11 independently
>   verified this and t12's review reconfirmed it).
> - **Closed field census**: the field set of `TypeCheckResult` = obligation inventory ∪
>   non-obligation registry (8 entries at `src/driver/obligations.rs:150`, each with a measured
>   consumption rationale); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: all hardening items recorded in this note have **landed**
>   (F1/F2/F4/F5; see sections 5, 7, and 8). The only two items not fully closed are the **known
>   boundary** in 8.4 and t7 F5, which has been converted into an issue; neither is an unimplemented
>   criterion of this mechanism.
>
> **8.3 Disposition of findings from each acceptance/review round (explicitly identified)**
>
> | Source        | Finding                                                                                                                                                                           | Status                                                                                                                                                                                                                                                                |
> | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is a plain-text pointer and can silently become stale if a replacement consumer is removed                                                                      | **Landed**: gate F1 anchor validation (file locatable and unique + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                           |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment; adding a new field without registration silently passes                                                                               | **Landed**: explicit F2 difference output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                     |
> | t4 R4         | The W-only versus promoted-to-E policy was not registered                                                                                                                         | **Landed**: sections 3 and 7 of this note + RFC-013 “Emission Channels” + W1090 help text and error_output examples in all six `locales/*.json` files                                                                                                                 |
> | t7 F5         | Dead-code analysis incorrectly counts references to overloaded methods (valid overloads are still falsely reported as `W1005`)                                                    | **Converted to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (registration in the 4.9 row of `docs/src/dev/architecture/09-execution-wbs.md`)—outside this secondary task                                                             |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in prose comments at the head of a module                                                                       | **Known boundary** (see 8.4)                                                                                                                                                                                                                                          |
> | t11 F8        | Validator-tool-side note: mutation-driven runs once wrote to the main repository by mistake (immediately cleared with `git restore`; that round's output was discarded and rerun) | **Informational item** (not a deliverable defect)                                                                                                                                                                                                                     |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document's table header differ from `INVENTORY`) triggered a red gate                                             | **Informational item / resolved**: the frozen commit passes self-consistently; “an intermediate state directly committed turns the gate red” is precisely the intended behavior of F2/F4                                                                              |
> | t12 F12-1     | This document gives the struct range as `16-73`, but the actual range is `16-76` (after t9 added three comment lines, with no mechanical reconciliation)                          | **Landed + mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; an error immediately turns it red); related figures were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                    |
> | t12 F12-2     | Settlement in multi-file form once replaced the first failure (a type error in `zzz.yx` was overwritten by W1090)                                                                 | **Fixed** (the captain amended the commit message): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression is caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning scope**: anchor validation covers only the text of **registered
>   entries**—non-obligation registry reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` promises in prose comments at the head of a module are outside the scanning
>   scope** (no currently registered entry is affected). If such an anchor promise is written into a
>   module header in the future, it must be handled through **manual review**, or F1's scanning
>   scope must be expanded—choose one; it must not be left unattended by default.
> - **The gate's static boundary**: collection-level validation of delivery markers is on the static
>   side (criterion 3), and removing a marker from an individual arm is covered by **runtime
>   settlement**—the static + runtime two-layer structure is the intended design, not a gap
>   (bidirectional verification through mutation in t11's M11A/M11B/M11C).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed):**
> of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural-identity keys** (`PlanId`) in accordance
> with RFC-039 **D20**. This note records the implemented form, criteria, classification basis, and
> known boundaries; definition lines are still reconciled by gate **F4** (the line numbers in the
> two corresponding rows above are current code facts).
>
> **1. Implemented form (carrier selected A: `PlanId`)**
>
> The key is a **structural identity** that travels with the AST node, rather than relying on the
> positional coincidence of “both sides calculating a span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → ownership of the release plan and consumption) and `Expr::Call.plan_id` (call-site
> identity → production and consumption of overload resolution); `Expr::call` is the repository's
> sole call-expression constructor, and synthetic calls outside the parser are always recorded as
> `UNPLANNED` (**not consumable**; see section 2).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**reading and accounting occur in
>   the same call**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is supplied by
>   `walk_stmt`'s set/restore mechanism + the CFG-node-to-statement-identity table; Drop ownership
>   at scope exit is assigned to the final statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger in the **same
>   object**), produced at `inference/expressions.rs:4550` (written by call-point `PlanId`, with the
>   dispatch path transparently passing the destructured `Expr::Call { plan_id }`), consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) through `OverloadResolutions::get`
>   (a hit is recorded).
>
> **2. Consumption-must-reach settlement (reusing the 4.3 ledger rather than creating a second
> system)**: the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), and each of the two tables owns one instance. When
> `generate_module_ir` returns successfully, both ledgers are marked at the **same point** as having
> “completed one full pass” (`ir_gen.rs:1095` for the release plan / `:1097` for overload
> resolution); the three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point connects one row for each of the two
> obligations—a nonempty difference produces an **E-level W1090** (same-code promotion to E; policy
> in section 3). The exemption criterion is attached to the **ledger's own completion marker** (not
> to whether `unit.ir` exists), so the Check form taking the IR product does not constitute an
> exemption.
>
> **3. D59 classification basis (C class, red before green)**: before the change, the form
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the resolved
> candidate was discarded and execution fell back to the last binding in definition order). After
> the change, the same program fails at compile time with
> `error [W1090] ... produced key was not consumed`. The chosen fix is **loud failure**, not “put
> the resolved name into the call with named arguments”: the latter would lose the named argument
> and merely replace one silent failure with another (final semantics are recorded in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition, `Drop` instructions are no-ops in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty
> differential result across 342 corpus cases is empirical evidence of this.
>
> **4. Criteria and evidence**: D41's equivalent criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference set must be empty + a floor for the
> evidence surface**: the number of files that produced a resolution must be `≥ 1`, the number of
> truly consumed entries must be `≥ 2`, and there is no whitelist). The nine pinned cases covering
> the resolution forms are in `src/frontend/core/sema/tests/overload_resolutions.rs` (expected
> return / positional argument / type-body binding / `?` operand / purely named / positional + named
> / ambiguous / variant constructor / unallocated identity). Mutation-driven verification: shifting
> a consumed key by one position ⇒ the criterion turns red + a real program reports
> `error [W1090]` + the corpus gate reports a difference; restoring it turns green again. Gate
> figures: `check-obligations.py` PASS, differential result across 342 corpus cases is empty, and
> `cargo test --lib` reports 2712 passed / 0 failed (zero IR snapshot drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: the **unbound consumption
> site** at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test surface
> (method declarations uniformly go through `register_type_bindings` to the bound consumption site),
> so the keying correctness of that branch has **no red state with which to falsify it**. The reason
> for retaining it is that it is the symmetric branch of the bound path; deleting it would rewrite
> the form “method call without `type_bindings`” into a silent one. This is registered by
> independent verification (t5 F-1) and review (t6 F2); **no new task is opened**.

### 3. Unified Driver: A Single `dispatch`

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
    /// May only be one of the six predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only return the six predefined combinations above** (one per `ProgramKind`);
callers cannot freely supply an arbitrary array. This is the key constraint preventing the `Program`
abstraction from degenerating into a “freely parameterized anything-goes argument,” and it is
asserted by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines skipping; stage return values do not
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce a diagnostic; see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // …all 12 arms; not one may be omitted
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed):** the four implementation
> deviations from the sketch above are semantically equivalent and recorded for reference:
>
> 1. The three-state `StageOutcome` in §4 is not returned by stage arms; it is expressed through
>    `State` failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are unchanged,
>    eliminating repetitive return values in every arm);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” determination in §5 supersedes the sketch field);
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-diff criterion in S2, “deliberately fix no
>    bugs,” requires this; external emission accompanies the 4.3 obligations ledger;
> 4. The visibility of `proof_execution` is widened to `pub(crate)`: the driver arm is the sole new
>    caller (L1→L2 is an allowed direction); the remaining existing orchestrator call sites return
>    to their proper layer after being migrated in 4.2;
> 5. In multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user decision on 2026-10-09): the original `compile_project` interleaved
>    proof execution within the per-file typecheck loop (file N's proof execution preceded file
>    N+1's typecheck). With a single failure, both are byte-for-byte identical; in the
>    multiple-failure scenario “file 1's proof fails + file 2's typecheck fails,” the first reported
>    error changes from the proof error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries products through channels determined by ProgramKind: `result` (the
>    pipeline contract) / `module` + `failure` (the orchestrator contract, 4.2.1)—the external error
>    contracts of the entry points (PipelineError / OrchestratorError) are not types that the Driver
>    can unify.
> 7. The Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; a clean file has an empty Vec—the existing
>    `check_project` contract); the Discovery product is extended to a “file set + `used_by` edge
>    set + shadowing events” triple in State (the MultiFile form has no downstream consumer, so it
>    is recorded but not emitted externally); the check path parses each file once rather than twice
>    following the C3 topological order.
> 8. The Lsp form landed (4.2.3, source-based routing decision, user determination on 2026-10-09):
>    Parse failures in disk files use the same plan B as Check (collect + exit the compilation
>    unit—degradation converges on both sides); parse failures in the edited buffer retain a partial
>    AST and continue to typecheck (the editor philosophy—an intermediate state while typing should
>    not make semantic functionality disappear). The Discovery arm overrides stale disk content with
>    the buffer source for Lsp; Typecheck in Lsp checks only the target file (other units exist
>    solely to provide signatures to the registry); within-file diagnostic-order normalization
>    extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior, in
>    which LSP hard-aborted on parse errors in unrelated disk files and the handler silently fell
>    back to the single-file path, is fixed by this step.
> 9. The Embedded form landed (4.2.4): `Program` gains a `shared_registry` field—the embedded std
>    module is a child compilation, and its registry is an **input**, not a product (EMBEDDED_STAGES
>    has no Registry stage), making #94's SymbolTable-sharing contract explicit as a program
>    declaration rather than something the caller must remember to pass. The error-path text is
>    normalized from `<std.test> (embedded std)` to the unit's virtual path `<std/test>`
>    (compiler-internal error surface only, with no test pinning it). At this point, all four
>    orchestrator entry points have moved to the Driver.
> 10. Standalone check unification (4.2.5, decision A + IR-stage decision, 2026-10-09): The Check
>     form faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive only errors, preferring omission
>     over false positives); relative `use` paths resolve relative to the importer's directory
>     (aligned with rustc's single-file mod behavior). The CHECK stage table adds GlobalSlotAlloc +
>     IrGeneration: the old standalone path (the full pipeline) already ran IR generation, and
>     errors such as E3019/E1014/E1015 are produced only by ir_gen (confirmed by the runner gate).
>     IrGeneration in Check form performs a pure check and does not consume the IR; the Script/Bin
>     split is represented by the None/Some value of module_key (the existing E3023 switch,
>     ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces only
>     resource-limit/internal errors and has zero corpus dependency). **〔Withdrawn on 2026-10-10
>     after verification in 3.4.8〕**—“zero corpus dependency” was in fact an evidence blind spot
>     caused by the corpus having no pathological-recursion fixture; E3005 is the only compile-time
>     defense for such programs. See note 14. Incidental repair of a latent defect: the Registry
>     arm's redundant harvesting of registrations for embedded std units could replace the native
>     half already merged by `with_std()` (false E1043); embedded units now skip duplicate
>     registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is removed and replaced by
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and the CLI now use the same
>     Driver. The SingleFile+CollectAll form has Parsing collect all parse errors while retaining a
>     partial AST and continuing to typecheck (the editor philosophy extended from the 4.2.3
>     decision; the stage is not marked failed, because topological skipping would prevent typecheck
>     from ever running); Typecheck dispatches the `check_module_collect_all` free function. The LSP
>     single-file path now covers proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and
>     IR-level errors; lexical failures now report real diagnostics (the old synthetic “E0001
>     lexical error” text disappears with the deleted sequence—real diagnostics have precise spans,
>     with no information loss).
> 12. Line-by-line verification of `check_module` / `check_module_collect_all` (4.2.7, mandatory C5
>     item): the two entry points had already converged separately—the mod.rs layer has
>     `check_module_inner(ast, env, collect_all)` with a single Boolean, and the checker layer has
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, with both pass-3 and drain modes
>     running and the difference carried by whether `collected_errors` is empty; there are five
>     collection points in statements.rs (function body/use/for/block/while). Verification also
>     exposed and fixed a duplicate-diagnostic defect: collection points placed the first error in
>     both `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err`
>     produced the same code, span, and message twice (up to three times when nested); two
>     independent mechanisms were also found: duplicate visits to annotation-validation signatures
>     and the whole annotation (E1003/E1103), and duplicate emission at the same point in the
>     ownership layer (E2014/E2018). The fix is at the module-result boundary: deduplication by
>     `(code, span, message)`—diagnostics are positional facts, and duplicates carry no information;
>     duplicate copies were removed from 63 corpus entries, with zero drift in the run list and exit
>     codes (the apparent “duplicate” in borrow_conflict_err is an artifact of the baseline tuple
>     format not containing a column number—the two E2018 entries are legitimate diagnostics at
>     different columns).
> 13. The wasm playground landed (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper layer no longer participates. Error text is aligned byte for byte with the old
>     `CompileError` Display prefix (`Parse error:/Type error:/Internal error:`), with zero visible
>     difference in the playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES +
>     FailFast (the existing 4.1 declaration, with a real production producer from this step
>     onward).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user decision on
>     2026-10-10, **partially withdrawing the mono exclusion in note 10**). Measurement disproved
>     the claim that “mono produces only resource-limit noise”: pathological generic recursion
>     (`f(x)=f([x])`) caused the compiler process to overflow the stack on the multi-file path
>     without monomorphization (0xc00000fd—the interpreter recurses at the Rust level, with no
>     graceful runtime error), and the mono depth gate is the only compile-time defense. The
>     standalone-check decision to exclude mono before 4.2.5 caused a silent-coverage regression,
>     because the old standalone path went through the full pipeline and did have mono (“zero corpus
>     dependency” only proves that the corpus has no pathological fixture). The “type-erasure
>     fallback” assumption was invalidated by the RFC-033 reflection decision—`^^List(Int)` needs
>     the true identity of an instantiation, and monomorphization is the foundation of reflection
>     rather than an optional optimization. Four design points: ① typecheck produces an
>     instantiation request for qualified calls (`lib.f(x)`, in the FieldAccess form), with the
>     generic_id using the qualified name and sharing the merged-IR namespace (currently only bare
>     Var is recognized, expressions.rs:2455); ② the `Stage::ALL` topology changes to Linking before
>     Monomorphization, and the MULTI_FILE stage table gains Monomorphization at the end—it consumes
>     the merged IR and aggregates requests from all units (qualifying the deferred bucket's
>     `containing_fn`), and mono is promoted from a single-module pass to a whole-program pass; the
>     single-file stage table is unchanged (no Linking, so the subsequence property is preserved and
>     IR snapshots do not drift); ③ CHECK gains Linking (pure merging—E3020 entry validation remains
>     in RoleClassification to prevent duplicate reporting) and Monomorphization (pure check without
>     consumption, like IrGeneration in Check form); ④ resource protection covers cross-unit mutual
>     recursion through whole-program BFS. The `Monomorphizer` itself is unchanged (its contract for
>     consuming a ModuleIR + request set already matches the merged-IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to design #14: `ExpressionInferrer` receives a module-qualified key
>     table (registered by the three `use` arms—ordinary / alias / item alias, with only
>     SubModule-class exports eligible), and `callee_generic_name` uses unified qualified-name
>     concatenation to obtain the same source as the merged IR (the concatenation implementation is
>     in `frontend::module::symbol`). The arity criterion on the first path for qualified calls is
>     tightened (when the parameter table cannot be resolved, the second path obtains arguments from
>     the signature, avoiding false E3018 reports caused by mismatching same-named functions across
>     modules). Three asymmetric defects on the mono side were empirically exposed and fixed in the
>     same round: (1) The deletion key (the set of generic names) and rewriting key (the
>     `containing_fn` + name + span triple) had mismatched granularity—a call site whose arguments
>     could not be resolved degraded into a symbol request in the deferred bucket; if the containing
>     function was non-generic, the bucket never drained, yet the original was already deleted by
>     name, leaving a dangling call (runtime E6006 demonstrated by `list_ops.yx:53`). The fix
>     restores the gate `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as
>     the sole source of truth to determine coverage and retaining the original for uncovered sites.
>     (2) The `HashSet` iteration order in the restoration section made the function-table order
>     nondeterministic (t2 review finding F1) → switched to `Vec`, sorted by name, making the output
>     byte-for-byte reproducible. (3) The TailCall / MakeClosure forms outside the rewriting surface
>     still looked up mapping keys (despite comments claiming conservative behavior) → introduced
>     the single-point determination `call_form_is_rewritten`, with every non-rewriting form counted
>     as uncovered. Each has a pinned test (red-state measurements before the fix demonstrate
>     discriminating power). The corpus differential baseline was regenerated in the same commit
>     with exactly one changed line (the `remove_at` frame gained `(int64)`). **Commit-strategy
>     note**: when typecheck collection and the mono fix were split into two commits, the staged
>     tree containing only the former triggered a red `stdlib_docs` check against an existing HEAD
>     defect (`list.is_empty([])` in the same E6006 family)—the two halves are each a prerequisite
>     for the other's green state, so they were combined into one atomic commit.
> 16. Registration of five decisions on 2026-10-10 (after R1 closure): (1) **D2 decision
>     A**—user-module exports carry `generic_fn_type_params_snapshot()` through
>     `Export.type_params`, isomorphic to the std path (`yx_sources.rs:62/108`): the declared name
>     becomes the authoritative source for qualified-call monomorphization, and signature inference
>     becomes a fallback (folded into R2's scope). (2) **HKT is live on the declaration side but
>     broken on the call side** (inferring type arguments E1002 / explicitly supplying type
>     arguments E1010) registered as WBS 3.5 (three level-3 tasks). (3) **The deferred bucket never
>     drains under a non-generic container** folded into acceptance subitem ② of 3.4.8: the bucket
>     must drain or fail loudly and must not silently remain unspecialized. (4) **Calling a closure
>     variable by name fails at runtime with E6006** registered as WBS 3.6; the semantics are (i)
>     support indirect calls / (ii) reject at compile time, and **(i) has been decided**—support
>     indirect calls: the `CallDyn` mechanism already exists (the closure-expression path at
>     `ir_gen.rs:7945`); the broken part is that a `Var` callee is downgraded to a by-name `Call`,
>     even though typecheck already accepts such programs. (5) **The trade-off criterion is
>     fixed**—a proposed approach may be rejected only on correctness and readability grounds;
>     “large change surface / introduces a new mechanism / high cost” does not constitute a
>     rejection reason. Record this in the second part of `coding-rules.md` and the HOWTO self-check
>     table; trade-offs requiring a user decision must follow all four items in `AGENTS.md` under
>     “Requesting a User Decision (Mandatory)” (situation + evidence / pros and cons for each option
>     / explicit recommendation + cost / explicit identification of breaking changes).
> 17. 3.4.8 ② landed and the F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the design in #14: `Stage::ALL` was changed so that Linking precedes
>     Monomorphization, and MONO was appended to the MULTI_FILE table; the Linking arm aggregates
>     whole-program requests **before taking each unit's IR** and qualifies `containing_fn`
>     (otherwise the deferred-bucket key and merged IR have different sources, and the bucket never
>     drains). **① closure (D2 decision A)**: user-module exports carry a declaration-time
>     type-parameter snapshot through `Export::type_params`, isomorphic to the std path
>     (`yx_sources.rs:62/108`). **Existing defects exposed and fixed during implementation**: (a)
>     **F1**—`qualify_names` rewrites only function bodies, and `ir.init` is never visited, so in
>     multi-file programs function definitions are qualified while top-level module calls remain
>     bare names → runtime E6006 (**reproduces even for non-generic code**, independently of
>     generics). The fix makes the init section reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono side's “globally unique suffix” heuristic and silent
>     `continue` on a miss → remove the heuristic and change misses to a **loud E3018** (silent
>     ambiguous mismatching is worse than failure to resolve). **Independent verification caught and
>     fixed a regression**: t6 found that a named import `use lib.{pick}` followed by a bare-name
>     call was falsely rejected with E3018 (the IR instruction name is the alias-qualified name,
>     while the request name is bare; the aggregation side qualified only by unit key) → the
>     aggregation side now **directly invokes ir_gen's** `import_aliases_from` and
>     `resolve_call_target` (extracted as a public single point, one producer and one consumer),
>     with matrix-pinned tests covering both named-import forms. **Record**: the blank `DefId` in
>     the init section was empirically confirmed to be a “consistency blank” rather than a live
>     defect (the codegen name fallback resolves, and all four forms of init call were measured
>     successfully), and was registered as WBS 3.7.4 to close the split between “function bodies use
>     def, init uses names.”
> 18. Closure of the init-section DefId policy split (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     was extracted into the single point `resolve_static_targets` (Call/TailCall string operands +
>     MakeClosure), and function bodies and `ir.init` share both the implementation and the
>     `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)` pairs
>     changed (other-lines 0; zero `(def=_)` remnants anywhere in the tree's init section). Under
>     both the codegen DefId primary path and the name fallback, translated results and execution
>     values are identical item by item (all four forms of init call measured); the corpus baseline
>     has zero drift. The probe flips under the new policy and is strengthened (identity + precise
>     index + execution value; zeroing init coverage turns it red). **Correction (2026-10-11)**: the
>     commit IDs above were rewritten by `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396
>     correspond respectively to the rebased 375789d2/d922a551/38bd4b9e; the new IDs are
>     authoritative in the pushed dev history (the old IDs were local temporary IDs and never
>     entered remote history).
> 19. 3.4.8 ③④ landed and verified (2026-10-11, 8ecf56f93). (The design basis for ③④ is in #14; the
>     implementation of ② is in #17, which this entry closes.) **Implementation**: the end of
>     `CHECK_STAGES` = `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs only
>     pure merging (request aggregation + `link_module_irs`), with the entire entry-validation block
>     left to RoleClassification; `ir_generation_check` retains `us.ir` after generation (memory
>     only); mono failures enter the collection channel in Check but retain `fail_with` in
>     MultiFile; `into_outcome` first takes `check_diagnostics` and then evaluates failures
>     (structural failures no longer clear already collected diagnostics). **Verification**: four
>     rounds of independent verification (t3/t4/t10 + a t5 cross-form matrix) and one review (t6)
>     all passed; the differential result across 340/340 corpus cases was line-for-line IDENTICAL,
>     with zero IR snapshot drift and test count increasing from 2643 to 2656. **Existing gaps found
>     and fixed in the same round**: F1 (in a rootless multi-file check, request names and IR
>     definition keys have different sources → silent failure to specialize; see the implementation
>     note in the WBS 3.4.8 row)—the fix precisely reverse-looks-up the actual definition key on the
>     aggregation side (not a heuristic); the first implementation likewise used a precise reverse
>     lookup (`flat_defs`); **same-day correction**: that bridge was removed during the regression
>     fix, and the root cause was instead solved by decoupling `module_key`'s dual role
>     (`script_mode`), with all three using the same source `{unit key}.{name}`. The location and
>     fix for that regression (the CI documentation-example gate went from 258 blocks / 10 failures
>     to 0 failures) are in the implementation note in the WBS 3.4.8 row. **Record of existing,
>     non-regression items**: E3005's help text does not match the resource-protection semantics;
>     the multi-file E3005 span label is misaligned (reproducible with the HEAD binary); the
>     type-argument position on the request-collection surface depends on the same surface and is
>     assigned to 3.5.
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed, with diagnostic-surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is reachable)**:
>     interpreted calls were changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push a frame and return
>     control to the outer `step_one`; the frame carries `ReturnTarget{Host|CallerSlot|Discard}`,
>     and on return the value is written back to the caller's slot, with multiple calls in one frame
>     continued through `pending_calls` in the frame. Baseline measurements showed that each
>     interpreted call level consumed **2544 B** of native stack, with the crash threshold at
>     `count_down(394)` (1 MiB main thread on Windows); after the fix, 20,000 levels of recursion
>     run on a 512 KiB stack thread (approximately zero per level). The guard (`push_frame`) is
>     therefore reachable from every entry point at every `max_stack_depth` setting;
>     `ExecutorError::StackOverflow` now carries the real limit (formerly `util/diagnostic/mod.rs`
>     always passed 0). **The original 3.11.2 (truncating deep-stack backtraces) was merged into
>     this task by user decision** (same file, and it cannot be split by file): the backtrace now
>     shows the first 16 frames + an elided count + the last 8 frames, with shallow stacks unchanged
>     frame by frame. **3.10.1 (no compile-time guard for native recursion, #447)**: locate before
>     changing—the crash point was recursive `MonoType::type_name` (**328 B/level**, determined
>     using an SP probe + critical operations at each level); the same path also has a derived
>     `Clone` (~400 B/level), a deep copy inside `substitute`, `type_name_hash` (164 B/level), and
>     `collect_generic_type_refs`. After converting these four sites to work-list processing and
>     removing one full-request deep copy per level, pathological generic recursion with
>     `mono.max_depth=5000` produces **E3005 (with the real limit)** instead of overflowing the
>     stack. **3.11.1 (task-boundary error-code fidelity, #448)**: the task-result error side
>     changed from a value to a **structured form** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), with values restored by value across the boundary;
>     E6001/E6003/E6004/E6008 inside tasks are identical character for character in Standard and
>     Embedded messages. Two **pub API payload-shape changes** are marked `BREAKING CHANGE` in the
>     commit. **Commit-strategy note**: 3.9.1 and 3.11.1 are **indivisible at file
>     granularity**—`interpreter/executor/executor.rs` contains both the trampoline and the
>     error-channel producer/consumer, while `TaskFailure` is defined in `runtime/engine.rs`;
>     staging only one side cannot compile (pre-commit compiles the staged subset and rejected it
>     twice), so they were combined into **one atomic commit** under the same policy as #15. **Known
>     boundaries and follow-up**: the derived Drop operation (~75 B/level) means
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation) gate applies to
>     native recursion (each level ~3696 B, and the default 256 levels ≈ 924 KiB), but it currently
>     has zero call sites and is unreachable in production; wiring it in the future requires the
>     same family of closure work. The Standard-mode task interpreter is constructed with the
>     default `ExecutorConfig`, so its E6004 limit is always 1024 (configuration propagation
>     registered as WBS 3.12, user decision B: a separate team); the test-runtime budget was opened
>     as issue #449 (a pathological fixture takes 76 seconds, and clippy runs both targets and the
>     commit runs the full suite), outside this RFC's implementation surface. **Corpus directory
>     stratification (same round, 8605a081b)**: 212 fixtures in `02-type-system` (78) and
>     `06-compile-errors` (134) were moved into 23 category-specific subdirectories, and 88 IR
>     snapshots were moved in mirrored paths; the corpus baseline was regenerated in the same commit
>     (342 lines, independently recomputed and compared line by line: only the `file` field and two
>     new changes), and 41 snapshots were regenerated because their file-header paths became longer,
>     with a line-by-line verification showing **only span-offset changes** (zero differences after
>     offset normalization).
> 21. LSP semantic-data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, and cross-file references in a project carry
>     `resolves_to` pointing to the true definition—the old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics went through the unified Driver pipeline,
>     while semantic data went through a manual lex→parse→single-file typecheck in server.rs (with
>     no module registry, in-project `use` resolution failed, reference events were not generated,
>     and cross-file navigation/rename/find-references silently returned nothing—the silent failure
>     registered as D55 is closed in this task). **Implementation**: (1) `Export` carries
>     `definition_span` (harvested from AST identifier nodes by `extract_module_info`, from the same
>     source and with the same value as typecheck's definition-event `def_span`; native std always
>     has None), and `ModuleInfo` carries `definition_file` (filled by the Registry arm from
>     DiscoveredFile); (2) when the checker drains reference events, it uses `import_def_locations`
>     to redirect `resolves_to` for cross-file imported items from the use line to the actual
>     definition location in the defining file (std imports have no registration and continue to
>     resolve to the use line); (3) the checker's module identity changes from the unified
>     `"<module>"` to the compilation-unit file path (this value is consumed only by the
>     semantic-data surface, with zero diagnostic consumption—verified across all four
>     `env.module_name` consumption sites); (4) `DriverOutcome` gains a `semantic_db` channel—fed by
>     CollectAll forms (the Lsp target file regardless of diagnostic success + the SingleFile
>     fallback); the Lsp arm injects top-level export definition surfaces for non-target files from
>     the registry at the end (`external_definitions`, enabling navigation even for unopened files;
>     type/submodule exports are not injected—typecheck-side type registration does not produce
>     definition events, and both sides consistently retain the same absence); (5) orchestrator
>     gains `analyze_source_in_project` (`check_source_in_project` delegates to it and takes only
>     the diagnostic half); (6) the LSP-side `analysis.rs` single-compilation entry point replaces
>     the manual pipeline (`update_semantic_db` is deleted from server.rs), and World keys are
>     unified as normalized file URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is idempotent;
>     drive-letter case and %XX encoding differences normalize to the same key). **Incidental fix
>     (D59 routing class B—old behavior did nothing silently and did not change the externally
>     visible behavior of any program that previously succeeded)**: cross-file
>     navigation/rename/find-references for files in a project now find the true definition instead
>     of silently returning nothing; when lexical errors exist, the old manual pipeline's deletion
>     of semantic information is replaced by retaining a partial AST for coloring (an extension of
>     the editor philosophy from 4.2.6); when the target file has type errors, semantic data is no
>     longer discarded together with `type_result` (coloring/navigation does not disappear while
>     typing). **Pinned tests**: six driver-level tests (target-file path key / cross-file
>     resolves_to + external-definition round trip / std-import fallback to the use line /
>     error-file retention / single-file fallback path key + error retention) and one LSP end-to-end
>     test (red before green, empirically demonstrating that navigation previously returned None),
>     plus four uri-normalization tests.

**Changes required for each of the ten entry functions (specified individually)**:

| Entry point                                                | After the change                                                                                                                                                                                          |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to the Driver                                                                                                           |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                   |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a constructor for `Program { kind: MultiFile, aggregation: FailFast }`                                                                                                                          |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a constructor for `Program { kind: Check, aggregation: CollectAll }`                                                                                                                            |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a Driver call + filter the result to the target file                                                                                                                                            |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                 |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                    |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                       |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In a project, use `Program { kind: Lsp, aggregation: CollectAll }`; **delete the manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                    |

**`check_project` and `compile_project` have approximately 40% overlap / 60% divergence.** What they
share is: vendor-consistency verification, file discovery, `build_registry_from` (removed on
2026-10-26 after its last caller was cleared; this historical narrative is retained),
`all_method_bindings`, the parsing loop, per-file checker assembly, and result handling. What
diverges is items 2/3/4/5/7/8/9/10/11 of the 11-item table above. **This 40% shared skeleton is
precisely the Driver's value range**—all of the divergent 60% consists of either “a different set of
stages” or “a different aggregation mode,” which the two `Program` fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had a non-blocking issue; continue
}
```

**Key design decision: “an upstream failure causes this stage to be skipped” is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If each stage returned
`Skipped` itself, the information about “why did I not run?” would be scattered across the 12 stages
and could not be centrally audited. Instead, `dispatch` determines this at the start of each loop
iteration according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability addressed by
this document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip” that
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every case of “Y did not run because X was not done” must be explainable.

Diagnostic wording must distinguish three skip reasons:

| Reason                                                           | Wording direction                                                                                                                         |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                    | “Stage Y did not execute because upstream stage X failed”                                                                                 |
| Caused by an empty prerequisite obligation                       | “Stage Y did not execute because there were no pending obligations (normal)”—**severity = Info; does not count toward the warning count** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y did not execute because it was not enabled by configuration”                                                                     |

The second category is critical: it distinguishes a “normal skip” from an “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract
depends on this count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                               | Current equivalent |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)       | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493`, `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project and single-file files in the same process, while the CLI's `run` and `check` are two
independent invocations; placing these on Program avoids mutable global state in the Driver.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether to
return early”—they are two different checker entry points. After unification, a single
implementation should be driven by the `Aggregation` parameter rather than retaining both functions
(**this is an additional item requiring verification in this document; see “Known Limitations and
Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None.** It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types do not change**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language's type system                                                                                            |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not the ownership module of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the remaining field types stay
in their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers (for example, `ir_gen` reading `release_plan`) continue to receive
the concrete field types through parameters and **must not `use crate::driver`**—consistent with the
driver no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not `use crate::driver` in
reverse**. A `Driver` appearing in a `TypeChecker` import is a violation, intercepted by
`scripts/ci/check-module-boundary.py` as proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | Five stages                                 | The same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone programs—it uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven by the rootless driver; this row now describes only the `lib::run_file` / `Compiler::compile` library path)                                                                                                                                                                              |
| `yaoxiang run` (multi-file)         | No proof_execution and no W1001/W1002/W1003 | proof_execution was already connected by the P3 stopgap; **dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the W-code set of multi-file `run` is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel”; Check / Lsp / MultiFile share the same implementation, with no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for external emission by that arm) |
| `yaoxiang check` (multi-file)       | No proof_execution                          | Already connected by the P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| LSP (in-project)                    | No proof_execution                          | Already connected by the P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**                | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see Compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Z3 not installed + multi-file       | Silently skipped                            | Unified as above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |

**The only intentional behavior break** is replacing the `.expect()` at `predicate.rs:35` with a
returned diagnostic. This conforms to RFC-027 §8's “not bound to a specific solver” positioning and
to the contract already declared in `backend.rs:60` (“callers should conservatively degrade”)—the
current implementation contradicts its own contract documentation.

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

| File                                          | Line range                                      | Change                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                  | `140-147`                                       | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                              |
| `src/lib.rs`                                  | `154-167`                                       | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                    | `141-227`                                       | Replace the `Pipeline::run` function body with a `Program` construction + Driver call                                                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                    | `275-278`                                       | Move dead-code analysis out and into the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                    | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                    | `381-389`                                       | Move monomorphization out and into the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`         | `450`                                           | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`         | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                       | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                           |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                       | Delete the manual stage sequence in `run_diagnostics` and call the Driver                                                                                                                                                                                                                                                                                                  |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                       | Unify `check_files_with_diagnostics` around `Program { kind: Check }`                                                                                                                                                                                                                                                                                                      |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                       | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                             |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they implicitly reach path 1 through `Compiler::compile_with_source`)                                                                                                                                                                                                        |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                         | Replace `.expect()` with a returned `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent discard point**)                                                                                                                                                                                                                                              |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/checker.rs`           | `1244`                                          | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; delete “a lower layer failing means the upper layer does not run” (no short-circuit exists)                                                                                                                                                                                                                |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure interface; change the three consumption sites in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared interface. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 supersedes the original wording |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated in P5 5.2.1)—the logic itself is correct;
**the problem is on the consumer side, not the production side**, and changing the production side
would conceal the architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch
(wasm branch reachability is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                                   | Handling                                                                                                                                                            |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently will now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches, with the expected diagnostic-set changes recorded in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that previously compiled now produce warnings            | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                      |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                          | W first, then E; split into two steps under S2/S4                                                                                                                   |
| Replace the `predicate.rs` panic with a diagnostic               | **Improvement**: no longer crashes                                              | No break                                                                                                                                                            |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                                   | These two paths do not pass through proof_execution                                                                                                                 |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                        | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list above                               |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness vulnerabilities)** and **P4 (Stage contracts and
unified Driver)** in RFC-039's global stage sequence. S1–S5 below are the implementation order
within this document, and **the acceptance criterion for every step cites the C2 category
(orchestration change) of the [Equivalence Criteria Document](07-equivalence-oracle.md)**: the
**diagnostic sets** of all entry points are identical + corpus behavior is identical.

### S1: Establish the Criteria (Red First)

**Prerequisite; cannot be skipped.** The vulnerability criterion must first be written as a failing
test.

| Deliverable                                                                                                                                                      | Acceptance                                                                                                     |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red**. If it unexpectedly passes, the vulnerability analysis in this document must be reviewed again |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions are comparable                           |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                               | Marked `#[ignore]`; turn green in S4 (red-state record: removing the sole settlement point yields `pairs: 0`)  |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` Without Changing Behavior

| Deliverable                                                | Acceptance (C2)                                                                                 |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs`   | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| Change `pipeline.rs:141-227` to call the Driver            | **The single-file path's diagnostic set and exit code are identical byte for byte**             |
| Differential test across the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr—all **zero-diff**    |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` the single commit; `pipeline.rs` returns to its original state.

### S3: Merge the Orchestrator's Four Entry Points + Fix the Vulnerabilities

| Deliverable                                                                                                                  | Acceptance (C2)                                                                                 |
| ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                              |
| Delete the LSP manual stage sequence                                                                                         | LSP and the CLI produce the same diagnostic set for the same file                               |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                | The vulnerability is fixed                                                                      |
| Fix the ownership layer's empty `Unproven` match arm (originally `checker.rs:1313`)                                          | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode           |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | No longer panics in an environment without Z3                                                   |

**Rollback point**: the vulnerability fixes and the structural merge must be **two separate
commits**. If the structural merge causes problems, only the structural commit can be rolled back
while retaining the vulnerability-fix commit—in that state,
`test_multifile_proof_obligation_not_dropped` remains green; the reverse—retaining the structural
change and rolling back the fix—turns it red, which is an unacceptable intermediate state and must
not be merged.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                                                                                                       |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implemented form in the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                     |
| Promote obligation diagnostics from W to E   | After the full-corpus differential test, manually review every new E                                                                             |

**Rollback point**: the severity of `assert_drained()` can be controlled through a configuration
option; switching between W and E does not require changing the code structure.

### S5: Finish the Proof Layer and wasm Cleanup

| Deliverable                                                                                                                                                                                                                                          | Acceptance                                                                                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                     | The documentation corresponds one by one to the call sites in `checker.rs`                                                                                                   |
| Process-wide shared singleton for `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes sharing between cargo test threads) | Production-path `default_solver()` call sites reach zero (tests only); cache hit rate is observable across all three consumption sites (the counter has landed in 89576fafd) |
| Add a warning to the degradation path at the `TypeChecker::ownership_inputs` criterion point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                  | A diagnostic is produced when there is no `body_checker`                                                                                                                     |
| Reachability determination for the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                 | The conclusion is delivered to the “F2 Determination” section of `06-cleanup-inventory.md`; this document records only the determination requirement                         |

**Note**: correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item must proceed independently of S1–S4** and must not
be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage model**          | `Stage` is a compile-time exhaustively enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                             | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail compilation. This is isomorphic to the mechanism already validated by the opcode table in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that currently require human reasoning                                                                                                    |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement together with the **static** gate `scripts/ci/check-obligations.py`; **the final severity is E level** (user decision A on 2026-10-13: same code promoted to E, code number unchanged; **implemented form in the §2 revision note**: the ledger does not copy the field structure; the red list—zero entries after closure—retains the mechanism but does not enter the user-visible diagnostic stream) | It fixes an entire class of bugs, not one bug: at least two instances of the same class have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are in scope. Splitting W → E into two steps keeps the C2 criterion usable at every step—now that the obligation inventory has been reduced to 13 entries and the red list is empty, zero emission in the healthy state makes E the final choice directly (see sections 7 and 8 of the §2 revision note) |
| **Repair scope**         | Repair the **consumer side** (the orchestration layer); **do not change** the production-side logic of the `Unproven` branches (now three branches in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                             | The problem is on the consumer side, not the production side. Changing the production side would conceal the architectural defect behind “the logic has been fixed,” while the logic in these branches is itself correct                                                                                                                                                                                                                                                             |

This design additionally eliminates two classes of cracks:

- **The crack between “comment promises” and “code behavior.”** The “silent pass must not be allowed
  to return” promise in `checker/refinement.rs` (originally `checker.rs:5165`) is a **comment-level
  contract**, enforced by `assert_drained()`.
- **The crack of “an array entry was forgotten.”** Today, five functions each manually implement the
  call sequence. After unification, there is one stage table, and a missing new stage causes a
  compile-time error.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **involve no type connection**; they are configuration questions about
  “whether an analysis should run.” This approach also cannot express `Aggregation`; it merely moves
  the `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this eliminates
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail compilation,
  replacing “five functions each manually implementing the calls” with “an array entry was
  forgotten,” precisely the cause of the current 11 inconsistencies. The repository already contains
  a counterexample of the same form: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` /
  `HotReloader`, described in `docs/src/dev/design/check/`, have zero implementations.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields; `Stage` remains scattered across five functions, and the next
  point of divergence will continue to grow from here. **It must be done first** (as part of S1/S3),
  because the structural refactoring needs a known red test to prove the criterion is effective.
- **Make `proof_calls` public and add a `debug_assert`**—`check_module` is a general entry point and
  does not know who the caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must occur there.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactoring that “centralizes
  the wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”—they are real bugs that
  simply were never reported. This must be stated clearly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual, field-by-field judgment.** Some
  obligation fields, such as `module_namespaces`, may be designed not to require consumption and
  should not trigger warnings. S4 must review every field rather than applying a one-size-fits-all
  policy.
- **The `Program` abstraction may be premature.** If an entry point's stage set remains unstable for
  a long time, `stages()` may degenerate into a free parameter that supplies a different array on
  every call, defeating the contract. **Mitigation: `test_program_stage_coverage` asserts that
  `stages()` can only come from the six predefined combinations.**
- **Criterion dependency**: the vulnerability-fix criteria in S1/S3 depend on the vulnerability
  tests in the [Equivalence Criteria Document](07-equivalence-oracle.md) first turning red. **If
  that document has not established the multi-file corpus layer, S1 cannot be accepted**—all 293
  existing corpus files take the single-file path and provide zero coverage for this class of
  defect. This document does not cover the IR verifier (`verify_loose`); that prerequisite work is
  outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse of
  their declared order). This is a diagnostic-set-change risk and must proceed independently.
- **Thread safety after converting the SMT backend to a singleton is undecided.** `backend.rs:13-19`
  states that `Solver` is `Send` but not `Sync`, and the `Z3Backend` cache uses `RefCell`; after
  converting it to a singleton shared across compilation units, every access path must be confirmed
  to pass through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependency defects** (such as the actual order
  dependency between termination and ownership). It should be considered only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for each
> decision. **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch-reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three criterion layers,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — explicit “silent pass must not be allowed to
  return” declaration (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — layer-order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — “callers should conservatively degrade”
  contract declaration (contradicts `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-89` — 14 obligation fields that downstream code must consume
  (after closure of 3.4.4/3.4.5/3.4.9)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — the unit test's own
  acknowledgment that it “stops before pipeline.rs”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  the corpus's own acknowledgment that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiles via `run_code`)—the playground's fourth
  compilation caller (the single-file path, so proof_execution executes)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen are in the shim crate rather than the main crate
- `build.rs:19-55` — the error-code build-time gate, an example of this project's enforcement
  mechanisms
