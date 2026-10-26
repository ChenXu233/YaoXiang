# Compilation Stage Contracts and the Obligation Ledger

> **Companion design document.** This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main RFC-039 document for the four-layer model, acceptance-criterion tiers, and
> execution-stage order; see the [index for this directory](index.md) for the positioning of each
> companion document.

## Positioning and Scope

This document is the implementation blueprint for the **L1 orchestration layer** of RFC-039. It
addresses one problem: **“which compilation stages ran, which did not, and why” is currently
distributed across six paths and ten entry functions, each manually wired on its own, with no single
place able to answer the question.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being broken by the architecture rather than by the
> logic.**

The original comment inside `revalidate_refined` in `checker/refinement.rs` states (originally
`checker.rs:5165`, relocated by P5 5.2.1): “Unproven → compile error, no degradation, no silent
pass……must not let silent pass return.” However, this invariant holds only on the **single-file
path**: proof-function constraints such as `x: Sorted(3)` silently pass on the multi-file
`yaoxiang run`, `yaoxiang check`, and LSP paths; the proof function is never executed, and no
diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustively enumerable, with no runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- A unified `Driver` with a single `dispatch`, plus the individual migration path for each of the
  ten entry functions;
- Stage-failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic-aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by these changes, compatibility impact, and implementation
  order.

### Not Covered

- The four-layer model, dependency-direction rules, and anti-rebound gates → `01-routing.md`
- Equivalence criteria (the C1–C6 tiers and three criterion layers) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead-code and wasm branch-reachability cleanup → `06-cleanup-inventory.md`
- Global execution order for P1–P10 and G1–G10 acceptance gates → the main RFC-039 document

### Division of Responsibility with RFC-039

RFC-039 explains **why** the refactoring is needed and **in what order** it should be performed;
this document defines L1’s **concrete form**, its **per-file change list**, and the **implementation
stages within this document** (corresponding to P3 “Fix correctness defects” and P4 “Stage contracts
and unified Driver” in RFC-039’s global sequence). If this document conflicts with RFC-039, RFC-039
takes precedence.

Equivalence criteria are applied under the **C2 (orchestration changes)** category of the
[equivalence criteria document](07-equivalence-oracle.md): the diagnostic sets of all entry points
are identical + corpus behavior is identical.

## Current State

> Everything in this section consists of **verified facts**, each backed by a file path + line
> number. Line numbers are based on `9e02e4db`.

### Stage boundaries are the sole structural defect where “one wrong connection silently affects the entire repository”

This differs in nature from the other two classes of defects, module boundaries and test wiring:

| Defect                   | Typical manifestation                    | Is there a signal?       |
| ------------------------ | ---------------------------------------- | ------------------------ |
| Lexical/syntactic error  | Incorrect source code                    | Yes, a diagnostic        |
| Type mismatch            | Incorrect types                          | Yes, a diagnostic        |
| Missing stage connection | A stage is never called at all           | **No signal whatsoever** |
| Unconsumed obligation    | A field is populated but nobody reads it | **No signal whatsoever** |

The common characteristic of the latter two is: **failure does not produce an error**. They
therefore cannot be solved by “writing the code more carefully” or “reviewing it more strictly”—code
review can reveal what was written, but not **what was not written**. This is precisely the root
cause diagnosed by RFC-039: “this project treats ‘design’ as a documentation convention rather than
as an executable constraint.”

### The repository already has enforcement mechanisms

The same repository already contains mature enforcement mechanisms; they simply have not been
extended to the stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are subject to a
  **build-time hard gate** through `build.rs:19-55` and `tools/code-tables`. Each entry is compared
  against the RFC-013 code table, and any discrepancy directly triggers `panic!` and refuses to
  compile.
- `src/package/` (**76 files / 13,012 lines**, including 6,227 lines under `tests/`, or 47.9%) has a
  high test density, and every module’s test subtree is explicitly wired up. This is the most
  completely wired test area in the repository and can serve as a formal reference for a stage-layer
  gate.

**The design capability is sufficient. What is missing is “an equally strong gate at the
orchestration layer.”**

(Line-count convention: `(Get-Content).Count`; see the “Line-count convention” section of
`06-cleanup-inventory.md`.)

### Six compilation paths and ten entry functions

| #   | Path                       | Entry function                                        | Location                                         | Which compilation path is used                                                     |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Five stages called directly                                                        |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file checking**    | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collecting all diagnostics                                     |
| 3c  | **In-project LSP**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + independent IR                                           |
| 4a  | **CLI single file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multiple files**     | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
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

The following table provides evidence cell by cell. **An empty cell does not mean that the entry
point “does not do this”; it means that “no code in that entry point does this”**—which is precisely
the essence of the defect.

| #   | Stage / behavior                                         | `pipeline` (single file)                            | `compile_project`                                              | `check_project`                                                       | `check_source_in_project` (LSP)    | `compile_embedded_module`                      |
| --- | -------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                         | **No**                                                                | **No**                             | **No**                                         |
| 2   | Dead-code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                         | Yes `356-377` (role-aware, no configuration gate)                     | **No**                             | **No**                                         |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                         | Yes `336-348`                                                         | **No**                             | **No**                                         |
| 4   | W1003 unused imports                                     | Yes (collects `type_result.warnings` at `277`)      | **Collected but never emitted**                                | Yes `350`                                                             | **No**                             | **No**                                         |
| 5   | W1001/W1002 dead-code family                             | Yes (same as 2)                                     | **No**                                                         | Yes (same as 2)                                                       | **No**                             | **No**                                         |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                         | Not applicable                                                        | Not applicable                     | **No**                                         |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **returns on the first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collecting everything | `check_module_collect_all` (`493`) | `check_module` (`1386`)                        |
| 8   | File discovery                                           | Not applicable                                      | `discover` (`101`, discards `used_by`/`shadow_events`)         | `discover_with_used` (`275`)                                          | `discover` (`454`)                 | Not applicable                                 |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                         | Yes (`321-328`)                                                       | **No**                             | **No**                                         |
| 10  | Global slot allocation                                   | Not applicable                                      | Yes `allocate_global_slots` (`147`)                            | **No**                                                                | **No**                             | **No**                                         |
| 11  | IR generation + qualified-name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                                | **No**                                                                | **No**                             | Yes (independent ModuleIR followed by merging) |

> **Review note (WBS 3.4.3, cell-by-cell verification on 2026-10-07):** This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is retained unchanged. The state after P3 landed differs
> as follows:
>
> - **Row 1 has been fixed**: all five entry points share the same `frontend/proof_execution.rs`
>   implementation (pipeline + four orchestrator entry points); the sole-consumer defect no longer
>   exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, user ruling “do it” on 2026-10-19): multi-file `run`
>   now emits W1003 and the dead-code-family warnings, and the W-code sets of the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) are normalized—the
>   criterion has a single source in the shared implementation of the `dead_code_analysis` arm.
> - **Row 6** (monomorphization exclusive to the single-file path) → assigned to WBS 3.4.8
>   (prerequisite 4.1.3; potential risk not proven to be a defect).
> - **Row 7**, the `check_module` / `check_module_collect_all` dual entry point → assigned to 4.2.7
>   (`Aggregation` parameter-driven).
> - The code-side reference to “ruling #434” previously had no registration in the docs—it has now
>   been registered as RFC-039 **D57**.
> - **Asymmetry in embedded std inclusion (new fact outside the table, added on 2026-10-09)**: the
>   single-file path unconditionally injects std.list through `merge_embedded_std_ir` (required for
>   desugaring `for` loops; #117 hard switch), whereas multi-file `discover` previously recognized
>   only explicit `use`—in-project `for` loops compiled successfully and then failed at runtime with
>   E6006 (confirmed by probe). This has been fixed and WBS 4.10.2 closed; the related “multi-file
>   parse hard-abort peculiarity” has been registered as WBS 4.10.1—**and has also been fixed**
>   (landed immediately after 4.2.2; the Check path was degraded to per-file collection; this audit
>   method examines the “stage coverage/field consumption” dimension, so this difference in
>   compilation-unit membership is an overlooked dimension).
>
> - **Embedded std registry surface coverage (discovered during 4.2.5 implementation, fixed)**: the
>   Registry arm repeatedly harvested registrations for embedded std units through
>   `extract_module_info`, overwriting the native half registered via `with_std()` (`result.is_err`,
>   etc. disappeared → std.test falsely reported E1043). This is the third overlooked dimension of
>   this audit: the merge semantics of “two registration sources for the same module key”—the
>   single-file path forms its registry only once through `with_std()`, whereas the multi-file/Check
>   Registry arm obtains coverage only by inserting each unit separately.

**Two points require precise wording; otherwise the conclusion would be wrong:**

- **The precise conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
  (`run_file` → `Pipeline::run`) it does. The reason lies in `compile_project`: it compresses the
  complete `TypeCheckResult` (including `warnings`) into `type_results`, but the function **has no
  read site for `result.warnings` anywhere**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for entry point 11 also has a different source**: `check_project:385-395`
  uses `surfaces.bins` (the manifest-declared surface), whereas `compile_project` uses `is_bin_role`
  (`250-252`, which only determines whether a manifest exists). The two functions give different
  answers to “which files must define `main`?”

### Correctness defect: proof obligations are silently discarded (complete evidence chain)

**This is the core of this document. All eight steps below are independently verifiable.**

**Step 1 — There is exactly one production point for obligations.**

`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire
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
the proof function must actually be executed to decide the result.”

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
> `This branch therefore becomes a real line of defense rather than a prospective fallback: silent pass must not return.`

In other words, the author knew that “not producing a diagnostic” was a defect and explicitly
classified it as a **known intermediate state with nowhere to go**, assuming by design that “someone
must eventually read `proof_calls`.”

**Step 3 — The field really is placed into the result.**

Inside `check_module_impl`, a local `proof_calls` is declared and collected (originally
`checker.rs:1234-1235`), then written into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration of proof-function obligations` (originally `:1439`). The
field is defined in `types.rs:29`.

**Step 4 — There is only one read site in the entire repository.**

In production code, `TypeCheckResult.proof_calls` (`types.rs:29`) is read at **only one site**:
`src/frontend/pipeline.rs:187` (passed as an argument at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(The other repository-wide hits for the `proof_calls` identifier fall into three categories, none of
which consumes this field: three production-side collections in `checker.rs` and
`checker/refinement.rs` (originally `:1234/4564/5494`); the identically named field in
`verdict.rs:61`; and `tests/rfc027_*.rs`, which reads `ProofResult`.)

**Step 5 — None of the orchestrator’s four entry points passes through that layer.**

The four entry points in `src/frontend/module/orchestrator.rs`—`compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module`
(`1374`)—**do not call `pipeline.rs`**. They call `TypeChecker::check_module` directly (`124` /
`493` / `1386`). They therefore cannot even reach the sole read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations in the standard library itself also take the discard
path.**

`compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is responsible for
compiling embedded std. This means **the embedded std module’s own proof obligations are likewise
never executed**.

**Step 7 — Consequence: constraints silently pass.**

`y: Sorted(3) = 5` (`Sorted: (x: Int) -> Type = { ... }`) **compiles successfully, runs
successfully, and produces no diagnostic** on the multi-file, check, and LSP paths. The proof
function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point inside the layer.**

When `check_module_impl` handles the ownership-layer result (originally `checker.rs:1303-1314`):

```rust
// 原 checker.rs:1313（现 check_module_impl 内）
ProofResult::Unproven { .. } => {}
```

**The ownership layer’s `Unproven` is swallowed by an empty match arm, with no diagnostic and no
accounting.** This directly contradicts the `5164` branch’s promise that “silent pass must not
return,” and it is **independent of the orchestrator problem**—even if the entry-point layer is
completely fixed, this point will remain silent. (Timing: it should be handled in the same batch as
the obligation ledger because it is the same class of problem.)

### Why the tests did not catch it

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]` functions) has zero hits for the three
keywords `Sorted`, `proof`, and `refin`.** Proof obligations on the multi-file path have zero
coverage.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, and `rfc027_return_refinement.rs`) do assert that `proof_calls` is
non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that the `result.proof_calls`
contains a `SumUpTo` call. However, they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`) and therefore **stop exactly before `pipeline.rs`**.

The test files’ own documentation comments already state this.
`rfc027_refined_transparency.rs:13-15`:

> `This file asserts only what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer.`

**This is exactly the shape of the problem: the tests verify that “the obligation was populated,”
while the bug is that “the consumer did not read it.”** A test that exercises only the producer and
not the consumer is inherently immune to this class of defect.

### Stronger finding: the subject corpus of the equivalence criteria is the single-file path

The [equivalence criteria document](07-equivalence-oracle.md) uses end-to-end differential testing
over the 293 `.yx` corpus files under `tests/yaoxiang/` as the **third criterion layer** and the
primary acceptance method for C2 stages. In practice:

- There is **no `yaoxiang.toml` anywhere under the `tests/` directory** (zero matches from a
  whole-directory glob).
- Therefore, `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) takes the `standalone` branch
  for every corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, and
  none cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` provides
concrete evidence of this point. The file is annotated with `// expect: compile-error E4018` (line
15), and its header comment states on line 7 that its status is “❌ should be rejected at compile
time”—**it passes precisely because it takes the only path on which the proof function is
executed**.

