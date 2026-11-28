# Compilation Stage Contract and Obligations Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main body of RFC-039 for the four-layer model, acceptance-criteria tiers, and execution-stage
> order; see the [index for this directory](index.md) for the role of each companion document.

## Positioning and Scope

This document is the construction blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“Which compilation stages ran, which did not, and why?” is currently wired manually
across six paths and ten entry functions, with no single place able to answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being violated by the architecture rather than by the
> logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` reads (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile error, no fallback, no silent
pass……silent pass must not be allowed to return.” However, this invariant holds only on the
**single-file path**: proof-function constraints such as `x: Sorted(3)` **silently pass** on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed,
and no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustively enumerable, with no runtime registration) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, plus the individual migration path for each of the
  ten entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by these changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency-direction rules, and bounce-back gate → `01-routing.md`
- Equivalence criteria (C1-C6 tiers and the three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigms → `05-frontend-paradigm.md`
- Dead-code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- The global P1-P10 execution order and G1-G10 acceptance gates → the main body of RFC-039

### Division of Labor with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should be performed;
this document defines L1's **concrete shape**, **per-file change list**, and **implementation stages
within this document** (corresponding to P3 “Fix correctness vulnerabilities” and P4 “Stage
contracts and unified Driver” in RFC-039's global sequence). If this document conflicts with
RFC-039, RFC-039 takes precedence.

Equivalence criteria are applied under the **C2 (orchestration change)** category of the
[equivalence-criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> Every item in this section is a **verified fact**, with a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Sole Structural Defect Where One Mistake Silently Affects the Entire Repository

This differs in nature from the other two classes of defects—module boundaries and test wiring:

| Defect                | Typical manifestation               | Is there a signal?       |
| --------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntax error  | Source code is wrong                | Yes, a diagnostic        |
| Type mismatch         | Types are wrong                     | Yes, a diagnostic        |
| Missing stage wiring  | A stage is never invoked            | **No signal whatsoever** |
| Unconsumed obligation | A field is populated but never read | **No signal whatsoever** |

The latter two share a common characteristic: **failure produces no error**. They therefore cannot
be solved by “writing the code more carefully” or “reviewing it more strictly”—code review can show
what was written, but not **what was not written**. This is precisely the root cause identified by
RFC-039: “This project treats ‘design’ as a documentation convention rather than as an executable
constraint.”

### Existing Enforcement Mechanisms in This Repository

The same repository already has mature enforcement mechanisms; they simply have not been extended to
the stage layer:

- The **145 error codes** under `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** through `tools/code-tables` at `build.rs:19-55`, which compares them
  entry by entry with the RFC-013 code table and rejects the build with `panic!` on any mismatch.
- `src/package/` has high test density (**76 files / 13,012 lines**, including 6,227 lines under
  `tests/`, or 47.9%), and every module's test subtree has declared wiring. This is the repository's
  most completely wired test area and can serve as a formal reference for a stage-layer gate.

**The design capability is sufficient. What is missing is “an equally strong gate in the
orchestration layer as well.”**

(Line-count convention: `(Get-Content).Count`; see “Line-count convention” in
`06-cleanup-inventory.md`.)

### Six Compilation Paths and Ten Entry Functions

| #   | Path                       | Entry function                                        | Location                                         | Which compilation path is used                                                     |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct calls to 5 stages                                                           |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **LSP within a project**   | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + separate IR                                              |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multiple files**     | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | Uses 3c within a project; otherwise manually lex→parse                             |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                     |
| ir_generation       | `205`         | `run_ir_generation`, with **monomorphization embedded in it** (`381-389`) |

### 11 Stage-Coverage Inconsistencies

The following table provides evidence cell by cell. **An empty cell does not mean that the entry
point “does not do this,” but rather that “no code in the entry point does this”**—which is
precisely the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                          | `check_project`                                                       | `check_source_in_project` (LSP)    | `compile_embedded_module`                   |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                                | **No**                             | **No**                                      |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, with no configuration gate)                | **No**                             | **No**                                      |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                         | **No**                             | **No**                                      |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)         | **Collected but never emitted**                            | Yes `350`                                                             | **No**                             | **No**                                      |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                       | **No**                             | **No**                                      |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | Not applicable                                                        | Not applicable                     | **No**                                      |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting everything | `check_module_collect_all` (`493`) | `check_module` (`1386`)                     |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                          | `discover` (`454`)                 | Not applicable                              |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                       | **No**                             | **No**                                      |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                        | **No**                                                                | **No**                             | **No**                                      |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                                | **No**                             | Yes (separate ModuleIR followed by merging) |

> **Review note (WBS 3.4.3; cell-by-cell verification on 2026-10-07)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. The current-state differences after
> P3 are as follows:
>
> - **Row 1 has been fixed**: all five entry points share the same `frontend/proof_execution.rs`
>   implementation for proof_execution (pipeline + four orchestrator entry points), eliminating the
>   unique-consumer defect.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6; user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and the dead-code-family warnings, and the W-code sets of the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) are normalized—the
>   single criterion source is the shared implementation in the `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk not demonstrated).
> - **Row 7**, the dual `check_module` / `check_module_collect_all` entry points → assigned to 4.2.7
>   (Aggregation-parameter-driven).
> - The code-side reference to “ruling #434” had previously not been registered in the docs—it has
>   now been registered as RFC-039 **D57**.
> - **Asymmetry in including embedded std (new fact outside the table, retroactively registered on
>   2026-10-09)**: the single-file path unconditionally injects std.list through
>   `merge_embedded_std_ir` (required for desugaring `for` loops, a hard #117 switch), whereas
>   multi-file `discover` previously recognized only explicit `use` declarations—`for` loops within
>   a project compiled successfully but failed at runtime with E6006 (confirmed by a probe). This
>   has been fixed and WBS 4.10.2 has been closed. The related “multi-file path lacks the
>   single-file parse hard-abort quirk” was registered as WBS 4.10.1—**it has also been fixed**
>   (immediately after 4.2.2; the Check path was downgraded to per-file collection. This audit
>   method examines the “stage coverage and field consumption” dimension, whereas this issue is a
>   compilation-unit membership difference and therefore slipped through that dimension).
>
> - **Embedded-std registration-surface coverage (discovered during 4.2.5 implementation, already
>   fixed)**: the Registry arm repeatedly harvested registrations for embedded std units through
>   `extract_module_info`, overwriting the native half registered by `with_std()` (the
>   `result.is_err` surface was lost, causing std.test to report a false E1043). This is the third
>   missed dimension of this audit: merge semantics for “two registration sources under the same
>   module key.” The single-file path's registry is formed only once through `with_std()`, whereas
>   the multi-file/Check Registry arm inserts unit by unit and therefore has coverage.

**Two points require precise wording; otherwise the description would be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` **never** reports W1001/W1002/W1003 on
  the **multi-file** path (`run_project` → `compile_project`); it **does** report them on the
  **single-file** path (`run_file` → `Pipeline::run`). The reason lies in `compile_project`: it
  packs the complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function
  **has no read site for `result.warnings` anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The entry-point `main` criterion in row 11 also has a different source**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, checking only whether a manifest exists). The two
  functions give different answers to “which files must define `main`?”

### Correctness Vulnerability: Proof Obligations Are Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are independently verifiable.**

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

Its semantics are: “The predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed before the constraint can be decided.”

**Step 2 — The consumption point is inside the checker, but it only produces and does not consume.**

`src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                      | Location                                                                                                                 | Behavior                |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Formal-parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`) | `if calls.is_empty()` → hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`)  | Hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)        | Hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no fallback, no silent pass.` ……
> `This branch therefore becomes a real line of defense rather than a forward-looking placeholder: silent pass must not be allowed to return.`

In other words, the author knew that “emitting no diagnostic” was a defect, explicitly classified it
as a **known intermediate state with nowhere to go**, and assumed by design that “someone would
eventually come to read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.**

`check_module_impl` declares a local `proof_calls` and collects into it (originally
`checker.rs:1234-1235`), then writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof-function obligations` (originally `:1439`). The
field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.**

In production code, `TypeCheckResult.proof_calls` (`types.rs:29`) is read in **only one place**,
`src/frontend/pipeline.rs:187` (passed as an argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(The other occurrences of the `proof_calls` identifier fall into three categories, none of which
consumes this field: the three production-side collections in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); `verdict.rs:61` is a same-named field of
`ProofResult`; and `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points passes through that layer.**

The four entry points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module`
(`1374`)—**all do not invoke `pipeline.rs`**. They call `TypeChecker::check_module` directly (`124`
/ `493` / `1386`). Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discard path.**

`compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible for
compiling embedded std. This means that **the embedded std module's own proof obligations are not
executed either**.

**Step 7 — Consequence: constraints silently pass.**

