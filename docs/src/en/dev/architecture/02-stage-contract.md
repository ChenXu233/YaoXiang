---
title: Compilation Stage Contract and Obligation Ledger
---

# Compilation Stage Contract and Obligation Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution stage sequence are in the RFC-039
> body; the positioning of each subsidiary document is in [this directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
problem: **"which stages ran this compilation, which didn't, and why"** is currently scattered
across six paths and ten entry functions, each manually wired — and no single place can answer that
question.

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` reads "Unproven → compile error, no
degradation, no silent pass……silent pass must not be resurrected." But this invariant only holds on
the **single-file path**: proof function constraints like `x: Sorted(3)` **silently pass** on the
multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths — the proof function never executes and
no diagnostic is produced.

### Coverage

- The `Stage` enum (exhaustive, no runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactoring plan for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line ranges touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction norms, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 body

### Division of Labor with RFC-039

RFC-039 gives the **why** of the refactoring and the **order in which to do it**; this document
gives L1's **concrete form**, **per-file change list**, and **implementation phases within this
document** (corresponding to P3 "fix correctness bugs" and P4 "stage contract and unified Driver" in
RFC-039's global sequence). In case of conflict between this document and RFC-039, RFC-039 takes
precedence.

Equivalence criteria are applied per the **C2 (orchestration change)** category in the
[Equivalence Criteria Document](07-equivalence-oracle.md): the diagnostic set at each entry point is
the same + the corpus behavior is the same.

## Current State

> All facts in this section are **verified**, each with file path + line number. Line numbers are
> based on `9e02e4db`.

### Stage Boundary Is the Unique "Miswrite Once, Silent Everywhere" Structural Defect

It differs in nature from the other two classes of defects (module boundaries, test wiring):

| Defect                      | Typical Manifestation           | Any Signal?              |
| --------------------------- | ------------------------------- | ------------------------ |
| Lexical/syntactic error     | Source code written wrong       | Has diagnostic           |
| Type mismatch               | Type written wrong              | Has diagnostic           |
| **Stage miss-wiring**       | **A stage is never called**     | **No signal whatsoever** |
| **Obligation not consumed** | **Field filled but never read** | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly" — code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentation convention rather than an executable constraint."

### Existing Enforcement Mechanisms in the Repository

The same repository already has mature enforcement mechanisms; they simply haven't been extended to
the stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are subjected to a
  **build-time hard gate** by `build.rs:19-55` via `tools/code-tables`, comparing each against the
  RFC-013 code table — any inconsistency directly `panic!`s and refuses compilation.
- `src/package/` (**76 files / 13012 lines**, of which `tests/` directory has 6227 lines, 47.9%) has
  high test density; each module's test subtree is wired in by declaration — this is the most
  complete test wiring in the repository, serving as a formal reference for stage-layer gates.

**Design capacity is sufficient. What's missing is "putting a same-level gate in the orchestration
layer too."**

(Line count metric: `(Get-Content).Count`, see the "Line Count Metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                         | Which Compilation                                                                  |
| --- | -------------------------- | ----------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`               | Direct 5-stage                                                                     |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`            | Wraps `Pipeline`                                                                   |
| 3a  | **Multi-file compile**     | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`     | Per-file `check_module` + IR + linking                                             |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399`    | Per-file typecheck, collect all diagnostics                                        |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`        | `check_module_collect_all` (`493`)                                                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`       | `check_module` (`1386`) + standalone IR                                            |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `run_file` in `src/lib.rs`                       | → `run_with_source_name` → Path 1                                                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `run_project` in `src/lib.rs`                    | → `compile_project` (Path 3a)                                                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`            | In-project goes to 3c, else manual lex→parse                                       |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`                 | → `check_project` (`590-591`)                                                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `run_code` / `test_compile` in `wasm/src/lib.rs` | `compile_playground` (`Program { kind: WasmPlayground }` + `Driver::run`) → Path 1 |

The 5 stages on the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined within** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The table below gives evidence cell by cell. **An empty cell doesn't mean "this entry doesn't do
this" — it means "no code in this entry does this"** — which is the very nature of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                            | `compile_project`                                      | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`            |
| --- | -------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------ |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                   | **No**                                                 | **No**                                                      | **No**                             | **No**                               |
| 2   | Dead code analysis                                       | Yes `275-278` (gated by `config.dead_code.enabled`) | **No**                                                 | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                               |
| 3   | W1006 local module shadowing                             | **No**                                              | **No**                                                 | Yes `336-348`                                               | **No**                             | **No**                               |
| 4   | W1003 unused import                                      | Yes (`277` consumes `type_result.warnings`)         | **Collected but never output**                         | Yes `350`                                                   | **No**                             | **No**                               |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                     | **No**                                                 | Yes (same as 2)                                             | **No**                             | **No**                               |
| 6   | **Monomorphization**                                     | Yes `381-389` (gated by `config.mono.enabled`)      | **No**                                                 | N/A                                                         | N/A                                | **No**                               |
| 7   | typecheck branch                                         | `check_module`                                      | `check_module` (`124`), **first-error-return** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)              |
| 8   | File discovery                                           | N/A                                                 | `discover` (`101`, discards `used_by`/`shadow_events`) | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                  |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                              | **No**                                                 | Yes (`321-328`)                                             | **No**                             | **No**                               |
| 10  | Global slot allocation                                   | N/A                                                 | Yes `allocate_global_slots` (`147`)                    | **No**                                                      | **No**                             | **No**                               |
| 11  | IR generation + qualified name rewriting + linking       | Yes (`205`)                                         | Yes (`152-236`)                                        | **No**                                                      | **No**                             | Yes (standalone ModuleIR then merge) |

