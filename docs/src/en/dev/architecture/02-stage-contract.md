# Compilation-Stage Contracts and the Obligation Ledger

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criterion tiers, and execution-stage order are described in the main
> RFC-039 text; see the [index for this directory](index.md) for the role of each companion
> document.

## Purpose and Scope

This document is the construction blueprint for RFC-039’s **L1 orchestration layer**. It addresses
one problem: **“Which stages did this compilation run, which did it not run, and why?”** is
currently manually wired across six paths and ten entry functions, with no single place able to
answer the question.

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being violated by the architecture, not by the
> logic.**

The inline comment inside `revalidate_refined` in `checker/refinement.rs` reads as follows
(originally `checker.rs:5165`; relocated by P5 5.2.1): “Unproven → compilation error, no fallback,
no silent pass……silent pass must not be revived.” However, this invariant holds only on the
**single-file path**: proof-function constraints such as `x: Sorted(3)` silently pass on three
paths—the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never
executed, and no diagnostic is produced.

### Included

- The `Stage` enum (exhaustively enumerable, not registered at runtime) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- The unified `Driver` with a single `dispatch`, and the individual transformation required for each
  of the ten entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers affected by these changes, compatibility impact, and implementation
  order.

### Not Included

- The four-layer model, dependency-direction rules, and anti-bounce gate → `01-routing.md`
- Equivalence criteria (the C1-C6 tiers and three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- The global execution order for P1-P10 and the G1-G10 acceptance gates → the main RFC-039 text

### Division of Labor with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should be performed.
This document defines the concrete form of L1, the file-by-file change list, and the implementation
stages within this document (corresponding to P3, “Fix correctness defects,” and P4, “Stage
contracts and unified Driver” in RFC-039’s global sequence). If this document conflicts with
RFC-039, RFC-039 takes precedence.

The equivalence criteria are evaluated under category **C2 (orchestration changes)** in the
[equivalence-criteria document](07-equivalence-oracle.md): the diagnostic set is identical at every
entry point, and corpus behavior is identical.

## Current State

> Every item in this section is a **verified fact**, with a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect in which “one incorrect edit silently affects the entire repository”

This differs in nature from the other two classes of defects—module boundaries and test wiring:

| Defect                  | Typical manifestation               | Is there a signal?       |
| ----------------------- | ----------------------------------- | ------------------------ |
| Lexical/syntactic error | The source code is wrong            | A diagnostic is produced |
| Type mismatch           | The type is wrong                   | A diagnostic is produced |
| Stage omission          | A stage is never called             | **No signal whatsoever** |
| Unconsumed obligation   | A field is populated but never read | **No signal whatsoever** |

The last two share a defining characteristic: **failure does not produce an error**. Therefore, they
cannot be solved by “writing the code more carefully” or “reviewing more strictly”—code review can
reveal what was written, but not **what was omitted**. This is precisely the root cause identified
by RFC-039: “This project treats ‘design’ as a documentation convention rather than as an executable
constraint.”

### Existing enforcement mechanisms in this repository

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes and 8 W codes) are subject to
  a **compile-time hard gate** through `build.rs:19-55` and `tools/code-tables`, which compares
  every entry against the RFC-013 code table. Any discrepancy directly triggers `panic!` and rejects
  the build.
- `src/package/` has high test density (**76 files / 13,012 lines**, including 6,227 lines under
  `tests/`, or 47.9%). Every module’s test subtree is explicitly wired. This is the repository’s
  most complete test wiring and can serve as a formal reference for the stage-layer gate.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

**Line-count convention:** `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation path taken                                                             |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages called directly                                                        |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + separate IR                                              |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
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

### 11 stage-coverage inconsistencies

The following table provides cell-by-cell evidence. **A blank cell does not mean that the entry
point “does not do this”; it means that “no code in that entry point does this.”** This is precisely
the nature of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                             | `compile_project`                                              | `check_project`                                                            | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | -------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                      | **Yes**, `187-203`                                   | **No**                                                         | **No**                                                                     | **No**                             | **No**                               |
| 2   | Dead-code analysis                                       | Yes, `275-278` (gated by `config.dead_code.enabled`) | **No**                                                         | Yes, `356-377` (role-aware, no configuration gate)                         | **No**                             | **No**                               |
| 3   | W1006 local-module shadowing                             | **No**                                               | **No**                                                         | Yes, `336-348`                                                             | **No**                             | **No**                               |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)          | **Collected but never emitted**                                | Yes, `350`                                                                 | **No**                             | **No**                               |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                      | **No**                                                         | Yes (same as 2)                                                            | **No**                             | **No**                               |
| 6   | **Monomorphization**                                     | Yes, `381-389` (gated by `config.mono.enabled`)      | **No**                                                         | Not applicable                                                             | Not applicable                     | **No**                               |
| 7   | Typecheck branch                                         | `check_module`                                       | `check_module` (`124`), **returns on the first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all diagnostics | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | File discovery                                           | Not applicable                                       | `discover` (`101`, discards `used_by`/`shadow_events`)         | `discover_with_used` (`275`)                                               | `discover` (`454`)                 | Not applicable                       |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                               | **No**                                                         | Yes (`321-328`)                                                            | **No**                             | **No**                               |
| 10  | Global slot allocation                                   | Not applicable                                       | Yes, `allocate_global_slots` (`147`)                           | **No**                                                                     | **No**                             | **No**                               |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                          | Yes (`152-236`)                                                | **No**                                                                     | **No**                             | Yes (separate ModuleIR, then merged) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Current-state differences after P3
> landed are as follows:
>
> - **Row 1 has been fixed**: the five entry points share the same `frontend/proof_execution.rs`
>   implementation for proof_execution (pipeline + four orchestrator entry points), so the
>   unique-consumer defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6; user decision “do it” on 2026-10-19): multi-file
>   `run` now emits W1003 and dead-code-family warnings, and the W-code sets across four
>   paths—single-file run / multi-file run / single-file check / multi-file check—have been
>   normalized. The single source of the criterion is the shared implementation in the
>   `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk not yet proven to be a defect).
> - **Row 7** (the `check_module` / `check_module_collect_all` dual entry points) → assigned to
>   4.2.7 (`Aggregation` parameter-driven).
> - The code-side reference to “decision #434” previously had no documentation entry. It has now
>   been registered as RFC-039 **D57**.
> - **Embedded-std inclusion asymmetry (new fact outside the table, added on 2026-10-09):** The
>   single-file path unconditionally injects `std.list` through `merge_embedded_std_ir` (required
>   for for-loop desugaring; #117 hard switch), whereas multi-file `discover` previously recognized
>   only explicit `use` statements. A project-local for loop therefore compiled successfully but
>   failed at runtime with E6006, as confirmed by a probe. This has been fixed and WBS 4.10.2
>   closed. The related “multi-file path lacks the single-file path’s one-step parse hard-abort
>   quirk” was registered as WBS 4.10.1 and **has also been fixed** (immediately after 4.2.2; the
>   Check path was degraded to per-file collection. This audit method examines “stage coverage and
>   field consumption”; this case is a compilation-unit membership difference and belongs to the
>   missed dimension).
>
> - **Embedded-std registry surface coverage (discovered during 4.2.5 construction; fixed):** The
>   Registry arm repeatedly harvested registrations for the embedded std unit through
>   `extract_module_info`, which could displace the native half registered through `with_std()`
>   using the “native + yx” surface merge (`result.is_err` and similar entries were lost, causing
>   std.test to report a false E1043). This is the audit’s third missed dimension: merge semantics
>   for two registration sources under the same module key. The single-file path forms its registry
>   exactly once through `with_std()`, while the multi-file/Check Registry arm inserts registrations
>   per unit.

**Two points require precise wording; otherwise, the document would be inaccurate:**

- **The precise conclusion for rows 4/5 is:** `yaoxiang run` **never** reports W1001/W1002/W1003 on
  the **multi-file** path (`run_project` → `compile_project`); it **does** report them on the
  **single-file** path (`run_file` → `Pipeline::run`). The reason is `compile_project`: it stores
  the complete `TypeCheckResult` (including `warnings`) in `type_results`, but the function has **no
  `result.warnings` read site anywhere**. The only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 does not come from the same source either:**
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, which checks only whether a manifest exists). The
  two functions give different answers to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of the document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for the obligation.**

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

Its meaning is: “The predicate arguments of this refinement constraint are compile-time literals;
the proof function must actually be executed before the constraint can be decided.”

**Step 2 — The consumption point is inside the checker, but it only emits and does not consume.**

`src/frontend/core/sema/checker.rs` has **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                      | Location                                                                                                                            | Behavior                   |
| ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| Formal-parameter refinement validation (`revalidate_refined`, originally `checker.rs:5164`) | `if calls.is_empty()` → produces a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Produces no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`)  | Produces a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                 | **Produces no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)        | Produces a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                 | **Produces no diagnostic** |

The original inline comment in `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) reads:

> `RFC-027 §4/§9: Unproven → compilation error, no fallback, no silent pass.` ……
> `This branch therefore becomes a true line of defense rather than a speculative fallback: silent pass must not be revived.`

In other words: the author knew that “producing no diagnostic” was a defect and explicitly
classified it as a **known intermediate state with nowhere to go**, on the design assumption that
“someone will eventually come to read `proof_calls`.”

**Step 3 — The field really is populated in the result.**

`check_module_impl` declares a local `proof_calls` and collects into it (originally
`checker.rs:1234-1235`), then writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof-function obligations` (originally `:1439`). The
field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.**

In production code, `TypeCheckResult.proof_calls` (`types.rs:29`) is read in **exactly one place**:
`src/frontend/pipeline.rs:187`, which passes it as an argument at `189`:

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(The other repository-wide hits for the `proof_calls` identifier fall into three categories, none of
which consumes this field: the three production-side collection sites in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); `verdict.rs:61`, which is a same-named field
of `ProofResult`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — All four orchestrator entry points bypass that layer.**

The four functions `compile_project` (`99`), `check_project` (`273`), `check_source_in_project`
(`450`), and `compile_embedded_module` (`1374`) in `src/frontend/module/orchestrator.rs` **do not
call `pipeline.rs`**. They call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, they never reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discard path.**

`compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible for
compiling embedded std. This means that **the embedded std module’s own proof obligations are not
executed either**.

**Step 7 — Consequence: constraints silently pass.**

`y: Sorted(3) = 5` (`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs
successfully, and produces no diagnostic** on the multi-file, check, and LSP paths. The proof
function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layer.**

When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`), it
contains:

