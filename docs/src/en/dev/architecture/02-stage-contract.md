# Compilation Stage Contract and Obligation Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the body of RFC-039 for the four-layer model, acceptance-criterion tiers, and execution-stage
> ordering; see [this directory's index](index.md) for the role of each companion document.

## Purpose and Scope

This document is the construction blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“Which compilation stages ran, which did not, and why?” is currently wired manually
across six paths and ten entry-point functions, with no single place able to answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are broken by the architecture, not by the logic.**

The comment in `revalidate_refined` within `checker/refinement.rs` originally read as follows
(formerly `checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compilation error, no degradation,
no silent pass… silent pass must not be allowed to return.” However, this invariant holds only on
the **single-file path**: proof-function constraints such as `x: Sorted(3)` are **silently
accepted** on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is
never executed, and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustively enumerable, not registered at runtime) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- A unified `Driver` (single `dispatch`) and the individual migration path for each of the ten
  entry-point functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- Every file and line range touched by these changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency-direction rules, and anti-bounce gates → `01-routing.md`
- Equivalence criteria (C1–C6 tiers and three levels of criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch-reachability cleanup → `06-cleanup-inventory.md`
- Global execution order for P1–P10 and G1–G10 acceptance gates → body of RFC-039

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** to perform it. This
document defines the **concrete shape**, **file-by-file change list**, and **internal implementation
stages** of L1, corresponding to P3 “Fix correctness defects” and P4 “Stage contracts and unified
Driver” in RFC-039's global sequence. If this document conflicts with RFC-039, RFC-039 takes
precedence.

Equivalence criteria are applied under **C2 (orchestration changes)** in the
[Equivalence Criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> This entire section consists of **verified facts**, each with a file path and line number. Line
> numbers are based on `9e02e4db`.

### Stage boundaries are the only structural defect where “one incorrect wiring causes the entire repository to remain silent”

This differs in nature from the other two classes of defects, module boundaries and test wiring:

| Defect                | Typical manifestation               | Signal                   |
| --------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntax error  | Incorrect source code               | Diagnostic               |
| Type mismatch         | Incorrect type                      | Diagnostic               |
| Missing stage wiring  | A stage is never invoked            | **No signal whatsoever** |
| Unconsumed obligation | A field is populated but never read | **No signal whatsoever** |

The latter two share a defining characteristic: **failure does not produce an error**. They
therefore cannot be solved by “writing the code more carefully” or “reviewing more rigorously”—code
review can show what was written, but not **what was omitted**. This is precisely the root cause
identified in RFC-039: “This project treats ‘design’ as a documentation convention rather than an
executable constraint.”

### Enforcement mechanisms already present in this repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes and 8 W codes) are subject to
  a **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`, which compares every
  entry against the RFC-013 code table and rejects compilation with `panic!` on any mismatch.
- `src/package/` has a high test density (**76 files / 13,012 lines**, including 6,227 lines under
  `tests/`, or 47.9%). Every module's test subtree is explicitly wired, making this the most
  completely wired area of the repository and a useful structural reference for stage-layer gates.

**The design capability is sufficient. What is missing is “an equally strong gate at the
orchestration layer.”**

(Line-count convention: `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry-point functions

| #   | Path                       | Entry-point function                                  | Location                                         | Compilation flow                                                                   |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages invoked directly                                                       |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **LSP within a project**   | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c within a project; otherwise manually lex→parse                             |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages on the single-file path (`pipeline.rs`) are:

| Stage               | Invocation site | Implementation                                                            |
| ------------------- | --------------- | ------------------------------------------------------------------------- |
| lexing              | `149`           | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`           | `run_parsing`                                                             |
| typecheck           | `173`           | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`**   | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`           | `run_ir_generation`, with **monomorphization embedded in it** (`381-389`) |

### Eleven stage-coverage inconsistencies

The following table provides evidence cell by cell. **An empty cell does not mean that the entry
point “does not do this”; it means that “no code in the entry point does this”**—which is the
essence of the defect.

| #   | Stage / behavior                                         | `pipeline` (single-file)                                | `compile_project`                                          | `check_project`                                              | `check_source_in_project` (LSP)    | `compile_embedded_module`                   |
| --- | -------------------------------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------ | ---------------------------------- | ------------------------------------------- |
| 1   | **proof_execution**                                      | **Present** `187-203`                                   | **Absent**                                                 | **Absent**                                                   | **Absent**                         | **Absent**                                  |
| 2   | Dead-code analysis                                       | Present `275-278` (gated by `config.dead_code.enabled`) | **Absent**                                                 | Present `356-377` (role-aware, no configuration gate)        | **Absent**                         | **Absent**                                  |
| 3   | W1006 local-module shadowing                             | **Absent**                                              | **Absent**                                                 | Present `336-348`                                            | **Absent**                         | **Absent**                                  |
| 4   | W1003 unused import                                      | Present (`277` collects `type_result.warnings`)         | **Collected but never emitted**                            | Present `350`                                                | **Absent**                         | **Absent**                                  |
| 5   | W1001/W1002 dead-code family                             | Present (same as 2)                                     | **Absent**                                                 | Present (same as 2)                                          | **Absent**                         | **Absent**                                  |
| 6   | **Monomorphization**                                     | Present `381-389` (gated by `config.mono.enabled`)      | **Absent**                                                 | Not applicable                                               | Not applicable                     | **Absent**                                  |
| 7   | Typecheck branch                                         | `check_module`                                          | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collects all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                     |
| 8   | File discovery                                           | Not applicable                                          | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                 | `discover` (`454`)                 | Not applicable                              |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **Absent**                                              | **Absent**                                                 | Present (`321-328`)                                          | **Absent**                         | **Absent**                                  |
| 10  | Global slot allocation                                   | Not applicable                                          | `allocate_global_slots` (`147`)                            | **Absent**                                                   | **Absent**                         | **Absent**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Present (`205`)                                         | Present (`152-236`)                                        | **Absent**                                                   | **Absent**                         | Present (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Differences in the current state
> after P3 landed are as follows:
>
> - **Row 1 has been fixed**: all five entry points use the same `frontend/proof_execution.rs`
>   implementation for proof_execution (pipeline + four orchestrator entry points); the
>   unique-consumer defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and the dead-code-family warnings, and the W-code sets of four paths
>   (single-file run / multi-file run / single-file check / multi-file check) are normalized—the
>   single source of truth is the shared implementation on the `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk not yet proven to be a defect).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (`Aggregation` parameter-driven).
> - The “#434 ruling” referenced by the code previously had no documentation registration—now
>   registered as RFC-039 **D57**.
> - **Asymmetry in embedded std inclusion (new fact outside the table, recorded on 2026-10-09)**:
>   the single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required
>   for desugaring `for` loops; hard switch under #117), whereas multi-file `discover` previously
>   recognized only explicit `use`—`for` loops in a project compiled successfully and failed at
>   runtime with E6006 (confirmed by a probe). This was fixed and WBS 4.10.2 was closed. The related
>   “multi-file path lacks the single-file path's one extra step”—the parse hard-abort
>   peculiarity—was registered as WBS 4.10.1—**and was also fixed** (immediately after 4.2.2: the
>   Check path was downgraded to per-file collection; this audit method examines the “stage coverage
>   and field consumption” dimension. This case is a difference in compilation-unit membership and
>   therefore slipped through that dimension).
>
> - **Embedded std registration-surface coverage (construction finding in 4.2.5, fixed)**: the
>   Registry arm repeatedly harvested registrations for embedded std units through
>   `extract_module_info`, replacing the native half registered by `with_std()` (`result.is_err`,
>   etc. disappeared → std.test falsely reported E1043). This exposed the audit's third overlooked
>   dimension: the merge semantics of **two registration sources for the same module key**. The
>   single-file path forms its registry only once through `with_std()`, whereas the multi-file/Check
>   Registry arm obtains coverage only by inserting each unit separately.

**Two points require precise wording; otherwise the conclusion would be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
  (`run_file` → `Pipeline::run`), it does report them. The reason lies in `compile_project`: it
  packs the complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function
  **has no `result.warnings` read site anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 also has a different source**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surfaces), whereas `compile_project` uses `is_bin_role`
  (`250-252`, which checks only whether a manifest exists). The two functions give different answers
  to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of the document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for obligations.**
`src/frontend/core/sema/layers/predicate.rs:232-239` is the **only** location in the entire
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

Its meaning is “the predicate arguments of this refinement constraint are compile-time literals, so
the proof function must actually be executed to decide the constraint.”

**Step 2 — The consumption point is inside the checker, but the checker only produces and never
consumes.** `src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                   | Location                                                                                                                       | Behavior                |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement validation (`revalidate_refined`, formerly `checker.rs:5164`)       | `if calls.is_empty()` → derives a hard error (formerly `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (formerly `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, formerly `checker.rs:5306`) | Derives a hard error (formerly `5308-5316`); `ctx.proof_calls.extend(calls)` (formerly `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, formerly `checker.rs:5420`)       | Derives a hard error (formerly `5442-5446`); `ctx.proof_calls.extend(calls)` (formerly `5448`)                                 | **Emits no diagnostic** |

The original comment in `revalidate_refined` within `checker/refinement.rs` (formerly
`checker.rs:5165-5169`) stated:

> `RFC-027 §4/§9: Unproven → compilation error, no degradation, no silent pass.` …
> `This branch therefore becomes a genuine defense line rather than forward-looking fallback: silent pass must not be allowed to return.`

In other words, the author knew that “emitting no diagnostic” was a defect, explicitly classified it
as a **known intermediate state with nowhere to go**, and assumed by design that “someone would
eventually read `proof_calls`.”

**Step 3 — The field is indeed written into the result.** `check_module_impl` declares a local
`proof_calls` and collects into it (formerly `checker.rs:1234-1235`), then writes it into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registered proof-function obligations` (formerly
`:1439`). The field is defined in `types.rs:29`.

**Step 4 — The entire repository has exactly one read site.** In production code,
`TypeCheckResult.proof_calls` (`types.rs:29`) is read in exactly one place:
`src/frontend/pipeline.rs:187`, which passes it as the argument at `189`:

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
`checker/refinement.rs` (formerly `:1234/4564/5494`); the identically named field in
`verdict.rs:61`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — All four orchestrator entry points bypass that layer.** The four entry points in
`src/frontend/module/orchestrator.rs`—`compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)—**do not call
`pipeline.rs`**. They call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discard path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is
responsible for compiling embedded std. This means **the embedded std module's own proof obligations
are likewise never executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles and runs successfully, with no diagnostic**, on
the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point inside the layer.**
When `check_module_impl` handles the ownership-layer result (formerly `checker.rs:1303-1314`), it
contains:

```rust
// 原 checker.rs:1313（现 check_module_impl 内）
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the promise at the `5164` branch that “silent pass must not
be allowed to return,” and it is **independent of the orchestrator problem**—even if the entry-point
layer were completely fixed, this point would still remain silent. Its handling should occur in the
same batch as the obligation ledger because it is the same class of problem.

### Why tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero matches for the
three keywords `Sorted`, `proof`, and `refin`.** The multi-file path has no coverage of proof
obligations.

RFC-027's three unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that the `result.proof_calls`
contains a `SumUpTo` call. However, they **all pass through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping immediately before `pipeline.rs`**.

The test file's own doc comment already states this. `rfc027_refined_transparency.rs:13-15` says:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so cases that violate refinement are caught at the .yx layer`

**This is exactly the shape of the problem: tests verify that “the obligation was populated,” while
the bug is that “the consumer did not read it.”** A test that exercises only the producer, not the
consumer, is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria uses only the single-file path

The [Equivalence Criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` fixtures in `tests/yaoxiang/` as its **third-level criterion** and primary
acceptance mechanism for C2. In practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero matches across the entire directory
  glob).
- Consequently, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone`
  branch for every fixture (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 fixtures use the single-file path, all cover proof_execution, and none
  cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this fact. The file declares `// expect: compile-error E4018` on line 15, and its header
comment says on line 7 that its status is “❌ should be rejected at compile time”—**it can pass
precisely because it uses the one path that executes proof functions**.

**Conclusion: the third-level criterion needs an additional multi-file fixture layer; otherwise it
cannot serve as the acceptance mechanism for this document.** See Implementation Point S1.

### Obligation fields: 16 fields for which “production is a contract,” with no enforcement mechanism

Among the fields of `TypeCheckResult` in `src/frontend/core/sema/types.rs:16-107`, **16 fields**
have doc comments explicitly stating “produced by stage X → consumed by stage Y”; in other words,
they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first doc-comment line—the comment block for one field
> may span multiple lines, and only the declaration line provides a unique anchor. The **struct
> range** in the sentence above (`src/frontend/core/sema/types.rs:16-107`) moves with the struct's
> line number and is mechanically reconciled by gate **F5** (an error immediately turns the gate
> red); definition lines are reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**:
> `ir_gen.rs:360` reads it (`type_result.release_plan.clone()`), and `:3103` uses it
> (`ReleasePlan::consume`); settlement-completed marker `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `66` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `69` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completed marker
> `:1097`) | | `try_expr_impls` | `74` | typecheck → IR generation (span-keyed: implementation
> **type identity** + success payload + residual payload, all three settled in one `query_try_impl`
> call; IR uses this to name four methods and assign their result types) | | `match_scrutinee_types`
> | `78` | typecheck → IR generation (span-keyed) | | `module_namespaces` | `81` | typecheck → IR
> generation | | `record_models` | `88` | typecheck (one declaration-time pass) → IR generation
> (quantifier propagation; as of B.2, the `TypeDeclField.quantity` of witness fields and the
> population of constructor arguments/default values both read this table) | | `type_identity_index`
> | `64` | typecheck → IR generation (**`Nominal`-keyed**—type identity: `Nominal → write name`; the
> lowerer's constructor-call identity, field table, and reverse “write name → identity” lookup all
> derive from this index; D64 §2.5, “identity must not be inferred backward from the name string”) |
> | `expr_result_types` | `106` | typecheck → IR generation (**`PlanId`-keyed**—call-site identity:
> produced in the Call arm of `inference/expressions/expr.rs`, consumed by
> `lower/query.rs::expr_value_type`; export points converge, and unconverged entries are not
> exported) |

**Reclassification record (3.4.9, user ruling A on 2026-10-13)**: the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design,
it is consumed within the phase and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor in
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed during this round).
- **Disposition**: delete the doc comment's declaration that it is “a mirror for consumption by the
  middle layer/LSP”; retain the field and its in-phase consumption. The field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP genuinely needs the
  table in the future, follow the new-registration path in 4.6.x; **do not create a consumer merely
  for this round**.

**These fields cross layers keyed by `PlanId` (statement/call-site structural identity), `Span`, or
name tables.** The Equivalence Criteria document already points out that a mismatch in a span-keyed
contract can “silently fail, with no error whatsoever” (`ReleasePlan` is a typical example: if the
two sides calculate spans differently in any way → the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module lies on every entry point's mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **a
  different entry-point implementation**. The orchestrator's four entry points do not pass through
  the `pipeline.rs` layer at all.

**Verified fact**: the repository currently has **no mechanism** guaranteeing that these 16 fields
are consumed. The only “protection” is that `ReleasePlan` happened to ride along with IR generation.

### Non-obligation field registry: channel / in-phase / in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (9 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation fields”
table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Unit-identity metadata (zero hits in `src/middle`)                                                                                                                                                                                                                                                                                                     |
| `diagnostics`             | Error-diagnostic channel (existing channel, not “production is a contract”)                                                                                                                                                                                                                                                                            |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but its doc comment does not declare a cross-stage contract ⇒ not incorporated                                                                                                                                                                                                                            |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings`—no declared contract                                                                                                                                                                                                                                                     |
| `semantic_db`             | Consumer is LSP (from 4.6.1 onward, emitted through the `DriverOutcome.semantic_db` channel, src/lsp/server.rs:482), not a compilation-stage obligation                                                                                                                                                                                                |
| `trait_table`             | Has cross-stage reads ir_gen.rs:4336/:6867, but its doc comment does not declare a contract                                                                                                                                                                                                                                                            |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                                                                                                                                                                                                            |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; see the reclassification record in the “§Obligation fields” section)                                                                                                                                                                                                                                                |
| `method_def_ir_names`     | Cross-stage mapping (typecheck produces mixed-in names by method-declaration `PlanId` → IR generation reads only the name, WBS 7c.3); a miss produces a loud E3005 failure with no silent surface; consumption granularity is per declaration rather than per non-empty unit, so the ledger's “unit output must be delivered” criterion does not apply |

**Why this is not “wiring up consumers”**: creating downstream consumers for these fields would
introduce artificial obligations and disguise fields that do not require cross-stage consumption by
contract as contract defects. The correct closure mechanism is **reverse containment**—the set of
`TypeCheckResult` fields = obligation inventory ∪ non-obligation registry. This is mechanically
enforced by criterion 1 of `scripts/ci/check-obligations.py`: adding a field without registration,
deleting a field without synchronization, a non-obligation registration without a reason, or overlap
between the two tables all immediately turn the gate red.

### Four additional contract defects in the proof layer

All of the following come from proof-layer analysis and are independent of the entry-point
divergence, but all belong to the class “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation, with no enforcement mechanism; the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (anchored by function name/driver site; line numbers are taken from
the current tree):

| Actual order | Invocation site                                                                                                                                                                                                                  | Corresponding layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` carries the E2030 semantics; `judge_refined_obligation` is the sole decision unit shared by call-site and return-position obligations) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                  | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                     | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                             | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                                                                                           | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comments and do not correspond to the call order. The README has no “number
→ invocation timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuit**—`check_module_impl` adds
each termination error and continues downward; “a lower layer's failure prevents an upper layer from
running” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: hard-failure panic was removed by P3's 3.3.1 (the SOLVER slot was
> made optional, and absence is conservatively downgraded to `SMTResult::Unknown`); “silently
> skipped (not injected)” was given a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is performed under **RFC-039 D58**, at the actual fix location
> `proof/smt/backend.rs` (adding a process-wide shared singleton + `with_shared_solver` closure
> interface). The wording in the 02 change list that `default_solver() → &'static` takes precedence
> under D58 (a bare `&'static` reference is infeasible because `dyn Solver` is not `Sync`).

| Consumer                                                                                                 | Acquisition strategy                                     | When the solver is unavailable                               |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                     | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected during construction via `check_module_impl`, formerly `checker.rs:1265-1268`) | Injected at construction with `with_solver_owned`        | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                              | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the kernel's capabilities’.” By contrast, the contract documentation at `backend.rs:60` says
“`None`: the backend is unavailable… **the caller should degrade conservatively**.” **Two
philosophies coexist in the same codebase, and the hard-failure path panics rather than returning an
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

**This is a factory function, not a singleton**—each call invokes `Z3Backend::new()`, creating a new
Z3 context. The `Z3Backend` cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the doc comment at
`z3_backend.rs:17` states that “SMT query results are cached in `cache`.” No sharing across calls ⇒
**the cache never hits in this invocation mode**.

(By contrast, the `LazyLock` at `predicate.rs:34-36` is genuinely a singleton. It is the same
backend under two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match actual behavior.** The two
`body_checker` fallbacks in `check_module_impl` (formerly `checker.rs:1283-1287` and `1289-1293`)
are:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker 为 None → 空类型账本
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker 为 None → 空调用所有权表
```

When `body_checker` is `None`, ownership checking receives an **empty type ledger and empty call
table** and proceeds normally—ownership analysis degrades to “nothing conflicts,” **with no warning
whatsoever**. Recommended handling: record a warning-level diagnostic, or at minimum leave a trace
in the obligation ledger.

The `check_module_impl` comment says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (formerly
`checker.rs:1320`; termination formerly `:1256`, ownership formerly `:1296`)—**the “before” in the
comment is actually “two layers later.”**

### Current wasm state: carried by a shim crate; the branches in 29 files are live

The wasm target **has been built and is built in CI**. The `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                   | Evidence                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate provides only `rlib`                        | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                    |
| **The shim crate provides `cdylib` + `wasm-bindgen`**  | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                    |
| The shim depends on the main crate (rlib) as a library | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                           |
| The main crate has a wasm-target dependency section    | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                       |
| The shim directory is excluded from the main workspace | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                              |
| **CI builds wasm in four places**                      | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten   | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL; missing files degrade to a warning)                                                                                                                                                          |
| **29 files** contain the literal `wasm32`              | **22** have actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (see the “F2 assessment” subsection of `06-cleanup-inventory.md` for the list and per-item assessment)                                                                                |
| `orchestrator.rs` has 16 occurrences                   | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                  |
| `src/lib.rs` has 11 occurrences                        | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` as a whole
(all three require `std::fs`), while the shim takes a different path.

**This creates a fact directly relevant to this document**: the playground entry point
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and hands
it to `Driver::run` (JS-side entry point `run_code`, `:122`)—the **single-file path**. Therefore:

| Path consuming `proof_calls`                              | Executes proof functions? |
| --------------------------------------------------------- | ------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                   |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                   |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                   |
| Multi-file `run` (`compile_project`)                      | **No**                    |
| `check` (`check_project`)                                 | **No**                    |
| LSP within a project (`check_source_in_project`)          | **No**                    |

In other words: **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** When the unified Driver is implemented,
this path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`). The implementation record appears in the
**4.2.8** row of `09-execution-wbs.md` (`wasm` switches to `ProgramKind::WasmPlayground`, `:274`).

(The cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)
are unreachable in the playground scenario—is assigned to the “F2 assessment” subsection of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time; six `not(wasm32)` arms are load-bearing in the native three stage tables; there are no
low-risk items that can be deleted directly. The RFC-039 open question “build a wasm target, or
delete these branches?” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, not registered at runtime)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum; runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can enforce at compile time that “every
new stage must be handled by the orchestration layer”—precisely the mechanism already proven
effective by the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check       Project
    Discovery,          // File discovery                 Project
    Parsing,            // Lexing + parsing               PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type checking (including embedded proof layers) PerModule
    DeadCodeAnalysis,   // Dead-code family analysis       Project
    ProofExecution,     // Compile-time proof-function execution PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merging Project
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
type-level fact. Stages with `Project` scope are structurally guaranteed by `dispatch` to run only
once, eliminating questions such as “should `allocate_global_slots` be called for every file?” that
currently require human reasoning in `orchestrator.rs`.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes IR
>    artifacts (`Monomorphizer::monomorphize(&ir, …)` in pipeline.rs), and the original order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to dead-code analysis **before** proof according to
>    `Stage::ALL` topology: in the current implementation, `check_project` runs proof before
>    dead-code analysis, but there is no data dependency between them and the diagnostic sets are
>    identical (C2 set semantics); only the within-file diagnostic order is normalized. The
>    single-file path (dead-code analysis embedded in typecheck, before proof) already conforms to
>    the ALL order, so byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved **before** `Registry`/`RoleClassification` (C3, user ruling on 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST artifacts—the
>    original order would force the Registry arm to “hide parsing,” making the stage table lie about
>    the data flow; multi-file parsing consequently fell from two passes to one. The current state
>    is faithfully registered as an aside: parse errors on the multi-file path were once a hard
>    abort (the `?` propagation in `build_registry_from`, in tension with CollectAll semantics,
>    registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 after its last caller
>    was removed, and the historical account is retained here)—**4.10.1 has been fixed** (user
>    ruling on 2026-10-09: Rust-style collection semantics + option B): parse failures on the Check
>    path are downgraded to per-file diagnostic collection, with the affected file leaving the
>    compilation unit (not entering the registry, and importers report E5001); the MultiFile path
>    retains the hard abort (under FailFast, a bad file cannot produce IR, which is correct
>    semantics and a long-term pinned rule).
> 4. The Check form (4.2.2, user ruling on 2026-10-09) adds two registrations: a. Data-dependency
>    edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the initial draft omitted these
>    (in the orchestrator era, these artifacts were produced by `discover_with_used` and discarded
>    by the `discover` wrapper, so they were lost because they never entered the table); b. The
>    within-file diagnostic-order normalization expands to the complete form of note 2: E3020 entry
>    validation moves into the RoleClassification arm (Check has no Linking stage, so the arm
>    holding `surfaces` and the AST performs validation), and the within-file diagnostic order =
>    stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set
>    is unchanged (C2 set semantics), and only the order of stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective**: turn “a field was produced but nobody consumed it” from an undetectable
condition into a fact assertable at compile time or test time. RFC-039 has identified this as “the
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
    pub try_expr_impls:          TryExprImpls,               // types.rs:74
    // …remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces one W-level diagnostic (by default) or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Case                                            | Criterion                                                 | Disposition                                                                 |
| ----------------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The field in `Obligations` is `take()`n / marked consumed | Pass                                                                        |
| The obligation is non-empty but has no consumer | The field is non-empty and not consumed                   | **Produce a diagnostic** (`W` by default; promoted to `E` in `strict` mode) |
| The obligation is empty                         | The field is empty                                        | Pass (no consumption required)                                              |

**Why start with W rather than E**: fixing obligation discard changes the **diagnostic set**. C2
requires that “the diagnostic sets across entry points are identical.” Promoting directly to E would
cause `yaoxiang check` to suddenly emit a large number of previously silent `Unproven` diagnostics.
Splitting the work into two steps—first W for observation, then promotion to E—keeps every
intermediate step testable against the criteria. See Implementation Point S4.

**The sole call site of `assert_drained()`**: `Driver::run`, at the end of the stage table and
before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing at
least twice but having no read site in a third file → failure). `assert_drained()` is the
**runtime** gate, while the script is the **static** gate; the two are complementary, and both are
required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: this note registers the
> **implemented form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, mapping each item to code file:line. The landing commit is `1941cc47a` (verification tree
> `99079bc52`). It also serves as the **third** of the three carriers of the red-list “no silence”
> principle (the other two are the audit surface and the static gate).
>
> **1. Implemented artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: a 13-entry obligation inventory `INVENTORY` (:201; field
>   name/consumer stage/red list), **non-obligation field registry `NON_OBLIGATION_FIELDS` (:175, 9
>   entries with reasons)**, delivery-marker ledger `DeliveryLedger`, pure settlement function
>   `settle_obligations` (:577), external emission `emit_obligation_diagnostics` (:654),
>   non-consumer gating `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped`
>   (:549), IrGeneration delivery set `IR_GENERATION_DELIVERED` (:285), and W1090 code constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path under `Aggregation::FailFast` also settles (the
>   ledger must balance).
> - **Seven delivery points** (one per arm; removing any one makes runtime W1090 sound):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only external read path for that
>   obligation on `TypeCheckResult`, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —all four use the same
>   delivery interface: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct
>   request-table read) / `:1221` (MultiFile·Check: aggregated by the Linking arm;
>   `driver/helpers.rs:295` reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by tests.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is documented above the entry at :26-31—the settlement
>   granularity is “unit × obligation,” with no source location that the user could be pointed to,
>   so no dummy span is inserted); six-language text is present in `locales/*.json`. **Emission
>   level**: from 4.3.2 onward it is **error** (same code promoted to E; see part 3)—determined by
>   the explicit `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444), while the
>   code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Case in the three-tier table above              | Implemented criterion                                                                                | Code                   |
> | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (the marker ↔ actual read is reconciled by the static gate; see part 5) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**; see part 3)                 | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                     | obligations.rs:587-591 |
>
> Two **additional tiers** (user ruling on 2026-10-13, absent from the sketch):
>
> 1. **S1**: The obligation's consumer stage is not present in this `ProgramKind`'s stage table →
>    **not judged outstanding** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: The consumer stage is skipped with `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not judged outstanding (obligations.rs:598-602). `NoObligations` does
>    **not** grant an exemption—an obligation that is non-empty while the stage declares no
>    obligations precisely indicates an unbalanced ledger and must be reported.
>
> **3. External-emission semantics (ruling “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: the mechanism emits only **W-level** diagnostics (W1090); there is
> **no strict switch**. **4.3.2 (final state, user ruling A on 2026-10-13)**: **the same code is
> promoted to E**—the W1090 code number remains unchanged, but the emission level is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (one code representing the same fact would create parallel representations).
> **Promotion changes only the emission level, not the emission conditions.** **This is a D-class
> breaking item under D59 (changes externally observable behavior), and is faithfully registered**:
> healthy programs emit nothing (all obligations have delivery markers), so existing corpora and
> user programs are unaffected (342/342 “difference: empty”); however, whenever a run really
> encounters an undelivered obligation, the diagnostic **counts toward `error_count`, renders as
> `error[W1090]`, and produces a nonzero exit code**—unlike the previous behavior where “W-level
> does not block compilation.” The separation of code family from emission level is registered in
> RFC-013's “Emission channel” section (the code prefix determines numbering ownership only;
> severity is determined explicitly at the construction site). The mechanism's behavior (measured):
> an explicit `.severity()` takes precedence; otherwise, “a code beginning with `W` → Warning”
> (`codes/builder.rs:186-203`). The registry's `ErrorCategory` does **not** participate in severity
> determination. The exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`); **W1090 follows
> `error_count` because of its explicit error severity (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's ruling C on
> 2026-10-13; the current red list has been reduced to zero; see part 4), for a reason that can be
> recomputed:
>
> - Ruling A (emit red-list fields as W) has a measured cost: `interface_impl_registry` is populated
>   from the typechecker's preset table and is **always non-empty**. What would be emitted per
>   compilation unit is therefore a **constant signal** with no discriminative value, while
>   violating three existing external contracts—“clean programs emit no diagnostics / LSP emits no
>   diagnostics / `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing
>   test cases turning red**, including **8 on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 were in `src/driver/tests/`). The corpus
>   baseline changed in 24/342 fixtures, and the only viable modification would be “filter W1090 out
>   of zero-diagnostic assertions,” which relaxes an existing criterion.
> - Ruling B (downgrade to Info) still breaks 9/12 list-based assertions, and Info is not “emission
>   according to the ruled level.”
> - Ruling C's measured result: **zero changes to existing tests** and **zero changes to the corpus
>   baseline** (342/342 “difference: empty, behaviorally equivalent”). Runtime W1090 becomes a
>   **discriminative regression detector**—the healthy tree remains completely silent, and removing
>   any delivery marker makes it sound (each of the three delivery points was tested with the marker
>   removed; CLI `check` emitted two `warning [W1090]` diagnostics when the Check arm's marker was
>   removed).
>
> The red list's “no silence” property is carried by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (emitted on every run, including WBS IDs
> and evidence pointers); ② `check-obligations.py`, which prints the red list on every run
> (currently empty) and performs stale detection; ③ this note.
>
> **4. Red-list status (cleanup closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three cleanup tasks landed on 2026-10-13:
>
> Note: the line anchors below record the file state **as of 2026-10-22**; P5 (2026-10-26) split the
> checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting all
> `checker.rs` line numbers, so the anchors below no longer correspond to the current file and are
> retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place form detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST form
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`); the call site at
>   `:7496` replaces the entire Call with `CreateVariant`. The function's doc states that “span keys
>   are unreliable across systems (the two sides interpret the span of a Call node differently), so
>   detection is performed in place”—**wiring up a consumer would reintroduce the already-fixed
>   span-mismatch defect**. Deleted surfaces: the `types.rs` field; collection at
>   `checker.rs:1489-1493/:1565`; the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`; and the `VariantCtorCall` structure in `environment.rs`. Type
>   inference and all diagnostics of `infer_variant_ctor_call` remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`); read in-phase
>   (`expressions.rs:4490-4550` decision site, the sole matching write at `:4550` into
>   `overload_resolutions`); what was deleted was the **zero-reader mirror** injected into
>   `TypeCheckResult` at `checker.rs:1558`. The naming fact still has a single expression: the
>   candidate-name table (in-phase) + the span-keyed decision (cross-stage, `ir_gen.rs:380`
>   constructs it, `:7632/:7705` consumes it); the definition-side mixed-in name is computed
>   independently by `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in this document's §Obligation fields section). What was deleted was the
>   **declaration** in the doc comment that it was “a mirror for consumption by the middle
>   layer/LSP” (types.rs:47-48); the field and its in-phase consumption remain
>   (`operator_interfaces.rs:623` operator lookup, seven constraint-solving lookups in
>   `inference/expressions.rs`, and the read-only accessor in `checker.rs:214-217`). Direct proof of
>   zero cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not
>   wire up a consumer**: the table's contents are an environment mirror populated by typecheck and
>   are always non-empty. If LSP genuinely needs it, follow the new-registration path in 4.6.x;
>   creating a consumer for a field that does not require cross-stage consumption by design would
>   introduce a false obligation into the ledger.
>
> After cleanup, **the field census is closed by “obligation inventory ∪ non-obligation field
> registry (`NON_OBLIGATION_FIELDS`, with reasons)”**—gate criterion 1's reverse containment
> protects this end to end (adding a field without registration / deleting a field without
> synchronization immediately turns the gate red). The red-list **mechanism** is retained: gate
> criterion 4's stale detection and the `--selftest` `stale-red-list` fixture continue to cover its
> detection surface; future registrations require an explicit ruling.
>
> Cleanup also updates the counts: obligation inventory 16 → **13** (`INVENTORY` / `Obligation::ALL`
> / this document's §Obligation fields table); the gate's `--selftest` now covers seven fixture
> classes (adding “unregistered field,” “stale anchor,” and “empty red list”), under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2), three criteria beyond the sketch's sentence**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document's “§Obligation
>    fields” table (including the table header's count declaration); any drift turns the gate red.
>    In addition, the `TypeCheckResult` field set must be **exactly equal** to “obligation inventory
>    ∪ non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field without
>    registration, deleting a field without synchronization, a non-obligation registration without a
>    reason, or overlap between the two tables all immediately turn the gate red;
> 2. **Read evidence**: a non-red-list obligation must have a genuine read (`.field_name` member
>    access) in the **declared consumer file**; comments, tests, and strings do not count. This
>    makes “delivery marker ↔ actual downstream read” a hard criterion (the primary risk of
>    mechanism A is drift between the two);
> 3. **Delivery markers**: a non-red-list obligation must be marked at its delivery point; red-list
>    obligations, conversely, must not be marked;
> 4. **Red-list stale detection**: a red-list field must have zero genuine reads (growing a consumer
>    turns the gate red), and each entry must carry a WBS ID and evidence pointer; there is also
>    `--selftest`, with built-in negative/positive fixtures: negatives must turn red and positives
>    must turn green, proving that the detector is not idle.
>
> **6. Deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains owned by `TypeCheckResult`, while the ledger records only the
>    “inventory + delivery markers + settlement conclusion.” The reason: copying them would create
>    two sources of truth (the fields are produced by different layers—the typechecker's checker and
>    the ownership layer—and would inevitably drift); the cross-layer data plane should also not be
>    duplicated into the orchestration layer.
> 2. The sketch's `take()` / “mark consumed” **was not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The reason: an obligation and its
>    artifact are the same data, and `take()` would remove what downstream code needs; moreover, 11
>    IrGeneration obligations take the form of “handing the entire `&TypeCheckResult` to ir_gen” and
>    cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (from 4.3.2 onward, W1090 is E-level)—the single-file path
>    uses `PipelineError::TypeCheck` (the obligations.rs:654 emission arm), while
>    multi-file/embedded paths record an `OrchestratorError` failure (otherwise `result.errors`
>    would have no consumer). The reason: returning `Vec<Diagnostic>` would lose form-based
>    routing—check diagnostics must retain per-file attribution.
> 4. The first tier's criterion changed from “the obligation is read by its declared consumer” to “a
>    delivery marker exists”; “marker ↔ actual read” is delegated to the static gate (criterion 2 in
>    part 5).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**;
>    instead, **4.3.2 promotes the same code to E** (it is promoted even without a strict switch
>    because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no read in
>    a third file” to the four criteria in part 5.
> 7. The two additional tiers absent from the sketch (S1 and the skip exemption) are described in
>    part 2.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table actually landed as
>    **20 test cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]` skeleton →
>    turned green; red state retained in the record: removing the sole settlement point yields
>    `pairs: 0` and six failures). The S1/S4 rows were synchronized to the actual file.
>
> **7. 4.3.2 implementation record (severity + gate hardening)**
>
> - **Per-field review inputs**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   drained in the run, available per run) and five counters (the conservation identity in part 2);
>   together with the audit surface `findings`, they provide the fact of “whether this field has
>   ever been consumed by any entry point.”
> - **Severity: same-code promotion to E has landed** (user ruling A on 2026-10-13; see the
>   final-state registration in part 3 and the D59 D-class breaking-item declaration).
>   Implementation: `to_diagnostic` explicitly calls `.severity(Severity::Error)`; the emission arm
>   routes by severity into the error channel. Mutation evidence: removing any delivery marker →
>   that form fails compilation, and `check` exits nonzero (`error [W1090]`); after restoration, the
>   healthy state emits nothing (corpus 342/342 zero changes). Locale text states in all six
>   languages that it is “a W-family code emitted at error level.”
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence**, validating that “the file is locatable
>     (uniquely), the line number is in range, and the symbol immediately after the anchor actually
>     appears in the anchored file”; an anchor pointing to a nonexistent symbol turns the gate red.
>     Built-in negative fixtures under `tests/baselines/obligations-gate/stale-anchor/` (one for a
>     nonexistent symbol and one for an out-of-range line). Note: the red list is currently empty,
>     so the criterion's effective landing point is the **non-obligation registry** and future
>     evidence pointers (the same code path).
>   - **F2/R3 reverse containment**: the gate outputs “the number of `TypeCheckResult` fields − the
>     number of obligations − the number of non-obligation registrations = the difference”; a
>     nonzero difference turns the gate red. Built-in negative fixture: `unregistered-field/` (a new
>     struct field with no registration).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain `type_result`, so the obligation
>   does not exist in the first place); this tier's semantics are pinned by a synthetic `State`
>   test, while real end-to-end coverage includes only `ConfigDisabled` (t3 T06).
>
> **8. Closure of 4.3's secondary tasks (red list reduced to zero + obligation diagnostics finalized
> at E level)**
>
> **8.1 Three-round implementation timeline** (each item mapped to code file:line; this round only
> registers conclusions and does not change code)
>
> | Round               | Conclusion                                                                                                                                                                                  | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
> | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger launch | `State::assert_drained()` sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                        | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                              |
> | 3.4.4 Cleanup       | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place form detection on the IR side                                                          | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                          |
> | 3.4.5 Cleanup       | `method_overload_ir_names` cross-stage mirror deleted (retain in-phase candidate-name table + `overload_resolutions`)                                                                       | Decision consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                |
> | 3.4.9 Cleanup       | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirror for middle/LSP consumption” declaration, retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, `src/frontend/core/sema/inference/expressions.rs` seven sites (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Severity      | **Same-code promotion to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; external routing by severity into the error channel (first failure takes precedence)      | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); separation of code family and severity registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                    |
>
> **8.2 Closure conclusions**
>
> - **Red list reduced to zero**: the number of entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) whose `red_list` is `Some` = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism is
>   **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the gate's criterion-4 stale
>   detection, and the `stale-red-list` fixture; the gate reports “red list: 0 entries—cleaned up to
>   zero.”
> - **Final severity for obligation diagnostics: E level**: the **code number and W-family
>   membership of W1090 remain unchanged**, but the emission level is `error` (final state under D5,
>   user ruling A on 2026-10-13). This **is a D-class breaking item under D59** (changes externally
>   observable behavior), registered both in part 3 of this note and in RFC-013's “Emission channel”
>   section. **Zero emission in the healthy state ⇒ no impact on existing corpora or user programs**
>   (corpus 342/342 “difference: empty,” zero IR snapshot drift; independently verified by t11 and
>   re-reviewed by t12).
> - **Field census closed**: the `TypeCheckResult` field set = obligation inventory ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a reason grounded in measured
>   consumption); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises left behind**: all hardening items registered in this note **have
>   landed** (F1/F2/F4/F5; see parts 5, 7, and 8). The only two items not closed are the **known
>   boundary** in 8.4 and t7 F5, which was converted into an issue; neither is an unimplemented
>   criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/review round**
>
> | Source        | Finding                                                                                                                                                                            | Status                                                                                                                                                                                                                                                                                 |
> | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence consists of plain-text pointers and can silently become stale when the replacement consumer is removed                                                           | **Landed**: gate F1 anchor validation (file uniquely locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`                                                                       |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, so adding a field without registration silently passes                                                                                 | **Landed**: explicit F2 difference output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                                      |
> | t4 R4         | W-only versus promotion to E was not registered                                                                                                                                    | **Landed**: parts 3 and 7 of this note + RFC-013 “Emission channel” + W1090 help and error_output examples in the six `locales/*.json` languages                                                                                                                                       |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (legal overloads are falsely reported as `W1005`)                                                                    | **Converted to issue `#450`**, assigned to WBS **4.9 correct reference-pool implementation** (registration: the 4.9 row of `docs/src/dev/architecture/09-execution-wbs.md`); outside this secondary task                                                                               |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                                  | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                           |
> | t11 F8        | Validator tool self-report: mutation-driven runs once wrote to the main repository (`git restore` immediately cleared it, and the affected round's output was discarded and rerun) | **Informational item** (not a deliverable defect)                                                                                                                                                                                                                                      |
> | t11 F9        | The main repository's intermediate state (unstaged files made this document's table header inconsistent with `INVENTORY`) turned the gate red                                      | **Informational / resolved**: the frozen commit is self-consistent and passes; “committing the intermediate state directly turns the gate red” is precisely F2/F4's intended behavior                                                                                                  |
> | t12 F12-1     | This document's struct range says `16-73`, but the actual range is `16-76` (t9 added three comment lines without mechanical reconciliation)                                        | **Landed + mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; an error immediately turns the gate red); related figures were grep-aligned (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                  |
> | t12 F12-2     | Settlement in multi-file form once superseded the first failure (a type error in `zzz.yx` was masked by W1090)                                                                     | **Fixed** (the captain amended the commit message to explain this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundary (a registered item, not an unimplemented promise)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence.
>   **`file:line` promises in module-header prose comments are outside the scanning surface**
>   (currently no registered entry is affected). If such an anchor promise is written in a module
>   header in the future, it must **be covered by human review, or F1's scanning surface must be
>   expanded**—one of the two is required; it must not default to being unowned.
> - **The gate's static boundary**: collection-level validation of delivery markers is handled
>   statically (criterion 3), while removal of an individual arm marker is covered by **runtime
>   settlement**. Static + runtime, two layers, is the intended design, not a gap (t11's
>   M11A/M11B/M11C mutation validation covered both directions).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId conversion
> landed)**: two of the 13 obligations above—**two span-keyed contracts** (`release_plan` /
> `overload_resolutions`)—were converted to **structural identity keys** (`PlanId`) under RFC-039
> **D20**. This note registers the implemented form, criteria, classification basis, and known
> boundaries; the definition lines are still reconciled by gate **F4** (the line numbers in the two
> corresponding rows are current code facts).
>
> **1. Implemented form (carrier choice A: `PlanId`)**
>
> The key is the **structural identity** carried by an AST node, rather than the positional
> coincidence of “each side calculating a span separately”: `PlanId` (`src/util/plan_id.rs`,
> monotonically increasing `u32`, unique within a compilation unit, with `UNPLANNED = u32::MAX` for
> synthetic nodes outside the parser) is issued by the **sole allocator**,
> `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement identity →
> ownership of release plans and their consumption) and `Expr::Call.plan_id` (call-site identity →
> production and consumption of overload resolutions). `Expr::call` is the repository's only
> call-expression constructor, and all synthetic calls outside the parser use `UNPLANNED` (**not
> consumable**; see part 2).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`); its sole
>   consumption entry point is `ReleasePlan::consume(stmt)`—**reading and accounting occur in the
>   same call**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is provided by
>   `walk_stmt`'s set/restore operations + the CFG-node-to-statement-identity table; Drop ownership
>   at scope exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one object**),
>   produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with dispatch
>   destructuring and forwarding from `Expr::Call { plan_id }`), consumed at `ir_gen.rs:7697` (with
>   bindings) / `:7772` (without bindings) through `OverloadResolutions::get` (a hit is recorded).
>
> **2. Mandatory-consumption settlement (reusing the 4.3 ledger rather than creating a second
> one)**: the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each table. When `generate_module_ir`
> returns successfully, both ledgers are marked as having “completed a full run” at the **same
> point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload resolutions). The
> three-state mapping is unique (`consumption_state` in `src/driver/obligations.rs`), and the sole
> settlement point connects one line for each of the two obligations—a nonempty difference produces
> an **E-level W1090** (same-code promotion to E; see part 3). The exemption criterion is attached
> to the **ledger's own completion marker** (not to whether `unit.ir` exists), so Check's removal of
> the IR artifact does not grant an exemption.
>
> **3. D59 classification basis (class C, red before green)**: before the change, the form
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` instead of `20`—the selected overload
> was discarded, and execution fell back to the last binding in definition order); afterward, the
> same program fails at compile time with `error [W1090] ... produced key was not consumed`. The fix
> chosen was **loud failure**, rather than “put the selected overload's name into the call with
> named arguments”: the latter would lose named arguments and replace one form of silence with
> another (final-state semantics: [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop`
> instructions are no-ops in the interpreter (the wildcard arm in
> `src/backends/interpreter/executor/ops/control.rs` only calls `advance()`), so migrating the
> release-plan keys does not change runtime behavior—the empty 342-fixture differential is empirical
> evidence of this point.
>
> **4. Criteria and evidence**: the D41-equivalent criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per fixture,
> lex→parse→typecheck→ir_gen; **the difference set must be empty + the evidence floor must hold**:
> the number of files that produced a resolution is `≥ 1`, and the number of genuinely consumed
> entries is `≥ 2`, with no allowlist). The nine pinned resolution forms are under
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional arguments /
> type-body binding / `?` operand / named only / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven validation: shifting a consumption key by
> one position ⇒ criterion turns red + the real program emits `error [W1090]` + the corpus gate
> reports a difference; restoring it turns green. Gate figures: `check-obligations.py` PASS, all 342
> fixtures have an empty differential, `cargo test --lib` reports 2712 passed / 0 failed (zero IR
> snapshot drift).
>
> **5. Known boundary (a registered item, not an unimplemented promise)**: the **unbound consumption
> site** at `ir_gen.rs:7766-7790` is currently **unreachable** across the existing corpus and test
> surface (method declarations uniformly reach the bound consumption site through
> `register_type_bindings`), so the keying correctness of this branch has **no red state that can
> falsify it**. It is retained because it is the symmetric counterpart of the bound path, and
> deleting it would turn “a method call with no `type_bindings`” into silence. This is registered by
> independent validation (t5 F-1) and review (t6 F2); **no new task is opened**.

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
    /// Must come from one of the six predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above** (one per `ProgramKind`);