> **Review note (WBS 3.4.3, 2026-10-07 cell-by-cell check)**: This table is the diagnostic snapshot
> for the `9e02e4db` baseline and is preserved unchanged. Differences in current state after P3
> lands:
>
> - **Row 1 fixed**: `proof_execution`'s five entries share the same implementation in
>   `frontend/proof_execution.rs` (pipeline + orchestrator four entries); the single-consumer defect
>   is gone.
> - **Rows 2/4/5 unchanged in current state** (multi-file `run` still doesn't output W1003, still
>   doesn't run dead code family) → attributed to WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → attributed to WBS 3.4.8 (prerequisite
>   4.1.3, latent risk unproven defect).
> - **Row 7** `check_module` / `check_module_collect_all` dual entries → attributed to 4.2.7
>   (Aggregation parameter driven).
> - The code-side reference to "Ruling #434" had zero docs registration previously — now registered
>   as RFC-039 **D57**.
> - **Embedded std incorporated into asymmetry (new out-of-table fact, 2026-10-09 retroactive
>   registration)**: The single-file path unconditionally injects `std.list` via
>   `merge_embedded_std_ir` (required for for-loop desugaring, #117 hard switch); multi-file
>   `discover` previously only recognized explicit `use` — in-project for-loops compile but fail at
>   runtime with E6006 (probe evidence). Fixed and ticked off WBS 4.10.2; sibling "multi-file
>   missing single-file step" parse hard-abort oddity registered as WBS 4.10.1 — **also fixed**
>   (immediately after 4.2.2, Check path degraded to per-file collection; this audit method examines
>   "stage coverage/field consumption" dimension, this is a compilation unit membership difference,
>   a missed-net dimension).
>
> - **Embedded std registration surface coverage (4.2.5 construction finding, fixed)**: The Registry
>   arm duplicate-collects registration for embedded std units via `extract_module_info`, which
>   overwrites the native half-face already registered by `with_std()` in merged form (loss of
>   `result.is_err` etc. → `std.test` falsely reports E1043). This audit's third missed-net
>   dimension: "the merge semantics of two registration sources for the same module key" — the
>   single-file path's registry only goes through `with_std()` once to form, multi-file/Check's
>   Registry arm inserts per-unit to have coverage.
>
> **Two places need precise wording, otherwise they will be written wrong:**
>
> - **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
>   (`run_project` → `compile_project`) never reports W1001/W1002/W1003; on the **single-file** path
>   (`run_file` → `Pipeline::run`) it does report. The reason is in `compile_project` — it packs the
>   full `TypeCheckResult` (including `warnings`) into `type_results`, but this function has **no
>   read site of `result.warnings` at all**; the only downstream use of `result` is
>   `generate_ir_with_context`.
> - **Row 11's `main` criterion comes from different sources across entries**:
>   `check_project:385-395` uses `surfaces.bins` (manifest-declared surface); `compile_project` uses
>   `is_bin_role` (`250-252`, only checks "whether there is a manifest"). The two functions give
>   different answers for "which files must define `main`".

### Correctness Bug: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are verifiable.**

**Step 1 — There is exactly one obligation production point.**
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

The semantics is: "the predicate argument of this refinement constraint is a compile-time literal;
the proof function must actually be executed to determine."

**Step 2 — The consumption point is inside the checker, but only produced, never consumed.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return-position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original text of the comment in `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch thus becomes a real defense line, not a forward-looking cushion: silent pass must not be resurrected.`

That is: the author knows "no diagnostic emitted" is a defect and explicitly classifies it as a
**known, nowhere-to-go intermediate state**, designed assuming "someone will definitely read
`proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares local
`proof_calls` and collects; `checker.rs:1439` writes to `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is **only read at `src/frontend/pipeline.rs:187`** in production code (argument
passed at `189`):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(Other hits of the `proof_calls` identifier in the repository fall into three categories, none of
which is a consumer of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-named field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — The four orchestrator entries all skip that layer.** `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
in `src/frontend/module/orchestrator.rs` **do not call `pipeline.rs`**; they directly call
`TypeChecker::check_module` (`124` / `493` / `1386`). Thus they can't even reach that sole read site
at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also take the
discarded path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible
for compiling embedded std. This means **the proof obligations of embedded std modules themselves
are also not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles, runs, with no diagnostic** on the multi-file /
check / LSP three paths. The proof function is never called.

**Step 8 (supplementary verification) — There is a second silent-discard point within the same
layer.** `checker.rs:1303-1314` when handling the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The ownership layer's `Unproven` is swallowed by an empty match arm, with no diagnostic and no
bookkeeping.** This directly contradicts the `5164` branch's promise of "silent pass must not be
resurrected", and is **independent of the orchestrator problem** — even if the entry layer is
completely fixed, this point will still be silent. (Processing timing: should be handled in the same
batch as the obligation ledger, because it is the same class of problem.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has zero hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of multi-file path proof obligations.

The three RFC-027 unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — e.g. `rfc027_return_refinement.rs:350-359` asserts that a `SumUpTo` call exists in
`result.proof_calls`. But they **all go via `check_source` → `checker.check_module(&module)`**
(`rfc027_return_refinement.rs:45/75`, `rfc027_refined_transparency.rs:27/57`), **stopping right
before `pipeline.rs`**.

The test file's own doc comment already states this. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases are at the .yx layer`

**This is exactly the shape of the problem: the test verifies "the obligation is filled", while the
bug is "the consumer end doesn't read".** A test that only tests the producer end, not the consumer
end, is naturally immune to this class of defect.

### Stronger Finding: The Equivalence Criteria's Main Corpus Is the Single-File Path

The [Equivalence Criteria Document](07-equivalence-oracle.md) takes the end-to-end diff of 293 `.yx`
corpus files in `tests/yaoxiang/` as the **third-tier criterion**, as the main acceptance method for
category C2 stages. But actual measurement shows:

- There is **no `yaoxiang.toml`** in the `tests/` directory (directory-wide glob has zero hits).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files take the single-file path, all cover proof_execution, all
  do not cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is exactly
the evidence for this point. That file is annotated `// expect: compile-error E4018` (line 15), and
its header comment on line 7 says "Status: ❌ Should be rejected at compile time" — **the fact that
it can pass is precisely because it takes the only path that executes the proof function**.

**Conclusion: The third-tier criterion needs a multi-file corpus layer added, otherwise it cannot
serve as this document's acceptance tool.** See implementation point S1.

### Obligation Fields: 13 "Produce-as-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** whose
doc comments explicitly say "produced by stage X → consumed by stage Y", i.e. they are essentially
**cross-stage obligations**:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Line-taking convention**: Definition line = the **declaration line** of the field in `types.rs`
> (the `pub` / `pub(super)` line), not the first line of the doc comment — the comment block for the
> same field may span multiple lines, and the declaration line is the only anchorable location. The
> **struct range** in the sentence above this table (`src/frontend/core/typecheck/types.rs:16-76`)
> floats with the struct line number and is mechanically reconciled by gate **F5** (miswrite is
> red); the definition line column is reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**`PlanId` keyed**:
> `ir_gen.rs:360` reads (`type_result.release_plan.clone()`), `:3103` uses (`ReleasePlan::consume`);
> settlement-complete marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId` keyed** — call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-complete marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Ruling record (3.4.9, 2026-10-13 user ruling A)**: The original row 14 `interface_impl_registry`
(`types.rs:49`) has been **reclassified as in-tree fact** — by design it is consumed within
typecheck phases, with zero cross-stage reads, so it is not an obligation:

- **At in-phase consumption points (live)**: `operator_interfaces.rs:623` (operator queries read
  `env.interface_impl_registry`), seven constraint-solver query sites in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), read-only accessors in
  `checker.rs:214-217`, body_checker injection in `checker.rs:974`.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; only the ledger's own registration
  and documentation on the `src/driver` side (removed in this round).
- **Disposition**: Delete the **declaration** "mirror for middle-end/LSP consumption" from the doc
  comment, keep the field and its in-phase consumption; the field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with reason). If LSP genuinely needs this
  table in the future, register it under 4.6.x, **don't create a consumer in this round**.

**All of these fields are passed across layers keyed by `Span` or name table.** The Equivalence
Criteria Document has already pointed out that span-keyed contract mismatches will "silently fail,
with no error whatsoever" (`ReleasePlan` is typical: any inconsistency in how the two sides compute
span → `Drop` instructions silently disappear).

Why did `release_plan` survive while `proof_calls` died? The difference is **structural, not
accidental**:

- The consumer of `release_plan` is `ir_gen.rs:360`
  (`release_plan: type_result.release_plan.clone()`) inside `generate_ir_with_context`, and
  `compile_project:154-156` **happens to pass the full `&TypeCheckResult` to it** — the consumer is
  in the downstream module, and the downstream module is on the necessary path of every entry.
- The consumer of `proof_calls` is at the **top level** of `pipeline.rs`, belonging to **another
  entry implementation**. The four orchestrator entries don't even go through the `pipeline.rs`
  layer.

**Verified fact**: The entire repository currently has **no mechanism** that ensures these 13 fields
are consumed. The only "protection" is that `ReleasePlan` happens to piggyback on IR generation.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Each field in `TypeCheckResult` that is not a cross-stage
obligation is registered individually (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why not a cross-stage obligation (key point)                                                                                                         |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (`src/middle` has 0 hits)                                                                                                     |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produce-as-contract")                                                                               |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but doc comment doesn't declare cross-stage contract ⇒ not included                                   |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings` — contract not declared                                              |
| `semantic_db`             | Consumer is in LSP (4.6.1 onwards exported via the `DriverOutcome.semantic_db` channel, `src/lsp/server.rs:482`), not a compilation stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, doc comment doesn't declare contract                                                                    |
| `warnings`                | Warning diagnostic channel (same class as `diagnostics`)                                                                                             |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption points see "Ruling record" in "§Obligation Fields")                                          |

**Why not "wire up a consumer"**: Creating a downstream for these fields would only introduce fake
obligations out of thin air, dressing up "designed not to need cross-stage consumption" as a
contract defect; the correct closure mechanism is **reverse containment** — `TypeCheckResult` field
set = obligation list ∪ non-obligation registration (hard-checked by
`scripts/ci/check-obligations.py` criterion 1: add a field but forget to register, delete a field
but forget to sync, non-obligation registration without reason, two tables overlapping all
immediately go red).

### Four Additional Contract Defects in the Proof Layer

All of the following come from proof layer analysis, independent of the entry-fork problem, but all
belong to the class "stage contract not enforced."

**(a) Layer order is only declared in documentation, with no mechanism to enforce; the `equivalence`
layer is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order in
`checker.rs::check_module_impl` (positioning anchored by function name / driver point, line numbers
taken from the current tree):

| Actual Order | Call Point                                                                                                                                         | Corresponding Layer |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| 1            | Refinement walk driver `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                    | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                       | `ownership.rs`      |
| 4            | Type constraint solving `env.solver().solve()` (not in `layers/`)                                                                                  | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                             | `equivalence.rs`    |

Layer number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) only exist in
each module's own header comment, and don't match the call order; there is no "number → call timing"
mapping in the README.

`equivalence` is only used as the `is_subtype` utility function by `inference/assignment.rs:17`, and
has nothing to do with `ProofResult`. At the same time **no short-circuit exists** —
`check_module_impl` continues running after termination errors are `add_error`ed one by one,
"lower-layer failure doesn't run upper-layer" is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Review note (2026-10-07)**: The hard-fail panic was eliminated by 3.3.1 in P3 (SOLVER slot made
> `Option`, missing-downgraded conservatively as `SMTResult::Unknown`); "silently skip (not
> injected)" was patched by 3.4.2 with W1081 signal. The unification of the three forms
> (singletonization) is executed per **RFC-039 D58**; the actual fix location is
> `proof/smt/backend.rs` (new process-level shared singleton + `with_shared_solver` closure port);
> the `default_solver() → &'static` wording in the 02 change list defers to D58 (the `&'static` raw
> reference is not feasible: `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Construction-time injection `with_solver_owned`          | `None` → **silently skip** (not injected)                 |
| `ownership.rs:627` (back-edge cut determination)        | **Per call** `default_solver()`                          | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly chose hard failure: "Initialization failure maintains
**hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds kernel
capability'." And `backend.rs:60`'s contract documentation says "`None`: backend
unavailable……**caller should conservatively degrade**." **Two philosophies coexist in the same
codebase, and the hard-failure one panics instead of returning an error.**

**(c) Each back-edge determination creates a new Z3 context, cache is a dead letter.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cut determination. And
`src/frontend/core/typecheck/proof/smt/backend.rs:67-72`:

```rust
pub fn default_solver() -> Option<Box<dyn Solver>> {
    match super::z3_backend::Z3Backend::new() {
        Ok(b) => Some(Box::new(b)),
        Err(_) => None,
    }
}
```

**This is a factory function, not a singleton** — every call does `Z3Backend::new()`, i.e. creates a
new Z3 context. The cache field of `Z3Backend` (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, but the doc comment at
`z3_backend.rs:17` claims "SMT query results are cached in `cache`". Not shared across calls ⇒
**this cache never hits under this call pattern**.

(Comparison: the `LazyLock` at `predicate.rs:34-36` really is a singleton. Same backend, two
lifetime strategies.)

**(d) Checker silent degradation + comment doesn't match reality.** `checker.rs:1283-1287` and
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
table** and proceeds normally — ownership analysis degenerates to "nothing conflicts", **with no
warning**. (Processing suggestion: should be recorded as a warning-level diagnostic, or at least
left a mark in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving." Actually `self.env.solver().solve()` is at `checker.rs:1320` — **termination is
at `1256`, ownership is at `1296` afterwards, i.e. the "before" mentioned in the comment is actually
"two layers later"**.

### wasm Status: Borne by Shim Crate, 29 Files' Branches Are Live

The wasm target **has been built and is built in CI**; the `cdylib` is not in the main crate but in
a separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has wasm target dependency section     | `Cargo.toml:139-140` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (corresponding non-wasm section at `Cargo.toml:131-137`: tokio/ureq/tempfile)                                                                                                       |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:77` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is pre-built by Emscripten | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetches `libz3.a` from fixed URL, degrades to warning if missing)                                                                                                                                                              |
| **29 files** contain the literal `wasm32`         | Of which **22 files** have actual `#[cfg(...)]` attributes, the other 7 only have doc references (the list and per-item determination see the "F2 Determination" section of `06-cleanup-inventory.md`)                                                                            |
| `orchestrator.rs` 16 places                       | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                              |
| `src/lib.rs` 11 places                            | All attributes (`28/31/33/47/145/147/151/170/187/196/269`)                                                                                                                                                                                                                        |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches in these 29 files are load-bearing.**
They determine which APIs of the main crate can be called by `wasm/src/lib.rs` (152 lines) under the
wasm target — `src/lib.rs:151/170/187` gate `run_file` / `run_project` / `build_bytecode` as a whole
(these three all need `std::fs`), and the shim takes a different path.

**But this brings a fact directly related to this document**: The playground entry
`compile_playground` (`wasm/src/lib.rs:84`) constructs `Program { kind: WasmPlayground }` and hands
it to `Driver::run` (JS-side entry `run_code`, `:122`) — **the single-file path**. Therefore:

| Path Consuming `proof_calls`                              | Whether Proof Function Is Executed |
| --------------------------------------------------------- | ---------------------------------- |
| Single-file CLI (`run_file` in `src/lib.rs`, `:152`)      | **Yes**                            |
| wasm playground (`run_code` in `wasm/src/lib.rs`, `:122`) | **Yes**                            |
| `build_bytecode` (`src/lib.rs`, `:188`)                   | **Yes**                            |
| Multi-file `run` (`compile_project`)                      | **No**                             |
| `check` (`check_project`)                                 | **No**                             |
| LSP in-project (`check_source_in_project`)                | **No**                             |

That is: **The existence of `wasm/` turns "proof_execution has only one consumer" into "has
three"**, but all three are on the single-file path side. When unifying the Driver, this path is
already covered by `ProgramKind::WasmPlayground` (`src/driver/program.rs:31`, sharing the same
single-file stage table as `SingleFile`, `:135`); the landing record is in the **4.2.8** row of
`09-execution-wbs.md` (`wasm` switches to `ProgramKind::WasmPlayground`, `:274`).

(Cleanup scope — which of the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative) are
unreachable in the playground scenario — goes to the "F2 Determination" section of
`06-cleanup-inventory.md`: 4 wasm arms are unreachable at playground runtime but load-bearing at
compile time, 6 `not(wasm32)` arms are load-bearing on the three native stage tables, no low-risk
items that can be directly deleted. The "build wasm target, or delete these branches" open issue in
RFC-039 **has expired**: the target is already built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, No Runtime Registration)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` enforces at compile time that "new stages must be
handled by the orchestration layer" — which is exactly the mechanism proven effective by the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check       Project
    Discovery,          // File discovery                 Project
    Parsing,            // Lexical + syntactic            PerModule
    Registry,           // Module registry build          Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type check (with embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis      Project
    ProofExecution,     // Proof function compile-time execution PerModule
    GlobalSlotAlloc,    // Global slot allocation         Project
    IrGeneration,       // AST → ModuleIR                 PerModule
    Monomorphization,   // Monomorphization               Project
    Linking,            // Cross-module linking / IR merge Project
}

impl Stage {
    /// All stages. When adding a new variant, this array and the exhaustive match
    /// in `dispatch` will both fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs once
at project level) make "which stages must run per module, which must run once at project level" a
type-level fact. Project-scope stages are structurally guaranteed to run only once in `dispatch` —
this eliminates current issues in `orchestrator.rs` like "should `allocate_global_slots` be called
per file" that require human reasoning.

**The stage table is ordered by topology rather than alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the draft are corrected per
> implementation evidence:
>
> 1. `Monomorphization` is moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, `pipeline.rs`); the draft order contradicts the data
>    flow.
> 2. The Check form's stage table is normalized to dead code **before** proof in `Stage::ALL`
>    topology: `check_project` currently runs proof before dead code, with no data dependency
>    between them, the diagnostic set is the same (C2 set semantics), only the in-file diagnostic
>    order is normalized. The single-file path (dead code embedded in typecheck, before proof)
>    already conforms to the ALL order, byte-by-byte acceptance is unaffected.
> 3. `Parsing` is moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    draft order would force the Registry arm to "hide parse", making the stage table lie to the
>    data flow; and multi-file parse thereby goes from 2 times to 1 time. Side-effect current state
>    honestly registered: multi-file path parse errors were previously a hard abort
>    (`build_registry_from`'s `?` propagation, the tension with CollectAll semantics registered as
>    WBS 4.10.1) — **4.10.1 has been fixed** (2026-10-09 user ruling rust-style collection
>    semantics + Plan B): Check path parse failure degraded to per-file diagnostic collection,
>    diseased file exits the compilation unit (not entering registry, importer reports E5001);
>    MultiFile path retains hard abort (FailFast produces no IR for bad files, this is correct
>    semantics, nailed permanently).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edges 14→16: `RoleClassification` consumes the `used_by` edge set from Discovery,
>    `DeadCodeAnalysis` consumes W1006 shadowing events from Discovery — the draft omitted these
>    (these two products were produced by `discover_with_used` in the orchestrator era, discarded by
>    `discover` wrapper, the data flow was lost because it wasn't in the table); b. The in-file
>    diagnostic order normalization is extended to the full form of note #2 above: E3020 entry-point
>    check is moved into the RoleClassification arm (Check has no Linking stage, the arm holding
>    surfaces and AST is responsible for checking), in-file diagnostic order = stage topology order
>    (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is unchanged (C2 set
>    semantics), only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: Turn "field produced but not consumed by anyone" from undetectable into a fact that
can be asserted at compile time or test time. RFC-039 has already listed this as "the most valuable
item."

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
    // …other span-keyed fields
}

impl Obligations {
    /// Settlement: called once at the end of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three levels)**:

| Situation                            | Criterion                                                    | Disposition                                                                     |
| ------------------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| Obligation read by declared consumer | That field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                            |
| Obligation non-empty but no consumer | Field non-empty and not consumed                             | **Produce diagnostic** (`W` level, default; `strict` mode upgrade to `E` level) |
| Obligation is empty                  | Field is empty                                               | Pass (no consumption needed)                                                    |

**Why start with W level rather than E level**: Fixing obligation discard will **change the
diagnostic set**. C2 criterion requires "diagnostic set at each entry is the same", if we go to E
level in one step, `yaoxiang check` will suddenly produce a large number of previously silenced
`Unproven` diagnostics. Splitting into two steps (W observation first, then E upgrade) allows each
step's criterion to be usable. See implementation point S4 for details.

**The call point of `assert_drained()` is unique**: `Driver::run` at the end of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (a field appearing
≥2 times but no third file read → fail). `assert_drained()` is a **runtime** gate, that script is a
**static** gate; the two are complementary and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note registers the
> **implementation form, measured numbers, and 4.3.2 interface** for the above sketch and
> three-level table, with one-to-one correspondence to code `file:line`. Landing commit `1941cc47a`
> (verification tree `99079bc52`). It is also the **third** of the three "no-silent" red-list
> bearers (the other two: audit surface, static gate).
>
> **One, Landing objects (`file:line`)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation inventory `INVENTORY` (`:201`, field
>   name / consumer stage / red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (`:150`, 8 entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), external emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **The only settlement point**: at the end of `Driver::run`'s stage table, before
>   `into_outcome()` produces `CompilationResult` — `src/driver/mod.rs:110`
>   (`State::assert_drained`, `obligations.rs:570`). The early-exit path of `Aggregation::FailFast`
>   is also settled (the books must balance).
> - **Seven delivery points** (one per arm; remove any one, runtime W1090 must fire): ProofExecution
>   `src/driver/arms.rs:554` (`proof_calls()` is the only external-read path of `TypeCheckResult`
>   for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) / `:726` (Embedded) /
>   `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — same delivery port at four places:
>   `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`; Monomorphization
>   `arms.rs:936` (single-file direct-read request table) / `:1221` (MultiFile·Check: Linking arm
>   aggregation, `driver/helpers.rs:295` reads each unit's `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation formula
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by test cases.
> - Diagnostic code: W1090 registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason written above the entry at `:26-31` — settlement granularity is
>   "unit × obligation", with no user-pointable source location, no dummy span stuffed);
>   six-language texts in `locales/*.json`. **Emission level**: starting from 4.3.2 it is **error**
>   (same code upgraded to E, see third section) — determined by the explicit
>   `.severity(Severity::Error)` of `to_diagnostic` (`obligations.rs:444`), code number still in the
>   W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, with `--selftest`).
>
> **Two, Three-level table per-cell landing + two additional levels**
>
> | Three-level table situation above    | Implementation criterion                                                                     | Code                     |
> | ------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------ |
> | Obligation read by declared consumer | **Delivery marker exists** (marker ↔ real read reconciled by static gate, see fifth section) | `obligations.rs:618-623` |
> | Obligation non-empty but no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error level**, see third section)  | `obligations.rs:624-630` |
> | Obligation is empty                  | `Obligation::is_produced` field-by-field empty check → pass                                  | `obligations.rs:587-591` |
>
> Two **additional levels** (2026-10-13 user ruling, not in sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **don't
>    judge owing** (`obligations.rs:593-597`). Evidence: `LSP_STAGES` has no
>    `IrGeneration`/`Monomorphization` (`program.rs:112-119`), `EMBEDDED_STAGES` has no
>    `Monomorphization` (`:121-126`).
> 2. **Skip exemption**: Consumer stage skipped with `SkipReason::UpstreamFailed` / `ConfigDisabled`
>    → don't judge owing (`obligations.rs:598-602`). `NoObligations` **does not** exempt —
>    obligation non-empty while that stage claims no obligations, that itself means the books are
>    unbalanced, must report.
>
> **Three, External emission caliber (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: This mechanism only emits at **W level** (W1090); **no strict
> switch**. **4.3.2 (terminal state, 2026-10-13 user ruling A)**: **Same code upgraded to E** — code
> number `W1090` unchanged, emission level raised to `error` (explicit `.severity(Severity::Error)`,
> `to_diagnostic` at `obligations.rs:444`); **no new E code** (one fact, two codes means parallel
> representation). **The upgrade only changes the emission level, not the emission conditions**.
> **This is a D59 class D break item (changes externally observable behavior), honestly
> registered**: healthy program zero-emit (zero emission when all obligations have delivery
> markers), so existing corpus and user programs are unaffected (342/342 "diff: empty"); but once a
> real run hits an undelivered obligation, that diagnostic is **counted into `error_count`, rendered
> as `error[W1090]`, exit code non-zero** — different from the previous "W level doesn't block
> compilation". The separation of code family and emission level caliber is registered in RFC-013
> "Emission Channel" section (code number prefix only determines number ownership, level is
> determined by explicit severity at construction point). Mechanism fact (measured): severity takes
> explicit `.severity()`, otherwise "code starts with `W` → Warning" (`codes/builder.rs:186-203`),
> the registry's `ErrorCategory` **does not participate** in severity; exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`), **W1090 goes into `error_count` (starting from 4.3.2) because it's
> explicit error**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> current red list has been zeroed, see fourth section), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is injected by the
>   typecheck preset table, **constantly non-empty**, what is emitted per compilation is a
>   **constant signal** (no discriminating power), yet it would break three existing external
>   contracts — "clean program zero diagnostic / LSP zero diagnostic / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing use cases turned red**, of which **8
>   are in out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 are
>   in `src/driver/tests/`), corpus baseline 24/342 fixtures changed, and the only feasible fix is
>   "filter W1090 out of the zero-diagnostic assertion" = relaxing existing criteria.
> - Ruling B (downgrade to Info level) still breaks 9/12 of the list-style assertions, and Info is
>   not "output per ruling level".
> - Ruling C measured: existing use cases **zero changes**, corpus baseline **zero changes**
>   (342/342 "diff: empty, behaviorally equivalent"), runtime W1090 becomes a **discriminating
>   regression detector** — healthy tree is silent throughout, only fires when a delivery marker is
>   removed (removal-mutation at each of the three delivery points, CLI `check` outputs 2
>   `warning [W1090]` when removing marker on Check arm).
>
> Red list's "no silence" is borne by three places: ① `DriverOutcome.obligations.findings` audit
> surface (carried on every run, with WBS id and evidence pointer); ② `check-obligations.py` prints
> the red list on every run (currently empty) + staleness check; ③ this note.
>
> **Four, Red list current state (cleanup closure) and landing records for 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0** — all three cleanup tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen produces
>   CreateVariant") is equivalently fulfilled by IR-side in-place shape detection:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST shape
>   `FieldAccess{Call(Var)/Var}` + takes declared order from `sum_type_variants`), call site `:7496`
>   replaces the entire Call with `CreateVariant`; the function doc explicitly states "span key
>   cross-system is unreliable (the two sides have different semantics for Call node spans), so
>   detect in place" — **wiring a consumer would bring back the already-fixed span mismatch
>   defect**. Deletion surface: `types.rs` field, collection in `checker.rs:1489-1493/:1565`,
>   collection chain in `inference/expressions.rs` and `inference/statements.rs`, `VariantCtorCall`
>   struct in `environment.rs`; `infer_variant_ctor_call`'s type inference and all diagnostics
>   **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (kept in in-phase candidate
>   name table and `overload_resolutions`). All read/write points: written in phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in phase
>   (`expressions.rs:4490-4550` resolution point, when unique `:4550` writes to
>   `overload_resolutions`); deleted is the **zero-reader mirror** injected by `checker.rs:1558`
>   into `TypeCheckResult`. Naming fact still expressed in a single point: candidate name table (in
>   phase) + span-keyed resolution (cross-stage, `ir_gen.rs:380` construct, `:7632/:7705` consume);
>   definition-side mixed names are self-computed by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user ruling A, see ruling
>   record in this document's "§Obligation Fields" table). Deleted is the **declaration** in the doc
>   comment "mirror for middle-end/LSP consumption" (`types.rs:47-48`), kept is the field and its
>   in-phase consumption (`operator_interfaces.rs:623` operator query, seven constraint-solver query
>   sites in `inference/expressions.rs`, `checker.rs:214-217` read-only accessor); cross-stage
>   zero-read direct evidence `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why
>   not wire a consumer**: the content of this table is an environment mirror injected by the
>   typecheck preset table (constantly non-empty); if LSP really needs it, it goes through 4.6.x new
>   registration; creating a consumer for "fields designed not to need cross-stage consumption" only
>   introduces fake obligations into the ledger.
>
> After the red list is zeroed, **field enumeration is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reasons)"** — gate criterion 1's reverse
> containment guards this end-to-end (add a field but forget to register / delete a field but forget
> to sync all immediately go red). Red list **mechanism** is kept: gate criterion 4's staleness
> check and `--selftest`'s `stale-red-list` fixtures continue to cover its detection surface; future
> re-registration must go through explicit ruling.
>
> Synchronized calibers with cleanup: obligation list 16 → **13** (`INVENTORY` / `Obligation::ALL` /
> "§Obligation Fields" table in this document); gate `--selftest` now covers seven types of fixtures
> (newly added "unregistered field", "stale anchor", and "red list is empty" three forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **Five, Actual criteria of static gate (9.2)** (three more hard criteria than the one-sentence
> sketch)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor check (F1)**: `INVENTORY` of
>    `obligations.rs` ↔ `TypeCheckResult` field definition ↔ "§Obligation Fields" table in this
>    document (including table header count declaration), any drift goes red; and
>    `TypeCheckResult`'s field set must **exactly equal** "obligation list ∪ non-obligation
>    registration (`NON_OBLIGATION_FIELDS`, with reasons)" — add a field but forget to register,
>    delete a field but forget to sync, non-obligation registration without reason, two tables
>    overlapping all immediately go red;
> 2. **Read-out evidence**: Non-red-list obligations must be **truly read** in the **declared
>    consumer file** (`.fieldname` member access; comments/tests/strings don't count) — this makes
>    "delivery marker ↔ downstream real read" a hard criterion (the main risk of mechanism A is
>    exactly that the two drift);
> 3. **Delivery marker**: Non-red-list obligations must be marked at delivery points; conversely,
>    red-list items must not be marked;
> 4. **Red list staleness check**: Red-list fields must have zero real reads (if a consumer grows,
>    go red), and each must have a WBS id and evidence pointer; also `--selftest`: includes
>    negative/positive fixtures, negative must be red, positive must be green (proving the detector
>    doesn't idle).
>
> **Six, Per-place deviations from the above sketch**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **doesn't copy field-by-field**: obligation
>    data is still held by `TypeCheckResult`, the ledger only records "inventory + delivery
>    markers + settlement conclusion". Reason: copying means two sources of fact (each field is
>    produced by typecheck's checker and ownership layer respectively), inevitably drift;
>    cross-layer data surface shouldn't be replicated to orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, changed to `is_produced` empty check +
>    `DeliveryLedger` delivery markers. Reason: obligation and product are the same piece of data,
>    `take()` would take away what the downstream needs; and the 11 IrGeneration obligations are in
>    the form of "pass the entire `&TypeCheckResult` to ir_gen", which can't be taken
>    field-by-field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusion written to `DriverOutcome.obligations`,
>    diagnostics dispatched by `ProgramKind`: Check/Lsp go through the per-file collection channel;
>    other forms dispatch by **level** (W1090 is E level starting from 4.3.2) — single-file goes to
>    `PipelineError::TypeCheck` (external emission arm of `obligations.rs:654`), multi-file/embedded
>    separately record `OrchestratorError` failures (otherwise `result.errors` has no consumer).
>    Reason: returning `Vec<Diagnostic>` would lose form dispatch — Check's diagnostics must be
>    attributed per file.
> 4. Three-level table first-level criterion changed from "obligation read by declared consumer" to
>    "has delivery marker"; "marker ↔ real read" handed to static gate (fifth section criterion 2).
> 5. Three-level table second-level's "`strict` mode upgrade to E level" **didn't go through strict
>    switch**, changed to **4.3.2 same-code upgrade to E** (upgraded even without strict switch,
>    because healthy state zero-emission is already safe enough, R4).
> 6. Static gate criterion upgraded from "field appears ≥2 times but no third file read" to the four
>    criteria in the fifth section.
> 7. The two additional levels in the sketch (S1, skip exemption) see the second section.
> 8. The unit test name `test_obligations_drained` mentioned in S1/S4 table: actually landed as **20
>    use cases** in `src/driver/tests/obligations.rs` (first `#[ignore]` skeleton → turned green;
>    red state archive: after removing the only settlement point `pairs: 0`, 6 failed). S1/S4 two
>    rows have been synchronized to the actual file.
>
> **Seven, Landing record for 4.3.2 (level + gate reinforcement)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the kinds of obligations
>   actually drained in this run, per-run obtainable) and five counts (conservation formula in
>   second section); combined with audit surface `findings` can give "whether the field has been
>   consumed by any entry" fact.
> - **Level: same code upgraded to E has landed** (2026-10-13 user ruling A; see terminal state
>   registration and D59 D class break declaration in third section). Implementation surface:
>   `to_diagnostic` explicit `.severity(Severity::Error)`; external emission arm dispatches error
>   channel by level. Mutation evidence: remove any delivery marker → that form's compilation fails,
>   `check` exit code non-zero (`error [W1090]`); after restoration healthy state zero-emission
>   (corpus 342/342 zero change). Locale text has been written in six languages "W family code,
>   emission level is error".
> - **Gate reinforcement items (F1/F2, both landed)**:
>   - **F1/R2 anchor check**: gate now parses **non-obligation registration reason and red list
>     evidence** 's `file:line` anchor, checks "file is locatable (unique) + line number is
>     registered + the symbol immediately after the anchor really appears in the anchored file"; if
>     the anchor points to a non-existent symbol, go red. Includes negative fixtures
>     `tests/baselines/obligations-gate/stale-anchor/` (symbol doesn't exist / line number out of
>     range, one each). Note: red list is currently empty, so the actual landing point of this
>     criterion is **non-obligation registration** and future evidence pointers (same code path).
>   - **F2/R3 reverse containment**: gate outputs "`TypeCheckResult` field count − obligation count
>     − non-obligation registration count = diff", non-empty diff goes red; includes negative
>     fixture `unregistered-field/` (struct added new field but not registered).
> - **Informational F3**: `SkipReason::UpstreamFailed` exemption is unreachable in real entries
>   (typecheck-failed units don't keep `type_result`, obligations don't exist in the first place);
>   this level's semantics is pinned by synthetic `State` use cases, real end-to-end only covers
>   `ConfigDisabled` (t3 T06).
>
> **Eight, 4.3 secondary task closure (red list zeroed + obligation diagnostic terminal state E
> level)**
>
> **8.1 Three-round landing timeline** (one-to-one correspondence to code `file:line`; this round
> only adds conclusions, doesn't change code)
>
> | Round               | Conclusion                                                                                                                                                                        | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
> | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger online | `State::assert_drained()` unique settlement point; 13-entry obligation inventory + delivery marker + three-level settlement; audit surface `DriverOutcome.obligations`            | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; external emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                            |
> | 3.4.4 Cleanup       | `variant_ctor_calls` delete field — declared responsibility equivalently fulfilled by IR-side in-place shape detection                                                            | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                 |
> | 3.4.5 Cleanup       | `method_overload_ir_names` delete cross-stage mirror (kept in in-phase candidate name table + `overload_resolutions`)                                                             | Resolution consumption points `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                           |
> | 3.4.9 Cleanup       | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirror for middle/LSP consumption" declaration, keep in-phase consumption; obligation 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment at `:47-51` in same file, ruling A rewrite); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, seven sites in `src/frontend/core/typecheck/inference/expressions.rs` (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor `:214-217`); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level         | **Same code upgraded to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; external emission dispatches error channel by level (first failure first)          | `src/driver/obligations.rs:450` (severity), `:676-699` (E level branch); code family and level separation registration in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                               |
>
> **8.2 Closure conclusion**
>
> - **Red list zeroed**: `INVENTORY` (`src/driver/obligations.rs:201`) number of entries with
>   `red_list: Some` = **0** (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0).
>   Mechanism **preserved**: `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's
>   staleness check and `stale-red-list` fixtures; gate output "red list 0 entries — cleanup
>   zeroed".
> - **Obligation diagnostic terminal state E level**: `W1090`'s **code number and W family ownership
>   unchanged**, emission level is `error` (D5 terminal, 2026-10-13 user ruling A). This **belongs
>   to D59's class D break item** (changes externally observable behavior), registered in this
>   note's third section and RFC-013 "Emission Channel" section; **healthy state zero-emission ⇒
>   existing corpus and user programs unaffected** (corpus 342/342 "diff: empty", IR snapshot zero
>   drift; t11 independent verification and t12 re-review each recheck).
> - **Field enumeration closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption
>   reason); gate F2 explicitly outputs the diff (currently `21 - 13 - 8 = 0`).
> - **No unlanded promises left**: all reinforcement items registered in this note have **landed**
>   (F1/F2/F4/F5, see fifth, seventh, eighth sections); the only two unclosed items are the **known
>   boundaries** of 8.4 and the already-issued t7 F5, neither is an unlanded criterion of this
>   mechanism.
>
> **8.3 Disposition of each round's acceptance / re-review findings** (one-by-one)
>
> | Source        | Finding                                                                                                                                                                 | Status                                                                                                                                                                                                                                                                |
> | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, silently stale when alternative consumer is removed                                                                            | **Landed**: gate F1 anchor check (file locatable + line number registered + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                  |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, silently PASS when new field is added but not registered                                                                    | **Landed**: F2 diff explicit output (`:577`) + F4 definition line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                           |
> | t4 R4         | W-only and upgrade-to-E caliber not registered                                                                                                                          | **Landed**: this note's third/seventh section + RFC-013 "Emission Channel" + six-language `locales/*.json` W1090 help and error_output examples                                                                                                                       |
> | t7 F5         | Dead code analysis has distortion in overload method reference counting (legal overloads still falsely reported as `W1005`)                                             | **Transferred to issue `#450`**, attributed to WBS **4.9 reference pool correct implementation** (registered at `docs/src/dev/architecture/09-execution-wbs.md:237`) — not this secondary task                                                                        |
> | t11 F7        | Gate `--root` mode F1 doesn't cover anchor commitments in module header prose comments                                                                                  | **Known boundary** (see 8.4)                                                                                                                                                                                                                                          |
> | t11 F8        | Validator's own tooling side note: mutation driver once mistakenly wrote to main repo (immediate `git restore` cleared, affected rounds' output invalidated and re-run) | **Informational** (not a deliverable defect)                                                                                                                                                                                                                          |
> | t11 F9        | Main repo intermediate state (unstaged files make this document's table header and `INVENTORY` desynchronized) triggered gate red                                       | **Informational / resolved**: frozen commit self-consistent PASS; "intermediate state direct commit will red gate" is exactly the design intent of F2/F4                                                                                                              |
> | t12 F12-1     | This document's struct range written as `16-73`, actually `16-76` (no mechanical reconciliation after t9 added 3 lines of comments)                                     | **Landed + mechanized**: range corrected to `16-76`, and new gate **F5** (range anchor, miswrite goes red); same-class numbers grep-aligned (`17 → 20 use cases`, `16 span-keyed fields` → 13)                                                                        |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                               | **Fixed** (captain has amended commit message to explain): `src/driver/obligations.rs:686`'s `failure.is_none()` guard; this regression was caught by **existing nailed** use case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registration items, not unlanded promises)**
>
> - **F1's scan surface**: Anchor check only covers **registered entries**' text — non-obligation
>   registration reasons (`NON_OBLIGATION_FIELDS`) and red list evidence; **`file:line` commitments
>   in module header prose comments are not in the scan surface** (currently no registered entries
>   are affected). If anchor commitments are written in module headers in the future, **human review
>   must bear the responsibility**, or expand F1's scan surface — choose one, no default unowned.
> - **Static boundary of gate**: The set-level check of delivery markers is on the static side
>   (criterion 3), single-arm removal is complemented by **runtime settlement** — static + runtime
>   double layer is the designed form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: Among the above 13 obligations, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note registers the landing form, criteria, classification basis, and known
> boundaries; the definition line is still reconciled by gate **F4** (the line numbers in the table
> above are the current code facts).
>
> **One, Landing form (carrier A: `PlanId`)**
>
> The key is a **structural identity** that follows AST nodes, no longer relying on the positional
> coincidence of "each side computes span once": `PlanId` (`src/util/plan_id.rs`, `u32` monotonic,
> unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes outside the
> parser) is issued by the **sole allocator** `ParserState::alloc_plan_id`. Carriers at two places:
> `Stmt.plan_id` (statement identity → release plan ownership and consumption) and
> `Expr::Call.plan_id` (call-site identity → overload resolution produce and consume); `Expr::call`
> is the only call expression constructor in the repository, synthetic calls outside the parser are
> all recorded as `UNPLANNED` (**not consumable**, see second section).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the only
>   consumption entry `ReleasePlan::consume(stmt)` — **read and bookkeeping in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (statement identity given by `walk_stmt`'s save/restore +
>   CFG node → statement identity table; Drop at scope exit belongs to the last statement of that
>   scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one body**),
>   production point `inference/expressions.rs:4550` (write by call-site `PlanId`, dispatch is
>   deconstructed transparently from `Expr::Call { plan_id }`), consumption points `ir_gen.rs:7697`
>   (with binding) / `:7772` (no binding) via `OverloadResolutions::get` (hit = bookkeeping).
>
> **Two, Consumption-must-reach settlement (reuses 4.3 ledger, no second one)**: Consumption ledger
> is the **sole implementation** `ConsumptionLedger<K>` (`src/util/consumption.rs`), each of the two
> tables holds one instance; when `generate_module_ir` successfully returns, **the same point**
> marks "complete run of one round" for both ledgers (`ir_gen.rs:1095` release plan / `:1097`
> overload resolution); three-state mapping is unique (`consumption_state` in
> `src/driver/obligations.rs`), the unique settlement point hooks in one line for each of the two
> obligations — non-empty diff goes **E-level W1090** (same code upgraded to E, see third section
> for caliber). Exemption criterion hangs on **the completion marker of the ledger itself** (not
> whether `unit.ir` exists), so `check` form taking IR products doesn't constitute exemption.
>
> **Three, D59 classification basis (class C, red before green)**:
> `Type.method(positional arg, name = value)` this form before the change was **silently giving the
> wrong answer** (measured: `r: Int = t.pick(1, a = 2)` compiles cleanly, runtime outputs `1`
> instead of `20` — the resolved candidate is discarded, falls back to the last binding in
> definition order); after the change the same program compile-time
> `error [W1090] ... produced key not consumed`. Fix takes **loud** rather than "stuffing the
> resolved name into the named-argument call": the latter loses named arguments, equivalent to
> another silence (terminal semantics see [09](./09-execution-wbs.md) §3.14 / issue #452).
> Additionally: `Drop` instruction is a no-op in the interpreter (general arm of
> `src/backends/interpreter/executor/ops/control.rs` only `advance()`), so the key migration of
> release plan doesn't change runtime behavior — corpus 342 diffs empty is evidence of this.
>
> **Four, Criteria and evidence**: D41 same-type criterion `test_release_plan_spans_consumed`
> (settle by `PlanId`) and new `test_overload_resolutions_consumed` (corpus per-file
> lex→parse→typecheck→ir_gen, **diff must be empty + evidence surface floor**: number of files that
> produced resolutions `≥ 1`, real consumed entries `≥ 2`, no whitelist); resolution form covers 9
> nailed items see `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return /
> positional args / type-body binding / `?` operand / pure named / positional+named / ambiguous /
> variant ctor / unassigned identity). Mutation driver: consumer key offset by one ⇒ criterion red +
> real program `error [W1090]` + corpus gate diff; green again after restoration. Gate numbers:
> `check-obligations.py` PASS, corpus 342 diffs empty, `cargo test --lib` 2712 passed / 0 failed (IR
> snapshot zero drift).
>
> **Five, Known boundaries (registration items, not unlanded promises)**: `ir_gen.rs:7766-7790`'s
> **no-binding consumption point** is **unreachable** in current corpus and test surface (method
> declarations uniformly go through `register_type_bindings` to the with-binding consumption point),
> so the key correctness of this branch **has no red state to falsify**; retained reason = it's the
> symmetric branch of the with-binding path, deletion would rewrite the "no `type_bindings` method
> call" form into silence. Registered by independent verification (t5 F-1) and re-review (t6 F2),
> **no new task**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // Pipeline::run / run_file / build_bytecode
    MultiFile,         // compile_project
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // compile_playground in wasm/src/lib.rs — fourth caller of single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of the 6 predefined combinations of `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`), it does
not accept the caller freely passing an array.** This is a key constraint to prevent the `Program`
abstraction from degenerating into "free parameters for anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology decides whether to skip, not by stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, none can be missing
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Unique settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the above sketch, semantically equivalent, registered for future reference:
>
> 1. §4's `StageOutcome` three states are not returned by the stage arm, but expressed by `State`
>    failure marker + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, saves per-arm
>    boilerplate return);
> 2. `Driver` has no `config` field — the only source of config is `Program.config` (§5 "Avoid
>    Driver holding mutable global state" determination covers the sketch field);
> 3. `Skipped` diagnostics in 4.1.3 are only recorded into `DriverOutcome.skipped` internal ledger,
>    not externally emitted — S2 "intentionally fix no bug" zero-diff criterion requires; external
>    emission goes with 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is allowed direction); the orchestrator's existing call sites will be
>    repositioned for further discussion after 4.2 migration.
> 5. Under multi-file form, ProofExecution is an independent stage, executed after **all** typecheck
>    (A1, 2026-10-09 user ruling): `compile_project` originally interspersed proof in the per-file
>    typecheck loop (file N's proof before file N+1's typecheck). Single failure both byte-by-byte
>    identical; the "file 1 proof fails + file 2 typecheck fails" multi-failure scenario, the first
>    report changes from proof error to typecheck error (test nailed).
> 6. `DriverOutcome` carries products by `ProgramKind` channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1) — each entry's external error contract
>    (PipelineError / OrchestratorError) is not a type surface the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every found file has an entry, clean files have empty Vec — `check_project` current contract);
>    Discovery product extended to "file set + used_by edge set + shadow events" triple into State
>    (MultiFile form has no downstream consumer, only records, doesn't emit); check path per-file
>    parse goes from 2 times to 1 time per C3 topology.
> 8. Lsp form landing (4.2.3, dispatched by file source, 2026-10-09 user ruling): Disk file parse
>    failure same as Check's Plan B (collect + exit compilation unit — both ends degradation
>    converges); edited buffer parse failure keeps incomplete AST to continue typecheck (editor
>    philosophy — intermediate state of typing shouldn't make semantic functions disappear).
>    Discovery arm for Lsp keeps buffer source overwriting disk stale content; Typecheck under Lsp
>    only checks target file (other units only provide signatures for registry); in-file diagnostic
>    order normalization principle extends to LSP (typecheck diagnostic → W code warnings → proof
>    errors). Old behavior where LSP hard-aborts on unrelated disk file parse errors, handler
>    silently falls back to single-file path oddity is fixed with this step.
> 9. Embedded form landing (4.2.4): `Program` adds `shared_registry` field — embedded std module is
>    a sub-compilation, registry is **input** rather than product (EMBEDDED_ STAGES has no Registry
>    stage), #94's SymbolTable sharing contract thereby goes from "caller remembers to pass the same
>    registry" to explicit program declaration. Error path text from `<std.test> (embedded std)`
>    normalized to unit virtual path `<std/test>` (only compiler internal error surface, no test
>    pinned). At this point orchestrator's four entries all migrate into Driver.
> 10. Standalone check unification (4.2.5, Ruling A + IR stage ruling, 2026-10-09): Check form is a
>     faithful bearer of single-file semantics for programs without project root — warning surface
>     only covers entry file (neighboring files only collect errors, prefer missing over wrong);
>     relative `use` resolves along importer directory (rustc single-file mod alignment). CHECK
>     stage table adds GlobalSlotAlloc + IrGeneration: standalone old path (pipeline full chain)
>     already runs IR generation, E3019/E1014/E1015 etc. only produced in ir_gen (runner gate
>     evidence). IrGeneration's Check form pure check doesn't consume IR; Script/Bin sub-form
>     expressed by `module_key`'s None/Some (E3023's existing switch, `ir_gen.rs:1660`).
>     Monomorphization doesn't enter CHECK (only produces resource-overrun/internal errors, zero
>     corpus dependency). **〔2026-10-10 withdrawn by 3.4.8 evidence〕** — "zero corpus dependency"
>     is actually evidence blind spot of corpus having no pathological recursion fixture; E3005 is
>     this class of program's only compile-time defense. See note #14. Side-effect fix latent
>     defect: Registry arm's duplicate-collect registration for embedded std unit overwrites native
>     half-face merged by with_std() (E1043 false positive) — embedded unit now skips duplicate
>     registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence deleted, switched to
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, keeps incomplete AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; doesn't mark stage failure,
>     otherwise topology skip makes typecheck never run); Typecheck dispatches
>     `check_module_collect_all` free function. LSP single-file path from this point on completes
>     proof (E4018), W code warnings (W1001–W1005, test nailed) and IR-level errors; lexical failure
>     switches to real diagnostic (old synthetic "E0001 lexical error" text disappears with sequence
>     deletion — real diagnostic has precise span, no information loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 must-do
>     item): the two entries had already converged — mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single bool, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, difference carried by
>     whether `collected_errors` is empty; five collection points in statements.rs (function
>     body/use/for/block/while). Verification confirmed duplicate diagnostic defect and fixed:
>     collection point puts first error both in `collected_errors` and `Err` return channel,
>     pass-3's add_error to Err causes same code same span same message ×2 (nested can be ×3); there
>     are also two independent mechanisms: annotation check signature param and overall annotation
>     double visit (E1003/E1103), ownership layer same point double-emit (E2014/E2018). Fix landed
>     at module result boundary: deduplicate by (code, span, message) — diagnostic is positional
>     fact, duplicates carry no information; 63 corpus entries remove duplicate copies, run column
>     and exit code zero drift (the "duplicate" in `borrow_conflict_err` is a baseline triple format
>     not-including-column artifact — two E2018 are actually legal diagnostics at different
>     columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver — `Compiler` wrapper layer no longer goes through
>     it. Error text byte-by-byte aligned with old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground and SingleFile
>     share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration, this step from now has real
>     producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partially withdraws note #10's mono exclusion**). Evidence overturns "mono only produces
>     resource limit noise": pathological generic recursion (`f(x)=f([x])`) without mono on
>     multi-file path compiler process stack overflows (0xc00000fd — interpreter recurses at Rust
>     layer, no graceful runtime error), mono depth gate is the only compile-time defense;
>     standalone check before 4.2.5 took pipeline full chain already has mono, exclusion ruling
>     caused silent coverage regression ("zero corpus dependency" only proves corpus has no
>     pathological fixture). "Type erasure fallback" hypothesis voided by RFC-033 reflection ruling
>     — `^^List(Int)` needs real identity of instantiation, monomorphization is reflection's
>     foundation not optional optimization. Design four points: ① typecheck produces
>     instantiation_request for qualified call (`lib.f(x)`, FieldAccess form), generic_id uses
>     qualified name, with merged IR same namespace (currently only recognizes bare Var,
>     `expressions.rs:2455`); ② `Stage::ALL` topology changed to Linking before Monomorphization,
>     MULTI_FILE stage table tail adds Monomorphization — consumes merged_ir, aggregates all-unit
>     requests (containing_fn qualification for deferred bucket), mono elevated from single-module
>     pass to whole-program pass; single-file stage table unchanged (no Linking, subsequence
>     property preserved, IR snapshot zero drift); ③ CHECK adds Linking (pure merge — E3020
>     entry-point check stays in RoleClassification, prevent double-report) and Monomorphization
>     (pure check doesn't consume, same as IrGeneration's Check form) two stages; ④ resource
>     protection goes with whole-program BFS covering cross-unit mutual recursion. Monomorphizer
>     body unchanged (consumes ModuleIR + request set contract is exactly merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per #14 design lands qualified call instantiation
>     collection: `ExpressionInferrer` injects module-qualified key table (`use` three-arm
>     registration — normal / alias / per-item alias and only SubModule class export),
>     `callee_generic_name` via `SymbolTable::qualify` concatenation and merged IR same-source
>     qualified name; qualified call first path arity criterion tightened (when argument table can't
>     be resolved fall to second path take arguments by signature, avoid cross-module same-name
>     function mismatching false positive E3018). During implementation three **asymmetric defects**
>     on mono side and same-round fix: (1) Delete key (generic name set) and rewrite key
>     (containing_fn + name + span triple) granularity mismatch — when real args can't be resolved
>     sites degenerate to symbol request entering deferred bucket, container function non-generic
>     when bucket never drains, original was deleted by name → dangling call (`list_ops.yx:53`
>     runtime E6006 evidence); fix is restore gate `restore_generics_with_uncovered_call_sites`,
>     with `build_call_site_map` as sole source of fact to determine coverage, uncovered sites'
>     originals preserved. (2) Restore section `HashSet` iteration order causes function table order
>     non-deterministic (t2 re-review F1) → changed to `Vec` and sorted by name, product byte-level
>     reproducible. (3) Non-rewrite surface form TailCall / MakeClosure original same as before
>     looks up map key (comment claims conservative) → `call_form_is_rewritten` single point
>     determination, non-rewrite forms all counted as uncovered. Each of the three with pinned test
>     (red state evidence discriminatory power before fix). Corpus diff baseline same commit
>     regenerated just one line (remove_at frame supplements `(int64)`). **Commit strategy note**:
>     typecheck collection and mono fix split into two commits, the staging tree containing only the
>     former triggered `stdlib_docs` red on HEAD existing defect (`list.is_empty([])` same family
>     E6006) — two halves are each other's green light prerequisite, so merged into one atomic
>     commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling A** — user module
>     export carries `generic_fn_type_params_snapshot()` with `Export.type_params`, with std path
>     (`yx_sources.rs:62/108`) isomorphic: declared name becomes authoritative source for qualified
>     call monomorphization, signature reverse inference downgraded to fallback (merged into R2
>     scope). (2) **HKT declaration side live, call side broken** (infer type argument E1002 /
>     explicit type argument E1010) registered as WBS 3.5 (3 three-level tasks). (3) **deferred
>     bucket in non-generic container never drains** merged into 3.4.8 ② acceptance sub-item: bucket
>     must drain or loudly error, no silent non-specialization. (4) **Closure variable name-call
>     runtime E6006** registered as WBS 3.6; semantics (i) support indirect call / (ii) compile-time
>     reject, **ruled (i)** — support indirect call: `CallDyn` mechanism already in place
>     (`ir_gen.rs:7945` closure expression path), break is `Var` callee being downgraded to
>     name-based `Call`, and typecheck already accepts this program. (5) **Tradeoff criterion
>     solidified** — plan rejection reason can only be based on correctness and readability, "large
>     change surface / introduce new mechanism / high cost" doesn't constitute rejection reason,
>     falls into `coding-rules.md` second part and HOWTO self-check list; tradeoffs needing user
>     ruling per `AGENTS.md` "When requesting user ruling (mandatory)" give all four items
>     (situation+evidence / per-item pros and cons / clear recommendation+cost / destructive
>     naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② per #14 design lands:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm **before taking each unit's IR** aggregates whole-program requests and qualifies
>     `containing_fn` (otherwise deferred bucket key and merged IR different source, bucket never
>     drains). **① closure (D2 ruling A)**: user module export carries declaration-phase type
>     parameter snapshot with `Export::type_params`, with std path (`yx_sources.rs:62/108`)
>     isomorphic. **Existing defects empirically found and fixed during process**: (a) **F1** —
>     `qualify_names` only rewrites function body, `ir.init` is never visited, multi-file function
>     definition already qualified but module top-level call still bare name → runtime E6006
>     (**reproduces even with non-generic**, has nothing to do with generics); fix is init reuses
>     same `rename`/`aliases` and same `rewrite_call_names`. (b) mono side "whole-program unique
>     suffix" heuristic with miss silent `continue` → delete heuristic, miss becomes **loud E3018**
>     (ambiguous silent mismatch is worse than not parsing). **Regression found and fixed by
>     independent verification**: t6 found that after named import `use lib.{pick}` bare name call
>     was falsely rejected by E3018 (IR instruction name is alias qualified name, request name is
>     bare name, aggregation side only qualifies by unit key) → fix is aggregation side **directly
>     call ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as pub single
>     point, one producer one consumer), and matrix-pinned test covers two forms of named import.
>     **Filing**: init section `DefId` blank empirically proven to be "consistency blank" rather
>     than live defect (codegen name fallback resolvable, four types of init call measured pass),
>     registered as WBS 3.7.4 to close the caliber fork of "function body takes def, init relies on
>     name".
> 18. init section DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), function
>     body and `ir.init` share the same implementation and same `resolve` closure; 208 IR snapshots
>     regenerated only 309 pairs `(def=_)`→`(def=dN)` changed (other-lines 0; whole tree init
>     section 0 residual `(def=_)`). codegen's DefId main path and name fallback two states
>     translation result and execution value one-by-one consistent (four types of init call
>     measured), corpus baseline zero drift; probe flipped per new caliber and strengthened
>     (identity + precise index + execution value, fallback init covered goes red). **Erratum
>     (2026-10-11)**: above commit numbers after `git pull --rebase` rewrite — original
>     b554497b/e60dcc87/b7ac7396 in order correspond to rebase 375789d2/d922a551/38bd4b9e; pushed
>     dev history takes new numbers as standard (old numbers are local temporary numbers, didn't
>     enter remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④ design basis see #14; ② landing
>     see #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregates requests + `link_module_irs`), entry-point check entirely left to
>     `RoleClassification`; `ir_generation_check` generated then kept `us.ir` (in-memory only); mono
>     failure in Check enters collection channel, in MultiFile keeps `fail_with`; `into_outcome`
>     first `take_check_diagnostics` then judge failure (structural failure no longer clears already
>     collected diagnostics). **Verification**: four rounds of independent verification (t3/t4/t10 +
>     t5 cross-form matrix) and one round of re-review (t6) all pass; corpus diff 340/340
>     line-by-line IDENTICAL, IR snapshot zero drift, test count 2643→2656. **Existing gaps found
>     and fixed in same round**: F1 (rootless multi-file check's request name and IR definition key
>     different source → silent non-specialization, see WBS 3.4.8 row landing note) — fix is
>     aggregation side exact reverse-lookup by real definition key (non-heuristic), first version
>     fix is aggregation side exact reverse-lookup by real definition key (`flat_defs`); **same-day
>     erratum**: that bridge was already deleted in regression fix, root cause changed to
>     `module_key` one-duty-two-use decoupling (`script_mode`) to solve, three same-source as
>     `{unit key}.{name}`. This regression (CI doc example gate 258 blocks 10 fails → 0 fails)
>     location and fix see WBS 3.4.8 row landing note. **Filing (existing, not this round's
>     regression)**: E3005 help text doesn't match resource protection semantics; multi-file E3005's
>     span label misalignment (HEAD binary reproducible); request collection surface type parameter
>     position dependency (same surface falls to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + subsequent two commits). **3.9.1 (execution layer guard reachable)**: Interpreted
>     call changed to **explicit stack-driven** — `CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` push frame then hand control back to outer `step_one`, frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, when returning write back to caller slot, multi-call
>     instructions use `pending_calls` in frame to continue. Baseline measured per-layer interpreted
>     call consumes native stack **2544 B**, crash critical at `count_down(394)` (Windows main
>     thread 1 MiB); after fix 20 000 layers of recursion can run on 512 KiB stack thread (each
>     layer ~0). Guard (`push_frame`) is reachable at any entry, any `max_stack_depth` value;
>     `ExecutorError::StackOverflow` changed to carry real upper limit (original
>     `util/diagnostic/mod.rs` always passes 0). **Original 3.11.2 (deep stack backtrace truncation)
>     merged into this task per user ruling** (same file, can't be split by file): backtrace changed
>     to first 16 frames + count ellipsis + last 8 frames, shallow stack frame-by-frame unchanged.
>     **3.10.1 (compile-time ungated native recursion, #447)**: locate first then change — crash
>     point is `MonoType::type_name` recursion (**328 B/layer**, SP probe + per-operation critical
>     determined), same path also has derived `Clone` (~400 B/layer), `substitute` inner deep copy,
>     `type_name_hash` (164 B/layer), `collect_generic_type_refs`; after four places change to work
>     list and remove per-layer one full request deep copy, `mono.max_depth=5000`'s pathological
>     generic recursion produces **E3005 (with real upper limit)**, no longer stack overflows.
>     **3.11.1 (task boundary error code fidelity, #448)**: Task result error side changed from
>     value to **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), cross-boundary restore `ExecutorError` by value;
>     task-internal E6001/E6003/ E6004/E6008 in Standard mode and Embedded messages
>     character-by-character consistent. Two **pub API payload form changes** already written in
>     commit `BREAKING CHANGE`. **Commit strategy erratum**: 3.9.1 and 3.11.1 **can't be split at
>     file granularity** — `interpreter/executor/executor.rs` simultaneously carries trampoline and
>     error channel producer/consumer, `TaskFailure` defined in `runtime/engine.rs`, only staging
>     one side won't compile (pre-commit with **staging subset** to compile, measured twice
>     rejected), so per #15's same caliber merged into **one atomic commit**. **Known boundaries and
>     follow-up**: Drop derive (~75 B/layer) ⇒ `mono.max_depth ≳ 13000` may still crash; E4012
>     (constant evaluation) gate bears on native recursion (each layer ~3 696 B, default 256 layers
>     ≈ 924 KiB), currently zero call site, production unreachable, future wiring must be
>     same-family closed; Standard mode task interpreter uses default `ExecutorConfig` construct ⇒
>     its E6004 upper limit always 1024 (config propagation registered as WBS 3.12, user ruling B:
>     separate team); test runtime budget separately opened issue #449 (pathological fixture 76s,
>     clippy double target, commit runs full amount), not belonging to this RFC construction
>     surface. **Corpus directory layering (same round, 8605a081b)**: `02-type-system` (78) and
>     `06-compile-errors` (134) total 212 fixtures split into 23 category-divided subdirectories, 88
>     IR snapshots per mirror path synchronized migration; corpus baseline same commit regenerated
>     (342 lines, independent recount line-by-line compare: only `file` field and 2 new changes), 41
>     snapshots regenerated due to file header path getting longer, line-by-line check **only span
>     offset changes** (after offset-removal normalization zero diff).
> 21. LSP semantic data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data
>     produced by **the same compilation**, in-project cross-file references carry `resolves_to`
>     pointing to real definition — old form is didOpen/didChange triggering **two rounds** of
>     compilation: diagnostics go through unified Driver pipeline, semantic data goes through
>     server.rs's manual lex→parse→single-file typecheck (no module registry, in-project `use` parse
>     failure, reference events not produced, cross-file jump/rename/reference lookup silently empty
>     — D55 registered silent failure closed in this task). **Implementation**: (1) `Export` carries
>     `definition_span` (extract_module_info harvests from AST identifier node, same source same
>     value as typecheck definition event's `def_span`; native std always None), `ModuleInfo`
>     carries `definition_file` (Registry arm fills by DiscoveredFile); (2) checker drains reference
>     events via `import_def_locations` to redirect cross-file import item's `resolves_to` from use
>     line to definition file's real position (std import not registered, maintain fall to use
>     line); (3) checker module identity changed from unified `"<module>"` to unit file path (this
>     value only consumed on semantic data surface, zero diagnostic consumption — empirically
>     `env.module_name` all 4 consumer points); (4) `DriverOutcome` adds `semantic_db` channel —
>     CollectAll form (Lsp target file regardless of diagnostic success + SingleFile fallback)
>     merged in, Lsp arm tail injects non-target file top-level export definition surface from
>     registry (`external_definitions`, unopened files can also jump; types/submodule exports not
>     injected — typecheck side type registration doesn't produce definition events, two sides
>     maintain consistent gap); (5) orchestrator adds `analyze_source_in_project`
>     (`check_source_in_project` transitively called and only takes diagnostic half); (6) LSP side
>     `analysis.rs` single compilation entry replaces manual pipeline (server.rs's
>     `update_semantic_db` deleted), World key unified to canonicalized file URI (`uri.rs`:
>     `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
>     difference normalized to same key). **Along-the-way fix (D59 diversion class B — old behavior
>     silently does nothing, doesn't change any previously successful program's external
>     behavior)**: in-project file's cross-file jump/rename/reference lookup from silently empty to
>     hitting real definition; lexical error old manual pipeline deleted semantic information
>     changed to incomplete AST coloring preserved (4.2.6 editor philosophy extension); target file
>     with type error semantic data no longer discarded with `type_result` (coloring/jump in
>     intermediate typing state not disappear). **Nailed**: driver level 6 (target file path
>     key/cross-file `resolves_to`+external definition surface round trip/std import fallback use
>     line/error file preserved/single-file fallback path key+error preserved), LSP e2e 1 (first red
>     then green evidence — before fix jump returns None), uri canonicalization 4. **Refactoring
>     plan for the ten entries (per function specified)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                          |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to constructing `Program { kind: SingleFile, … }` to Driver                                                                                                                   |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                    |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                              |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtered to target file                                                                                                                                                 |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                  |
| `lib::run_file`                                            | Unchanged (transitively calls `run_project` or `SingleFile`)                                                                                                                                               |
| `lib::run_project`                                         | Unchanged (transitively calls Driver)                                                                                                                                                                      |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project switched to `Program { kind: Lsp, aggregation: CollectAll }`; single-file branch's **manual lex→parse→… sequence deleted**, switched to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                              |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% forked)**. The
shared part: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
parsing loop, per-file checker assembly, result processing. The forked part: items
2/3/4/5/7/8/9/10/11 in the 11-item table above. **This 40% shared skeleton is exactly the value
range of `Driver`** — the forked 60% is all "different stage set" or "different aggregation mode",
and these are exactly what `Program`'s two fields can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design determination: "upstream failure causes this stage to skip" does not belong to the
stage return value.**

