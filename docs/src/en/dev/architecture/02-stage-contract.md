---
title: 编译阶段契约与义务账本
---

# Compilation Stage Contract and Obligation Ledger

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion gradation, and execution stage ordering are in the main
> body of RFC-039; the positioning of each companion document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 Orchestration Layer** construction blueprint of RFC-039. It addresses one
problem: **"Which stages ran in this compilation, which didn't, and why" is currently scattered
across six paths and ten entry functions, each manually wired, with no single place that can answer
this question.**

The current state can be summarized in one sentence:

> **The invariants the code itself declares are violated by architecture rather than by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` reads "Unproven → compile error, no
degradation, no silent pass... do not let silent pass be resurrected." But this invariant only holds
on the **single-file path**: refinement constraints of the form `x: Sorted(3)` in the proof function
**silently pass** under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the
proof function is never executed and produces no diagnostics.

### Coverage

- `Stage` enum (exhaustive, non-runtime registration) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- Unified `Driver` (single `dispatch`) and the per-entry-function transformation of the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation patterns
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Out of Scope

- Four-layer model, dependency direction specification, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 gradation, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → main body of RFC-039

### Division of Labor with RFC-039

RFC-039 provides the **why** of the refactor and the **order** in which to do it; this document
provides the **specific form** of L1, the **per-file change list**, and the **implementation stages
within this document** (corresponding to P3 "fix correctness holes" and P4 "stage contract and
unified Driver" of RFC-039's global sequence). Wherever this document conflicts with RFC-039,
RFC-039 prevails.

The equivalence criteria are executed per the **C2 (orchestration changes)** category of the
[Equivalence Criteria document](07-equivalence-oracle.md): same diagnostic set per entry + same
corpus behavior.

## Current State

> All items in this section are **verified facts**, each with a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only Structural Defect Where "One Mistake Silences the Whole Repo"

It differs in nature from the other two defect types (module boundaries, test wiring):

| Defect                  | Typical Manifestation               | Has Signal?          |
| ----------------------- | ----------------------------------- | -------------------- |
| Lexical/syntactic error | Source code written wrong           | Has diagnostics      |
| Type mismatch           | Type written wrong                  | Has diagnostics      |
| Stage missed            | Some stage not called at all        | **No signal at all** |
| Obligation unconsumed   | Field filled in but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed in RFC-039:
"This project treats 'design' as a documentation convention rather than an executable constraint."

### Enforcement Mechanisms Already Present in This Repo

The same repo already has mature enforcement mechanisms; they just haven't been extended to the
stage layer:

- The **145 error codes** in `src/util/diagnostic/codes/` (137 E + 8 W) are gated at **build time**
  by `build.rs:19-55` via `tools/code-tables`, which compares each entry against the RFC-013 code
  table and any inconsistency causes a `panic!` that refuses compilation.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` is 6227 lines / 47.9%) has a high
  test density, and the test subtree of each module is declaratively wired—this is the most complete
  test wiring in the whole repo, and can serve as the formal reference for stage-layer gates.

**The design capability is sufficient. What's missing is "placing an equivalent gate at the
orchestration layer."**

(Line count basis: `(Get-Content).Count`, see the "Line count basis" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                         | Which Compilation                                                                  |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | 5-stage direct call                                                                |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collects all diagnostics                                       |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`:493`)                                                |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`:1386`) + independent IR                                          |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project via 3c, otherwise manual lex→parse                                      |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`:590-591`)                                                     |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → path 1 |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`:230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`:268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`:306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded in it** (`:381-389`) |

### 11 Stage Coverage Inconsistencies

The table below provides cell-by-cell evidence. **A blank cell does not mean "this entry doesn't do
this thing" but rather "no code in this entry does this thing"**—that is the very nature of the
defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                       | `check_project`                                                | `check_source_in_project` (LSP)     | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------- | -------------------------------------------------------------- | ----------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                  | **No**                                                         | **No**                              | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                  | Yes `356-377` (role-aware, no config gating)                   | **No**                              | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                  | Yes `336-348`                                                  | **No**                              | **No**                                |
| 4   | W1003 unused import                                      | Yes (`:277` collects `type_result.warnings`)        | **Collects but never emits**                            | Yes `:350`                                                     | **No**                              | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                  | Yes (same as 2)                                                | **No**                              | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                  | N/A                                                            | N/A                                 | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`:124`), **fail-fast return** (`:132`)  | `check_module_collect_all` (via `:315` → `:493`), collects all | `check_module_collect_all` (`:493`) | `check_module` (`:1386`)              |
| 8   | File discovery                                           | N/A                                                 | `discover` (`:101`, discards `used_by`/`shadow_events`) | `discover_with_used` (`:275`)                                  | `discover` (`:454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                  | Yes (`:321-328`)                                               | **No**                              | **No**                                |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`:147`)                    | **No**                                                         | **No**                              | **No**                                |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`:205`)                                        | Yes (`:152-236`)                                        | **No**                                                         | **No**                              | Yes (independent ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline and is preserved unchanged. Differences in the current state
> after P3 lands:
>
> - **Row 1 has been fixed**: proof_execution now shares the same implementation in
>   `frontend/proof_execution.rs` across five entries (pipeline + four orchestrator entries); the
>   sole-consumer defect no longer exists.
> - **Rows 2/4/5 have been fixed** (WBS 3.4.6, 2026-10-19 user ruling "do it"): the multi-file `run`
>   now emits W1003 and the dead code family warnings; the W-code set across the four paths
>   (single-file run / multi-file run / single-file check / multi-file check) is unified—the single
>   source of criteria is on the shared implementation of the `dead_code_analysis` arm.
> - **Row 6** (monomorphization single-file exclusive) → assigned to WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk not-yet-demonstrated defect).
> - **Row 7** dual-entry `check_module` / `check_module_collect_all` → assigned to 4.2.7
>   (Aggregation parameter-driven).
> - The code-side reference to "ruling #434" was previously zero-registered in docs—now registered
>   as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new fact outside the table, registered 2026-10-09)**: the
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch), while multi-file `discover` previously only recognized
>   explicit `use`—in-project for-loops compiled fine but threw E6006 at runtime (proven by probe).
>   Fixed and marked off WBS 4.10.2; the same-family "multi-file missing single-file step" parse
>   hard-abort quirk was registered as WBS 4.10.1—**also fixed** (landed immediately after 4.2.2,
>   the Check path degraded to per-file collection; under this audit's "stage coverage / field
>   consumption" dimension, this is a compilation-unit membership difference and falls into the
>   net-skipping dimension).
>
> - **Embedded std registration surface coverage (discovered during 4.2.5 construction, fixed)**:
>   the Registry arm used `extract_module_info` to re-harvest the embedded std unit, which would
>   override the native half registered by `with_std()` under "native + yx surface merge"
>   (`result.is_err` etc. lost → std.test falsely reports E1043). This audit's third net-skipping
>   dimension: "merge semantics for two registration sources of the same module key"—the
>   registration table of the single-file path is shaped by a single pass through `with_std()`,
>   while the Registry arm of multi-file/Check does per-unit insert to achieve coverage.

**Two points that need precise phrasing, or they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` **never** reports W1001/W1002/W1003 on
  the **multi-file** path (`run_project` → `compile_project`); on the **single-file** path
  (`run_file` → `Pipeline::run`) it does report. The reason lies in `compile_project`—it pushes the
  full `TypeCheckResult` (including `warnings`) into `type_results`, but this function has **no
  `result.warnings` read site anywhere in its body**; the only downstream use of `result` is
  `generate_ir_with_context`.
- **The `main` criterion for row 11 also comes from different sources**: `check_project:385-395`
  uses `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role`
  (`:250-252`, only checks "has manifest or not"). The two functions give different answers for
  "what file must define `main`".

### Correctness Hole: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — The obligation production point is unique.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** place in the entire repo
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

The semantics are "the predicate arguments of this refinement constraint are compile-time literals;
this proof function must actually be executed to determine the judgment."

**Step 2 — The consumption point is inside the checker, but it only emits, never consumes.** There
are **three isomorphic branches** in `src/frontend/core/typecheck/checker.rs` handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                        | Behavior                |
| -------------------------- | --------------------------------------------------------------------------------------------------------------- | ----------------------- |
| Parameter refinement check | `:5164` `if calls.is_empty()` → push hard error (`:5170-5177`); `:5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `:5306` → push hard error (`:5308-5316`); `:5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `:5420` → push hard error (`:5442-5446`); `:5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The comment at `checker.rs:5165-5169` reads verbatim:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch is therefore a real front line, not forward-looking padding: do not let silent pass be resurrected.`

That is: the author knew "producing no diagnostic" was a defect and explicitly classified it as a
**known, nowhere-to-go intermediate state**, assuming by design that "someone will definitely come
to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares the local
`proof_calls` and collects; `checker.rs:1439` writes
`proof_calls, // Phase 2.5 pre-registered proof function obligations` into `TypeCheckResult`. The
field is defined at `types.rs:29`.

**Step 4 — There is only one read site in the entire repo.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is read in **only one place** in production code: `src/frontend/pipeline.rs:187`
(`:189` passes as argument):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the repo fall into three categories, none of which
are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four orchestrator entries bypass that layer.** `compile_project` (`:99`),
`check_project` (`:273`), `check_source_in_project` (`:450`), and `compile_embedded_module`
(`:1374`) in `src/frontend/module/orchestrator.rs` **all do not call `pipeline.rs`**; they directly
call `TypeChecker::check_module` (`:124` / `:493` / `:1386`). Therefore they cannot even reach that
unique read site at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also take the
discard path.** `compile_embedded_module` (`:1374`, `check_module` call at `:1386`) is responsible
for compiling the embedded std. This means **the proof obligations of the embedded std modules
themselves are also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, and emits no diagnostic** under the
multi-file / check / LSP paths. The proof function is never called.

**Step 8 (Supplementary verification) — There is a second silent-discard point within the layer.**
`checker.rs:1303-1314`, when handling ownership layer results:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` from the ownership check layer is swallowed by the empty match arm—no diagnostic,
no bookkeeping.** This directly contradicts the promise at `:5164` that "do not let silent pass be
resurrected", and is **independent of the orchestrator problem**—even if the entry layer is
completely fixed, this point will still remain silent. (Handling timing: should be done in the same
batch as the obligation ledger, because it's the same kind of problem as the obligation ledger.)

### Why Tests Did Not Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts that `SumUpTo` calls exist
in `result.proof_calls`. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping precisely before `pipeline.rs`**.

The test file's own doc comment has already written this out.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation cases are at the .yx layer.`

**This is exactly the shape of the problem: the test verifies "the obligation was filled", and the
bug is "the consumer doesn't read it".** A test that only tests the producer and not the consumer is
naturally immune to this class of defects.