callers cannot freely pass an arbitrary array. This is the key constraint preventing the `Program`
abstraction from degenerating into a “free parameter that can accept anything,” and it is asserted
by `test_program_stage_coverage`.

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
                // … 12 arms; not one may be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: there are four implementation
> deviations from the sketch above, semantically equivalent and recorded for reference:
>
> 1. §4's three-state `StageOutcome` is not returned by stage arms; it is expressed through `State`
>    failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are unchanged,
>    eliminating per-arm boilerplate returns).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (§5's
>    “avoid mutable global state in Driver” assessment covers the sketch field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-diff criterion of S2 “deliberately fix no bugs”
>    requires this; external emission follows with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is
>    its only new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their original visibility after migration in 4.2.
> 5. In the multi-file form, ProofExecution is an independent stage that runs **after all
>    typechecking** (A1, user ruling on 2026-10-09): the original `compile_project` interleaved
>    proof within the per-file typecheck loop (file N's proof preceded file N+1's typecheck). The
>    byte-for-byte output is identical for a single failure; in a multiple-failure scenario of “file
>    1 proof failure + file 2 typecheck failure,” the first reported error changes from a proof
>    error to a typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through separate channels by ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    the entry points (PipelineError / OrchestratorError) are not part of the type surface Driver
>    can unify.
> 7. The Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; clean files have an empty Vec—the current
>    `check_project` contract); Discovery's artifact expands to a “file set + used_by edge set +
>    shadowing events” triple stored in State (the MultiFile form has no downstream consumer, so it
>    is recorded but not emitted externally); on the check path, parsing each file falls from two
>    passes to one as a result of the C3 topology.
> 8. The Lsp form landed (4.2.3, per-file-source routing ruling, user decision on 2026-10-09): Parse
>    failures for files on disk follow Check's option B (collect and leave the compilation unit—both
>    sides converge on degradation); parse failures in an edited buffer retain a partial AST and
>    continue typechecking (the editor philosophy—semantic functionality should not disappear while
>    the user is typing). The Discovery arm retains buffer source over stale on-disk content for
>    Lsp; Typecheck checks only the target file under Lsp (other units exist only to provide
>    signatures to the registry); the principle of normalizing within-file diagnostic order extends
>    to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior—LSP
>    hard-aborting on an unrelated disk file's parse error, with the handler silently falling back
>    to the single-file path—was fixed by this step.
> 9. The Embedded form landed (4.2.4): `Program` gains a `shared_registry` field—the embedded std
>    module is a child compilation, and the registry is an **input**, not an artifact
>    (EMBEDDED_STAGES has no Registry stage), making #94's SymbolTable-sharing contract explicit as
>    a program declaration rather than “the caller must remember to pass the same registry.”
>    Error-path text changes from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler-internal error surface only, with no pinned test). At this point, all four
>    orchestrator entry points have migrated to the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR-stage ruling, 2026-10-09): The Check form
>     faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive errors only; missing a warning
>     is preferable to a false one); relative `use` paths resolve relative to the importer's
>     directory (aligned with rustc's single-file mod behavior). The CHECK stage table gains
>     GlobalSlotAlloc + IrGeneration: the old standalone path (the full pipeline) already ran IR
>     generation, and E3019/E1014/E1015 and similar diagnostics are produced only by ir_gen
>     (confirmed by runner gates). IrGeneration's Check form performs checking only and does not
>     consume IR; the Script/Bin branch is represented by the presence/absence of `module_key` (the
>     existing E3023 switch, ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces
>     only resource-limit/internal errors and has no corpus dependency). **[Withdrawn on 2026-10-10
>     based on 3.4.8 evidence]**—“zero corpus dependency” was actually the evidence blind spot of a
>     corpus with no pathological-recursion fixture; E3005 is the only compile-time defense for this
>     class of program. See note 14. Incidental fix for a latent defect: the Registry arm's repeated
>     harvesting of registrations for embedded std units could replace the native half already
>     merged by with_std() (false E1043)—embedded units now skip duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI use the same Driver. In
>     the SingleFile+CollectAll form: Parsing collects all parse errors and retains a partial AST to
>     continue typechecking (the editor philosophy from the 4.2.3 ruling; the stage is not marked
>     failed, because otherwise topological skipping prevents typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now covers proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and IR-level errors;
>     lexical failures now report real diagnostics—the old synthetic “E0001 lexical error” text
>     disappeared with the deleted sequence, with no information loss because the real diagnostic
>     carries an exact span.
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, mandatory C5
>     item): the two entry points had already converged—mod.rs uses the single boolean
>     `check_module_inner(ast, env, collect_all)`, while the checker layer uses
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`. Both pass-3 and drain modes run,
>     with the difference carried by whether `collected_errors` is empty; there are five collection
>     sites in statements.rs (function body/use/for/block/while). Verification confirmed and fixed a
>     duplicate-diagnostic defect: collection sites put the first error into both `collected_errors`
>     and the `Err` return channel, and pass-3's `add_error` for `Err` produced two diagnostics with
>     the same code, span, and message (up to three when nested); two independent mechanisms were
>     also fixed: duplicate validation of annotation signature parameters and whole annotations
>     (E1003/E1103), and duplicate emission at the same point in the ownership layer (E2014/E2018).
>     The fix landed at the module-result boundary: deduplication by `(code, span, message)`—a
>     diagnostic is a positional fact, and duplicates carry no information; 63 corpus entries lost
>     duplicate copies, with zero drift in the run list and exit code (the apparent “duplicate” in
>     borrow_conflict_err is an artifact of the baseline triplet format not containing column
>     numbers—the two E2018 diagnostics are legitimate diagnostics at different columns).
> 13. wasm playground landed (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper is no longer involved. Error text is aligned byte-for-byte with the old `CompileError`
>     Display prefix (`Parse error:/Type error:/Internal error:`), with no visible difference in the
>     playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the
>     declaration already existed in 4.1; this step gives it a real production caller).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing the mono exclusion in note 10**). Evidence disproved the
>     claim that “mono produces only resource-limit noise”: pathological generic recursion
>     (`f(x)=f([x])`) makes the compiler process overflow its stack on the multi-file path without
>     mono (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime error), and the
>     mono depth gate is the only compile-time defense. The standalone-check path before 4.2.5 used
>     the full pipeline and therefore already had mono; the exclusion ruling introduced a silent
>     coverage regression (“zero corpus dependency” proves only that the corpus has no pathological
>     fixture). The “type-erasure fallback” assumption was invalidated by the RFC-033 reflection
>     ruling—`^^List(Int)` requires a real instantiated identity, and monomorphization is the
>     foundation of reflection, not an optional optimization. Four design points: ① typecheck
>     produces an instantiation request for a qualified call (`lib.f(x)`, FieldAccess form), with
>     `generic_id` using the qualified name and the same namespace as merged IR (currently only bare
>     Var is recognized, expressions.rs:2455); ② `Stage::ALL` topology changes to Linking before
>     Monomorphization, and the MULTI_FILE stage table gains Monomorphization at the end—consume
>     merged_ir, aggregate all units' requests (qualify the deferred bucket's `containing_fn`), and
>     elevate mono from a single-module pass to a whole-program pass; the single-file stage table is
>     unchanged (no Linking, preserving subsequence properties and zero IR snapshot drift); ③ CHECK
>     gains Linking (pure merging—E3020 entry validation stays in RoleClassification to prevent
>     duplicate reports) and Monomorphization (checking only, without consuming, in the same manner
>     as IrGeneration's Check form); ④ resource protection covers cross-unit mutual recursion
>     through whole-program BFS. The `Monomorphizer` itself is unchanged (its contract for consuming
>     a ModuleIR + request set happens to match the merged-IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the design in note 14: `ExpressionInferrer` receives a
>     module-qualified-key table (the three `use` arms are registered—normal / alias / item alias,
>     with only SubModule-class exports eligible), and `callee_generic_name` obtains a qualified
>     name through the same unified qualified-name concatenation, with the same source as merged IR
>     (the concatenation implementation is in `frontend::module::symbol`); the qualified-call first
>     path's arity criterion is tightened (when the parameter table cannot be resolved, it falls
>     back to the second path and obtains arguments by signature, avoiding a false E3018 caused by
>     mismatching same-named functions across modules). Implementation empirically exposed and fixed
>     **three mono-side asymmetries** in the same round: (1) The deletion key (set of generic names)
>     and rewrite key (the `containing_fn` + name + span triplet) had mismatched granularity—a site
>     whose arguments could not be resolved degraded into a symbolic request in the deferred bucket,
>     but the bucket never drained when the containing function was non-generic, while the original
>     artifact had already been deleted by name → dangling call (runtime E6006 demonstrated by
>     `list_ops.yx:53`); the fix was a restoration gate,
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the sole source
>     of truth to determine coverage and retaining artifacts for uncovered sites. (2) `HashSet`
>     iteration order in the restoration section made function-table order nondeterministic (t2
>     review F1) → changed to `Vec` and sorted by name, making the artifact byte-for-byte
>     reproducible. (3) TailCall / MakeClosure forms outside the rewrite surface still looked up
>     mapping keys even though the comment claimed they were handled conservatively → introduced the
>     single-point decision `call_form_is_rewritten`, with all non-rewrite forms counted as
>     uncovered. Each issue has a pinning test (empirical discrimination of the red state before the
>     fix). The corpus differential baseline was regenerated in the same commit with exactly one
>     changed line (the `remove_at` frame gained `(int64)`). **Commit-strategy correction**: when
>     typecheck collection and the mono fix were split into two commits, the staged tree containing
>     only the former triggered `stdlib_docs`'s red result for a pre-existing HEAD defect
>     (`list.is_empty([])`, same E6006 family)—the two halves are prerequisites for each other's
>     green result, so they were combined into one atomic commit.
> 16. Registration of five rulings on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—user-module
>     exports carry `generic_fn_type_params_snapshot()` through `Export.type_params`, isomorphic to
>     the std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified-call monomorphization, while signature inference becomes a fallback (incorporated
>     into R2's scope). (2) **HKT declarations live, call sites are broken** (inference passes type
>     arguments E1002 / explicit type arguments E1010) is registered as WBS 3.5 (three third-level
>     tasks). (3) **The deferred bucket never drains in a non-generic container** is incorporated
>     into the acceptance subitems of 3.4.8 ②: the bucket must drain or fail loudly and must not
>     silently remain unspecialized. (4) **Calling a closure variable by name fails at runtime with
>     E6006** is registered as WBS 3.6; the semantics are (i) support indirect calls / (ii) reject
>     at compile time, and **(i) has been ruled**: indirect calls are supported: the `CallDyn`
>     mechanism already exists (`ir_gen.rs:7945`, closure-expression path); what is broken is a
>     `Var` callee being lowered to a name-based `Call`, while typecheck already accepts the
>     program. (5) **Trade-off criteria are fixed**—a proposal may be rejected only on correctness
>     and readability grounds; “large change surface / introduces a new mechanism / high cost” are
>     not grounds for rejection. This is recorded in part 2 of `coding-rules.md` and the HOWTO
>     self-check list; trade-offs requiring a user ruling must provide all four items specified in
>     `AGENTS.md` under “When requesting a user ruling (mandatory)” (situation + evidence / itemized
>     pros and cons / explicit recommendation + cost / explicit breaking-change identification).
> 17. 3.4.8 ② landed and F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the design in note 14: `Stage::ALL` changed to Linking before
>     Monomorphization, and MULTI_FILE gained mono at the end; the Linking arm aggregates
>     whole-program requests **before taking each unit's IR**, and qualifies `containing_fn`
>     (otherwise deferred-bucket keys and merged IR would not share a source, and the bucket would
>     never drain). **① closure (D2 ruling A)**: user-module exports carry a declaration-time
>     type-parameter snapshot through `Export::type_params`, isomorphic to the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically discovered and fixed during
>     implementation**: (a) **F1**—`qualify_names` rewrites only function bodies, and `ir.init` is
>     never visited, so under multi-file compilation, function definitions were qualified while
>     top-level module calls remained bare names → runtime E6006 (**reproduces without generics**,
>     unrelated to generics); the fix makes init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) On the mono side, the “whole-program unique suffix” heuristic and
>     silent `continue` on a miss were removed; a miss now produces a **loud E3018** (ambiguous
>     silent mismatch is worse than failing to resolve). **Independent validation caught and fixed a
>     regression**: t6 found that `use lib.{pick}` followed by a bare-name call was falsely rejected
>     with E3018 (the IR instruction name is the alias-qualified name, while the request name is
>     bare, and aggregation qualified only by unit key) → the fix makes the aggregation side
>     **directly call ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted into a
>     single public point, one producer and one consumer), with a matrix pinning both named-import
>     forms. **Retained record**: the blank `DefId` in the init section was empirically classified
>     as a “consistency blank” rather than a live defect (codegen name fallback can resolve it, and
>     all four kinds of init call execute correctly); registered as WBS 3.7.4 to close the “function
>     body uses def, init uses name” semantic split.
> 18. Closure of the init-section DefId semantic split (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     was extracted into a single-point `resolve_static_targets` (Call/TailCall string operands +
>     MakeClosure); function bodies and `ir.init` share the same implementation and the same
>     `resolve` closure; regenerating the 208 IR snapshots produced only 309 `(def=_)`→`(def=dN)`
>     changes (other-lines 0; zero residual `(def=_)` entries in the init sections across the tree).
>     Under codegen's primary DefId path and name-fallback mode, translated output and execution
>     values agree item by item (all four kinds of init call empirically tested), with zero
>     corpus-baseline drift; the probe flips and strengthens under the new semantics (identity +
>     exact index + execution value; removing init coverage turns it red). **Correction
>     (2026-10-11)**: the commit IDs above were rewritten by `git pull --rebase`—the original
>     b554497b/e60dcc87/b7ac7396 correspond respectively to 375789d2/d922a551/38bd4b9e after the
>     rebase; the new IDs are authoritative in the dev history (the old IDs were local temporary IDs
>     and never entered the remote history).
> 19. 3.4.8 ③④ landed and validated (2026-10-11, 8ecf56f93). (③④'s design basis is note 14; ②'s
>     implementation is note 17, which this note closes.) **Implementation**: `CHECK_STAGES` ends
>     with `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs pure merging
>     only (aggregate requests + `link_module_irs`), and the entire entry-validation block remains
>     in `RoleClassification`; after `ir_generation_check` runs, `us.ir` is retained in memory only;
>     mono failures enter the collection channel under Check and retain `fail_with` under MultiFile;
>     `into_outcome` first calls `take_check_diagnostics` and then evaluates failures (structural
>     failures no longer clear already collected diagnostics). **Validation**: four rounds of
>     independent validation (t3/t4/t10 + a t5 cross-form matrix) and one review round (t6) all
>     passed; the corpus differential is 340/340 line-by-line IDENTICAL, IR snapshots have zero
>     drift, and the test count grew from 2643 to 2656. **An existing gap discovered and fixed in
>     the same round**: F1 (the request name and IR definition key in rootless multi-file check do
>     not share a source → silent failure to specialize, see the implementation note in WBS 3.4.8's
>     row)—the first fix precisely reverse-looked up the real definition key (rather than using a
>     heuristic) (`flat_defs`); **same-day correction**: that bridge was removed during the
>     regression fix, with the root cause instead resolved by decoupling the dual role of
>     `module_key` (`script_mode`), so all three share the source `{unit key}.{name}`. The
>     regression's localization (the CI documentation-example gate went from 258 blocks / 10
>     failures to 0 failures) and fix are described in the implementation note in WBS 3.4.8's row.
>     **Retained record (existing, not a regression from this round)**: E3005's help text does not
>     match the resource-protection semantics; the multi-file E3005 span label is misaligned
>     (reproducible with the HEAD binary); the type-parameter position dependency on the collection
>     surface remains on the same side (assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and diagnostic-surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard reachability)**:
>     interpreted calls changed to an **explicit stack-driven**
>     model—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frames and return control
>     to the outer `step_one`; frames carry `ReturnTarget{Host|CallerSlot|Discard}`, and on return
>     they write back to the caller's slot; multiple call instructions continue through
>     `pending_calls` in the frame. Baseline measurement showed each interpreted-call layer consumed
>     **2544 B** of native stack, with the crash threshold at `count_down(394)` (Windows main
>     thread, 1 MiB); after the fix, 20,000 levels of recursion run on a 512 KiB stack thread
>     (approximately zero per layer). The guard (`push_frame`) is thus reachable from every entry
>     point under every `max_stack_depth` setting; `ExecutorError::StackOverflow` now carries the
>     real limit (formerly `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2
>     (deep-stack echo truncation) was merged into this task by user ruling** (same file and
>     inseparable at file granularity): echo now shows the first 16 frames + an omitted count + the
>     last 8 frames, while shallow stacks remain unchanged frame by frame. **3.10.1 (no compile-time
>     guard for native recursion, #447)**: locate first, then modify—the crash point was recursive
>     `MonoType::type_name` (**328 B/layer**, determined with an SP probe + per-operation
>     thresholds); the same path also contains derived `Clone` (~400 B/layer), a deep copy inside
>     `substitute`, `type_name_hash` (164 B/layer), and `collect_generic_type_refs`. After
>     converting all four to work lists and removing the per-layer deep copy of the entire request,
>     pathological generic recursion under `mono.max_depth=5000` produces **E3005 (with the real
>     limit)** instead of overflowing the stack. **3.11.1 (task-boundary error-code fidelity,
>     #448)**: the task result's error side changed from a value to a **structured** form
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), with values
>     restored by value across the boundary; E6001/E6003/E6004/E6008 inside tasks are word-for-word
>     identical in Standard and Embedded messages. Two **pub API payload-shape changes** are marked
>     `BREAKING CHANGE` in the commit. **Commit-strategy correction**: 3.9.1 and 3.11.1 are
>     **inseparable at file granularity**—`interpreter/executor/executor.rs` carries both the
>     trampoline and the error-channel producer/consumer, while `TaskFailure` is defined in
>     `runtime/engine.rs`; staging only one side cannot compile (the pre-commit hook compiles the
>     staged subset and rejected it twice), so under the same principle as note 15 they were
>     combined into **one atomic commit**. **Known boundaries and follow-ups**: Drop derivation (~75
>     B/layer) ⇒ `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation) gate
>     relies on native recursion (each layer ~3,696 B; the default 256 layers ≈ 924 KiB) and
>     currently has no call sites and is unreachable in production; future wiring must close the
>     same family of issues. The Standard-mode task interpreter uses the default `ExecutorConfig` ⇒
>     its E6004 limit is always 1024 (configuration propagation registered as WBS 3.12, user ruling
>     B: a separate team); test-runtime budgets are assigned to issue #449 (pathological fixture 76
>     s, clippy on both targets, full commit run), outside this RFC's construction scope. **Corpus
>     directory stratification (same round, 8605a081b)**: `02-type-system` (78) and
>     `06-compile-errors` (134), totaling 212 fixtures, were split into 23 category-based
>     subdirectories; 88 IR snapshots were relocated to mirrored paths; the corpus baseline was
>     regenerated in the same commit (342 lines, independently recalculated and compared line by
>     line: only the `file` field and two new changes), and 41 snapshots were regenerated because
>     their file-header paths became longer; line-by-line verification showed **only span-offset
>     changes** (zero differences after offset normalization).
> 21. LSP semantic-data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, and cross-file references in a project carry
>     `resolves_to` pointing to the real definition. The old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics went through the unified Driver pipeline,
>     while semantic data went through server.rs's manual lex→parse→single-file typecheck (no module
>     registry, so in-project `use` resolution failed and reference events were not produced;
>     cross-file navigation/rename/find-references silently failed to resolve—D55's silent-failure
>     class was closed by this task). **Implementation**: (1) `Export` carries `definition_span`
>     (harvested from AST identifier nodes by `extract_module_info`, with the same source and value
>     as the typecheck definition event's `def_span`; native std always has None), and `ModuleInfo`
>     carries `definition_file` (filled by the Registry arm from `DiscoveredFile`); (2) when the
>     checker drains reference events, it uses `import_def_locations` to redirect a cross-file
>     import item's `resolves_to` from the use line to the real location in the definition file (std
>     imports have no registration and continue to resolve to the use line); (3) the checker's
>     module identity changes from the unified `"<module>"` to the unit file path (this value is
>     consumed only by the semantic-data surface, with zero diagnostic consumers—all four
>     `env.module_name` consumers were empirically verified); (4) `DriverOutcome` gains a
>     `semantic_db` channel—collected in CollectAll forms (Lsp target file regardless of diagnostic
>     success + SingleFile fallback), and the Lsp arm injects top-level export definitions for
>     non-target files from the registry into the tail (`external_definitions`, allowing navigation
>     to unopened files; type/submodule exports are not injected—typecheck-side type registration
>     does not produce definition events, so both sides preserve the same omission); (5) the
>     orchestrator gains `analyze_source_in_project` (`check_source_in_project` delegates to it and
>     takes only the diagnostic half); (6) the LSP side's `analysis.rs` single-compilation entry
>     point replaces the manual pipeline (`update_semantic_db` is deleted from server.rs), and the
>     World key is unified to a normalized file URI (`uri.rs`: `path_to_uri(uri_to_path(u))` is
>     idempotent and normalizes drive-letter case and `%XX` encoding differences to the same key).
>     **Incidental fix (D59 routing class B—old behavior silently did nothing and does not change
>     the externally visible behavior of any previously successful program)**: in-project cross-file
>     navigation/rename/find-references changed from silently resolving to nothing to matching real
>     definitions; on lexical errors, the old manual pipeline deleted semantic information, whereas
>     the new pipeline retains a partially typed AST for coloring (an extension of the 4.2.6 editor
>     philosophy); when the target file has type errors, semantic data is no longer discarded with
>     `type_result`, so coloring and navigation do not disappear while the user is typing. **Pinned
>     tests**: six driver-level tests (target-file path key / cross-file resolves_to +
>     external-definition round trip / std-import fallback to use line / error-file retention /
>     single-file fallback path key + error retention), one LSP end-to-end test (empirical
>     red-to-green—navigation returned None before the fix), and four URI-normalization tests.