Reason: Skip is determined by **topology**, not by the stage itself. If each stage returns `Skipped`
on its own, then the information "why didn't I run" is scattered across 12 stages and cannot be
centrally audited. Change to `dispatch` to determine at the beginning of the loop based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the bug to be fixed in this
document — the `if !proof_calls.is_empty()` at `pipeline.rs:187` is itself a silent "skip", it
produces no diagnostic, just because the obligation happens to be empty. Once the rule is
established, any "because X wasn't done so Y wasn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                              |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                               | "Stage Y not executed because upstream stage X failed"                                                                      |
| Caused by prerequisite obligation being empty            | "Stage Y not executed because no pending obligation (normal)" — **this item severity = Info, not counted in warning count** |
| Condition not met (e.g., `config.mono.enabled == false`) | "Stage Y not executed because configuration not enabled"                                                                    |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic flow, and doesn't pollute `yaoxiang check`'s `warning_count` (`diagnostic/mod.rs:637-641`
depends on the non-blocking contract of this count).

### 5. Diagnostic Aggregation Mode: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | User                                                                                                                                                                                 | Current State Mapping |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)                   | Already exists        |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` doesn't early-exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists        |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
in-project and single-file within the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the current semantic differences between the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "early-exit or not" —
they are two different checker entries. After unification, the same implementation should be driven
by the `Aggregation` parameter, rather than keeping two functions (**this is an item this document
needs to additionally verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Add `driver/` module                        | **None**. Does not introduce new type representations, does not touch `MonoType` / `PolyType` / `ir::Type`                                           |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to language type system                                                                                               |

**This document does not introduce a fourth type representation.** Convergence of the three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Ownership and layering of obligation types**: What migrates to the ledger is **settlement
responsibility**, not the ownership module of the type. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g., `ir_gen` reads `release_plan`) continue to receive specific field types via parameters,
**must not `use crate::driver`** — this aligns with the driver's non-reverse-dependency red line
below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not `use crate::driver` in reverse**.
`Driver` appearing in `TypeChecker`'s import is considered a violation, intercepted by the
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before Refactoring                       | After Refactoring                                                                                                                                                                                                                                                                                                                |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer takes this pipeline for standalone — uniformly switched to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this line now only describes `lib::run_file` / `Compiler::compile` library path) |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already patched-in at P3; **warning surface maintained without new additions** (Decision B1, 2026-10-09: dead code family warnings don't enter compile path — pool semantics being corrected by 4.9 defect, unified warning surface to wait until after 4.9; this line's draft "add warning output" voided)      |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already patched-in at P3                                                                                                                                                                                                                                                                                                         |
| LSP (in-project)                    | No proof_execution                       | Already patched-in at P3                                                                                                                                                                                                                                                                                                         |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                       |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                           |