`y: Sorted(3) = 5` (`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs
successfully, and produces no diagnostics** on the multi-file, check, and LSP paths. The proof
function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layers.**

When `check_module_impl` handles ownership-layer results (originally `checker.rs:1303-1314`):

```rust
// 原 checker.rs:1313（现 check_module_impl 内）
ProofResult::Unproven { .. } => {}
```

**The ownership-checking layer's `Unproven` is swallowed by an empty match arm, with no diagnostic
and no accounting.** This directly contradicts the promise at the `5164` branch that “silent pass
must not be allowed to return,” and is **independent of the orchestrator problem**—even if the
entry-point layer is completely fixed, this point will remain silent. (Processing timing: it should
be handled in the same batch as the obligations ledger because it is the same class of problem.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** The multi-file path has zero coverage of proof
obligations.

The three RFC-027 unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists in
`result.proof_calls`. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test files' own doc comments already make this point. `rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so cases that violate refinements are at the .yx layer`

**This is exactly the shape of the problem: the tests verify that “obligations are populated,” while
the bug is that “the consumer never reads them.”** A test that exercises only the producer, not the
consumer, is inherently immune to this class of defect.

### Stronger Finding: The Main Corpus for Equivalence Criteria Uses the Single-File Path

The [equivalence-criteria document](07-equivalence-oracle.md) defines end-to-end differential
testing over the 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and
the primary acceptance method for C2-stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero glob matches in the
  entire directory).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files use the single-file path, all cover proof_execution, and
  none covers the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is concrete
evidence of this point. The file is annotated `// expect: compile-error E4018` on line 15, and its
header comment on line 7 says “Status: ❌ should be rejected at compile time”—**it passes precisely
because it uses the only path that executes proof functions**.

**Conclusion: the third criterion layer needs an additional multi-file corpus layer; otherwise it
cannot serve as the acceptance tool for this document.** See “Implementation Highlights” S1.

### Obligation Fields: 15 “Produced Means Contractual” Fields with Zero Enforcement Mechanism

Among the `TypeCheckResult` fields in `src/frontend/core/sema/types.rs:16-98`, **15 fields** have
doc comments explicitly stating “produced by stage X → consumed by stage Y”; in other words, they
are inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first doc-comment line—the comment block for the same
> field may span multiple lines, and only the declaration line provides a unique anchor. The
> **struct range** in the sentence above this table (`src/frontend/core/sema/types.rs:16-98`) moves
> with the struct's line number and is mechanically reconciled by gate **F5** (a mistake fails
> immediately); definition-line columns are reconciled by **F4**. | `proof_calls` | `36` | typecheck
> → **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by
> `PlanId`**: read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-completion marker `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation | | `record_models` | `79` | typecheck (one declaration-time
> pass) → IR generation (quantifier propagation; from B.2 onward, both the witness field's
> `TypeDeclField.quantity` and constructor-argument/default-value population read this table) | |
> `call_result_types` | `97` | typecheck → IR generation (**keyed by `PlanId`**—call-site identity:
> produced by the Call arm in `inference/expressions/expr.rs`, consumed by
> `lower/query.rs::expr_value_type`; export-point convergence; unresolved entries are not exported)
> |

**Reclassification record (3.4.9, user ruling A on 2026-10-13)**: the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design it
is consumed within the typecheck phase and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption points (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), read-only accessors at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed in this round).
- **Disposition**: remove the documentation comment's claim that the field is “a mirror for
  consumption by the middle layer/LSP”; retain the field and its in-phase consumption. The field is
  registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP later
  genuinely needs the table, register it through a new 4.6.x entry; **do not invent a consumer in
  this round**.

**These fields cross layers keyed by `PlanId` (statement/call-site structural identity), `Span`, or
name tables.** The equivalence-criteria document already points out that a mismatch in a span-keyed
contract can “fail silently, with no error whatsoever” (`ReleasePlan` is a typical case: if the two
sides compute spans in any slightly different way, the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module lies on every entry point's mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to a
  **different entry-point implementation**. The orchestrator's four entry points never pass through
  the `pipeline.rs` layer.

**Verified fact**: the repository currently has **no mechanism whatsoever** that guarantees these 15
fields are consumed. The only “protection” is that `ReleasePlan` happened to catch a ride with IR
generation.

### Registration of Non-Obligation Fields: Channel / In-Phase / In-Tree-Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (9 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation Fields”
table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Unit-identity metadata (0 hits in `src/middle`)                                                                                                                                                                                                                                                                                                                                      |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not a “produced means contract” obligation)                                                                                                                                                                                                                                                                                           |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its doc comment does not declare a cross-stage contract ⇒ not incorporated                                                                                                                                                                                                                                                        |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                                                                                                                                                                                                                                                 |
| `semantic_db`             | Consumer is in LSP (from 4.6.1, emitted through the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`), not a compilation-stage obligation                                                                                                                                                                                                                                |
| `trait_table`             | Has cross-stage reads at `ir_gen.rs:4336/:6867`, but its doc comment does not declare a contract                                                                                                                                                                                                                                                                                     |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                                                                                                                                                                                                                                          |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption points are described in “§Obligation Fields”)                                                                                                                                                                                                                                                                                |
| `method_def_ir_names`     | Cross-stage mapping (typecheck produces mixed-compilation names by method-declaration statement `PlanId` → IR generation reads only the name, WBS 7c.3); a miss produces a loud E3005 failure with no silent surface; consumption granularity is per declaration rather than non-empty per unit, so the ledger's “unit produced means it must be delivered” criterion does not apply |

**Why this is not “wiring up a consumer”**: creating downstream consumers for these fields would
invent false obligations and disguise “cross-stage consumption is unnecessary by design” as a
contractual defect. The correct closure mechanism is **reverse containment**—the `TypeCheckResult`
field set = obligation list ∪ non-obligation registry (hard-checked by criterion 1 of
`scripts/ci/check-obligations.py`: adding a field without registering it, deleting a field without
synchronizing the registry, omitting a reason for a non-obligation entry, or overlapping the two
tables all fail immediately).

### Four Additional Contract Defects in the Proof Layer

The following all come from proof-layer analysis and are independent of the entry-point divergence
issue, but all belong to the class “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation, with no mechanism enforcing it; the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (located by function name/driver point; line numbers are taken from
the current tree):

| Actual order | Call site                                                                                                                                                                                                                            | Corresponding layer |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` carries its own E2030 semantics; `judge_refined_obligation` is the sole decision unit shared by call-site and return-position obligations) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                      | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                         | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                                 | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                                                                                               | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comments and do not correspond to the call order; the README has no “number
→ call timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuit
whatsoever**—`check_module_impl` adds termination errors one by one with `add_error` and then
continues downward, so “an upper layer failing means the lower layer does not run” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-failure panic has been eliminated by P3's 3.3.1 (the SOLVER
> slot was made optional, and absence conservatively degrades to `SMTResult::Unknown`); “silent
> skipping (no injection)” was given a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is to be performed according to **RFC-039 D58**, at the actual fix location
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> interface). The formulation in the 02 change list stating `default_solver() → &'static` is
> superseded by D58 (a bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                     | Acquisition strategy                                     | When the solver is unavailable                               |
| -------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                         | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (via injection from `check_module_impl`, originally `checker.rs:1265-1268`) | Constructor-time injection through `with_solver_owned`   | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                  | **New instance on every call** via `default_solver()`    | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver kernel's capabilities’.” In contrast, the contract documentation at `backend.rs:60` says
that for `None`, “the backend is unavailable……**callers should degrade conservatively**.” **Two
philosophies coexist in the same codebase, and the hard-failure path panics instead of returning an
error.**

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

**This is a factory function, not a singleton**—each call creates a new `Z3Backend`, and therefore a
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the doc comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Because it is not shared
across calls, **the cache can never hit under this calling pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. It is the same
backend with two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match actual behavior.**

The two `body_checker` fallbacks in `check_module_impl` (originally `checker.rs:1283-1287` and
`1289-1293`):

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
whatsoever**. (Recommendation: record this as a warning-level diagnostic, or at least leave a trace
in the obligations ledger.)

The comment in `check_module_impl` says that termination checking runs “after type checking and
before constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm State: Carried by a Shim Crate, with the Branches in 29 Files Still Live

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate contains only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                    |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                    |
| The shim depends on the main crate (`rlib`) as a library  | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                           |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                       |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                              |
| **CI has 4 wasm builds**                                  | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt with Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from a fixed URL; degrades to a warning if missing)                                                                                                                                                              |
| **29 files** contain the literal `wasm32`                 | **22** have actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (the inventory and per-entry determination are in the “F2 Determination” subsection of `06-cleanup-inventory.md`)                                                                     |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                  |
| `src/lib.rs` has 11 occurrences                           | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call under the wasm
target—`src/lib.rs:151/170/187` gate `run_file` / `run_project` / `build_bytecode` in their entirety
(all three require `std::fs`), while the shim follows a different path.

**However, this produces a fact directly relevant to this document**: the playground entry point
`compile_playground` (`:84`) in `wasm/src/lib.rs` constructs `Program { kind: WasmPlayground }` and
passes it to `Driver::run` (the JS-side entry point `run_code`, `:122`)—the **single-file path**.
Therefore:

| Path consuming `proof_calls`                              | Executes proof functions? |
| --------------------------------------------------------- | ------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                   |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                   |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                   |
| Multi-file `run` (`compile_project`)                      | **No**                    |
| `check` (`check_project`)                                 | **No**                    |
| LSP within a project (`check_source_in_project`)          | **No**                    |

In other words: **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** The unified Driver already covers this path
through `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the same single-file
stage table as `SingleFile`, `:135`); the implementation record appears in the **4.2.8** row of
`09-execution-wbs.md` (`wasm` now uses `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 Determination” subsection of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time; six `not(wasm32)` arms are load-bearing in the native three stage tables; there are no
low-risk items that can be deleted directly. The RFC-039 open question “build a wasm target, or
delete these branches?” **has expired**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustively Enumerable, No Runtime Registration)

**Core constraint: `Stage` is a compile-time exhaustively enumerable enum; runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can enforce at compile time that “a new
stage must be handled by the orchestration layer”—exactly the mechanism already proven effective by
the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Supplier consistency verification      Project
    Discovery,          // File discovery                     Project
    Parsing,            // Lexical + syntactic analysis          PerModule
    Registry,           // Module registry construction           Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type checking (including embedded proof layers)   PerModule
    DeadCodeAnalysis,   // Dead-code family analysis                  Project
    ProofExecution,     // Compile-time execution of proof functions PerModule
    GlobalSlotAlloc,    // Global slot allocation                    Project
    IrGeneration,       // AST → ModuleIR                            PerModule
    Monomorphization,   // Monomorphization                           Project
    Linking,            // Cross-module linking / IR merging          Project
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

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once per
project) turn “which stages must run per module and which must run once per project” into a
type-level fact. Stages with `Project` scope are structurally guaranteed by `dispatch` to run only
once, eliminating questions such as “should `allocate_global_slots` be called for every file?” that
currently require human reasoning.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the draft's order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to dead-code analysis **before** proof according to
>    `Stage::ALL`'s topological order: in the current `check_project`, proof runs before dead-code
>    analysis, and the two have no data dependency. Their diagnostic sets are identical (C2 set
>    semantics); only the within-file diagnostic order is normalized. The single-file path
>    (dead-code analysis embedded in typecheck and preceding proof) already conforms to ALL's order,
>    so byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user ruling on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products—the draft's order would force the Registry arm to “hide parse,” making the stage table
>    misrepresent the data flow; multi-file parsing consequently dropped from 2 passes to 1. The
>    accompanying current-state fact is faithfully registered: multi-file parse errors were once a
>    hard abort (the `?` propagation in `build_registry_from`, in tension with CollectAll semantics,
>    registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 as part of clearing
>    zero-caller code, and this historical account is retained)—**4.10.1 has been fixed** (user
>    ruling on 2026-10-09: Rust-style collection semantics + plan B): a Check-path parse failure is
>    downgraded to per-file diagnostic collection, and the affected file exits the compilation unit
>    (does not enter the registry; its importer reports E5001); the MultiFile path retains its hard
>    abort (under FailFast, a bad file cannot produce IR, which is correct semantics and remains
>    permanently pinned).
> 4. The Check form's implementation (4.2.2, user ruling on 2026-10-09) added two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the draft omitted these (in the
>    orchestrator era these products were produced by `discover_with_used`, discarded by the
>    `discover` wrapper, and lost because they never entered the table); b. The within-file
>    diagnostic-order normalization was expanded to the full form of note #2: E3020 entry validation
>    moved into the RoleClassification arm (Check has no Linking stage, so the arm holding surfaces
>    and the AST performs validation); within-file diagnostic order = stage topological order (E3020
>    → typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set
>    semantics), and only the order of stderr entries changes.