### Migration of the ten entry points (each function specified individually)

| Entry point                                                | After migration                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and hand it to the Driver                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                           |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a constructor for `Program { kind: MultiFile, aggregation: FailFast }`                                                                                                                                  |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a constructor for `Program { kind: Check, aggregation: CollectAll }`                                                                                                                                    |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter the result to the target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                         |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                            |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                               |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                            |

**Approximately 40% shared code / 60% divergence between `check_project` and `compile_project`.**
The shared parts are vendor-consistency checking, file discovery, `build_registry_from` (deleted on
2026-10-26 after its last caller was removed; the historical account is retained here),
`all_method_bindings`, the parsing loop, per-file checker assembly, and result handling. The
divergent parts are items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton
is precisely Driver's value range**—the divergent 60% consists entirely of “a different set of
stages” or “a different aggregation mode,” which are exactly the two things expressible by
`Program`'s fields.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had non-blocking problems; continue
}
```

**Key design decision: “an upstream failure causes this stage to be skipped” is not a stage return
value.**

The reason is that skipping is determined by **topology**, not by the stage itself. If each stage
returned `Skipped`, the information “why did I not run?” would be dispersed across 12 stages and
could not be centrally audited. Instead, `dispatch` determines this at the beginning of each loop
iteration according to dependency relationships:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect targeted by this
document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip” that
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every “Y did not run because X was not done” must be explainable.

Diagnostic text must distinguish three skip reasons:

| Reason                                                   | Text direction                                                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Caused by an upstream `Abort`                            | “Stage Y was not executed because upstream stage X failed”                                                                           |
| Caused by an empty prerequisite obligation               | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; not included in the warning count** |
| Conditions not met (e.g. `config.mono.enabled == false`) | “Stage Y was not executed because it was not enabled by configuration”                                                               |

The second category is critical: it distinguishes “normal skipping” from “abnormal skipping” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract on
which `diagnostic/mod.rs:637-641` depends).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Stop at the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                              | Current equivalent |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` returns on the first error as `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)    | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project files and single files within the same process, while CLI `run` and `check` are
independent invocations. Placing it on Program avoids mutable global state in the Driver.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ in more than “whether they
return early”—they are two different checker entry points. After unification, a single
implementation should be driven by the `Aggregation` parameter rather than preserving both functions
(**this is an additional item requiring verification; see “Known Limitations and Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                          |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| `TypeCheckResult` → `Obligations` migration | **Field types do not change**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                              |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain in
their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers (such as `ir_gen` reading `release_plan`) continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
driver no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2), `middle` (L3), and `backends` (L4); **L2/L3/L4 must not depend on `crate::driver`
in reverse**. The presence of `Driver` in a `TypeChecker` import is a violation and is blocked by
the `scripts/ci/check-module-boundary.py` gate proposed in RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | Five stages                              | Same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone files—it uniformly uses `compile_project` (MULTI_FILE stage table), with Script semantics driven by the rootless path; this row now describes only the `lib::run_file` / `Compiler::compile` library path)                                                                                                                                                                     |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected as P3's stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel”; Check / Lsp / MultiFile share the same implementation, with no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for this arm to emit) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Connected as P3's stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| LSP (within a project)              | No proof_execution                       | Connected as P3's stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

The **only intentional breaking behavior** is changing the `.expect()` at `predicate.rs:35` to
return a diagnostic. This conforms to RFC-027 §8's position of “not binding to a specific solver”
and to the contract already declared in `backend.rs:60` (“the caller should degrade
conservently”)—the current implementation contradicts its own contract documentation.

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

| File                                          | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                  | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                              |
| `src/lib.rs`                                  | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                            |
| `src/frontend/pipeline.rs`                    | `141-227`                                      | Replace the `Pipeline::run` body with `Program` construction + Driver invocation                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                    | `275-278`                                      | Move dead-code analysis out; make it the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                    | `306-311`                                      | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                    | `381-389`                                      | Move monomorphization out; make it the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`         | `450`                                          | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`         | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                      | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                           |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics`; call the Driver                                                                                                                                                                                                                                                                                                     |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                      | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                                |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                             |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently reaching path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                          |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                              |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “a lower layer's failure prevents an upper layer from running” (there is no short-circuit)                                                                                                                                                                                                          |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                        | **Under D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure interface; change the `predicate.rs`/`checker.rs`/`ownership.rs` consumers to use the shared interface. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`), and D58 takes precedence over the original wording |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed across
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; formerly
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is in the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; all `#[cfg(target_arch = "wasm32")]` branches (wasm branch
reachability is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                       | Compatibility                                                              | Disposition                                                                                                                                                      |
| ------------------------------------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                   | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in stages, and document the expected diagnostic-set change in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                 | **Breaking**: programs that compiled successfully now produce warnings     | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                   |
| First run of `assert_drained()` produces W-level diagnostics | **Breaking**: new diagnostics                                              | W first, then E; split S2/S4 into two steps                                                                                                                      |
| Change `predicate.rs` panic to a diagnostic                  | **Improvement**: no longer crashes                                         | No breakage                                                                                                                                                      |
| `build` / `dump_bytecode` subcommands                        | **No impact**                                                              | These paths do not pass through proof_execution                                                                                                                  |
| Migrate `TypeCheckResult` fields to `Obligations`            | **Internal refactoring**                                                   | If external consumers depend on the `pub` API, synchronize them; all in-repository consumers are listed in the change list                                       |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contracts and unified
Driver)** in RFC-039's global stage sequence. The following S1–S5 are the internal implementation
order of this document. **Each step's acceptance criteria reference C2** in the
[Equivalence Criteria document](07-equivalence-oracle.md) (orchestration changes: identical
**diagnostic sets** across entry points + identical corpus behavior).