**Conclusion: the third criterion layer needs an additional multi-file corpus layer; otherwise it
cannot serve as the acceptance tool for this document.** See “Implementation Points” S1.

### Obligation fields: 13 “production implies contract” fields with no enforcement mechanism

The `TypeCheckResult` in `src/frontend/core/typecheck/types.rs:16-76` contains **13 fields** whose
documentation comments explicitly state “produced by stage X → consumed by stage Y”; in other words,
they are inherently **cross-stage obligations**:

| Field | Definition line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-number convention**: the definition line = the **declaration line** of the field in
> `types.rs` (the `pub` / `pub(super)` line), not the first line of its documentation comment—a
> field’s comment block may span multiple lines, and only the declaration line provides a unique
> anchor. The **struct range** in the sentence above this table
> (`src/frontend/core/typecheck/types.rs:16-76`) floats with the struct’s line number and is
> mechanically reconciled by gate **F5** (an error turns the gate red); definition lines are
> reconciled by **F4**. | `proof_calls` | `36` | typecheck → **proof_execution** | | `release_plan`
> | `38` | ownership → IR generation (**`PlanId`-keyed**: read at `ir_gen.rs:360`
> (`type_result.release_plan.clone()`), used at `:3103` (`ReleasePlan::consume`); completion marker
> at `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | | `instantiation_requests` |
> `42` | typecheck → monomorphization (`pipeline.rs:381`) | | `existential_coercions` | `44` |
> typecheck → IR generation | | `implementation_proofs` | `46` | typecheck → IR generation | |
> `sum_types` | `54` | typecheck → IR generation | | `sum_type_param_names` | `57` | typecheck → IR
> generation | | `operator_dispatches` | `59` | typecheck → IR generation (span-keyed) | |
> `overload_resolutions` | `62` | typecheck → IR generation (**`PlanId`-keyed**—call-site identity:
> produced at `inference/expressions.rs:4550`, consumed at `ir_gen.rs:7697` / `:7772`
> (`OverloadResolutions::get`); completion marker at `:1097`) | | `try_expr_impls` | `65` |
> typecheck → IR generation (span-keyed) | | `match_scrutinee_types` | `69` | typecheck → IR
> generation (span-keyed) | | `module_namespaces` | `72` | typecheck → IR generation |

**Reclassification record (3.4.9, user ruling A, 2026-10-13)**: the original row 14,
`interface_impl_registry` (`types.rs:49`), has been **reclassified as an in-tree fact**—by design it
is consumed within a phase and is not read across stages, so it is not an obligation:

- **In-phase consumption sites (live)**: `operator_interfaces.rs:623` (operator lookup reads
  `env.interface_impl_registry`), seven constraint-solving lookups in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, and injection into `body_checker` at `checker.rs:974`.
- **Zero cross-stage reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; the `src/driver` side contains only the ledger’s own registration and documentation
  (removed in this round).
- **Disposition**: delete the **claim** in the documentation comment that it is “a mirror for
  consumption by the middle layer/LSP”; retain the field and its in-phase consumption. The field is
  registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with a rationale). If LSP
  genuinely needs the table in the future, register it through 4.6.x; **do not create a consumer in
  this round**.

**All these fields are passed across layers keyed by `Span` or by name tables.** The equivalence
criteria document has already pointed out that mismatches in span-keyed contracts can “fail
silently, without any error” (`ReleasePlan` is a typical example: if the two sides calculate spans
differently in any one place → the `Drop` instruction silently disappears).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan`, `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`), is inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the complete `&TypeCheckResult` to it**—the consumer
  is in a downstream module, and that downstream module lies on every entry point’s mandatory path.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, which belongs to **another
  entry-point implementation**. The orchestrator’s four entry points do not pass through the
  `pipeline.rs` layer at all.

**Verified fact**: the repository currently has **no mechanism whatsoever** that guarantees these 13
fields are consumed. The only “protection” is that `ReleasePlan` happened to hitch a ride with IR
generation.

### Registration of non-obligation fields: channel / in-phase / in-tree-fact fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + rationale), complementing the “§ Obligation
Fields” table above:

| Field                     | Why it is not a cross-stage obligation (summary)                                                                                                      |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Compilation-unit identity metadata (zero hits in `src/middle`)                                                                                        |
| `diagnostics`             | Error-diagnostic channel (an existing channel, not “production implies contract”)                                                                     |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but its documentation comment does not declare a cross-stage contract ⇒ not brought under management   |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`); like `bindings`—no declared contract                                                     |
| `semantic_db`             | Consumer is in LSP (from 4.6.1, emitted through the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`); not a compilation-stage obligation |
| `trait_table`             | Has cross-stage reads at `ir_gen.rs:4336/:6867`; documentation comment declares no contract                                                           |
| `warnings`                | Warning-diagnostic channel (same category as `diagnostics`)                                                                                           |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; see the reclassification record in “§ Obligation Fields”)                                                          |

**Why this is not “wiring up a consumer”**: creating downstream consumers for these fields would
invent spurious obligations and disguise fields that “do not require cross-stage consumption by
design” as contract defects. The correct closure mechanism is **reverse containment**—the
`TypeCheckResult` field set = obligation inventory ∪ non-obligation registry, enforced as a hard
criterion by `scripts/ci/check-obligations.py`: adding a field without registering it, deleting a
field without synchronizing the registry, a non-obligation entry without a rationale, or overlap
between the two tables all immediately turn the gate red.

### Four additional contract defects in the proof layer

All of the following come from analysis of the proof layer and are independent of the entry-point
split, but all belong to the class of “stage contracts are not enforced.”

**(a) Layer order is declared only in documentation; there is no mechanism enforcing it, and the
`equivalence` layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (positioning is anchored by function name/driver point, with line
numbers taken from the current tree):

| Actual order | Call site                                                                                                                                                  | Corresponding layer |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement traversal driver `predicate::check_predicate` (the three sites `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                            | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                               | `ownership.rs`      |
| 4            | Type-constraint solving `env.solver().solve()` (not under `layers/`)                                                                                       | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                     | `equivalence.rs`    |

The layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) exist only in
each module’s own header comments and do not correspond to the call order; the README has no “number
→ call timing” mapping.

`equivalence` is used only as an `is_subtype` utility by `inference/assignment.rs:17` and has
nothing to do with `ProofResult`. At the same time, **there is no short-circuiting
whatsoever**—`check_module_impl` adds termination errors one by one with `add_error` and then
continues downward; “a lower layer failing means an upper layer does not run” is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: hard-failure panics were removed by P3’s 3.3.1 (the SOLVER slot was
> made optional, and absence conservatively degrades to `SMTResult::Unknown`); “silent skip (do not
> inject)” was supplemented with a W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is to be performed according to **RFC-039 D58**, at the actual fix location
> `proof/smt/backend.rs` (adding a process-wide shared singleton + a `with_shared_solver` closure
> entry point). The wording in the 02 change list that says `default_solver() → &'static` is
> superseded by D58 (a bare `&'static` reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                                                                                       | Acquisition strategy                                     | When the solver is unavailable                             |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ---------------------------------------------------------- |
| `predicate.rs:34-36`                                                                                           | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**  |
| `termination.rs` (injected during construction through `check_module_impl`, originally `checker.rs:1265-1268`) | Construct-time injection via `with_solver_owned`         | `None` → **silently skipped** (not injected)               |
| `ownership.rs:627` (back-edge-cut decision)                                                                    | **Every call** to `default_solver()`                     | `None => return false` (`629`) → conservatively do not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: “Initialization failure remains
a **hard failure**: softening it would misdiagnose ‘Z3 is not installed’ as ‘the constraint exceeds
the kernel’s capabilities’.” By contrast, the contract documentation at `backend.rs:60` says:
“`None`: backend unavailable……**the caller should degrade conservatively**.” **Two failure
philosophies coexist in the same codebase, and the hard-failure path panics instead of returning an
error.**

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

**This is a factory function, not a singleton**—every call performs `Z3Backend::new()`, creating a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`,
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-local**, while the documentation comment at
`z3_backend.rs:17` states that “SMT query results are cached in `cache`.” Because it is not shared
across calls ⇒ **the cache can never produce a hit in this invocation mode**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. It is the same
backend with two different lifecycle strategies.)

**(d) The checker silently degrades, and its comments do not match reality.**

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
whatsoever**. (Recommended handling: record a warning-level diagnostic, or at least leave a trace in
the obligation ledger.)

The comment in `check_module_impl` says termination checking runs “after type checking and before
constraint solving.” In reality, `self.env.solver().solve()` runs after it (originally
`checker.rs:1320`; termination originally `:1256`, ownership originally `:1296`)—**the “before” in
the comment is actually “after two layers.”**

### Current wasm state: hosted by a shim crate, with 29 files containing live branches

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
an independent shim crate.

| Fact                                                      | Evidence                                                                                                                                                                                                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The main crate contains only an `rlib`                    | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                    |
| **The shim crate provides the `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                    |
| The shim depends on the main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                           |
| The main crate has a wasm-target dependency section       | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                       |
| The shim directory is excluded from the main workspace    | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                              |
| **CI contains four wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. the playground), `nightly.yml:110-114` |
| The Z3 wasm static library is prebuilt by Emscripten      | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from a fixed URL; degrades to a warning if absent)                                                                                                                                                               |
| **29 files** contain the literal `wasm32`                 | **22** have actual `#[cfg(...)]` attributes; the other 7 mention it only in documentation (see the “F2 determination” subsection of `06-cleanup-inventory.md`)                                                                                                                        |
| `orchestrator.rs` has 16 occurrences                      | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                                  |
| `src/lib.rs` has 11 occurrences                           | All are attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                        |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which main-crate APIs the 152-line `wasm/src/lib.rs` can call under the wasm
target—`src/lib.rs:151/170/187` gate `run_file` / `run_project` / `build_bytecode` as a whole (all
three require `std::fs`), while the shim takes a different route.

**This also yields a fact directly relevant to this document**: the playground entry point
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and passes
it to `Driver::run` (the JS-side entry point is `run_code`, `:122`)—the **single-file path**.
Therefore:

| Path consuming `proof_calls`                              | Executes the proof function? |
| --------------------------------------------------------- | ---------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                      |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                      |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                      |
| Multi-file `run` (`compile_project`)                      | **No**                       |
| `check` (`check_project`)                                 | **No**                       |
| In-project LSP (`check_source_in_project`)                | **No**                       |

In other words, **the existence of `wasm/` changes “proof_execution has only one consumer” into “it
has three,” but all three remain on the single-file side.** When the unified Driver is introduced,
this path is already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing
the same single-file stage table as `SingleFile`, `:135`); the implementation record is the
**4.2.8** row of `09-execution-wbs.md` (`wasm` now goes through `ProgramKind::WasmPlayground`,
`:274`).

(Cleanup scope—which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in playground scenarios—is assigned to the “F2 determination” subsection of
`06-cleanup-inventory.md`: four wasm arms are unreachable at playground runtime but load-bearing at
compile time, and six `not(wasm32)` arms are load-bearing across the three native stage tables;
there are no low-risk items that can be deleted outright. The RFC-039 open question “build a wasm
target, or delete these branches” is now **obsolete**: the target has been built.)

---

## Target Design

### 1. Stage model: the `Stage` enum (exhaustively enumerable, with no runtime registration)

**Core constraint: `Stage` is a compile-time-exhaustively-enumerable enum, and runtime registration
is forbidden.** The reason is that Rust’s exhaustive `match` can enforce at compile time that “a new
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
    DeadCodeAnalysis,   // Dead-code-family analysis              Project
    ProofExecution,     // Compile-time execution of proof functions         PerModule
    GlobalSlotAlloc,    // Global slot allocation              Project
    IrGeneration,       // AST → ModuleIR            PerModule
    Monomorphization,   // Monomorphization                    Project
    Linking,            // Cross-module linking / IR merging       Project
}