**The only intentional behavior break** is `predicate.rs:35`'s `.expect()` changed to return a
diagnostic. This is in line with RFC-027 §8's positioning of "not bound to a specific solver", and
is also the contract already declared by `backend.rs:60` ("caller should conservatively degrade") —
the current implementation is the opposite of its own contract documentation.

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

| File                                               | Line Range                                      | Change                                                                                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib.rs`                                       | `24-36` (module declaration block)              | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                      |
| `src/lib.rs`                                       | `140-147`                                       | `run_file` changed to constructing `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                          |
| `src/lib.rs`                                       | `154-167`                                       | `run_project` changed to constructing `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `141-227`                                       | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `275-278`                                       | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                     |
| `src/frontend/pipeline.rs`                         | `306-311`                                       | `run_proof_execution` **kept**, moved into `driver` and as `Stage::ProofExecution`'s implementation                                                                                                                                                                                                                                        |
| `src/frontend/pipeline.rs`                         | `381-389`                                       | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                       |
| `src/frontend/module/orchestrator.rs`              | `99-237`                                        | `compile_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                      |
| `src/frontend/module/orchestrator.rs`              | `273-399`                                       | `check_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                        |
| `src/frontend/module/orchestrator.rs`              | `450`                                           | `check_source_in_project` switched to Driver                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `1374`                                          | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                          |
| `src/frontend/module/orchestrator.rs`              | `486-494`                                       | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to be driven by `Aggregation` parameter                                                                                                                                                                                                      |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                                       | `run_diagnostics`'s manual stage sequence deleted, switched to Driver                                                                                                                                                                                                                                                                      |
| `src/util/diagnostic/mod.rs`                       | `565-619`                                       | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                        |
| `src/util/diagnostic/mod.rs`                       | `621-661`                                       | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                            |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                                | `test_compile` / `run_code` changed to constructing `Program { kind: WasmPlayground }` (currently transitively goes to Path 1 via `Compiler::compile_with_source`)                                                                                                                                                                         |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                                         | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                      |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                                     | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix second silent discard point**)                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/checker.rs`           | `TypeChecker::ownership_inputs` criterion point | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                             |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                                          | Fix comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/layers/README.md`     | Full text                                       | Layer order table changed to **actual** execution order; delete "lower-layer failure doesn't run upper-layer" (no short-circuit exists)                                                                                                                                                                                                    |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                                         | **Per D58**: Add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs`/`checker.rs`/`ownership.rs` three consumer points switched to shared port. `&'static` raw reference form is not feasible (`dyn Solver` is not `Sync`), original text defers to D58 |