```rust
// Original checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer’s `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the promise at the `5164` branch that “silent pass must not
be revived,” and it is **independent of the orchestrator problem**—even if the entry-point layer is
completely fixed, this point will remain silent. (Its handling time should be the same batch as the
obligation ledger, because it is the same class of problem.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

RFC-027’s three unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test files’ own doc comments already state this. `rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer.`

**This is exactly the shape of the problem: the tests verify that “the obligation was populated,”
whereas the bug is that “the consumer never reads it.”** A test that covers only the producer and
not the consumer is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria is the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over 293 `.yx` corpus files under `tests/yaoxiang/` as the **third-layer criterion** and the primary
acceptance mechanism for C2-stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under `tests/`** (zero glob matches in the entire
  directory).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx`
demonstrates this point directly. The file is annotated with `// expect: compile-error E4018` (line
15), while its header comment on line 7 says “Status: ❌ should be rejected at compile time”—**it
passes precisely because it takes the only path that executes proof functions**.

**Conclusion: the third-layer criterion needs an additional multi-file corpus layer; otherwise, it
cannot serve as the acceptance tool for this document.** See “Implementation Points” S1.

### Obligation fields: 13 fields whose “production implies a contract,” with no mechanical guarantee

Among the fields of `TypeCheckResult` in `src/frontend/core/sema/types.rs:16-76`, **13 fields** have
doc comments that explicitly state “produced by stage X → consumed by stage Y.” In other words, they
are inherently **cross-stage obligations**:

| Field | Definition line | Producer → consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: Definition line = the field’s **declaration line** in `types.rs` (the
> line containing `pub` / `pub(super)`), not the first line of its doc comment—the comment block for
> a given field may span multiple lines, and only the declaration line is a uniquely anchorable
> position. The **struct scope** in the sentence above (`src/frontend/core/sema/types.rs:16-76`)
> floats with the struct’s line numbers and is mechanically reconciled by gate **F5** (a mismatch
> turns the gate red); definition lines are reconciled by **F4**. | `proof_calls` | `36` | typecheck
> → **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**keyed by
> `PlanId`**: read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-complete marker at `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**keyed by `PlanId`**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-complete marker at
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Reclassification record (3.4.9, user decision A on 2026-10-13):** The original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**: by design,
it is consumed in-phase by typecheck and has zero cross-stage reads, so it is not an obligation:

- **In-phase consumption sites (live):** `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct evidence):**
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; the `src/driver` side contains only
  the ledger’s own registration and documentation (removed during this round).
- **Disposition:** Delete the **declaration** in the doc comment saying that the field is “mirrored
  for consumption by the middle side/LSP”; retain the field and its in-phase consumption. The field
  is registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a reason). If LSP
  genuinely needs the table in the future, register it anew under 4.6.x; **do not create a consumer
  in this round**.

**All these fields cross layers keyed by `Span` or by a name table.** The equivalence-criteria
document already points out that a mismatch in a span-keyed contract can “fail silently, without any
error” (`ReleasePlan` is a typical case: any difference in how the two sides compute a span → the
`Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**. The consumer
  is in a downstream module, and every entry point must pass through that downstream module.
- The consumer of `proof_calls` is at the **top level of `pipeline.rs`**, which belongs to **another
  entry-point implementation**. The four orchestrator entry points do not pass through the
  `pipeline.rs` layer at all.

**Verified fact:** The repository currently has **no mechanism whatsoever** that guarantees these 13
fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride with IR
generation.

### Non-obligation field registry: channel / in-phase / in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries):** Every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), complementing the “§Obligation Fields”
table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (zero hits in `src/middle`)                                                                                          |
| `diagnostics`             | Error-diagnostic channel (existing channel, not “production implies a contract”)                                                                        |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its doc comment does not declare a cross-stage contract ⇒ not incorporated                           |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                                    |
| `semantic_db`             | Consumer is in LSP (from 4.6.1, exposed through the `DriverOutcome.semantic_db` channel at `src/lsp/server.rs:482`); not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, but its doc comment does not declare a contract                                                            |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                             |
| `interface_impl_registry` | In-tree fact (user decision A in 3.4.9; see the reclassification record in “§Obligation Fields”)                                                        |

**Why not “wire up a consumer”**: Creating downstream consumers for these fields would introduce
fictitious obligations out of nowhere and disguise “no cross-stage consumption is required by
design” as a contract defect. The correct closure mechanism is **reverse containment**—the
`TypeCheckResult` field set = obligation inventory ∪ non-obligation registry (hard-checked by
criterion 1 of `scripts/ci/check-obligations.py`: adding a field without registration, deleting a
field without synchronizing the registry, omitting a reason for a non-obligation entry, or
overlapping the two tables all turn the gate red immediately).

### Four additional contract defects in the proof layer

The following all come from proof-layer analysis. They are independent of the entry-point divergence
problem, but all belong to the class of “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation; there is no mechanical enforcement, and the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (locations are anchored by function name/driver point; line numbers
are taken from the current tree):

| Actual order | Call site                                                                                                                                             | Corresponding layer |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement walk driver `predicate::check_predicate` (the three sites `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                       | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                          | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                  | —                   |
| —            | `equivalence::check_type_equivalence`—**zero calls in `checker.rs`**                                                                                  | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module’s own header comment and do not correspond to the call order. The README has no “number
→ invocation point” mapping.

`equivalence` is used only as an `is_subtype` utility function in `inference/assignment.rs:17` and
is unrelated to `ProofResult`. At the same time, **there is no short-circuit
whatsoever**—`check_module_impl` adds termination errors one by one and then continues downward. “A
lower layer fails, so the upper layer does not run” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07):** The hard-failure panic was eliminated by P3 task 3.3.1 (the SOLVER
> slot was made optional; a missing solver conservatively degrades to `SMTResult::Unknown`). “Silent
> skip (no injection)” was supplemented with a W1081 signal by 3.4.2. Unifying the three forms
> through singleton ownership is to be performed under **RFC-039 D58**, at the real fix site
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> entry point). The `default_solver() → &'static` formulation in the 02 change list is superseded by
> D58 (a bare `&'static` reference is infeasible because `dyn Solver` is not `Sync`).

| Consumer                                                                                                   | Acquisition strategy                                     | When the solver is unavailable                               |
| ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected at construction through `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection via `with_solver_owned`         | `None` → **silently skipped** (no injection)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                                | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the solver kernel’s capabilities.’” By contrast, the contract documentation at `backend.rs:60`
states: “`None`: the backend is unavailable……**callers should conservatively degrade**.” **Two
philosophies coexist in the same codebase, and the hard-failure path panics instead of returning an
error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**

`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge-cut decision.
Meanwhile, `src/frontend/core/sema/proof/smt/backend.rs:67-72` contains:

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
across calls ⇒ **the cache never hits in this calling mode**.

(For comparison, the `LazyLock` at `predicate.rs:34-36` really is a singleton. It is the same
backend with two lifecycle strategies.)