impl Stage {
    /// All stages. When a variant is added, compilation of both this array and the exhaustive match in `dispatch` will fail.
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
once—this eliminates questions such as “should `allocate_global_slots` be called once per file?”
that currently require human reasoning in `orchestrator.rs`.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on the order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the initial draft were
> corrected based on implementation evidence—
>
> 1. `Monomorphization` was moved to **after** `IrGeneration`: monomorphization consumes the IR
>    artifact (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the original order
>    contradicted the data flow.
> 2. The Check-form stage table was normalized to dead code **before** proof according to the
>    topology of `Stage::ALL`: the current `check_project` executes proof before dead code; the two
>    have no data dependency, so their diagnostic sets are identical (C2 set semantics), and only
>    the within-file diagnostic order is normalized. The single-file path (dead code embedded in
>    typecheck, before proof) already conforms to the ALL order, so byte-for-byte acceptance is
>    unaffected.
> 3. `Parsing` was moved to **before** `Registry`/`RoleClassification` (C3, user ruling on
>    2026-10-09): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    artifacts—the original order would force the Registry arm to “hide parse,” making the stage
>    table lie about the data flow; as a result, multi-file parsing dropped from two passes to one.
>    The current state is faithfully registered as well: parse errors in the multi-file path used to
>    be a hard abort (the `?` propagation from `build_registry_from`, representing tension with
>    CollectAll semantics, registered as WBS 4.10.1)—**4.10.1 has been fixed** (user ruling on
>    2026-10-09: Rust-style collection semantics + plan B): parse failures in the Check path degrade
>    to per-file diagnostic collection, and the defective file exits the compilation unit (it is not
>    entered into the registry, and importers report E5001); the MultiFile path retains the hard
>    abort (under FailFast a defective file cannot produce an IR, which is correct semantics and
>    remains permanently pinned).
> 4. The Check form (4.2.2, user ruling on 2026-10-09) added two registrations: a. Data-dependency
>    edge 14→16: `RoleClassification` consumes the used_by edge set from Discovery, and
>    `DeadCodeAnalysis` consumes Discovery’s W1006 shadowing events—the initial draft omitted these
>    (in the orchestrator era, these artifacts were produced by `discover_with_used` and discarded
>    by the `discover` wrapper, so they were lost because they never entered the table); b. The
>    within-file diagnostic-order normalization was extended to the full form of note #2: E3020
>    entry validation was moved into the RoleClassification arm (Check has no Linking stage, so the
>    arm holding surfaces and the AST is responsible for validation), and the within-file diagnostic
>    order = topological stage order (E3020 → typecheck → W1006/W1003/dead code → proof); the
>    diagnostic set is unchanged (C2 set semantics), and only the order of stderr entries changes.

### 2. Obligation ledger: `Obligations` + `assert_drained()`

**Design objective**: turn “a field was produced but nobody consumed it” from an undetectable
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
    // ...remaining span-keyed fields
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

**Why W level comes before E level**: fixing discarded obligations will **change the diagnostic
set**. The C2 criterion requires “the diagnostic sets of all entry points are identical”; moving
directly to E level would cause `yaoxiang check` to suddenly produce many previously silent
`Unproven` diagnostics. Splitting the work into two steps—first W-level observation, then promotion
to E—keeps the criteria for every step usable. See “Implementation Points” S4.

**The call site of `assert_drained()` is unique**: `Driver::run` calls it at the end of the stage
table, before producing `CompilationResult`.

**Companion static gate**: RFC-039 has already proposed `scripts/ci/check-obligations.py` (a field
appears at least twice but has no read in a third file → failure). `assert_drained()` is a
**runtime** gate, while the script is a **static** gate; the two complement each other, and both are
required.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: this note registers the
> **implementation form, measured figures, and 4.3.2 interface** of the sketch and three-tier table
> above, with a direct correspondence to code file:line. Implementation commit `1941cc47a`
> (verification tree `99079bc52`). It is also the **third** of the three “must not be silent”
> mechanisms (the other two are the audit surface and the static gate).
>
> **I. Implementation artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (:201, field
>   name/consumer stage/denylist), **non-obligation field registry `NON_OBLIGATION_FIELDS` (:150, 8
>   entries with rationales)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (:577), emission `emit_obligation_diagnostics` (:654), non-consumer gating
>   `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped` (:549), IrGeneration
>   delivery set `IR_GENERATION_DELIVERED` (:285), and W1090 code constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **The only settlement point**: at the end of the `Driver::run` stage table, before
>   `into_outcome()` produces `CompilationResult` — `src/driver/mod.rs:110`
>   (`State::assert_drained`, obligations.rs:570). The early-return path of `Aggregation::FailFast`
>   settles as well (the ledger must balance).
> - **Seven delivery points** (one per arm; remove any one and the runtime W1090 must fire):
>   ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole outward read path from
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — all four use the
>   same delivery entry point: `AstToIrGenerator::new_with_type_result` /
>   `middle::generate_ir_with_context`; Monomorphization `arms.rs:936` (single-file direct
>   request-table read) / `:1221` (MultiFile·Check: aggregated by the Linking arm;
>   `driver/helpers.rs:295` reads each unit’s `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation identity
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 is registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, with the rationale written above the entry at :26-31 — the settlement
>   granularity is “unit × obligation,” and there is no user-identifiable source location, so no
>   dummy span is inserted); six locale texts are provided in `locales/*.json`. **Emission level**:
>   from 4.3.2 onward it is **error** (same code promoted to E; see the third section) — determined
>   by the explicit `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444); the code
>   number remains in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Cell-by-cell implementation of the three-tier table + two added tiers**
>
> | Case in the three-tier table above              | Implementation criterion                                                                       | Code                   |
> | ----------------------------------------------- | ---------------------------------------------------------------------------------------------- | ---------------------- |
> | The obligation is read by its declared consumer | **A delivery marker exists** (marker ↔ real read reconciled by the static gate; see section V) | obligations.rs:618-623 |
> | The obligation is non-empty but has no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see section III)      | obligations.rs:624-630 |
> | The obligation is empty                         | `Obligation::is_produced` checks each field for emptiness → pass                               | obligations.rs:587-591 |
>
> Two **added tiers** (user ruling on 2026-10-13; not included in the sketch):
>
> 1. **S1**: the obligation’s consumer stage is not in this `ProgramKind`’s stage table → **not
>    considered owed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), and `EMBEDDED_STAGES` has no
>    Monomorphization (:121-126).
> 2. **Skip exemption**: the consumer stage is skipped because of `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not considered owed (obligations.rs:598-602). `NoObligations` does **not**
>    grant an exemption—if an obligation is non-empty while the stage claims it has none, the ledger
>    is not balanced and must be reported.
>
> **III. Outward-emission policy (ruling “C”) and the measured figures behind it**
>
> **4.3.1 (observation period)**: this mechanism emits only **W-level** diagnostics (W1090); there
> is **no strict switch**. **4.3.2 (final state, user ruling A on 2026-10-13)**: **the same code is
> promoted to E**—the `W1090` code number remains unchanged, but the emission level is raised to
> `error` (explicit `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E
> code is added** (one code for the same fact would create parallel representations). **The
> promotion changes only the emission level, not the emission conditions**. **This is a D-class
> breaking item under D59 (an externally observable behavior change) and is faithfully registered**:
> healthy programs emit nothing (when all obligations have delivery markers, there is no report), so
> existing corpora and user programs are unaffected (342/342 “difference: empty”); however, when a
> run really encounters an undelivered obligation, the diagnostic **counts toward `error_count`**,
> is rendered as `error[W1090]`, and produces a nonzero exit code—unlike the previous “W level does
> not block compilation.” The separation of code family and emission level has been registered in
> RFC-013’s “emission channel” section (a code prefix determines only numbering ownership; the level
> is determined by the explicit severity at the construction site). Mechanism facts (measured): the
> severity is taken from the explicit `.severity()`; otherwise “a code beginning with `W` → Warning”
> (`codes/builder.rs:186-203`); the registry’s `ErrorCategory` does **not** participate in severity.
> The exit code is determined by `(error_count, warning_count)` (`util/diagnostic/command.rs:87`)
> and `--deny-warnings` (`src/main.rs:567-568`), so **W1090 follows `error_count` because of its
> explicit error severity (from 4.3.2 onward)**.
>
> **Denylist fields do not enter the user-visible diagnostic stream** (captain ruling C on
> 2026-10-13; the current denylist is empty, see section IV), for a reproducible reason:
>
> - Ruling A (emit denylist entries as W) had a measured cost: `interface_impl_registry` is
>   populated from the typecheck precomputed table and is **always non-empty**, so the
>   per-compilation emission is a **constant signal** with no discriminating power, yet it breaks
>   three existing external contracts—“healthy programs emit no diagnostics / LSP emits no
>   diagnostics / `warning_count == 0`”: `cargo test --lib --all-features` measured **12 existing
>   test cases turning red**, of which **8 were on out-of-scope paths**
>   (`orchestrator/tests/namespace_data_access.rs` ×5, `lsp/handlers/tests/diagnostics.rs` ×2,
>   `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 were under `src/driver/tests/`), and 24/342
>   fixture baselines changed; the only viable fix would be “filter W1090 out of zero-diagnostic
>   assertions,” which means weakening an existing criterion.
> - Ruling B (downgrade to Info level) would still break 9/12 of the list-style assertions, and Info
>   is not “emitted according to the ruling level.”
> - The measured effect of ruling C: **zero changes to existing test cases**, **zero changes to the
>   corpus baseline** (342/342 “difference: empty, behaviorally equivalent”), and runtime W1090
>   becomes a **discriminating regression detector**—the healthy tree remains silent throughout, and
>   removing any one delivery marker makes it fire (all three delivery points were tested by
>   marker-removal mutations; CLI `check` emitted two `warning [W1090]` entries when the marker was
>   removed from the Check arm).
>
> The denylist’s “must not be silent” requirement is borne by three mechanisms: ①
> `DriverOutcome.obligations.findings`, the audit surface (emitted on every run, with the WBS id and
> evidence pointers); ② `check-obligations.py`, which prints the denylist on every run (currently
> empty) and performs stale detection; ③ this note.
>
> **IV. Current denylist state (ledger closure) and implementation records for 3.4.4/3.4.5/3.4.9**
>
> The denylist has gone from 3 → **0**—all three clearing tasks landed on 2026-10-13:
>
> Note: the line anchors below record the file state as of **2026-10-22**; after P5 (2026-10-26)
> split the checker into `checker.rs` + `checker/{semantic_tokens,refinement,annotations}.rs`, the
> `checker.rs` line numbers shifted as a whole, so the anchors below no longer correspond to the
> current file and are retained only as historical citations.
>
> - **3.4.4 `variant_ctor_calls` → field deleted**. Its declared responsibility (“ir_gen generates
>   CreateVariant”) is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (the same AST shape
>   `FieldAccess{Call(Var)/Var}` + declaration order obtained from `sum_type_variants`); the call
>   site at `:7496` replaces the entire Call with `CreateVariant`; the function’s doc explicitly
>   states that “span keys are unreliable across systems (the two sides interpret the span of a Call
>   node differently), so detection is performed in place” —**wiring up a consumer would reintroduce
>   the already-fixed span-mismatch defect**. Deleted surface: the `types.rs` field, collection at
>   `checker.rs:1489-1493/:1565`, the collection chain in `inference/expressions.rs` and
>   `inference/statements.rs`, and the `VariantCtorCall` structure in `environment.rs`;
>   `infer_variant_ctor_call` type inference and all diagnostics **remain unchanged**.
> - **3.4.5 `method_overload_ir_names` → cross-stage mirror deleted** (retained as an in-phase
>   candidate-name table and in `overload_resolutions`). All read/write sites: written in phase
>   (`environment.rs:205-219`, registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550` resolution point, the unique hit at `:4550` writes to
>   `overload_resolutions`); what was deleted is the **zero-reader mirror** that `checker.rs:1558`
>   poured into `TypeCheckResult`. The naming fact still has a single expression: the candidate-name
>   table (in phase) + span-keyed resolution (cross-stage, `ir_gen.rs:380` construction,
>   `:7632/:7705` consumption); the mixed definition-side name is computed independently by
>   `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as an in-tree fact** (user ruling A; see the
>   reclassification record in this document’s § Obligation Fields table). What was deleted is the
>   **claim** in the documentation comment that it is “a mirror for consumption by the middle
>   layer/LSP” (`types.rs:47-48`); the field and its in-phase consumption are retained
>   (`operator_interfaces.rs:623` operator lookup, seven constraint-solving lookups in
>   `inference/expressions.rs`, `checker.rs:214-217` read-only accessor); direct proof of zero
>   cross-stage reads: `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why no
>   consumer was wired up**: the table’s content is an environment mirror populated by the typecheck
>   precomputed table (always non-empty); if LSP genuinely needs it, register it through 4.6.x;
>   creating a consumer for a field that “does not require cross-stage consumption by design” would
>   introduce a false obligation.
>
> After the ledger was cleared, the **field census is closed by “obligation inventory ∪
> non-obligation field registry (`NON_OBLIGATION_FIELDS`, with rationales)”**— the
> reverse-containment end of gate criterion 1 protects this end to end (adding a field without
> registering it or deleting a field without synchronizing it immediately turns the gate red). The
> denylist **mechanism** is retained: the gate’s criterion 4 stale detection and the `--selftest`
> `stale-red-list` fixture continue to cover its detection surface; any future registration must go
> through an explicit ruling.
>
> In conjunction with the clearing, the inventory was reduced from 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / this document’s § Obligation Fields table); the gate’s `--selftest` now covers
> seven fixture categories (adding “unregistered field,” “stale anchor,” and “empty denylist”), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Actual criteria of the static gate (9.2) (three hard criteria beyond the sketch’s one
> sentence)**
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**: `INVENTORY`
>    in `obligations.rs` ↔ the `TypeCheckResult` field definitions ↔ this document’s “§ Obligation
>    Fields” table (including the table header’s field-count declaration); any drift turns the gate
>    red; moreover, the `TypeCheckResult` field set must be **exactly equal** to “obligation
>    inventory ∪ non-obligation registry (`NON_OBLIGATION_FIELDS`, with rationales)”— adding a field
>    without registering it, deleting a field without synchronizing it, a non-obligation entry
>    without a rationale, or overlap between the two tables all immediately turn the gate red;
> 2. **Read evidence**: non-denylist obligations must have a real read in the **declared consumer
>    file** (`.field_name` member access; comments/tests/strings do not count)—this makes “delivery
>    marker ↔ actual downstream read” a hard criterion (the principal risk of mechanism A is
>    precisely drift between the two);
> 3. **Delivery markers**: non-denylist obligations must be marked at a delivery point; denylist
>    entries, conversely, must not be marked;
> 4. **Denylist stale detection**: denylist fields must have zero real reads (a new consumer turns
>    the gate red), and each entry must carry a WBS id and evidence pointer; there is also
>    `--selftest`: built-in negative/positive fixtures, with negatives necessarily red and positives
>    necessarily green (proving that the detector is not a no-op).
>
> **VI. Every deviation from the sketch above**
>
> 1. The sketch’s `pub struct Obligations { …fields… }` **does not copy the fields one by one**: the
>    obligation data remains owned by `TypeCheckResult`; the ledger records only the “inventory +
>    delivery markers + settlement conclusions.” Reason: copying creates two sources of truth (each
>    field is produced by a different checker/ownership layer in typecheck), which must inevitably
>    drift; the cross-layer data plane should not be copied into the orchestration layer.
> 2. The sketch’s `take()` / “mark consumed” **was not adopted**; instead, `is_produced` emptiness
>    checks + `DeliveryLedger` delivery markers are used. Reason: the obligation and the artifact
>    are the same data, and `take()` would remove what the downstream layer needs; moreover, 11
>    IrGeneration obligations take the form of “handing the entire `&TypeCheckResult` to ir_gen,” so
>    they cannot be taken field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusions are written to `DriverOutcome.obligations`, and
>    diagnostics are routed by `ProgramKind`: Check/Lsp use the per-file collection channel; other
>    forms are routed by **level** (from 4.3.2 onward W1090 is E level)—the single-file path uses
>    `PipelineError::TypeCheck` (the emission arm at obligations.rs:654), while multi-file/embedded
>    forms record an `OrchestratorError` failure (otherwise `result.errors` would have no consumer).
>    Reason: returning `Vec<Diagnostic>` would lose form-specific routing—check diagnostics must
>    retain per-file attribution.
> 4. The first criterion in the three-tier table became “a delivery marker exists” rather than “the
>    obligation is read by its declared consumer”; “marker ↔ actual read” is handled by the static
>    gate (criterion 2 in section V).
> 5. The second tier’s “promote to E level in `strict` mode” **did not use a strict switch**;
>    instead, **4.3.2 promotes the same code to E** (it is promoted even without a strict switch,
>    because zero emission in the healthy state is sufficiently safe, R4).
> 6. The static-gate criterion was upgraded from “a field appears at least twice but has no read in
>    a third file” to the four criteria in section V.
> 7. The two added tiers not present in the sketch (S1, skip exemption) are described in section II.
> 8. The single test name `test_obligations_drained` mentioned in the S1/S4 table actually landed as
>    **20 test cases** in `src/driver/tests/obligations.rs` (initially an `#[ignore]` skeleton →
>    turned green; the red state is retained in the record: removing the only settlement point
>    yields `pairs: 0` and six failures). The S1/S4 rows have been synchronized to the actual file.
>
> **VII. Implementation record for 4.3.2 (level assignment + gate hardening)**
>
> - **Inputs for the field-by-field review**: `ObligationSettlement.drained_kinds` (the obligation
>   kinds actually drained in the current run, available per run) and the five counts (the
>   conservation identity in section II); together with the audit surface `findings`, they establish
>   the fact of “whether this field has ever been consumed by any entry point.”
> - **Level: same-code promotion to E has landed** (user ruling A on 2026-10-13; see the final-state
>   registration in section III and the D59 D-class breaking-item declaration). Implementation:
>   `to_diagnostic` explicitly uses `.severity(Severity::Error)`; the emission arm routes errors
>   through the error channel. Mutation evidence: removing any one delivery marker → that form fails
>   to compile and `check` returns a nonzero exit code (`error [W1090]`); after restoration, the
>   healthy state emits nothing (342/342 corpus entries with zero changes). The locale text now
>   states “W-family code, emission level is error” in all six languages.
> - **Gate hardening (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses `file:line` anchors in **non-obligation
>     registry rationales and denylist evidence**, validating “the file can be located uniquely +
>     the line number is registered + the symbol immediately following the anchor actually appears
>     in the anchored file”; an anchor pointing to a nonexistent symbol turns the gate red. It
>     includes negative fixtures `tests/baselines/obligations-gate/stale-anchor/` (one nonexistent
>     symbol / one out-of-range line). Note: the denylist is currently empty, so the criterion’s
>     actual landing point is the **non-obligation registry** and future evidence pointers (the same
>     code path).
>   - **F2/R3 reverse containment**: the gate prints “`TypeCheckResult` field count − obligation
>     count − non-obligation registry count = difference”; a nonzero difference turns the gate red;
>     it includes the negative fixture `unregistered-field/` (a new struct field not registered).
> - **Informational item F3**: the `SkipReason::UpstreamFailed` exemption is unreachable in real
>   entry points (a unit whose typecheck failed does not retain `type_result`, so the obligation
>   does not exist in the first place); the semantics of this tier are pinned by a synthetic `State`
>   test, and real end-to-end coverage includes only `ConfigDisabled` (t3 T06).
>
> **VIII. Closure of 4.3’s secondary tasks (denylist cleared + final E-level obligation
> diagnostic)**
>
> **8.1 Three-round implementation timeline** (each item corresponds directly to a code file:line;
> this round only registers the conclusions and does not change code)
>
> | Round                  | Conclusion                                                                                                                                                                                  | Code anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
> | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger goes live | `State::assert_drained()` is the only settlement point; 13-entry obligation inventory + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                 | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                                  |
> | 3.4.4 Ledger clearing  | `variant_ctor_calls` field deleted—the declared responsibility is equivalently fulfilled by in-place shape detection on the IR side                                                         | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                              |
> | 3.4.5 Ledger clearing  | `method_overload_ir_names` cross-stage mirror deleted (in-phase candidate-name table + `overload_resolutions` retained)                                                                     | Resolution consumption point `ir_gen.rs:380/:7632/:7705`; definition-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                         |
> | 3.4.9 Ledger clearing  | `interface_impl_registry` reclassified as an **in-tree fact**: the “mirror for middle/LSP consumption” claim deleted, in-phase consumption retained; obligations 14 → 13, denylist 1 → 0    | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten under ruling A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven sites (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor at :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level assignment | **Same-code promotion to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; outward emission routes by level through the error channel (first failure takes priority) | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code-family/level separation registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                                  |
>
> **8.2 Closure conclusions**
>
> - **Denylist cleared**: the number of entries in `INVENTORY` (`src/driver/obligations.rs:201`)
>   whose `red_list` is `Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs`
>   → 0). The mechanism is **retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, the
>   gate’s criterion 4 stale detection, and the `stale-red-list` fixture; the gate prints “denylist
>   0 entries—cleared to zero.”
> - **Final E-level obligation diagnostic**: the **code number and W-family membership of `W1090`
>   remain unchanged**, while the emission level is `error` (D5 final state, user ruling A on
>   2026-10-13). This is a **D-class breaking item under D59** (an externally observable behavior
>   change), registered in both section III of this note and RFC-013’s “emission channel” section;
>   **zero emission in the healthy state ⇒ zero impact on existing corpora and user programs**
>   (corpus 342/342 “difference: empty,” zero IR-snapshot drift; t11 independent verification and
>   t12 re-review each reconfirm this).
> - **Field census closed**: the `TypeCheckResult` field set = obligation inventory ∪ non-obligation
>   registry (8 entries, `src/driver/obligations.rs:150`, each with a measured consumption-surface
>   rationale); gate F2 explicitly prints the difference (currently `21 - 13 - 8 = 0`).
> - **No unimplemented promises remain**: all hardening items registered in this note have
>   **landed** (F1/F2/F4/F5; see sections V, VII, and VIII); the only two items not fully closed are
>   the **known boundary** in 8.4 and t7 F5, which was converted to an issue; neither is an
>   unimplemented criterion of this mechanism.
>
> **8.3 Disposition status of findings from each acceptance/re-review round**
>
> | Source        | Finding                                                                                                                                                                         | Status                                                                                                                                                                                                                                                                                 |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Denylist evidence is a plain-text pointer and can silently become stale when a replacement consumer is removed                                                                  | **Landed**: gate F1 anchor validation (uniquely locatable file + registered line number + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                     |
> | t3 F2 / t4 R3 | Gate criterion 1 lacked reverse containment; adding a field silently PASSed                                                                                                     | **Landed**: F2 explicitly prints the difference (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                                |
> | t4 R4         | W-only versus promotion to E was not registered                                                                                                                                 | **Landed**: sections III / VII of this note + RFC-013 “emission channel” + W1090 help and error_output examples in the six `locales/*.json` files                                                                                                                                      |
> | t7 F5         | Dead-code analysis miscounts references to overloaded methods (legal overloads are still falsely reported as `W1005`)                                                           | **Converted to issue `#450`**, assigned to WBS **4.9 Correct Reference-Pool Implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—not part of this secondary task                                                                                       |
> | t11 F7        | Under gate `--root` mode, F1 does not cover anchor promises in module-header prose comments                                                                                     | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                           |
> | t11 F8        | Verifier-side self-record: mutation-driven verification once wrote to the main repository (cleared immediately with `git restore`; affected-round output invalidated and rerun) | **Informational item** (not a deliverable defect)                                                                                                                                                                                                                                      |
> | t11 F9        | An intermediate main-repository state (unstaged files made this document’s table header differ from `INVENTORY`) triggered a red gate                                           | **Informational item / resolved**: the frozen commit was self-consistent and PASSed; “an intermediate state committed directly turns the gate red” is precisely F2/F4’s intended behavior                                                                                              |
> | t12 F12-1     | This document’s struct range says `16-73`, but the actual range is `16-76` (three comment lines added by t9 without mechanical reconciliation)                                  | **Landed + mechanized**: the range was corrected to `16-76`, and gate **F5** was added (range anchor; an error turns the gate red); related numbers were aligned by grep (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                           |
> | t12 F12-2     | Settlement in the multi-file form once overwrote the first failure (the type error in `zzz.yx` was covered by W1090)                                                            | **Fixed** (the captain amended the commit message to explain this): the `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned test** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not by a new test |
>
> **8.4 Known boundaries (registered items, not unimplemented promises)**
>
> - **F1’s scanning surface**: anchor validation covers only text in **registered entries**—the
>   rationales in the non-obligation registry (`NON_OBLIGATION_FIELDS`) and denylist evidence;
>   **`file:line` promises in module-header prose comments are outside the scanning surface** (no
>   registered entry is currently affected). If such an anchor promise is written into a module
>   header in the future, it must **rely on human review**, or F1’s scanning surface must be
>   expanded—choose one; neither may be left unattended by default.
> - **The gate’s static boundary**: collection-level validation of delivery markers is handled
>   statically (criterion 3), while removal of an individual arm is covered by **runtime
>   settlement**— static + runtime as two layers is the intended design, not a gap (t11’s
>   M11A/M11B/M11C mutation verification works in both directions).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed)**:
> two of the **13 obligations above**, both **span-keyed contracts** (`release_plan` /
> `overload_resolutions`), have been converted to **structural-identity keys** (`PlanId`) under
> RFC-039 **D20**. This note registers the implementation form, criteria, classification basis, and
> known boundaries; the definition lines are still reconciled by gate **F4** (the line numbers in
> the two corresponding rows above are current code facts).
>
> **I. Implementation form (carrier selected: A, `PlanId`)**
>
> The key is a **structural identity** that travels with the AST node, no longer relying on the
> positional coincidence of “both sides calculating a span separately”: `PlanId`
> (`src/util/plan_id.rs`, a monotonically increasing `u32`, unique within a compilation unit, with
> `UNPLANNED = u32::MAX` representing synthetic nodes outside the parser) is issued by the **sole
> allocator**, `ParserState::alloc_plan_id`. There are two carriers: `Stmt.plan_id` (statement
> identity → release-plan ownership and consumption) and `Expr::Call.plan_id` (call-site identity →
> overload-resolution production and consumption); `Expr::call` is the repository’s sole
> call-expression constructor, and synthetic calls outside the parser all use `UNPLANNED` (**not
> consumable**, see section II).
>
> - `release_plan`: `drops` is a `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), and the
>   sole consumption entry point is `ReleasePlan::consume(stmt)`—**the read and the accounting occur
>   in the same call**; `ir_gen.rs:360` reads it and `:3103` uses it (statement identity is provided
>   by `walk_stmt`’s set/restore operations + the CFG-node-to-statement-identity table; the Drop at
>   scope exit belongs to the last statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one object**),
>   produced at `inference/expressions.rs:4550` (written by call-site `PlanId`, with dispatch
>   destructuring and forwarding from `Expr::Call { plan_id }`), consumed at `ir_gen.rs:7697` (with
>   binding) / `:7772` (without binding) through `OverloadResolutions::get` (a hit is recorded).
>
> **II. Mandatory-consumption settlement (reusing the 4.3 ledger, not creating a second one)**: the
> consumption ledger is a **single implementation**, `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), and each of the two tables holds an instance; when
> `generate_module_ir` returns successfully, both ledgers are marked as having “completed a full
> pass” at the **same point** (`ir_gen.rs:1095` for the release plan / `:1097` for overload
> resolution); the three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), and the sole settlement point connects one row for each of the two
> obligations—a nonempty difference produces an **E-level W1090** (same-code promotion to E; see
> section III). The exemption criterion is attached to the **ledger’s own completion marker** (not
> to whether `unit.ir` exists), so the Check form taking away the IR artifact does not constitute an
> exemption.
>
> **III. D59 classification basis (class C, red before green)**: the
> `Type.method(positional argument, name = value)` form was previously **silently given the wrong
> answer** (measured: `r: Int = t.pick(1, a = 2)` compiled cleanly and printed `1` rather than
> `20`—the resolved candidate was discarded and execution fell back to the last binding in
> definition order); after the change, the same program produces a compile-time
> `error [W1090] ... produced key was not consumed`. The fix deliberately chooses **loudness**
> rather than “stuffing the resolved name into a call with named arguments”: the latter would lose
> the named argument and replace one silent failure with another (final semantics see
> [09](./09-execution-wbs.md) §3.14 / issue #452). In addition: `Drop` instructions are a no-op in
> the interpreter (the wildcard arm in `src/backends/interpreter/executor/ops/control.rs` only calls
> `advance()`), so migrating the release-plan keys does not change runtime behavior—the 342-entry
> corpus differential being empty is empirical proof of this.
>
> **IV. Criteria and evidence**: the same-shaped criterion D41, `test_release_plan_spans_consumed`
> (settled by `PlanId`), and the new `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen over the corpus, **the difference must be empty + the evidence floor**:
> the number of files that produced a resolution `≥ 1`, the number of truly consumed entries `≥ 2`,
> with no allowlist); nine pinned resolution forms are covered in
> `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return / positional argument
> / in-type body binding / `?` operand / named-only / positional + named / ambiguous / variant
> construction / unallocated identity). Mutation-driven verification: shifting a consumption key by
> one position ⇒ the criterion turns red + a real program produces `error [W1090]` + the corpus gate
> differs; restoration returns it to green. Gate figures: `check-obligations.py` PASS, the 342-entry
> corpus differential is empty, and `cargo test --lib` reports 2712 passed / 0 failed (zero
> IR-snapshot drift).
>
> **V. Known boundary (registered item, not an unimplemented promise)**: the **binding-free
> consumption point** at `ir_gen.rs:7766-7790` is **unreachable** under the current corpus and test
> surface (method declarations uniformly use the binding-bearing consumption point through
> `register_type_bindings`), so the key correctness of this branch has **no red state with which it
> can be falsified**; the reason for retaining it = it is the symmetric branch of the
> binding-bearing path, and deleting it would rewrite the “method call without `type_bindings`” form
> into a silent one. This is recorded by independent verification (t5 F-1) and re-review (t6 F2),
> and **no new task is opened**.

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
    /// May only be one of the six predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` may only be one of the six predefined combinations above (one per `ProgramKind`);
callers cannot freely pass an array.** This is the key constraint preventing the `Program`
abstraction from degenerating into a “free parameter that can accept anything,” and it is asserted
by `test_program_stage_coverage`.

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
                // ... 12 arms; not one may be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // The only settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: four implementation deviations
> from the sketch above are semantically equivalent and registered for reference—
>
> 1. The three-state `StageOutcome` from §4 is not returned by stage arms; it is expressed by
>    `State` failure flags + `Aggregation` gating (Continue / Warn / Abort semantics remain
>    unchanged, eliminating repetitive return boilerplate in every arm);
> 2. `Driver` has no `config` field—the sole source of configuration is `Program.config` (the “avoid
>    mutable global state in Driver” determination in §5 covers the sketch field);
> 3. `Skipped` diagnostics are recorded only in the internal `DriverOutcome.skipped` ledger in 4.1.3
>    and are not emitted—the zero-diff criterion of S2 “deliberately fix no bugs” requires this;
>    emission follows with the 4.3 obligation ledger;
> 4. `proof_execution` module visibility is relaxed to `pub(crate)`: the driver arm is the sole new
>    caller (L1→L2 is an allowed direction); the existing orchestrator call sites are restored to
>    their prior visibility after they migrate away in 4.2.
> 5. In the multi-file form, ProofExecution is an independent stage executed after **all**
>    typechecking (A1, user ruling on 2026-10-09): the original `compile_project` interleaved proof
>    with the per-file typecheck loop (file N’s proof preceded file N+1’s typecheck). A single
>    failure produces byte-for-byte identical results; in the multiple-failure scenario “file 1
>    proof fails + file 2 typecheck fails,” the first reported error changes from the proof error to
>    the typecheck error (pinned by a test).
> 6. `DriverOutcome` carries artifacts through channels selected by ProgramKind: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of
>    the entry points (PipelineError / OrchestratorError) are outside the type surface that Driver
>    can unify.
> 7. The Check channel landed (4.2.2): `DriverOutcome.check_diagnostics` carries per-file
>    diagnostics (every discovered file has an entry; a clean file has an empty Vec—the existing
>    check_project contract); the Discovery artifact was extended to a “file set + used_by edge
>    set + shadowing event” triple in State (the MultiFile form has no downstream consumer, so it is
>    only recorded and not emitted); check path parsing dropped from two passes to one per file with
>    the C3 topology.
> 8. The Lsp form landed (4.2.3, file-source routing ruling, user decision on 2026-10-09): parse
>    failures in disk files follow Check’s plan B (collect + exit the compilation unit—both sides
>    degrade consistently); parse failures in edited buffers retain a partial AST and continue to
>    typecheck (editor philosophy—semantic features should not disappear while the user is typing).
>    The Discovery arm lets Lsp override stale disk contents with buffer source code; Typecheck in
>    Lsp checks only the target file (other units are retained only to provide signatures for the
>    registry); the within-file diagnostic-order normalization principle extends to LSP (typecheck
>    diagnostics → W-code warnings → proof errors). The old behavior in which LSP hard-aborted on
>    parse errors in unrelated disk files and the handler silently fell back to the single-file path
>    is fixed by this step.
> 9. The Embedded form landed (4.2.4): `Program` gained a `shared_registry` field—the embedded std
>    module is a child compilation, and the registry is an **input**, not an artifact (EMBEDDED_
>    STAGES has no Registry stage), making #94’s SymbolTable-sharing contract explicit as a program
>    declaration rather than “the caller must remember to pass the same registry.” The error-path
>    text was normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>`
>    (compiler-internal error surface only, with no pinned test). At this point, all four
>    orchestrator entry points have migrated to the Driver.
> 10. Standalone check unified (4.2.5, ruling A + IR-stage ruling, 2026-10-09): the Check form
>     faithfully carries single-file semantics for programs without a project root—the warning
>     surface covers only the entry file (neighboring files receive only errors; prefer omission
>     over false positives); relative `use` is resolved along the importer’s directory (aligned with
>     rustc’s single-file mod behavior). The CHECK stage table adds GlobalSlotAlloc + IrGeneration:
>     the old standalone path (the full pipeline) already ran IR generation, so E3019/E1014/E1015
>     etc. are produced only by ir_gen (confirmed by runner gates). IrGeneration in the Check form
>     is check-only and does not consume the IR; the Script/Bin distinction is expressed by the
>     None/Some value of module_key (the existing E3023 switch, ir_gen.rs:1660). Monomorphization is
>     not included in CHECK (it produces only resource-limit/internal errors and has no corpus
>     dependency). **〔Withdrawn on 2026-10-10 following empirical validation in 3.4.8〕**—“zero
>     corpus dependency” was merely evidence from the absence of pathological-recursion fixtures;
>     E3005 is the sole compile-time line of defense for this class of program. See note #14.
>     Incidental fix for a latent defect: repeated registration harvesting by the Registry arm for
>     embedded std units would overwrite the native half already merged by with_std() (false
>     E1043)—embedded units now skip repeated registration.
> 11. LSP single-file fallback unified (4.2.6): the manual lex→parse→check_module_collect_all
>     sequence in `run_diagnostics` was deleted and replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and the CLI now use the same
>     Driver. In the SingleFile+CollectAll form: Parsing collects all parse errors, retains a
>     partial AST, and continues to typecheck (extending the editor philosophy established by the
>     4.2.3 ruling; the stage is not marked as failed, because otherwise topological skipping would
>     prevent typecheck from ever running); Typecheck dispatches the `check_module_collect_all` free
>     function. The LSP single-file path now adds proof (E4018), W-code warnings (W1001–W1005,
>     pinned by tests), and IR-level errors; lexical failures now report real diagnostics (the old
>     synthetic “E0001 lexical error” text disappeared with the sequence’s deletion—real diagnostics
>     carry precise spans, with no information loss).
> 12. `check_module` / `check_module_collect_all` verified line by line (4.2.7, mandatory C5 item):
>     the two entry points had already converged separately—the mod.rs layer uses
>     `check_module_inner(ast, env, collect_all)` with a single Boolean, and the checker layer uses
>     `check_module_impl(module, collect_all)`; the only divergence is
>     `init_body_checker(collect_all)` → `set_collect_all_errors`; pass 3 and drain run in both
>     modes, with the difference carried by whether `collected_errors` is empty; there are five
>     collection points in statements.rs (function body/use/for/block/while). Verification also
>     proved and fixed a duplicate-diagnostic defect: collection points put the first error into
>     both `collected_errors` and the Err return channel, and pass 3’s `add_error` for the Err
>     produced two identical diagnostics with the same code, span, and message ×2 (or ×3 when
>     nested); two additional independent mechanisms were also found: duplicate validation of
>     annotation-signature parameters versus the whole annotation (E1003/E1103), and double emission
>     from the ownership layer at the same point (E2014/E2018). The fix landed at the module-result
>     boundary: deduplication by (code, span, message)—a diagnostic is a positional fact, and a
>     duplicate carries no information; duplicate copies were removed from 63 corpus entries, with
>     zero drift in the run column and exit code (the apparent “duplicate” in borrow_conflict_err is
>     an artifact of the baseline triple format not including a column number—the two E2018 entries
>     are legitimate diagnostics at different columns).
> 13. wasm playground landed (4.2.8): `run_code`/`test_compile` construct
>     `Program { kind: WasmPlayground }` + Driver through `compile_playground`—the `Compiler`
>     wrapper layer no longer participates. Error text is aligned byte-for-byte with the old
>     `CompileError` Display prefix (`Parse error:/Type error:/Internal error:`), with zero visible
>     difference in the playground UI. WasmPlayground and SingleFile share SINGLE_FILE_STAGES +
>     FailFast (the existing 4.1 declaration, now with a real producer).
> 14. Multi-file monomorphization and CHECK stage-table correction (3.4.8, user ruling on
>     2026-10-10, **partially withdrawing the mono exclusion in note #10**). Empirical evidence
>     disproved the claim that “mono only produces resource-limit noise”: with pathological generic
>     recursion (`f(x)=f([x])`), the compiler process overflowed the stack on the multi-file path
>     without mono (0xc00000fd—the interpreter recurses in Rust, with no graceful runtime error),
>     and the mono depth gate is the only compile-time line of defense; standalone check before
>     4.2.5 went through the full pipeline and therefore already had mono, so the exclusion ruling
>     introduced a silent coverage regression (“zero corpus dependency” only proves that the corpus
>     has no pathological fixture). The “type-erasure fallback” assumption was invalidated by the
>     RFC-033 reflection ruling—`^^List(Int)` requires a real instantiated identity, and
>     monomorphization is the foundation of reflection rather than an optional optimization. Four
>     design points: ①typecheck produces an instantiation_request for qualified calls (`lib.f(x)`,
>     FieldAccess form), with the generic_id using the qualified name, in the same namespace as the
>     merged IR (currently only bare Var is recognized, expressions.rs:2455); ②the `Stage::ALL`
>     topology changes to Linking before Monomorphization, and the MULTI_FILE stage table gains
>     Monomorphization at its end— consuming merged_ir and aggregating requests from all units
>     (qualifying the deferred bucket’s containing_fn), elevating mono from a single-module pass to
>     a whole-program pass; the single-file stage table remains unchanged (no Linking, so the
>     subsequence property is preserved and IR snapshots show zero drift); ③CHECK gains Linking
>     (pure merge—E3020 entry validation remains in RoleClassification to prevent duplicate reports)
>     and Monomorphization (check-only, like IrGeneration in the Check form); ④resource protection
>     now covers cross-unit mutual recursion through whole-program BFS. The `Monomorphizer` itself
>     is unchanged (its contract for consuming ModuleIR + the request set happens to match the
>     merged IR form).
> 15. 3.4.8 R1 (①) landed (2026-10-10, a9438007). Qualified-call instantiation collection was
>     implemented according to the #14 design: `ExpressionInferrer` is injected with a
>     module-qualified key table (the three `use` arms register it—ordinary / alias / item-by-item
>     alias and only for SubModule-class exports), and `callee_generic_name` is joined through
>     `SymbolTable::qualify` with a qualified name from the same source as the merged IR; the first
>     path for qualified calls has a tighter arity criterion (when the parameter list cannot be
>     resolved, it falls back to the second path using the signature to obtain actual arguments,
>     avoiding false E3018 reports for same-named functions across modules). Three asymmetric
>     defects on the mono side were empirically found and fixed in the same round: (1) the
>     deletion-key (generic-name set) and rewrite-key (containing_fn + name + span triple)
>     granularity mismatch—a call site whose actual arguments cannot be resolved degraded into a
>     symbolic request in the deferred bucket; if the containing function was non-generic, the
>     bucket never drained, but the original entry had already been deleted by name → a dangling
>     call (empirically confirmed by runtime E6006 in `list_ops.yx:53`); fixed by restoring the gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the sole source
>     of truth to determine coverage, and retaining the original entry for uncovered sites. (2) the
>     `HashSet` iteration order in the restoration section made function-table order
>     nondeterministic (t2 re-review F1) → changed to `Vec` and sorted by name, making the artifact
>     byte-for-byte reproducible. (3) the TailCall / MakeClosure forms still looked up mapping keys
>     despite the comment claiming conservative behavior → a single-point `call_form_is_rewritten`
>     determination was introduced, and all non-rewritten forms are counted as uncovered. Each of
>     the three has a pinned test (red-state empirical evidence of discriminating power). The corpus
>     differential baseline was regenerated in the same commit with exactly one changed line (a
>     `remove_at` frame gained `(int64)`). **Commit-strategy note**: when typecheck collection and
>     the mono fix were split into two commits, the staging tree containing only the former
>     triggered a `stdlib_docs` red against a pre-existing HEAD defect (`list.is_empty([])`, same
>     E6006 family)—the two halves are each other’s prerequisite for a green build, so they were
>     combined into one atomic commit.
> 16. Five rulings registered on 2026-10-10 (after R1 closure): (1) **D2 ruling A**—a user module’s
>     exports carry `Export.type_params`, together with `generic_fn_type_params_snapshot()`, in the
>     same form as the std path (`yx_sources.rs:62/108`): the declared name becomes the
>     authoritative source for qualified-call monomorphization, and signature reverse-inference is
>     downgraded to a fallback (folded into R2’s scope). (2) **HKT declaration side live, call side
>     broken** (inference passes type arguments E1002 / explicit type arguments E1010) is registered
>     as WBS 3.5 (three third-level tasks). (3) **The deferred bucket never drains under a
>     non-generic container** is folded into acceptance subtask 3.4.8 ②: the bucket must drain or
>     fail loudly, and it must not silently remain unspecialized. (4) **Closure variables called by
>     name produce runtime E6006** is registered as WBS 3.6; the semantics (i) support indirect
>     calls / (ii) reject at compile time, and **(i) was ruled**—indirect calls are supported: the
>     `CallDyn` mechanism already exists (the closure-expression path in `ir_gen.rs:7945`); what is
>     broken is the `Var` callee being downgraded to a by-name `Call`, even though typecheck already
>     accepts the program. (5) **Trade-off criteria fixed**—a proposal may be rejected only on
>     correctness and readability grounds; “large change surface / introduces a new mechanism / high
>     cost” does not constitute a rejection reason, and this is recorded in the second part of
>     `coding-rules.md` and the HOWTO self-check table; trade-offs requiring a user ruling must
>     provide all four items in `AGENTS.md` “When requesting a user ruling (mandatory)” (situation +
>     evidence / pros and cons per option / explicit recommendation + cost / explicit statement of
>     breakage).
> 17. 3.4.8 ② landed and F1 root cause identified (2026-10-11, 375789d2 + d922a551). ② was
>     implemented according to the #14 design: `Stage::ALL` changed to Linking before
>     Monomorphization, and the MULTI_FILE table gained mono at its end; the Linking arm aggregates
>     whole-program requests and qualifies `containing_fn` **before taking away each unit’s IR**
>     (otherwise the deferred-bucket keys and merged IR have different sources and the bucket never
>     drains). **① closure (D2 ruling A)**: user-module exports carry a declaration-time
>     type-parameter snapshot in `Export::type_params`, in the same form as the std path
>     (`yx_sources.rs:62/108`). **Existing defects empirically found and fixed during
>     implementation**: (a) **F1**—`qualify_names` rewrites only function bodies; `ir.init` is never
>     visited, so in multi-file builds function definitions are qualified while module-top-level
>     calls remain bare names → runtime E6006 (**reproduced even for non-generic code**, unrelated
>     to generics); fixed by making init reuse the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) the mono-side “whole-program unique suffix” heuristic and the silent
>     `continue` on a miss → the heuristic was removed, and misses now produce a **loud E3018**
>     (ambiguous silent mismatch is worse than failure to resolve). **A regression found and fixed
>     by independent verification**: t6 found that a named import (`use lib.{pick}`) followed by a
>     bare-name call was falsely rejected with E3018 (the IR instruction name is the alias-qualified
>     name, while the request name is the bare name; the aggregation side qualified only by unit
>     key) → fixed by having the aggregation side **directly call ir_gen’s** `import_aliases_from`
>     and `resolve_call_target` (extracted as a public single point, one producer and one consumer),
>     with a matrix of pinned tests covering both named-import forms. **Retained record**: the blank
>     `DefId` in the init section was empirically found to be a “consistency blank” rather than a
>     live defect (the codegen name fallback can resolve it, and all four kinds of init call have
>     been tested successfully), and is registered as WBS 3.7.4 to close the split in the “function
>     bodies use def, init relies on names” policy.
> 18. Init-section DefId policy split closed (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` was
>     refactored to extract the `resolve_static_targets` single point (Call/TailCall string
>     operands + MakeClosure), and function bodies and `ir.init` share both the implementation and
>     the `resolve` closure; after regenerating 208 IR snapshots, only 309 `(def=_)`→`(def=dN)`
>     pairs changed (other-lines 0; no residual `(def=_)` anywhere in the init sections of the full
>     tree). The codegen DefId primary path and the name-fallback state produce identical
>     translation results and execution values item by item (all four kinds of init call tested),
>     and the corpus baseline shows zero drift; the probe was inverted and strengthened under the
>     new policy (identity + exact index + execution value; falling back to init coverage turns it
>     red). **Erratum (2026-10-11)**: the above commit numbers were rewritten by
>     `git pull --rebase`—the original b554497b/e60dcc87/b7ac7396 correspond in order to the
>     post-rebase 375789d2/d922a551/38bd4b9e; the pushed dev history uses the new numbers (the old
>     numbers were local temporary numbers and never entered remote history).
> 19. 3.4.8 ③④ landed and verified (2026-10-11, 8ecf56f93). (③④’s design basis is in #14; ②’s
>     implementation is in #17, and this item closes it.) **Implementation**: the tail of
>     `CHECK_STAGES` = `…IrGeneration → Linking → Monomorphization`; Linking in Check does only a
>     pure merge (aggregate requests + `link_module_irs`), with the entire entry-validation block
>     left to `RoleClassification`; `ir_generation_check` retains `us.ir` after generation (in
>     memory only); mono failures enter the collection channel in Check, while MultiFile retains
>     `fail_with`; `into_outcome` first takes `check_diagnostics` and then checks for failures
>     (structural failures no longer clear already collected diagnostics). **Verification**: four
>     rounds of independent verification (t3/t4/t10 + t5 cross-form matrix) and one re-review (t6)
>     all passed; the corpus differential was 340/340 line-for-line IDENTICAL, IR snapshots showed
>     zero drift, and the test count increased from 2643 to 2656. **Existing gaps found and fixed in
>     the same round**: F1 (the request name and IR definition key in rootless multi-file check had
>     different sources → silently remained unspecialized; see the implementation note in the WBS
>     3.4.8 row) — the fix was for the aggregation side to perform an exact reverse lookup by the
>     true definition key (not a heuristic); the first version used the true definition key for an
>     exact reverse lookup (`flat_defs`); **same-day erratum**: this bridge was subsequently deleted
>     during regression fixing, and the root cause was instead solved by decoupling the dual use of
>     `module_key` (`script_mode`), with all three having the same source, `{unit key}.{name}`. The
>     location and fix for this regression (CI documentation-example gate: 258 blocks / 10 failures
>     → 0 failures) are described in the implementation note in the WBS 3.4.8 row. **Retained record
>     (existing, not a regression from this round)**: the E3005 help text does not match the
>     resource-protection semantics; the span label for multi-file E3005 is misplaced (reproducible
>     with the HEAD binary); the type-parameter position dependency in the request-collection
>     surface belongs to the same family as 3.5.

> 20. 3.9.1 / 3.10.1 / 3.11.1 landed and the diagnostic surface closed (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution-layer guard reachability)**:
>     interpreted calls were changed to **explicit stack-driven
>     execution**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/ `SpawnFromList` push frames and return
>     control to the outer `step_one`; a frame carries `ReturnTarget{Host|CallerSlot|Discard}`, and
>     on return it writes the value back to the caller’s slot; multiple call instructions continue
>     through the frame’s `pending_calls`. Baseline measurements showed that each interpreted call
>     consumed **2544 B** of native stack, with the crash threshold at `count_down(394)` (Windows
>     main thread, 1 MiB); after the fix, 20,000 levels of recursion can run on a 512 KiB stack
>     thread (approximately zero per level). The guard (`push_frame`) is thus reachable under every
>     entry point and every `max_stack_depth` value; `ExecutorError::StackOverflow` was changed to
>     carry the real limit (previously `util/diagnostic/mod.rs` always passed 0). **The original
>     3.11.2 (deep-stack echo truncation) was merged into this task by user ruling** (same file,
>     inseparable by file): the echo now shows the first 16 frames + an omitted count + the final 8
>     frames, with shallow stacks unchanged frame by frame. **3.10.1 (no compile-time guard against
>     native recursion, #447)**: locate first, then fix—the crash point was recursion in
>     `MonoType::type_name` (**328 B/layer**, SP probe + exact threshold determination); the same
>     path also has a derived `Clone` (~400 B/layer), deep copies inside `substitute`, and
>     `type_name_hash` (164 B/layer), plus `collect_generic_type_refs`; after changing all four to
>     work lists and removing one full-request deep copy per layer, a pathological generic recursion
>     with `mono.max_depth=5000` now produces **E3005 (with the real limit)** and no longer
>     overflows the stack. **3.11.1 (task-boundary error-code fidelity, #448)**: the error side of
>     task results was changed from a value to a **structured form**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`), with values
>     restored across the boundary; E6001/E6003/ E6004/E6008 inside tasks are word-for-word
>     identical in Standard and Embedded messages. Two **pub API payload-shape changes** were marked
>     `BREAKING CHANGE` in the commit. **Commit-strategy note**: 3.9.1 and 3.11.1 are **inseparable
>     at file granularity**—`interpreter/executor/executor.rs` hosts both the trampoline and the
>     error-channel producer/consumer sites, while the `TaskFailure` definition is in
>     `runtime/engine.rs`; splitting the two sides into separate commits will not compile
>     (pre-commit compiles the staged subset and rejected it twice), so under the same policy as #15
>     they were combined into **one atomic commit**. **Known boundaries and follow-up**: derived
>     Drop (~75 B/layer) ⇒ `mono.max_depth ≳ 13000` may still crash; the E4012 (constant evaluation)
>     gate bears on native recursion (approximately 3,696 B/layer; default 256 layers ≈ 924 KiB),
>     with no current call sites and no production reachability; future wiring must close the same
>     family; the Standard-mode task interpreter is constructed with the default `ExecutorConfig` ⇒
>     its E6004 limit is always 1024 (configuration propagation registered as WBS 3.12, user ruling
>     B: start a separate team); the test-runtime budget has been moved to issue #449 (pathological
>     fixture 76s, clippy for both targets, full submission run), which is outside this RFC’s
>     implementation surface. **Corpus directory stratification (same round, 8605a081b)**:
>     `02-type-system` (78) and `06-compile-errors` (134), 212 fixtures in total, were split into 23
>     category-based subdirectories, and 88 IR snapshots were relocated along their mirrored paths;
>     the corpus baseline was regenerated in the same commit (342 lines, independently recomputed
>     and compared line by line: only the `file` field and two new changes), and 41 snapshots were
>     regenerated because their file-header paths became longer, with line-by-line verification that
>     **only span offsets changed** (zero differences after normalizing offsets).
> 21. LSP semantic-data pipeline unified (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, and cross-file references in a project carry
>     `resolves_to` pointing to the real definition— the old form triggered **two separate
>     compilation passes** through didOpen/didChange: diagnostics went through the unified Driver
>     pipeline, while semantic data went through server.rs’s manual lex→parse→single-file typecheck
>     (with no module registry, so in-project `use` resolution failed and reference events were not
>     produced, causing cross-file jump/rename/find-references to fail silently—the silent failure
>     registered as D55 is closed by this task). **Implementation**: (1) `Export` carries
>     `definition_span` (harvested by extract_module_info from AST identifier nodes, with the same
>     source and value as the typecheck definition event’s `def_span`; native std is always None),
>     and `ModuleInfo` carries `definition_file` (filled by the Registry arm from DiscoveredFile);
>     (2) when the checker drains reference events, it uses `import_def_locations` to redirect
>     cross-file imported items’ `resolves_to` from the use line to the real location in the
>     defining file (std imports have no registration and continue to resolve to the use line); (3)
>     the checker module identity was changed from the unified `"<module>"` to the unit file path
>     (this value is consumed only on the semantic-data surface, with no diagnostic
>     consumption—confirmed by all four `env.module_name` consumption sites); (4) `DriverOutcome`
>     gained a `semantic_db` channel—CollectAll forms (an Lsp target file regardless of diagnostic
>     success + the SingleFile fallback) feed into it, and the Lsp arm’s tail injects top-level
>     exported definitions from non-target files into the registry (`external_definitions`, so even
>     unopened files can be jumped to; type/submodule exports are not injected—typecheck-side type
>     registration does not produce definition events, so both sides preserve the same absence); (5)
>     the orchestrator gained `analyze_source_in_project` (`check_source_in_project` now delegates
>     to it and takes only the diagnostic half); (6) the LSP-side `analysis.rs` single-compilation
>     entry point replaced the manual pipeline (server.rs’s `update_semantic_db` was deleted), and
>     World keys were unified as normalized file URIs (`uri.rs`: `path_to_uri(uri_to_path(u))` is an
>     idempotent normalization; drive-letter case and %XX encoding differences are normalized to the
>     same key). **Incidental fix (D59 routing class B—the old behavior silently did nothing and did
>     not change the externally visible behavior of any previously successful program)**: cross-file
>     jump/rename/find-references in project files changed from silently missing to hitting the real
>     definition; on lexical errors, the old manual pipeline deleted semantic information, whereas
>     the new behavior retains partial-AST highlighting (an extension of the 4.2.6 editor
>     philosophy); when the target file has type errors, semantic data is no longer discarded along
>     with `type_result` (highlighting/jump do not disappear while the user is typing). **Pinned
>     tests**: six at driver level (target-file path key / cross-file resolves_to +
>     external-definition round trip / std-import fallback to the use line / error-file retention /
>     single-file fallback path key + error retention), and one LSP e2e test (red before
>     green—before the fix, the jump returned None), plus four URI-normalization tests.

**Migration method for the ten entry functions (each function specified individually)**:

| Entry point                                                | After migration                                                                                                                                                                                                      |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete the function body; construct `Program { kind: SingleFile, … }` and pass it to Driver                                                                                                                          |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                              |
| `compile_project` (`orchestrator.rs:99-237`)               | Reduce to a `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                         |
| `check_project` (`orchestrator.rs:273-399`)                | Reduce to a `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                           |
| `check_source_in_project` (`orchestrator.rs:450`)          | Replace with one Driver call + filter the result to the target file                                                                                                                                                  |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Replace with `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                         |
| `lib::run_file`                                            | Unchanged (delegates to `run_project` or `SingleFile`)                                                                                                                                                               |
| `lib::run_project`                                         | Unchanged (delegates to Driver)                                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project mode uses `Program { kind: Lsp, aggregation: CollectAll }`; delete the **manual lex→parse→… sequence** in the single-file branch and call `Program { kind: SingleFile, aggregation: CollectAll }` instead |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | Delete the `standalone` branch (`614-616` → `check_single_file` `623-661`) and uniformly use `Program { kind: Check }`                                                                                               |

**The degree of duplication between `check_project` and `compile_project` (approximately 40% shared
/ 60% divergent)**. The shared parts are vendor-consistency verification, file discovery,
`build_registry_from`, `all_method_bindings`, the parsing loop, per-file checker assembly, and
result handling. The divergent parts are items 2/3/4/5/7/8/9/10/11 in the 11-item table above.
**This 40% shared skeleton is precisely Driver’s value interval**—the divergent 60% consists
entirely of “a different set of stages” or “a different aggregation mode,” and these are exactly
what the two fields of `Program` can fully express.

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

The reason is that skipping is determined by **topology**, not by the stage itself. If every stage
returned `Skipped` on its own, then the information “why did I not run?” would be scattered across
12 stages and could not be centrally audited. Instead, `dispatch` determines it at the beginning of
the loop according to dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the defect addressed by this
document: the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent “skip”—it
produces no diagnostic, merely because the obligation happens to be empty. Once this rule is
established, every “Y did not run because X was not done” must be explainable.

The diagnostic text must distinguish three skip reasons:

| Reason                                                      | Text direction                                                                                                                            |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by an upstream `Abort`                               | “Stage Y did not execute because upstream stage X failed”                                                                                 |
| Caused by an empty prerequisite obligation                  | “Stage Y did not execute because there were no pending obligations (normal)”—**severity = Info; does not count toward the warning total** |
| Preconditions not met (e.g. `config.mono.enabled == false`) | “Stage Y did not execute because it was not enabled by configuration”                                                                     |

The second category is critical: it distinguishes a “normal skip” from an “abnormal skip” in the
diagnostic stream and does not pollute `yaoxiang check`’s `warning_count` (the non-blocking contract
relies on this count at `diagnostic/mod.rs:637-641`).

### 5. Diagnostic aggregation modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // Abort on the first error (compile paths)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                              | Current equivalent |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137`, returns `OrchestratorError::TypeCheck` on the first error), `Pipeline::run` (four early returns at `pipeline.rs:150/162/174/193`)    | Existing           |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397`, no early return), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Existing           |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project files and single files within the same process, while the CLI’s `run` and `check` are two
independent calls; placing them on Program avoids Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ in more than “whether they
return early”—they are two different checker entry points. After unification, one implementation
should be driven by the `Aggregation` parameter rather than retaining both functions (**this is an
additional item requiring verification in this document; see “Known Limitations and Risks”**).

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
`layers/ownership.rs`, `ProofFunctionCall` remains in `proof/`, and the other field types remain
where they are; `driver/obligations.rs` holds only the aggregate container and `assert_drained()`.
L3 consumers (e.g. `ir_gen` reading `release_plan`) continue to receive concrete field types through
parameters and **must not `use crate::driver`**—consistent with the driver no-reverse-dependency red
line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not depend back on
`use crate::driver`**. A `Driver` appearing in a `TypeChecker` import is considered a violation and
is blocked by RFC-039’s proposed `scripts/ci/check-module-boundary.py`.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single file) | Five stages                              | The same five stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone input no longer uses this pipeline—it uniformly goes through `compile_project` (the MULTI_FILE stage table), with Script semantics driven by the rootless path; this row now describes only the `lib::run_file` / `Compiler::compile` library path)                                                                                                                                                                                           |
| `yaoxiang run` (multiple files)     | No proof_execution, no W1001/W1002/W1003 | proof_execution was connected as a P3 stopgap; **the dead-code-family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the W-code set of multi-file `run` is normalized with single-file `check` (the `dead_code_analysis` arm dispatches by “non-pipeline channel,” and Check / Lsp / MultiFile share the same implementation, with no second copy of the criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` and hands them to that arm for emission) |
| `yaoxiang check` (multiple files)   | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| LSP (in-project)                    | No proof_execution                       | Connected as a P3 stopgap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Z3 not installed + single file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change; see Compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Z3 not installed + multiple files   | Silently skipped                         | Unified with the above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

**The only intentional behavior break** is changing the `.expect()` at `predicate.rs:35` to return a
diagnostic. This conforms to RFC-027 §8’s “do not bind to a specific solver” positioning and to the
contract already declared by `backend.rs:60` (“the caller should degrade conservently”)—the current
implementation contradicts its own contract documentation.

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

| File                                               | Line range                                     | Change                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block)             | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                            |
| `src/lib.rs`                                       | `140-147`                                      | Change `run_file` to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                                    |
| `src/lib.rs`                                       | `154-167`                                      | Change `run_project` to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/pipeline.rs`                         | `141-227`                                      | Replace the `Pipeline::run` function body with `Program` construction + Driver invocation                                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `275-278`                                      | Move dead-code analysis out; implement it as the `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                   |
| `src/frontend/pipeline.rs`                         | `306-311`                                      | **Retain** `run_proof_execution`; move it into `driver` as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `381-389`                                      | Move monomorphization out; implement it as the `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                     |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                       | Reduce `compile_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                      | Reduce `check_project` to a Program constructor                                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `450`                                          | Change `check_source_in_project` to use Driver                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/module/orchestrator.rs`              | `1374`                                         | Change `compile_embedded_module` to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                      | Drive the `check_module` / `check_module_collect_all` choice in `typecheck_with_registry_in` from the `Aggregation` parameter                                                                                                                                                                                                                                                    |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                      | Delete the manual stage sequence in `run_diagnostics`; call Driver instead                                                                                                                                                                                                                                                                                                       |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                      | Uniformly use `Program { kind: Check }` in `check_files_with_diagnostics`                                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                      | **Delete** `check_single_file`                                                                                                                                                                                                                                                                                                                                                   |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                               | Change `test_compile` / `run_code` to construct `Program { kind: WasmPlayground }` (currently they implicitly reach path 1 through `Compiler::compile_with_source`)                                                                                                                                                                                                              |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                        | Change `.expect()` to return `SMTResult::Unknown` + a diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                    | Change the empty `ProofResult::Unproven { .. } => {}` arm to produce a diagnostic (**fixes the second silent-discard point**)                                                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` decision point | Add a warning diagnostic to the `unwrap_or_default()` degradation path                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                         | Correct the comment that disagrees with `1320`                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/layers/README.md`     | Entire file                                    | Change the layer-order table to the **actual** execution order; delete “a lower layer failing means an upper layer does not run” (no short-circuiting exists)                                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                        | **Per D58**: add a process-wide shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + a `with_shared_solver` closure entry point; change the three consumers in `predicate.rs`/`checker.rs`/`ownership.rs` to use the shared entry point. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`), and the original wording is superseded by D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the production-side logic of the `Unproven` branches (now distributed among
`checker/refinement.rs`’s `revalidate_refined` / `check_call_arg_refinements` /
`check_return_refinement` and the ownership-result handling arm in `checker.rs`; originally
`checker.rs:5164/5179/5306/5318/5420/5448`, relocated by P5 5.2.1)—the logic itself is correct;
**the problem is in the consumer, not the producer**, and changing the producer would conceal the
architectural defect; `Cargo.toml`; every `#[cfg(target_arch = "wasm32")]` branch (the reachability
determination for wasm branches belongs to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                       | Compatibility                                                                     | Handling                                                                                                                                                            |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add proof_execution to the multi-file path                   | **Breaking**: constraints that previously passed silently now report E4018        | `test_multifile_proof_obligation_not_dropped` is written **red first**, then fixed; staged releases, with expected diagnostic-set changes recorded in the CHANGELOG |
| Add W1001/W1002/W1003 to the multi-file path                 | **Breaking**: programs that previously compiled successfully now produce warnings | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), so the exit code does not change                                    |
| `assert_drained()` produces W-level diagnostics on first run | **Breaking**: the diagnostic set grows                                            | W first, then E; split into two steps under S2/S4                                                                                                                   |
| Change the `predicate.rs` panic to a diagnostic              | **Improvement**: no longer crashes                                                | No breakage                                                                                                                                                         |
| `build` / `dump_bytecode` subcommands                        | **No impact**                                                                     | These two paths do not pass through proof_execution                                                                                                                 |
| Migrate `TypeCheckResult` fields to `Obligations`            | **Internal refactoring**                                                          | If the `pub` API surface has external dependencies, synchronize them; all in-repository consumers are listed in the change list                                     |

---

## Implementation Points

This document corresponds to **P3 (Fix correctness defects)** and **P4 (Stage contracts and unified
Driver)** in RFC-039’s global stage sequence. S1–S5 below are the implementation order within this
document, and **the acceptance criterion for every step refers to the C2 category of the
[equivalence criteria document](07-equivalence-oracle.md)** (orchestration changes: the **diagnostic
sets** of all entry points are identical + corpus behavior is identical).

### S1: Establish the criteria (red first)

**Prerequisite; cannot be skipped.** The defect criterion must first be written as a failing test.

| Deliverable                                                                                                                                                    | Acceptance                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                  | **Must be red**. If it unexpectedly passes, the defect analysis in this document must be reviewed again                           |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48—do not mix it into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | The diagnostic sets of the single-file and multi-file versions of the corpus can be compared                                      |
| `src/driver/tests/obligations.rs` skeleton (initially `#[ignore]`)                                                                                             | Mark it `#[ignore]`; turn it green in S4 (red state retained in the record: removing the only settlement point yields `pairs: 0`) |

**Rollback point**: no code changes; tests are added only.

### S2: Introduce `Stage` + `Program` without changing behavior

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| Change `pipeline.rs:141-227` to invoke Driver            | **The single-file path’s diagnostic set and exit code are byte-for-byte identical**             |
| Differential test over the full corpus (293 `.yx` files) | Diagnostic lists (sorted by `(code, file, line)`), exit codes, stdout/stderr: **all zero-diff** |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. The
acceptance standard is a C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit; `pipeline.rs` returns to its original state.

### S3: Merge the four orchestrator entry points + fix the defects

| Deliverable                                                                                                                         | Acceptance (C2)                                                                                 |
| ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Change all of `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` into Program constructors | The diagnostic sets of the four entry points are identical after normalization by `Aggregation` |
| Delete `check_single_file` (`diagnostic/mod.rs:621-661`)                                                                            | `yaoxiang check` behaves consistently inside and outside a project                              |
| Delete the manual LSP stage sequence                                                                                                | LSP and CLI produce the same diagnostic set for the same file                                   |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                       | The defect is fixed                                                                             |
| Fix the empty ownership-layer `Unproven` match arm (originally `checker.rs:1313`)                                                   | `test_no_silent_pass_on_unproven`: `Unproven` must produce a diagnostic in every mode           |
| Change `.expect()` at `predicate.rs:34-36` to a diagnostic                                                                          | No panic when Z3 is absent                                                                      |

**Rollback point**: the defect fixes and the structural merge must be **two separate commits**. If
the structural merge causes problems, only the structural commit can be rolled back while retaining
the defect-fix commit—at that point `test_multifile_proof_obligation_not_dropped` remains green; the
reverse (retaining the structure and rolling back the fix) turns red, which is an unacceptable
intermediate state and must not be merged.

### S4: Enable the obligation ledger

| Deliverable                                  | Acceptance                                                                                                                                   |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | The 20 cases in `src/driver/tests/obligations.rs` turn green (implementation form see the §2 revision note; the count is based on that file) |
| `scripts/ci/check-obligations.py`            | The static gate takes effect                                                                                                                 |
| Promote obligation diagnostics from W → E    | After the full corpus differential, manually review every new E                                                                              |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration item;
switching between W and E does not require structural changes.

### S5: Finish the proof layer and wasm

| Deliverable                                                                                                                                                                                                                                         | Acceptance                                                                                                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change the layer order in `layers/README.md` to the actual order                                                                                                                                                                                    | The documentation corresponds line by line to the call sites in `checker.rs`                                                                                           |
| Process-wide shared singleton in `backend.rs` (D58; **hard prerequisite: Unknown must not enter the cache**—otherwise one timeout becomes fixed across compilations through the process-wide cache and pollutes sharing between cargo test threads) | Production-path `default_solver()` call sites drop to zero (tests only); cache-hit rate is observable across all three consumers (counter already landed in 89576fafd) |
| Add a warning to the degradation at the `TypeChecker::ownership_inputs` decision point in `checker.rs` (ownership block `:1296-1340`, emission point `:1531`)                                                                                       | A diagnostic is produced when there is no `body_checker`                                                                                                               |
| Determine the reachability of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                                 | The conclusion is delivered to the “F2 determination” subsection of `06-cleanup-inventory.md`; this document only registers the need for a determination               |

**Note**: correcting the layer order will change the diagnostic set and may expose many previously
silent `Unproven` results. **This item should proceed independently of S1–S4** and must not be mixed
into the same PR as the entry-point merge.

## Key Decisions and Rationale

| Decision                 | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a compile-time-exhaustively-enumerable enum with 12 variants; **runtime registration is rejected**                                                                                                                                                                                                                                                                                                                                                               | An exhaustive `match` is zero-cost compile-time enforcement: adding a variant causes `dispatch` to fail to compile. This is structurally identical to the mechanism already proven by the opcode table in RFC-039 routing table B. `StageScope` further turns “per module or per project” into a type-level fact, eliminating call-count questions that require human reasoning                                                                                                              |
| **Obligation mechanism** | Use `Obligations` + `assert_drained()` for **runtime** settlement, together with the **static** gate `scripts/ci/check-obligations.py`; **the final severity is E level** (user ruling A on 2026-10-13: same-code promotion to E, code number unchanged; **implementation form see the §2 revision note**: the ledger does not copy the field structure; the denylist—zero entries after clearing—retains the mechanism, does not enter the user-visible diagnostic stream) | This fixes an entire class of bugs, not one bug: at least two verified instances of the same hazard exist (`proof_calls`, `checker.rs:1313`), and all 13 obligation fields are in scope. Splitting W→E into two steps was intended to keep the C2 criterion usable at every step—now that the obligation inventory has been cleared to 13 and the denylist is zero, the healthy state emits nothing, so the final state takes E directly (see sections VII and VIII of the §2 revision note) |
| **Fix scope**            | Fix the **consumer** (orchestration layer); do **not** change the production-side logic of the `Unproven` branches (now distributed among three sites in `checker/refinement.rs` and one in `checker.rs`; originally `checker.rs:5164/5179/5306/5318/5420/5448`)                                                                                                                                                                                                            | The problem is in the consumer, not the producer. Changing the producer would conceal the architectural defect as “the logic has been fixed,” even though the logic of these three branches is itself correct                                                                                                                                                                                                                                                                                |

This design additionally eliminates two classes of cracks:

- **The crack between “a comment’s promise” and “the code’s behavior.”** The “must not let silent
  pass return” statement in `checker/refinement.rs` (originally `checker.rs:5165`) is a
  **comment-level contract**, fulfilled by `assert_drained()`.
- **The crack of “an array that someone forgot to append to.”** Currently, five functions each
  hand-write their call sequence; after unification there is one stage table, and a missing new
  stage causes a compile-time error.

### Rejected Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—seven of the 11
  inconsistencies (two dead-code-analysis implementations, W1006, W1003, monomorphization, role
  context, and IR generation/linking) **involve no type handoff**; they are configuration questions
  of “whether a given analysis should run”; it also cannot express `Aggregation`, and would merely
  move the `check_module` / `check_module_collect_all` choice from the function layer to the type
  layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—this cancels
  compile-time exhaustiveness: adding a stage no longer causes `dispatch` to fail to compile,
  replacing “five functions each hand-write their calls” with “an array someone forgot to append
  to,” which is precisely the cause of the current 11 inconsistencies. The repository already
  contains a counterexample of the same form: the `ModuleDependencyGraph` / `affected_modules` /
  `ModuleCache` / `HotReloader` described under `docs/src/dev/design/check/` have zero
  implementations.
- **Fix only the bugs without changing the structure**—this can fix the two known bugs, but covers
  only one of the obligation fields; `Stage` remains scattered across five functions, and the next
  point of divergence will continue to grow from here. **It must be done first** (as part of S1/S3),
  because the structural refactoring needs a known red test to prove that the criterion works.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry point
  and does not know who its caller is, so `debug_assert!(<caller will handle>)` cannot hold;
  `#[must_use]` warns only when the field is discarded **as a whole**. This searches for the bug at
  the wrong layer: the bug is in the orchestration layer, so detection must also be in the
  orchestration layer.

## Known Limitations and Risks

- **Slimming down `orchestrator.rs` makes it harder to read in the short term**. After the 139-line
  `compile_project` is split into a “Program constructor + several driver arms,” readers must cross
  two files to understand the flow. This is the common cost of every refactoring that “centralizes
  the wiring.”
- **S3 will significantly change the diagnostic set, in the direction of “exposing problems that
  were previously silent.”** After the fix, users may report a batch of “new errors”—they are real
  bugs that were never reported before. This must be stated explicitly in the CHANGELOG.
- **Switching the severity of `assert_drained()` requires manual judgment field by field**. For some
  obligation fields, such as `module_namespaces`, the absence of a consumer may mean that
  consumption is not needed by design and should not trigger an alert. S4 must review each field
  rather than treating all fields uniformly.
- **The `Program` abstraction may be premature**. If the stage sets of certain entry points remain
  unstable for a long time, `stages()` will degenerate into a free parameter that accepts a
  different array on every call, and the contract constraint will fail. **Mitigation: the
  `test_program_stage_coverage` assertion ensures that `stages()` can only come from the six
  predefined combinations.**
- **Criterion dependency**: the defect-fix criteria in S1/S3 depend on the defect tests in the
  [equivalence criteria document](07-equivalence-oracle.md) turning red first. **If that document
  has not first established a multi-file corpus layer, S1 cannot be accepted**—the existing 293
  corpus entries all take the single-file path and provide zero coverage for this class of defect.
  This document does not cover the IR verifier (`verify_loose`); that prerequisite work is outside
  this document’s scope.
- **Correcting the layer order will expose previously silent `Unproven` results** (`equivalence` is
  not in the pipeline at all, and the actual order of `termination` and `ownership` is the reverse
  of the declared order). This is a diagnostic-set change risk and should proceed independently.
- **Thread safety after changing the SMT backend to a singleton is undecided**. `backend.rs:13-19`
  states that `Solver` is only `Send`, not `Sync`, and `Z3Backend`’s cache is a `RefCell`; after
  changing it to a singleton shared across compilation units, all access paths must be confirmed to
  pass through a `Mutex`.
- **Stage parallelization has not been evaluated**. Multi-file typechecking is naturally
  parallelizable, but parallelism can **mask order-dependency defects** (such as the actual order
  dependency between termination and ownership). This should be considered only after the
  equivalence criteria are stable.

> **The open questions originally listed in this section have all been ruled on.** See
> [RFC-039 Ruling Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document retains no unresolved items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage tables and dependency-direction rules,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch-reachability cleanup;
  `checker/semantic_tokens.rs` `include!` refactoring (implementation steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three criterion layers,
  `test_multifile_proof_obligation_not_dropped` defect criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source for Phase 2.5 proof-function execution
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker/refinement.rs` — Explicit statement that “silent pass must
  not return” (originally `checker.rs:5165-5169`)
- `src/frontend/core/typecheck/layers/README.md` — Layer-order call table and the true identity of
  `equivalence` (no short-circuiting)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit rationale for hard failure
  during SMT initialization
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration that “the caller
  should degrade conservably” (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream code must
  consume (after clearing under 3.4.4/3.4.5/3.4.9)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test’s own
  admission that it “stops before the pipeline”
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus’s own admission that it is “executed by pipeline.rs after check_module”
- `compile_playground` in `wasm/src/lib.rs` (compiled through by `run_code`) — the playground’s
  fourth compilation caller (the single-file path, so proof_execution executes)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Build-time error-code gate, a model for this project’s enforcement mechanisms
