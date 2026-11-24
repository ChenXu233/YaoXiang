# Compilation-Stage Contracts and the Obligation Ledger

> **Companion design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiers, and execution-stage order are defined in the main
> RFC-039 text; see the [directory index](index.md) for the positioning of each companion document.

## Scope and Positioning

This document is the implementation blueprint for RFC-039's **L1 orchestration layer**. It addresses
one problem: **“which compilation stages ran, which did not, and why” is currently hand-wired
independently across six paths and ten entry functions, with no single place able to answer the
question.**

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are being broken by the architecture rather than by
> the logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` states (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile error, no fallback, no silent
pass……silent pass must not be allowed to return.” However, this invariant holds only on the
**single-file path**: proof-function constraints such as `x: Sorted(3)` silently pass on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths. The proof function is never executed,
and no diagnostic is produced.

### Covered

- The `Stage` enum (exhaustively enumerable, not registered at runtime) and `StageScope`;
- The `Obligations` ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, plus the individual changes required for each of the
  ten entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by these changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-bounce gates → `01-routing.md`
- Equivalence criteria (C1-C6 tiers and the three-level criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm-branch reachability cleanup → `06-cleanup-inventory.md`
- The global execution order of P1-P10 and the G1-G10 acceptance gates → the main RFC-039 text

### Division of Labor with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should proceed. This
document defines L1's **concrete shape**, **per-file change list**, and **implementation stages
within this document** (corresponding to P3 “Fix correctness defects” and P4 “Stage contracts and a
unified Driver” in RFC-039's global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

Equivalence criteria are applied under the **C2 (orchestration change)** category of the
[equivalence-criteria document](07-equivalence-oracle.md): identical diagnostic sets across entry
points + identical corpus behavior.

## Current State

> Every item in this section is a **verified fact**, with a file path and line number. Line numbers
> are based on `9e02e4db`.

### Stage boundaries are the only structural defect where “one incorrect implementation silently disables the entire repository”

This differs in nature from the other two classes of defects, module boundaries and test wiring:

| Defect                   | Typical manifestation                    | Is there a signal?       |
| ------------------------ | ---------------------------------------- | ------------------------ |
| Lexical/syntax error     | Source code is incorrect                 | A diagnostic is produced |
| Type mismatch            | Type is incorrect                        | A diagnostic is produced |
| Missing stage connection | A stage is never invoked                 | **No signal whatsoever** |
| Unconsumed obligation    | A field is populated but nobody reads it | **No signal whatsoever** |

The latter two share a defining characteristic: **failure does not produce an error**. They
therefore cannot be solved by “writing the code more carefully” or “reviewing more strictly”—code
review can reveal what was written, but not **what was omitted**. This is precisely the root cause
identified by RFC-039: “This project treats ‘design’ as a documentation convention rather than as an
executable constraint.”

### Enforcement mechanisms already present in this repository

The same repository already has mature enforcement mechanisms; they simply have not been extended to
the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E codes and 8 W codes) are subject to
  a **build-time hard gate** by `build.rs:19-55` through `tools/code-tables`, which compares them
  item by item against the RFC-013 code table. Any mismatch causes compilation to be rejected with
  `panic!`.
- `src/package/` (**76 files / 13,012 lines**, including 6,227 lines under `tests/`, or 47.9%) has a
  high testing density, and every module's test subtree is explicitly wired in. This is the
  repository's most complete test-wiring area and can serve as a formal reference for stage-layer
  gates.

**The design capability is sufficient. What is missing is “a gate of the same level in the
orchestration layer as well.”**

(The line-count convention is `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Compilation flow                                                                   |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct invocation of 5 stages                                                      |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + standalone IR                                            |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project uses 3c; otherwise manual lex→parse                                     |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The five stages of the single-file path (`pipeline.rs`) are:

| Stage               | Call site     | Implementation                                                            |
| ------------------- | ------------- | ------------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                                  |
| parsing             | `161`         | `run_parsing`                                                             |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                               |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`)—RFC-027 Phase 2.5                       |
| ir_generation       | `205`         | `run_ir_generation`, **with monomorphization embedded in it** (`381-389`) |

### 11 stage-coverage inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean that the entry
point “does not do this”; it means that “there is no code in this entry point that does
this”**—which is precisely the essence of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                          | `check_project`                                                | `check_source_in_project` (LSP)    | `compile_embedded_module`                     |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------- | --------------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                     | **No**                                                         | **No**                             | **No**                                        |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                     | Yes `356-377` (role-aware, no configuration gate)              | **No**                             | **No**                                        |
| 3   | W1006 local-module shadowing                             | **No**                                              | **No**                                                     | Yes `336-348`                                                  | **No**                             | **No**                                        |
| 4   | W1003 unused import                                      | Yes (collects `type_result.warnings` at `277`)      | **Collected but never emitted**                            | Yes `350`                                                      | **No**                             | **No**                                        |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                     | Yes (same as 2)                                                | **No**                             | **No**                                        |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                     | Not applicable                                                 | Not applicable                     | **No**                                        |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                       |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                   | `discover` (`454`)                 | Not applicable                                |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                     | Yes (`321-328`)                                                | **No**                             | **No**                                        |
| 10  | Global-slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                        | **No**                                                         | **No**                             | **No**                                        |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                            | **No**                                                         | **No**                             | Yes (standalone ModuleIR followed by merging) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. Current differences after P3
> implementation:
>
> - **Row 1 has been fixed**: all five proof_execution entry points share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + four orchestrator entry points), so the sole-consumer
>   defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6; user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and the dead-code-family warnings. The W-code sets across the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) have been
>   normalized—the criterion has a single source in the shared implementation behind the
>   `dead_code_analysis` arm.
> - **Row 6** (single-file exclusive monomorphization) → assigned to WBS 3.4.8 (prerequisite 4.1.3;
>   the potential-risk defect has not been proven).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry points → assigned to 4.2.7
>   (parameter-driven `Aggregation`).
> - The code-side reference to “ruling #434” had previously not been registered in the docs—it has
>   now been registered as RFC-039 **D57**.
> - **Asymmetry in embedded-std inclusion (new fact outside the table, registered on 2026-10-09)**:
>   the single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required
>   for desugaring `for` loops, hard switch under #117), whereas multi-file `discover` previously
>   recognized only explicit `use` statements—project-local `for` loops compiled successfully and
>   then failed at runtime with E6006 (confirmed by a probe). This has been fixed and WBS 4.10.2
>   closed. The related oddity in which the multi-file path lacked the single-file parse hard-abort
>   behavior was registered as WBS 4.10.1—**it has also been fixed** (immediately after 4.2.2; the
>   Check path was degraded to per-file collection, with a faulty file leaving the compilation unit
>   and therefore not entering the registry, causing importers to report E5001). This audit method
>   examines the “stage coverage and field consumption” dimension; this case concerns
>   compilation-unit membership differences and is an out-of-scope dimension that escaped detection.
>
> - **Embedded-std registration-surface coverage (discovered during 4.2.5 implementation, fixed)**:
>   the Registry arm repeatedly harvests registrations for embedded std units through
>   `extract_module_info`, which overwrites the native half registered by `with_std()` using the
>   combined “native + yx surface” model (`result.is_err` and others are lost, causing std.test to
>   report a false E1043). This audit's third escaped dimension is the merge semantics of “two
>   registration sources for the same module key”: the single-file path's registry is formed through
>   `with_std()` exactly once, whereas the multi-file/Check Registry arm obtains coverage only
>   through per-unit insertion.

**Two points require precise wording; otherwise the conclusion will be incorrect:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; it does report them on the
  **single-file** path (`run_file` → `Pipeline::run`). The cause is `compile_project`: it packs the
  complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function **has no
  read site for `result.warnings` anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The entry-point criterion for `main` in row 11 also comes from different sources**:
  `check_project:385-395` uses `surfaces.bins` (the manifest-declared surface), whereas
  `compile_project` uses `is_bin_role` (`250-252`, which checks only whether a manifest exists). The
  two functions give different answers to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is a single origin for obligations.**
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
so the proof function must actually be executed to determine the result.”

**Step 2 — The consumer is inside the checker, but the checker only produces and does not consume.**
`src/frontend/core/sema/checker.rs` contains **three isomorphic branches** that handle
`ProofResult::Unproven`:

| Branch                                                                                     | Location                                                                                                                         | Behavior                |
| ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter-refinement validation (`revalidate_refined`, originally `checker.rs:5164`)       | `if calls.is_empty()` → infer a hard error (originally `5170-5177`); `ctx.proof_calls.extend(calls.clone())` (originally `5179`) | **Emits no diagnostic** |
| Call-site argument validation (`check_call_arg_refinements`, originally `checker.rs:5306`) | Infers a hard error (originally `5308-5316`); `ctx.proof_calls.extend(calls)` (originally `5318`)                                | **Emits no diagnostic** |
| Return-position obligation (`check_return_refinement`, originally `checker.rs:5420`)       | Infers a hard error (originally `5442-5446`); `ctx.proof_calls.extend(calls)` (originally `5448`)                                | **Emits no diagnostic** |

The original comment inside `revalidate_refined` in `checker/refinement.rs` (originally
`checker.rs:5165-5169`) states:

> `RFC-027 §4/§9: Unproven → compile error, no fallback, no silent pass.` ……
> `This branch therefore becomes a real defensive line rather than a speculative backstop: silent pass must not be allowed to return.`

In other words, the author knew that “not producing a diagnostic” was a defect and explicitly
classified it as a **known intermediate state with nowhere to go**, on the assumption that “someone
would eventually read `proof_calls`.”

**Step 3 — The field is indeed populated in the result.** `check_module_impl` declares a local
`proof_calls` and collects into it (originally `checker.rs:1234-1235`), then writes it into
`TypeCheckResult` as `proof_calls, // Phase 2.5 pre-registration of proof-function obligations`
(originally `:1439`). The field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** In production code,
`TypeCheckResult.proof_calls` (`types.rs:29`) is read in **only one place**,
`src/frontend/pipeline.rs:187`, which passes it onward at `189`:

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other repository-wide occurrences of the `proof_calls` identifier fall into three categories, none
of which consumes this field: the three production-side collections in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); the identically named field of `ProofResult`
in `verdict.rs:61`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator's four entry points pass through that layer.** The
`compile_project` (`99`), `check_project` (`273`), `check_source_in_project` (`450`), and
`compile_embedded_module` (`1374`) functions in `src/frontend/module/orchestrator.rs` **all do not
invoke `pipeline.rs`**; they call `TypeChecker::check_module` directly (`124` / `493` / `1386`).
Consequently, they cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also follow the
discard path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is
responsible for compiling embedded std. This means that **the embedded std module's own proof
obligations are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5`
(`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs successfully, and produces no
diagnostic** on the multi-file, check, and LSP paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the layers.**
When `check_module_impl` processes ownership-layer results (originally `checker.rs:1303-1314`):

```rust
// Original checker.rs:1313 (now inside check_module_impl)
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the `5164` branch's promise that “silent pass must not be
allowed to return” and is **independent of the orchestrator problem**—even if the entry-point layer
is completely fixed, this location will remain silent. It should be processed in the same batch as
the obligation ledger because it is the same class of problem.

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

The three RFC-027 unit tests (`src/frontend/core/sema/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), and therefore **stop exactly before `pipeline.rs`**.

