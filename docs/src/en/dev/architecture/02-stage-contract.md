# Compilation-Stage Contracts and the Obligation Ledger

> **Companion design document.** This document is a companion to
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md).
> The four-layer model, acceptance-criterion tiers, and execution-stage order are covered in the
> main RFC-039 text; the role of each companion document is listed in the
> [index for this directory](index.md).

## Positioning and Scope

This document is the implementation blueprint for RFC-039’s **L1 orchestration layer**. It addresses
one problem: **“Which compilation stages ran this time, which did not, and why” is currently
distributed across six paths and ten entry points, each wired manually, with no single place able to
answer it.**

The current state can be summarized in one sentence:

> **Invariants declared by the code are being violated by the architecture, not by the logic.**

The comment in `revalidate_refined` inside `checker/refinement.rs` originally read as follows
(formerly `checker.rs:5165`; relocated by P5 5.2.1): “Unproven → compilation error, no fallback, no
silent pass……silent pass must not be allowed to return.” However, this invariant holds only on the
**single-file path**: proof-function constraints such as `x: Sorted(3)` silently pass on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed,
and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustively enumerable, with no run-time registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, and the specific migration of each of the ten
  entry-point functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- Every affected file and line number, compatibility impact, and implementation order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-bounce gates → `01-routing.md`
- Equivalence criteria (the C1-C6 tiers and three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- The global P1-P10 execution order and G1-G10 acceptance gates → main RFC-039 text

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should be performed.
This document defines L1’s **concrete form**, its **per-file change list**, and its **internal
implementation stages** (corresponding to P3 “Fix correctness defects” and P4 “Stage contracts and a
unified Driver” in RFC-039’s global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

The equivalence criteria are executed under **C2 (orchestration changes)** from the
[equivalence-criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> Every item in this section is a **verified fact**, accompanied by a file path and line number.
> Line numbers are based on `9e02e4db`.

### Stage boundaries are the only structural defect where “one mistake silences the entire repository”

This differs in nature from the other two defect classes (module boundaries and test wiring):

| Defect                  | Typical manifestation                    | Is there a signal?       |
| ----------------------- | ---------------------------------------- | ------------------------ |
| Lexical/syntactic error | Source code is wrong                     | Yes, with a diagnostic   |
| Type mismatch           | Type is wrong                            | Yes, with a diagnostic   |
| Missing stage wiring    | A stage is never called                  | **No signal whatsoever** |
| Unconsumed obligation   | A field is populated but nobody reads it | **No signal whatsoever** |

The last two share a defining characteristic: **failure does not produce an error**. They therefore
cannot be solved by “writing the code more carefully” or “reviewing more strictly”—code review can
show what was written, but not **what was not written**. This is precisely the root cause identified
by RFC-039: “This project treats ‘design’ as a documentation convention rather than an executable
constraint.”

### Strong enforcement mechanisms already exist in this repository

The same repository already has mature enforcement mechanisms; they simply have not been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes and 8 W codes) are subject to
  a **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`. Every entry is
  compared against the RFC-013 code table, and any discrepancy directly triggers `panic!` and
  rejects the build.
- `src/package/` (**76 files / 13,012 lines**, of which 6,227 lines—47.9%—are under `tests/`) has
  high test density, and every module’s test subtree is explicitly wired. This is the repository’s
  most complete test-wiring area and can serve as a formal reference for a stage-layer gate.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry-point functions

| #   | Path                       | Entry-point function                                  | Location                                         | Compilation path used                                                              |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages called directly                                                        |
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

### Eleven stage-coverage inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point “does not do this”; it means that “no code in this entry point does this.”** That is precisely
the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                                | `compile_project`                                              | `check_project`                                                       | `check_source_in_project` (LSP)    | `compile_embedded_module`                   |
| --- | -------------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------- | ------------------------------------------- |
| 1   | **proof_execution**                                      | **Present** `187-203`                                   | **Absent**                                                     | **Absent**                                                            | **Absent**                         | **Absent**                                  |
| 2   | Dead-code analysis                                       | Present `275-278` (gated by `config.dead_code.enabled`) | **Absent**                                                     | Present `356-377` (role-aware, with no configuration gate)            | **Absent**                         | **Absent**                                  |
| 3   | W1006 local-module shadowing                             | **Absent**                                              | **Absent**                                                     | Present `336-348`                                                     | **Absent**                         | **Absent**                                  |
| 4   | W1003 unused imports                                     | Present (`277` collects `type_result.warnings`)         | **Collected but never emitted**                                | Present `350`                                                         | **Absent**                         | **Absent**                                  |
| 5   | W1001/W1002 dead-code family                             | Present (same as 2)                                     | **Absent**                                                     | Present (same as 2)                                                   | **Absent**                         | **Absent**                                  |
| 6   | **Monomorphization**                                     | Present `381-389` (gated by `config.mono.enabled`)      | **Absent**                                                     | Not applicable                                                        | Not applicable                     | **Absent**                                  |
| 7   | typecheck branch                                         | `check_module`                                          | `check_module` (`124`), **returns on the first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting everything | `check_module_collect_all` (`493`) | `check_module` (`1386`)                     |
| 8   | File discovery                                           | Not applicable                                          | `discover` (`101`, discards `used_by`/`shadow_events`)         | `discover_with_used` (`275`)                                          | `discover` (`454`)                 | Not applicable                              |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **Absent**                                              | **Absent**                                                     | Present (`321-328`)                                                   | **Absent**                         | **Absent**                                  |
| 10  | Global-slot allocation                                   | Not applicable                                          | `allocate_global_slots` (`147`)                                | **Absent**                                                            | **Absent**                         | **Absent**                                  |
| 11  | IR generation + qualified-name rewriting + linking       | Present (`205`)                                         | Present (`152-236`)                                            | **Absent**                                                            | **Absent**                         | Present (independent ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. The differences in the current
> state after P3 are as follows:
>
> - **Row 1 has been fixed**: all five entry points share the same proof-execution implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entry points), so the sole-consumer
>   defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and the dead-code-family warnings, and the W-code sets of four paths
>   (single-file run / multi-file run / single-file check / multi-file check) have been
>   normalized—the single source of the criterion is the shared implementation on the
>   `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to single file) → assigned to WBS 3.4.8 (prerequisite
>   4.1.3; potential risk not proven to be a defect).
> - **Row 7**, the two `check_module` / `check_module_collect_all` entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “decision #434” previously had no documentation entry—now recorded as
>   RFC-039 **D57**.
> - **Embedded-std asymmetry (new fact outside the table, added on 2026-10-09):** The single-file
>   path unconditionally injects std.list through `merge_embedded_std_ir` (required for desugaring
>   for-loops, a hard switch under #117), whereas multi-file `discover` previously recognized only
>   explicit `use` statements—as a result, for-loops in projects compiled successfully but failed at
>   run time with E6006 (proven by a probe). This has been fixed and WBS 4.10.2 closed. The related
>   “multi-file path lacks one single-file step” parse hard-abort quirk was registered as WBS
>   4.10.1—**and has also been fixed** (immediately after 4.2.2: the Check path was downgraded to
>   per-file collection; malformed files exit the compilation unit and do not enter the registry,
>   with importers reporting E5001). This audit method examines “stage coverage and field
>   consumption”; this issue is a difference in compilation-unit membership and therefore fell
>   through this dimension.
>
> - **Embedded-std registry-surface coverage (construction finding in 4.2.5, fixed):** The Registry
>   arm repeatedly harvests registrations for embedded std units with `extract_module_info`,
>   overwriting the native half registered through `with_std()` under the “native + yx surface
>   merge” model (the `result.is_err` etc. were lost → std.test falsely reported E1043). This audit
>   exposed a third dimension that escaped detection: merge semantics for “two registration sources
>   under the same module key.” The single-file path forms its registry only once through
>   `with_std()`, whereas the multi-file/Check Registry arm inserts each unit separately to achieve
>   coverage.

**Two points require precise wording; otherwise, the conclusion would be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
  (`run_file` → `Pipeline::run`), it does. The reason lies in `compile_project`: it stores the
  complete `TypeCheckResult` (including `warnings`) in `type_results`, but the function has **no
  read site for `result.warnings` anywhere**. The only downstream use of `result` is
  `generate_ir_with_context`.
- **The entry-point `main` criterion in row 11 also has a different source**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, determining only whether a manifest exists). The
  two functions give different answers to “which files must define `main`.”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is a single producer of obligations.**
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

The semantics are: “The predicate arguments of this refinement constraint are compile-time literals,
so the proof function must actually be executed before the constraint can be decided.”

**Step 2 — The consumer is inside the checker, but the checker only produces and never consumes.**
There are **three isomorphic branches** in `src/frontend/core/sema/checker.rs` that handle
`ProofResult::Unproven`:

| Branch                                                                                    | Location                                                                                                                     | Behavior                |
| ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Formal-parameter refinement validation (`revalidate_refined`, formerly `checker.rs:5164`) | `if calls.is_empty()` → emits a hard error (formerly `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (formerly `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, formerly `checker.rs:5306`)  | Emits a hard error (formerly `5308-5316`); `ctx.proof_calls.extend(calls)` (formerly `5318`)                                 | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, formerly `checker.rs:5420`)        | Emits a hard error (formerly `5442-5446`); `ctx.proof_calls.extend(calls)` (formerly `5448`)                                 | **Emits no diagnostic** |

The comment inside `revalidate_refined` in `checker/refinement.rs` originally read as follows
(formerly `checker.rs:5165-5169`):

> `RFC-027 §4/§9: Unproven → compile-time error, no fallback, no silent pass.` ……
> `This branch is therefore a real line of defense rather than a forward-looking placeholder: silent pass must not be allowed to return.`

In other words, the author knew that “not producing a diagnostic” was a defect and explicitly
classified it as a **known intermediate state with nowhere to go**, on the design assumption that
“someone would eventually read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.** `check_module_impl` declares a local
`proof_calls` and collects into it (formerly `checker.rs:1234-1235`), then writes it into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registration of proof-function obligations`
(formerly `:1439`). The field is defined at `types.rs:29`.

**Step 4 — The entire repository has only one read site.** In production code,
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

(The other occurrences of the `proof_calls` identifier fall into three categories, none of which
consumes this field: three producer-side collections in `checker.rs` and `checker/refinement.rs`
(formerly `:1234/4564/5494`); `verdict.rs:61`, which is a same-named field of `ProofResult`; and
`tests/rfc027_*.rs`, which read `ProofResult`.)

**Step 5 — All four orchestrator entry points bypass that layer.** The four entry points in
`src/frontend/module/orchestrator.rs`—`compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)——**do not call
`pipeline.rs`**. They call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discarded path.** `compile_embedded_module` (`1374`, with its `check_module` call at `1386`) is
responsible for compiling embedded std. This means that **the proof obligations of the embedded std
modules themselves are not executed either**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostic** on the multi-file, check, or LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent discard point within the layers.**
When `check_module_impl` processes ownership-layer results (formerly `checker.rs:1303-1314`):