**Deleted**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s manual lex/parse sequence.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logic itself is
correct — **the problem is on the consumer end, not the producer end**, changing the producer end
would mask the architectural defect); `Cargo.toml`; any `#[cfg(target_arch = "wasm32")]` branches
(reachability determination of wasm branches goes to `06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                      | Disposition                                                                                                                                     |
| -------------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                     | **Break**: previously silently-passed constraints now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first then fix**; release in batches, expected diagnostic set changes go in CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Break**: previously-compiling programs start producing warnings  | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                           |
| `assert_drained()` first run produces W-level diagnostic | **Break**: diagnostic set added                                    | W first then E, S2/S4 split into two steps                                                                                                      |
| `predicate.rs` panic changed to diagnostic               | **Improvement**: no longer crashes                                 | No break                                                                                                                                        |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                      | These two paths don't go through proof_execution                                                                                                |
| `TypeCheckResult` fields migrated to `Obligations`       | **Internal refactoring**                                           | If `pub` API surface has external dependencies need to sync; in-repository consumers are all listed in the change list                          |

---

## Implementation Points

This document corresponds to RFC-039 global stage sequence's **P3 (fix correctness bugs)** and **P4
(stage contract and unified Driver)**. S1-S5 below are the implementation order within this
document, **each step's acceptance criterion cites the C2 category of the
[Equivalence Criteria Document](07-equivalence-oracle.md)** (orchestration change: each entry's
**diagnostic set** is the same + corpus behavior is the same).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The bug criteria must first be written to fail.

| Deliverable                                                                                                                                             | Acceptance                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                           | **Must be red**. If accidentally green, indicates this document's bug analysis needs re-review            |
| Multi-file corpus layer: New `tests/yaoxiang-multifile/` (Decision D48 — not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file two-version corpus diagnostic sets comparable                                  |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                          | Marked `#[ignore]`, S4 turns green (red state archive: after removing unique settlement point `pairs: 0`) |

**Rollback point**: No code changes, purely new tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changed to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage intentionally fixes no bugs** — it only moves existing behavior into Driver. The
acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` single commit, `pipeline.rs` restored to original is sufficient.

### S3: Merge Orchestrator Four Entries + Fix Bugs

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four entries' diagnostic set normalized by `Aggregation` is the same              |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` has consistent behavior in and out of project                    |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give the same diagnostic set for the same file                        |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Bug fixed                                                                         |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` in any mode must produce diagnostic |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in Z3-less environment                                           |

**Rollback point**: Bug fix and structural merge **into two commits**. If structural merge has
issues, only rollback structural commit, keep bug fix commit — at this time
`test_multifile_proof_obligation_not_dropped` stays green; the reverse (keep structure, rollback
fix) will turn red, is unacceptable intermediate state, forbidden to merge.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                              |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 use cases turn green (implementation form see §2 revision note; use case count based on that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                                |
| Obligation diagnostic W → E upgrade          | After full corpus diff, manual review of each new E                                                                                     |

**Rollback point**: `assert_drained()`'s severity can be controlled by configuration item, W/E
switch doesn't require code structure change.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                      | Acceptance                                                                                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                           | Documentation corresponds item-by-item with `checker.rs` call points                                                                                                  |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown doesn't enter cache** — otherwise one timeout gets固化 across compilations via process-level cache, and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call points zero (only tests retain); cache hit rate observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs`'s `TypeChecker::ownership_inputs` criterion point (ownership block `:1296-1340`, emission point `:1531`) degradation adds warning                                                                                   | Has diagnostic when no `body_checker`                                                                                                                                 |
| `orchestrator.rs` 10 wasm attributes (4 positive + 6 negative) reachability determination                                                                                                                                        | Conclusion handed to `06-cleanup-inventory.md`'s "F2 Determination" section, this document only registers determination needs                                         |

**Note**: Fixing layer order will change the diagnostic set, may expose a large amount of previously
silenced `Unproven`. **This item should proceed independently of S1-S4**, don't mix with entry merge
in the same PR.

## Key Decisions and Rationale

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                               | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **reject runtime registration**                                                                                                                                                                                                                                                                                                                                                       | Exhaustive `match` is zero-cost compile-time enforcement: when adding a new variant `dispatch` won't compile. This is isomorphic to the mechanism validated in RFC-039 routing table B by the opcode table. `StageScope` further turns "per module or project-level" into a type fact, eliminating the call-count problem that requires human reasoning                                                                                             |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used alongside `scripts/ci/check-obligations.py`'s **static** gate; **severity terminal state E level** (2026-10-13 user ruling A: same code upgraded to E, code number unchanged; **implementation form see §2 revision note**: ledger doesn't copy field structure; red list — after cleanup 0 entries — mechanism preserved, doesn't enter user-visible diagnostic stream) | What is fixed is a whole class of bugs, not one: currently verified at least 2 same-class latent defects (`proof_calls`, `checker.rs:1313`), 13 obligation fields all in range. W→E split into two steps is to make each step's C2 criterion usable — now the obligation inventory has been cleaned to 13 entries, red list zeroed, healthy state zero-emission, so terminal state directly takes E (see §2 revision note seventh, eighth sections) |
| **Fix scope**            | Fix **consumer end** (orchestration layer), **don't touch** `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branch producer end logic                                                                                                                                                                                                                                                                                             | The problem is on the consumer end, not the producer end. Changing the producer end would mask the architectural defect as "logic fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                                                                 |

This design additionally eliminates two classes of cracks:

- **The crack between "comment commitment" and "code behavior"**. `checker.rs:5165`'s "silent pass
  must not be resurrected" is a **comment-level contract**, fulfilled by `assert_drained()`.
- **"Forgot to append to an array"**. Currently five functions each manually write call sequences;
  after unification it's a single stage table, and new stage miss-wiring will report error at
  compile time.

### Directions Not Adopted

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — Of the 11 inconsistencies, 7
  (dead code analysis two implementations, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, they're a configuration question of "whether
  to run a certain analysis"; it also can't express `Aggregation`, can only move `check_module` /
  `check_module_collect_all`'s two-choice from function layer to type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when adding a new stage `dispatch` no longer fails to compile, equivalent to
  changing "five functions each manually write call" to "an array forgot to append", exactly the
  cause of the current 11 inconsistencies. This repository already has same-form counterexamples:
  `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader` described in
  `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs without touching structure** — Can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered in 5 functions, the next fork point will continue to
  grow from here. **It must be done first** (is part of S1/S3), because structural refactoring needs
  a known red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a generic entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` can't be established;
  `#[must_use]` only warns when the field is **overall** discarded. This is looking for a bug at the
  wrong level: the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After 139 lines of
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactorings.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silenced problems"**. After the fix, a batch of "new error" user reports may appear —
  they are real existing bugs, just never reported. Must be clearly stated in CHANGELOG.
- **`assert_drained()`'s severity switch needs per-field manual judgment**. Some of the obligation
  fields (e.g., `module_namespaces`) "no consumer" may be by design not needing consumption,
  shouldn't alert. S4 needs to go through each field one by one, can't take a one-size-fits-all
  approach.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable long-term,
  `stages()` will degenerate into "every call passes a different array" free parameter, contract
  constraint falls through. **Mitigation is `test_program_stage_coverage` asserting `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's bug fix criterion depends on the bug test in
  [Equivalence Criteria Document](07-equivalence-oracle.md) being red first. **If that document
  doesn't first establish multi-file corpus layer, S1 cannot be accepted** — because the existing
  293 corpus files all take the single-file path, with zero coverage of this class of defect. This
  document doesn't involve the IR verifier (`verify_loose`), that part of prerequisite work is not
  in this document's scope.
- **Layer order fix will expose previously silenced `Unproven`** (`equivalence` not in pipeline at
  all, `termination` and `ownership` order opposite to declaration). This is diagnostic set change
  risk, should proceed independently.
- **Thread safety after SMT backend changed to singleton is undetermined**. `backend.rs:13-19` has
  already stated `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to a singleton shared across compilation units, need to confirm all access paths go
  through `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as termination and ownership's actual
  order dependency). Should be opened after equivalence criteria stabilize.

> **All open questions originally listed in this section have been ruled on.** Per-item
> determination see [RFC-039 Ruling Registration](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction norms,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of three parallel type
  representations (this document doesn't introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactoring (construction steps attributed to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` bug criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  `build.rs:19-55` gate's comparison code table
- `src/frontend/core/typecheck/checker.rs:5165-5169` — Explicit declaration of "silent pass must not
  be resurrected"
- `src/frontend/core/typecheck/layers/README.md` — Layer order call table and `equivalence`'s real
  identity (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — Explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — Contract declaration of "caller should
  conservatively degrade" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 cleanup)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — Unit test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  Corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs`'s `compile_playground` (compiled via `run_code`) — Fourth compilation caller of
  playground (single-file path, therefore executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in shim crate rather than main crate
- `build.rs:19-55` — Error code build-time gate, exemplar of this project's enforcement mechanism