The test files' own documentation comments already state this.
`rfc027_refined_transparency.rs:13-15` says:

> `This file asserts only what check_module can observe. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer.`

**This is precisely the shape of the problem: the tests verify that “the obligation was populated,”
while the bug is that “the consumer did not read it.”** A test that covers only the producer and not
the consumer is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria is the single-file path

The [equivalence-criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` files under `tests/yaoxiang/` as its **third-level criterion** and the primary
acceptance mechanism for C2-stage changes. In practice:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero glob matches).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution is executed**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none covers the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is direct
evidence of this point. The file is marked `// expect: compile-error E4018` (line 15), and its
header comment states on line 7 that its status is “❌ should be rejected at compile time”—**it
passes precisely because it takes the only path that executes proof functions**.

**Conclusion: the third-level criterion needs an additional multi-file corpus layer; otherwise it
cannot serve as the acceptance tool for this document.** See implementation point S1.

### Obligation fields: 14 fields that constitute contracts merely by being produced, with no mechanical guarantee

Among the fields of `TypeCheckResult` in `src/frontend/core/sema/types.rs:16-80`, **14 fields** have
documentation comments explicitly stating “produced by stage X → consumed by stage Y”; in other
words, they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: Definition line = the field's **declaration line** in `types.rs` (the
> line containing `pub` / `pub(super)`), not the first line of its doc comment—the comment block may
> span multiple lines, and only the declaration line provides a unique anchor. The **struct range**
> in the sentence above (`src/frontend/core/sema/types.rs:16-80`) moves with the struct's line
> numbers and is mechanically reconciled by gate **F5** (an error fails the gate); definition lines
> are reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | |
> `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**: `ir_gen.rs:360` reads it
> (`type_result.release_plan.clone()`), `:3103` uses it (`ReleasePlan::consume`);
> settlement-completion marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-completion marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation | | `record_models` | `79` | typecheck (one pass at declaration)
> → IR generation (quantifier propagation; from B.2 onward, both the `TypeDeclField.quantity` of
> witness fields and constructor-parameter/default-value population read this table) |

**Reclassification record (3.4.9, user ruling A on 2026-10-13)**: The original row 14,
`interface_impl_registry` (`types.rs:49`), was **reclassified as an in-tree fact**—by design it is
consumed within typecheck's phase, has zero cross-stage reads, and is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side, only the ledger's own registration and documentation remain
  (removed in this round).
- **Disposition**: delete the documentation comment's claim that this is a “mirror for consumption
  by the middle layer/LSP”; retain the field and its in-phase consumption. The field is registered
  in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with its rationale). If LSP later
  genuinely needs this table, it must be registered through 4.6.x; **no consumer must be fabricated
  in this round**.

**All these fields cross layers keyed by `Span` or name tables.** The equivalence-criteria document
already notes that mismatches in span-keyed contracts can “fail silently, with no error whatsoever”
(`ReleasePlan` is a typical example: if the two sides compute spans differently, the `Drop`
instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module that every entry point must traverse.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator's four entry points never traverse the
  `pipeline.rs` layer.

**Verified fact**: The repository currently has **no mechanism whatsoever** to ensure that these 14
fields are consumed. The only “protection” is that `ReleasePlan` happened to ride along with IR
generation.

### Registration of non-obligation fields: channel, in-phase, and in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually, with its field name and rationale. This list is
**complementary** to the “§Obligation fields” table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                     |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (0 hits in `src/middle`)                                                                                          |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not “production implies a contract”)                                                                  |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its documentation does not declare a cross-stage contract ⇒ not adopted                           |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), like `bindings`—contract not declared                                                   |
| `semantic_db`             | Consumer is LSP (from 4.6.1, emitted through the `DriverOutcome.semantic_db` channel at `src/lsp/server.rs:482`), not a compilation-stage obligation |
| `trait_table`             | Has cross-stage reads at `ir_gen.rs:4336/:6867`, but its documentation does not declare a contract                                                   |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                          |
| `interface_impl_registry` | In-tree fact (user ruling A in 3.4.9; see the reclassification record in “§Obligation fields”)                                                       |

**Why this is not “wiring up a consumer”**: Creating downstream consumers for these fields would
fabricate obligations and disguise “fields that do not require cross-stage consumption by design” as
contract defects. The correct closure mechanism is **reverse containment**—the `TypeCheckResult`
field set = obligation inventory ∪ non-obligation registry (with reasons), mechanically enforced by
criterion 1 of `scripts/ci/check-obligations.py`: adding a field without registering it, deleting a
field without synchronizing the registries, omitting a reason from the non-obligation registry, or
overlapping the two tables all fail immediately.

### Four additional contract defects in the proof layer

The following findings come from analysis of the proof layer. They are independent of the
entry-point divergence problem, but all belong to the class of “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation; there is no mechanical enforcement, and the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/sema/layers/README.md` describes the actual invocation order of
`checker.rs::check_module_impl` (anchored by function name/invocation point, with line numbers taken
from the current tree):

| Actual order | Invocation point                                                                                                                                                                                                                       | Corresponding layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (two sites: `revalidate_refined` carries the E2030 interpretation; `judge_refined_obligation` is the sole decision unit shared by call-site obligations and return positions) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                                                                                                        | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                                                                                           | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                                                                                                   | —                   |
| —            | `equivalence::check_type_equivalence`—**zero calls in `checker.rs`**                                                                                                                                                                   | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module's own header comments and do not correspond to the invocation order. The README has no
“number → invocation point” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and is
unrelated to `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds termination errors one by one and continues downward, so “a
lower-layer failure prevents upper layers from running” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic has been removed by P3's 3.3.1 (the SOLVER
> slot has been made optional, and a missing solver conservatively degrades to
> `SMTResult::Unknown`); “silent skipping (not injected)” has been supplemented with a W1081 signal
> by 3.4.2. Unification of the three forms (singletonization) is to be implemented according to
> **RFC-039 D58**, at the actual fix location `proof/smt/backend.rs` (a process-wide shared
> singleton + a `with_shared_solver` closure-based access point). The `default_solver() → &'static`
> wording in the change list in section 02 is superseded by D58 (a bare `&'static` reference is
> infeasible because `dyn Solver` is not `Sync`).

| Consumer                                                                                                   | Acquisition strategy                                     | When the solver is unavailable                               |
| ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `predicate.rs:34-36`                                                                                       | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**    |
| `termination.rs` (injected at construction through `check_module_impl`, originally `checker.rs:1265-1268`) | Injected during construction with `with_solver_owned`    | `None` → **silently skipped** (not injected)                 |
| `ownership.rs:627` (back-edge-cut decision)                                                                | Calls `default_solver()` **on every invocation**         | `None => return false` (`629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure must
remain a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint
exceeds the solver's capabilities’.” In contrast, the contract documentation at `backend.rs:60`
says: “`None`: backend unavailable……**callers should degrade conservatively**.” **Two philosophies
coexist in the same codebase, and the hard-failure path panics instead of returning an error.**

**(c) Every back-edge decision creates a new Z3 context, making the cache effectively useless.**
`ownership.rs:627` invokes `default_solver()` on the hot path of back-edge-cut decisions. Meanwhile,
`src/frontend/core/sema/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—every invocation calls `Z3Backend::new()`, creating
a new Z3 context. The `Z3Backend` cache field (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **per-instance**, while the documentation comment at
`z3_backend.rs:17` claims that “SMT query results are cached in `cache`.” Because it is not shared
across invocations ⇒ **the cache can never be hit in this invocation mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. It is the same
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

When `body_checker` is `None`, ownership checking receives **an empty type ledger and an empty call
table** and proceeds normally—ownership analysis degrades to “nothing conflicts,” **with no warning
whatsoever**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm state: carried by a shim crate, with branches in 29 files that are live

The wasm target **has been built and is built in CI**. The `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The main crate provides only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                         |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                         |
| The shim depends on the main crate's `rlib` as a library  | `wasm/Cargo.toml:17`, `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                               |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140`, `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                           |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3`, `exclude = ["wasm", ...]`                                                                                                                                                                                                                                                  |
| **CI contains 4 wasm builds**                             | `_build-wasm.yml` (reusable workflow, `:77`, `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), and `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt with Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (downloads `libz3.a` from a fixed URL, degrading to a warning if missing)                                                                                                                                                                |
| **29 files** contain the literal `wasm32`                 | **22** have actual `#[cfg(...)]` attributes, while the other 7 only mention it in documentation (see the “F2 assessment” subsection of `06-cleanup-inventory.md` for the list and item-by-item assessment)                                                                                 |
| `orchestrator.rs` contains 16 occurrences                 | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                       |
| `src/lib.rs` contains 11 occurrences                      | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                             |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs `wasm/src/lib.rs` (152 lines) can call under the wasm
target—`src/lib.rs:151/170/187` gate off `run_file` / `run_project` / `build_bytecode` as a whole
(all three require `std::fs`), while the shim takes a different route.

**However, this produces one fact directly relevant to this document**: the playground entry point
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and passes
it to `Driver::run` (the JS-side entry point `run_code`, `:122`)—the **single-file path**.
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
has three,” but all three are on the single-file side.** When the unified Driver is implemented,
this path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`), which
shares the single-file stage table with `SingleFile` (`:135`); see item **4.2.8** in
`09-execution-wbs.md`.

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in the playground scenario—is assigned to the “F2 assessment” subsection of
`06-cleanup-inventory.md`: 4 wasm arms are unreachable at playground runtime but load-bearing at
compile time, while 6 `not(wasm32)` arms are load-bearing in the three native stage tables. There is
no low-risk item that can be deleted directly. The RFC-039 open question “build a wasm target, or
delete these branches?” is **now obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, not registered at runtime)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum; runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` enforces at compile time that “a new stage
must be handled by the orchestration layer”—exactly the mechanism already proven effective by the
opcode table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency verification       Project
    Discovery,          // File discovery                       Project
    Parsing,            // Lexing + parsing                     PerModule
    Registry,           // Module-registry construction          Project
    RoleClassification, // Role classification (Script/Bin/…)    Project
    Typecheck,          // Type checking (including proof layer) PerModule
    DeadCodeAnalysis,   // Dead-code-family analysis            Project
    ProofExecution,     // Compile-time execution of proof funcs PerModule
    GlobalSlotAlloc,    // Global-slot allocation               Project
    IrGeneration,       // AST → ModuleIR                       PerModule
    Monomorphization,   // Monomorphization                     Project
    Linking,            // Cross-module linking / IR merging    Project
}