```rust
// Former checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer’s `Unproven` result is swallowed by an empty match arm, with no diagnostic and
no accounting.** This directly contradicts the `5164` branch’s promise that “silent pass must not be
allowed to return,” and it is **independent of the orchestrator problem**—even if the entry-point
layer is completely fixed, this point will still remain silent. (Timing: it should be handled in the
same batch as the obligation ledger because it is the same class of problem.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

The three RFC-027 unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they all go through `check_source` →
`checker.check_module(&module)` (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), which means they **stop immediately before `pipeline.rs`**.

The test files’ own doc comments already state this. `rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so cases that violate refinements are rejected at the .yx layer.`

**This is exactly the shape of the problem: the tests verify that “the obligation has been
populated,” whereas the bug is that “the consumer never reads it.”** A test that exercises only the
producer and not the consumer is inherently immune to this class of defect.

### Stronger finding: the main corpus for the equivalence criteria exercises only the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` files in `tests/yaoxiang/` as the **third-layer criterion** and the primary
acceptance method for C2-stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero matches across the entire
  directory).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this. The file is annotated with `// expect: compile-error E4018` (line 15), and its
header comment on line 7 says “Status: ❌ should be rejected at compile time”—**it passes precisely
because it takes the only path that executes the proof function**.

**Conclusion: the third-layer criterion needs an additional multi-file corpus layer; otherwise, it
cannot serve as the acceptance tool for this document.** See implementation point S1.

### Obligation fields: fourteen “production implies contract” fields with zero enforcement

`TypeCheckResult` in `src/frontend/core/sema/types.rs:16-89` contains **14 fields** whose doc
comments explicitly state “produced by stage X → consumed by stage Y.” In other words, they are
inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention:** the definition line is the field’s **declaration line** in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of its doc comment—a field’s comment block may
> span multiple lines, and only the declaration line provides a unique anchor. The **struct range**
> in the sentence above this table (`src/frontend/core/sema/types.rs:16-89`) moves with the struct’s
> line numbers and is mechanically reconciled by gate **F5** (an incorrect value turns the gate
> red); definition lines are reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by `PlanId`**:
> read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-completion marker at `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> at `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation | | `record_models` | `79` | typecheck (one pass at declaration
> time) → IR generation (quantifier propagation; from B.2 onward, the `TypeDeclField.quantity` of
> witness fields and the population of constructor arguments/default values both read from this
> table) |

**Reclassification record (3.4.9, user decision A on 2026-10-13):** The original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design,
it is consumed within a phase by typecheck and has zero cross-stage reads, so it is not an
obligation:

- **In-phase consumption sites (live):** `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), and the read-only accessor
  at `checker.rs:214-217`, with `checker.rs:974` injecting it into `body_checker`.
- **Zero cross-stage reads (direct proof):** `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger’s own registration and documentation remain
  (removed in this round).
- **Disposition:** delete the doc comment’s **claim** that it is “mirrored for consumption by the
  middle layer/LSP”; retain the field and its in-phase consumption. The field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP genuinely needs the
  table in the future, register it anew under 4.6.x; **do not create a consumer in this round**.

**All these fields are passed across layers using `Span` or name tables.** The equivalence-criteria
document already points out that mismatches in span-keyed contracts “fail silently, without any
error” (`ReleasePlan` is a typical example: any mismatch in how the two sides calculate spans →
`Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module lies on every entry point’s mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator’s four entry points do not pass through the
  `pipeline.rs` layer at all.

**Verified fact:** the repository currently has **no mechanism whatsoever** that guarantees these 14
fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride with IR
generation.

### Non-obligation field registry: channel, in-phase, and in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (9 entries):** Every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§ Obligation Fields”
table above:

| Field                     | Why it is not a cross-stage obligation (key point)                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (0 hits in `src/middle`)                                                                                                                                                                                                                                                                                                          |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not “production implies contract”)                                                                                                                                                                                                                                                                                    |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its doc comment does not declare a cross-stage contract ⇒ not brought under management                                                                                                                                                                                                                            |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                                                                                                                                                                                                                                 |
| `semantic_db`             | Consumed by LSP (from 4.6.1 onward via the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`); not a compilation-stage obligation                                                                                                                                                                                                                         |
| `trait_table`             | Has cross-stage reads `ir_gen.rs:4336/:6867`; no contract declared in its doc comment                                                                                                                                                                                                                                                                                |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                                                                                                                                                                                                                                          |
| `interface_impl_registry` | In-tree fact (3.4.9 user decision A; see the reclassification record in “§ Obligation Fields”)                                                                                                                                                                                                                                                                       |
| `method_def_ir_names`     | Cross-stage mapping (typecheck produces mixed-compilation names by the method declaration’s `PlanId` → IR generation reads only the name, WBS 7c.3); a miss fails loudly with E3005, with no silent surface; consumption granularity is per declaration rather than non-empty per unit, so the ledger’s “a unit produces it and must deliver it” rule does not apply |

**Why this is not “wiring up consumers”**: creating downstream consumers for these fields would
invent spurious obligations and disguise fields that “do not require cross-stage consumption by
design” as contract defects. The correct closure mechanism is **reverse containment**—the
`TypeCheckResult` field set = obligation inventory ∪ non-obligation registry (hard-enforced by
criterion 1 of `scripts/ci/check-obligations.py`: adding a field without registration, deleting a
field without synchronization, a non-obligation registration without a reason, or overlap between
the two tables all immediately turn the gate red).

### Four additional contract defects in the proof layer

The following all come from analysis of the proof layer. They are independent of the entry-point
divergence problem, but all belong to the class “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation, with no enforcing mechanism; the `equivalence`
layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (anchored by function name/call site, with line numbers taken from
the current tree):

| Actual order | Call site                                                                                                                                                                                                                             | Corresponding layer |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` carries its own E2030 criterion; `judge_refined_obligation` is the sole decision unit shared by call-site obligations and return positions) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                       | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                          | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not in `layers/`)                                                                                                                                                                     | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                                                                                                | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module’s own header comment and do not correspond to the call order. The README has no “number
→ call timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and has
nothing to do with `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds termination errors one by one and then continues downward; “an
upper-layer failure prevents lower layers from running” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** The hard-failure panic was eliminated by P3’s 3.3.1 (the SOLVER slot
> was made optional, and absence is conservatively downgraded to `SMTResult::Unknown`); “silent
> skipping (no injection)” was addressed by 3.4.2 with the W1081 signal. Unifying the three forms
> (singletonization) is to be executed according to **RFC-039 D58**, at the actual fix site
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> interface). The wording in the change list for document 02, `default_solver() → &'static`, is
> superseded by D58 (a bare `&'static` reference is infeasible: `dyn Solver` is not `Sync`).

| Consumption site                                                                                         | Acquisition strategy                                     | When the solver is unavailable                               |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                     | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected at construction through `check_module_impl`, formerly `checker.rs:1265-1268`) | Ownership injection via `with_solver_owned`              | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                              | **On every call**, `default_solver()`                    | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver’s capability’.” By contrast, the contract documentation at `backend.rs:60` says: “`None`:
the backend is unavailable……**the caller should degrade conservatively**.” **Two philosophies
coexist in the same codebase, and the hard-failure path panics instead of returning an error.**

**(c) Each back-edge decision creates a new Z3 context, making the cache effectively useless.**
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

**This is a factory function, not a singleton**—every call executes `Z3Backend::new()`, creating a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the doc comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache never hits in this calling mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. It is the same
backend with two lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match reality.** The two `body_checker`
fallbacks in `check_module_impl` (formerly `checker.rs:1283-1287` and `1289-1293`):

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, ownership checking receives an **empty type ledger and empty call
table** and continues running as usual—ownership analysis degenerates into “nothing conflicts,”
**with no warning whatsoever.** (Recommended handling: record a warning-level diagnostic, or at
least leave a trace in the obligation ledger.)