### A Stronger Finding: The Primary Corpus of the Equivalence Criteria Is the Single-File Path

The [Equivalence Criteria document](07-equivalence-oracle.md) treats the end-to-end differential of
the 293 `.yx` corpus files in `tests/yaoxiang/` as the **third-tier criterion** and the primary
acceptance method for C2 stages. But measurement shows:

- The `tests/` directory has **no `yaoxiang.toml` whatsoever** (zero hits on full-directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`:614-616`) for each corpus file → `check_single_file` (`:623-661`) →
  `Compiler::compile_with_source` (`:635`) → `Pipeline::run` → **proof_execution runs**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, none
  cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is exactly
the evidence of this. The file is annotated `// expect: compile-error E4018` (line 15), and its
header comment at line 7 reads "Status: ❌ Should be rejected at compile time"—**it can pass
precisely because it takes the only path that executes the proof function**.

**Conclusion: the third-tier criterion needs to be supplemented with a multi-file corpus layer,
otherwise it cannot serve as this document's acceptance tool.** See "Implementation Points" S1.

### Obligation Fields: 13 "Produce-as-Contract" Fields, Zero Mechanism Guarantee

In `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** of `TypeCheckResult` whose
doc comments explicitly state "produced by stage X → consumed by stage Y"—they are by nature
**cross-stage obligations**:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-taking convention**: definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment—the comment block for a
> field may be multi-line, and the declaration line is the only uniquely anchorable position. The
> **struct range** in the sentence above this table (`src/frontend/core/typecheck/types.rs:16-76`)
> floats with the struct's line numbers and is mechanically reconciled by gate **F5** (wrong = red);
> the definition line column is reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**:
> `ir_gen.rs:360` reads (`type_result.release_plan.clone()`), `:3103` uses (`ReleasePlan::consume`);
> settlement complete marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement complete marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Ruling record (3.4.9, 2026-10-13 user ruling A)**: the original 14th row `interface_impl_registry`
(`types.rs:49`) has been **reclassified as an in-tree fact**—by design it's consumed within the
typecheck phase and has zero cross-stage reads; it is not an obligation:

- **In-phase consumption points (living)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solving queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), read-only accessor at
  `checker.rs:214-217`, `checker.rs:974` injects into body_checker.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; only ledger-side registration and
  docs in `src/driver` (removed in this round).
- **Disposition**: removed the **declaration** in the doc comment that said "mirrored for
  middle-side/LSP consumption"; kept the field and its in-phase consumption; this field is
  registered in `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with reason). If LSP later
  genuinely needs this table, register it fresh per 4.6.x, **do not create a consumer in this
  round**.

**All of these fields are passed across layers keyed by `Span` or name table.** The Equivalence
Criteria document has pointed out that mismatch in span-keyed contracts will "silently fail, with no
error whatsoever" (`ReleasePlan` is a typical case: any inconsistency in how the two sides compute
span → `Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`) is
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in a downstream module, and that downstream module is
  on the required path of all entries.
- `proof_calls`'s consumer is at the **top level of `pipeline.rs`**, in **a different entry
  implementation**. The four orchestrator entries do not even pass through this layer of
  `pipeline.rs`.