impl Stage {
    /// All stages. Adding a variant makes both this array and dispatch's exhaustive match fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once per
project) make “which stages must run per module and which must run once per project” a fact
expressed by the type system. Stages with `Project` scope are structurally guaranteed to run only
once by `dispatch`, eliminating questions that currently require human reasoning, such as whether
`allocate_global_slots` should be called once per file.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the initial draft were
> corrected based on implementation evidence:
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes IR output
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the initial order contradicted the
>    data flow.
> 2. The Check-form stage table was normalized to dead code **before** proof according to the
>    `Stage::ALL` topological order: `check_project` currently executes proof before dead code, but
>    the two have no data dependency and produce the same diagnostic set (C2 set semantics); only
>    intra-file diagnostic order is normalized. The single-file path (dead-code analysis embedded in
>    typecheck and preceding proof) already conforms to the ALL order, so byte-for-byte acceptance
>    is unaffected.
> 3. `Parsing` was moved **before** `Registry`/`RoleClassification` (C3, user ruling on 2026-10-09):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST output—the
>    initial order would force the Registry arm to “hide parsing,” making the stage table lie about
>    the data flow; it also reduces multi-file parsing from 2 times to 1. The current state is also
>    faithfully registered: multi-file parse errors used to be a hard abort (the `?` propagation in
>    `build_registry_from`, in tension with CollectAll semantics, registered as WBS 4.10.1;
>    `build_registry_from` was deleted on 2026-10-26 after its last caller was removed, and the
>    historical account is retained here)—**4.10.1 has been fixed** (user ruling on 2026-10-09:
>    rust-style collection semantics + option B): on the Check path, a parse failure degrades to
>    per-file diagnostic collection, and the faulty file leaves the compilation unit (does not enter
>    the registry, so importers report E5001); the MultiFile path retains a hard abort (under
>    FailFast, a faulty file cannot produce IR, which is correct semantics and remains permanently
>    pinned).
> 4. Check-form implementation (4.2.2, user ruling on 2026-10-09) adds two registrations: a.
>    Data-dependency edge 14→16: `RoleClassification` consumes Discovery's used_by edge set, and
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events—the initial draft omitted these
>    (these products were produced by `discover_with_used` in the orchestrator era but discarded by
>    the `discover` wrapper, so they disappeared from the data-flow table); b. Intra-file
>    diagnostic-order normalization expands to the full form of note 2 above: E3020 entry validation
>    moves into the RoleClassification arm (Check has no Linking stage, so the arm holding surfaces
>    and AST performs validation), and intra-file diagnostic order = stage topological order (E3020
>    → typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set
>    semantics), and only stderr entry order changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design goal**: Turn “a field is produced but nobody consumes it” from an undetectable condition
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
    /// Each unconsumed field produces one W-level diagnostic (by default) or a hard error (in strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Case                                            | Criterion                                                             | Handling                                                                          |
| ----------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| The obligation is read by its declared consumer | The corresponding field in `Obligations` is `take()`n/marked consumed | Pass                                                                              |
| The obligation is non-empty but has no consumer | The field is non-empty and not consumed                               | **Produce a diagnostic** (`W` level by default; promoted to `E` in `strict` mode) |
| The obligation is empty                         | The field is empty                                                    | Pass (no consumption required)                                                    |

**Why begin with W rather than E**: Fixing discarded obligations will **change the diagnostic set**.
C2 requires that “the diagnostic sets of all entry points be identical.” Moving directly to E would
cause `yaoxiang check` to suddenly emit many previously silent `Unproven` diagnostics. Splitting the
work into two steps—first W-level observation, then promotion to E—keeps every step independently
verifiable. See implementation point S4.

**There is exactly one call site for `assert_drained()`**: `Driver::run`, at the end of the stage
table and before producing `CompilationResult`.