The comment in `check_module_impl` says that termination checking runs “after type checking and
before constraint solving.” In reality, `self.env.solver().solve()` runs after it (formerly
`checker.rs:1320`; termination formerly at `:1256`, ownership formerly at `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm status: carried by a shim crate, with the branches in 29 files still live

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
an independent shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate contains only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                        |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                        |
| The shim depends on the main crate (`rlib`) as a library  | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                           |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI contains four wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten      | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL; degrades to a warning if missing)                                                                                                                                                                |
| **29 files** contain the literal `wasm32`                 | **22** contain actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (the list and item-by-item assessment are in the “F2 Assessment” section of `06-cleanup-inventory.md`)                                                                                 |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                      |
| `src/lib.rs` has 11 occurrences                           | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                            |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call on the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` as a whole
(all three require `std::fs`)—while the shim follows a different path.

**However, this brings us directly to a fact relevant to this document:** the playground entry point
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

In other words, **the existence of `wasm/` turns “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** When the unified Driver is introduced, this
path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the
same single-file stage table as `SingleFile`, `:135`); the implementation record is in the **4.2.8**
row of `09-execution-wbs.md` (`wasm` now uses `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 Assessment” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground run time but load-bearing at
compile time, while the six `not(wasm32)` arms are load-bearing across the three native stage
tables; there are no low-risk items that can be deleted directly. The RFC-039 open question “build a
wasm target, or delete these branches” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, with no run-time registration)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum; run-time registration is
forbidden.** The reason is that Rust’s exhaustive `match` can enforce at compile time that “a new
stage must be handled by the orchestration layer”—exactly the mechanism already proven effective by
the opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency verification          Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexing + parsing               PerModule
    Registry,           // Module registry construction            Project
    RoleClassification, // Role classification (Script/Bin/…)   Project
    Typecheck,          // Type checking (including embedded proof layer)   PerModule
    DeadCodeAnalysis,   // Dead-code family analysis              Project
    ProofExecution,     // Compile-time proof-function execution         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When a variant is added, this array and the exhaustive match in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`:** `PerModule` (run once per compilation unit) and `Project` (run once per
project) make “which stages must run per module and which must run once per project” a fact encoded
in the type system. Stages with `Project` scope are structurally guaranteed to run only once in
`dispatch`—this eliminates the need for human reasoning about questions such as “should
`allocate_global_slots` be called once per file?” in the current `orchestrator.rs`.

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07):** Two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes IR
>    artifacts (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the draft’s original order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to the `Stage::ALL` topological order, with dead-code
>    analysis **before** proof: the current `check_project` executes proof before dead-code
>    analysis, but the two have no data dependency, their diagnostic sets are identical (C2 set
>    semantics), and only intra-file diagnostic order is normalized. The single-file path (dead-code
>    analysis embedded in typecheck, before proof) already conforms to the ALL order, so
>    byte-for-byte acceptance is unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    artifacts—the draft’s original order would force the Registry arm to “hide parse,” making the
>    stage table lie about the data flow; it also reduced multi-file parsing from two passes to one.
>    The current state is also faithfully recorded: multi-file parse errors used to be a hard abort
>    (the `?` propagation from `build_registry_from`, with the tension against CollectAll semantics
>    registered as WBS 4.10.1; `build_registry_from` was deleted on 2026-10-26 together with its
>    zero callers, so the historical account is retained here)—**4.10.1 has been fixed** (user
>    decision on 2026-10-09: Rust-style collection semantics + plan B): on the Check path, parse
>    failures are downgraded to per-file diagnostic collection, and malformed files exit the
>    compilation unit (they do not enter the registry; importers report E5001); the MultiFile path
>    retains hard abort (under FailFast, a malformed file cannot produce IR, which is correct
>    semantics, and this long-term policy remains in force).
> 4. The Check form’s implementation (4.2.2, user decision on 2026-10-09) added two registrations:
>    a. Data-dependency edges 14→16: `RoleClassification` consumes Discovery’s `used_by` edge set,
>    and `DeadCodeAnalysis` consumes Discovery’s W1006 shadowing events—the draft omitted these
>    (these artifacts were produced by `discover_with_used` in the orchestrator era and discarded by
>    the `discover` wrapper, so they were lost before being represented in the table); b. Intra-file
>    diagnostic-order normalization was extended to the full form of revision note #2: E3020 entry
>    validation moved into the RoleClassification arm (Check has no Linking stage, so the arm
>    holding `surfaces` and the AST is responsible for validation), and intra-file diagnostic order
>    = stage topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic
>    set is unchanged (C2 set semantics), and only the order of stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective:** turn “a field was produced but nobody consumed it” from an undetectable
condition into a fact assertable at compile time or test time. RFC-039 already lists this as “the
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
    // …the remaining span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces one W-level diagnostic (by default) or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers):**

| Situation                                        | Criterion                                                   | Handling                                                                                |
| ------------------------------------------------ | ----------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| The obligation was read by its declared consumer | The field in `Obligations` was `take()`n or marked consumed | Pass                                                                                    |
| The obligation is non-empty and has no consumer  | The field is non-empty and not consumed                     | **Produce a diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| The obligation is empty                          | The field is empty                                          | Pass (no consumption required)                                                          |

**Why start with W rather than E:** fixing obligation discard will **change the diagnostic set**.
The C2 criterion requires that “the diagnostic sets of all entry points be identical.” If this were
promoted directly to E level, `yaoxiang check` would suddenly produce a large number of previously
silent `Unproven` diagnostics. Splitting the work into two steps—W-level observation first, E-level
promotion second—ensures that every step has usable acceptance criteria. See implementation point
S4.

**There is exactly one call site for `assert_drained()`:** `Driver::run`, at the end of the stage
table and before producing `CompilationResult`.

**Companion static gate:** RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appearing at least twice but with no third file reading it → failure). `assert_drained()` is the
**run-time** gate, while the script is the **static** gate. The two complement each other, and both
are required.