### S1: Establish the criteria (red first)

**A prerequisite that cannot be skipped.** The defect criteria must be written as failing tests
first.

| Deliverable                                                                                                                                                        | Acceptance                                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                      | **Must be red**. If it unexpectedly passes, this document's defect analysis must be reviewed again          |
| Multi-file fixture layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file fixture tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions can be compared                              |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                                 | Mark `#[ignore]`; turn green in S4 (red-state record: removing the sole settlement point yields `pairs: 0`) |

**Rollback point**: no code changes; pure test additions.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                           |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array           |
| Change `pipeline.rs:141-227` to call the Driver          | **The single-file path's diagnostic set and exit code are byte-for-byte identical**                       |
| Differential test over the full corpus (293 `.yx` files) | Diagnostics list (sorted by `(code, file, line)`), exit code, stdout/stderr all have **zero differences** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance criterion is zero-diff at the C1/C2 level.

**Rollback point**: `git revert` one commit to restore the original `pipeline.rs`.

### S3: Merge the four orchestrator entry points + fix the defects

| Deliverable                                                                                                                  | Acceptance (C2)                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points become identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                                 |
| Delete the LSP manual stage sequence                                                                                         | LSP and CLI produce the same diagnostic set for the same file                                      |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                | The defect is fixed                                                                                |
| Fix the ownership layer's empty `Unproven` match arm (formerly `checker.rs:1313`)                                            | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode              |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | No panic when Z3 is absent                                                                         |