**Verified fact**: the repo currently has **no mechanism whatsoever** to ensure that these 13 fields
are consumed. The only "protection" is that `ReleasePlan` happens to piggyback on IR generation.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: every field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why It's Not a Cross-Stage Obligation (Key Points)                                                                                                     |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Unit identity meta-info (0 hits in `src/middle`)                                                                                                       |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produce-as-contract")                                                                                 |
| `bindings`                | Has cross-stage reads (ir_gen.rs:735/:5034), but the doc comment does not declare a cross-stage contract ⇒ not incorporated                            |
| `local_var_types`         | Used for diagnostic rendering (ir_gen.rs:852/:4543/:4794), same as `bindings`—no contract declared                                                     |
| `semantic_db`             | Consumer is in LSP (since 4.6.1 exported through the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`), not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads at ir_gen.rs:4336/:6867, doc comment does not declare a contract                                                                     |
| `warnings`                | Warning diagnostic channel (same kind as `diagnostics`)                                                                                                |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; see "Ruling Record" in "§Obligation Fields" for in-phase consumption points)                                        |

**Why we don't "wire up a consumer" for these fields**: creating a downstream for these fields would
only introduce fake obligations and disguise "not requiring cross-stage consumption by design" as a
contract defect; the correct closure mechanism is **reverse containment**—`TypeCheckResult` field
set = obligation list ∪ non-obligation registration (judged hard by criterion 1 of
`scripts/ci/check-obligations.py`: add field and forget to register, delete field and forget to
sync, non-obligation registration without reason, overlap of the two tables all turn red
immediately).

### Four Additional Contract Defects in the Proof Layer

The following all come from proof-layer analysis, independent of the entry-fork problem, but all
belong to the "stage contract not enforced" category.

**(a) Layer ordering is only declared in docs, with no mechanism to enforce it; the `equivalence`
layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (positioning anchored by function name / drive point, line numbers
taken from the current tree):

| Actual Order | Call Point                                                                                                                                         | Corresponding Layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement walk driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                    | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                       | `ownership.rs`      |
| 4            | Type constraint solving `env.solver().solve()` (not in `layers/`)                                                                                  | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                             | `equivalence.rs`    |

Layer-number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) only exist in
each module's own header comment and do not correspond to call order; there is no "number → call
timing" mapping in the README.

`equivalence` is only used by `inference/assignment.rs:17` as a `is_subtype` utility function,
unrelated to `ProofResult`. There is also **no short-circuit whatsoever**—`check_module_impl`
`add_error`s each termination error then continues walking; "lower layer fails, upper layer doesn't
run" is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: the hard-fail panic has been removed by 3.3.1 of P3 (SOLVER slot
> made `Option`, missing causes conservative degradation to `SMTResult::Unknown`); "silent skip (no
> injection)" is supplemented with W1081 signal by 3.4.2. Unification of the three forms
> (singleton-ization) is executed per **RFC-039 D58**, the real fix is in `proof/smt/backend.rs`
> (new process-level shared singleton + `with_shared_solver` closure entry point); the
> `default_solver() → &'static` description in the 02 change list is subject to D58 (bare `&'static`
> references are infeasible: `dyn Solver` is not `Sync`).

| Consumption Point                                      | Acquisition Strategy                                     | When Solver Unavailable                                       |
| ------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic**     |
| `termination.rs` (injected via `checker.rs:1265-1268`) | `with_solver_owned` injection at construction            | `None` → **silent skip** (no injection)                       |
| `ownership.rs:627` (back-edge cutoff judgment)         | `default_solver()` called **each time**                  | `None => return false` (`:629`) → conservatively does not cut |

The comment at `predicate.rs:31-32` explicitly chooses hard-fail: "Initial failure maintains **hard
fail**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel capability'".
Yet the contract doc at `backend.rs:60` says "None: backend unavailable... **caller should
conservatively degrade**". **Two philosophies coexist in the same codebase, and the hard-fail one
panics instead of returning an error.**

**(c) Each back-edge judgment creates a new Z3 context, rendering the cache effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff judgment. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton**—each call does `Z3Backend::new()`, i.e., creates a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(For comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. The same backend, two
different lifecycle strategies.)

**(d) Checker silent degradation + comment inconsistent with reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call ownership table
```

When `body_checker` is `None`, the ownership check gets an **empty type ledger and empty call
table** and runs as usual—ownership analysis degenerates to "nothing conflicts", **with no warning
whatsoever**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least
left as a mark in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving". In fact, `self.env.solver().solve()` is at `checker.rs:1320`—**termination is
at `:1256` and ownership is after `:1296`, that is, what the comment calls "before" is actually "two
layers later"**.

### Current wasm State: shim crate carries it, 29-file branch is alive

The wasm target **is built and built in CI**, the `cdylib` is not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                             |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                   |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                   |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                          |
| Main crate has wasm target dependency section     | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:131-137` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                      |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                             |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., the playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning when missing)                                                                                                                                                               |
| **29 files** contain the `wasm32` literal         | Of which **22** have actual `#[cfg(...)]` attributes, another 7 are only doc mentions (list and per-item determination in the "F2 Determination" subsection of `06-cleanup-inventory.md`)                                                                                            |
| `orchestrator.rs` 16 occurrences                  | 10 attributes (4 positive + 6 negative) + 6 comment sites                                                                                                                                                                                                                            |
| `src/lib.rs` 11 occurrences                       | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                           |

**Conclusion: the `#[cfg(target_arch = "wasm32")]` branches of these 29 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (152 lines) under the
wasm target—`src/lib.rs:151/170/187` gate the entirety of `run_file` / `run_project` /
`build_bytecode` (all three need `std::fs`), while the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry
`compile_playground` (`:84`) of `wasm/src/lib.rs` constructs `Program { kind: WasmPlayground }` and
hands it to `Driver::run` (the JS-side entry `run_code`, `:122`)—**the single-file path**.
Therefore:

| Path Consuming `proof_calls`                              | Executes Proof Function |
| --------------------------------------------------------- | ----------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                 |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                 |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                 |
| Multi-file `run` (`compile_project`)                      | **No**                  |
| `check` (`check_project`)                                 | **No**                  |
| LSP in-project (`check_source_in_project`)                | **No**                  |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. When the Driver is unified, this path is
already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, which shares the same
single-file stage table as `SingleFile`, `:135`); landing record in row **4.2.8** of
`09-execution-wbs.md` (`wasm` is rerouted via `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope—which of the 10 wasm attributes (4 positive + 6 negative) in `orchestrator.rs` are
unreachable in the playground scenario—goes to the "F2 Determination" subsection of
`06-cleanup-inventory.md`: 4 wasm arms are unreachable in the playground runtime but load-bearing at
compile time, 6 `not(wasm32)` arms are load-bearing for the three native stage tables; no low-risk
items can be directly removed. The "build wasm target, or delete these branches" item in RFC-039's
open questions **is now obsolete**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-Runtime Registration)

**Core constraint: `Stage` is a compile-time-exhaustive enum, and runtime registration is
forbidden.** The reason is that Rust's exhaustive `match` can enforce at compile time that "a new
stage must be handled by the orchestration layer"—this is the exact same mechanism that the opcode
table has already proven effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check       Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexical + syntactic             PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/...) Project
    Typecheck,          // Type check (including embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis       Project
    ProofExecution,     // Compile-time execution of proof functions PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merge Project
}

impl Stage {
    /// All stages. When a new variant is added, both this array and the
    /// exhaustive `match` in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once
at project level) make "which stages must be run per module, which must be run once at project
level" a fact at the type level. The `Project` scope stage is structurally guaranteed to run only
once in `dispatch`—this eliminates questions like "should `allocate_global_slots` be called per file
or once" in the current `orchestrator.rs` that require human reasoning.

**The stage table is ordered topologically rather than alphabetically**, because failure propagation
depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: two deviations from the draft are corrected per
> implementation evidence—
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), and the draft order contradicts the data
>    flow.
> 2. The Check form's stage table is normalized to dead code **before** proof according to the
>    `Stage::ALL` topological order: the current state of `check_project` is proof executing before
>    dead code, with no data dependency between the two and identical diagnostic set (C2 set
>    semantics); only the in-file diagnostic order is normalized. The single-file path (dead code
>    embedded in typecheck, before proof) already conforms to the ALL order, and byte-level
>    acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry` / `RoleClassification` (C3, 2026-10-09 user
>    ruling): signature collection (`extract_module_info`) and `ast_has_main` both consume AST
>    products— the draft order would force the Registry arm to "hide parse", making the stage table
>    lie about data flow; and multi-file parse is thereby reduced from 2 times to 1.
>    As-current-state truthfully registered: multi-file path parse errors were once hard-aborted
>    (`?` propagation in `build_registry_from`, the tension with CollectAll semantics registered as
>    WBS 4.10.1)—**4.10.1 has been fixed** (2026-10-09 user ruling, rust-style collection
>    semantics + Plan B): Check path parse failure degrades to per-file diagnostic collection, the
>    sick file exits the compilation unit (doesn't enter the registry, importer reports E5001);
>    MultiFile path retains hard abort (under FailFast, a bad file cannot produce IR, which is the
>    correct semantics, valid on the nail-board long-term).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data-dependency
>    edges 14→16: `RoleClassification` consumes Discovery's used_by edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadow events—the draft omitted these (these two products were
>    produced by `discover_with_used` in the orchestrator era and discarded by the `discover`
>    wrapper; the data flow was lost once it didn't enter the table); b. The in-file diagnostic
>    order normalization is extended to the full form of note #2: the E3020 entry validation is
>    moved into the RoleClassification arm (check has no Linking stage, the arm holding surfaces and
>    AST is responsible for validation); in-file diagnostic order = stage topological order (E3020 →
>    typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set semantics),
>    only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "fields produced but unconsumed" from undetectable into a fact assertable at
compile time or test time. RFC-039 has listed this as "the highest-value item."

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
    /// Each unconsumed field produces a W-level diagnostic (default) or a hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                            | Criterion                                                   | Disposition                                                        |
| ------------------------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------ |
| Obligation read by declared consumer | The field in `Obligations` is `take()`-en / marked consumed | Pass                                                               |
| Obligation non-empty but no consumer | Field non-empty and not consumed                            | **Emit diagnostic** (W-level by default; E-level in `strict` mode) |
| Obligation empty                     | Field is empty                                              | Pass (no consumption required)                                     |

**Why start with W-level rather than E-level**: fixing obligation discard will **change the
diagnostic set**. The C2 criterion requires "same diagnostic set per entry", so if you go straight
to E-level, `yaoxiang check` will suddenly produce a bunch of previously-silent `Unproven`
diagnostics. Splitting it into two steps (first observe with W, then upgrade to E) makes every
step's criterion usable. See "Implementation Points" S4.

**The call site of `assert_drained()` is unique**: `Driver::run`, at the end of the stage table,
before producing `CompilationResult`.

**Accompanying static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears
≥2 times but no third file reads → fail). `assert_drained()` is the **runtime** gate, that script is
the **static** gate, they complement each other; both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: this note registers the
> **implementation form, measured numbers, and 4.3.2 interface** of the above sketch and three-tier
> table, corresponding code file:line by item. The landing commit is `1941cc47a` (verification tree
> `99079bc52`). It is also the **third** of the "no silence" three bearers of the red list (the
> other two: audit surface, static gate).
>
> **One, Deliverables (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (`:201`, field name /
>   consumer stage / red list), **non-obligation field registration `NON_OBLIGATION_FIELDS` (`:150`,
>   8 entries with reason)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), external emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer gating `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **Unique settlement point**: at the end of `Driver::run`'s stage table, before `into_outcome()`
>   produces `CompilationResult`— `src/driver/mod.rs:110` (`State::assert_drained`,
>   obligations.rs:570). The early-exit path of `Aggregation::FailFast` is also settled (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; removing any one will make the runtime W1090 necessarily
>   fire): ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the only outward-read path of
>   `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four identical
>   delivery ports: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read of request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()`
>   nailed by test cases.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, the reason is written in the entry above `:26-31` — the settlement
>   granularity is "unit × obligation", with no user-pointable source location, so a dummy span is
>   not stuffed in); six-language copy is in `locales/*.json`. **Emission level**: as of 4.3.2 it is
>   **error** (same code promoted to E, see paragraph three) — determined by explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444), the code number is still
>   in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, includes `--selftest`).
>
> **Two, Cell-by-cell landing of the three-tier table + two added tiers**
>
> | Three-tier Table Situation Above     | Implementation Criterion                                                                       | Code                   |
> | ------------------------------------ | ---------------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer | **Delivery marker present** (marker ↔ real read reconciled by static gate, see paragraph five) | obligations.rs:618-623 |
> | Obligation non-empty but no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see paragraph three)  | obligations.rs:624-630 |
> | Obligation empty                     | `Obligation::is_produced` checks field-by-field empty → pass                                   | obligations.rs:587-591 |
>
> Two **added tiers** (2026-10-13 user ruling, sketch did not include):
>
> 1. **S1**: the obligation's consumer stage is not in this `ProgramKind`'s stage table → **not
>    judged in arrears** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: consumer stage skipped with `SkipReason::UpstreamFailed` / `ConfigDisabled`
>    → not judged in arrears (obligations.rs:598-602). `NoObligations` is **not** exempted—if a
>    stage says it has no obligations while the obligations are non-empty, the ledger is unbalanced
>    and must be reported.
>
> **Three, External emission gauge (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism emits only **W level** (W1090); **no strict
> switch**. **4.3.2 (terminal state, 2026-10-13 user ruling A)**: **same code promoted to E**—the
> `W1090` code number is unchanged, the emission level is lifted to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code added** (two
> codes for the same fact is parallel representation). **The upgrade only changes the emission
> level, not the emission condition.** **This is a D-class breaking item of D59 (changing externally
> observable behavior), truthfully registered**: a healthy program emits zero (no emission when all
> obligations have delivery markers), so existing corpus and user programs are zero-impact (342/342
> "diff: empty"); but once a run actually hits an undelivered obligation, the diagnostic **counts
> toward `error_count`, renders as `error[W1090]`, and the exit code is non-zero**—different from
> the previous "W level does not block compilation". The decoupling of code family and emission
> level has been registered in the "Emission Channel" section of RFC-013 (the code number prefix
> only determines the numbering category, the level is determined by the explicit severity at the
> construction point). Mechanism fact (measured): severity takes the explicit `.severity()`;
> otherwise "code starts with `W` → Warning" (`codes/builder.rs:186-203`); the `ErrorCategory` in
> the registry **does not participate** in severity; exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`), **W1090 takes `error_count` due to explicit error (as of 4.3.2)**.
>
> **Red list fields do not enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> current red list is zero, see paragraph four), for a reproducible reason:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is injected from the
>   typecheck pre-prepared table, **always non-empty**, and what is emitted per compilation is a
>   **constant signal** (no discrimination), but it would break three existing external
>   contracts—"clean program zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing test cases turned red**, of which **8
>   in out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 in
>   `src/driver/tests/`), the corpus baseline 24/342 fixtures changed, and the only feasible fix is
>   "filter W1090 from zero-diagnostic assertions" = relaxing existing criteria.
> - Ruling B (downgrade to Info level) would still break 9/12 of those list-style assertions, and
>   Info is not "emitted at the ruling level".
> - Ruling C measured result: existing test cases **zero changes**, corpus baseline **zero change**
>   (342/342 "diff: empty, behavior equivalent"), runtime W1090 becomes a **discriminating
>   regression detector**—the healthy tree is silent throughout, and only fires when a delivery
>   marker is removed (three delivery points each underwent marker-removal mutations, CLI `check`
>   printed 2 `warning [W1090]` when the Check arm's marker was removed).
>
> The "no silence" of the red list is borne by three places: ① the audit surface
> `DriverOutcome.obligations.findings` (carried every run, with WBS id and evidence pointers); ②
> `check-obligations.py` prints the red list every run (currently empty) + staleness check; ③ this
> note.
>
> **Four, Current state of the red list (cleared and closed) and landing records of 3.4.4 / 3.4.5 /
> 3.4.9**
>
> Red list 3 → **0**—the three clearing tasks were all landed by 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is equivalently fulfilled by in-situ pattern detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST pattern
>   `FieldAccess{Call(Var)/Var}` + takes declaration order from `sum_type_variants`), the call point
>   `:7496` swaps the entire Call for `CreateVariant`; the function's doc explicitly states "the
>   span key across systems is unreliable (the two sides have different semantics for the Call node
>   span), so detect in situ"—**wiring up a consumer would bring back the already-fixed span
>   mismatch defect**. Deletion surface: `types.rs` field, collection at
>   `checker.rs:1489-1493/:1565`, collection chains in `inference/expressions.rs` and
>   `inference/statements.rs`, `VariantCtorCall` struct in `environment.rs`; type inference in
>   `infer_variant_ctor_call` and all diagnostics **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (kept in-phase candidate name
>   table and `overload_resolutions`). All read/write points: written in phase
>   (`environment.rs:205-219` registered in the same order as `method_overloads` → `checker.rs:972`
>   → `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase (decision
>   point at `expressions.rs:4490-4550`, the unique hit writes to `overload_resolutions` at
>   `:4550`); what was deleted is the **zero-reader mirror** poured into `TypeCheckResult` at
>   `checker.rs:1558`. The naming fact is still expressed at a single point: candidate name table
>   (in phase) + span-keyed decision (cross-stage, `ir_gen.rs:380` constructs, `:7632/:7705`
>   consume); the definition-side hybrid name is self-calculated by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user ruling A, see the
>   ruling record in the §Obligation Fields table of this document). What was deleted is the
>   **declaration** in the doc comment that said "mirrored for middle-side/LSP consumption"
>   (types.rs:47-48); the field and its in-phase consumption are kept (`operator_interfaces.rs:623`
>   operator query, seven constraint-solving queries in `inference/expressions.rs`, read-only
>   accessor at `checker.rs:214-217`); cross-stage zero reads directly proven by
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why no consumer is wired up**:
>   the content of this table is the environment mirror injected from the typecheck pre-prepared
>   table (always non-empty); if LSP truly needs it, register it fresh per 4.6.x; creating a
>   consumer for a field that "by design does not require cross-stage consumption" would only
>   introduce fake obligations into the ledger.
>
> After the red list is zeroed, **the field census is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reason)"**—gate criterion 1's reverse
> containment holds this end-to-end (add field and forget to register / delete field and forget to
> sync all turn red immediately). The red list **mechanism** is retained: gate criterion 4's
> staleness check and the `stale-red-list` fixture of `--selftest` continue to cover its detection
> surface; future re-registration must go through an explicit ruling.
>
> The gauge synchronized with the clearing: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / the §Obligation Fields table in this document); gate `--selftest` now covers
> seven types of fixtures (newly added: "unregistered field", "stale anchor", and "red list empty";
> see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`).
>
> **Five, The Actual Criteria of the Static Gate (9.2)** (three more hard criteria than the one
> sentence in the sketch)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor check (F1)**: `INVENTORY` of
>    `obligations.rs` ↔ `TypeCheckResult` field definition ↔ the "§Obligation Fields" table of this
>    document (including the count declaration in the table header), any drift turns red; and the
>    field set of `TypeCheckResult` must **equal exactly** "obligation list ∪ non-obligation
>    registration (`NON_OBLIGATION_FIELDS`, with reason)"— add field and forget to register, delete
>    field and forget to sync, non-obligation registration without reason, two-table overlap all
>    turn red immediately;
> 2. **Read-out evidence**: non-red-list obligations must have a real read in the **declared
>    consumer file** (`.field_name` member access; comments / tests / strings don't count)—this
>    makes "delivery marker ↔ downstream real read" a hard criterion (the main risk of mechanism A
>    is exactly the drift between the two);
> 3. **Delivery marker**: non-red-list obligations must be marked at the delivery point; red-list
>    ones must not be marked;
> 4. **Red list staleness check**: red-list fields must have zero real reads (a consumer growing on
>    it turns red), and each entry must carry a WBS id and an evidence pointer; plus `--selftest`:
>    comes with counter-example / positive-example fixtures, counter-examples must be red,
>    positive-examples must be green (proving the detector doesn't idle).
>
> **Six, Cell-by-cell deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy by field**: the obligation
>    data is still held by `TypeCheckResult`, and the ledger only records "list + delivery marker +
>    settlement conclusion". Reason: copying would create two sources of fact (each field is
>    produced from typecheck's checker and ownership layer respectively), which will inevitably
>    drift; the cross-layer data surface also should not be copied to the orchestration layer.
> 2. The sketch's `take()` / "marked consumed" was **not adopted**; instead `is_produced` emptiness
>    check + `DeliveryLedger` delivery marker is used. Reason: obligation and product are the same
>    data; `take()` would take away what the downstream needs; and 11 IrGeneration obligations are
>    of the form "the entire `&TypeCheckResult` is handed to ir_gen", which cannot be `take()`-d
>    field by field.
> 3. The sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written to `DriverOutcome.obligations`,
>    and the diagnostic is split by `ProgramKind`: Check/Lsp goes through the per-file collection
>    channel; Other forms are split by **level** (W1090 is E level as of 4.3.2)—single-file goes to
>    `PipelineError::TypeCheck` (obligations.rs:654 emission arm), multi-file/embedded additionally
>    records `OrchestratorError` failures (otherwise `result.errors` has no consumer). Reason:
>    returning `Vec<Diagnostic>` would lose form splitting—check diagnostics must belong to per
>    file.
> 4. The first-tier criterion of the three-tier table fell from "obligation read by declared
>    consumer" to "delivery marker present"; "marker ↔ real read" is left to the static gate
>    (paragraph five, criterion 2).
> 5. The "strict mode upgrade to E" of the second tier **does not go through a strict switch**, but
>    is replaced by **4.3.2 same-code promotion to E** (no strict switch, promoted, because the
>    healthy state emits zero, which is already safe enough, R4).
> 6. Static gate criteria upgrade from "field appears ≥2 times but no third file reads" to the four
>    in paragraph five.
> 7. The two added tiers (S1, skip exemption) not in the sketch are in paragraph two.
> 8. The single-test name `test_obligations_drained` mentioned in the S1/S4 table: actually landed
>    as **20 test cases** in `src/driver/tests/obligations.rs` (skeleton with `#[ignore]` first →
>    turned green; red state preserved: removing the unique settlement point results in `pairs: 0`,
>    6 failures). The two lines in S1/S4 are synchronized to the actual file.
>
> **Seven, 4.3.2 Landing Record (level setting + gate reinforcement)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the kinds of obligations
>   actually drained in this run, available per run) and the five counts (conservation equation of
>   paragraph two); combined with audit surface `findings` you can give the fact of "whether this
>   field is consumed by any entry".
> - **Level: same-code promotion to E has landed** (2026-10-13 user ruling A; see paragraph three's
>   terminal state registration and D59 D-class breaking item declaration). Implementation: explicit
>   `.severity(Severity::Error)` in `to_diagnostic`; emission arm splits the error channel by level.
>   Mutation evidence: removing any delivery marker → that form's compilation fails, `check` exit
>   code non-zero (`error [W1090]`); after restoration, healthy state emits zero (corpus 342/342
>   zero change). locale copy is written in six languages as "W family code, emission level is
>   error".
> - **Gate reinforcement items (F1/F2, all landed)**:
>   - **F1/R2 anchor check**: the gate now parses the **non-obligation registration reasons and red
>     list evidence**'s `file:line` anchors, checks "file locatable (uniquely) + line number on
>     record + the symbol immediately following the anchor actually appears in the anchored file";
>     an anchor pointing to a non-existent symbol turns red. Comes with a counter-example fixture
>     `tests/baselines/obligations-gate/stale-anchor/` (symbol doesn't exist / line number out of
>     range, one each). Note: the red list is currently empty, so the actual landing point of this
>     criterion is the **non-obligation registration** and future evidence pointers (same code
>     path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference", a non-empty difference turns red;
>     comes with counter-example fixture `unregistered-field/` (a new field in the struct is not
>     registered).
> - **Info item F3**: `SkipReason::UpstreamFailed` exemption is unreachable on real entries (the
>   unit that fails typecheck does not retain `type_result`, the obligation does not exist in the
>   first place); this tier's semantics is nailed by synthetic `State` test cases, the real
>   end-to-end only covers `ConfigDisabled` (t3 T06).
>
> **Eight, 4.3 Secondary Task Closure (red list zeroed + obligation diagnostic terminal E level)**
>
> **8.1 Three Rounds of Landing Timeline** (corresponding to code file:line item by item; this round
> only supplements the conclusion, no code changes)
>
> | Round                  | Conclusion                                                                                                                                                                            | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
> | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger goes live | `State::assert_drained()` unique settlement point; 13-entry obligation list + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                      | Call point `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                          |
> | 3.4.4 Clearing         | `variant_ctor_calls` delete field—the declared responsibility is equivalently fulfilled by in-situ pattern detection on the IR side                                                   | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call point)                                                                                                                                                                                                                                                                                                                                                                                                      |
> | 3.4.5 Clearing         | `method_overload_ir_names` delete cross-stage mirror (kept in-phase candidate name table + `overload_resolutions`)                                                                    | Decision consumption point `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                    |
> | 3.4.9 Clearing         | `interface_impl_registry` reclassified as **in-tree fact**: deleted "mirrored for middle/LSP consumption" declaration, kept in-phase consumption; obligations 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is in the same file `:47-51`, ruling A rewrote it); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven places in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level setting    | **Same code promoted to E**: explicit `.severity(Severity::Error)`, code number remains `W1090`; emission splits the error channel by level (first failure priority)                  | `src/driver/obligations.rs:450` (severity), `:676-699` (E level branch); code-family-and-level decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                       |
>
> **8.2 Closure Conclusion**
>
> - **Red list zeroed**: the number of entries with `red_list: Some` in `INVENTORY`
>   (`src/driver/obligations.rs:201`) = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism **is
>   retained**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's staleness
>   check and `stale-red-list` fixture; gate output "red list 0 entries—cleared to zero".
> - **Obligation diagnostic terminal E level**: the **code number and W-family affiliation of
>   `W1090` is unchanged**, emission level is `error` (D5 terminal state, 2026-10-13 user ruling A).
>   This **is a D-class breaking item of D59** (changing externally observable behavior), registered
>   in both paragraph three of this note and the "Emission Channel" section of RFC-013; **healthy
>   state emits zero ⇒ existing corpus and user programs zero impact** (corpus 342/342 "diff:
>   empty", IR snapshot zero drift; t11 independent verification and t12 re-review each
>   independently rechecked).
> - **Field census closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption surface
>   reason); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No unlanded commitments remain**: all the reinforcement items registered in this note **have
>   been landed** (F1/F2/F4/F5, see paragraphs five, seven, eight); the only two unclosed items are
>   the **known boundaries** of 8.4 and the F5 already transferred to an issue at t7, neither of
>   which are unlanded criteria for this mechanism.
>
> **8.3 Disposition Status of Each Round's Acceptance / Re-review Findings** (marked item by item)
>
> | Source        | Finding                                                                                                                                                                     | Status                                                                                                                                                                                                                                                                          |
> | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, silently stale when the substitute consumer is removed                                                                             | **Landed**: gate F1 anchor check (file locatable + line number on record + symbol anchor, `scripts/ci/check-obligations.py:251`); counter-example fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                      |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, silently PASS when a new field is added without registration                                                                    | **Landed**: F2 difference explicit output (`:577`) + F4 definition line reconciliation (`:455`) + F5 range reconciliation; counter-example fixture `unregistered-field/`                                                                                                        |
> | t4 R4         | W-only and promotion-to-E gauge not registered                                                                                                                              | **Landed**: paragraphs three / seven of this note + RFC-013 "Emission Channel" + W1090 help and error_output examples in six-language `locales/*.json`                                                                                                                          |
> | t7 F5         | Dead code analysis miscounts overloaded method references (legal overloads still falsely report `W1005`)                                                                    | **Transferred to issue `#450`**, belongs to WBS **4.9 Reference Pool Correct Implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`)—not this secondary task                                                                                       |
> | t11 F7        | In `--root` mode of the gate, F1 does not cover anchor commitments in module-header prose comments                                                                          | **Known boundary** (see 8.4)                                                                                                                                                                                                                                                    |
> | t11 F8        | Validator's self-recorded tool side: mutation driver once mistakenly wrote to the main repo (cleared by `git restore` in time, affected rounds' output discarded and rerun) | **Info item** (not a deliverable defect)                                                                                                                                                                                                                                        |
> | t11 F9        | Main repo intermediate state (unstaged files made this document's table header out of sync with `INVENTORY`) triggered gate red                                             | **Info item / resolved**: freeze-commit self-consistent PASS; "intermediate state committed directly will red the gate" is exactly the design intent of F2/F4                                                                                                                   |
> | t12 F12-1     | The struct range in this document is written as `16-73`, actually `16-76` (after t9 added 3 lines of comments, no mechanical reconciliation)                                | **Landed + mechanized**: range corrected to `16-76`, and gate **F5** added (range anchor, wrong = red); similar numbers grep-aligned in one go (`17 → 20 test cases`, `16 span-keyed fields` → 13)                                                                              |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                                   | **Fixed** (captain has amended the commit message to explain): `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by **existing nail-board** test case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known Boundaries (registrations, not unlanded commitments)**
>
> - **Scan surface of F1**: anchor check only covers the **on-record entries**' text—non-obligation
>   registration reasons (`NON_OBLIGATION_FIELDS`) and red list evidence; **`file:line` commitments
>   in module-header prose comments are not in the scan surface** (currently no on-record entries
>   are affected). If anchor commitments are written in module headers in the future, it must be
>   **borne by manual review**, or the scan surface of F1 must be expanded—take one of the two, do
>   not let it default to unmanaged.
> - **Static boundary of the gate**: the set-level check of delivery markers is on the static side
>   (criterion 3), the single-arm removal is supplemented by **runtime settlement**— the static +
>   runtime double layer is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   verification).
>
> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: the **two span-keyed contracts** in the 13 obligations above (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note registers the landing form, criteria, classification basis, and known boundary;
> the definition line is still reconciled by gate **F4** (the line numbers in the two rows of the
> table above are the current code fact).
>
> **One, Landing Form (carrier choice A: `PlanId`)**
>
> The key is a **structural identity** that follows the AST node, no longer relying on "the two
> sides each compute a span" positional coincidence: `PlanId` (`src/util/plan_id.rs`, `u32`
> monotonic, unique within the compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside the parser) is issued by the **unique allocator** `ParserState::alloc_plan_id`. Carriers
> are two: `Stmt.plan_id` (statement identity → affiliation and consumption of release plan) and
> `Expr::Call.plan_id` (call-site identity → production and consumption of overload decision);
> `Expr::call` is the unique call expression constructor in the whole repo, and synthetic calls
> outside the parser are uniformly recorded as `UNPLANNED` (**cannot be consumed**, see paragraph
> two).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the unique
>   consumption entry is `ReleasePlan::consume(stmt)`—**read and bookkeeping in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (statement identity is given by `walk_stmt`'s set/restore +
>   CFG node→statement identity table; the Drop of scope exit belongs to the last statement of this
>   scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in the same body**),
>   production point `inference/expressions.rs:4550` (writes by call-site `PlanId`, dispatch is
>   transparently passed via `Expr::Call { plan_id }` destructuring), consumption point
>   `ir_gen.rs:7697` (with binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit
>   immediately books).
>
> **Two, Consumption-Must-Reach Settlement (reuses the 4.3 ledger, does not create a second set)**:
> the consumption ledger is a **single implementation** `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), each of the two tables holds an instance; when `generate_module_ir`
> returns successfully, it **at the same point** marks "completed one full pass" for both ledgers
> (`ir_gen.rs:1095` release plan / `:1097` overload decision); the three-state mapping is unique
> (`consumption_state` of `src/driver/obligations.rs`), the unique settlement point connects one
> line to each of the two obligations—a non-empty difference is **E level W1090** (same-code
> promoted to E, gauge see paragraph three). The exemption criterion hangs on the **completion
> marker of the ledger itself** (not on whether `unit.ir` exists), so the Check form taking away the
> IR product does not constitute an exemption.
>
> **Three, D59 Classification Basis (Class C, red before green)**: the form
> `Type.method(positional args, name = value)` was **silently giving the wrong answer** before the
> change (measured: `r: Int = t.pick(1, a = 2)` compiled cleanly, ran with output `1` instead of
> `20`—the decision-resolved candidate was discarded, falling back to the last binding in
> declaration order); after the change, the same program at compile time
> `error [W1090] ... produced key not consumed`. The fix takes the **loud** approach rather than
> "stuff the decision name into a call with named arguments": the latter would lose named arguments,
> equivalent to another kind of silence (terminal semantics see [09](./09-execution-wbs.md) §3.14 /
> issue #452). Also: `Drop` instructions are no-op in the interpreter
> (`src/backends/interpreter/executor/ops/control.rs` wildcard arm only `advance()`), so the key
> migration of the release plan doesn't change runtime behavior—the corpus 342 differential empty is
> empirical evidence of this.
>
> **Four, Criteria and Evidence**: D41 same-type criterion `test_release_plan_spans_consumed`
> (settles by `PlanId`) and newly added `test_overload_resolutions_consumed` (per-file
> lex→parse→typecheck→ir_gen for corpus, **difference set must be empty + evidence floor**: files
> with produced decisions ≥ 1, items actually consumed ≥ 2, no whitelist); decision form coverage 9
> items of nail-board see `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected
> return / positional args / type-body binding / `?` operand / pure named / positional+named /
> ambiguous / variant ctor / unallocated identity). Mutation driver: consumption key offset by one →
> criterion red + real program `error [W1090]` + corpus gate difference; green again after
> restoration. Gate values: `check-obligations.py` PASS, corpus 342 differential empty,
> `cargo test --lib` 2712 passed / 0 failed (IR snapshot zero drift).
>
> **Five, Known Boundary (registration, not unlanded commitment)**: the **unbound consumption
> point** at `ir_gen.rs:7766-7790` is **unreachable** in the current corpus and test surface (method
> declarations uniformly go through `register_type_bindings` to the bound consumption point), so the
> keying correctness of this branch **has no red state to falsify**; the reason for retention is =
> it is the symmetric branch of the bound path, removing it would rewrite the "method call without
> `type_bindings`" form into silence. Registered by independent verification (t5 F-1) and re-review
> (t6 F2), **no new tasks are opened**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs's compile_playground — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only be one of the 6 predefined combinations from `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be the 6 predefined combinations above (one per `ProgramKind`), and does not
accept caller-passed free arrays.** This is the key constraint that prevents the `Program`
abstraction from degenerating into "a free parameter that takes anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must emit diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // ...12 arms, not one can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Unique settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: four implementation deviations
> from the sketch above, semantically equivalent, registered for reference—
>
> 1. §4's `StageOutcome` three-state is not returned by the stage arm, but is expressed by `State`
>    failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged, saves each
>    arm's boilerplate return);
> 2. `Driver` has no `config` field—config is uniquely sourced from `Program.config` (§5 "Avoid
>    Driver holding mutable global state" ruling covers sketch field);
> 3. `Skipped` diagnostics are only recorded in `DriverOutcome.skipped` internal ledger in 4.1.3,
>    not externally emitted—S2 "deliberately fix no bugs" zero-diff criterion requirement; external
>    emission follows the 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is the allowed direction); orchestrator existing call points will be relocated
>    and reconsidered after 4.2 migration.
> 5. In multi-file form, ProofExecution is an independent stage that executes **after all**
>    typecheck (A1, 2026-10-09 user ruling): `compile_project` originally interspersed proof in the
>    per-file typecheck loop (file N's proof before file N+1's typecheck). Single failure, both are
>    byte-identical; the "file 1 proof failure + file 2 typecheck failure" multiple-failure
>    scenario, the first report changes from proof error to typecheck error (test nail-board).
> 6. `DriverOutcome` carries products by ProgramKind in separate channels: `result` (pipeline
>    contract) / `module` + `failure` (orchestrator contract, 4.2.1)—each entry's external error
>    contract (PipelineError / OrchestratorError) is not a type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files are empty Vec—`check_project` current
>    contract); Discovery product extended to "file set + used_by edge set + shadow events" triplet
>    into State (MultiFile form has no downstream consumer, only records, no external emission);
>    check path's per-file parse drops from 2 times to 1 time with C3 topology.
> 8. Lsp form landing (4.2.3, split by file source ruling, 2026-10-09 user decision): on-disk file
>    parse failure same as Check's Plan B (collection + exit compilation unit—two-side degradation
>    convergence); edited buffer parse failure retains a residual AST to continue typecheck (editor
>    philosophy—intermediate typing state should not make semantic features disappear). Discovery
>    arm for Lsp reserves buffer source code to cover on-disk stale content; Typecheck under Lsp
>    only checks the target file (other units are only for registry to provide signatures); in-file
>    diagnostic order normalization principle extends to LSP (typecheck diagnostics → W-code
>    warnings → proof errors). The old behavior's LSP hard-aborting on irrelevant on-disk file parse
>    errors, handler silently falling back to the single-file path, is fixed by this step.
> 9. Embedded form landing (4.2.4): `Program` adds a new `shared_registry` field—embedded std
>    modules are sub-compilations, the registry is an **input** rather than a product
>    (EMBEDDED_STAGES has no Registry stage), #94's SymbolTable sharing contract is thereby made
>    explicit as a program declaration from "caller remembers to pass the same registry". Error path
>    text is normalized from `<std.test> (embedded std)` to the unit virtual path `<std/test>` (only
>    compiler internal error surface, no test nailed). At this point, all four orchestrator entries
>    are migrated to Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): Check form
>     faithfully carries the single-file semantics for programs without a project root—warning
>     surface only covers the entry file (adjacent files only collect errors, rather miss than
>     misfire); relative `use` resolved along importer directory (rustc single-file mod aligned).
>     CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full
>     chain) already runs IR generation, E3019/E1014/E1015 are only produced in ir_gen (runner gate
>     empirical). IrGeneration's Check form pure check doesn't consume IR; Script/Bin bifurcation
>     expressed with module_key's None/Some (E3023 existing switch, ir_gen.rs:1660).
>     Monomorphization does not enter CHECK (only produces resource overage / internal errors, zero
>     corpus dependency). **〔2026-10-10 empirically withdrawn via 3.4.8〕**—"zero corpus
>     dependency" is actually evidence of a blind spot with no pathological recursive fixture in the
>     corpus; E3005 is the only compile-time defense for this class of programs. See note #14.
>     Latent defect fixed along the way: Registry arm's repeated harvesting of registration for
>     embedded std unit would override the native half already merged by `with_std()` (E1043 false
>     report)—the embedded unit now skips the duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, rerouted to
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains a residual AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; not marking stage failure,
>     otherwise topology skipping keeps typecheck from ever running); Typecheck dispatches the
>     `check_module_collect_all` free function. LSP single-file path thereby fills in proof (E4018),
>     W-code warnings (W1001–W1005, test nail-board) and IR-level errors; lexical failure now
>     reports real diagnostics (old synthetic "E0001 lexical error" text disappears with the
>     sequence deletion—real diagnostics carry precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 must-do
>     item): the two entries have long since converged individually—mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain both run with the difference carried by whether
>     collected_errors is empty; statements.rs five collection points (function body / use / for /
>     block / while). The verification confirmed and fixed a duplicate-diagnostic defect: the
>     collection point puts the first error into both collected_errors and the Err return channel,
>     pass-3's add_error to Err causes the same code/span/message ×2 (nested can be ×3); there are
>     also two independent mechanisms: annotation validation signature parameter and overall
>     annotation double visit (E1003/E1103), ownership layer same-point double emission
>     (E2014/E2018). The fix lands at the module result boundary: deduplicate by (code, span,
>     message)—diagnostics are positional facts, duplication carries no information; 63 corpus
>     entries have duplicate copies removed, run column and exit code zero drift
>     (`borrow_conflict_err`'s "duplicates" is a baseline triple-format artifact without column
>     number—two E2018 are actually legal diagnostics at different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper layer no longer
>     participates. Error text byte-aligned with old `CompileError` Display prefix (Parse
>     error:/Type error:/Internal error:), playground UI zero visible difference. WasmPlayground and
>     SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step has real
>     producers from now on).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial withdrawal of note #10's mono exclusion**). Empirical evidence overturns "mono only
>     produces resource-limit noise": pathological generic recursion (`f(x)=f([x])`) causes the
>     compiler process to stack overflow on the multi-file path without mono (0xc00000fd—the
>     interpreter recurses at the Rust layer, no graceful runtime error), the mono depth gate is the
>     only compile-time defense; the standalone check before 4.2.5 took the pipeline full chain and
>     had mono, the exclusion ruling caused a silent coverage regression ("zero corpus dependency"
>     only proves the corpus has no pathological fixture). The "type erasure fallback" assumption is
>     voided by RFC-033 reflection ruling—`^^List(Int)` needs the real identity of instantiation,
>     and monomorphization is the foundation of reflection rather than an optional optimization.
>     Four design points: ① typecheck produces instantiation_request for qualified calls
>     (`lib.f(x)`, FieldAccess form), generic_id uses qualified name, in the same namespace as
>     merged IR (currently only recognizes bare Var, expressions.rs:2455); ② `Stage::ALL` topology
>     changed to Linking before Monomorphization, MULTI_FILE stage table tail adds
>     Monomorphization—consumes merged_ir, aggregates all-unit requests (containing_fn qualification
>     in the deferred bucket), mono upgraded from single-module pass to whole-program pass;
>     single-file stage table unchanged (no Linking, subsequence property preserved, IR snapshot
>     zero drift); ③ CHECK adds Linking (pure merge—E3020 entry validation left to
>     RoleClassification, to prevent double reporting) and Monomorphization (pure check no consume,
>     same as IrGeneration's Check form) two stages; ④ resource protection covers cross-unit mutual
>     recursion with whole-program BFS. The Monomorphizer body itself is unchanged (the contract
>     consuming ModuleIR + request set is exactly the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Implements qualified call instantiation
>     collection per #14 design: `ExpressionInferrer` injects module qualified key table (`use`
>     three-arm registration—normal / alias / per-item alias and only SubModule kind export),
>     `callee_generic_name` is concatenated with `SymbolTable::qualify` to form a qualified name
>     with the same source as merged IR; qualified call first-path arity criterion tightened (when
>     the parameter table cannot be resolved, fall back to second path to take arguments by
>     signature, to avoid cross-module same-name function mismatch false report E3018). During
>     implementation, three **asymmetry defects** on the mono side were empirically confirmed and
>     fixed in the same round: (1) The granularity mismatch between deletion key (generic name set)
>     and rewrite key (containing_fn + name + span triple)—call sites that cannot resolve arguments
>     degenerate to symbol requests into the deferred bucket, the container function when
>     non-generic never drains the bucket, but the original has been deleted by name → dangling call
>     (`list_ops.yx:53` runtime E6006 empirical); fixed by restoring the gate
>     `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as the single source
>     of fact to determine coverage, the original is preserved for uncovered call sites. (2) The
>     recovery section's `HashSet` iteration order caused the function table order to be
>     indeterminate (t2 review F1) → changed to `Vec` and sorted by name, the product is byte-level
>     reproducible. (3) The non-rewrite surface forms TailCall / MakeClosure originally queried the
>     map key as usual (the comment claimed conservative) → `call_form_is_rewritten` single-point
>     judgment, non-rewrite forms all count as uncovered. Each comes with a nail test (red state
>     empirical discriminatory power before fix). Corpus differential baseline regenerated with the
>     same commit has exactly one line (`remove_at` frame supplemented with `(int64)`). **Submission
>     strategy correction**: when the typecheck collection and mono fix are split into two commits,
>     the staging tree containing only the former triggers `stdlib_docs`'s red on HEAD's existing
>     defect (`list.is_empty([])` same-family E6006)—the two halves are each other's green-light
>     prerequisite, so they are merged into one atomic commit.
> 16. 2026-10-10 Five Rulings Registration (after R1 closure): (1) **D2 ruling A**—user module
>     export carries `generic_fn_type_params_snapshot()` along with `Export.type_params`, isomorphic
>     to the std path (`yx_sources.rs:62/108`): the declaration name becomes the authoritative
>     source for qualified call monomorphization, signature reverse-inference is downgraded to
>     fallback (incorporated into R2 scope). (2) **HKT declaration side alive, call side broken**
>     (inferred type argument E1002 / explicit type argument E1010) registered as WBS 3.5 (3
>     tertiary tasks). (3) **deferred bucket under non-generic container never drains** incorporated
>     into 3.4.8 ② acceptance sub-item: the bucket must drain or loudly report an error, silent
>     non-specialization is not allowed. (4) **Closure variable called by name runtime E6006**
>     registered as WBS 3.6; semantics (i) support indirect call / (ii) compile-time refusal,
>     **ruled (i)**—support indirect call: `CallDyn` mechanism already exists (`ir_gen.rs:7945`
>     closure expression path), what was broken was `Var` callee being downgraded to by-name `Call`,
>     and typecheck originally accepted this program. (5) **Trade-off criteria solidification**—the
>     only basis for rejecting a plan is correctness and readability, "large change surface / new
>     mechanism / high cost" is not a reason for rejection, put in `coding-rules.md` part two and
>     the HOWTO self-check list; trade-offs needing user decision are given all four items per
>     `AGENTS.md` "When requesting user ruling (mandatory)" (situation + evidence / per-item pros
>     and cons / clear recommendation + cost / breaking impact itemization).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm aggregates the whole-program request **before taking each unit's IR** and
>     qualifies `containing_fn` (otherwise the deferred bucket key and merged IR have different
>     sources, the bucket never drains). **① closure (D2 ruling A)**: user module export carries the
>     declaration-time type parameter snapshot along with `Export::type_params`, isomorphic to the
>     std path (`yx_sources.rs:62/108`). **Existing defects empirically confirmed and fixed in the
>     process**: (a) **F1**—`qualify_names` only rewrites the function body, `ir.init` is never
>     visited, multi-file function definitions are qualified but module top-level calls remain bare
>     name → runtime E6006 (**reproducible when non-generic, unrelated to generics**); fixed by init
>     reusing the same `rename`/`aliases` and the same `rewrite_call_names`. (b) The mono-side
>     "whole-program unique suffix" heuristic and the missed silent `continue` → deleted the
>     heuristic, missed case changed to **loud E3018** (ambiguous silent mismatch is worse than
>     non-parse). **Regression caught and fixed by independent verification**: t6 found that after
>     named import `use lib.{pick}` the bare-name call was falsely rejected by E3018 (the IR
>     instruction name is the alias qualified name, the request name is the bare name, the
>     aggregation side only qualifies by unit key) → fixed by the aggregation side **directly
>     calling ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as a single pub
>     point, one producer one consumer), and covered with a matrix nail test for the two forms of
>     named import. **Records**: init section `DefId` blank empirically confirmed to be a
>     "consistency blank" rather than a living defect (codegen name fallback is parseable, four
>     types of init calls empirically pass), registered as WBS 3.7.4 to close the gauge fork of
>     "function body goes def, init relies on name".
> 18. Init section DefId gauge fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` as a single point (Call/TailCall string operand + MakeClosure),
>     function body and `ir.init` share the same implementation and the same `resolve` closure; 208
>     IR snapshots regenerated with only 309 pairs of `(def=_)`→`(def=dN)` change (other-lines 0;
>     whole-tree init section 0 remaining `(def=_)`). codegen's DefId main path and the two states
>     of name fallback have consistent translation results and execution values (four types of init
>     calls empirically tested), corpus baseline zero drift; the probe is flipped and strengthened
>     per the new gauge (identity + precise index + execution value, fallback init coverage causes
>     red). **Errata (2026-10-11)**: the above commit numbers are rewritten by
>     `git pull --rebase`—original b554497b/e60dcc87/b7ac7396 correspond to post-rebase
>     375789d2/d922a551/38bd4b9e; the pushed dev history takes the new numbers as standard (the old
>     numbers are local temporary numbers, not entered the remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④'s design basis see #14; ②'s
>     landing see #17, this item is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate request + `link_module_irs`), entry validation block left to `RoleClassification`;
>     `ir_generation_check` retains `us.ir` after generation (memory only); mono failure in Check
>     enters collection channel, in MultiFile retains `fail_with`; `into_outcome` first
>     `take_check_diagnostics` then judges failure (structural failure no longer clears collected
>     diagnostics). **Verification**: four rounds of independent verification (t3/t4/t10 + t5
>     cross-form matrix) and one round of re-review (t6) all pass; corpus differential 340/340
>     line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps found
>     and fixed in the same round**: F1 (rootless multi-file check's request name and IR definition
>     key from different sources → silent non-specialization, see landing note of WBS 3.4.8 row)—fix
>     is that the aggregation side precisely reverse-queries by the real definition key
>     (non-heuristic), the first version fix is that the aggregation side precisely reverse-queries
>     by the real definition key (`flat_defs`); **same-day errata**: this bridge has been deleted in
>     the regression fix, the root cause is solved by `module_key`'s one-job-two-use decoupling
>     (`script_mode`), the three sources are unified as `{unit key}.{name}`. The regression (CI doc
>     example gate 258 blocks 10 failures → 0 failures) location and fix see landing note of WBS
>     3.4.8 row. **Records (existing, not this round's regression)**: E3005 help text inconsistent
>     with resource protection semantics; multi-file E3005's span label misaligned (HEAD binary
>     reproducible); request collection surface's type parameter position dependency (same-surface
>     to 3.5).
>
> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two subsequent commits). **3.9.1 (execution layer guard reachable)**: interpreted
>     call changed to **explicit
>     stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` pushes a frame then
>     returns control to the outer `step_one`, the frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, writes back to the caller slot on return, multiple
>     call instructions continue running with `pending_calls` in the frame. Baseline measured each
>     layer of interpreted call consumes native stack **2544 B**, crash critical at
>     `count_down(394)` (Windows main thread 1 MiB); after the fix, 20 000 layers of recursion can
>     run on a 512 KiB stack thread (each layer ~0). The guard (`push_frame`) is thereby reachable
>     at any entry, any `max_stack_depth` value; `ExecutorError::StackOverflow` changed to carry the
>     real upper limit (original `util/diagnostic/mod.rs` always passed 0). **Original 3.11.2 (deep
>     stack backtrace truncation) was merged into this task by user ruling** (in the same file,
>     indivisible by file): the backtrace is changed to first 16 frames + count omission + last 8
>     frames, shallow stack frame-by-frame unchanged. **3.10.1 (no compile-time gate for native
>     recursion, #447)**: locate first then change—crash point is `MonoType::type_name` recursion
>     (**328 B/layer**, SP probe + per-operation critical point), the same path has derived `Clone`
>     (~400 B/layer), `substitute` inner deep copy, `type_name_hash` (164 B/layer),
>     `collect_generic_type_refs`; after four places changed to working list and removed the
>     per-layer one full request deep copy, `mono.max_depth=5000`'s pathological generic recursion
>     produces **E3005 (with real upper limit)**, no more stack overflow. **3.11.1 (task boundary
>     error code fidelity, #448)**: task result error side changed from value to **structured**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`),
>     cross-boundary restores `ExecutorError` by value; task-internal E6001/E6003/E6004/E6008 in
>     Standard mode and Embedded messages are character-identical. Two **pub API payload shape
>     changes** have been written with `BREAKING CHANGE` in the commit. **Submission strategy
>     correction**: 3.9.1 and 3.11.1 **are indivisible at the file
>     granularity**—`interpreter/executor/executor.rs` simultaneously hosts the
>     production/consumption points of the trampoline and the error channel, `TaskFailure` is
>     defined in `runtime/engine.rs`, only staging either side won't compile (pre-commit with
>     **staging subset** compilation, measured two refusals), so per the same gauge of #15 they are
>     merged into **one atomic commit**. **Known boundary and follow-up**: Drop derive (~75 B/layer)
>     ⇒ `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate is borne on
>     native recursion (each layer ~3 696 B, default 256 layers ≈ 924 KiB), currently zero call
>     points, production unreachable, future wiring must close the same family; Standard mode task
>     interpreter constructs with default `ExecutorConfig` ⇒ its E6004 upper limit is always 1024
>     (config propagation registered as WBS 3.12, user ruling B: separate team); test runtime budget
>     separately opened issue #449 (pathological fixture 76s, clippy dual target, commit runs full
>     amount), not in this RFC's construction surface. **Corpus directory layering (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split
>     into 23 category-divided subdirectories, 88 IR snapshots relocated along the mirror path;
>     corpus baseline regenerated with the same commit (342 lines, independent recount line-by-line
>     comparison: only `file` field and 2 new entries changed), 41 snapshots regenerated due to file
>     header path lengthening, line-by-line check **only span offset change** (zero difference after
>     offset-stripped normalization).
> 21. LSP semantic data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, in-project cross-file references carry `resolves_to`
>     pointing to the real definition— the old form was didOpen/didChange triggering **two passes**
>     of compilation: diagnostics went through the unified Driver pipeline, semantic data went
>     through server.rs's manual lex→parse→single-file typecheck (no module registry, in-project
>     `use` resolution failed, reference events not produced, cross-file jump/rename/reference
>     lookup silently fell through—D55-registered silent failure is closed by this task).
>     **Implementation**: (1) `Export` carries `definition_span` (`extract_module_info` harvests
>     from AST identifier nodes, same source and value as typecheck definition event's `def_span`;
>     native std always None), `ModuleInfo` carries `definition_file` (Registry arm fills by
>     DiscoveredFile); (2) when checker drains reference events, via `import_def_locations` it
>     redirects the cross-file import item's `resolves_to` from the use line to the real location of
>     the definition file (std import has no registration, keeps falling to use line); (3) checker
>     module identity changed from unified `"<module>"` to the unit file path (this value is only
>     consumed on the semantic data surface, zero diagnostic consumption—empirically
>     `env.module_name` all 4 consumption points); (4) `DriverOutcome` adds a new `semantic_db`
>     channel—CollectAll form (Lsp target file regardless of diagnostic success + SingleFile
>     fallback) merges in, Lsp arm tail injects non-target file's top-level export definition
>     surface from the registry (`external_definitions`, even unopened files can jump; type /
>     submodule exports not injected—typecheck side type registration does not produce definition
>     events, two sides keep consistent gaps); (5) orchestrator adds `analyze_source_in_project`
>     (`check_source_in_project` is converted to call and only takes the diagnostic half); (6) LSP
>     side `analysis.rs` single compilation entry replaces the manual pipeline (server.rs's
>     `update_semantic_db` deleted), World key unified as normalized file URI (`uri.rs`:
>     `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
>     differences normalized to the same key). **Side-fix (D59 shunt Class B—old behavior silently
>     does nothing, does not change the externally observable behavior of any previously successful
>     program)**: in-project file's cross-file jump/rename/reference lookup turned from silent
>     fall-through to hitting the real definition; lexical error time the old manual pipeline's
>     semantic information deletion changed to residual AST colorization retention (4.2.6 editor
>     philosophy extension); target file with type errors no longer has semantic data discarded
>     along with `type_result` (colorization / jump doesn't disappear in intermediate typing state).
>     **Nail-board**: driver-level 6 (target file path key / cross-file resolves_to+external
>     definition surface round-trip / std import fallback to use line / error file retention /
>     single-file fallback path key+error retention), LSP e2e 1 (red before green empirical—jump
>     returns None before fix), uri normalization 4.

**The transformation method for each of the ten entries (function-by-function specification)**:

| Entry                                                      | After Transformation                                                                                                                                                                                                  |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                        |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                               |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                         |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                           |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one call of Driver + filter results to target file                                                                                                                                                          |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                             |
| `lib::run_file`                                            | Unchanged (transitively calls `run_project` or `SingleFile`)                                                                                                                                                          |
| `lib::run_project`                                         | Unchanged (transitively calls Driver)                                                                                                                                                                                 |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project reroutes to `Program { kind: Lsp, aggregation: CollectAll }`; **delete the manual lex→parse→… sequence** of the single-file branch, change to call `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, uniformly goes to `Program { kind: Check }`                                                                                                  |

**The degree of duplication between `check_project` and `compile_project` (about 40% shared / 60%
fork)**. The shared part is: vendor consistency check, file discovery, `build_registry_from`,
`all_method_bindings`, parsing loop, per-file checker assembly, result handling. The forked parts
are items 2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton is exactly the
value range of `Driver`**—the forked 60% are all "different stage set" or "different aggregation
mode", and that is exactly what the two fields of `Program` can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design ruling: "upstream failure causes this stage to be skipped" is not a stage return
value.**

Reason: skip is determined by **topology**, not by the stage itself. If each stage were allowed to
return `Skipped`, the information of "why didn't I run" would be scattered across 12 stages and
could not be centrally audited. Change to `dispatch` to judge at the beginning of the loop by
dependency:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must emit a diagnostic.** This rule directly targets the hole this document wants to
fix—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip" that doesn't
emit any diagnostic, just because the obligation happens to be empty. After the rule is established,
any "Y didn't run because X didn't happen" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                        | Text Direction                                                                                                                  |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Upstream `Abort` causes                                       | "Stage Y did not execute because upstream stage X failed"                                                                       |
| Pre-obligation empty causes                                   | "Stage Y did not execute because no pending obligations (normal)"—**this item's severity = Info, not counted in warning count** |
| Condition not satisfied (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because config is not enabled"                                                                         |

The second category is the key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute the `warning_count` of `yaoxiang check` (which
`diagnostic/mod.rs:637-641` depends on for its non-blocking contract).

### 5. Diagnostic Aggregation Mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error stops immediately (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                             | Current State Correspondence |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)               | Exists                       |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`:493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Exists                       |

**`Aggregation` must be a field of `Program`, not a global setting of Driver**—LSP serves both
in-project files and single files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program can avoid Driver holding mutable global state.

**Note the semantic difference of the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`:493`) are not just a "early-exit or not"
difference—they are two different checker entries. After unification, the same implementation should
be driven by the `Aggregation` parameter, rather than retaining two functions (**this is an item
that this document needs additional verification, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add `driver/` module                        | **None**. Does not introduce new type representation, does not touch `MonoType` / `PolyType` / `ir::Type`                                            |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth set of type representations.** The convergence of the
three parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**The affiliation and layering of obligation types**: what is migrated into the ledger is the
**settlement responsibility**, not the type's affiliation module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the remaining field types stay
where they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`.
L3's consumers (such as `ir_gen` reading `release_plan`) continue to receive specific field types
via parameters, **must not `use crate::driver`**—this is consistent with the Driver's
non-reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on the interfaces of
`frontend` (L2) / `middle` (L3) / `backends` (L4); **L2/L3/L4 must not reverse
`use crate::driver`**. `Driver` appearing in `TypeChecker`'s import is considered a violation,
intercepted by the `scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Transformation                    | After Transformation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone no longer takes this pipeline—uniformly reroutes to `compile_project` (MULTI_FILE stage table), Script semantics is driven by rootless; this line now only describes `lib::run_file` / `Compiler::compile` library paths)                                                                                                                                                                                                     |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution has been plugged in P3; **dead code family warnings and W1003 / W1004 / W1005 / W1006 now enter the MultiFile `run` path through the same criterion**—the W-code set of multi-file `run` is unified with single-file `check` (the `dead_code_analysis` arm is dispatched by "non-pipeline channel", Check / Lsp / MultiFile share the same implementation, no second criterion; the FailFast branch of `typecheck_orchestrated` temporarily stores `result.warnings` and hands them to this arm for external emission) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already plugged in P3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| LSP (in-project)                    | No proof_execution                       | Already plugged in P3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Z3 not installed + single-file      | `predicate.rs:35` **panics**             | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Z3 not installed + multi-file       | Silently skipped                         | Same as above, unified                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |

**The only intentional behavior break** is the `.expect()` at `predicate.rs:35` being changed to
return a diagnostic. This aligns with RFC-027 §8 "not bound to a specific solver" positioning, and
is also the contract already declared at `backend.rs:60` ("caller should conservatively
degrade")—the current implementation contradicts its own contract documentation.

### Compiler Change List

**New Files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single-file 1 / multi-file N)                                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified Files**

| File                                               | Line Range                                      | Change                                                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                                        |
| `src/lib.rs`                                       | `140-147`                                       | `run_file` changed to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                               |
| `src/lib.rs`                                       | `154-167`                                       | `run_project` changed to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                         | `141-227`                                       | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                                             |
| `src/frontend/pipeline.rs`                         | `275-278`                                       | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                                       |
| `src/frontend/pipeline.rs`                         | `306-311`                                       | `run_proof_execution` **kept**, moved into `driver` as the implementation of `Stage::ProofExecution`                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `381-389`                                       | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                                         |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                        | `compile_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                       | `check_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `450`                                           | `check_source_in_project` reroutes to Driver                                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `1374`                                          | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                       | The `check_module` / `check_module_collect_all` two-choice of `typecheck_with_registry_in` changed to be driven by the `Aggregation` parameter                                                                                                                                                                                                                               |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                       | `run_diagnostics`'s manual stage sequence deleted, reroutes to Driver                                                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                       | `check_files_with_diagnostics` uniformly goes to `Program { kind: Check }`                                                                                                                                                                                                                                                                                                   |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                       | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                              |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                                | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                         | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                                        |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                     | `ProofResult::Unproven { .. } => {}` empty arm changed to emit diagnostic (**fix the second silent discard point**)                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                                               |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                          | Fix the comment that doesn't match `:1320`                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/layers/README.md`     | Full text                                       | Layer order table changed to **actual** execution order; delete "lower layer fails, upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access point; the three consumer points of `predicate.rs`/`checker.rs`/`ownership.rs` are rerouted to the shared port. The bare `&'static` reference form is infeasible (`dyn Solver` is not `Sync`), the original description is subject to D58 |

**Deleted**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; the manual lex/parse
sequence at `src/lsp/handlers/diagnostics.rs:161-227`.

**Unchanged**: the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` (the logic
itself is correct—**the problem is at the consumer, not at the producer**; changing the producer
would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branch
(wasm branch reachability determination goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                 | Compatibility                                                         | Disposition                                                                                                                                      |
| ------------------------------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Multi-file path adds proof_execution                   | **Breaking**: previously silently passed constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; staged release, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                 | **Breaking**: programs that compiled now produce warnings             | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code does not change                        |
| `assert_drained()` first run emits W-level diagnostics | **Breaking**: diagnostic set adds                                     | First W then E, S2/S4 split into two steps                                                                                                       |
| `predicate.rs` panic changed to diagnostic             | **Improvement**: no longer crashes                                    | None breaking                                                                                                                                    |
| `build` / `dump_bytecode` subcommands                  | **No impact**                                                         | These two paths don't go through proof_execution                                                                                                 |
| `TypeCheckResult` field migration to `Obligations`     | **Internal refactor**                                                 | If `pub` API surface has external dependencies, they need to be synchronized; in-repo consumers are listed in the change list                    |

---

## Implementation Points

This document corresponds to **P3 (fix correctness holes)** and **P4 (stage contract and unified
Driver)** in RFC-039's global stage sequence. S1-S5 below is the implementation order within this
document, **each step's acceptance criterion references the **C2 category** of the
[Equivalence Criteria document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The hole criterion must be written as failing first.

| Deliverable                                                                                                                                           | Acceptance                                                                                                           |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                         | **Must be red**. If unexpectedly green, the hole analysis of this document needs to be re-reviewed                   |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (D48 ruling—not mixed into single-file corpus tree; project fixture with `yaoxiang.toml`) | The diagnostic sets of single-file and multi-file corpus can be compared                                             |
| `src/driver/tests/obligations.rs` skeleton (with `#[ignore]` first)                                                                                   | Mark `#[ignore]`, turn green at S4 (red state preserved: removing the unique settlement point results in `pairs: 0`) |

**Rollback point**: no code changes, pure new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                                 |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals the expected array |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path's diagnostic set and exit code are byte-identical**                          |
| Full corpus (293 `.yx` files) differential               | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**    |

**This stage deliberately fixes no bugs**—it only moves existing behavior into Driver. The
acceptance criterion is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` restores to original.

### S3: Merge Four Orchestrator Entries + Fix Holes

| Deliverable                                                                                                                     | Acceptance (C2)                                                                        |
| ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | The four entries' diagnostic sets, after normalization by `Aggregation`, are identical |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project                   |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                             |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Hole fixed                                                                             |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must emit diagnostic in any mode         |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics when Z3 environment is absent                                         |

**Rollback point**: the hole fix and the structure merge **are two separate commits**. If the
structure merge has issues, you can only roll back the structure commit, keep the hole fix commit—at
which point `test_multifile_proof_obligation_not_dropped` stays green; the reverse (keep structure,
roll back fix) will turn red, which is an unacceptable intermediate state and is forbidden to merge.

### S4: Enable the Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                        |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 test cases turn green (implementation form see §2 revision note; count subject to that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                          |
| Obligation diagnostic W → E upgrade          | Manual review of each new E after full corpus differential                                                                        |

**Rollback point**: the severity of `assert_drained()` can be controlled by a config item, W/E
switching does not require code structure changes.

### S5: Proof Layer and wasm Cleanup

| Deliverable                                                                                                                                                                                                                                    | Acceptance                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                                         | Doc and `checker.rs` call points correspond item by item                                                                                                      |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter cache**—otherwise one timeout will be solidified across compilations through the process-level cache and pollute cargo test thread-shared state) | Production path `default_solver()` call points zero (only test retains); cache hit rate observable across three consumer points (counter landed in 89576fafd) |
| `checker.rs`'s `TypeChecker::ownership_inputs` criterion point (ownership block `:1296-1340`, emission point `:1531`) degradation adds warning                                                                                                 | No `body_checker` time has diagnostic                                                                                                                         |
| Reachability determination of the 10 wasm attributes of `orchestrator.rs` (4 positive + 6 negative)                                                                                                                                            | Conclusion goes to the "F2 Determination" subsection of `06-cleanup-inventory.md`, this document only registers the determination requirement                 |

**Note**: fixing the layer order will change the diagnostic set and may expose a large number of
previously-silent `Unproven`. **This item should go independently of S1-S4**, not be mixed in the
same PR as the entry merge.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                        | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **refuse runtime registration**                                                                                                                                                                                                                                                                                                                                                | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism the opcode table has already proven in RFC-039 routing table B. `StageScope` further turns "per module or project level" into a type fact, eliminating the call-count question that needs human reasoning                                                                             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used together with the **static** gate of `scripts/ci/check-obligations.py`; **terminal severity E level** (2026-10-13 user ruling A: same code promoted to E, code number unchanged; **implementation form see §2 revision note**: ledger does not copy field structure; red list—0 after clearing—mechanism retained, does not enter user-visible diagnostic stream) | What is fixed is a whole class of bugs, not one bug: at least 2 same-type latent issues have been verified ( `proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E split into two steps is to make every step's C2 criterion usable—now the obligation list has been cleared to 13, red list is zero, healthy state emits zero, so the terminal state directly takes E (see §2 revision note paragraphs seven, eight) |
| **Fix scope**            | Fix the **consumer** (orchestration layer), **don't touch** the six `Unproven` branches at `checker.rs:5164/5179/5306/5318/5420/5448` of the producer logic                                                                                                                                                                                                                                                                          | The problem is at the consumer, not the producer. Changing the producer would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                                                                       |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment commitment" and "code behavior"**. The "do not let silent pass be
  resurrected" at `checker.rs:5165` is a **comment-level contract**, redeemed by `assert_drained()`.
- **"Forgot to append to an array"**. Currently, each of five functions manually writes its own call
  sequence; after unification there is a single stage table, and missing a new stage will report an
  error at compile time.

### Unadopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—of the 11 inconsistencies, 7
  (the two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **do not involve type bridging**, they are configuration questions of "whether
  to run a certain analysis"; it also cannot express `Aggregation`, it can only move the
  `check_module` / `check_module_collect_all` two-choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, equivalent to
  changing "five functions each manually write calls" to "an array forgot to append", which is
  exactly the cause of the current 11 inconsistencies. This repo already has a same-form
  counter-example: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implementation.
- **Only fix bugs but don't change structure**—can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered across 5 functions, the next fork point will
  continue to grow from here. **It must be done first** (as part of S1/S3), because the structure
  transformation needs a known-red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for the bug at the wrong
  level: the bug is at the orchestration layer, and the detection must be at the orchestration
  layer.

## Known Limitations and Risks

- **The slimming of `orchestrator.rs` will make it harder to read in the short term**. After the
  139-line `compile_project` is split into "Program constructor + several driver arms", readers need
  to cross two files to understand the flow. This is the common cost of all "centralizing the
  wiring" refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously-silent issues"**. After the fix, a batch of "new errors" user reports may appear—they
  are real bugs that just weren't reported before. This must be clearly stated in the CHANGELOG.
- **The severity switch of `assert_drained()` needs per-field manual judgment**. For some of the
  obligation fields (such as `module_namespaces`), "no consumer" may be by design not requiring
  consumption and should not be reported. S4 needs to go through field by field, can't be
  one-size-fits-all.
- **The `Program` abstraction may be premature**. If some entry's stage set is unstable in the long
  term, `stages()` will degenerate into a "different array passed every call" free parameter, and
  the contract constraint will fall through. **The mitigation is `test_program_stage_coverage`
  asserting that `stages()` can only come from 6 predefined combinations.**
- **Criterion dependency**: the hole fix criteria of S1/S3 depend on the hole test of the
  [Equivalence Criteria document](07-equivalence-oracle.md) being red first. **If that document
  doesn't first establish the multi-file corpus layer, S1 cannot be accepted**—because the existing
  293 corpus files all take the single-file path, zero coverage for this class of defects. This
  document does not involve the IR verifier (`verify_loose`), that part of the prerequisite work is
  not in this document's scope.
- **Fixing the layer order will expose previously-silent `Unproven`** (`equivalence` is not in the
  pipeline at all, the actual order of `termination` and `ownership` is opposite to the declared
  one). This is a diagnostic set change risk, should go independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19`
  already states that `Solver` is only `Send` and not `Sync`, the cache of `Z3Backend` is `RefCell`;
  after changing to a cross-compilation-unit shared singleton, it needs to be confirmed that all
  access paths go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency of
  termination and ownership). It should be opened after the equivalence criteria stabilize.

> **The open questions originally listed in this section have all been ruled on.** The item-by-item
> decisions are in the [RFC-039 Rulings Registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document has no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document does not introduce a fourth set)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 gradation, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` hole criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "do not let
  silent pass be resurrected"
- `src/frontend/core/typecheck/layers/README.md` — Layer order call table and the real identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard-fail
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 clearing)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test's
  self-acknowledged "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus's self-acknowledged "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs`'s `compile_playground` (compiled via `run_code`) — The fourth compilation caller
  of the playground (single-file path, hence executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Error code build-time gate, an example of the enforcement mechanism in this
  project