> **Revision note (P4 implementation, 2026-10-13, implementation of 4.3.1):** This note records the
> **implemented form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, item by item against code file:line. Implementation commit `1941cc47a` (verification tree
> `99079bc52`). It is also the **third** of the three places where the “no silence” red list is
> enforced (the other two are the audit surface and the static gate).
>
> **1. Implementation artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: a 13-entry obligation inventory `INVENTORY` (:201, field
>   name/consumer stage/red list), the **non-obligation field registry `NON_OBLIGATION_FIELDS`
>   (:175, 9 entries with reasons)**, the delivery-marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (:577), external emission `emit_obligation_diagnostics` (:654),
>   non-consumer gating `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped`
>   (:549), the IrGeneration delivery set `IR_GENERATION_DELIVERED` (:285), and the W1090 code
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **The sole settlement point:** at the end of the `Driver::run` stage table, before
>   `into_outcome()` produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path for `Aggregation::FailFast` settles as well (the
>   ledger must balance).
> - **Seven delivery points** (one per arm; removing any one causes the run-time W1090 to fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—all four use the
>   same delivery interface: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single file reads the
>   request table directly) / `:1221` (MultiFile·Check: aggregation in the Linking arm,
>   `driver/helpers.rs:295` reads each unit’s `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, with the reason written above the entry at :26-31—the settlement
>   granularity is “unit × obligation,” with no source location that the user can point to, so no
>   dummy span is inserted); text for all six languages is in `locales/*.json`. **Emission level:**
>   from 4.3.2 onward it is **error** (the same code is promoted to E; see the third
>   section)—determined by the explicit `.severity(Severity::Error)` in `to_diagnostic`
>   (obligations.rs:444); the code number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI gains a new `obligations` job
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above          | Implemented criterion                                                                                        | Code                   |
> | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ | ---------------------- |
> | The obligation was read by its declared consumer | **A delivery marker exists** (markers ↔ real reads are reconciled by the static gate; see the fifth section) | obligations.rs:618-623 |
> | The obligation is non-empty and has no consumer  | Non-empty + the consumer stage has run + no marker → W1090 (**error level**, see the third section)          | obligations.rs:624-630 |
> | The obligation is empty                          | `Obligation::is_produced` checks each field for emptiness → pass                                             | obligations.rs:587-591 |
>
> Two **additional tiers** (user decision on 2026-10-13; not present in the sketch):
>
> 1. **S1**: The obligation’s consumer stage is not present in this `ProgramKind`’s stage table →
>    **not treated as missing** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: The consumer stage was skipped because of `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not treated as missing (obligations.rs:598-602). `NoObligations` does
>    **not** grant an exemption—an obligation that is non-empty while the stage declares that it has
>    no obligation precisely means that the ledger does not balance and must be reported.
>
> **3. External-emission policy (decision “C”) and the measured figures behind it**
>
> **4.3.1 (observation period):** This mechanism externally emits only **W-level** diagnostics
> (W1090); there is **no strict switch**. **4.3.2 (final state, user decision A on 2026-10-13):**
> **The same code is promoted to E**—the code number remains W1090, but the emission level is raised
> to `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (one fact with two codes would create parallel representations). **The promotion
> changes only the emission level, not the emission conditions.** **This is a D-class breaking item
> under D59 (it changes externally observable behavior), faithfully recorded**: healthy programs
> emit nothing (no diagnostic when all obligations have delivery markers), so existing corpora and
> user programs are unaffected (342/342 “difference: empty”); but when a run actually encounters an
> undelivered obligation, that diagnostic **counts toward `error_count`, is rendered as
> `error[W1090]`, and causes a nonzero exit code**—unlike the previous behavior in which “W-level
> diagnostics did not block compilation.” The convention that code family and emission level are
> decoupled has been registered in RFC-013’s “Emission Channels” section (a code prefix determines
> only numbering ownership; severity is determined explicitly at the construction site). Mechanism
> facts (measured): severity comes from explicit `.severity()`; otherwise, “a code beginning with
> `W` → Warning” (`codes/builder.rs:186-203`), and the registry’s `ErrorCategory` does **not**
> participate in severity; the exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`), so **W1090
> follows `error_count` because of its explicit error severity (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain decision C on
> 2026-10-13; the current red list is already zero; see the fourth section), for a reproducible
> reason:
>
> - Decision A (externally emit red-list fields as W) measured cost: `interface_impl_registry` is
>   populated from typecheck’s prebuilt table, is **always non-empty**, and therefore emits a
>   **constant signal** with no discriminating power. It would break three existing external
>   contracts—“clean programs emit no diagnostics / LSP emits no diagnostics /
>   `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing cases turning
>   red**, of which **8 were on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs`
>   ×5, `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4
>   were under `src/driver/tests/`). The corpus baseline changed for 24/342 fixtures, and the only
>   feasible change would be “filter W1090 out of zero-diagnostic assertions,” which weakens an
>   existing criterion.
> - Decision B (downgrade to Info) would still break 9/12 list-style assertions, and Info is not
>   “emitted according to the decision level.”
> - Decision C measured result: **zero changes to existing cases and zero changes to the corpus
>   baseline** (342/342 “difference: empty, behavior equivalent”). Run-time W1090 becomes a
>   **discriminating regression detector**—it remains silent throughout the healthy tree, and
>   removing any delivery marker makes it fire (all three delivery-point types were tested by
>   marker-removal mutation; the CLI `check` emitted two `warning [W1090]` diagnostics when the
>   marker was removed from the Check arm).
>
> The red list’s “no silence” is enforced in three places: ① the
> `DriverOutcome.obligations.findings` audit surface (emitted on every run, including WBS IDs and
> evidence pointers); ② `check-obligations.py` prints the red list on every run (currently empty) +
> performs stale detection; ③ this note.
>
> **4. Red-list status (closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—all three closure tasks were completed on 2026-10-13:
>
> Note: The line anchors below record the file state **as of 2026-10-22**. P5 (2026-10-26) split the
> checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting the
> line numbers in `checker.rs`; the anchors below no longer correspond to the current file and are
> retained only as historical citations.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order from `sum_type_variants`); the call site at
>   `:7496` replaces the entire Call with `CreateVariant`. The function’s doc explicitly states that
>   “span keys are unreliable across systems (the two ends interpret a Call node’s span
>   differently), so detection is performed in place”—**wiring up a consumer would reintroduce the
>   already-fixed span-mismatch defect**. Deletion surface: the `types.rs` field; collection in
>   `checker.rs:1489-1493/:1565`; the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`; and the `VariantCtorCall` structure in `environment.rs`;
>   `infer_variant_ctor_call`’s type inference and all diagnostics **remain unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550`, decision point; the sole hit writes to `overload_resolutions` at
>   `:4550`); what was deleted is the **zero-reader mirror** in `checker.rs:1558` that was poured
>   into `TypeCheckResult`. Naming facts remain singly represented: the candidate-name table
>   (in-phase) + span-keyed decisions (cross-stage, `ir_gen.rs:380` constructs, `:7632/:7705`
>   consumes); the definition-side mixed-compilation name is computed independently by
>   `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user decision A, detailed
>   in this document’s § Obligation Fields table). What was deleted is the **claim** in the doc
>   comment that it was “mirrored for consumption by the middle layer/LSP” (types.rs:47-48); the
>   field and its in-phase consumption are retained (`operator_interfaces.rs:623` operator lookup;
>   seven constraint-solving lookups in `inference/expressions.rs`; `checker.rs:214-217` read-only
>   accessor); direct proof of zero cross-stage reads:
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the table’s contents are an environment mirror populated from typecheck’s prebuilt table (always
>   non-empty); if LSP genuinely needs it, follow 4.6.x and register it anew; creating a consumer
>   for a field that “does not require cross-stage consumption by design” would introduce a false
>   obligation into the ledger.
>
> After the closure, **field enumeration is closed by “obligation inventory ∪ non-obligation field
> registry (`NON_OBLIGATION_FIELDS`, with reasons)”**—the reverse-containment endpoint of gate
> criterion 1 protects this end to end (adding a field without registration / deleting a field
> without synchronization both immediately turn the gate red). The red-list **mechanism** is
> retained: stale detection under gate criterion 4 and the `--selftest` `stale-red-list` fixture
> continue to cover its detection surface; any future registration must go through an explicit
> decision.
>
> Alongside the closure, the convention changed: obligation inventory 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / this document’s § Obligation Fields table); the gate’s `--selftest` now covers
> seven fixture categories (adding “unregistered field,” “stale anchor,” and “empty red list”), as
> seen in
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2) (three hard criteria beyond the sketch’s wording)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: the
>    `INVENTORY` in `obligations.rs` ↔ the field definitions in `TypeCheckResult` ↔ this document’s
>    “§ Obligation Fields” table (including the table header’s count declaration); any drift turns
>    the gate red. In addition, the `TypeCheckResult` field set must be **exactly equal** to
>    “obligation inventory ∪ non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding
>    a field without registration, deleting a field without synchronization, a non-obligation
>    registration without a reason, and overlap between the two tables all immediately turn the gate
>    red.
> 2. **Read evidence**: every non-red-list obligation must have a real read in its **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings do not count)—this turns
>    “delivery marker ↔ downstream real read” into a hard criterion (the primary risk in mechanism A
>    is precisely drift between the two).
> 3. **Delivery markers**: every non-red-list obligation must be marked at a delivery point;
>    conversely, a red-list entry must not be marked.
> 4. **Red-list stale detection**: red-list fields must have zero real reads (the emergence of a
>    consumer turns the gate red), and each entry must carry a WBS ID and evidence pointer; there is
>    also `--selftest`, with built-in negative/positive fixtures: negatives must turn the gate red
>    and positives must turn it green (proving that the detector is not idle).
>
> **6. Deviations from the sketch above, item by item**
>
> 1. The sketch’s `pub struct Obligations { …fields… }` **does not copy the fields one by one**:
>    obligation data remains owned by `TypeCheckResult`, and the ledger records only the
>    “inventory + delivery markers + settlement conclusions.” Reason: copying creates two sources of
>    truth (the fields are produced separately by typecheck’s checker and by the ownership layer),
>    which must inevitably drift; the cross-layer data plane should not be duplicated into the
>    orchestration layer.
> 2. The sketch’s `take()` / “mark consumed” **was not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. Reason: an obligation and its artifact
>    are the same data, so `take()` would remove something the downstream still needs; moreover, 11
>    IrGeneration obligations take the form of “passing the entire `&TypeCheckResult` to ir_gen,”
>    which cannot be taken field by field.
> 3. The sketch’s signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (from 4.3.2 onward, W1090 is E level)—single file uses
>    `PipelineError::TypeCheck` (the external-emission arm in obligations.rs:654), while
>    multi-file/embedded record an `OrchestratorError` failure (otherwise, `result.errors` would
>    have no consumer). Reason: returning `Vec<Diagnostic>` would lose form-specific routing—check
>    diagnostics must retain their per-file attribution.
> 4. The first tier of the three-tier table changed from “the obligation was read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ real read” is delegated to the static gate
>    (criterion 2 of the fifth section).
> 5. The second tier’s “promote to E level in `strict` mode” **did not use a strict switch**; it
>    became **promotion of the same code to E in 4.3.2** (no strict switch, yet it is promoted,
>    because zero emissions in the healthy state are already safe enough, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no
>    third-file read” to the four criteria in the fifth section.
> 7. The two additional tiers absent from the sketch (S1 and skip exemption) are covered in the
>    second section.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table actually became
>    **20 test cases** in `src/driver/tests/obligations.rs` (first an `#[ignore]` skeleton → then
>    green; red-state record: removing the sole settlement point yields `pairs: 0` and six
>    failures). The S1/S4 rows were updated to the actual file.
>
> **7. Implementation record for 4.3.2 (severity + gate hardening)**
>
> - **Inputs for field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in the current run, available per run) and the five counters (the conservation
>   identity in the second section); together with the audit surface `findings`, these provide the
>   fact of “whether this field has ever been consumed by any entry point.”
> - **Severity: promotion of the same code to E is implemented** (user decision A on 2026-10-13; see
>   the final-state registration and D59 D-class breaking-item declaration in the third section).
>   Implementation: `to_diagnostic` explicitly calls `.severity(Severity::Error)`; the
>   external-emission arm routes by severity into the error channel. Mutation evidence: removing any
>   delivery marker causes that form’s compilation to fail and `check` to return a nonzero exit code
>   (`error [W1090]`); after restoration, the healthy state emits nothing (corpus 342/342 with zero
>   changes). The locale text now states “W-family code, emission level is error” in all six
>   languages.
> - **Gate-hardening items (F1/F2, both implemented)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence**, validating that “the file is locatable
>     (uniquely), the line number is in range, and the symbol immediately following the anchor
>     really appears in the anchored file”; an anchor pointing to a nonexistent symbol turns the
>     gate red. Built-in negative fixtures: `tests/baselines/obligations-gate/stale-anchor/` (one
>     nonexistent symbol and one out-of-range line). Note: the red list is currently empty, so the
>     criterion’s actual landing point is **non-obligation registration** and future evidence
>     pointers (the same code path).
>   - **F2/R3 reverse containment**: the gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference”; a nonzero difference turns the gate
>     red; a built-in negative fixture is provided in `unregistered-field/` (a new struct field
>     without registration).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck fails does not retain a `type_result`, so the obligation
>   does not exist in the first place); this tier’s semantics are pinned by a synthetic `State`
>   test, while real end-to-end coverage includes only `ConfigDisabled` (t3 T06).
>
> **8. Closure of the second-level 4.3 tasks (red list zero + final E-level obligation
> diagnostics)**
>
> **8.1 Three-round implementation timeline** (mapped item by item to code file:line; this round
> only records conclusions and changes no code)
>
> | Round               | Conclusion                                                                                                                                                                                     | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
> | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger launch | `State::assert_drained()` is the sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                    | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; external emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                       |
> | 3.4.4 closure       | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                            | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                            |
> | 3.4.5 closure       | `method_overload_ir_names` cross-stage mirror deleted (retain the in-phase candidate-name table + `overload_resolutions`)                                                                      | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.9 closure       | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirror for middle/LSP consumption” claim, retain in-phase consumption; obligations 14 → 13, red list 1 → 0          | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under decision A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, `src/frontend/core/sema/inference/expressions.rs` seven sites (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 severity      | **Promote the same code to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; external emission routes by severity into the error channel (first failure takes priority) | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); registration of the convention that code family and severity are decoupled is at `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                          |
>
> **8.2 Closure conclusions**
>
> - **Red list is zero**: the number of entries with `red_list: Some` in `INVENTORY`
>   (`src/driver/obligations.rs:201`) = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The **mechanism is
>   retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, stale detection under gate
>   criterion 4, and the `stale-red-list` fixture; the gate prints “red list: 0 entries—closed to
>   zero.”
> - **Final E-level obligation diagnostics**: the **code number and W-family membership of `W1090`
>   remain unchanged**, while the emission level is `error` (D5 final state, user decision A on
>   2026-10-13). This **is a D-class breaking item under D59** (it changes externally observable
>   behavior) and has been registered in both the third section of this note and RFC-013’s “Emission
>   Channels” section; **zero emissions in the healthy state ⇒ no impact on existing corpora or user
>   programs** (corpus 342/342 “difference: empty,” zero IR-snapshot drift; t11 independent
>   verification and t12 review each checked it again).
> - **Closed field enumeration**: the `TypeCheckResult` field set = obligation inventory ∪
>   non-obligation registry (8 entries, `src/driver/obligations.rs:150`, each with a measured
>   consumption-surface reason); gate F2 explicitly prints the difference (currently
>   `21 - 13 - 8 = 0`).
> - **No unimplemented promises are left**: all hardening items recorded in this note have been
>   **implemented** (F1/F2/F4/F5; see the fifth, seventh, and eighth sections); the only two
>   unresolved items are the **known boundary** in 8.4 and t7 F5, which was converted into an issue;
>   neither is an unimplemented criterion of this mechanism.
>
> **8.3 Handling status of findings from each round’s acceptance/review** (identified individually)
>
> | Source        | Finding                                                                                                                                                                            | Status                                                                                                                                                                                                                                                                |
> | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is a plain-text pointer, so it can become silently stale if the alternative consumer is removed                                                                  | **Implemented**: gate F1 anchor validation (file uniquely locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures in `tests/baselines/obligations-gate/stale-anchor/`                                                    |
> | t3 F2 / t4 R3 | Gate criterion 1 lacked reverse containment, so a newly added field that was not registered silently passed                                                                        | **Implemented**: F2 explicitly prints the difference (`:577`) + F4 declaration-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                         |
> | t4 R4         | The W-only versus E-promotion policy was not registered                                                                                                                            | **Implemented**: third/seventh sections of this note + RFC-013 “Emission Channels” + W1090 help text and error-output examples in the six `locales/*.json` files                                                                                                      |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (a valid overload is falsely reported as `W1005`)                                                                    | **Converted to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (registration in the 4.9 row of `docs/src/dev/architecture/09-execution-wbs.md`)—not part of this second-level task                                                      |
> | t11 F7        | Under the gate’s `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                                  | **Known boundary** (see 8.4)                                                                                                                                                                                                                                          |
> | t11 F8        | Validator-side self-record: a mutation-driven run once wrote to the main repository by mistake (cleared immediately with `git restore`; affected round output discarded and rerun) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                          |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document’s table header differ from `INVENTORY`) turned the gate red                                               | **Informational / resolved**: the frozen commit is self-consistent and passes; “directly committing an intermediate state turns the gate red” is precisely the intended behavior of F2/F4                                                                             |
> | t12 F12-1     | This document gave the struct range as `16-73`, while the actual range was `16-76` (after t9 added three comment lines, with no mechanical reconciliation)                         | **Implemented and mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; an incorrect value turns the gate red); related numbers were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                         |
> | t12 F12-2     | Settlement in the multi-file form once overrode the first failure (the type error in `zzz.yx` was overwritten by W1090)                                                            | **Fixed** (the captain amended the commit message): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression is caught by an **existing pinned test**, `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1’s scanning surface**: anchor validation covers only the text of **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If such an anchor promise is added to a module header
>   in the future, **manual review must assume responsibility**, or F1’s scanning surface must be
>   expanded—choose one; neither may be left unattended by default.
> - **The gate’s static boundary**: set-level validation of delivery markers is handled statically
>   (criterion 3), while removal of an individual arm is covered by **run-time settlement**—the
>   static + run-time two-layer design is intentional, not a gap (bidirectional mutation
>   verification M11A/M11B/M11C in t11).
>
> **Revision note (P4 implementation, 2026-10-13, implementation of the 4.5 cross-layer PlanId
> contract):** Of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) according to
> RFC-039 **D20**. This note records the implementation form, criteria, classification rationale,
> and known boundaries; definition lines are still reconciled by gate **F4** (the line numbers in
> the two corresponding rows above are the current code facts).
>
> **1. Implementation form (carrier chosen A: `PlanId`)**
>
> The key is a **structural identity** carried with the AST node, no longer relying on the
> positional coincidence of “each side calculating a span separately”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` reserved for synthesized nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. It has two carriers: `Stmt.plan_id` (statement identity
> → ownership of the release plan and its consumption) and `Expr::Call.plan_id` (call-site identity
> → production and consumption of overload resolutions); `Expr::call` is the repository’s sole
> call-expression constructor, and synthesized calls outside the parser all record `UNPLANNED`
> (**not consumable**; see the second section).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**reading and accounting occur in
>   the same call**; `ir_gen.rs:360` reads and `:3103` uses it (statement identity is provided by
>   `walk_stmt`’s set/restore operations + the CFG-node-to-statement-identity table; Drop ownership
>   at scope exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (the table and consumption ledger **in one
>   structure**), produced at `inference/expressions.rs:4550` (written by call-site `PlanId`;
>   dispatch transparently passes it through from `Expr::Call { plan_id }`), consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) through `OverloadResolutions::get`
>   (a hit is recorded).
>
> **2. Guaranteed consumption settlement (reusing the 4.3 ledger, without creating a second
> system)**: the consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each of the two tables; when
> `generate_module_ir` returns successfully, both ledgers are marked as having “completed a full
> round” **at the same point** (`ir_gen.rs:1095` release plan / `:1097` overload resolutions); the
> three-state mapping is unique (`consumption_state` in `src/driver/obligations.rs`), and the sole
> settlement point adds one line for each of the two obligations—a nonempty difference produces an
> **E-level W1090** (the same code promoted to E; policy in the third section). The exemption
> criterion is attached to **the ledger’s own completion marker** (not to whether `unit.ir` exists),
> so the Check form’s consumption of the IR artifact does not constitute an exemption.
>
> **3. D59 classification rationale (class C, red before green)**: before the change,
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> candidate was discarded and resolution fell back to the last binding in definition order); after
> the change, the same program produces a compile-time
> `error [W1090] ... produced key was not consumed`. The fix takes the **loud** approach rather than
> “put the resolved name into the call with named arguments”: the latter would lose named arguments
> and merely replace one form of silence with another (final-state semantics in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition: `Drop` instructions are no-ops in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change run-time behavior—the zero
> difference over 342 corpus files is empirical proof of this.
>
> **4. Criteria and evidence**: the same-kind criterion as D41, `test_release_plan_spans_consumed`
> (settlement by `PlanId`), and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus, **the difference set must be empty + evidence-floor
> assertions**: number of files that produced a resolution `≥ 1`, number of truly consumed entries
> `≥ 2`, with no allowlist); the nine pinned resolution shapes are covered in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> type-body binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven verification: shifting a consumed key by one
> position ⇒ criterion turns red + the real program produces `error [W1090]` + the corpus gate
> reports a difference; restoring it turns everything green again. Gate figures:
> `check-obligations.py` PASS, zero differences across 342 corpus files, `cargo test --lib` 2712
> passed / 0 failed (zero IR-snapshot drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: the **no-bindings
> consumption point** at `ir_gen.rs:7766-7790` is **unreachable under the current corpus and test
> surface** (method declarations uniformly go through `register_type_bindings` to the with-bindings
> consumption point), so the keying correctness of that branch **cannot be falsified in a red
> state**; the reason for retaining it is that it is the symmetric branch of the with-bindings path,
> and deleting it would turn “a method call without `type_bindings`” into another silent behavior.
> This was registered by independent verification (t5 F-1) and review (t6 F2), and **no new task is
> opened**.

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
    /// Must be one of the six predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be the six predefined combinations above (one per `ProgramKind`); callers may
not freely supply an array.** This is the key constraint preventing the `Program` abstraction from
degrading into “a free-form parameter that can accept anything,” and it is asserted by
`test_program_stage_coverage`.

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
                // …12 arms; not one may be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, implementation of 4.1.3):** There are four
> implementation deviations from the sketch above. They are semantically equivalent and recorded for
> reference:
>
> 1. The three-state `StageOutcome` from §4 is not returned by stage arms. It is expressed through
>    `State` failure markers + `Aggregation` gating (Continue / Warn / Abort semantics are
>    unchanged, avoiding repetitive return boilerplate in every arm).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (§5, “Avoid
>    mutable global state in Driver,” covers the sketch’s field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-difference criterion in S2, “Deliberately fix no
>    bugs,” requires this; external emission follows with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the Driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their proper location after the 4.2 migration, to be reconsidered later.
> 5. In the multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user decision on 2026-10-09): the original `compile_project` interleaved
>    proof within the per-file typecheck loop (file N’s proof ran before file N+1’s typecheck). For
>    a single failure, both are byte-for-byte identical; in the multiple-failure scenario “file 1
>    proof failure + file 2 typecheck failure,” the first reported error changes from the proof
>    error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through ProgramKind-specific channels: `result` (the pipeline
>    contract) / `module` + `failure` (the orchestrator contract, 4.2.1)—the external error
>    contracts of each entry point (PipelineError / OrchestratorError) do not belong to the type
>    surface that Driver can unify.
> 7. The Check channel is implemented (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; clean files have an empty Vec—the existing
>    `check_project` contract). The Discovery artifact is extended to a triple of “file set +
>    used_by edge set + shadowing events” in State (the MultiFile form has no downstream consumer,
>    so it only records them and does not emit them externally); the Check path’s per-file parse
>    count falls from two to one following the C3 topology.
> 8. The Lsp form is implemented (4.2.3, decision by file source on 2026-10-09, user’s ruling):
>    parse failures for on-disk files use Check’s plan B (collect + exit the compilation unit—both
>    sides converge on degradation); parse failures in an edited buffer retain a partial AST and
>    continue to typecheck (the editor philosophy—an intermediate state while typing should not make
>    semantic functionality disappear). The Discovery arm preserves buffer source over stale on-disk
>    content for Lsp; Typecheck checks only the target file in Lsp (other units exist only to
>    provide signatures to the registry); the intra-file diagnostic-order normalization principle
>    extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old LSP
>    quirks—hard-aborting on parse errors in unrelated on-disk files and the handler silently
>    falling back to the single-file path—are fixed in this step.
> 9. The Embedded form is implemented (4.2.4): `Program` gains a `shared_registry` field—the
>    embedded std module is a child compilation, and its registry is an **input**, not an artifact
>    (EMBEDDED_STAGES has no Registry stage), making #94’s SymbolTable-sharing contract explicit as
>    a program declaration rather than “the caller must remember to pass the same registry.” The
>    error-path text changes from `<std.test> (embedded std)` to the unit’s virtual path
>    `<std/test>` (compiler-internal error surface only, with no pinned test). At this point, all
>    four orchestrator entry points have moved to Driver.
> 10. Standalone check is unified (4.2.5, decision A + IR-stage decision on 2026-10-09): the Check
>     form faithfully carries the single-file semantics for programs without a project root—the
>     warning surface covers only the entry file (neighboring files receive only errors; missing a
>     warning is preferable to a false positive); relative `use` is resolved relative to the
>     importer’s directory (aligned with rustc’s single-file mod behavior). The CHECK stage table
>     adds GlobalSlotAlloc + IrGeneration: the old standalone path (the full pipeline) already ran
>     IR generation, and E3019/E1014/E1015 are produced only in ir_gen (verified by the runner
>     gate). IrGeneration in Check form is pure checking and does not consume IR; the Script/Bin
>     split is represented by the None/Some value of module_key (the existing E3023 switch,
>     ir_gen.rs:1660). Monomorphization is not included in CHECK (it only produces
>     resource-limit/internal errors and has zero corpus dependency). **〔Withdrawn on 2026-10-10
>     after empirical verification in 3.4.8〕**—“zero corpus dependency” is merely evidence from the
>     absence of pathological-recursion fixtures in the corpus; E3005 is the sole compile-time
>     defense for this class of programs. See revision note #14. Incidental fix for a latent defect:
>     duplicate registration harvesting by the Registry arm for embedded std units would overwrite
>     the native half already merged through with_std() (false E1043)—embedded units now skip
>     duplicate registration.
> 11. The LSP single-file fallback is unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` is deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     In the SingleFile+CollectAll form, Parsing collects all parse errors and retains a partial AST
>     to continue typechecking (the editor philosophy extended by the 4.2.3 decision; it does not
>     mark the stage as failed, because topology-based skipping would prevent typecheck from ever
>     running); Typecheck dispatches the `check_module_collect_all` free function. The LSP
>     single-file path now gains proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and
>     IR-level errors; lexical failures now report real diagnostics (the old synthetic “E0001
>     lexical error” text disappears with the deleted sequence—real diagnostics carry precise spans,
>     with no information loss).
> 12. `check_module` / `check_module_collect_all` are verified line by line (4.2.7, mandatory C5
>     item): the two entry points had already converged separately—the mod.rs layer uses
>     `check_module_inner(ast, env, collect_all)` with one boolean, and the checker layer uses
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, with pass-3 and drain modes both
>     running and the difference carried by whether `collected_errors` is empty; the five collection
>     sites in statements.rs (function body/use/for/block/while). Verification also proved and fixed
>     a duplicate-diagnostic defect: the collection sites placed the first error in both
>     `collected_errors` and the `Err` return channel, and pass-3’s `add_error` for `Err` produced
>     the same code/span/message ×2 (up to ×3 when nested); there were also two independent
>     mechanisms for double-visiting annotations (E1003/E1103) and ownership-layer emission
>     (E2014/E2018). The fix is at the module-result boundary: deduplication by (code, span,
>     message)—a diagnostic is a positional fact, and a duplicate carries no information; duplicate
>     copies were removed from 63 corpus entries, with zero drift in the run column and exit code
>     (the apparent “duplicate” in borrow_conflict_err is an artifact of the baseline triplet format
>     omitting the column number—the two E2018 diagnostics are legitimate diagnostics at different
>     columns).
> 13. The wasm playground is implemented (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper layer no longer participates. Error text is byte-for-byte aligned with the old
>     `CompileError` Display prefix (`Parse error:`/`Type error:`/`Internal error:`), with no
>     visible difference in the playground UI. WasmPlayground and SingleFile share
>     SINGLE_FILE_STAGES + FailFast (the 4.1 declaration now has a real producer).
> 14. Multi-file monomorphization and the CHECK stage-table correction (3.4.8, user decision on
>     2026-10-10, **partially withdrawing the mono exclusion in revision note #10**). Empirical
>     evidence disproves the claim that “mono only produces resource-limit noise”: with pathological
>     generic recursion (`f(x)=f([x])`) and no monomorphization on the multi-file path, the compiler
>     process overflows the stack (0xc00000fd—the interpreter recurses in Rust with no graceful
>     run-time error), and the mono depth gate is the sole compile-time defense; the exclusion
>     decision for standalone check introduced a silent coverage regression because the old
>     standalone-check path ran the full pipeline and therefore already had mono (“zero corpus
>     dependency” only proves that the corpus has no pathological fixture). The “type-erasure
>     fallback” assumption was invalidated by the RFC-033 reflection decision—`^^List(Int)` needs a
>     real instantiation identity, and monomorphization is the foundation of reflection rather than
>     an optional optimization. Four design points: ① typecheck produces an instantiation_request
>     for qualified calls (`lib.f(x)`, FieldAccess shape), with generic_id using the qualified name
>     in the same namespace as merged IR (currently only bare Var is recognized,
>     expressions.rs:2455); ② the `Stage::ALL` topology is changed to place Linking before
>     Monomorphization, and the MULTI_FILE stage table appends Monomorphization—it consumes
>     merged_ir and aggregates all-unit requests (qualifying the deferred bucket’s containing_fn),
>     so mono is promoted from a per-module pass to a whole-program pass; the single-file stage
>     table is unchanged (no Linking, so the subsequence property is preserved and IR snapshots have
>     zero drift); ③ CHECK gains Linking (pure merge—E3020 entry validation remains in
>     RoleClassification to prevent duplicate reports) and Monomorphization (pure checking with no
>     consumption, like IrGeneration in Check form); ④ resource protection covers cross-unit mutual
>     recursion through whole-program BFS. The Monomorphizer itself is unchanged (its ModuleIR +
>     request-set contract exactly matches the merged-IR form).
> 15. Implementation of 3.4.8 R1 (①) (2026-10-10, a9438007). Qualified-call instantiation collection
>     is implemented according to design #14: `ExpressionInferrer` receives a module-qualified key
>     table (all three `use` arms register it—ordinary / alias / item-by-item alias and only for
>     SubModule-class exports), and `callee_generic_name` is assembled by `SymbolTable::qualify`
>     using the same qualified-name source as merged IR; the qualified-call first-path arity
>     criterion is tightened (when the parameter table cannot be resolved, it falls back to the
>     second path and uses actual arguments from the signature, avoiding false E3018 reports caused
>     by matching same-named functions across modules). Implementation empirically exposed and fixed
>     **three mono-side asymmetries** in the same round: (1) The deletion key (the generic-name set)
>     and the rewriting key (the containing_fn + name + span triplet) had different granularities—a
>     call site whose actual arguments could not be resolved degraded into a symbolic request in the
>     deferred bucket, and if the containing function was non-generic, the bucket never drained,
>     while the original had already been deleted by name → a dangling call (run-time E6006
>     demonstrated by `list_ops.yx:53`); fixed with the restoration gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the sole source
>     of truth to determine coverage and retaining the original for uncovered sites. (2) The
>     iteration order of the restoration section’s `HashSet` made the function-table order
>     nondeterministic (t2 review F1) → changed to `Vec`, sorted by name, making the artifact
>     byte-for-byte reproducible. (3) The non-rewritten forms TailCall / MakeClosure still looked up
>     mapping keys as before (although the comment claimed conservatism) → a single-point
>     `call_form_is_rewritten` decision was introduced, and every non-rewritten form is counted as
>     uncovered. Each of the three issues has a pinned test (empirical red-state discrimination
>     before the fix). The corpus difference baseline was regenerated from the same commit, yielding
>     exactly one changed line (the `remove_at` frame gained `(int64)`). **Commit-strategy note**:
>     when typecheck collection and the mono fix were split into two commits, the staging tree
>     containing only the former triggered an `stdlib_docs` red against an existing HEAD defect
>     (`list.is_empty([])`, same E6006 family)—the two halves are each other’s prerequisite for a
>     green build, so they were combined into one atomic commit.
> 16. Registration of five decisions on 2026-10-10 (after closure of R1): (1) **D2 decision
>     A**—user-module exports carry `generic_fn_type_params_snapshot()` through
>     `Export.type_params`, structurally identical to the std path (`yx_sources.rs:62/108`): the
>     declaration name becomes the authoritative source for qualified-call monomorphization, and
>     signature reverse-inference is demoted to a fallback (included in R2’s scope). (2) **The HKT
>     declaration side is live, but the call side is broken** (inference passing type arguments
>     produces E1002 / explicitly passing type arguments produces E1010) is registered as WBS 3.5
>     (three third-level tasks). (3) **The deferred bucket never drains under a non-generic
>     container** is included in the acceptance sub-items of 3.4.8 ②: the bucket must drain or fail
>     loudly and must not silently fail to specialize. (4) **Calling a closure variable by name
>     produces run-time E6006** is registered as WBS 3.6; semantics: (i) support indirect calls /
>     (ii) reject at compile time, with **(i) chosen**—indirect calls are supported: the `CallDyn`
>     mechanism already exists (`ir_gen.rs:7945`, closure-expression path); what is broken is a
>     `Var` callee being downgraded to a by-name `Call`, and typecheck already accepts such
>     programs. (5) **Trade-off criteria are fixed**—a proposed direction may be rejected only on
>     correctness and readability grounds; “large change surface / introduction of a new mechanism /
>     high cost” do not constitute rejection reasons. This is written into the second part of
>     `coding-rules.md` and the HOWTO self-check list; trade-offs requiring a user decision follow
>     all four requirements in `AGENTS.md` under “Requesting a user decision (mandatory)”
>     (situation + evidence / itemized pros and cons / explicit recommendation + cost / explicit
>     naming of breaking changes).
> 17. Implementation of 3.4.8 ② and root cause F1 (2026-10-11, 375789d2 + d922a551). ② follows
>     design #14: `Stage::ALL` is changed to place Linking before Monomorphization, and the
>     MULTI_FILE table appends mono; the Linking arm aggregates whole-program requests and qualifies
>     `containing_fn` **before taking each unit’s IR** (otherwise, deferred-bucket keys would not
>     share the same source as merged IR and the bucket would never drain). **① closure (D2 decision
>     A)**: user-module exports carry a declaration-time type-parameter snapshot through
>     `Export::type_params`, structurally identical to the std path (`yx_sources.rs:62/108`).
>     **Existing defects empirically exposed and fixed during the process**: (a)
>     **F1**—`qualify_names` rewrote only function bodies, and `ir.init` was never visited, so in
>     multi-file projects function definitions were qualified while top-level module calls remained
>     bare → run-time E6006 (**reproduces even for non-generic code**, unrelated to generics); fixed
>     by having init reuse the same `rename`/`aliases` and the same `rewrite_call_names`. (b) The
>     mono side’s “whole-program unique suffix” heuristic and silent `continue` on a miss → the
>     heuristic was deleted, and misses now produce a **loud E3018** (silently mismatching an
>     ambiguous name is worse than failing to resolve it). **A regression found and fixed by
>     independent verification**: t6 discovered that a named import `use lib.{pick}` followed by a
>     bare-name call was falsely rejected with E3018 (the IR instruction name is the alias-qualified
>     name, while the request name is bare, and the aggregation side only qualified by unit key) →
>     fixed by having the aggregation side **directly call ir_gen’s** `import_aliases_from` and
>     `resolve_call_target` (extracted as a single public point, one producer and one consumer),
>     with a matrix pinned test covering both named-import forms. **Record**: the blank `DefId` in
>     the init section was empirically shown to be “consistent blankness” rather than a live defect
>     (the codegen-name fallback can resolve it, and all four kinds of init calls passed in
>     practice), and is registered under WBS 3.7.4 to close the semantic split between “function
>     bodies use def, init uses names.”
> 18. Closure of the init-section DefId semantic split (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs`
>     extracts the single point `resolve_static_targets` (Call/TailCall string operands +
>     MakeClosure), and function bodies and `ir.init` share both the implementation and the
>     `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)` pairs
>     changed (other-lines 0; zero residual `(def=_)` in the init sections across the entire tree).
>     The codegen DefId main path and the name-fallback path produce identical translation results
>     and execution values item by item (all four kinds of init calls measured), and the corpus
>     baseline has zero drift; the probe flips to the new criterion and is strengthened (identity +
>     exact index + execution value, with any fallback-init coverage turning it red). **Correction
>     (2026-10-11)**: the commit numbers above were rewritten by `git pull --rebase`—the original
>     b554497b/e60dcc87/b7ac7396 correspond respectively to 375789d2/d922a551/38bd4b9e after the
>     rebase; the pushed dev history uses the new numbers (the old numbers were local temporary
>     numbers and never entered remote history).
> 19. Implementation and verification of 3.4.8 ③④ (2026-10-11, 8ecf56f93). (The design basis for ③④
>     is #14; ②’s implementation is #17, which this entry closes.) **Implementation**: the
>     `CHECK_STAGES` tail = `…IrGeneration → Linking → Monomorphization`; Check’s `Linking` performs
>     only a pure merge (aggregating requests + `link_module_irs`), with entry validation left
>     entirely to `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (in
>     memory only); mono failures enter the Check collection channel, while MultiFile retains
>     `fail_with`; `into_outcome` first calls `take_check_diagnostics` and only then checks for
>     failures (structural failures no longer clear already-collected diagnostics).
>     **Verification**: four rounds of independent verification (t3/t4/t10 + a t5 cross-form matrix)
>     and one review (t6) all passed; corpus differences were byte-for-byte IDENTICAL across 340/340
>     files, IR snapshots had zero drift, and the test count rose from 2643 to 2656. **An existing
>     gap found and fixed in the same round**: F1 (in rootless multi-file check, the request name
>     and IR definition key did not share the same source → silent failure to specialize; see the
>     implementation note in WBS 3.4.8’s row)—the first fix was for the aggregation side to look up
>     the actual definition key precisely (using `flat_defs`); **same-day correction**: this bridge
>     was later deleted in the regression fix, and the root cause was instead decoupled by having
>     `module_key` serve two purposes (`script_mode`), so all three use the same `{unit key}.{name}`
>     source. The regression’s location and fix (the CI documentation-example gate went from 258
>     blocks / 10 failures to 0 failures) are recorded in WBS 3.4.8’s implementation note. **Record
>     (existing, not a regression from this round)**: the E3005 help text does not match
>     resource-protection semantics; the multi-file E3005 span label is misaligned (reproducible
>     with the HEAD binary); the type-parameter position dependency on the collection surface
>     remains (same-surface item under 3.5).
>
> 20. Implementation of 3.9.1 / 3.10.1 / 3.11.1 and closure of the diagnostic surface (2026-10-12,
>     809b9b936 + 8605a081b + two subsequent commits). **3.9.1 (execution-layer guard is
>     reachable)**: interpreted calls are changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frames and then
>     return control to the outer `step_one`; frames carry `ReturnTarget{Host|CallerSlot|Discard}`,
>     and return writes the result back to the caller’s slot, while multiple calls in one
>     instruction continue through the frame’s `pending_calls`. Baseline measurement showed that
>     each interpreted-call level consumed **2544 B** of native stack, with the crash threshold at
>     `count_down(394)` (Windows main thread, 1 MiB); after the fix, 20,000 levels of recursion run
>     on a 512 KiB stack thread (approximately zero per level). The guard (`push_frame`) is
>     therefore reachable from every entry point under every `max_stack_depth` setting;
>     `ExecutorError::StackOverflow` now carries the actual limit (formerly `util/diagnostic/mod.rs`
>     always passed 0). **The original 3.11.2 (truncating deep-stack echoes) was merged into this
>     task by user decision** (same location in the same file, indivisible): echoes now show the
>     first 16 frames + an omitted count + the last 8 frames, leaving shallow stacks unchanged.
>     **3.10.1 (no compile-time guard against native recursion, #447)**: locate first, then fix—the
>     crash point was recursive `MonoType::type_name` (**328 B/level**, determined with an SP
>     probe + point-by-point operation thresholds), and the same path also had a derived `Clone`
>     (~400 B/level), a deep copy in `substitute`, and `type_name_hash` (164 B/level), plus
>     `collect_generic_type_refs`; after changing these four sites to work lists and removing one
>     full-request deep copy per level, the pathological generic recursion with
>     `mono.max_depth=5000` produces **E3005 (with the actual limit)** instead of overflowing the
>     stack. **3.11.1 (preserving task-boundary error codes, #448)**: the task result’s error side
>     changes from a value to a **structured form** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), with values restored by value across the boundary;
>     E6001/E6003/E6004/E6008 inside tasks are verbatim identical in Standard and Embedded modes.
>     Two **pub API payload-shape changes** are declared as `BREAKING CHANGE` in the commit.
>     **Commit-strategy note**: 3.9.1 and 3.11.1 are **indivisible at file
>     granularity**—`interpreter/executor/executor.rs` simultaneously carries the trampoline and the
>     error-channel producer/consumer, and `TaskFailure` is defined in `runtime/engine.rs`, so
>     compiling with only one side staged fails (the pre-commit staged-subset compilation rejected
>     it twice); they were therefore combined into **one atomic commit** under the same principle as
>     #15. **Known boundaries and follow-ups**: derived Drop (~75 B/level) ⇒
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation) gate currently runs
>     on native recursion (about **3696 B/level**, default 256 levels ≈ 924 KiB), has zero call
>     sites, and is currently unreachable in production; future wiring must close the same family of
>     issues; the Standard-mode task interpreter is constructed with the default `ExecutorConfig` ⇒
>     its E6004 limit is always 1024 (configuration propagation is registered as WBS 3.12, user
>     decision B: a separate team is to be formed); the test-runtime budget is tracked in separate
>     issue #449 (pathological fixture 76s, clippy on two targets, full test run on commit), outside
>     this RFC’s implementation surface. **Corpus directory stratification (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134), 212 fixtures in total, are
>     split into 23 category-based subdirectories; 88 IR snapshots move in mirror paths; the corpus
>     baseline is regenerated in the same commit (342 lines, independently recomputed and compared
>     line by line: only the `file` field and two new changes), 41 snapshots are regenerated because
>     their header paths become longer, and line-by-line verification confirms **only span-offset
>     changes** (zero differences after offset normalization).
> 21. Unified LSP semantic-data pipeline (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, and cross-file references in projects carry
>     `resolves_to` pointing to the real definition—the old form ran **two passes** on
>     didOpen/didChange: diagnostics used the unified Driver pipeline, while semantic data used a
>     manual lex→parse→single-file typecheck in server.rs (no module registry, project-internal
>     `use` resolution failed, reference events were not produced, and cross-file
>     navigation/rename/find-references silently failed—the silent failure registered as D55 is
>     closed in this task). **Implementation**: (1) `Export` carries `definition_span` (harvested by
>     `extract_module_info` from AST identifier nodes, with the same source and value as the
>     typecheck definition event’s `def_span`; native std is always None), and `ModuleInfo` carries
>     `definition_file` (filled by the Registry arm from `DiscoveredFile`); (2) when the checker
>     drains reference events, it uses `import_def_locations` to redirect cross-file imported items’
>     `resolves_to` from the use line to the real location in the definition file (std imports are
>     unregistered and remain on the use line); (3) the checker’s module identity changes from the
>     unified `"<module>"` to the unit file path (this value is consumed only on the semantic-data
>     surface, with zero diagnostic consumers—verified across all four `env.module_name` consumption
>     sites); (4) `DriverOutcome` gains a `semantic_db` channel—fed by the CollectAll forms (Lsp
>     target file regardless of diagnostic success + SingleFile fallback)—and the Lsp arm’s tail
>     injects top-level export definitions from non-target files into State (`external_definitions`,
>     so even unopened files can be navigated to; type/submodule exports are not
>     injected—type-registration on the typecheck side does not produce definition events, and both
>     sides consistently leave the gap); (5) orchestrator gains `analyze_source_in_project`
>     (`check_source_in_project` delegates to it and takes only the diagnostic half); (6) the LSP
>     side’s `analysis.rs` single-compilation entry point replaces the manual pipeline
>     (`server.rs`’s `update_semantic_db` is deleted), and the World key is unified as a normalized
>     file URI (`uri.rs`: `path_to_uri(uri_to_path(u))` provides idempotent normalization, with
>     drive-letter case and `%XX` encoding differences normalized to the same key). **Incidental fix
>     (D59 category B—the old behavior did nothing silently and did not change the external
>     appearance of any previously successful program)**: cross-file
>     navigation/rename/find-references for in-project files now successfully reaches the real
>     definition instead of silently failing; on lexical errors, the old manual pipeline deleted
>     semantic information, whereas the new behavior retains a partially colored AST (the 4.2.6
>     editor-philosophy extension); when the target file has type errors, semantic data is no longer
>     discarded along with `type_result` (coloring/navigation do not disappear while typing).
>     **Pinned tests**: six Driver-level tests (target-file path key / cross-file resolves_to +
>     external-definition-surface round trip / std-import fallback to use line / error-file
>     retention / single-file fallback path key + error retention) and one LSP end-to-end test
>     (empirical red-before-green—before the fix, navigation returned None); four URI-normalization
>     tests.

**Migration method for the ten entry points (each function specified individually):**

| Entry point                                                | After migration                                                                                                                                                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to Driver                                                                                                                          |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a constructor for `Program { kind: MultiFile, aggregation: FailFast }`                                                                                                                                     |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a constructor for `Program { kind: Check, aggregation: CollectAll }`                                                                                                                                       |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to a Driver call + filter results to the target file                                                                                                                                                          |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                            |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project`                                         | Unchanged (delegates to Driver)                                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project paths use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                               |

**`check_project` and `compile_project` are approximately 40% shared / 60% divergent.** What they
share is: vendor-consistency verification, file discovery, `build_registry_from` (deleted on
2026-10-26 together with its zero callers; the historical account is retained here),
`all_method_bindings`, the parsing loop, per-file checker setup, and result handling. What differs
is items 2/3/4/5/7/8/9/10/11 from the 11-item table above. **This shared 40% skeleton is precisely
Driver’s value interval**—the divergent 60% consists entirely of “a different set of stages” or “a
different aggregation mode,” and these are exactly what the two `Program` fields can fully express.

### 4. Failure semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded; continue
    Abort,     // Stage failed; terminate this compilation
    Warn,      // Stage had a non-blocking problem; continue
}
```

**Key design decision: “an upstream failure caused this stage to be skipped” is not a stage return
value.**

Reason: skipping is determined by **topology**, not by the stage itself. If every stage returned
`Skipped` itself, the information about “why did I not run?” would be distributed across the 12
stages and could not be audited centrally. Instead, `dispatch` decides at the beginning of each loop
iteration based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect targeted by this
document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”—it
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every instance of “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                              | Direction of the text                                                                                                     |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                       | “Stage Y was not executed because upstream stage X failed”                                                                |
| Caused by an empty prerequisite obligation                          | “Stage Y was not executed because there was no pending obligation (normal)”—**severity = Info, not counted as a warning** |
| Preconditions not met (for example, `config.mono.enabled == false`) | “Stage Y was not executed because it was disabled by configuration”                                                       |

The second category is critical: it distinguishes “normal skipping” from “abnormal skipping” in the
diagnostic stream without polluting `yaoxiang check`’s `warning_count` (the non-blocking contract
depends on that count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

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

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project files and single files in the same process, while the CLI’s `run` and `check` are two
independent calls; placing the mode on Program avoids mutable global state in Driver.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not merely in “whether they
return early”—they are two different checker entry points. After unification, a single
implementation should be driven by the `Aggregation` parameter rather than retaining both functions
(**this is an additional item that must be verified in this document; see “Known Limitations and
Risks”**).

---

## Detailed Design

### Impact on the Type System

| Change                                           | Type-layer impact                                                                                                                                          |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                             | **None.** It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                 |
| Migration from `TypeCheckResult` → `Obligations` | **Field types do not change**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`              | **Entirely new types**, unrelated to the language’s type system                                                                                            |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain
where they are; `driver/obligations.rs` holds only the aggregate container and `assert_drained()`.
L3 consumers (such as `ir_gen` reading `release_plan`) continue to receive the concrete field types
through parameters and **must not `use crate::driver`**—this is consistent with the driver-layer
prohibition against reverse dependencies described below.

**Dependency direction** (RFC-039’s four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend on `crate::driver` in
reverse**. The presence of `Driver` in a `TypeChecker` import is a violation and is intercepted by
the proposed `scripts/ci/check-module-boundary.py` in RFC-039.

### Run-Time Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | Five stages                              | The same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer takes this pipeline for standalone programs—it uniformly uses `compile_project` (the MULTI_FILE stage table), and Script semantics are provided by rootless execution; this row now describes only the `lib::run_file` / `Compiler::compile` library paths)                                                                                                                                                  |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel”; Check / Lsp / MultiFile share the same implementation, with no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` for that arm to emit) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| LSP (in-project)                    | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Z3 not installed + single file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Z3 not installed + multi-file       | Silently skipped                         | Unified with the above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

**The only intentional behavioral break** is changing `.expect()` at `predicate.rs:35` to return a
diagnostic. This is consistent with RFC-027 §8’s positioning as “not bound to a specific solver” and
with the contract already declared at `backend.rs:60` (“the caller should degrade conservently”)—the
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

| File                                          | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib.rs`                                  | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                              |
| `src/lib.rs`                                  | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                  | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                    |
| `src/frontend/pipeline.rs`                    | `141-227`                                      | Replace the `Pipeline::run` function body with `Program` construction + a Driver call                                                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                    | `275-278`                                      | Move dead-code analysis out and implement it as the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                    | `306-311`                                      | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                    | `381-389`                                      | Move monomorphization out and implement it as the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                    |
| `src/frontend/module/orchestrator.rs`         | `450`                                          | Change `check_source_in_project` to use Driver                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/module/orchestrator.rs`         | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                      | Drive the choice between `check_module` / `check_module_collect_all` in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                           |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics` and call Driver                                                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                      | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                     |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                             |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent discard point**)                                                                                                                                                                                                                                      |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                             |
| `src/frontend/core/sema/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “an upper-layer failure prevents lower layers from running” (there is no short-circuiting)                                                                                                                                                                                                  |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                        | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure interface; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared interface. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 supersedes the original wording |

**Deletions**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; formerly
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct, and
**the problem is in the consumer, not the producer**; changing the producer would conceal the
architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch (wasm branch
reachability assessment belongs to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Handling                                                                                                                                                        |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches, and record the expected diagnostic-set change in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking**: programs that previously compiled now produce warnings       | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                  |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                     | W first, then E; S2/S4 proceed in two steps                                                                                                                     |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no more crash                                             | No break                                                                                                                                                        |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These two paths do not pass through proof_execution                                                                                                             |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                 |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contracts and a
unified Driver)** in RFC-039’s global stage sequence. S1-S5 below are this document’s internal
implementation order, and **every step’s acceptance criterion cites category C2 from the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration changes: identical
**diagnostic sets** across entry points + identical corpus behavior).

### S1: Establish the criteria (red first)

**Prerequisite; it must not be skipped.** The defect criterion must first be written as a failing
test.

| Deliverable                                                                                                                                                      | Acceptance                                                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                    | **Must be red.** If it is unexpectedly green, the defect analysis in this document must be reviewed again         |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                             |
| `src/driver/tests/obligations.rs` skeleton (first marked `#[ignore]`)                                                                                            | Mark it `#[ignore]`; turn it green in S4 (red-state record: removing the sole settlement point yields `pairs: 0`) |

**Rollback point**: no code changes; additions only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                                   |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array                   |
| Change `pipeline.rs:141-227` to call Driver              | **The single-file path’s diagnostic set and exit code remain byte-for-byte identical**                            |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, and stdout/stderr must all have **zero differences** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. The
acceptance criterion is C1/C2-level zero difference.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Merge the four orchestrator entry points + fix the defect

| Deliverable                                                                                                                  | Acceptance (C2)                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are normalized according to `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                     |
| Delete the LSP manual stage sequence                                                                                         | LSP and CLI produce the same diagnostic set for the same file                          |
| **Turn `test_multifile_proof_obligation_not_dropped` green**                                                                 | The defect is fixed                                                                    |
| Fix the ownership-layer `Unproven` empty match arm (formerly `checker.rs:1313`)                                              | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode  |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | Removing the Z3 environment no longer causes a panic                                   |

**Rollback point**: the defect fix and the structural merge must be **two separate commits**. If the
structural merge causes problems, only the structural commit can be rolled back while the defect-fix
commit is retained—then `test_multifile_proof_obligation_not_dropped` remains green; the reverse
(retaining the structural change and rolling back the fix) turns it red, which is an unacceptable
intermediate state and must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | The 20 test cases in `src/driver/tests/obligations.rs` turn green (for the implemented form, see the revision note in §2; the count is authoritative in that file) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                                       |
| Promote obligation diagnostics from W → E    | Manually review every new E after the full corpus differential test                                                                                                |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration option;
switching between W and E does not require changing the code structure.

### S5: Finish the proof layer and wasm cleanup

| Deliverable                                                                                                                                                                                                                                              | Acceptance                                                                                                                                                                                       |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                         | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                                                     |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise a single timeout becomes fixed across compilations through the process-wide cache and pollutes sharing between cargo test threads) | Production-path `default_solver()` call sites fall to zero (tests only retain them); cache-hit rates across the three consumers are observable (the counter is already implemented in 89576fafd) |
| Add a warning to the degradation path at the `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                       | A diagnostic is produced when there is no `body_checker`                                                                                                                                         |
| Determine the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                      | The conclusion is handed to the “F2 Assessment” section of `06-cleanup-inventory.md`; this document records only the assessment requirement                                                      |

**Caution**: correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item should proceed independently of S1-S4** and must
not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustively-enumerable enum with 12 variants; **run-time registration is rejected**                                                                                                                                                                                                                                                                                                                                                                                                  | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is structurally identical to the opcode-table mechanism already proven in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating the need to reason about invocation counts                                                                                                                                              |
| **Obligation mechanism** | `Obligations` + `assert_drained()` provide **run-time** settlement, used together with the **static** `scripts/ci/check-obligations.py` gate; **the final severity is E level** (user decision A on 2026-10-13: promote the same code to E, leaving the code number unchanged; **for the implemented form, see the revision note in §2**: the ledger does not duplicate the field structures; the red list—zero after closure—is retained as a mechanism and does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not just one bug: at least two instances of the same class (`proof_calls`, `checker.rs:1313`) have been verified, and all 13 obligation fields are within scope. The W→E split was intended to make every C2 criterion usable at each step—now that the obligation inventory has been closed to 13 entries and the red list is zero, and the healthy state emits nothing, the final state directly uses E (see the seventh and eighth sections of the revision note in §2) |
| **Fix scope**            | Fix the **consumer** (orchestration layer); do **not** change the production-side logic of the `Unproven` branches (now three sites in `checker/refinement.rs` and one in `checker.rs`; formerly `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                                                                    | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic has been fixed,” even though the logic of these branches is correct                                                                                                                                                                                                                                                                                                           |

This design additionally eliminates two classes of cracks:

- **The crack between “comment promises” and “code behavior.”** The “silent pass must not be allowed
  to return” statement in `checker/refinement.rs` (formerly `checker.rs:5165`) is a **comment-level
  contract**, fulfilled by `assert_drained()`.
- **The crack of “one array that someone forgot to append to.”** Today, five functions each manually
  implement the call sequence; after unification, there is one stage table, and a missing stage
  causes a compile-time error.

### Rejected Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—seven of the eleven
  inconsistencies (two dead-code-analysis implementations, W1006, W1003, monomorphization, role
  context, IR generation/linking) **do not involve type transitions**; they are configuration
  questions of “whether an analysis should run.” It also cannot express `Aggregation`, so it merely
  moves the choice between `check_module` / `check_module_collect_all` from the function layer to
  the type layer.
- **`dyn Stage` + run-time registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions with manually written call sequences” with “one array that someone
  forgets to append to,” which is precisely the cause of the current eleven inconsistencies. This
  repository already has a same-shape counterexample: `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader`, described in `docs/src/dev/design/check/`, have no implementation
  at all.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields, while `Stage` remains scattered across five functions and the
  next divergence point will continue to grow from here. **This must be done first** (as part of
  S1/S3), because the structural refactoring needs a known red test proving that the criterion has
  discriminating power.
- **Make `proof_calls` public and add `debug_assert`**—`check_module` is a general entry point and
  does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must also occur in the
  orchestration layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into “a Program constructor + several Driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactoring that “centralizes
  the wiring.”
- **S3 significantly changes the diagnostic set, and the direction of the change is “exposing
  previously silent problems.”** After the fix, users may report a batch of “new errors”—these are
  real bugs that were never reported before. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual, field-by-field judgment.** Some
  obligation fields, such as `module_namespaces`, may intentionally require no consumer and should
  not generate a warning. S4 requires a field-by-field review rather than a one-size-fits-all
  policy.
- **The `Program` abstraction may be premature.** If the stage sets of certain entry points remain
  unstable for a long time, `stages()` could degrade into “a free-form parameter that accepts a
  different array on every call,” and the contract constraint would fail. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can only come from the six predefined
  combinations.**
- **Criterion dependency**: the defect-fix criteria in S1/S3 depend on the multi-file corpus layer
  in the [equivalence-criteria document](07-equivalence-oracle.md) being established first. **If
  that document has not first established the multi-file corpus layer, S1 cannot be accepted**—the
  existing 293 corpus files all take the single-file path and provide zero coverage for this class
  of defect. This document does not address the IR verifier (`verify_loose`); that prerequisite work
  is outside its scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the order of `termination` and `ownership` is the reverse of their
  declarations). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undecided.** `backend.rs:13-19`
  already states that `Solver` is `Send` but not `Sync`, and the cache of `Z3Backend` uses
  `RefCell`; after changing it to a singleton shared across compilation units, all access paths must
  be confirmed to pass through `Mutex`.
- **Stage parallelization has not been evaluated.** Multi-file typechecking is naturally
  parallelizable, but parallelism can **conceal order-dependency defects** (such as the actual order
  dependency between termination and ownership). It should be considered only after the equivalence
  criteria are stable.

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> item-by-item resolutions. **No pending items remain in this document.**

## See Also

- [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) —
  the four-layer model, routing tables A/B/C, G1-G10 acceptance gates, and P1-P10 execution order
- [01-routing.md](01-routing.md) — stage tables and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of the three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (implementation steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria, and the
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — the design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — the
  comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — the explicit statement that “silent pass must not
  be allowed to return” (formerly `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — the layer call-order table and the true identity of
  `equivalence` (no short-circuiting)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — the explicit rationale for hard failure on
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — the contract declaration that “the caller
  should degrade conservably” (contradicting `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-89` — 14 obligation fields that downstream code must consume
  (after the 3.4.4/3.4.5/3.4.9 closures)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — the unit test’s own
  admission that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  the corpus’s own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground’s
  fourth compilation caller (the single-file path, so proof_execution executes)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — the build-time error-code gate, a model for this project’s enforcement
  mechanisms