**Companion static gate**: RFC-039 proposed `scripts/ci/check-obligations.py` (a field appearing at
least twice but having no read in a third file → failure). `assert_drained()` is the **runtime**
gate, while the script is the **static** gate. They complement each other, and both are required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: This note registers the
> implemented shape, measured figures, and 4.3.2 interface of the sketch and three-tier table above,
> mapping each item to code file:line. The landing commit is `1941cc47a` (verification tree
> `99079bc52`). It also serves as the **third** of the three places responsible for the red list's
> “no silence” requirement (the other two are the audit surface and the static gate).
>
> **1. Delivered artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (`:201`, field
>   name/consumer stage/red-list status), non-obligation registry `NON_OBLIGATION_FIELDS` (`:150`, 8
>   entries with reasons), delivery-marker ledger `DeliveryLedger`, pure settlement function
>   `settle_obligations` (`:577`), external emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer gating `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), and W1090 code constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: the end of the `Driver::run` stage table, before `into_outcome()`
>   produces `CompilationResult` — `src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-return path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one makes runtime W1090 fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four instances of
>   the same delivery point: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single file directly reads
>   the request table) / `:1221` (MultiFile·Check: the Linking arm aggregates, and
>   `driver/helpers.rs:295` reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by tests.
> - Diagnostic code: W1090 is registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`; the reason is documented above the entry at `:26-31` —settlement
>   granularity is “unit × obligation,” with no source location to which a user can point, so no
>   dummy span is inserted); text exists in all six languages in `locales/*.json`. **Emission
>   severity**: from 4.3.2 onward it is **error** (same code promoted to E; see section
>   3)—determined by the explicit `.severity(Severity::Error)` in `to_diagnostic`
>   (obligations.rs:444); the code itself remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; the CI job `obligations` was added
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **2. Cell-by-cell implementation of the three-tier table + two additional tiers**
>
> | Case in the table above                         | Implemented criterion                                                                            | Code                   |
> | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ actual read reconciled by the static gate; see section 5) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see section 3)          | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                                 | obligations.rs:587-591 |
>
> Two **additional tiers** (user ruling on 2026-10-13; not present in the sketch):
>
> 1. **S1**: The consumer stage is not in the current `ProgramKind`'s stage table → **not considered
>    outstanding** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (`:121-126`).
> 2. **Skip exemption**: The consumer stage was skipped due to `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not considered outstanding (obligations.rs:598-602). `NoObligations` does
>    **not** grant an exemption—if the obligation is non-empty while the stage acknowledges having
>    no obligation, the ledger is precisely unbalanced and must be reported.
>
> **3. External-emission policy (ruling “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: This mechanism emits only **W-level** diagnostics (W1090); there
> is **no strict switch**. **4.3.2 (terminal state, user ruling A on 2026-10-13)**: **The same code
> is promoted to E**—the W1090 code number remains unchanged, but the emission severity is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (one fact cannot have two parallel representations). **The promotion changes only
> the emission severity, not the emission conditions**. **This is a D-class breaking item under D59
> (changes externally observable behavior), faithfully registered**: healthy programs emit nothing
> (all obligations have delivery markers), so existing corpora and user programs are unaffected
> (342/342 “difference: empty”); but whenever a run actually encounters an undelivered obligation,
> that diagnostic **counts toward `error_count`, renders as `error[W1090]`, and causes a nonzero
> exit code**—unlike the previous “W-level does not block compilation” behavior. The policy that
> code family and emission severity are decoupled is registered in RFC-013's “emission channel”
> section (the code prefix determines numbering ownership only; severity is determined explicitly at
> the construction site). Mechanism facts (measured): severity comes from an explicit `.severity()`;
> otherwise “a code beginning with `W` → Warning” (`codes/builder.rs:186-203`), and the registry's
> `ErrorCategory` does **not** participate in severity. The exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`); **because W1090 is explicitly an error, it counts toward `error_count`
> (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain ruling C on
> 2026-10-13; the current red list is empty; see section 4), for a reproducible reason:
>
> - Measured cost of ruling A (red list emitted as W): `interface_impl_registry` is populated from
>   typecheck's prebuilt table and is **always non-empty**; the per-compilation emission is
>   therefore a **constant signal** (with no discriminative power), yet it would break three
>   existing external contracts—“healthy programs emit no diagnostics / LSP emits no diagnostics /
>   `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing tests turning
>   red**, of which **8 are on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs`
>   ×5, `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4
>   are under `src/driver/tests/`), and the corpus baseline changed in 24/342 fixtures. The only
>   viable change would be “filter W1090 out of zero-diagnostic assertions,” which weakens existing
>   criteria.
> - Ruling B (demote to Info) would still break 9/12 list-style assertions, and Info is not “output
>   according to the ruling's severity.”
> - Measured outcome of ruling C: **zero changes to existing tests**, **zero changes to the corpus
>   baseline** (342/342 “difference: empty, behaviorally equivalent”), and runtime W1090 becomes a
>   **discriminative regression detector**—the healthy tree remains silent throughout, and removing
>   any delivery marker makes it fire (each of the three delivery points was tested by
>   marker-removal mutation; the CLI `check` emits 2 `warning [W1090]` messages when the Check arm's
>   marker is removed).
>
> The red list's “no silence” requirement is borne by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (emitted on every run, including WBS IDs
> and evidence pointers); ② `check-obligations.py`, which prints the red list on every run
> (currently empty) and performs staleness detection; ③ this note.
>
> **4. Current red-list state (ledger cleanup and closure) and implementation records for
> 3.4.4/3.4.5/3.4.9**
>
> The red list went from 3 → **0**; all three cleanup tasks were completed on 2026-10-13:
>
> Note: The line anchors below record the file state **as of 2026-10-22**. P5 (2026-10-26) split
> `checker.rs` into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, shifting
> the line numbers in `checker.rs`; the anchors below no longer correspond to the current files and
> are retained only as historical evidence.
>
> - **3.4.4 `variant_ctor_calls` → delete the field**. Its declared responsibility (“ir_gen
>   generates CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order obtained from `sum_type_variants`), with the
>   call site at `:7496` replacing the entire Call with `CreateVariant`; the function's doc
>   explicitly states that “span keys are unreliable across systems (the two ends interpret a Call
>   node's span differently), so detection is performed in place”—**wiring a consumer would
>   reintroduce the already-fixed span-mismatch defect**. Deleted: the `types.rs` field, collection
>   in `checker.rs:1489-1493/:1565`, the collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`;
>   `infer_variant_ctor_call`'s type inference and all diagnostics are **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete the cross-stage mirror** (retain the in-phase
>   candidate-name table and `overload_resolutions`). All read/write sites: written in-phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550` decision point; the sole hit at `:4550` writes
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** populated into
>   `TypeCheckResult` at `checker.rs:1558`. The naming fact still has a single expression:
>   candidate-name table (in-phase) + span-keyed decision (cross-stage, constructed at
>   `ir_gen.rs:380`, consumed at `:7632/:7705`); the mixed naming on the definition side is computed
>   independently by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in the §Obligation fields table of this document). What was deleted is
>   the documentation claim that it is a “mirror for consumption by middle/LSP” (`types.rs:47-48`);
>   the field and its in-phase consumption are retained (`operator_interfaces.rs:623` operator
>   lookup, seven constraint-solving lookups in `inference/expressions.rs`, `checker.rs:214-217`
>   read-only accessor); direct proof of zero cross-stage reads:
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the table's content is an environment mirror populated by typecheck's prebuilt table (always
>   non-empty); if LSP genuinely needs it, register it through 4.6.x. Creating a consumer for a
>   field that does not require cross-stage consumption by design would introduce a false obligation
>   into the ledger.
>
> After the red list was cleared, **the field census is closed by “obligation inventory ∪
> non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”**— the reverse-containment end of
> gate criterion 1 protects this end to end (adding a field without registration / deleting a field
> without synchronization fails immediately). The red-list **mechanism** is retained: the gate's
> criterion 4 staleness detection and the `--selftest` `stale-red-list` fixture continue to cover
> its detection surface; any future registration must go through an explicit ruling.
>
> In conjunction with the cleanup, the policy changed from obligation inventory 16 → **13**
> (`INVENTORY` / `Obligation::ALL` / the §Obligation fields table in this document); the gate's
> `--selftest` now covers seven fixture categories (adding “unregistered field,” “stale anchor,” and
> “empty red list”), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **5. Actual criteria of the static gate (9.2) (three hard criteria beyond the sketch's sentence)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ `TypeCheckResult` field definitions ↔ the “§Obligation fields” table in
>    this document (including the table header's count declaration); any drift fails. Furthermore,
>    the `TypeCheckResult` field set must be **exactly equal** to “obligation inventory ∪
>    non-obligation registry (`NON_OBLIGATION_FIELDS`, with reasons)”— adding a field without
>    registering it, deleting a field without synchronization, omitting a reason from the
>    non-obligation registry, or overlapping the two tables all fail immediately;
> 2. **Read evidence**: every non-red-list obligation must have a real read (`.fieldname` member
>    access) in the **declared consumer file**—comments/tests/strings do not count. This turns
>    “delivery marker ↔ actual downstream read” into a hard criterion (the principal risk of
>    mechanism A is drift between the two);
> 3. **Delivery markers**: every non-red-list obligation must be marked at its delivery point;
>    red-list entries, conversely, must not be marked;
> 4. **Red-list staleness detection**: red-list fields must have zero real reads (growing a consumer
>    fails), and every entry must include a WBS ID and evidence pointer; `--selftest` additionally
>    provides counterexample/positive fixtures: counterexamples must fail and positive examples must
>    pass (proving that the detector is not idle).
>
> **6. Item-by-item deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy each field**: obligation
>    data remains owned by `TypeCheckResult`, and the ledger records only “inventory + delivery
>    markers + settlement conclusions.” Reason: copying would create two sources of truth (the
>    fields are produced by different places—the checker and ownership layer in typecheck) and would
>    inevitably drift; cross-layer data should not be duplicated into the orchestration layer.
> 2. The sketch's `take()` / “mark consumed” **was not adopted**; it was replaced by `is_produced`
>    emptiness checks + `DeliveryLedger` delivery markers. Reason: an obligation and its product are
>    the same data, and `take()` would remove what downstream code needs; moreover, 11 IrGeneration
>    obligations take the form of “hand the entire `&TypeCheckResult` to ir_gen,” so fields cannot
>    be taken individually.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed **by severity** (from 4.3.2, W1090 is E level)—single file uses
>    `PipelineError::TypeCheck` (obligations.rs:654 emission arm), while multi-file/embedded record
>    an `OrchestratorError` failure (otherwise `result.errors` would have no consumer). Reason:
>    returning `Vec<Diagnostic>` would lose form-based routing—check diagnostics must retain
>    per-file attribution.
> 4. The first tier of the three-tier table changed from “the obligation is read by its declared
>    consumer” to “a delivery marker exists”; “marker ↔ actual read” is delegated to the static gate
>    (criterion 2 in section 5).
> 5. The second tier's “promote to E level in `strict` mode” **did not use a strict switch**; it was
>    changed to **promoting the same code to E in 4.3.2** (promotion occurs even without a strict
>    switch, because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no read in
>    a third file” to the four criteria in section 5.
> 7. The two additional tiers (S1, skip exemption) not present in the sketch are described in
>    section 2.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table was actually
>    implemented as **20 test cases** in `src/driver/tests/obligations.rs` (`#[ignore]` skeleton
>    first → turned green; red-state record: after removing the sole settlement point, `pairs: 0`
>    and 6 failures). The S1/S4 rows have been synchronized with the actual files.
>
> **7. Implementation record for 4.3.2 (severity + gate hardening)**
>
> - **Inputs for field-by-field review**: `ObligationSettlement.drained_kinds` (the kinds of
>   obligations actually drained in a given run, available per run) and the five counters (the
>   conservation identity in section 2); together with the audit surface `findings`, these establish
>   “whether this field has been consumed by any entry point.”
> - **Severity: promotion of the same code to E has landed** (user ruling A on 2026-10-13;
>   terminal-state registration and D59 D-class breaking-item declaration in section 3).
>   Implementation: explicit `.severity(Severity::Error)` in `to_diagnostic`; emission arms route
>   errors through the error channel according to severity. Mutation evidence: removing any delivery
>   marker → compilation fails in that form, and `check` exits nonzero (`error [W1090]`); after
>   restoration, the healthy state emits nothing (corpus 342/342 zero changes). Locale text in all
>   six languages states that “the code belongs to the W family, while the emission severity is
>   error.”
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registry rationales and red-list evidence**, validating “the file is locatable (uniquely) +
>     the line number is in range + the symbol immediately following the anchor actually appears in
>     the anchored file”; an anchor pointing to a nonexistent symbol fails. A counterexample fixture
>     `tests/baselines/obligations-gate/stale-anchor/` is provided (nonexistent symbol /
>     out-of-range line). Note: The red list is currently empty, so the criterion's actual
>     application points are **non-obligation registrations** and future evidence pointers (the same
>     code path).
>   - **F2/R3 reverse containment**: the gate outputs “`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference”; a nonzero difference fails. A
>     counterexample fixture `unregistered-field/` is provided (a new struct field not registered).
> - **Informational item F3**: The `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain its `type_result`, so the obligation
>   does not exist); this tier's semantics are pinned by synthetic `State` cases, while real
>   end-to-end coverage only includes `ConfigDisabled` (t3 T06).
>
> **8. Closure of 4.3's secondary task (red list cleared + obligation diagnostic promoted to
> terminal E level)**
>
> **8.1 Three-round implementation timeline** (each item mapped to code file:line; this round only
> registers conclusions and changes no code)
>
> | Round                     | Conclusion                                                                                                                                                                                   | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
> | ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger online       | `State::assert_drained()` sole settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                         | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                            |
> | 3.4.4 cleanup             | Delete `variant_ctor_calls`—its declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                                 | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                        |
> | 3.4.5 cleanup             | Delete the cross-stage mirror `method_overload_ir_names` (retain the in-phase candidate-name table + `overload_resolutions`)                                                                 | Decision consumer `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                              |
> | 3.4.9 cleanup             | Reclassify `interface_impl_registry` as an **in-tree fact**: delete the “mirror for consumption by middle/LSP” declaration, retain in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/sema/types.rs:52` (field declaration line; doc comment at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/sema/operator_interfaces.rs:623`, seven sites in `src/frontend/core/sema/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/sema/checker.rs:214` (read-only accessor at `:214-217`); registration `src/driver/obligations.rs:150` |
> | 4.3.2 severity assignment | **Promote the same code to E**: explicit `.severity(Severity::Error)`, code remains W1090; errors routed through the error channel according to severity (first failure takes priority)      | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/severity decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                         |
>
> **8.2 Closure conclusions**
>
> - **Red list cleared**: the number of `INVENTORY` entries (`src/driver/obligations.rs:201`) whose
>   `red_list` is `Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0).
>   The **mechanism is retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the gate's
>   criterion 4 staleness detection, and the `stale-red-list` fixture; the gate reports “red list 0
>   entries—cleanup complete.”
> - **Terminal E severity for obligation diagnostics**: W1090's **code number and W-family
>   membership remain unchanged**, while the emission severity is `error` (terminal D5 state, user
>   ruling A on 2026-10-13). This **is a D-class breaking item under D59** (changes externally
>   observable behavior), registered in both section 3 of this note and RFC-013's “emission channel”
>   section; **zero emission in the healthy state ⇒ no impact on existing corpora or user programs**
>   (corpus 342/342 “difference: empty,” IR snapshots have zero drift; t11 independently verified it
>   and t12's review rechecked it).
> - **Field census closed**: the `TypeCheckResult` field set = obligation inventory ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a measured consumption-surface
>   rationale); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises retained**: all hardening items registered in this note have
>   **landed** (F1/F2/F4/F5; see sections 5, 7, and 8); the only two unresolved items are 8.4's
>   **known boundary** and t7 F5, which has been converted into an issue; neither is an
>   unimplemented criterion of this mechanism.
>
> **8.3 Status of findings from each acceptance round / review** (each item explicitly identified)
>
> | Source        | Finding                                                                                                                                                                                           | Status                                                                                                                                                                                                                                                                                  |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence was merely a textual pointer and could become silently stale when the replacement consumer was removed                                                                          | **Landed**: gate F1 anchor validation (file locatable + line in range + symbol anchor, `scripts/ci/check-obligations.py:251`); counterexample fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                                  |
> | t3 F2 / t4 R3 | Gate criterion 1 lacked reverse containment, so adding a field without registration silently passed                                                                                               | **Landed**: F2 explicitly outputs the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; counterexample fixture `unregistered-field/`                                                                                                          |
> | t4 R4         | The W-only versus promoted-to-E policy was not registered                                                                                                                                         | **Landed**: sections 3 and 7 of this note + RFC-013 “emission channel” + W1090 help and `error_output` examples in all six `locales/*.json` files                                                                                                                                       |
> | t7 F5         | Dead-code analysis miscounted references to overloaded methods, incorrectly reporting `W1005` for legal overloads                                                                                 | **Converted to issue `#450`**, assigned to WBS **4.9 Correct implementation of the reference pool** (`docs/src/dev/architecture/09-execution-wbs.md:237`)—outside this secondary task                                                                                                   |
> | t11 F7        | Under the gate's `--root` mode, F1 did not cover anchor promises in module-header prose comments                                                                                                  | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                            |
> | t11 F8        | Verifier-tool self-report: mutation-driven runs once mistakenly wrote to the main repository (immediately cleared with `git restore`; outputs from the affected round were invalidated and rerun) | **Informational item** (not a delivered-artifact defect)                                                                                                                                                                                                                                |
> | t11 F9        | An intermediate state in the main repository (unstaged files made this document's header inconsistent with `INVENTORY`) caused the gate to fail                                                   | **Informational item / resolved**: the frozen commit was self-consistent and passed; “committing an intermediate state directly fails the gate” is precisely F2/F4's intended behavior                                                                                                  |
> | t12 F12-1     | This document gave the struct range as `16-73`, but the actual range was `16-76` (t9 added 3 comment lines without mechanical reconciliation)                                                     | **Landed + mechanized**: range corrected to `16-76`, and gate **F5** was added (range anchor; an error fails the gate); related figures were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                                        |
> | t12 F12-2     | Settlement in multi-file form once overwrote the first failure (the type error in `zzz.yx` was covered by W1090)                                                                                  | **Fixed** (the captain amended the commit message to document this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1's scanning surface**: anchor validation covers only text in **registered
>   entries**—non-obligation registry rationales (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If such an anchor promise is written in a module header
>   in the future, it must **be covered by manual review, or F1's scanning surface must be
>   expanded**—one of the two; leaving it unmanaged by default is forbidden.
> - **The gate's static boundary**: collection-level validation of delivery markers is static
>   (criterion 3), while removal of an individual arm is covered by **runtime settlement**— static +
>   runtime is the intended two-layer design, not a gap (t11's M11A/M11B/M11C mutations were
>   validated in both directions).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ification
> landed)**: Of the 13 obligations above, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been converted from spans to **structural identity keys** (`PlanId`)
> under RFC-039 **D20**. This note registers the implemented shape, criteria, classification
> rationale, and known boundaries; definition lines are still reconciled by gate **F4** (the line
> numbers for the two rows above are current code facts).
>
> **1. Implemented shape (carrier selected: `PlanId`)**
>
> The key is a **structural identity** carried by the AST node, rather than relying on the
> positional coincidence of “each side computing a span independently”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator** `ParserState::alloc_plan_id`. It has two carriers: `Stmt.plan_id` (statement identity
> → ownership of the release plan and its consumption) and `Expr::Call.plan_id` (call-site identity
> → production and consumption of overload decisions); `Expr::call` is the repository's only
> call-expression constructor, and synthetic calls outside the parser are recorded as `UNPLANNED`
> (**not consumable**; see section 2).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry is `ReleasePlan::consume(stmt)`—**reading and accounting occur in the
>   same invocation**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is supplied
>   by `walk_stmt`'s placement/restoration + the CFG-node→statement-identity table; the Drop for a
>   scope exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one object**),
>   produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with dispatch
>   passing it through from `Expr::Call { plan_id }`), consumed at `ir_gen.rs:7697` (with bindings)
>   / `:7772` (without bindings) through `OverloadResolutions::get` (a hit is accounted for).
>
> **2. Guaranteed consumption settlement (reusing the 4.3 ledger rather than creating a second
> one)**: the consumption ledger has a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), with one instance held by each table; when `generate_module_ir`
> returns successfully, both ledgers are marked as having “completed one full pass” at the same
> point (`ir_gen.rs:1095` release plan / `:1097` overload decisions); the three-state mapping is
> unique (`consumption_state` in `src/driver/obligations.rs`), and the sole settlement point
> connects one row for each of the two obligations—a nonempty difference produces **an E-level
> W1090** (same-code promotion to E; policy in section 3). The exemption criterion hangs on the
> **ledger's own completion marker** (not on whether `unit.ir` exists), so the Check form's removal
> of the IR product does not constitute an exemption.
>
> **3. D59 classification rationale (C class, red before green)**: Before the change,
> `Type.method(positional argument, name = value)` **silently returned the wrong answer** (measured:
> `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than `20`—the selected
> candidate was discarded and execution fell back to the last binding in definition order); after
> the change, the same program produces a compile-time
> `error [W1090] ... produced key was not consumed`. The fix deliberately chooses to **fail loudly**
> rather than “put the selected name into the call with named arguments”: the latter would lose the
> named argument and merely replace one form of silence with another (terminal semantics in
> [09](./09-execution-wbs.md) §3.14 / issue #452). Moreover: `Drop` instructions are no-ops in the
> interpreter (the wildcard arm of `src/backends/interpreter/executor/ops/control.rs` merely calls
> `advance()`), so migrating the release-plan key does not change runtime behavior—the empty
> difference across 342 corpus items is empirical proof of this.
>
> **4. Criteria and evidence**: D41's analogous criterion `test_release_plan_spans_consumed`
> (settled by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus; **the difference must be empty + the evidence surface
> has a floor**: number of files that produced a decision `≥ 1`, genuinely consumed entries `≥ 2`,
> no allowlist); the decision shape is covered by 9 pinned cases in
> `src/frontend/core/sema/tests/overload_resolutions.rs` (expected return / positional argument /
> type-body binding / `?` operand / purely named / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven validation: shifting a consumed key by one
> position ⇒ criterion fails + the real program emits `error [W1090]` + the corpus gate reports a
> difference; restoring it turns the criterion green. Gate figures: `check-obligations.py` PASS,
> corpus 342-item difference empty, `cargo test --lib` 2712 passed / 0 failed (IR snapshots have
> zero drift).
>
> **5. Known boundary (registered item, not an unimplemented promise)**: The **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test surface
> (method declarations uniformly use `register_type_bindings`, which takes the bound consumption
> path), so the key correctness of this branch has **no red state with which to falsify it**; the
> reason for retention is that it is the symmetric branch of the bound path, and deleting it would
> rewrite “a method call without `type_bindings`” into silence. This is registered by independent
> verification (t5 F-1) and review (t6 F2); **no new task is opened**.

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
    /// May only be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers may not supply an arbitrary array.** This is the key constraint preventing the `Program`
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
                // … 12 arms; not one may be omitted
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: The sketch above has four
> implementation deviations that are semantically equivalent and are registered for reference:
>
> 1. The three-state `StageOutcome` from section 4 is not returned by stage arms. It is expressed
>    through `State` failure flags + `Aggregation` gating (Continue / Warn / Abort semantics are
>    unchanged, avoiding per-arm boilerplate).
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” assessment in section 5 covers the sketch's field).
> 3. In 4.1.3, `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped`
>    ledger and are not emitted externally—the zero-diff criterion in S2, “deliberately fix no
>    bugs,” requires this; external emission comes with the 4.3 obligation ledger.
> 4. The visibility of the `proof_execution` module was relaxed to `pub(crate)`: the driver arm is
>    the only new caller (L1→L2 is an allowed direction); the existing orchestrator call sites
>    return to their proper visibility after their 4.2 migration.
> 5. In multi-file form, ProofExecution is an independent stage executed after **all** typechecks
>    (A1, user ruling on 2026-10-09): `compile_project` originally interleaved proof with the
>    per-file typecheck loop (file N's proof preceded file N+1's typecheck). A single failure is
>    byte-for-byte identical; in the multiple-failure scenario “file 1 proof failure + file 2
>    typecheck failure,” the first reported error changes from the proof error to the typecheck
>    error (pinned by a test).
> 6. `DriverOutcome` carries products through separate channels according to `ProgramKind`: `result`
>    (pipeline contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error
>    contracts of each entry point (PipelineError / OrchestratorError) are not part of the type
>    surface the Driver can unify.
> 7. Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, while a clean file has an empty Vec—the existing
>    `check_project` contract); Discovery's product is extended to a “file set + used_by edge set +
>    shadowing events” triple stored in State (the MultiFile form has no downstream consumer, so it
>    is recorded but not emitted externally); on the check path, per-file parsing drops from 2 times
>    to 1 following C3's topology.
> 8. Lsp form landed (4.2.3, ruling by file source, user decision on 2026-10-09): parse failures in
>    on-disk files follow the Check form's option B (collect + leave the compilation unit—both sides
>    converge on degradation); parse failures in edited buffers retain the partial AST and continue
>    typechecking (editor philosophy—the intermediate state of typing should not make semantic
>    functionality disappear). The Discovery arm preserves buffer source over stale on-disk content
>    for Lsp; Typecheck checks only the target file under Lsp (other units exist solely to provide
>    registry signatures); intra-file diagnostic order normalization extends to LSP (typecheck
>    diagnostics → W-code warnings → proof errors). The old LSP behavior of hard-aborting on an
>    unrelated on-disk file's parse error and silently falling back to the single-file path was
>    fixed in this step.
> 9. Embedded form landed (4.2.4): `Program` gains a `shared_registry` field—the embedded std module
>    is a sub-compilation, and the registry is an **input**, not a product (EMBEDDED_ STAGES has no
>    Registry stage), making issue #94's SymbolTable-sharing contract explicit in the program
>    declaration rather than dependent on “the caller remembers to pass the same registry.”
>    Error-path text changed from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler-internal error surface only, with no pinned test). All four orchestrator entry points
>    have now migrated to the Driver.
> 10. Standalone check unified (4.2.5, ruling A + IR-stage ruling, 2026-10-09): The Check form
>     faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files collect errors only; omit rather than
>     over-report); relative `use` is resolved relative to the importer's directory (aligned with
>     rustc's single-file mod semantics). CHECK's stage table adds GlobalSlotAlloc + IrGeneration:
>     the old standalone path (the full pipeline) already performed IR generation, and
>     E3019/E1014/E1015 and others are produced only by ir_gen (verified by the runner gate). The
>     Check form of IrGeneration checks without consuming IR; the Script/Bin split is represented by
>     the None/Some value of module_key (the existing E3023 switch, ir_gen.rs:1660).
>     Monomorphization is not included in CHECK (it produces only resource-limit/internal errors and
>     has no corpus dependency). **〔Withdrawn on 2026-10-10 after empirical verification in
>     3.4.8〕**—the claim of “zero corpus dependency” was merely evidence from the absence of a
>     pathological-recursion fixture in the corpus; E3005 is the only compile-time defense for this
>     class of program. See note 14. Incidental fix for a latent defect: repeated harvesting of
>     registrations for embedded std units by the Registry arm overwrites the native half merged by
>     with_std() (false E1043)—embedded units now skip duplicate registration.
> 11. LSP single-file fallback unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` was deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI now use the same Driver.
>     Under SingleFile+CollectAll, Parsing collects all parse errors, retains the partial AST and
>     continues typechecking (the editor philosophy from the 4.2.3 ruling extends here; the stage is
>     not marked failed, otherwise topological skipping would prevent typecheck from ever running);
>     Typecheck dispatches the `check_module_collect_all` free function. The LSP single-file path
>     now adds proof (E4018), W-code warnings (W1001–W1005, pinned by tests), and IR-level errors;
>     lexical failures now report real diagnostics (the old synthetic “E0001 lexical error” text
>     disappeared with the deleted sequence—real diagnostics carry precise spans and lose no
>     information).
> 12. `check_module` / `check_module_collect_all` verified line by line (4.2.7, mandatory C5 item):
>     the two entry points had already converged separately—the mod.rs layer uses
>     `check_module_inner(ast, env, collect_all)` with a single boolean, and the checker layer uses
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`, where pass 3 and drain both run
>     and differences are carried by whether `collected_errors` is empty; the five collection points
>     in statements.rs (function body/use/for/block/while). Verification uncovered and fixed a
>     duplicate-diagnostic defect: collection points placed the first error in both
>     `collected_errors` and the Err return channel, and pass 3's add_error for Err produced the
>     same code, span, and message twice (or three times when nested); two independent mechanisms
>     were also found: duplicate annotation-validation signatures (parameter and whole-annotation),
>     and duplicate ownership-layer emission at the same point (E2014/E2018). The fix landed at the
>     module-result boundary: deduplication by (code, span, message)—a diagnostic is a positional
>     fact, and duplication carries no information; 63 corpus entries lost duplicate copies, with
>     zero drift in the run column and exit code (the “duplicate” in borrow_conflict_err is an
>     artifact of the baseline triplet format not containing a column number—the two E2018
>     diagnostics are legally at different columns).
> 13. wasm playground landed (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper is no longer involved. Error text is byte-for-byte aligned with the old `CompileError`
>     Display prefix (Parse error:/Type error:/Internal error:), producing no visible difference in
>     the playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (the 4.1
>     existing declaration, now with a real producer).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing the mono exclusion in note 10**). Empirical evidence
>     disproved the claim that “mono produces only resource-limit noise”: with pathological generic
>     recursion (`f(x)=f([x])`), the multi-file path without mono overflows the compiler process's
>     stack (0xc00000fd—the interpreter recurses in Rust, without a graceful runtime error), and the
>     mono depth gate is the only compile-time defense; standalone check took the full pipeline
>     before 4.2.5 and therefore already had mono, so the exclusion ruling introduced a silent
>     coverage regression (“zero corpus dependency” only proves that the corpus has no pathological
>     fixture). The “type-erasure fallback” assumption was invalidated by RFC-033's reflection
>     ruling—`^^List(Int)` requires the real identity of an instantiation, so monomorphization is
>     the foundation of reflection, not an optional optimization. Four design points: ①typecheck
>     produces an instantiation_request for qualified calls (`lib.f(x)`, the FieldAccess shape),
>     with generic_id using the qualified name and sharing the merged IR's namespace (currently only
>     bare Var is recognized, expressions.rs:2455); ②`Stage::ALL` topology changes to Linking before
>     Monomorphization, and MULTI_FILE's stage table gains Monomorphization at the end— consuming
>     merged_ir and aggregating requests across all units (qualifying the deferred bucket's
>     containing_fn), and mono is promoted from a per-module pass to a whole-program pass; the
>     single-file stage table remains unchanged (without Linking, the subsequence property is
>     preserved and IR snapshots have zero drift); ③CHECK gains Linking (pure merging—E3020 entry
>     validation remains in RoleClassification to prevent duplicate reporting) and Monomorphization
>     (pure checking without consumption, like the Check form of IrGeneration) as two stages;
>     ④resource protection covers cross-unit mutual recursion through whole-program BFS. The
>     Monomorphizer itself is unchanged (its contract for consuming ModuleIR + a request set happens
>     to match the merged IR shape).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the #14 design: `ExpressionInferrer` receives a module-qualified key
>     table (the three `use` arms register ordinary / alias / item-by-item alias, with only
>     SubModule exports eligible), and `callee_generic_name` is concatenated through
>     `SymbolTable::qualify` using a qualified name with the same source as the merged IR; the first
>     qualified-call path's arity criterion was tightened (when the parameter table cannot be
>     resolved, use the second path to obtain arguments from the signature, avoiding false E3018
>     reports caused by same-named functions in different modules). Implementation empirically
>     uncovered and fixed **three asymmetric defects** in the mono side in the same round: (1) The
>     deletion-key (generic-name set) and rewriting-key (containing_fn + name + span triplet) had
>     mismatched granularity—a site whose arguments could not be resolved degraded into a symbolic
>     request in the deferred bucket; when the containing function was non-generic, the bucket never
>     drained, yet the original had already been deleted by name → dangling call (runtime E6006
>     demonstrated by `list_ops.yx:53`); the fix restores the gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the sole source
>     of truth to determine coverage and retaining the original at uncovered sites. (2)
>     Nondeterministic function-table order caused by iteration order over the restoration segment's
>     `HashSet` (t2 review F1) → changed to `Vec` and sorted by name, making the output
>     byte-for-byte reproducible. (3) Non-rewritten forms such as TailCall / MakeClosure still
>     looked up mapping keys unchanged (despite the comment claiming conservatism) →
>     `call_form_is_rewritten` is now the single decision point, and all non-rewritten forms are
>     counted as uncovered. Each defect has a pinned test (empirically demonstrating discriminative
>     power in the red state). The corpus differential baseline was regenerated in the same commit
>     with exactly one line changed (the `remove_at` frame gained `(int64)`). **Commit-strategy
>     note**: when typecheck collection and the mono fix were split into two commits, the staging
>     tree containing only the former triggered `stdlib_docs` failing due to an existing HEAD defect
>     (`list.is_empty([])` from the same E6006 family)—the two halves are each prerequisite for the
>     other's green state, so they were combined into one atomic commit.
> 16. Five rulings registered on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—user-module
>     exports carry `Export.type_params` through `generic_fn_type_params_snapshot()`, isomorphic to
>     the std path (`yx_sources.rs:62/108`): the declared name becomes the authoritative source for
>     qualified-call monomorphization, while signature reverse inference is demoted to a fallback
>     (included in R2's scope). (2) **HKT is live on the declaration side but broken on the call
>     side** (inferred type arguments E1002 / explicit type arguments E1010) registered as WBS 3.5
>     (three third-level tasks). (3) **The deferred bucket never drains under a non-generic
>     container** is included as an acceptance subitem of 3.4.8 ②: the bucket must drain or fail
>     loudly, and non-specialization must not be silent. (4) **Calling a closure variable by name
>     yields runtime E6006** registered as WBS 3.6; semantics (i) support indirect calls / (ii)
>     reject at compile time, with **(i) chosen**—support indirect calls: the `CallDyn` mechanism
>     already exists (the closure-expression path at `ir_gen.rs:7945`); what is broken is the `Var`
>     callee being degraded to a name-based `Call`, even though typecheck already accepts such a
>     program. (5) **Trade-off criteria are fixed**—a rejection rationale may be based only on
>     correctness and readability; “large change surface / introduces a new mechanism / high cost”
>     does not constitute a rejection rationale and belongs in the second part of `coding-rules.md`
>     and the HOWTO self-check list; trade-offs requiring a user decision must follow the “mandatory
>     when requesting a user ruling” section of `AGENTS.md` with all four items (situation +
>     evidence / pros and cons of each option / explicit recommendation + cost / explicit
>     identification of breaking changes).
> 17. 3.4.8 ② landed with F1 root cause (2026-10-11, 375789d2 + d922a551). ② implemented according
>     to the #14 design: `Stage::ALL` changed to Linking before Monomorphization, and MULTI_FILE's
>     table gained mono at the end; the Linking arm aggregates whole-program requests **before
>     taking each unit's IR** and qualifies `containing_fn` (otherwise the deferred bucket's key and
>     merged IR have different sources and the bucket never drains). **① closure (D2 ruling A)**:
>     user-module exports carry the declaration-time type-parameter snapshot through
>     `Export::type_params`, isomorphic to the std path (`yx_sources.rs:62/108`). **Existing defects
>     empirically uncovered and fixed during implementation**: (a) **F1**—`qualify_names` rewrites
>     only function bodies, and `ir.init` is never visited; under multi-file compilation, function
>     definitions are qualified while top-level module calls retain bare names → runtime E6006
>     (**reproduces even without generics**, unrelated to generics); fixed by making init reuse the
>     same `rename`/`aliases` and the same `rewrite_call_names`. (b) The mono side's “whole-program
>     unique suffix” heuristic and silent `continue` on a miss → removed the heuristic and changed a
>     miss to a **loud E3018** (silent ambiguous mismatch is worse than failing to resolve). **An
>     independent review caught and fixed a regression**: t6 found that a named import
>     `use lib.{pick}` followed by a bare-name call was incorrectly rejected with E3018 (the IR
>     instruction name is the alias's qualified name, while the request name is bare; the
>     aggregation side qualified only by unit key) → fixed by having the aggregation side **call
>     ir_gen's** `import_aliases_from` and `resolve_call_target` directly (extracted as a public
>     single point, with one producer and one consumer), and by pinning both named-import forms with
>     a matrix test. **Record**: the blank DefId in the init section was empirically a “consistency
>     blank” rather than a live defect (the codegen name fallback resolves it, and all four
>     init-call forms were empirically tested), and is registered under WBS 3.7.4 to close the
>     policy split between “function bodies use def” and “init relies on names.”
> 18. Init-section DefId policy split closed (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` was
>     extracted into a single-point `resolve_static_targets` (Call/TailCall string operands +
>     MakeClosure), and function bodies and `ir.init` share the same implementation and the same
>     `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)` pairs
>     changed (other-lines 0; 0 residual `(def=_)` entries across the whole tree's init sections).
>     The codegen DefId primary path and the two-state name fallback produced identical translation
>     results and execution values item by item (all four init-call forms tested empirically), and
>     the corpus baseline had zero drift; the probe was flipped and strengthened by the new policy
>     (identity + precise index + execution value; removing fallback-init coverage makes it fail).
>     **Erratum (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase`—the
>     original b554497b/e60dcc87/b7ac7396 correspond respectively to the rebased
>     375789d2/d922a551/38bd4b9e; the pushed dev history uses the new numbers (the old numbers were
>     local temporary numbers and never entered remote history).
> 19. 3.4.8 ③④ landed and verified (2026-10-11, 8ecf56f93). (The design basis for ③④ is in #14; the
>     implementation of ② is in #17, which this item closes.) **Implementation**: `CHECK_STAGES`
>     ends with `…IrGeneration → Linking → Monomorphization`; Check's `Linking` performs only pure
>     merging (aggregate requests + `link_module_irs`), with the entire entry-validation block left
>     to `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (memory only);
>     mono failures enter Check's collection channel, while MultiFile retains `fail_with`;
>     `into_outcome` first takes `check_diagnostics` and only then evaluates failure (structural
>     failures no longer clear already collected diagnostics). **Verification**: four independent
>     verification rounds (t3/t4/t10 + a t5 cross-form matrix) and one review (t6) all passed;
>     corpus differential 340/340 line-by-line IDENTICAL, IR snapshots have zero drift, test count
>     2643→2656. **An existing gap uncovered and fixed in the same round**: F1 (the request name and
>     IR definition key in rootless multi-file check had different sources → silent failure to
>     specialize; see the implementation note in WBS 3.4.8's row)—the aggregation side must look up
>     by the actual definition key precisely, not heuristically; the first version of the fix looked
>     up precisely by the actual definition key (`flat_defs`); **same-day erratum**: that bridge was
>     later deleted during regression repair, and the root cause was decoupled by giving
>     `module_key` a single responsibility (`script_mode`), so all three share the source
>     `{unit key}.{name}`. The location and fix for this regression (CI documentation-example gate:
>     258 blocks / 10 failures → 0 failures) are described in WBS 3.4.8's implementation note.
>     **Record (existing, not a regression from this round)**: E3005's help text does not match
>     resource-protection semantics; the multi-file E3005 span label is misplaced (reproducible from
>     the HEAD binary); the position of type parameters in the request-collection surface depends on
>     the same surface (assigned to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landed, with diagnostic-surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two later commits). **3.9.1 (execution-layer guard is reachable)**: interpreted
>     calls were changed to **explicit stack driving**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` push frames and return control to the outer `step_one`; frames carry
>     `ReturnTarget{Host|CallerSlot|Discard}`, and on return they write back to the caller's slot,
>     with multiple calls in one instruction continuing through the frame's `pending_calls`.
>     Baseline measurement showed each interpreted call layer consumed **2544 B** of native stack,
>     with the crash threshold at `count_down(394)` (Windows main thread: 1 MiB); after the fix,
>     20,000 recursive layers complete on a 512 KiB stack thread (approximately 0 B per layer). The
>     guard (`push_frame`) is therefore reachable from every entry point and under every
>     `max_stack_depth` value; `ExecutorError::StackOverflow` was changed to carry the real limit
>     (previously `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2 (deep-stack echo
>     truncation) was merged into this task by user ruling** (same location in the same file and
>     inseparable by file): echo now shows the first 16 frames + an omission count + the last 8
>     frames; shallow-stack output is unchanged. **3.10.1 (no compile-time guard for native
>     recursion, #447)**: locate first, then fix—the crash point was recursive `MonoType::type_name`
>     (**328 B/layer**, determined with an SP probe + per-operation threshold analysis); the same
>     path also has derived `Clone` (~400 B/layer), a deep copy in `substitute`, `type_name_hash`
>     (164 B/layer), and `collect_generic_type_refs`; after converting all four to work lists and
>     removing one full-request deep copy per layer, the pathological generic recursion with
>     `mono.max_depth=5000` produces **E3005 (including the real limit)** and no longer overflows
>     the stack. **3.11.1 (preserving task-boundary error codes, #448)**: the task-result error side
>     was changed from a value to a **structured form**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), and values
>     are restored by type across the boundary; E6001/E6003/ E6004/E6008 within tasks are
>     word-for-word identical in Standard and Embedded modes. Two **pub API payload-shape changes**
>     include `BREAKING CHANGE` in the commit. **Commit-strategy note**: 3.9.1 and 3.11.1 are
>     **inseparable at file granularity**—`interpreter/executor/executor.rs` contains both the
>     trampoline and the error-channel producer/consumer, and `TaskFailure` is defined in
>     `runtime/engine.rs`; staging only one side makes compilation fail (pre-commit compiles the
>     staged subset and rejected it twice), so they were combined into **one atomic commit** under
>     the same policy as #15. **Known boundaries and follow-ups**: derived Drop (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; the E4012 (constant-evaluation) gate carries native
>     recursion (approximately 3,696 B/layer; the default 256 layers ≈ 924 KiB), with zero call
>     sites and unreachable in production, so future wiring must close the same family; the
>     Standard-mode task interpreter constructs its default `ExecutorConfig` ⇒ its E6004 limit is
>     always 1024 (configuration propagation is registered as WBS 3.12, user ruling B: separate
>     team); the test-runtime budget is tracked in a separate issue #449 (pathological fixture 76s,
>     clippy dual-target, full submission run), and is outside this RFC's implementation scope.
>     **Corpus-directory stratification (same round, 8605a081b)**: `02-type-system` (78) and
>     `06-compile-errors` (134), totaling 212 fixtures, were split into 23 category-based
>     subdirectories; 88 IR snapshots were moved to mirrored paths; the corpus baseline was
>     regenerated in the same commit (342 lines, independently recomputed and compared line by line:
>     only the `file` field and 2 new entries changed), and 41 snapshots were regenerated because
>     the file-header path became longer; line-by-line verification showed **only span-offset
>     changes** (zero differences after normalizing offsets).
> 21. LSP semantic-data pipeline unified (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same** compilation, and cross-file references within a project carry
>     `resolves_to` pointing to the real definition— the old form triggered **two separate
>     compilations** on didOpen/didChange: diagnostics used the unified Driver pipeline, while
>     semantic data used a manual lex→parse→single-file typecheck in server.rs (no module registry,
>     project-local `use` resolution failed, reference events were not produced, and cross-file
>     navigation/rename/find-references silently returned nothing—the silent failure registered
>     under D55 was closed by this task). **Implementation**: (1) `Export` carries `definition_span`
>     (harvested by extract_module_info from AST identifier nodes, with the same source and value as
>     typecheck's definition event `def_span`; native std always has None), and `ModuleInfo` carries
>     `definition_file` (filled by the Registry arm from DiscoveredFile); (2) when draining
>     reference events, the checker redirects cross-file import items' `resolves_to` from the `use`
>     line to the real definition location through `import_def_locations` (std imports have no
>     registration and remain on the `use` line); (3) the checker's module identity changed from the
>     unified `"<module>"` to the unit file path (this value is consumed only on the semantic-data
>     surface and has zero diagnostic consumers—empirically all 4 `env.module_name` consumers); (4)
>     `DriverOutcome` gains a `semantic_db` channel—CollectAll forms (the Lsp target file regardless
>     of diagnostic success + SingleFile fallback) feed into it, and the Lsp arm's tail injects
>     top-level export definitions for non-target files from the registry (`external_definitions`,
>     enabling navigation even for files that have not been opened; type/ submodule exports are not
>     injected—typecheck-side type registration does not produce definition events, and both sides
>     preserve the same absence); (5) the orchestrator gains `analyze_source_in_project`
>     (`check_source_in_project` delegates to it and takes only the diagnostic half); (6) the LSP's
>     `analysis.rs` single-compilation entry point replaces the manual pipeline
>     (`update_semantic_db` was deleted from server.rs), and World keys are unified as normalized
>     file URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is an idempotent normalization, with
>     drive-letter case and %XX encoding differences normalized to the same key). **Incidental fix
>     (D59 diversion B class—old behavior silently did nothing and did not change the externally
>     visible behavior of any previously successful program)**: cross-file
>     navigation/rename/find-references within a project, which previously silently returned
>     nothing, now hit the real definition; lexical errors now retain semantic information via
>     partial-AST coloring instead of deleting it (the 4.2.6 editor-philosophy extension); when the
>     target file has type errors, semantic data is no longer discarded along with `type_result`
>     (coloring/navigation no longer disappears during an intermediate editing state). **Pinned
>     tests**: 6 at the driver level (target-file path key / cross-file resolves_to +
>     external-definition-surface round trip / std-import fallback to the use line / error-file
>     retention / single-file fallback path key + error retention), and 1 LSP end-to-end test
>     (demonstrated red before green—navigation previously returned None), plus 4 uri-normalization
>     tests.

**Changes to each of the ten entry functions (function by function)**:

| Entry point                                                | After the change                                                                                                                                                                                                |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to the Driver                                                                                                                 |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                         |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                    |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                      |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter the result to the target file                                                                                                                                                |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                       |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                          |
| `lib::run_project`                                         | Unchanged (delegates to the Driver)                                                                                                                                                                             |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project: use `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                          |

**`check_project` and `compile_project` have approximately 40% shared / 60% divergent code**. The
shared portion consists of vendor consistency verification, file discovery, `build_registry_from`
(deleted on 2026-10-26 after its last caller was removed; the historical account is retained here),
`all_method_bindings`, the parse loop, per-file checker assembly, and result handling. The divergent
portion consists of items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton
is precisely the Driver's value range**—all of the divergent 60% consists of either “a different set
of stages” or “a different aggregation mode,” and these are fully expressible through the two
`Program` fields.

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

Reason: skipping is determined by **topology**, not by the stage itself. If each stage returned
`Skipped`, the information “why did I not run?” would be scattered across 12 stages and could not be
centrally audited. Instead, `dispatch` determines this at the beginning of each loop iteration based
on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the defect targeted by this
document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”—it
produces no diagnostic, merely because the obligation happened to be empty. Once this rule is
established, every case of “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                           | Text direction                                                                                                                             |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Caused by an upstream `Abort`                                    | “Stage Y was not executed because upstream stage X failed”                                                                                 |
| Caused by an empty prerequisite obligation                       | “Stage Y was not executed because there were no pending obligations (normal)”—**severity = Info; does not count toward the warning count** |
| Conditions not met (for example, `config.mono.enabled == false`) | “Stage Y was not executed because it was not enabled by configuration”                                                                     |

The second category is critical: it distinguishes a “normal skip” from an “abnormal skip” in the
diagnostic stream without polluting `yaoxiang check`'s `warning_count` (the non-blocking contract
that `diagnostic/mod.rs:637-641` relies on).

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

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project and single-file files in the same process, while the CLI's `run` and `check` are
independent invocations. Placing it on Program avoids mutable global state in the Driver.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not only in early return, but
in that they are two different checker entry points. After unification, one implementation should be
driven by the `Aggregation` parameter rather than retaining two functions (**this is an additional
item requiring verification in this document; see “Known limitations and risks”**).

---

## Detailed Design

### Type-System Impact

| Change                                      | Type-layer impact                                                                                                                                             |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. It introduces no new type representation and does not touch `MonoType` / `PolyType` / `ir::Type`                                                    |
| `TypeCheckResult` → `Obligations` migration | **Field types remain unchanged**; only ownership changes. `release_plan` remains `ownership::ReleasePlan`, and `proof_calls` remains `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Entirely new types**, unrelated to the language type system                                                                                                 |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is in scope for
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: what moves into the ledger is **settlement
responsibility**, not ownership of the types. `ReleasePlan` remains defined in
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the remaining field types stay
in their original locations; `driver/obligations.rs` holds only the aggregate container and
`assert_drained()`. L3 consumers (for example, `ir_gen` reading `release_plan`) continue to receive
concrete field types through parameters and **must not `use crate::driver`**—consistent with the
no-reverse-dependency red line for the driver described below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend back on
`use crate::driver`**. The presence of `Driver` in a `TypeChecker` import is a violation and is
blocked by RFC-039's proposed `scripts/ci/check-module-boundary.py`.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**revision in 4.10.3, 2026-10-15**: CLI `run` no longer uses this pipeline for standalone programs—it now uniformly uses `compile_project` (the MULTI_FILE stage table), with Script semantics driven rootlessly; this row now describes only the library paths `lib::run_file` / `Compiler::compile`)                                                                                                                                                                                           |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected by the P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the multi-file `run` W-code set is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” and Check / Lsp / MultiFile share the same implementation with no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` and hands them to that arm for emission) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Connected by the P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| LSP (in-project)                    | No proof_execution                       | Connected by the P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Z3 not installed + single file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above; unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

**The only intentional behavior break** is replacing `.expect()` at `predicate.rs:35` with a
returned diagnostic. This conforms to RFC-027 §8's positioning as “not bound to a concrete solver”
and to the contract already declared by `backend.rs:60` (“callers should degrade conservently”)—the
current implementation contradicts its own contract documentation.

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

| File                                          | Line range                                      | Change                                                                                                                                                                                                                                                                                                                                                                           |
| --------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                  | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                            |
| `src/lib.rs`                                  | `140-147`                                       | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                  | `154-167`                                       | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                    | `141-227`                                       | Replace the `Pipeline::run` body with `Program` construction + a Driver call                                                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                    | `275-278`                                       | Move dead-code analysis out into a `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                    | `306-311`                                       | **Retain** `run_proof_execution`, move it into `driver`, and use it as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                    | `381-389`                                       | Move monomorphization out into a `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`         | `99-237`                                        | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`         | `273-399`                                       | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`         | `450`                                           | Change `check_source_in_project` to use the Driver                                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`         | `1374`                                          | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`         | `486-494`                                       | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` through the `Aggregation` parameter                                                                                                                                                                                                                                                 |
| `src/lsp/handlers/diagnostics.rs`             | `146-227`                                       | Delete `run_diagnostics`'s manual stage sequence and invoke the Driver                                                                                                                                                                                                                                                                                                           |
| `src/util/diagnostic/mod.rs`                  | `565-619`                                       | Uniformly use `Program { kind: Check }` in `check_files_with_diagnostics`                                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                  | `621-661`                                       | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                                   |
| `wasm/src/lib.rs`                             | `30-36`, `42-51`                                | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they reach path 1 implicitly through `Compiler::compile_with_source`)                                                                                                                                                                                                              |
| `src/frontend/core/sema/layers/predicate.rs`  | `34-36`                                         | Replace `.expect()` with returning `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                     |
| `src/frontend/core/sema/checker.rs`           | `1303-1314`                                     | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                                    |
| `src/frontend/core/sema/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/sema/checker.rs`           | `1244`                                          | Correct the comment that conflicts with `1320`                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/sema/layers/README.md`     | Entire file                                     | Change the layer-order table to the **actual** execution order; remove “a lower-layer failure prevents upper layers from running” (no short-circuit exists)                                                                                                                                                                                                                      |
| `src/frontend/core/sema/proof/smt/backend.rs` | `67-72`                                         | **Under D58**: Add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure-based access point; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared access point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`); D58 supersedes the original wording |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement` in
`checker/refinement.rs` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct, and
**the problem lies in the consumer side, not the producer side**; changing the producer would
conceal the architectural defect. Also unchanged: `Cargo.toml`; every
`#[cfg(target_arch = "wasm32")]` branch (wasm-branch reachability is assigned to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                           | Compatibility                                                              | Handling                                                                                                                                                              |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to multi-file paths                          | **Breaking**: constraints that previously passed silently now report E4018 | Write `test_multifile_proof_obligation_not_dropped` **red first**, then fix it; release in batches, with the expected diagnostic-set change recorded in the CHANGELOG |
| Add W1001/W1002/W1003 to multi-file paths                        | **Breaking**: programs that compiled successfully now produce warnings     | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`) and do not change the exit code                                        |
| The first run of `assert_drained()` produces W-level diagnostics | **Breaking**: the diagnostic set grows                                     | Move from W to E in two steps, S2/S4                                                                                                                                  |
| Change the `predicate.rs` panic to a diagnostic                  | **Improvement**: no longer crashes                                         | No breakage                                                                                                                                                           |
| `build` / `dump_bytecode` subcommands                            | **No impact**                                                              | These paths do not pass through proof_execution                                                                                                                       |
| Migrate `TypeCheckResult` fields to `Obligations`                | **Internal refactoring**                                                   | If the public API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                      |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contracts and a
unified Driver)** in RFC-039's global stage sequence. The following S1-S5 define the implementation
order within this document. **The acceptance criterion for every step cites category C2 of the
[equivalence-criteria document](07-equivalence-oracle.md)** (orchestration change: identical
**diagnostic sets** across entry points + identical corpus behavior).

### S1: Establish the criteria (red first)

**Prerequisite; cannot be skipped.** The defect criterion must first be written as a failing test.

| Deliverable                                                                                                                                                    | Acceptance                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red**. If it unexpectedly passes, the defect analysis in this document must be reviewed again       |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions can be compared                                |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                             | Marked `#[ignore]`; turn green in S4 (red-state record: after removing the sole settlement point, `pairs: 0`) |

**Rollback point**: no code changes; purely new tests.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                      |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array      |
| Change `pipeline.rs:141-227` to invoke the Driver        | **The single-file path's diagnostic set and exit code are byte-for-byte identical**                  |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic list (sorted by `(code, file, line)`), exit code, and stdout/stderr are **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into the Driver. The
acceptance standard is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Consolidate the orchestrator's four entry points + fix the defects

| Deliverable                                                                                                                  | Acceptance (C2)                                                                        |
| ---------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Change `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are normalized according to `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                     | `yaoxiang check` behaves consistently inside and outside a project                     |
| Delete the LSP manual stage sequence                                                                                         | LSP and CLI produce the same diagnostic set for the same file                          |
| **Turn `test_multifile_proof_obligation_not_dropped` green**                                                                 | The defect is fixed                                                                    |
| Fix the ownership-layer `Unproven` empty match arm (originally `checker.rs:1313`)                                            | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode  |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                   | No panic in an environment without Z3                                                  |

**Rollback point**: the defect fixes and structural consolidation must be **two separate commits**.
If structural consolidation causes problems, only the structural commit may be rolled back while
retaining the defect-fix commit—in that state, `test_multifile_proof_obligation_not_dropped` remains
green. The reverse—retaining the structure while rolling back the fix—turns it red and is an
unacceptable intermediate state that must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                         |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implemented shape in the §2 revision note; use that file as the authoritative count) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                       |
| Promote obligation diagnostics from W to E   | After the full corpus differential, manually review every new E                                                                                    |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration option;
switching between W and E does not require structural changes.

### S5: Finish the proof layer and wasm work

| Deliverable                                                                                                                                                                                                                                         | Acceptance                                                                                                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                    | The document corresponds line by line to the call sites in `checker.rs`                                                                                          |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout would be fixed across compilations by the process-wide cache and pollute shared state between cargo-test threads) | Production call sites of `default_solver()` drop to zero (tests only); cache hit rate across the three consumers is observable (the counter landed in 89576fafd) |
| Add a warning to the `TypeChecker::ownership_inputs` criterion point in `checker.rs` (the ownership block at `:1296-1340`, emission point at `:1531`)                                                                                               | A diagnostic is produced when there is no `body_checker`                                                                                                         |
| Assess the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                    | Submit the conclusion to the “F2 assessment” subsection of `06-cleanup-inventory.md`; this document only registers the assessment requirement                    |

**Note**: Correcting the layer order changes the diagnostic set and may expose a large number of
previously silent `Unproven` results. **This item should proceed independently of S1-S4** and must
not be mixed into the same PR as entry-point consolidation.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Stage model**          | `Stage` is a compile-time-exhaustively-enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                                      | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant makes `dispatch` fail to compile. This is isomorphic to the opcode-table mechanism already validated in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating questions about invocation counts that currently require human reasoning                                                                                                         |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement, together with the **static** gate `scripts/ci/check-obligations.py`; **terminal severity is E level** (user ruling A on 2026-10-13: promote the same code to E without changing the code number; **implemented shape in the §2 revision note**: the ledger does not copy the field structure; the red list—0 entries after cleanup—retains the mechanism but does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not a single bug: at least 2 similar hazards have been verified (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are in scope. Moving from W to E in two steps was intended to keep every step's C2 criterion usable—now that the obligation inventory has been cleaned to 13 entries and the red list is empty, the healthy state emits nothing, so the terminal state directly uses E (see sections 7 and 8 of the §2 revision note) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer) and **do not change** the production-side logic of the `Unproven` branches (now distributed among three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                           | The problem lies in the consumer side, not the producer side. Changing the producer would conceal the architectural defect behind “the logic is fixed,” while the logic of these branches is itself correct                                                                                                                                                                                                                                                                                |

This design additionally eliminates two classes of cracks:

- **The crack between “commentary promises” and “code behavior.”** The “silent pass must not be
  allowed to return” promise in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, enforced by `assert_drained()`.
- **The crack of “an array that somebody forgot to append to.”** Today, five functions each
  hand-write their invocation sequence. After unification, there is one stage table, and a newly
  added stage that is not wired in causes a compile-time error.

### Rejected Approaches

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two dead-code-analysis implementations, W1006, W1003, monomorphization, role context, and IR
  generation/linking) **do not involve type transitions**; they are configuration questions about
  “whether to run an analysis.” This approach also cannot express `Aggregation`, and would only move
  the `check_module` / `check_module_collect_all` choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this removes
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions each hand-writing their invocation sequence” with “an array that
  somebody forgot to append to,” which is precisely the cause of the current 11 inconsistencies.
  This repository already has an analogous counterexample: the `ModuleDependencyGraph` /
  `affected_modules` / `ModuleCache` / `HotReloader` described under `docs/src/dev/design/check/`
  all have zero implementations.
- **Fix only the bugs without changing the structure**—this would fix the 2 known bugs but cover
  only 1 of the obligation fields, while `Stage` would remain scattered across 5 functions and the
  next divergence would continue to grow from there. **This must be done first** (as part of S1/S3),
  because structural refactoring requires a known red test demonstrating that the criterion is
  effective.
- **Make `proof_calls` public and add a `debug_assert`**—`check_module` is a general entry point and
  does not know its caller, so `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  warns only when the field is discarded **as a whole**. This searches for the bug at the wrong
  layer: the bug is in the orchestration layer, and detection must occur in the orchestration layer.

## Known Limitations and Risks

- **Slimming `orchestrator.rs` will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactor that “centralizes the
  wiring.”
- **S3 will significantly change the diagnostic set, in the direction of “exposing previously silent
  problems.”** After the fix, users may report a batch of “new errors”—these are real bugs that
  simply were never reported. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires field-by-field manual judgment.** Some
  obligation fields, such as `module_namespaces`, may be intentionally unnecessary to consume and
  should not generate warnings. S4 must review every field rather than applying a one-size-fits-all
  policy.
- **The `Program` abstraction may be premature**. If some entry points' stage sets remain unstable
  for a long time, `stages()` may degenerate into a free parameter that accepts a different array on
  every call, causing the contract constraint to lose its effect. **Mitigation:
  `test_program_stage_coverage` asserts that `stages()` can only come from the 6 predefined
  combinations.**
- **Dependency on criteria**: the defect-fix criteria for S1/S3 depend on the defect test in the
  [equivalence-criteria document](07-equivalence-oracle.md) being red first. **If that document has
  not first established a multi-file corpus layer, S1 cannot be accepted**—the existing 293 corpus
  items all take the single-file path and provide zero coverage for this class of defect. This
  document does not cover the IR verifier (`verify_loose`); that prerequisite work is outside its
  scope.
- **Correcting the layer order will expose previously silent `Unproven` results** (`equivalence` is
  not in the pipeline at all, and the `termination` and `ownership` order is the reverse of the
  declared order). This is a diagnostic-set change risk and must proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undetermined**.
  `backend.rs:13-19` states that `Solver` is `Send` but not `Sync`, and the `Z3Backend` cache uses
  `RefCell`; after changing it to a singleton shared across compilation units, all access paths must
  be confirmed to pass through `Mutex`.
- **Stage parallelization has not been evaluated**. Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependent defects**, such as the actual order
  dependency between termination and ownership. This should be considered only after the equivalence
  criteria have stabilized.

> **All open questions originally listed in this section have been ruled on.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) for the individual
> decisions (D1-D50). **No unresolved items remain in this document.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm-branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactoring (implementation steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-level criteria, and the
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/sema/checker/refinement.rs` — explicit declaration that “silent pass must not
  be allowed to return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/sema/layers/README.md` — layer invocation table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/sema/layers/predicate.rs:31-32` — explicit rationale for hard failure during
  SMT initialization
- `src/frontend/core/sema/proof/smt/backend.rs:60` — contract declaration that “callers should
  degrade conservently” (conflicts with `predicate.rs`)
- `src/frontend/core/sema/types.rs:16-80` — 14 fields that downstream code must consume (after the
  3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/sema/tests/rfc027_refined_transparency.rs:13-15` — the unit test's own
  acknowledgement that it “stops before pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  the corpus's own acknowledgement that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground's
  fourth compilation caller (single-file path, so it executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — build-time error-code gate, an example of this project's enforcement mechanism