### 2. Obligations Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn “a field was produced but nobody consumed it” from an undetectable condition
into a fact assertable at compile time or test time. RFC-039 already lists this as “the single
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
    /// Each unconsumed field produces one W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                                       | Criterion                                                               | Disposition                                                                     |
| ----------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| The declared consumer of an obligation reads it | The corresponding field in `Obligations` is `take()`n / marked consumed | Pass                                                                            |
| Obligation is non-empty but has no consumer     | Field is non-empty and not consumed                                     | **Produce a diagnostic** (`W` level by default; promoted to `E` in strict mode) |
| Obligation is empty                             | Field is empty                                                          | Pass (nothing to consume)                                                       |

**Why start with W rather than E**: fixing obligation discarding **changes the diagnostic set**. C2
requires that “the diagnostic sets of all entry points be identical.” Promoting directly to E would
cause `yaoxiang check` to suddenly emit a large number of previously silent `Unproven` diagnostics.
Splitting the work into two steps—first W for observation, then promotion to E—keeps every step's
criteria usable. See “Implementation Highlights” S4.

**`assert_drained()` has exactly one call site**: `Driver::run`, at the end of the stage table and
before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing at
least twice but with no read in a third file → failure). `assert_drained()` is the **runtime** gate,
while the script is the **static** gate; the two are complementary, and both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 implemented)**: this note registers the
> **implemented form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a corresponding code file:line for every item. The implementation commit is
> `1941cc47a` (verification tree `99079bc52`). It also carries the third of the three “not silent”
> responsibilities (the other two are the audit surface and the static gate).
>
> **1. Implemented artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201; field
>   names/consumer stages/red list), **non-obligation field registry `NON_OBLIGATION_FIELDS` (:175,
>   9 entries with reasons)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (:577), outward emission `emit_obligation_diagnostics` (:654), non-consumer
>   gating `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped` (:549),
>   IrGeneration delivery set `IR_GENERATION_DELIVERED` (:285), and W1090 code constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of the `Driver::run` stage table and before
>   `into_outcome()` produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path for `Aggregation::FailFast` settles as well (the
>   ledger must balance).
> - **Seven delivery points** (one per arm; removing any one makes the runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is `TypeCheckResult`'s sole outward
>   read path for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726`
>   (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) —four instances of the same
>   delivery point: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct request-table read) / `:1221`
>   (MultiFile·Check: aggregation in the Linking arm, with `driver/helpers.rs:295` reading each
>   unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is written above the entry at :26-31 —the settlement granularity
>   is “unit × obligation,” with no user-identifiable source location, so no dummy span is
>   inserted); all six locale texts are present in `locales/*.json`. **Emission severity**: starting
>   with 4.3.2 it is **error** (same code promoted to E; see the third subsection)—determined by the
>   explicit `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code number
>   remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above         | Implementation criterion                                                                                  | Code                   |
> | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ---------------------- |
> | The declared consumer of an obligation reads it | **A delivery marker exists** (marker ↔ real read reconciled by the static gate; see the fifth subsection) | obligations.rs:618-623 |
> | Obligation is non-empty but has no consumer     | Non-empty + consumer stage has run + no marker → W1090 (**error level**; see the third subsection)        | obligations.rs:624-630 |
> | Obligation is empty                             | `Obligation::is_produced` determines emptiness field by field → pass                                      | obligations.rs:587-591 |
>
> Two **additional tiers** (user ruling on 2026-10-13; not included in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **not
>    considered outstanding** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: The consumer stage is skipped because of `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not considered outstanding (obligations.rs:598-602). `NoObligations` **does
>    not** grant an exemption—if an obligation is non-empty while that stage acknowledges having no
>    obligations, the ledger is unbalanced and must be reported.
>
> **3. Outward-emission semantics (ruling “C”) and the measured figures behind it**
>
> **4.3.1 (observation phase)**: this mechanism emits **only W-level** diagnostics (W1090); there is
> **no strict switch**. **4.3.2 (final state, user ruling A on 2026-10-13)**: **the same code is
> promoted to E**—the `W1090` number remains unchanged, but the emission severity is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (using two codes for the same fact would create parallel representations). **The
> promotion changes only the emission severity, not the emission conditions.** **This is a D-class
> breaking item under D59 (an externally observable behavior change), faithfully registered**:
> healthy programs emit nothing (when all obligations have delivery markers), so existing corpora
> and user programs are unaffected (342/342 “difference: empty”); however, when a run really does
> encounter an undelivered obligation, that diagnostic now **counts toward `error_count`, is
> rendered as `error[W1090]`, and causes a nonzero exit code**—unlike the previous “W-level
> diagnostics do not block compilation.” The code family and emission severity are decoupled and
> this semantic is registered in RFC-013's “Emission Channel” section (a code prefix determines only
> numbering ownership; severity is determined explicitly at the construction point). Mechanism facts
> (measured): severity comes from an explicit `.severity()`; otherwise, “a code beginning with `W` →
> Warning” (`codes/builder.rs:186-203`), and the registry's `ErrorCategory` **does not participate**
> in severity; exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`), so **W1090
> follows `error_count` because of its explicit error severity (starting with 4.3.2)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain's ruling C on
> 2026-10-13; the current red list is empty, as described in the fourth subsection), for a reason
> that can be recalculated:
>
> - Ruling A (emit the red list as W) measured cost: `interface_impl_registry` is populated by
>   typecheck's prebuilt table and is **always non-empty**, so what is emitted per compilation unit
>   is a **constant signal** with no discriminating power, yet it breaks three existing external
>   contracts— “zero diagnostics for clean programs / zero diagnostics for LSP /
>   `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing cases turning
>   red**, of which **8 are in out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs`
>   ×5, `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4
>   are in `src/driver/tests/`), and the corpus baseline changes in 24/342 fixtures; the only
>   feasible workaround is to “filter W1090 out of zero-diagnostic assertions,” which weakens
>   existing criteria.
> - Ruling B (downgrade to Info) would still break 9/12 list-style assertions, and Info is not
>   “emitted according to the ruling's severity.”
> - Ruling C's measurement: **zero changes to existing cases** and **zero changes to the corpus
>   baseline** (342/342 “difference: empty, behaviorally equivalent”); runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains silent throughout, and removing
>   any one delivery marker makes it fire (all three delivery points were mutation-tested by
>   removing a marker; CLI `check` printed 2 `warning [W1090]` entries when the Check arm's marker
>   was removed).
>
> The red list's “not silent” guarantee is carried by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (carried on every run, with WBS IDs and
> evidence pointers); ② `check-obligations.py`, which prints the red list on every run (currently
> empty) and performs stale detection; ③ this note.
>
> **4. Red-list status (closed out) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three closure tasks were implemented on 2026-10-13:
>
> Note: the line anchors below record the file state **as of 2026-10-22**; after P5 (2026-10-26)
> split checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, the line
> numbers in `checker.rs` shifted earlier, so the anchors below no longer correspond to the current
> file and are retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → field deleted**. Its declared responsibility (“ir_gen generates
>   CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`), with the call site
>   at `:7496` replacing the entire Call with `CreateVariant`; the function's doc explicitly states
>   that “span keys are unreliable across systems (the two ends interpret the span of Call nodes
>   differently), so detection is performed in place”—**wiring a consumer would reintroduce the
>   already-fixed span-mismatch defect**. Deletion surface: the `types.rs` field; collection at
>   `checker.rs:1489-1493/:1565`; the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`; and the `VariantCtorCall` structure in `environment.rs`.
>   `infer_variant_ctor_call` type inference and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → cross-stage mirror deleted** (the in-phase candidate-name
>   table and `overload_resolutions` remain). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered alongside `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550`, the decision point; on the sole match, `:4550` writes into
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** injected into
>   `TypeCheckResult` at `checker.rs:1558`. The naming fact remains singly expressed: candidate-name
>   table (in-phase) + span-keyed decision (cross-stage, `ir_gen.rs:380` construction, `:7632/:7705`
>   consumption); the definition-side mixed-compilation name is computed independently by
>   `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in this document's §Obligation Fields table). What was deleted is the
>   documentation comment's claim that the field is “a mirror for consumption by the middle
>   layer/LSP” (`types.rs:47-48`); the field and its in-phase consumption remain
>   (`operator_interfaces.rs:623` operator lookup, seven constraint-solving lookups in
>   `inference/expressions.rs`, `checker.rs:214-217` read-only accessors); direct proof of zero
>   cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why a
>   consumer was not wired up**: the table's content is an environment mirror populated from
>   typecheck's prebuilt table (always non-empty); if LSP truly needs it, it should be registered
>   through 4.6.x; creating a consumer for a field that by design requires no cross-stage
>   consumption would introduce false obligations into the ledger.
>
> After closure, **the field inventory is closed by “obligation list ∪ non-obligation field registry
> (`NON_OBLIGATION_FIELDS`, with reasons)”**— the reverse-containment endpoint of gate criterion 1
> enforces this end to end (adding a field without registration or deleting a field without
> synchronization fails immediately). The red-list **mechanism** is retained: criterion 4's stale
> detection and the `--selftest` `stale-red-list` fixture continue to cover its detection surface;
> any future registration requires an explicit ruling.
>
> In conjunction with closure, the field-count semantic changed from 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / this document's §Obligation Fields table); the gate's `--selftest` now covers
> seven fixture classes (adding “unregistered field,” “stale anchor,” and “empty red list”), under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2) (three hard criteria beyond the sketch's sentence)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document's “§Obligation
>    Fields” table (including the table header's count declaration); any drift fails immediately;
>    furthermore, the `TypeCheckResult` field set must be **exactly equal** to “obligation list ∪
>    non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field without
>    registration, deleting one without synchronization, omitting a reason for a non-obligation
>    entry, or overlapping the two tables all fail immediately;
> 2. **Read evidence**: every non-red-list obligation must have a real read in the **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings do not count)—this makes
>    “delivery marker ↔ downstream real read” a hard criterion (the principal risk of mechanism A is
>    precisely drift between the two);
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    conversely, red-list entries must not be marked;
> 4. **Red-list stale detection**: red-list fields must have zero real reads (growing a consumer
>    causes a failure), and every entry must carry a WBS ID and evidence pointer; there is also
>    `--selftest`: it includes negative/positive fixtures, where negatives must fail and positives
>    must pass (proving the detector is not vacuous).
>
> **6. Every deviation from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy the fields one by one**:
>    obligation data remains held by `TypeCheckResult`, while the ledger records only the
>    “inventory + delivery markers + settlement result.” The reason is that copying the fields would
>    create two sources of truth (each field is produced by a different producer within typecheck's
>    checker and ownership layers) and inevitably cause drift; the cross-layer data plane should not
>    be copied into the orchestration layer either.
> 2. The sketch's `take()` / “mark consumed” was **not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The reason is that an obligation and its
>    product are the same data, and `take()` would remove what downstream code needs; moreover, 11
>    IrGeneration obligations have the shape “hand the entire `&TypeCheckResult` to ir_gen,” which
>    cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the result is written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed by **severity** (since 4.3.2, W1090 is E level)—the single-file path uses
>    `PipelineError::TypeCheck` (the outward-emission arm at obligations.rs:654), while
>    multi-file/embedded paths record an `OrchestratorError` failure (otherwise `result.errors`
>    would have no consumer). The reason is that returning `Vec<Diagnostic>` would discard
>    form-based routing—Check diagnostics must retain per-file attribution.
> 4. The first tier's criterion changed from “the declared consumer reads the obligation” to “a
>    delivery marker exists”; “marker ↔ real read” is handled by the static gate (criterion 2 in the
>    fifth subsection).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**; it was
>    replaced by **4.3.2's same-code promotion to E** (it is promoted even without a strict switch
>    because the healthy state emits nothing and is therefore safe enough, R4).
> 6. The static-gate criterion was upgraded from “field appears at least twice but has no read in a
>    third file” to the four criteria in the fifth subsection.
> 7. The two additional tiers not included in the sketch (S1 and the skip exemption) are described
>    in the second subsection.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table was actually
>    implemented as **20 test cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]`
>    skeleton → green; red-state archival record: removing the sole settlement point yields
>    `pairs: 0` and six failures). The S1/S4 rows have been synchronized with the actual files.
>
> **7. 4.3.2 implementation record (severity assignment + gate hardening)**
>
> - **Inputs for the field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation
>   kinds actually drained in the run, available per run) and the five counts (the conservation
>   identity in the second subsection); together with the audit surface `findings`, these establish
>   the fact of “whether this field has ever been consumed by any entry point.”
> - **Severity: same-code promotion to E implemented** (user ruling A on 2026-10-13; see the
>   final-state registration in the third subsection and the D-class breaking-item declaration under
>   D59). Implementation: `to_diagnostic` explicitly calls `.severity(Severity::Error)`; the
>   outward-emission arm routes through the error channel according to severity. Mutation evidence:
>   removing any one delivery marker → compilation fails in that form, and `check` exits nonzero
>   (`error [W1090]`); after restoration, the healthy state emits nothing (corpus 342/342 zero
>   changes). The locale texts in all six languages state that it is “a W-family code whose emission
>   severity is error.”
> - **Gate-hardening items (F1/F2, both implemented)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence** and validates that “the file can be located
>     (uniquely) + the line number exists + the symbol immediately following the anchor really
>     appears in the anchored file”; an anchor pointing to a nonexistent symbol causes a failure. It
>     includes negative fixtures under `tests/baselines/obligations-gate/stale-anchor/` (one each
>     for nonexistent symbol and out-of-range line). Note: the red list is currently empty, so the
>     criterion's actual landing point is **non-obligation registration** and future evidence
>     pointers (the same code path).
>   - **F2/R3 reverse containment**: the gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference”; a nonzero difference causes a
>     failure. It includes a negative fixture under `unregistered-field/` (a newly added struct
>     field that was not registered).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable at real
>   entry points (a unit whose typecheck fails does not retain its `type_result`, so the obligation
>   does not exist in the first place); this tier's semantics are pinned by synthetic `State` cases,
>   while real end-to-end coverage includes only `ConfigDisabled` (t3 T06).
>
> **8. Closing out the 4.3 secondary task (red list cleared + final E-level obligation
> diagnostics)**
>
> **8.1 Three-round implementation timeline** (each item corresponds to a code file:line; this round
> only records conclusions and does not change code)
>
> | Round                     | Conclusion                                                                                                                                                                                                 | Code anchors                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
> | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger goes live    | `State::assert_drained()` as the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                                | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                    |
> | 3.4.4 Closure             | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                                        | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                |
> | 3.4.5 Closure             | `method_overload_ir_names` cross-stage mirror deleted (in-phase candidate-name table + `overload_resolutions` remain)                                                                                      | Decision consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                      |
> | 3.4.9 Closure             | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirror for middle/LSP consumption” declaration, retain in-phase consumption; obligations 14 → 13, red list 1 → 0                | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; doc comment at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, `src/frontend/core/sema/inference/expressions.rs` seven sites (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Severity assignment | **Same-code promotion to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; outward emission routes through the error channel according to severity (first failure takes precedence) | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/severity decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                 |
>
> **8.2 Closure conclusions**
>
> - **Red list cleared**: the number of entries with `red_list: Some` in `INVENTORY`
>   (`src/driver/obligations.rs:201`) is **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The **mechanism is
>   retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the gate's criterion 4 stale
>   detection, and the `stale-red-list` fixture; the gate prints “red list: 0 entries—cleared.”
> - **Final E-level obligation diagnostics**: the **code number and W-family membership of `W1090`
>   remain unchanged**, while the emission severity is `error` (D5 final state, user ruling A on
>   2026-10-13). This **is a D-class breaking item under D59** (externally observable behavior
>   changes), registered both in the third subsection of this note and in RFC-013's “Emission
>   Channel” section; **zero emission in the healthy state ⇒ zero impact on existing corpora and
>   user programs** (corpus 342/342 “difference: empty,” zero IR snapshot drift; independently
>   reverified by t11 and re-reviewed by t12).
> - **Closed field inventory**: the `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a reason based on measured
>   consumption); gate F2 explicitly prints the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises retained**: all hardening items registered in this note have been
>   **implemented** (F1/F2/F4/F5; see the fifth, seventh, and eighth subsections); the only two
>   items not closed are the **known boundary** in 8.4 and t7 F5, which has been converted to an
>   issue, and neither is an unimplemented criterion of this mechanism.
>
> **8.3 Disposition status found by each round's acceptance/review process** (each item explicitly
> identified)
>
> | Source        | Finding                                                                                                                                                                         | Status                                                                                                                                                                                                                                                                             |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence consists of plain-text pointers and can silently become stale if a replacement consumer is removed                                                            | **Implemented**: gate F1 anchor validation (file can be located + line number exists + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`                                                             |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, so adding a field without registration silently passes                                                                              | **Implemented**: F2 prints the difference explicitly (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture under `unregistered-field/`                                                                                                 |
> | t4 R4         | W-only versus promotion to E semantics were not registered                                                                                                                      | **Implemented**: the third/seventh subsections of this note + RFC-013's “Emission Channel” + W1090 help text and error_output examples in six locales under `locales/*.json`                                                                                                       |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (a legal overload is still falsely reported as `W1005`)                                                           | **Converted to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (registration in the 4.9 row of `docs/src/dev/architecture/09-execution-wbs.md`)—outside this secondary task                                                                          |
> | t11 F7        | Under the gate's `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                               | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                       |
> | t11 F8        | Validator-side note: mutation-driven testing once mistakenly wrote to the main repository (cleared immediately with `git restore`; affected-round output invalidated and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                                       |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document's table header inconsistent with `INVENTORY`) caused the gate to fail                                  | **Informational / resolved**: the frozen commit passes consistently; “an intermediate state fails the gate when committed directly” is precisely the intended behavior of F2/F4                                                                                                    |
> | t12 F12-1     | This document gave the struct range as `16-73`, while the actual range was `16-76` (three comment lines added by t9 without mechanical reconciliation)                          | **Implemented and mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; a mistake fails immediately); related figures were grep-aligned (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                   |
> | t12 F12-2     | Settlement in the multi-file form once overwrote the first failure (the type error in `zzz.yx` was masked by W1090)                                                             | **Fixed** (captain amended the commit message to explain this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only the text of **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If an anchor promise is written in a module header in
>   the future, it must either **rely on manual review** or have F1's scanning surface
>   extended—either is acceptable, but leaving it unowned by default is not.
> - **The gate's static boundary**: set-level validation of delivery markers resides on the static
>   side (criterion 3), while removal of an individual arm's marker is covered by **runtime
>   settlement**—static + runtime double coverage is the intended design, not a gap (t11's
>   M11A/M11B/M11C mutations were verified in both directions).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId conversion
> implemented)**: two of the 13 obligations above, which were **span-keyed contracts**
> (`release_plan` / `overload_resolutions`), have been converted to **structural-identity keys**
> (`PlanId`) according to RFC-039 **D20**. This note registers the implemented form, criteria,
> classification rationale, and known boundaries; definition lines are still reconciled by gate
> **F4** (the line numbers in the two relevant rows above are current code facts).
>
> **1. Implemented form (carrier choice A: `PlanId`)**
>
> The key is a **structural identity** carried by AST nodes, rather than the positional coincidence
> of “each side computing a span”: `PlanId` (`src/util/plan_id.rs`, a monotonically increasing
> `u32`, unique within a compilation unit, with `UNPLANNED = u32::MAX` representing synthesized
> nodes outside the parser) is issued by the **sole allocator**, `ParserState::alloc_plan_id`. There
> are two carriers: `Stmt.plan_id` (statement identity → release-plan ownership and consumption) and
> `Expr::Call.plan_id` (call-site identity → overload-resolution production and consumption);
> `Expr::call` is the repository's only call-expression constructor, and synthesized calls outside
> the parser are always recorded as `UNPLANNED` (**not consumable**; see the second subsection).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), with the
>   sole consumption entry point `ReleasePlan::consume(stmt)`—**read and accounting occur in the
>   same call**; `ir_gen.rs:360` reads it, `:3103` uses it (statement identity is provided by
>   `walk_stmt`'s set/restore operations + the CFG-node-to-statement-identity table; Drop ownership
>   on scope exit is assigned to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (the table and consumption ledger are **one
>   entity**), produced at `inference/expressions.rs:4550` (written by call-point `PlanId`, with
>   dispatch forwarding it by destructuring `Expr::Call { plan_id }`), and consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) via `OverloadResolutions::get` (a
>   hit is recorded).
>
> **2. Consumption-completeness settlement (reusing the 4.3 ledger, not creating a second system)**:
> the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with each of the two tables holding one instance; when
> `generate_module_ir` returns successfully, the same point marks “a complete run” for both ledgers
> (`ir_gen.rs:1095` release plan / `:1097` overload resolutions); the three-state mapping is unique
> (`consumption_state` in `src/driver/obligations.rs`), and the sole settlement point adds one line
> for each of the two obligations—a nonempty difference produces an **E-level W1090** (same-code
> promotion to E; semantics described in the third subsection). The exemption criterion is attached
> to the **ledger's own completion marker** (not to whether `unit.ir` exists), so a Check form that
> takes the IR product does not constitute an exemption.
>
> **3. D59 classification rationale (Class C, red before green)**: before the change, the form
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> resolution was discarded, and execution fell back to the last binding in definition order); after
> the change, the same program fails at compile time with
> `error [W1090] ... produced key was not consumed`. The fix deliberately chooses **loud failure**
> rather than “put the resolved name into the call with a named argument”: the latter would lose the
> named argument and merely substitute a different form of silence (for the final semantics, see
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition, `Drop` instructions are a no-op in
> the interpreter (the wildcard arm at `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the empty
> 342-file corpus differential is empirical evidence of this point.
>
> **4. Criteria and evidence**: the D41-equivalent criterion `test_release_plan_spans_consumed`
> (settled by `PlanId`) and the new `test_overload_resolutions_consumed` (per corpus file:
> lex→parse→typecheck→ir_gen; **the difference must be empty + an evidence floor**: number of files
> that produced resolutions `≥ 1`, number of genuinely consumed entries `≥ 2`, with no allowlist);
> the nine pinned decision shapes are covered in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> type-body binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven testing: shifting a consumption key by one →
> criterion fails + real program reports `error [W1090]` + corpus gate differs; restoration returns
> it to green. Gate figures: `check-obligations.py` PASS, 342-file corpus differential empty,
> `cargo test --lib` reports 2712 passed / 0 failed (zero IR snapshot drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: the **unbound consumption
> point** at `ir_gen.rs:7766-7790` is unreachable under the current corpus and test surface (method
> declarations uniformly use `register_type_bindings` and therefore take the bound consumption
> point), so this branch's key correctness has **no red state with which it can be falsified**; the
> reason for retaining it is that it is the symmetric branch of the bound path, and deleting it
> would rewrite the “method call without `type_bindings`” form into silence. This is registered by
> independent verification (t5 F-1) and re-review (t6 F2), with **no new task opened**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (within an LSP project; results filtered to the target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Must come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers cannot freely supply an arbitrary array.** This is the key constraint preventing the
`Program` abstraction from degenerating into a “free parameter that accepts anything,” and it is
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
                // … 12 arms; not one may be omitted
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 implemented)**: four implementation
> deviations from the sketch above are semantically equivalent and are registered for reference:
>
> 1. The three-state `StageOutcome` in §4 is not returned by the stage arms; it is expressed through
>    `State` failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are unchanged,
>    eliminating per-arm boilerplate returns);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (§5's
>    “avoid mutable global state in Driver” determination covers the draft field);
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted outward—the zero-diff criterion of S2, “deliberately fix no bugs,”
>    requires this; outward emission follows with the 4.3 obligations ledger;
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the existing orchestrator call sites will
>    be relocated after the 4.2 migration, and the question will be revisited then.
> 5. In the multi-file form, ProofExecution is an independent stage that runs **after all**
>    typechecking (A1, user ruling on 2026-10-09): `compile_project` originally interleaved proof
>    with the per-file typecheck loop (file N's proof preceded file N+1's typecheck). With a single
>    failure, both are byte-for-byte identical; in the multi-failure scenario “file 1 proof
>    failure + file 2 typecheck failure,” the first reported failure changes from the proof error to
>    the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries products through channels according to ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the entry points' external
>    error contracts (PipelineError / OrchestratorError) are outside the type surface that Driver
>    can unify.
> 7. The Check channel was implemented (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry, while a clean file has an empty Vec—the
>    current `check_project` contract); the Discovery product was extended to a three-part tuple of
>    “file set + `used_by` edge set + shadowing events” stored in State (the MultiFile form has no
>    downstream consumer, so it is merely recorded and not emitted); per-file parsing on the Check
>    path dropped from 2 passes to 1 as a result of the C3 topology.
> 8. The Lsp form was implemented (4.2.3, per-file-source routing ruling, user decision on
>    2026-10-09): disk-file parse failures use Check's plan B (collect + exit the compilation
>    unit—both ends converge on degradation); edited-buffer parse failures retain the partial AST
>    and continue to typecheck (editor philosophy—an intermediate typing state should not make
>    semantic functionality disappear). The Discovery arm preserves buffer source over stale on-disk
>    content for Lsp; Typecheck under Lsp checks only the target file (other units exist only to
>    provide signatures to the registry); the within-file diagnostic-order normalization principle
>    extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior in
>    which LSP hard-aborted on parse errors in unrelated disk files and the handler silently fell
>    back to the single-file path is fixed by this step.
> 9. The Embedded form was implemented (4.2.4): the `Program` structure gains a `shared_registry`
>    field—the embedded std module is a sub-compilation, and the registry is an **input**, not a
>    product (EMBEDDED_STAGES has no Registry stage), making #94's SymbolTable sharing contract
>    explicit as a program declaration rather than something callers must remember to pass.
>    Error-path text changes from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler-internal error surface only; no test pins it). All four orchestrator entry points
>    have now moved to Driver.
> 10. Standalone check was unified (4.2.5, ruling A + IR-stage ruling, 2026-10-09): the Check form
>     faithfully carries the single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (adjacent files receive errors only, preferring omission
>     over false positives); relative `use` declarations resolve relative to the importing file's
>     directory (aligned with rustc's single-file mod behavior). The CHECK stage table gains
>     GlobalSlotAlloc + IrGeneration: the old standalone path (the full pipeline) already ran IR
>     generation, and errors such as E3019/E1014/E1015 are produced only by ir_gen (confirmed by the
>     runner gate). IrGeneration in the Check form is check-only and does not consume the IR; the
>     Script/Bin split is expressed through the None/Some value of module_key (the existing E3023
>     switch, ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces only
>     resource-limit/internal errors and has no corpus dependency). **〔Withdrawn after empirical
>     verification in 3.4.8 on 2026-10-10〕**—the claim of “zero corpus dependency” was based on the
>     evidence blind spot that the corpus has no pathological-recursion fixture; E3005 is the
>     compiler's only compile-time defense against such programs. See note #14. Incidental fix for a
>     latent defect: repeated registration harvesting by the Registry arm for embedded std units
>     would overwrite the native half already merged by with_std() (false E1043)—embedded units now
>     skip duplicate registration.
> 11. The LSP single-file fallback was unified (4.2.6): the manual
>     lex→parse→check_module_collect_all sequence in `run_diagnostics` was deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     In the SingleFile+CollectAll form, Parsing collects all parse errors and retains the partial
>     AST to continue typechecking (the editor philosophy from the 4.2.3 ruling extends here; the
>     stage is not marked failed, because otherwise topological skipping would prevent typecheck
>     from ever running); Typecheck dispatches the `check_module_collect_all` free function. The LSP
>     single-file path now adds proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and
>     IR-level errors; lexical failures now report real diagnostics (the old synthetic “E0001
>     lexical error” text disappears with the deleted sequence—real diagnostics carry exact spans,
>     with no information loss).
> 12. Line-by-line verification of `check_module` / `check_module_collect_all` (4.2.7, mandatory C5
>     item): the two entry points had already converged separately—the mod.rs layer has
>     `check_module_inner(ast, env, collect_all)` with a single Boolean, while the checker layer has
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, with pass-3 and drain modes both
>     running, and the difference carried by whether `collected_errors` is empty; five collection
>     points in statements.rs (function body/use/for/block/while). Verification uncovered and fixed
>     a duplicate-diagnostic defect: collection points placed the first error in both
>     `collected_errors` and the `Err` return channel, and pass-3's `add_error` for `Err` produced
>     the same code/span/message twice (up to three times when nested); two independent mechanisms
>     were also found: duplicate annotation-validation signatures versus whole-annotation validation
>     (E1003/E1103), and duplicate ownership-layer emission (E2014/E2018). The fix is at the
>     module-result boundary: deduplicate by (code, span, message)—diagnostics are positional facts,
>     and duplicates carry no information; duplicate copies were removed from 63 corpus entries,
>     with zero drift in the run list and exit codes (the “duplicate” `borrow_conflict_err` is an
>     artifact of the baseline triplet format lacking a column number—the two E2018 diagnostics are
>     legitimate diagnostics at different columns).
> 13. The wasm playground was implemented (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper is no longer involved. Error text is aligned byte-for-byte with the old `CompileError`
>     Display prefix (Parse error:/Type error:/Internal error:), with no visible difference in the
>     playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the existing
>     4.1 declaration, now with a real production caller).
> 14. Multi-file monomorphization and CHECK stage-table corrections (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing the mono exclusion in note #10**). Empirical evidence
>     disproves the claim that “mono only produces resource-limit noise”: pathological generic
>     recursion (`f(x)=f([x])`) causes the compiler process to overflow the stack on the multi-file
>     path without mono (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime
>     error), and the mono depth gate is the only compile-time defense; the standalone check path
>     used the full pipeline before 4.2.5 and therefore already had mono, so the exclusion ruling
>     introduced a silent coverage regression (“zero corpus dependency” only proves that the corpus
>     has no pathological fixture). The “type-erasure fallback” assumption was invalidated by the
>     RFC-033 reflection ruling—`^^List(Int)` requires the true identity of an instantiation, and
>     monomorphization is the foundation of reflection, not an optional optimization. The design has
>     four points: ① typecheck produces an instantiation request for a qualified call (`lib.f(x)`,
>     FieldAccess shape), with the generic_id using the qualified name to share a namespace with
>     merged IR (currently only bare Vars are recognized, expressions.rs:2455); ② `Stage::ALL`'s
>     topology is changed to place Linking before Monomorphization, and MULTI_FILE's stage table
>     gains Monomorphization at the end—it consumes merged_ir and aggregates all units' requests
>     (qualifying the deferred bucket's containing_fn), elevating mono from a single-module pass to
>     a whole-program pass; the single-file stage table is unchanged (no Linking, the subsequence
>     property is preserved, and IR snapshots do not drift); ③ CHECK gains Linking (pure merge—E3020
>     entry validation remains in RoleClassification to avoid duplicate reporting) and
>     Monomorphization (check-only without consumption, like the Check form of IrGeneration) stages;
>     ④ resource protection covers cross-unit mutual recursion through whole-program BFS. The
>     Monomorphizer itself is unchanged (its ModuleIR + request-set contract happens to match the
>     merged IR form).
> 15. 3.4.8 R1 (①) implemented (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the #14 design: `ExpressionInferrer` receives a module-qualified key
>     table (all three `use` arms register it—ordinary / alias / item-level alias and only for
>     SubModule-class exports), and `callee_generic_name` obtains a unified qualified name through
>     the same qualified-name concatenation used by merged IR (the concatenation implementation is
>     in `frontend::module::symbol`); the qualified-call first path's arity criterion was tightened
>     (when the parameter table cannot be resolved, it falls back to the second path and obtains
>     arguments by signature, avoiding false E3018 reports for same-named functions in different
>     modules). Implementation empirically uncovered and fixed **three asymmetric mono-side
>     defects** in the same round: (1) deletion keys (the set of generic names) and rewrite keys
>     (the containing_fn + name + span triplet) differ in granularity—a site whose arguments cannot
>     be resolved degrades to a symbol request in the deferred bucket, but if the containing
>     function is not generic, the bucket never drains, while the original item was already deleted
>     by name → dangling call (runtime E6006 confirmed by `list_ops.yx:53`); the fix restores a
>     gate, `restore_generics_with_uncovered_call_sites`, with `build_call_site_map` as the sole
>     source of truth for coverage, and retains the original item for uncovered sites. (2) `HashSet`
>     iteration order in the restoration section made function-table order nondeterministic (t2
>     re-review F1) → changed to `Vec`, sorted by name, making the product byte-for-byte
>     reproducible. (3) TailCall / MakeClosure shapes outside the rewrite surface still looked up
>     mapping keys (despite comments claiming conservatism) → `call_form_is_rewritten` is now the
>     single decision point, and every non-rewrite shape is counted as uncovered. Each of the three
>     has a pinned test (the red state before the fix empirically demonstrates discriminating
>     power). The corpus differential baseline was regenerated in the same commit with exactly one
>     line changed (a `(int64)` cast added to a `remove_at` frame). **Commit-strategy note**: when
>     typecheck collection and the mono fix were split into two commits, the intermediate tree
>     containing only the former caused `stdlib_docs` to fail because of an existing HEAD defect
>     (`list.is_empty([])`, same E6006 family)—the two halves are each the other's green-light
>     prerequisite, so they were combined into one atomic commit.
> 16. Five rulings registered on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—user-module
>     exports carry `generic_fn_type_params_snapshot()` via `Export.type_params`, with the same
>     structure as the std path (`yx_sources.rs:62/108`): the declaration name becomes the
>     authoritative source for qualified-call monomorphization, and signature reverse inference is
>     demoted to a fallback (folded into R2's scope). (2) **The HKT declaration side is live, but
>     the call side is broken** (type-argument inference E1002 / explicit type arguments E1010)
>     registered as WBS 3.5 (three third-level tasks). (3) **The deferred bucket never drains in a
>     non-generic container** is folded into acceptance sub-item ② of 3.4.8: the bucket must drain
>     or fail loudly and must not silently leave calls unspecialized. (4) **Calls to closure
>     variables by name cause runtime E6006**, registered as WBS 3.6; the semantics (i) support
>     indirect calls / (ii) reject at compile time, **and (i) has been selected**—support indirect
>     calls: the `CallDyn` mechanism already exists (the closure-expression path at
>     `ir_gen.rs:7945`); what is broken is that a `Var` callee is degraded to a by-name `Call`, even
>     though typecheck already accepts such programs. (5) **Trade-off criteria are fixed**—a
>     proposal may be rejected only on correctness and readability grounds; “large change surface /
>     introducing a new mechanism / high cost” do not constitute rejection reasons. They are
>     recorded in the second part of `coding-rules.md` and the HOWTO self-check table; trade-offs
>     requiring a user decision must follow `AGENTS.md`'s “When requesting a user ruling
>     (mandatory)” and provide all four items (situation + evidence / itemized pros and cons /
>     explicit recommendation + cost / explicit breaking-change identification).
> 17. 3.4.8 ② implemented and F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the #14 design: `Stage::ALL` was changed to place Linking before
>     Monomorphization, and MULTI_FILE's stage table gained mono; the Linking arm aggregates
>     whole-program requests **before taking each unit's IR** and qualifies `containing_fn`
>     (otherwise the deferred-bucket key differs in origin from merged IR, and the bucket never
>     drains). **① closure (D2 ruling A)**: user-module exports carry a declaration-time
>     type-parameter snapshot via `Export::type_params`, with the same structure as the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically uncovered and fixed during
>     implementation**: (a) **F1**—`qualify_names` rewrote only function bodies and never visited
>     `ir.init`; in multi-file builds, function definitions had qualified names while module-level
>     calls remained bare → runtime E6006 (**reproduces even without generics** and is unrelated to
>     generics); the fix makes init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono side's “whole-program unique suffix” heuristic and silent
>     `continue` on a miss → the heuristic was deleted and a miss now produces a **loud E3018**
>     (ambiguous silent mismatch is worse than failure to resolve). **Independent verification
>     caught and fixed a regression**: t6 found that a named import (`use lib.{pick}`) followed by a
>     bare-name call was falsely rejected with E3018 (the IR instruction name is the alias's
>     qualified name, while the request name is bare, and the aggregation side qualified only by
>     unit key) → the fix makes the aggregation side **directly call ir_gen's**
>     `import_aliases_from` and `resolve_call_target` (extracted as a public single point, with one
>     producer and one consumer), and a matrix of pinned tests covers both forms of named import.
>     **Archival note**: the blank `DefId` in the init section was empirically identified as a
>     “consistency blank” rather than a live defect (codegen's name fallback can resolve it, and all
>     four kinds of init call were tested successfully); it is registered as WBS 3.7.4 to close the
>     “function bodies use def, init uses names” semantic divergence.
> 18. The init-section DefId semantic divergence was closed (3.7.4, 2026-10-11, 38bd4b9e):
>     `assign_defs` was extracted into a single-point `resolve_static_targets` (Call/TailCall string
>     operands + MakeClosure), and function bodies and `ir.init` share both the implementation and
>     the `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)`
>     pairs changed (other-lines 0; zero residual `(def=_)` in the init sections of the full tree).
>     Translation results and execution values are item-by-item identical under both codegen DefId
>     paths and the name fallback (all four kinds of init call tested), with zero corpus-baseline
>     drift; the probe flipped under the new semantics and was strengthened (identity + exact
>     index + execution value; fallback to init coverage causes failure). **Erratum (2026-10-11)**:
>     the commit IDs above were rewritten by `git pull --rebase`—the original
>     b554497b/e60dcc87/b7ac7396 correspond in order to the rebased 375789d2/d922a551/38bd4b9e; the
>     pushed dev history uses the new IDs (the old IDs were local temporary IDs and never entered
>     the remote history).
> 19. 3.4.8 ③④ implemented and verified (2026-10-11, 8ecf56f93). (③④'s design basis is in #14; ②'s
>     implementation is in #17, which this item closes.) **Implementation**: the `CHECK_STAGES` tail
>     = `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs only pure merging
>     (aggregating requests + `link_module_irs`), while the entire entry-validation block remains in
>     `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (in memory only);
>     mono failures enter the collection channel under Check, while MultiFile retains `fail_with`;
>     `into_outcome` takes `check_diagnostics` before deciding failure (structural failures no
>     longer clear already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + the t5 cross-form matrix) and one re-review (t6) all passed; the
>     corpus differential was 340/340 line-by-line IDENTICAL, IR snapshots did not drift, and the
>     test count increased from 2643 to 2656. **An existing gap uncovered and fixed in the same
>     round**: F1 (rootless multi-file check's request names and IR definition keys have different
>     origins → silent failure to specialize; see the implementation note in the 3.4.8 row of the
>     WBS)——the initial fix was to look up precisely by the real definition key on the aggregation
>     side (`flat_defs`); **same-day erratum**: that bridge was later deleted in the regression fix
>     because the root cause was solved by decoupling the dual role of `module_key` (`script_mode`),
>     with all three origins unified as `{unit key}.{name}`. The localization and fix for this
>     regression (the CI documentation-example gate went from 258 blocks with 10 failures to 0
>     failures) are described in the implementation note in the 3.4.8 row of the WBS. **Archival
>     notes (existing, not regressions from this round)**: E3005's help text does not match its
>     resource-protection semantics; the multi-file E3005 span label is misplaced (reproducible with
>     the HEAD binary); the request-collection surface depends on the type-parameter position (the
>     same-surface issue is assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 implemented and the diagnostic surface closed (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is reachable)**:
>     interpreted calls were changed to **explicit stack-driven
>     operation**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/ `SpawnFromList` push frames and return
>     control to the outer `step_one`, with frames carrying `ReturnTarget{Host|CallerSlot|Discard}`;
>     on return, control writes back to the caller's slot, and multiple call instructions continue
>     through `pending_calls` in the frame. Baseline measurement showed each interpreted call level
>     consumed **2544 B** of native stack, with the crash threshold at `count_down(394)` (1 MiB on
>     the Windows main thread); after the fix, 20,000 levels of recursion run on a 512 KiB stack
>     thread (approximately 0 per level). The guard (`push_frame`) is therefore reachable from every
>     entry point under every `max_stack_depth` value; `ExecutorError::StackOverflow` now carries
>     the actual limit (previously `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2
>     (deep-stack echo truncation) was merged into this task by user ruling** (same file,
>     inseparable at file granularity): the echo now shows the first 16 frames + an omission count +
>     the last 8 frames, with shallow stacks unchanged frame by frame. **3.10.1 (no compile-time
>     gate for native recursion, #447)**: first locate, then change—the crash point was recursive
>     `MonoType::type_name` (**328 B/level**, determined by an SP probe + per-operation
>     critical-point calculation); the same path also has a derived `Clone` (~400 B/level), deep
>     copies in `substitute`, `type_name_hash` (164 B/level), and `collect_generic_type_refs`; after
>     changing all four to use a work list and removing one deep copy of the entire request per
>     level, pathological generic recursion with `mono.max_depth=5000` produces **E3005 (with the
>     actual limit)** instead of overflowing the stack. **3.11.1 (faithful task-boundary error
>     codes, #448)**: the task result's error side was changed from a value to a **structured** form
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), restored by
>     value across the boundary; task-internal E6001/E6003/ E6004/E6008 are verbatim identical in
>     Standard mode and Embedded messages. Two **pub API payload-shape changes** were recorded in
>     the commits with `BREAKING CHANGE`. **Commit-strategy note**: 3.9.1 and 3.11.1 are
>     **inseparable at file granularity**—`interpreter/executor/executor.rs` carries both the
>     trampoline and the error-channel producer/consumer points, and `TaskFailure` is defined in
>     `runtime/engine.rs`; staging only either side fails to compile (the pre-commit check compiles
>     the staged subset and rejected it twice), so under the same criterion as #15 they were
>     combined into **one atomic commit**. **Known boundaries and follow-ups**: Drop derivation (~75
>     B/level) ⇒ `mono.max_depth ≳ 13000` may still crash; the E4012 (constant-evaluation) gate
>     bears on native recursion (each level ~3,696 B, default 256 levels ≈ 924 KiB) and currently
>     has zero call sites and is unreachable in production; any future wiring must close the same
>     family; the Standard-mode task interpreter is constructed with the default `ExecutorConfig`,
>     so its E6004 limit is always 1024 (configuration propagation registered as WBS 3.12, user
>     ruling B: separate team); the test-runtime budget is a separate issue #449 (pathological
>     fixture 76 s, clippy for both targets, full test run on commit), outside this RFC's
>     implementation surface. **Corpus directory stratification (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134), totaling 212 fixtures, were split into 23
>     category-specific subdirectories; 88 IR snapshots were relocated along mirrored paths; the
>     corpus baseline was regenerated in the same commit (342 lines, independently recalculated and
>     compared line by line: only the `file` field and two new changes), and 41 snapshots were
>     regenerated because their file-header paths became longer; line-by-line verification found
>     **only span-offset changes** (zero differences after offset normalization).
> 21. The LSP semantic-data pipeline was unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references within a project carry
>     `resolves_to` pointing to the true definition— the old form used **two compilation passes**
>     triggered by didOpen/didChange: diagnostics went through the unified Driver pipeline, while
>     semantic data went through server.rs's manual lex→parse→single-file typecheck (with no module
>     registry, project-level `use` resolution failed, reference events were not produced, and
>     cross-file navigation/rename/find-references silently found nothing—the silent failure
>     registered as D55 is closed by this task). **Implementation**: (1) `Export` carries
>     `definition_span` (harvested from AST identifier nodes by extract_module_info, with the same
>     origin and value as typecheck's definition event's `def_span`; native std is always None), and
>     `ModuleInfo` carries `definition_file` (filled by the Registry arm from DiscoveredFile); (2)
>     when the checker drains reference events, it uses `import_def_locations` to redirect
>     cross-file imported items' `resolves_to` from the use line to the true location in the
>     definition file (std imports have no registration and continue to resolve to the use line);
>     (3) the checker's module identity changed from the unified `"<module>"` to the unit file path
>     (this value is consumed only by the semantic-data surface and has zero diagnostic
>     consumers—confirmed for all 4 consumption points of `env.module_name`); (4) `DriverOutcome`
>     gains a `semantic_db` channel—CollectAll forms (Lsp target file regardless of diagnostic
>     success + SingleFile fallback) feed into it, and the Lsp arm's tail injects top-level export
>     definitions for non-target files from the registry (`external_definitions`; even unopened
>     files can be navigated to; type/submodule exports are not injected—typecheck-side type
>     registration does not produce definition events, so both sides retain the same gap); (5) the
>     orchestrator gains `analyze_source_in_project` (`check_source_in_project` delegates to it and
>     takes only the diagnostic half); (6) on the LSP side, `analysis.rs` becomes the single
>     compilation entry point and replaces the manual pipeline (`update_semantic_db` in server.rs is
>     deleted), and World keys are unified as normalized file URIs (`uri.rs`:
>     `path_to_uri(uri_to_path(u))` is idempotently normalized, with drive-letter case and %XX
>     encoding differences normalized to the same key). **Incidental fix (D59 routing Class B—old
>     behavior did nothing silently and does not change the externally visible behavior of any
>     previously successful program)**: cross-file navigation/rename/find-references for files
>     within a project change from silently finding nothing to hitting the true definition; after a
>     lexical error, the old manual pipeline deleted semantic information, whereas now the partial
>     AST retains its coloring (an extension of the 4.2.6 editor philosophy); when the target file
>     has type errors, semantic data is no longer discarded along with `type_result`
>     (coloring/navigation do not disappear during an intermediate typing state). **Pinned tests**:
>     6 at the driver level (target-file path key/cross-file resolves_to + external-definition round
>     trip/std import fallback to use line/error-file retention/single-file fallback path key +
>     error retention), and 1 LSP end-to-end test (red before green as empirical evidence—before the
>     fix, navigation returned None), plus 4 URI-normalization tests.

**Migration approach for the ten entry functions (each function specified individually)**:

| Entry point                                                | After migration                                                                                                                                                                                                         |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to Driver                                                                                                                             |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                 |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                            |
| `check_project` (`orchestrator.rs:273-237`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                              |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter the result to the target file                                                                                                                                                        |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                               |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                                  |
| `lib::run_project`                                         | Unchanged (delegates to Driver)                                                                                                                                                                                         |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | Within a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and invoke `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                                  |

**The duplication between `check_project` and `compile_project` (approximately 40% shared / 60%
divergent)**. The shared parts are: vendor consistency verification, file discovery,
`build_registry_from` (deleted on 2026-10-26 as part of clearing zero-caller code; this historical
account is retained), `all_method_bindings`, the parsing loop, per-file checker assembly, and result
handling. The divergent parts are items 2/3/4/5/7/8/9/10/11 from the 11-item table above. **This
shared 40% skeleton is precisely Driver's value interval**—all of the divergent 60% consists of
either “a different set of stages” or “a different aggregation mode,” and these are exactly what the
two `Program` fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had non-blocking issues; continue
}
```

**Key design decision: “an upstream failure causes this stage to be skipped” is not a stage return
value.**

The reason is that skipping is determined by **topology**, not by the stage itself. If every stage
returned `Skipped` itself, the information about “why did I not run?” would be scattered across 12
stages and could not be centrally audited. Instead, `dispatch` determines this at the start of the
loop according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability addressed by
this document—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”; it
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                   | Text direction                                                                                                                      |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                            | “Stage Y was not executed because upstream stage X failed”                                                                          |
| Caused by an empty prerequisite obligation               | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; not counted in the warning total** |
| Conditions not met (e.g. `config.mono.enabled == false`) | “Stage Y was not executed because it was not enabled by configuration”                                                              |

The second category is critical: it distinguishes “normal skipping” from “abnormal skipping” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract of
`diagnostic/mod.rs:637-641` depends on this count).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                              | Current equivalent |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (`pipeline.rs:150/162/174/193`, four early returns)      | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both project
files and single files in the same process, while the CLI's `run` and `check` are two independent
invocations; placing these on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether they
return early,” but in being two different checker entry points. After unification, the same
implementation should be driven by the `Aggregation` parameter rather than retaining two functions
(**this is an additional item that must be verified, as described under “Known Limitations and
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
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and stratification of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain
where they were; `driver/obligations.rs` holds only the aggregate container and `assert_drained()`.
L3 consumers (e.g. `ir_gen` reading `release_plan`) continue to receive the concrete field types
through parameters and **must not `use crate::driver`**—consistent with the no-reverse-dependency
red line for driver below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend back on
`use crate::driver`**. The appearance of `Driver` in a `TypeChecker` import is considered a
violation and is intercepted by `scripts/ci/check-module-boundary.py`, as proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**revision 4.10.3, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone—instead, it uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven by rootless operation; this row now describes only the library paths `lib::run_file` / `Compiler::compile`)                                                                                                                                                                        |
| `yaoxiang run` (multiple files)     | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline path,” with Check / Lsp / MultiFile sharing one implementation and no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for outward emission by that arm) |
| `yaoxiang check` (multiple files)   | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| LSP (within a project)              | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Z3 not installed + single file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Z3 not installed + multiple files   | Silently skipped                         | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

**The only intentional behavioral break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This is consistent with RFC-027 §8's positioning that no specific solver is mandated,
and with the contract already declared by `backend.rs:60` (“callers should degrade
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

| File                                          | Line range                                      | Change                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                     |
| `src/lib.rs`                                  | `140-147`                                       | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                             |
| `src/lib.rs`                                  | `154-167`                                       | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                    | `141-227`                                       | Replace the `Pipeline::run` function body with `Program` construction + Driver invocation                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                    | `275-278`                                       | Move dead-code analysis out and into the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                    | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                      |
| `src/frontend/pipeline.rs`                    | `381-389`                                       | Move monomorphization out and into the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`         | `450`                                           | Change `check_source_in_project` to use Driver                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`         | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                       | Make `check_module` / `check_module_collect_all` in `typecheck_with_registry_in` parameter-driven by `Aggregation`                                                                                                                                                                                                                                                        |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                       | Delete the manual stage sequence in `run_diagnostics` and invoke Driver                                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                       | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                               |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                       | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                            |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they implicitly reach path 1 through `Compiler::compile_with_source`)                                                                                                                                                                                                       |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                         | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                    |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                             |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` fallback path                                                                                                                                                                                                                                                                                                       |
| `src/frontend/core/sema/checker.rs`           | `1244`                                          | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; delete “an upper layer failing means the lower layer does not run” (there is no short-circuit)                                                                                                                                                                                                            |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure interface; change all three consumers—`predicate.rs`/`checker.rs`/`ownership.rs`—to use the shared interface. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`), and the original wording is superseded by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (currently distributed across
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is in the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch (wasm branch
reachability determination belongs to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Disposition                                                                                                                                                  |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches; route expected diagnostic-set changes through the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that compiled successfully now produce warnings     | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                               |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set gains entries                             | Move from W to E in two steps, S2/S4                                                                                                                         |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no more crash                                             | No break                                                                                                                                                     |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not pass through proof_execution                                                                                                          |
| Migrate `TypeCheckResult` fields into `Obligations`              | **Internal refactoring**                                                   | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                              |

---

## Implementation Highlights

This document corresponds to **P3 (Fix correctness vulnerabilities)** and **P4 (Stage contracts and
unified Driver)** in RFC-039's global stage sequence. S1-S5 below are this document's internal
implementation order, and **each step's acceptance criterion cites the C2 category of the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration change: identical
**diagnostic sets** across entry points + identical corpus behavior).

### S1: Establish the Criteria (Red First)

**A prerequisite; it cannot be skipped.** The vulnerability criterion must first be written as a
failing test.

| Deliverable                                                                                                                                                    | Acceptance                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red**. If it unexpectedly passes, this document's vulnerability analysis must be reviewed again                  |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                                      |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                             | Mark it `#[ignore]`; turn it green in S4 (red-state archival record: removing the sole settlement point yields `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` Without Changing Behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| Change `pipeline.rs:141-227` to invoke Driver            | **The single-file path's diagnostic set and exit code are byte-for-byte identical**             |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr—all **zero-diff**    |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. The
acceptance criterion is zero-diff at the C1/C2 level.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original form.

### S3: Merge the Orchestrator's Four Entry Points + Fix the Vulnerabilities

| Deliverable                                                                                                                  | Acceptance (C2)                                                                       |
| ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | Normalize the four entry points' diagnostic sets according to `Aggregation`           |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                    |
| Delete the LSP manual stage sequence                                                                                         | LSP and CLI produce the same diagnostic set for the same file                         |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                | The vulnerability is fixed                                                            |
| Fix the ownership layer's empty `Unproven` match arm (originally `checker.rs:1313`)                                          | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | The compiler no longer panics when Z3 is absent                                       |

**Rollback point**: the vulnerability fixes and the structural merge must be **two separate
commits**. If the structural merge causes problems, only the structural commit can be rolled back
while retaining the vulnerability-fix commit—in that state,
`test_multifile_proof_obligation_not_dropped` remains green; the reverse (retaining the structure
while rolling back the fixes) turns red and is an unacceptable intermediate state, so it must not be
merged.

### S4: Enable the Obligations Ledger

| Deliverable                                  | Acceptance                                                                                                                                                 |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implemented form described in the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                               |
| Promote obligation diagnostics from W to E   | After the full-corpus differential, manually review every new E                                                                                            |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration option;
switching between W/E does not require structural changes.

### S5: Finish the Proof Layer and wasm Cleanup

| Deliverable                                                                                                                                                                                                                                          | Acceptance                                                                                                                                                                |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                     | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                              |
| `backend.rs` process-wide shared singleton (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout would become fixed across compilations through the process-wide cache and pollute cross-thread sharing in cargo test) | Production-path `default_solver()` call sites reach zero (tests only); cache hit rate is observable across all three consumers (counter already implemented in 89576fafd) |
| Add a warning to the `TypeChecker::ownership_inputs` criterion point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                                          | A diagnostic is produced when there is no `body_checker`                                                                                                                  |
| Reachability determination for the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                 | Record the conclusion in the “F2 Determination” subsection of `06-cleanup-inventory.md`; this document records only the determination requirement                         |

**Note**: correcting the layer order changes the diagnostic set and may expose many previously
silent `Unproven` results. **This item should proceed independently of S1-S4** and must not be mixed
into the entry-point merge PR.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time exhaustively enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                        | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is structurally identical to the opcode-table mechanism already validated in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that require human reasoning                                                                                                                         |
| **Obligation mechanism** | `Obligations` + `assert_drained()` perform **runtime** settlement, used together with the **static** gate `scripts/ci/check-obligations.py`; **the final severity is E level** (user ruling A on 2026-10-13: same code promoted to E, number unchanged; **implemented form described in the §2 revision note**: the ledger does not duplicate the field structure; the red list—0 entries after closure—retains the mechanism and does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not one bug: at least two instances of the same risk have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are within scope. Moving from W to E in two steps keeps each step's C2 criteria usable—now that the obligation inventory has been closed to 13 entries and the red list is empty, healthy execution emits nothing, so the final state directly uses E (see the seventh and eighth subsections of the §2 revision note) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer); do **not** change the production-side logic of the `Unproven` branches (currently three in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                                  | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic has been fixed,” whereas the logic of these three branches is itself correct                                                                                                                                                                                                                                                                                       |

This design additionally eliminates two classes of cracks:

- **The crack between “comment promises” and “code behavior.”** The “silent pass must not be allowed
  to return” statement in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, now enforced through `assert_drained()`.
- **The crack of “an array that someone forgot to append to.”** Currently, five functions each
  manually spell out their call sequence; after unification, there is one stage table, and a missing
  new stage produces a compile-time error.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type continuity**; they are configuration questions of the
  form “should this analysis run?”; this approach also cannot express `Aggregation`, and would
  merely move the `check_module` / `check_module_collect_all` choice from the function layer to the
  type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this eliminates
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions each manually spelling out their calls” with “an array someone forgot to
  append to,” which is precisely the cause of the current 11 inconsistencies. This repository
  already contains a counterexample of the same shape: the `ModuleDependencyGraph` /
  `affected_modules` / `ModuleCache` / `HotReloader` described under `docs/src/dev/design/check/`
  all have zero implementations.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but it
  covers only one of the obligation fields, while `Stage` remains scattered across five functions
  and the next divergence will continue to grow from there. **It must be done first** (as part of
  S1/S3), because the structural migration requires a known red test that proves the criterion is
  effective.
- **Making `proof_calls` public and adding a `debug_assert`**—`check_module` is a general entry
  point and does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` only warns when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, so detection must occur in the
  orchestration layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After a 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of refactors that centralize wiring.
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”; these are real bugs that
  simply were not being reported. This must be clearly documented in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual judgment field by field.** Some
  obligation fields, such as `module_namespaces`, may legitimately have no consumer and should not
  report an error. S4 must review every field rather than applying a blanket rule.
- **The `Program` abstraction may be premature.** If the stage sets of certain entry points remain
  unstable for a long time, `stages()` may degenerate into a free parameter that accepts a different
  array on every call, and the contract constraint will fail to hold. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can come only from the six predefined
  combinations.**
- **Criterion dependency**: S1/S3's vulnerability-fix criteria depend on the vulnerability tests in
  the [equivalence-criteria document](07-equivalence-oracle.md) turning red first. **If that
  document has not first established the multi-file corpus layer, S1 cannot be accepted**—because
  all 293 existing corpus files use the single-file path and provide zero coverage for this class of
  defect. This document does not cover the IR validator (`verify_loose`); that prerequisite work is
  outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the `termination` and `ownership` order is the reverse of what was
  declared). This is a diagnostic-set change risk and must proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undecided.** `backend.rs:13-19`
  states that `Solver` is `Send` but not `Sync`, and `Z3Backend`'s cache uses `RefCell`; after
  changing it to a singleton shared across compilation units, it must be confirmed that every access
  path goes through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependency defects** (such as the actual order
  dependency between termination and ownership). It should be considered only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been ruled on.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **This document retains no unresolved items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` migration (implementation steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three criterion layers,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Diagnostic Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Reference code table for the gate at `build.rs:19-55`
- `src/frontend/core/sema/checker/refinement.rs` — Explicit declaration that “silent pass must not
  be allowed to return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — Layer-order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — Explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — Contract declaration that “callers should
  degrade conservatively” (contradicts `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-98` — 15 obligation fields that downstream code must consume
  (after 3.4.4/3.4.5/3.4.9 closure)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — The unit test's own
  admission that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus's own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (used by `run_code` to compile) — the playground's
  fourth compilation caller (single-file path, so it executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Build-time diagnostic-code gate, an example of this project's enforcement
  mechanisms