**Rollback point**: use **two commits**, one for defect fixes and one for structural merging. If the
structural merge causes problems, only the structural commit can be rolled back while retaining the
defect-fix commit—in that state, `test_multifile_proof_obligation_not_dropped` remains green; the
reverse (retaining the structure and rolling back the fix) turns it red, which is an unacceptable
intermediate state and must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                                 |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 test cases in `src/driver/tests/obligations.rs` turn green (implementation form: see the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                               |
| Promote obligation diagnostics from W to E   | Manually review every new E after the full corpus differential                                                                                             |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration item;
switching between W and E does not require structural changes.

### S5: Finish the proof layer and wasm work

| Deliverable                                                                                                                                                                                                                                               | Acceptance                                                                                                                                                         |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                          | Every entry corresponds to a call site in `checker.rs`                                                                                                             |
| Process-wide shared singleton for `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes shared state between cargo-test threads) | Production-path `default_solver()` call sites fall to zero (tests only); cache-hit rate is observable across the three consumers (the counter landed in 89576fafd) |
| Add a warning to the `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`) degradation                                                                                                    | A diagnostic is produced when there is no `body_checker`                                                                                                           |
| Reachability assessment of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                          | The conclusion is submitted to the “F2 assessment” subsection of `06-cleanup-inventory.md`; this document registers only the assessment requirement                |

**Caution**: correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item should proceed independently of S1–S4** and must
not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustively-enumerable enum; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                                    | An exhaustive `match` provides zero-cost compile-time enforcement: adding a variant causes `dispatch` not to compile. This is isomorphic to the mechanism already proven effective in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that require human reasoning                                                                                                                      |
| **Obligation mechanism** | `Obligations` + `assert_drained()` provide **runtime** settlement, used together with the **static** gate `scripts/ci/check-obligations.py`; **the final severity is E** (user ruling A on 2026-10-13: same-code promotion to E with the code number unchanged; **implementation form: see the §2 revision note**: the ledger does not duplicate the field structure; the red list—zero entries after cleanup—retains its mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not one bug: at least two instances of the same hazard have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. The two-step W→E process makes every intermediate step testable against C2—now that the inventory has been cleaned to 13 entries and the red list is zero, with zero emission in the healthy state, the final state directly uses E (see parts 7 and 8 of the §2 revision note) |
| **Fix scope**            | Fix the **consumer** (orchestration layer); do **not** change the production-side logic of the `Unproven` branches (now three sites in `checker/refinement.rs` and one in `checker.rs`; formerly `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                                               | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic has been fixed,” while the logic of these branches is itself correct                                                                                                                                                                                                                                                                            |

This design additionally eliminates two classes of cracks:

- The crack between a **comment-level promise** and **code behavior**. The “silent pass must not be
  allowed to return” promise in `checker/refinement.rs` (formerly `checker.rs:5165`) is a
  **comment-level contract**, enforced through `assert_drained()`.
- The crack where **“an array was not appended to.”** Today, five functions each hand-write their
  invocation sequence. After unification, there is one stage table, and a missing new stage causes a
  compile-time error.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type continuity**; they are configuration questions of
  “should this analysis run?” This approach also cannot express `Aggregation`, and merely moves the
  `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail, replacing “five
  functions each hand-write their invocations” with “an array was not appended to,” which is
  precisely the cause of the current 11 inconsistencies. This repository already contains a
  counterexample of the same form: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache`
  / `HotReloader` described in `docs/src/dev/design/check/` all have zero implementations.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields; `Stage` remains scattered across five functions, and the next
  point of divergence will continue to grow from here. **The structural change must come first** (as
  part of S1/S3), because the structural refactoring requires a known red test proving that the
  criteria are effective.
- **Make `proof_calls` `pub` and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must occur there.

## Known Limitations and Risks

- **Slimming `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several Driver arms,” a reader must cross
  two files to understand the flow. This is the common cost of every refactoring that “centralizes
  wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors.” These are real bugs that
  were never reported before. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires field-by-field manual judgment.** In some
  obligation fields, such as `module_namespaces`, the absence of a consumer may mean that no
  consumption was designed, so no warning should be emitted. S4 must review every field rather than
  applying a blanket rule.
- **The `Program` abstraction may be premature.** If the stage sets of some entry points remain
  unstable for a long time, `stages()` may degenerate into a free parameter that accepts a different
  array on every call, and the contract constraint will fail. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can only come from the six predefined
  combinations.**
- **Dependencies on criteria**: the defect-fix criteria for S1/S3 depend on the defect tests in the
  [Equivalence Criteria document](07-equivalence-oracle.md) turning red first. **If that document
  has not first established a multi-file fixture layer, S1 cannot be accepted**—all 293 existing
  fixtures use the single-file path and provide zero coverage for this class of defect. This
  document does not cover the IR verifier (`verify_loose`); that prerequisite is out of scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse of
  their declaration). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undecided.** `backend.rs:13-19`
  already states that `Solver` is `Send` but not `Sync`, and the `Z3Backend` cache uses `RefCell`;
  after changing it to a singleton shared across compilation units, all access paths must be
  confirmed to pass through `Mutex`.
- **Stage parallelization has not been assessed.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **hide ordering-dependent defects** (such as the actual
  ordering dependency between termination and ownership). It should be considered only after the
  equivalence criteria are stable.

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **This document retains no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1–G10 acceptance gates, P1–P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch-reachability cleanup;
  `checker/semantic_tokens.rs` `include!` conversion (construction steps assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1–C6 tiers, three levels of criteria,
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — Explicit statement that “silent pass must not be
  allowed to return” (formerly `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — Layer-order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — Explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — Contract statement that “the caller should
  degrade conservately” (contradicts `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-107` — 16 obligation fields that downstream code must consume
  (after the 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — Unit test's own
  acknowledgment that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Fixture's own acknowledgment that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (used by `run_code` to compile) — The playground's
  fourth compilation caller (the single-file path, so proof_execution runs)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — The error-code build-time gate, an example of this project's enforcement
  mechanisms