**(d) The checker silently degrades, and its comment does not match actual behavior.**

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
table** and runs as usual. Ownership analysis degrades to “nothing conflicts,” **without any
warning**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm state: the shim crate carries the target, and branches in 29 files are live

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                     | Evidence                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate is `rlib` only                            | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                           |
| **The shim crate provides `cdylib` + `wasm-bindgen`**    | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                           |
| The shim depends on the main crate (rlib) as its library | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                                 |
| The main crate has a wasm-target dependency section      | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                             |
| The shim directory is excluded from the main workspace   | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                    |
| **CI has four wasm builds**                              | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, namely the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten     | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL; missing files degrade to a warning)                                                                                                                                                                 |
| **29 files** contain the literal `wasm32`                | Of these, **22** have actual `#[cfg(...)]` attributes, while the other 7 mention it only in documentation (the list and cell-by-cell determination are in the “F2 Determination” section of `06-cleanup-inventory.md`)                                                                       |
| `orchestrator.rs` has 16 occurrences                     | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                         |
| `src/lib.rs` has 11 occurrences                          | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                               |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call under the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` as a whole
(all three require `std::fs`)—while the shim takes a different path.

**However, this produces a fact directly relevant to this document:** the playground entry point
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
| In-project LSP (`check_source_in_project`)                | **No**                    |

In other words: **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three are on the single-file side.** When the unified Driver was implemented,
this path was already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`). The landing record is the **4.2.8** row
of `09-execution-wbs.md` (`wasm` now takes `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 Determination” section of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time; six `not(wasm32)` arms are load-bearing across the three native stage tables. There
are no directly deletable low-risk items. The RFC-039 open question “build a wasm target, or delete
these branches?” is **obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, not registered at runtime)

**Core constraint: `Stage` is a compile-time exhaustively enumerable enum; runtime registration is
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
    ProofExecution,     // Compile-time execution of proof functions         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merge       Project
}

impl Stage {
    /// All stages. When a new variant is added, this array and the exhaustive match in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* All 12, in topological order */ ];

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

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07):** Two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    product (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the initial order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to dead code **before** proof, following the
>    topological order of `Stage::ALL`: in the current `check_project`, proof executes before dead
>    code. The two have no data dependency, and the diagnostic sets are identical (C2 set
>    semantics); only the within-file diagnostic order is normalized. The single-file path (dead
>    code embedded in typecheck, before proof) already follows the ALL order, so byte-for-byte
>    acceptance is unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user decision on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products. The initial order would force the Registry arm to “hide parsing,” making the stage
>    table lie about the data flow; multi-file parsing was consequently reduced from two passes to
>    one. The current state is faithfully registered as well: parse errors on the multi-file path
>    used to be a hard abort (the `?` propagation from `build_registry_from`, whose tension with
>    CollectAll semantics was registered as WBS 4.10.1; `build_registry_from` was deleted on
>    2026-10-26 when it had no callers, and the historical account is retained here)—**4.10.1 has
>    been fixed** (user decision on 2026-10-09: Rust-style collection semantics + plan B): on the
>    Check path, parse failures degrade to per-file diagnostic collection, and the defective file
>    exits the compilation unit (without entering the registry; importers report E5001); the
>    MultiFile path retains a hard abort (under FailFast, a defective file cannot produce IR; this
>    is correct semantics and remains permanently pinned).
> 4. The Check form landed in 4.2.2 (user decision on 2026-10-09) with two additional registrations:
>    a. Data-dependency edges 14→16: `RoleClassification` consumes Discovery’s `used_by` edge set,
>    and `DeadCodeAnalysis` consumes Discovery’s W1006 shadowing events. The initial draft omitted
>    these (in the orchestrator era, these products were produced by `discover_with_used` and
>    discarded by the `discover` wrapper; because the data flow never entered the table, it was lost
>    there); b. The normalization of within-file diagnostic order was extended to the full form in
>    note 2 above: E3020 entry validation moved into the RoleClassification arm (Check has no
>    Linking stage, so the arm holding `surfaces` and the AST is responsible for validation);
>    within-file diagnostic order = stage topological order (E3020 → typecheck → W1006/W1003/dead
>    code → proof). The diagnostic set remains unchanged (C2 set semantics); only the order of
>    stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective:** Turn “a field was produced but nobody consumes it” from an undetectable
condition into a fact that can be asserted at compile time or test time. RFC-039 has already
identified this as “the single highest-value item.”

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

| Situation                                       | Criterion                                                               | Handling                                                                                |
| ----------------------------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The corresponding field in `Obligations` is `take()`n / marked consumed | Pass                                                                                    |
| The obligation is non-empty but has no consumer | The field is non-empty and not consumed                                 | **Produce a diagnostic** (`W` level by default; promoted to `E` level in `strict` mode) |
| The obligation is empty                         | The field is empty                                                      | Pass (no consumption required)                                                          |

**Why start with W rather than E:** Fixing discarded obligations will **change the diagnostic set**.
The C2 criterion requires that “the diagnostic set is identical at every entry point.” Promoting
directly to E would cause `yaoxiang check` to suddenly emit large numbers of previously silent
`Unproven` diagnostics. A two-step approach—first W for observation, then promotion to E—ensures
that the criteria for every step remain usable. See “Implementation Points” S4.

**There is exactly one call site for `assert_drained()`:** `Driver::run`, at the end of the stage
table and before producing `CompilationResult`.

**Companion static gate:** RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing ≥2
times but with no third file reading it → failure). `assert_drained()` is a **runtime** gate; the
script is a **static** gate. Both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed):** This note registers the
> **implemented form, measured numbers, and 4.3.2 interface** of the sketch and three-tier table
> above, with a file:line correspondence for each item. The landing commit is `1941cc47a`
> (verification tree `99079bc52`). It is also the **third** of the three carriers of the red-list
> “must not be silent” rule (the other two are the audit surface and the static gate).
>
> **1. Landed artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: the 13-entry obligation inventory `INVENTORY` (`:201`, field
>   name/consumer stage/red-list status), the **non-obligation field registry
>   `NON_OBLIGATION_FIELDS` (`:150`, 8 entries with reasons)**, the delivery-marker ledger
>   `DeliveryLedger`, pure settlement `settle_obligations` (`:577`), external emission
>   `emit_obligation_diagnostics` (`:654`), non-consumer gating `consumer_visible_unit` (`:538`),
>   skip criterion `consumer_stage_skipped` (`:549`), the IrGeneration delivery set
>   `IR_GENERATION_DELIVERED` (`:285`), and the W1090 code constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point:** at the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` settles as well (the
>   ledger must balance).
> - **Seven delivery points** (one per arm; removing any one makes runtime W1090 fire):
>   ProofExecution at `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration at `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—all four use the
>   same delivery point: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization at `arms.rs:936` (direct request-table read
>   for a single file) / `:1221` (MultiFile·Check: aggregation in the Linking arm, with
>   `driver/helpers.rs:295` reading each unit’s `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is written above the entry at `:26-31`—settlement granularity is
>   “unit × obligation,” with no user-pointable source location, so no dummy span is inserted).
>   Six-language text is provided in `locales/*.json`. **Emission level:** from 4.3.2 onward it is
>   **error** (the same code promoted to E; see section 3)—determined by the explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code number remains in
>   the W family.
> - Static gate: `scripts/ci/check-obligations.py`; a new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above         | Implementation criterion                                                                         | Code                   |
> | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ actual read reconciled by the static gate; see section 5) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see section 3)          | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                 | obligations.rs:587-591 |
>
> Two **additional tiers** (user decisions on 2026-10-13; not present in the sketch):
>
> 1. **S1**: The consumer stage is not present in the current `ProgramKind`’s stage table → **do not
>    classify as outstanding** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (`:121-126`).
> 2. **Skip exemption**: The consumer stage was skipped because of `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → do not classify as outstanding (obligations.rs:598-602). `NoObligations`
>    **does not** grant an exemption—if an obligation is non-empty while the stage declares that it
>    has none, the ledger is unbalanced and an error must be reported.
>
> **3. External-emission policy (decision “C”) and its measured basis**
>
> **4.3.1 (observation period):** This mechanism emitted only **W-level** diagnostics (W1090); there
> was **no strict switch**. **4.3.2 (terminal state, user decision A on 2026-10-13):** **The same
> code was promoted to E**—the `W1090` code number remains unchanged, while the emission level was
> raised to `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444);
> **no new E code was added** (two codes for one fact would create parallel representations). **The
> promotion changes only the emission level, not the emission conditions.** This is a **D-class
> destructive item** under D59 (changes externally observable behavior), faithfully registered:
> healthy programs emit nothing (no diagnostic is produced when all obligations have delivery
> markers), so existing corpora and user programs are unaffected (342/342 “difference: empty”);
> however, when a run genuinely encounters an undelivered obligation, the diagnostic is **included
> in `error_count`, rendered as `error[W1090]`, and produces a nonzero exit code**—unlike the
> previous “W-level does not block compilation” behavior. The policy that decouples the code family
> from the emission level has been registered in RFC-013’s “Emission Channel” section (the code
> prefix determines only numbering ownership; severity is determined explicitly by the construction
> site). Mechanism facts (measured): severity comes from an explicit `.severity()`; otherwise, “code
> beginning with `W` → Warning” (`codes/builder.rs:186-203`). The registry’s `ErrorCategory` **does
> not participate** in severity. The exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`); **W1090 follows
> `error_count` because of its explicit error severity (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain’s decision C on
> 2026-10-13; the current red list is empty; see section 4). The reason is reproducible:
>
> - Measured cost of decision A (red list emitted as W): `interface_impl_registry` is populated from
>   the typecheck pre-registered table and is **always non-empty**, so the per-compilation output is
>   a **constant signal** with no discriminating power. Yet it breaks three existing external
>   contracts: “clean programs produce no diagnostics / LSP produces no diagnostics /
>   `warning_count == 0`.” `cargo test --lib --all-features` measured **12 existing test cases
>   turning red**, of which **8 are on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 are under `src/driver/tests/`). The corpus
>   baseline changed in 24/342 fixtures, and the only feasible modification was “filter W1090 out of
>   zero-diagnostic assertions,” which would relax an existing criterion.
> - Decision B (downgrade to Info) would still break 9/12 list-based assertions, and Info is not
>   emitted “according to the adjudicated level.”
> - Measured result of decision C: **zero changes** to existing tests and **zero changes** to the
>   corpus baseline (342/342 “difference: empty, behavior equivalent”). Runtime W1090 becomes a
>   **discriminating regression detector**—the healthy tree remains silent throughout, and removing
>   any delivery marker makes it fire. (At each of the three delivery points, a marker-removal
>   mutation was performed; the CLI `check` path emitted two `warning [W1090]` messages when the
>   Check arm’s marker was removed.)
>
> The red list’s “must not be silent” property is carried by three mechanisms: ① the
> `DriverOutcome.obligations.findings` audit surface (present on every run, including the WBS ID and
> evidence pointers); ② `check-obligations.py`, which prints the red list on every run (currently
> empty) and performs stale-entry detection; ③ this note.
>
> **4. Current red-list state (account closure) and landing records for 3.4.4/3.4.5/3.4.9**
>
> The red list went from 3 → **0**; all three account-closure tasks landed on 2026-10-13:
>
> Note: The line anchors below record the file state **as of 2026-10-22**. P5 (2026-10-26) split
> `checker.rs` into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting
> all line numbers in `checker.rs`; the anchors below no longer correspond to the current file and
> are retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order obtained from `sum_type_variants`); the call
>   site at `:7496` replaces the entire Call with `CreateVariant`. The function’s doc explicitly
>   states that “span keys are unreliable across systems (the two sides interpret the span of a Call
>   node differently), so detect in place”—**wiring a consumer would reintroduce the already-fixed
>   span-mismatch defect**. Deletion surface: the `types.rs` field; collection at
>   `checker.rs:1489-1493/:1565`; the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`; and the `VariantCtorCall` structure in `environment.rs`. Type
>   inference by `infer_variant_ctor_call` and all diagnostics remain **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retained in the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase (the
>   decision point at `expressions.rs:4490-4550`; the sole hit at `:4550` writes
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** at `checker.rs:1558`,
>   which was poured into `TypeCheckResult`. The naming fact still has a single expression: the
>   candidate-name table (in-phase) + the span-keyed decision (cross-stage, `ir_gen.rs:380`
>   construction, `:7632/:7705` consumption); the definition-side mixed names are computed
>   independently by `method_def_ordinals` (`:1828`).
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user decision A; see the
>   reclassification record in this document’s §Obligation Fields table). What was deleted is the
>   **declaration** in the doc comment at `types.rs:47-48` saying “mirrored for consumption by the
>   middle side/LSP”; the field and its in-phase consumption are retained
>   (`operator_interfaces.rs:623` for operator lookup; seven constraint-solving lookups in
>   `inference/expressions.rs`; the read-only accessor at `checker.rs:214-217`). Direct evidence of
>   zero cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not
>   wire up a consumer:** the table’s content is an environment mirror populated by typecheck
>   (always non-empty). If LSP genuinely needs it, register it anew under 4.6.x; creating a consumer
>   for a field that, by design, does not require cross-stage consumption would introduce a
>   fictitious obligation into the ledger.
>
> After the red list reached zero, the **field census was closed by “obligation inventory ∪
> non-obligation field registry (`NON_OBLIGATION_FIELDS`, with reasons)”**—criterion 1 of the gate
> enforces reverse containment end to end (adding a field without registration / deleting a field
> without synchronization turns the gate red immediately). The red-list **mechanism** remains:
> criterion 4’s stale-entry detection and the `--selftest` `stale-red-list` fixture continue to
> cover its detection surface. Any future registration must go through an explicit decision.
>
> Along with the account closure, the accounting was updated: obligation inventory 16 → **13**
> (`INVENTORY` / `Obligation::ALL` / this document’s §Obligation Fields table); the gate’s
> `--selftest` now covers seven fixture categories (adding “unregistered field,” “stale anchor,” and
> “empty red list”), as seen under
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2)** (three hard criteria beyond the sketch)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document’s “§Obligation
>    Fields” table (including the table-header count declaration); any drift turns the gate red. In
>    addition, the `TypeCheckResult` field set must be **exactly equal** to “obligation inventory ∪
>    non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”—adding a field without
>    registration, deleting a field without synchronization, omitting a reason for a non-obligation
>    entry, or overlapping the two tables all turn the gate red immediately.
> 2. **Read evidence**: Every non-red-list obligation must have a genuine read in the **declared
>    consumer file** (`.field_name` member access; comments/tests/strings do not count). This makes
>    “delivery marker ↔ downstream actual read” a hard criterion and addresses the primary risk of
>    mechanism A: drift between the two.
> 3. **Delivery markers**: Every non-red-list obligation must be marked at a delivery point;
>    conversely, red-list entries must not be marked.
> 4. **Red-list stale-entry detection**: A red-list field must have zero genuine reads (the
>    emergence of a consumer turns the gate red), and every entry must carry a WBS ID and evidence
>    pointer. There is also a `--selftest` with built-in negative/positive fixtures: negative
>    fixtures must turn the gate red and positive fixtures must turn it green, proving that the
>    detector is not inert.
>
> **6. Deviations from the sketch above**
>
> 1. The sketch’s `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains held by `TypeCheckResult`, and the ledger records only the “inventory +
>    delivery markers + settlement conclusions.” Copying the data would create two sources of truth
>    (the fields are produced in different places—the checker’s typecheck stage and the ownership
>    layer)—and they would inevitably drift; the cross-layer data plane should not be copied into
>    the orchestration layer either.
> 2. The sketch’s `take()` / “mark consumed” **was not adopted**. It was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. The reason: the obligation and the
>    product are the same data, and `take()` would remove what downstream code needs; moreover, the
>    11 IrGeneration obligations take the form of handing the complete `&TypeCheckResult` to ir_gen,
>    so they cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; all
>    other forms are routed **by level** (from 4.3.2 onward, W1090 is E-level)—the single-file path
>    uses `PipelineError::TypeCheck` (the external-emission arm at obligations.rs:654), while
>    multi-file/embedded record an `OrchestratorError` failure (otherwise `result.errors` has no
>    consumer). The reason: returning `Vec<Diagnostic>` would lose form-specific routing—check
>    diagnostics must remain attributable to individual files.
> 4. The first tier of the three-tier table changed from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is delegated to the static gate
>    (criterion 2 in section 5).
> 5. The second tier’s “promote to E level in `strict` mode” **did not use a strict switch**. It was
>    replaced by **the same-code promotion to E in 4.3.2** (promoted even without a strict switch,
>    because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion changed from “a field appears ≥2 times but has no third-file read” to
>    the four criteria in section 5.
> 7. The two additional tiers not present in the sketch (S1 and the skip exemption) are described in
>    section 2.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table was actually
>    implemented as **20 test cases** in `src/driver/tests/obligations.rs` (first an `#[ignore]`
>    skeleton, then turned green; red-state record: removing the sole settlement point produces
>    `pairs: 0` and six failures). The S1/S4 rows were synchronized with the actual file.
>
> **7. Landing record for 4.3.2 (severity + gate hardening)**
>
> - **Inputs for field-by-field review:** `ObligationSettlement.drained_kinds` (the obligation kinds
>   actually drained in the run, available per run) and the five counts (the conservation identity
>   in section 2); combined with the audit-surface `findings`, these provide the fact of “whether
>   this field has ever been consumed by any entry point.”
> - **Severity: same-code promotion to E has landed** (user decision A on 2026-10-13; see the
>   terminal-state registration and D59 D-class destructive-item declaration in section 3).
>   Implementation: an explicit `.severity(Severity::Error)`; external-emission arms route by level
>   into the error channel. Mutation evidence: removing any delivery marker causes that form’s
>   compilation to fail, and `check` returns a nonzero exit code (`error [W1090]`); after
>   restoration, the healthy state emits nothing (corpus 342/342 with zero changes). The locale text
>   in all six languages states that the W-family code is emitted at error level.
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: The gate now parses `file:line` anchors in **non-obligation
>     registry reasons and red-list evidence**, and validates that “the file can be located
>     uniquely + the line number is in range + the symbol immediately following the anchor actually
>     appears in the anchored file.” An anchor pointing to a nonexistent symbol turns the gate red.
>     Built-in negative fixtures: `tests/baselines/obligations-gate/stale-anchor/` (one for a
>     nonexistent symbol, one for an out-of-range line number). Note: The red list is currently
>     empty, so the criterion’s actual landing points are the **non-obligation registry** and future
>     evidence pointers (the same code path).
>   - **F2/R3 reverse containment**: The gate outputs “the number of `TypeCheckResult` fields − the
>     number of obligations − the number of non-obligation registrations = the difference”; a
>     nonzero difference turns the gate red. A built-in negative fixture exists under
>     `unregistered-field/` (a new struct field without registration).
> - **Informational item F3**: The `SkipReason::UpstreamFailed` exemption is unreachable from real
>   entry points (a unit whose typecheck failed does not retain a `type_result`, so the obligation
>   does not exist); this tier’s semantics are pinned by synthetic `State` cases, and real
>   end-to-end coverage only includes `ConfigDisabled` (t3 T06).
>
> **8. Closure of the 4.3 second-level task (red list cleared + terminal E-level obligation
> diagnostics)**
>
> **8.1 Three-round landing timeline** (each item corresponds to a code file:line; this round
> records conclusions only and changes no code)
>
> | Round                 | Conclusion                                                                                                                                                                                       | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
> | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | 4.3.1 Ledger launch   | `State::assert_drained()` sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                             | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                       |
> | 3.4.4 Account closure | `variant_ctor_calls` field deleted—its declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                              | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                   |
> | 3.4.5 Account closure | `method_overload_ir_names` cross-stage mirror deleted (retained in the in-phase candidate-name table + `overload_resolutions`)                                                                   | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                |
> | 3.4.9 Account closure | `interface_impl_registry` reclassified as an **in-tree fact**: delete the “mirrored for middle/LSP consumption” declaration and retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under decision A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, seven sites in `src/frontend/core/sema/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), and `src/frontend/core/sema/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Severity        | **Same-code promotion to E**: explicit `.severity(Severity::Error)`; code remains in the W family; external emission routes by level into the error channel (first failure takes precedence)     | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/severity decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                    |
>
> **8.2 Closure conclusions**
>
> - **Red list cleared**: The number of entries in `INVENTORY` (`src/driver/obligations.rs:201`)
>   whose `red_list` is `Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs`
>   → 0). The mechanism is **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`,
>   criterion 4’s stale-entry detection, and the `stale-red-list` fixture. The gate outputs “red
>   list: 0 entries—account closed.”
> - **Terminal E-level obligation diagnostics**: The **code number and W-family membership of
>   `W1090` remain unchanged**, while the emission level is `error` (D5 terminal state, user
>   decision A on 2026-10-13). This is a **D59 D-class destructive item** (changes externally
>   observable behavior), registered both in section 3 of this note and in RFC-013’s “Emission
>   Channel” section. **Zero emission in the healthy state ⇒ zero impact on existing corpora and
>   user programs** (corpus 342/342 “difference: empty”; IR snapshots have zero drift; t11
>   independently verified it and t12’s re-review checked it again).
> - **Field census closed**: The `TypeCheckResult` field set = obligation inventory ∪ non-obligation
>   registry (8 entries at `src/driver/obligations.rs:150`, each with a reason based on measured
>   consumption); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: All hardening items registered in this note have landed
>   (F1/F2/F4/F5; see sections 5, 7, and 8). The only two remaining open items are the **known
>   boundary** in 8.4 and t7 F5, which was moved to an issue; neither is an unimplemented criterion
>   of this mechanism.
>
> **8.3 Disposition status found by each acceptance/re-review round**
>
> | Source        | Finding                                                                                                                                                                                     | Status                                                                                                                                                                                                                                                                               |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | t3 F1 / t4 R2 | Red-list evidence consists of textual pointers and becomes silently stale when the replacement consumer is removed                                                                          | **Landed**: gate F1 anchor validation (unique file location + in-range line + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixtures under `tests/baselines/obligations-gate/stale-anchor/`                                                                        |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment; a newly added field without registration silently passes                                                                                        | **Landed**: F2 explicitly outputs the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 scope reconciliation; negative fixture `unregistered-field/`                                                                                                             |
> | t4 R4         | The W-only versus E-promotion policy was not registered                                                                                                                                     | **Landed**: sections 3 and 7 of this note + RFC-013 “Emission Channel” + the W1090 help and error_output examples in all six `locales/*.json` files                                                                                                                                  |
> | t7 F5         | Dead-code analysis miscounts overloaded-method references (a legal overload is falsely reported as `W1005`)                                                                                 | **Moved to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—outside this second-level task                                                                                          |
> | t11 F7        | Under the gate’s `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                                           | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                         |
> | t11 F8        | Validator-side tooling note: mutation-driven validation once wrote into the main repository (immediately cleared with `git restore`; the affected round’s output was invalidated and rerun) | **Informational item** (not a delivered-artifact defect)                                                                                                                                                                                                                             |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document’s table header diverge from `INVENTORY`) turned the gate red                                                       | **Informational item / resolved**: the frozen commit passes self-consistently; “committing an intermediate state directly turns the gate red” is precisely the intended behavior of F2/F4                                                                                            |
> | t12 F12-1     | This document states the struct scope as `16-73`, but the actual scope is `16-76` (three comment lines added in t9 without mechanical reconciliation)                                       | **Landed + mechanized**: the scope was corrected to `16-76`, and gate **F5** was added (scope anchor; a mismatch turns the gate red). Similar numbers were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                       |
> | t12 F12-2     | Settlement in multi-file form once displaced the first failure (the type error in `zzz.yx` was overwritten by W1090)                                                                        | **Fixed** (the captain amended the commit message to explain this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; the regression is caught by the **existing pinned case** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1’s scan surface**: Anchor validation covers only the text of **registered entries**—the
>   reasons in the non-obligation registry (`NON_OBLIGATION_FIELDS`) and red-list evidence.
>   **`file:line` promises in module-header prose comments are outside the scan surface** (no
>   registered entry is currently affected). If an anchor promise is written in a module header in
>   the future, it must either be covered by **manual review** or by extending F1’s scan surface;
>   one of the two is mandatory, and it must not be left unattended by default.
> - **The gate’s static boundary**: Set-level validation of delivery markers lives on the static
>   side (criterion 3), while removal of an individual arm is covered by **runtime settlement**—the
>   static + runtime two-layer structure is the intended design, not a gap (bidirectional validation
>   through t11’s M11A/M11B/M11C mutations).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer PlanId contract landed):** Two of
> the 13 obligations above—**the two span-keyed contracts** (`release_plan` /
> `overload_resolutions`)—have been changed from **span-keyed** to **structurally identity-keyed**
> (`PlanId`) under RFC-039 **D20**. This note registers the landed form, criteria, classification
> basis, and known boundaries; definition lines are still reconciled by gate **F4** (the line
> numbers in the two table rows above are current code facts).
>
> **1. Landed form (carrier choice A: `PlanId`)**
>
> The key is a **structural identity** that travels with the AST node, rather than relying on the
> positional coincidence of “both sides computing a span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → ownership of the release plan and its consumption) and `Expr::Call.plan_id` (call-site
> identity → production and consumption of overload resolutions); `Expr::call` is the repository’s
> only call-expression constructor, and all synthetic calls outside the parser use `UNPLANNED`
> (**cannot be consumed**; see section 2).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the sole
>   consumption entry point is `ReleasePlan::consume(stmt)`—**the read and the accounting occur in
>   the same invocation**; `ir_gen.rs:360` reads it and `:3103` uses it. Statement identity is
>   supplied by `walk_stmt`’s set/restore operations plus the CFG-node-to-statement-identity table;
>   a scope-exit Drop is attributed to the scope’s last statement.
> - `overload_resolutions`: `OverloadResolutions` (the table and consumption ledger are **the same
>   object**), produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with
>   dispatch passing through the `PlanId` from `Expr::Call { plan_id }`), and consumed at
>   `ir_gen.rs:7697` (with bindings) / `:7772` (without bindings) through `OverloadResolutions::get`
>   (a hit is accounted for).
>
> **2. Consumption-required settlement (reuse the 4.3 ledger; do not create a second system)**: The
> consumption ledger has a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), and each of the two tables owns an instance. When
> `generate_module_ir` returns successfully, both ledgers are marked “completed one full pass” **at
> the same point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload resolutions); the
> three-state mapping is unique (`consumption_state` in `src/driver/obligations.rs`), and the sole
> settlement point connects one row for each of the two obligations—a nonempty difference produces
> an **E-level W1090** (same-code promotion to E; see section 3). The exemption criterion is
> attached to **the ledger’s own completion marker** (not to whether `unit.ir` exists), so the Check
> form’s removal of the IR product does not constitute an exemption.
>
> **3. D59 classification basis (C class, red before green)**: Before the change,
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> candidate was discarded, and execution fell back to the last binding in definition order). After
> the change, the same program fails at compile time with
> `error [W1090] ... produced key was not consumed`. The fix chosen is **loud** rather than “put the
> resolved name into the call with named arguments”: the latter would lose the named argument and
> merely replace one silent failure with another (the terminal-state semantics are described in
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition: `Drop` instructions are no-ops in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the 342-corpus
> differential with an empty result is empirical evidence of this fact.
>
> **4. Criteria and evidence**: D41’s same-form criterion `test_release_plan_spans_consumed`
> (settlement by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + evidence-surface
> floor**: the number of files that produced a resolution `≥ 1`, the number of genuinely consumed
> entries `≥ 2`, no allowlist). The nine pinned resolution forms are listed in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> type-body binding / `?` operand / named only / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven validation: shifting a consumption key by
> one position ⇒ criterion red + real program `error [W1090]` + corpus-gate difference; restoring it
> returns the tree to green. Gate values: `check-obligations.py` PASS, 342-corpus differential
> empty, `cargo test --lib` 2712 passed / 0 failed (IR snapshots have zero drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: The **binding-free
> consumption point** at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test
> surface (method declarations uniformly take the binding-bearing consumption point through
> `register_type_bindings`), so the keying correctness of that branch has **no red state with which
> to falsify it**. The reason for retaining it is that it is the symmetric counterpart of the
> binding-bearing path; deleting it would rewrite the “method call without `type_bindings`” form
> into silence. This was registered by independent validation (t5 F-1) and re-review (t6 F2); **no
> new task is opened**.

### 3. Unified Driver: a single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (in-project LSP; results filtered to the target file)
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

**`stages()` may only be one of the six predefined combinations above** (one per `ProgramKind`);
callers cannot freely supply an array. This is the key constraint preventing the `Program`
abstraction from degenerating into “a free parameter that can accept anything,” and it is asserted
by `test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not the stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must emit a diagnostic; see below
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

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed):** The four implementation
> deviations from the sketch above are semantically equivalent and are recorded for reference:
>
> 1. The three-state `StageOutcome` from §4 is not returned by stage arms. It is expressed through
>    `State` failure flags + the `Aggregation` gate (Continue / Warn / Abort semantics are
>    unchanged, eliminating per-arm boilerplate).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “Avoid
>    mutable global state in Driver” determination in §5 supersedes the sketch field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted—the zero-diff criterion of S2, “deliberately fix no bugs”; external
>    emission follows with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is
>    the sole new caller (L1→L2 is an allowed direction); the existing orchestrator call sites will
>    be moved in 4.2, after which the question can be revisited.
> 5. In multi-file form, ProofExecution is an independent stage that executes **after all
>    typechecking** (A1, user decision on 2026-10-09): the original `compile_project` interleaved
>    proof within the per-file typecheck loop (file N’s proof preceded file N+1’s typecheck). With a
>    single failure, the two are byte-for-byte identical; in the multiple-failure case of “file 1
>    proof failure + file 2 typecheck failure,” the first-reported error changes from the proof
>    error to the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries products through separate channels by `ProgramKind`: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1). The external error contracts
>    of the individual entry points (`PipelineError` / `OrchestratorError`) are not part of the type
>    surface that the Driver can unify.
> 7. The Check channel landed in 4.2.2: `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; a clean file has an empty Vec—the existing
>    `check_project` contract). The Discovery product is extended to a three-part tuple of “file
>    set + used_by edge set + shadowing events” in State (the MultiFile form has no downstream
>    consumer, so these are recorded but not emitted); parsing on the check path is reduced from two
>    passes to one by the C3 topology.
> 8. The Lsp form landed in 4.2.3 (per-file-source routing decision, user ruling on 2026-10-09):
>    parse failures in on-disk files use the Check plan B (collect + exit the compilation unit—both
>    sides degrade consistently); parse failures in the edited buffer retain a partial AST and
>    continue to typecheck (the editor philosophy—an intermediate state while typing should not make
>    semantic features disappear). The Discovery arm lets Lsp override stale on-disk content with
>    buffer source code; Typecheck in Lsp checks only the target file (other units exist only to
>    provide signatures for the registry); the within-file diagnostic-order normalization principle
>    extends to LSP (typecheck diagnostics → W-code warnings → proof errors). The old behavior—where
>    LSP hard-aborted on parse errors in unrelated on-disk files and the handler silently fell back
>    to the single-file path—has been fixed by this step.
> 9. The Embedded form landed in 4.2.4: `Program` gains a `shared_registry` field—the embedded std
>    module is a child compilation, and its registry is an **input**, not a product
>    (`EMBEDDED_STAGES` has no Registry stage). The #94 SymbolTable-sharing contract is thereby made
>    explicit as a program declaration rather than something “the caller must remember to pass the
>    same registry.” Error-path text was normalized from `<std.test> (embedded std)` to the unit
>    virtual path `<std/test>` (compiler-internal error surface only; no test pins it). At this
>    point, all four orchestrator entry points have moved to the Driver.
> 10. Standalone check was unified in 4.2.5 (decision A + IR-stage decision, 2026-10-09): The Check
>     form faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive only errors, preferring omission
>     over false positives); relative `use` resolution follows the importer’s directory (aligned
>     with rustc’s single-file `mod` behavior). The CHECK stage table gains GlobalSlotAlloc +
>     IrGeneration: the old standalone path (the full pipeline) already ran IR generation, and
>     errors such as E3019/E1014/E1015 are produced only by ir_gen (confirmed by the runner gate).
>     IrGeneration in the Check form is check-only and does not consume the IR; the Script/Bin split
>     is expressed through the `module_key` being None/Some (the existing E3023 switch,
>     ir_gen.rs:1660). Monomorphization is not included in CHECK (it produces only
>     resource-limit/internal errors, with no corpus dependency). **〔Withdrawn on 2026-10-10 after
>     empirical verification in 3.4.8〕**—“Zero corpus dependency” was an evidence blind spot caused
>     by the corpus’s lack of pathological recursive fixtures. E3005 is the sole compile-time guard
>     for this class of program. See note 14. Incidental fix for a latent defect: the Registry arm’s
>     repeated registration harvesting for the embedded std unit could displace the native half
>     already merged through `with_std()` (causing false E1043)—the embedded unit now skips repeated
>     registration.
> 11. The LSP single-file fallback was unified in 4.2.6: the manual
>     lex→parse→check_module_collect_all sequence in `run_diagnostics` was removed and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     In the SingleFile+CollectAll form, Parsing collects all parse errors, retains a partial AST,
>     and continues to typecheck (the editor philosophy extended by the 4.2.3 decision; the stage is
>     not marked failed, because otherwise topological skipping would prevent typecheck from ever
>     running); Typecheck dispatches the `check_module_collect_all` free function. The LSP
>     single-file path now adds proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and
>     IR-level errors; lexical failures now report genuine diagnostics—the old synthetic “E0001
>     lexical error” text disappears with the deleted sequence, and the genuine diagnostic provides
>     an exact span with no loss of information.
> 12. `check_module` / `check_module_collect_all` were verified line by line in 4.2.7 (mandatory C5
>     item): the two entry points had already converged independently—at the mod.rs layer,
>     `check_module_inner(ast, env, collect_all)` is a single Boolean; at the checker layer,
>     `check_module_impl(module, collect_all)` is likewise a single Boolean. The only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`; pass-3 and drain modes both run,
>     with the difference carried by whether `collected_errors` is empty. The five collection sites
>     in statements.rs (function body/use/for/block/while). Verification also proved and fixed a
>     duplicate-diagnostic defect: collection sites placed the first error in both
>     `collected_errors` and the Err-return channel, and pass-3’s `add_error` on Err produced two
>     identical diagnostics (code/span/message; up to three when nested). Two independent mechanisms
>     were also found: annotation-validation signature parameters being visited twice along with the
>     whole annotation (E1003/E1103), and the ownership layer emitting twice at the same point
>     (E2014/E2018). The fix was applied at the module-result boundary: deduplication by (code,
>     span, message)—diagnostics are positional facts, and duplicates carry no information.
>     Duplicate copies were removed from 63 corpus entries, with zero drift in run columns and exit
>     codes. The “duplicate” in `borrow_conflict_err` is a baseline-triplet-format artifact that
>     omits the column number—the two E2018 diagnostics are at different columns and are both
>     legitimate.
> 13. The wasm playground landed in 4.2.8: `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + the Driver through `compile_playground`; the `Compiler`
>     wrapper layer no longer participates. Error text is aligned byte-for-byte with the old
>     `CompileError` Display prefix (`Parse error:`/`Type error:`/`Internal error:`), with zero
>     visible difference in the playground UI. WasmPlayground and SingleFile share
>     SINGLE_FILE_STAGES + FailFast (declared in 4.1; from this step onward there is a real
>     producer).
> 14. Multi-file monomorphization and the CHECK stage table were corrected in 3.4.8 (user decision
>     on 2026-10-10, **partially withdrawing the mono exclusion in note 10**). Empirical evidence
>     disproved the claim that “mono produces only resource-limit noise”: pathological generic
>     recursion (`f(x)=f([x])`) caused the compiler process to overflow the stack on the multi-file
>     path without mono (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime
>     error), and the mono depth gate is the sole compile-time guard. The standalone check path
>     before 4.2.5 already ran the full pipeline and therefore mono; excluding it caused a silent
>     coverage regression (“zero corpus dependency” proves only that the corpus lacks pathological
>     fixtures). The “type-erasure fallback” assumption was invalidated by the RFC-033 reflection
>     decision—`^^List(Int)` needs the real identity of an instantiated type, so monomorphization is
>     the foundation of reflection, not an optional optimization. Four design points: ① typecheck
>     produces an instantiation_request for qualified calls (`lib.f(x)`, in FieldAccess form), with
>     generic_id using the qualified name and sharing the merged IR namespace (currently only bare
>     Var is recognized, expressions.rs:2455); ② the `Stage::ALL` topology changes to Linking before
>     Monomorphization, and the MULTI_FILE stage table gains Monomorphization at the end—it consumes
>     merged_ir and aggregates requests from all units (qualifying the deferred bucket’s
>     containing_fn), elevating mono from a per-module pass to a whole-program pass. The single-file
>     stage table remains unchanged (without Linking, the subsequence property is preserved and IR
>     snapshots have zero drift); ③ CHECK gains Linking (pure merge—E3020 entry validation remains
>     in RoleClassification to avoid duplicate reporting) and Monomorphization (check-only, like
>     IrGeneration in the Check form); ④ resource protection covers cross-unit mutual recursion
>     through whole-program BFS. The Monomorphizer itself is unchanged (its ModuleIR + request-set
>     contract is exactly the merged-IR form).
> 15. 3.4.8 R1 (①) landed on 2026-10-10 (a9438007). Qualified-call instantiation collection was
>     implemented as designed in #14: `ExpressionInferrer` receives a module-qualified key table
>     (the three `use` arms—ordinary / alias / item-by-item alias, with only SubModule-class exports
>     eligible), and `callee_generic_name` is joined through `SymbolTable::qualify` using the same
>     source of qualified names as the merged IR (currently only bare Var is recognized,
>     expressions.rs:2455); the qualified-call first path’s arity criterion was tightened (when the
>     parameter table cannot be resolved, it falls back to the second path and obtains actual
>     arguments from the signature, preventing false E3018 matches against same-named functions in
>     other modules). Implementation empirically proved and fixed **three mono-side asymmetries** in
>     the same round: (1) The deletion-key set (generic-name set) and rewrite-key set
>     (containing_fn + name + span triplet) had mismatched granularity—a call site whose actual
>     arguments could not be resolved degraded into a symbolic request in the deferred bucket; if
>     the containing function was not generic, the bucket never drained, while the original had
>     already been deleted by name ⇒ dangling call (runtime E6006 evidence at `list_ops.yx:53`). The
>     fix restores the gate `restore_generics_with_uncovered_call_sites`, using
>     `build_call_site_map` as the sole source of truth to determine coverage and preserving
>     originals at uncovered sites. (2) `HashSet` iteration order during restoration made the
>     function-table order nondeterministic (t2 re-review F1) ⇒ switched to `Vec` sorted by name,
>     producing byte-for-byte reproducible output. (3) TailCall / MakeClosure forms outside the
>     rewrite surface still looked up mapping keys (despite the comment claiming conservative
>     behavior) ⇒ introduced the single-point decision `call_form_is_rewritten`; all non-rewritten
>     forms count as uncovered. Each has a pinned test (red-state empirical verification before the
>     fix proves discriminating power). The corpus differential baseline was regenerated on the same
>     commit with exactly one changed line (`remove_at` frame gained `(int64)`). **Commit-strategy
>     note:** When typecheck collection and the mono fix were split into two commits, the
>     intermediate tree containing only the former triggered a pre-existing `stdlib_docs` failure
>     against HEAD (`list.is_empty([])`, same E6006 family). The two halves are each other’s
>     green-light prerequisite and were therefore committed atomically.
> 16. Five decisions were registered on 2026-10-10 (after R1 closure): (1) **D2 decision
>     A**—user-module exports carry `Export.type_params` together with
>     `generic_fn_type_params_snapshot()`, structurally identical to the std path
>     (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified-call monomorphization, while signature inference is demoted to a fallback
>     (incorporated into the R2 scope). (2) **HKT is live on the declaration side but broken on the
>     call side** (inferred type arguments E1002 / explicitly supplied type arguments E1010),
>     registered as WBS 3.5 (three third-level tasks). (3) The deferred bucket never draining in a
>     non-generic container was incorporated into acceptance sub-item ③ of 3.4.8: the bucket must
>     drain or fail loudly; it must not silently remain unspecialized. (4) Closure-variable calls by
>     name produce runtime E6006, registered as WBS 3.6; semantics: (i) support indirect calls /
>     (ii) reject at compile time. **Decision (i) was made**: support indirect calls—the `CallDyn`
>     mechanism already exists (`ir_gen.rs:7945`, closure-expression path); the break is that a
>     `Var` callee is downgraded to a by-name `Call`, even though typecheck already accepts the
>     program. (5) The tradeoff criterion was fixed—rejection reasons may be based only on
>     correctness and readability. “Large change surface / new mechanism / high cost” do not
>     constitute rejection reasons; this was recorded in the second part of `coding-rules.md` and
>     the HOWTO self-check list. Tradeoffs requiring a user decision must follow the four-part
>     format in `AGENTS.md` under “Requesting a user decision (mandatory)” (situation + evidence /
>     pros and cons / explicit recommendation + cost / explicit naming of breaking changes).
> 17. 3.4.8 ② landed and the F1 root cause was found on 2026-10-11 (375789d2 + d922a551). ② was
>     implemented according to the design in #14: `Stage::ALL` changed to Linking before
>     Monomorphization, and the MULTI_FILE table gained mono at the end. The Linking arm aggregates
>     whole-program requests **before taking each unit’s IR** and qualifies `containing_fn`
>     (otherwise deferred-bucket keys differ from those of the merged IR, and the bucket never
>     drains). **① closure (D2 decision A):** user-module exports carry a declaration-time
>     type-parameter snapshot through `Export::type_params`, structurally identical to the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically found and fixed during
>     implementation**: (a) **F1**—`qualify_names` rewrote only function bodies; `ir.init` was never
>     visited, so in multi-file form function definitions were qualified while top-level module
>     calls remained bare names ⇒ runtime E6006 (**reproduces even without generics** and is
>     unrelated to generics). The fix makes `init` reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) The mono-side “whole-program unique suffix” heuristic and silent
>     `continue` on a miss were removed; a miss now produces a **loud E3018** (ambiguous silent
>     mismatching is worse than not resolving). **Independent validation caught and fixed a
>     regression**: t6 found that a named import (`use lib.{pick}`) followed by a bare-name call was
>     falsely rejected with E3018 (the IR instruction name was the alias-qualified name, while the
>     request name was bare; the aggregation side qualified only by unit key) ⇒ the aggregation side
>     now **directly calls** ir_gen’s `import_aliases_from` and `resolve_call_target` (extracted as
>     a public single point, with one producer and one consumer), and a matrix-pinned test covers
>     both forms of named import. **Archive**: the blank `DefId` in the `init` section was
>     empirically confirmed to be a “consistency blank” rather than a live defect (the codegen name
>     fallback can resolve it, and all four forms of init call were verified in practice). It was
>     registered under WBS 3.7.4 to close the policy split between “function bodies use def” and
>     “init uses names.”
> 18. The init-section DefId policy split was closed in 3.7.4 (2026-10-11, 38bd4b9e): `assign_defs`
>     was extracted into the single-point implementation `resolve_static_targets` (Call/TailCall
>     string operands + MakeClosure), and function bodies and `ir.init` share both the
>     implementation and the `resolve` closure. After regenerating 208 IR snapshots, only 309
>     `(def=_)`→`(def=dN)` pairs changed (other-lines 0; no `(def=_)` remains in any init section
>     across the tree). Codegen’s primary DefId path and its name-fallback state produce identical
>     translation results and execution values item by item (all four forms of init call were
>     empirically verified), with zero drift in the corpus baseline. The probe was inverted and
>     strengthened under the new policy (identity + exact index + execution value; a fallback in
>     init coverage turns it red). **Erratum (2026-10-11):** The commit IDs above were rewritten by
>     `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396 correspond, in order, to the
>     post-rebase 375789d2/d922a551/38bd4b9e. The pushed dev history uses the new IDs; the old IDs
>     were local temporary IDs and never entered the remote history.
> 19. 3.4.8 ③④ landed and were verified on 2026-10-11 (8ecf56f93). (The design basis for ③④ is in
>     #14; ② landed in #17, which is its closure.) **Implementation**: the `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Linking in Check performs only a pure merge
>     (aggregating requests + `link_module_irs`), with the entire entry-validation block left to
>     RoleClassification; `ir_generation_check` retains `us.ir` after generation (memory only); mono
>     failures enter the collection channel in Check, while MultiFile retains `fail_with`;
>     `into_outcome` first calls `take_check_diagnostics` and then checks for failure (structural
>     failures no longer clear already-collected diagnostics). **Verification**: four rounds of
>     independent validation (t3/t4/t10 + the t5 cross-form matrix) and one re-review (t6) all
>     passed; the corpus differential was byte-identical for 340/340 entries, IR snapshots had zero
>     drift, and the test count rose from 2643 to 2656. **Existing gaps found and fixed in the same
>     round**: F1 (on the rootless multi-file check path, the request name and IR definition key did
>     not share a source ⇒ silent failure to specialize; see the landing note in the WBS 3.4.8 row)
>     ⇒ the aggregation side now performs an exact reverse lookup using the actual definition key
>     rather than a heuristic; the first version of the fix was the same exact reverse lookup
>     (`flat_defs`); **same-day erratum**: this bridge was deleted during the regression fix, and
>     the root cause was instead decoupled by giving `module_key` a single dual role
>     (`script_mode`), so all three use the same source of truth: `{unit key}.{name}`. The
>     regression (the CI documentation-example gate had 10 failures across 258 blocks → 0 failures)
>     is located and explained in the landing note in the WBS 3.4.8 row. **Archive (existing, not a
>     regression from this round)**: E3005’s help text is inconsistent with the resource-protection
>     semantics; the multi-file E3005 span label is misaligned (reproducible with the HEAD binary);
>     type-parameter-position dependence exists on the collection surface (assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and the diagnostic surface was closed on 2026-10-12
>     (809b9b936 + 8605a081b + two subsequent commits). **3.9.1 (execution-layer guard reachable)**:
>     Interpreted calls switched to an **explicit stack-driven**
>     model—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` push frames and return control
>     to the outer `step_one`; each frame carries `ReturnTarget{Host|CallerSlot|Discard}`, and the
>     return writes the value back to the caller’s slot. Multiple calls in one instruction continue
>     through `pending_calls` in the frame. Baseline measurement showed each interpreted call
>     consumed **2544 B** of native stack, with the crash threshold at `count_down(394)` on the
>     Windows main thread’s 1 MiB stack. After the fix, 20,000 recursive levels run successfully on
>     a 512 KiB-stack thread (approximately zero per level). The guard (`push_frame`) is therefore
>     reachable from every entry point and at every `max_stack_depth` setting;
>     `ExecutorError::StackOverflow` now carries the actual limit (previously
>     `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (truncation of deep-stack
>     backtraces) was incorporated into this task by user decision** (same file and inseparable at
>     file granularity): the backtrace now shows the first 16 frames + an elision count + the final
>     8 frames; shallow-stack frames remain unchanged. **3.10.1 (native recursion with no
>     compile-time gate, #447)**: Locate before fixing—the crash point was recursive
>     `MonoType::type_name` (**328 B/level**, SP probe + step-by-step critical-point determination);
>     the same path also has derived `Clone` (~400 B/level), a deep copy in `substitute`, and
>     `type_name_hash` (164 B/level), plus `collect_generic_type_refs`. After converting all four to
>     worklists and removing one full-request deep copy per level, pathological generic recursion
>     with `mono.max_depth=5000` produces **E3005 with the real limit**, and no longer overflows the
>     stack. **3.11.1 (preserving task-boundary error codes, #448)**: The task result’s error side
>     changed from a value to a **structured representation**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), with values
>     restored across the boundary; E6001/E6003/E6004/E6008 inside tasks are word-for-word identical
>     in Standard and Embedded modes. Two **pub API payload-shape changes** are explicitly marked
>     `BREAKING CHANGE` in the commits. **Commit-strategy note:** 3.9.1 and 3.11.1 are **inseparable
>     at file granularity**—`interpreter/executor/executor.rs` carries both the trampoline and the
>     error-channel producer/consumer, while the `TaskFailure` definition in `runtime/engine.rs`
>     means that stashing only one side cannot compile (the pre-commit process compiles a staged
>     subset and rejected it twice). Following the same principle as #15, the two changes were
>     committed atomically as one commit. **Known boundaries and follow-ups**: Derived `Drop` (~75
>     B/level) ⇒ `mono.max_depth ≳ 13000` may still crash; the E4012 (constant-evaluation) gate
>     currently bears the native-recursion burden (~3696 B/level, default 256 levels ≈ 924 KiB), has
>     zero call sites, and is unreachable in production. Future wiring must close the same family of
>     issues; the Standard-mode task interpreter is constructed with the default `ExecutorConfig`,
>     so its E6004 limit is permanently 1024 (configuration propagation is registered as WBS 3.12,
>     user decision B: a separate team); the test-runtime budget is assigned to separate issue #449
>     (pathological fixture 76 s, clippy dual target, full commit run), outside this RFC’s
>     construction scope. **Corpus-directory stratification (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134), totaling 212 fixtures, were split into 23
>     category-based subdirectories; 88 IR snapshots were moved to mirrored paths. The corpus
>     baseline was regenerated on the same commit (342 lines; independent recomputation compared
>     line by line: only the `file` field and two new changes), and 41 snapshots were regenerated
>     because their file-header paths became longer; line-by-line verification found **only
>     span-offset changes** (zero differences after offset normalization).
> 21. The LSP semantic-data pipeline was unified (4.6.1, 2026-10-16). Diagnostics and semantic data
>     are produced by the **same compilation**, and cross-file references in a project carry
>     `resolves_to` pointing to the actual definition. The old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics used the unified Driver pipeline, while
>     semantic data used a manual lex→parse→single-file typecheck in server.rs (with no module
>     registry, project-local `use` resolution failed, no reference events were produced, and
>     cross-file navigation/rename/find-references silently returned nothing—the silent failure
>     registered as D55 was closed by this task). **Implementation**: (1) `Export` carries
>     `definition_span` (harvested by `extract_module_info` from the AST identifier node, with the
>     same source and value as the typecheck definition event’s `def_span`; native std is always
>     None), and `ModuleInfo` carries `definition_file` (filled by the Registry arm from
>     `DiscoveredFile`); (2) when the checker drains reference events, it uses
>     `import_def_locations` to redirect a cross-file imported item’s `resolves_to` from the use
>     line to the actual location in the definition file (std imports are not registered and
>     continue to point to the use line); (3) the checker’s module identity changed from the unified
>     `"<module>"` to the unit file path (this value is consumed only by the semantic-data surface,
>     with zero diagnostic consumers—verified across all four `env.module_name` consumers); (4)
>     `DriverOutcome` gains a `semantic_db` channel—populated by CollectAll forms (the Lsp target
>     file regardless of diagnostic success + the SingleFile fallback), after which the Lsp arm tail
>     injects top-level export definition surfaces for non-target files (`external_definitions`, so
>     even unopened files can be navigated to; type/submodule exports are not injected because
>     typecheck-side type registration does not produce definition events, preserving the same
>     absence on both sides); (5) orchestrator gains `analyze_source_in_project`
>     (`check_source_in_project` delegates to it and takes only the diagnostic half); (6) the
>     LSP-side `analysis.rs` single-compilation entry point replaces the manual pipeline
>     (`server.rs`’s `update_semantic_db` is deleted), and World keys are unified as normalized file
>     URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is idempotent normalization, with drive-letter
>     case and %XX encoding differences normalized to the same key). **Incidental fix (D59 routing
>     class B—old behavior silently did nothing and did not change the externally visible behavior
>     of any previously successful program)**: cross-file navigation/rename/find-references for
>     files in a project no longer silently return nothing but now resolve to the actual definition;
>     on lexical errors, the old manual pipeline deleted semantic information, whereas the new path
>     retains partial-AST coloring (the 4.2.6 editor philosophy extended); when the target file has
>     type errors, semantic data is no longer discarded along with `type_result`
>     (coloring/navigation no longer disappear while the user is typing). **Pinned cases**: six at
>     the driver level (target-file path key / cross-file `resolves_to` +
>     external-definition-surface round trip / std-import fallback to the use line / error-file
>     retention / single-file fallback path key + error retention), and one LSP e2e case
>     (empirically red before green—before the fix, navigation returned None), plus four
>     URI-normalization cases.

**Transformation of the ten entry points (specified individually):**

| Entry point                                                | After transformation                                                                                                                                                                                      |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to the Driver                                                                                                           |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                   |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduced to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                             |
| `check_project` (`orchestrator.rs:273-399`)                | Reduced to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                               |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver invocation + filter the result to the target file                                                                                                                                    |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                 |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                    |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                       |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In a project, use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                    |

**The duplication between `check_project` and `compile_project` is approximately 40% shared / 60%
divergent.** What they share is vendor-consistency verification, file discovery,
`build_registry_from` (deleted on 2026-10-26 when it had no callers; the historical account is
retained here), `all_method_bindings`, the parsing loop, per-file checker setup, and result
handling. What they do not share is items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40%
shared skeleton is precisely the Driver’s value range**—all 60% of the divergence consists of either
“a different set of stages” or “a different aggregation mode,” and both are fully expressible
through the two `Program` fields.

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
returned `Skipped` on its own, the information about “why did I not run?” would be scattered across
12 stages and could not be centrally audited. Instead, `dispatch` determines it at the beginning of
each iteration based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect targeted by this
document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip” that
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every case where “X was not done, so Y did not run” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                           | Text direction                                                                                                                         |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                                    | “Stage Y did not execute because upstream stage X failed”                                                                              |
| Caused by an empty prerequisite obligation                       | “Stage Y did not execute because there were no pending obligations (normal)”—**severity = Info and not included in the warning count** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y did not execute because it was not enabled by configuration”                                                                  |

The second category is critical: it distinguishes “normal skip” from “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`’s `warning_count` (the non-blocking contract
depends on this count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

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

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both project
files and single files within the same process, while the CLI’s `run` and `check` are two
independent invocations. Placing it on the Program avoids mutable global state in the Driver.

**Note the semantic difference between the current two typecheck branches:** `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not only in “early return or
not”—they are two different checker entry points. After unification, a single implementation should
be driven by the `Aggregation` parameter rather than retaining two functions (**this is an
additional item requiring verification in this document; see “Known Limitations and Risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                      |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| New `driver/` module                        | **None.** It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                             |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                          |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types:** What moves into the ledger is **settlement
responsibility**, not the module that owns the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the types of the remaining
fields stay in their original locations; `driver/obligations.rs` holds only the aggregate container
and `assert_drained()`. L3 consumers (for example, `ir_gen` reading `release_plan`) continue to
receive concrete field types through parameters and **must not `use crate::driver`**—consistent with
the driver’s no-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2), `middle` (L3), and `backends` (L4); **L2/L3/L4 must not depend on
`use crate::driver` in reverse**. A `Driver` appearing in a `TypeChecker` import is considered a
violation and is blocked by RFC-039’s proposed `scripts/ci/check-module-boundary.py`.

### Runtime Behavior

| Scenario                            | Before                                      | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | Five stages                                 | The same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone input—it uniformly uses `compile_project` with the MULTI_FILE stage table, and Script semantics are driven rootlessly; this row now describes only the library path `lib::run_file` / `Compiler::compile`)                                                                                                                                                                               |
| `yaoxiang run` (multi-file)         | No proof_execution and no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel”; Check / Lsp / MultiFile share one implementation, with no second criterion. The FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` and hands them to that arm for emission) |
| `yaoxiang check` (multi-file)       | No proof_execution                          | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| LSP (in-project)                    | No proof_execution                          | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Z3 not installed + single file      | `predicate.rs:35` **panics**                | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| Z3 not installed + multi-file       | Silently skipped                            | Unified as above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

**The only intentional behavioral break** is replacing the `.expect()` at `predicate.rs:35` with a
diagnostic return. This is consistent with RFC-027 §8’s positioning that the compiler “must not bind
to a specific solver” and with the existing contract at `backend.rs:60` (“callers should
conservatively degrade”)—the current implementation contradicts its own contract documentation.

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

| File                                          | Line range                                      | Change                                                                                                                                                                                                                                                                                                                                                                                        |
| --------------------------------------------- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                                         |
| `src/lib.rs`                                  | `140-147`                                       | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                                 |
| `src/lib.rs`                                  | `154-167`                                       | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                    | `141-227`                                       | Replace the `Pipeline::run` function body with `Program` construction + a Driver invocation                                                                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                    | `275-278`                                       | Move dead-code analysis out and make it the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                    | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                                          |
| `src/frontend/pipeline.rs`                    | `381-389`                                       | Move monomorphization out and make it the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`         | `450`                                           | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`         | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                       | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                                              |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                       | Delete the manual stage sequence in `run_diagnostics` and invoke the Driver                                                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                       | Make `check_files_with_diagnostics` uniformly use `Program { kind: Check }`                                                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                       | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                                                |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                                           |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                         | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                                        |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                                                 |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/core/sema/checker.rs`           | `1244`                                          | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                                                |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; delete “a lower layer fails, so the upper layer does not run” (there is no short-circuit)                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                         | **Under D58:** add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure access point; change the three consumers (`predicate.rs` / `checker.rs` / `ownership.rs`) to use the shared access point. The bare `&'static` reference form is infeasible because `dyn Solver` is not `Sync`; the original formulation is superseded by D58 |

**Deleted:** `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged:** The production-side logic of the `Unproven` branches (now split among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is in the consumer, not the producer**. Changing the producer would conceal the
architectural defect. Also unchanged: `Cargo.toml` and every `#[cfg(target_arch = "wasm32")]` branch
(wasm-branch reachability determination is assigned to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                                     | Handling                                                                                                                                                        |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                       | **Breaking:** constraints that previously passed silently now report E4018        | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix; release in batches, and record the expected diagnostic-set change in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                     | **Breaking:** programs that previously compiled successfully now produce warnings | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                  |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking:** the diagnostic set grows                                            | W first, then E; perform S2/S4 in two steps                                                                                                                     |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement:** no more crash                                                    | No break                                                                                                                                                        |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                                     | These two paths do not pass through proof_execution                                                                                                             |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                          | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                 |

---

## Implementation Points

This document corresponds to **P3 (fix correctness defects)** and **P4 (stage contracts and unified
Driver)** in RFC-039’s global stage sequence. S1-S5 below are the implementation order within this
document. **The acceptance criterion for every step refers to category C2 (orchestration changes) in
the [equivalence-criteria document](07-equivalence-oracle.md)**: the **diagnostic set** is identical
at every entry point, and corpus behavior is identical.

### S1: Establish the criteria (red first)

**Prerequisite; cannot be skipped.** The defect criterion must first be written as a failing test.

| Deliverable                                                                                                                                                            | Acceptance                                                                                                          |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                          | **Must be red.** If it unexpectedly passes, the defect analysis in this document must be reviewed again             |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (decision D48—do not mix it into the single-file corpus tree; project fixtures containing `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file corpus versions can be compared                               |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                                     | Mark it `#[ignore]`; turn it green in S4 (red-state record: removing the sole settlement point produces `pairs: 0`) |

**Rollback point:** No code changes; additions to tests only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                                | Acceptance (C2)                                                                                           |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs`   | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array           |
| Change `pipeline.rs:141-227` to invoke the Driver          | **The single-file path’s diagnostic set and exit code are byte-for-byte identical**                       |
| Differential test over the entire corpus (293 `.yx` files) | Diagnostics list (sorted by `(code, file, line)`), exit code, stdout/stderr all have **zero differences** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance standard is zero-diff at the C1/C2 level.

**Rollback point:** `git revert` the single commit; `pipeline.rs` returns to its original state.

### S3: Merge the four orchestrator entry points + fix the defects

| Deliverable                                                                                                                  | Acceptance (C2)                                                                                 |
| ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                              |
| Delete the manual LSP stage sequence                                                                                         | LSP and CLI produce the same diagnostic set for the same file                                   |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                | The defect is fixed                                                                             |
| Fix the ownership-layer `Unproven` empty match arm (originally `checker.rs:1313`)                                            | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode           |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | Removing Z3 from the environment no longer causes a panic                                       |

**Rollback point:** The defect fixes and the structural merge must be **two separate commits**. If
the structural merge causes problems, only the structural commit can be rolled back while retaining
the defect-fix commit; at that point, `test_multifile_proof_obligation_not_dropped` remains green.
The reverse—retaining the structural change and rolling back the fix—turns it red and is an
unacceptable intermediate state that must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                       |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implemented form in the §2 revision note; the file is authoritative for the count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                     |
| Promote obligation diagnostics from W to E   | Manually review every new E after the full corpus differential                                                                                   |

**Rollback point:** The severity of `assert_drained()` can be controlled by a configuration option;
switching between W and E does not require changing the code structure.

### S5: Finish the proof layer and wasm

| Deliverable                                                                                                                                                                                                                                             | Acceptance                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                        | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                     |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes cross-thread sharing during cargo test) | Production call sites of `default_solver()` drop to zero (tests only); cache-hit rate across the three consumers is observable (the counter landed in 89576fafd) |
| Add a warning to the degradation path at the `TypeChecker::ownership_inputs` criterion point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                     | A diagnostic is produced when there is no `body_checker`                                                                                                         |
| Reachability determination for the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                    | The conclusion is handed to the “F2 Determination” section of `06-cleanup-inventory.md`; this document registers only the need for determination                 |

**Note:** Correcting the layer order will change the diagnostic set and may expose large numbers of
previously silent `Unproven` results. **This item should proceed independently from S1-S4** and must
not be mixed into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time exhaustively enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                      | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is structurally identical to the opcode-table mechanism already proven in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating questions about invocation counts that currently require human reasoning                                                                                                               |
| **Obligation mechanism** | `Obligations` + `assert_drained()` provide **runtime** settlement, used together with the **static** `scripts/ci/check-obligations.py` gate; **the terminal severity is E** (user decision A on 2026-10-13: same-code promotion to E, code number unchanged; **implemented form in the §2 revision note**: the ledger does not copy the field structure; the red list—0 entries after account closure—retains its mechanism but does not enter the user-visible diagnostic stream) | The target is an entire class of bugs, not one bug: at least two instances of the same class have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are in scope. W→E was originally a two-step process so that every step would retain a usable C2 criterion. Now that the obligation inventory has been closed to 13 entries and the red list is empty, and the healthy state emits nothing, the terminal state directly uses E (see sections 7 and 8 of the §2 revision note) |
| **Fix scope**            | Fix the **consumer** (orchestration layer); do not change the production-side logic of the `Unproven` branches (now three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                                                         | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect behind “the logic is now fixed,” while the logic of all three branches is itself correct                                                                                                                                                                                                                                                                                                       |

This design additionally closes two kinds of gap:

- **The gap between “comment promises” and “code behavior.”** The “silent pass must not be revived”
  promise at `checker/refinement.rs` (originally `checker.rs:5165`) was a **comment-level
  contract**; it is now enforced by `assert_drained()`.
- **The gap caused by “one array entry was forgotten.”** Currently, five functions each manually
  spell out their call sequence. After unification, there is one stage table, and adding a stage
  without wiring it in causes a compile-time error.

### Rejected Directions

- **Generic stage chains `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—seven of the 11
  inconsistencies (two dead-code-analysis implementations, W1006, W1003, monomorphization, role
  context, and IR generation/linking) **do not involve type transitions**; they are configuration
  questions of “should this analysis run?” This approach also cannot express `Aggregation`, and
  would merely move the `check_module` / `check_module_collect_all` choice from the function layer
  to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “each of five functions manually spells out its calls” with “one array entry was
  forgotten,” which is precisely the cause of the current 11 inconsistencies. This repository
  already contains a counterexample of the same form: `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader`, all described under `docs/src/dev/design/check/`, have zero
  implementations.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields; `Stage` remains scattered across five functions, and the next
  divergence will continue to grow from there. **It must be done first** (as part of S1/S3), because
  the structural refactoring requires a known red test proving that the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry point
  and does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold.
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, and detection must be in the orchestration
  layer.

## Known Limitations and Risks

- **Slimming `orchestrator.rs` makes it harder to read in the short term.** After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactoring that “centralizes
  the wiring.”
- **S3 significantly changes the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”—they are real bugs that
  simply were never reported. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires field-by-field human judgment.** Some
  obligation fields, such as `module_namespaces`, may legitimately have no consumer and should not
  produce warnings. S4 must review every field rather than applying one policy indiscriminately.
- **The `Program` abstraction may be premature.** If the stage sets of some entry points remain
  unstable for a long time, `stages()` may degenerate into “a free parameter that accepts a
  different array on every call,” and the contract will lose its force. **Mitigation:**
  `test_program_stage_coverage` asserts that `stages()` can only come from the six predefined
  combinations.
- **Dependency on criteria:** The defect-fix criteria in S1/S3 depend on the defect tests in the
  [equivalence-criteria document](07-equivalence-oracle.md) being red first. **If that document has
  not first established a multi-file corpus layer, S1 cannot be accepted**, because all 293 existing
  corpus files take the single-file path and provide zero coverage for this class of defect. This
  document does not cover the IR validator (`verify_loose`); that prerequisite work is outside its
  scope.
- **Correcting the layer order exposes previously silent `Unproven` results** (`equivalence` is not
  in the pipeline at all, and the order of `termination` and `ownership` is the reverse of the
  declared order). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after converting the SMT backend to a singleton is not settled.**
  `backend.rs:13-19` already states that `Solver` is `Send` but not `Sync`, and the `Z3Backend`
  cache uses `RefCell`; after converting to a singleton shared across compilation units, all access
  paths must be confirmed to pass through `Mutex`.
- **Stage parallelism has not been evaluated.** Multi-file typechecking is naturally parallelizable,
  but parallelism can **mask order-dependent defects** (such as the actual order dependency between
  termination and ownership). This should be opened only after the equivalence criteria are stable.

> **All open questions originally listed in this section have been adjudicated.** See
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1–D50). **No pending item remains in this document.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of the three parallel type
  representations (this document introduces no fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (construction steps are assigned to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria, and the
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — explicit declaration that “silent pass must not
  be revived” (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — layer call-order table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — contract declaration that “callers should
  conservatively degrade” (contradicts `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-76` — 13 obligation fields that downstream code must consume
  (after the 3.4.4/3.4.5/3.4.9 account closures)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — the unit test’s own
  admission that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  the corpus’s own admission that execution happens “by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (invoked by `run_code` to compile)—the playground’s
  fourth compilation caller (the single-file path, so proof_execution is executed)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen live in the shim crate rather than the main crate
- `build.rs:19-55` — compile-time error-code gate and an example of this project’s enforcement
  mechanism
