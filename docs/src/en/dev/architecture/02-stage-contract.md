# Compilation Stage Contracts and Obligation Ledger

> **Supplementary design document**. This document is a supplement to
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution stage order are in the RFC-039 body;
> the positioning of each supplementary document is in the [directory index](index.md).

## Positioning and Scope

This document is the **L1 orchestration layer** construction blueprint for RFC-039. It addresses one
question: **"Which stages did this compilation run, which did it not, and why"** is currently
scattered across six paths and ten entry functions, each manually wired, with no single place that
can answer this question.

The current state can be summarized in one sentence:

> **The invariants declared by the code itself are broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally reads "Unproven → compile
error, no degradation, no silent pass… silent pass must not be resurrected." But this invariant only
holds on the **single-file path**: proof function constraints like `x: Sorted(3)` **silently pass**
on the multi-file `yaoxiang run`, `yaoxiang check`, and LSP paths, proof functions are never
executed, and no diagnostics are produced.

### Coverage

- The `Stage` enum (exhaustive, non-runtime registration) and `StageScope`;
- The `Obligations` obligation ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-function refactoring plan for the ten entry
  functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers affected by the above changes, compatibility impact, implementation
  order.

### Not Covered

- Four-layer model, dependency direction specification, anti-rebound gate → `01-routing.md`
- Equivalence criteria (C1-C6 grading, three-tier criteria) → `07-equivalence-oracle.md`
- Convergence of three parallel type representations → `03-type-unification.md`; SSA conversion →
  `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → RFC-039 body

### Division of Labor with RFC-039

RFC-039 provides **why** the refactor and **in what order**; this document provides the L1
**concrete form**, **per-file change list**, and **implementation stages within this document**
(corresponding to RFC-039 global sequence P3 "fix correctness vulnerabilities" and P4 "stage
contracts and unified Driver"). Where this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are executed per the [Equivalence Oracle document](07-equivalence-oracle.md)
**C2 (orchestration changes)** category: same diagnostic set per entry + same corpus behavior.

## Current State

> This section is entirely **verified fact**, each entry with file path + line number. Line numbers
> are based on `9e02e4db`.

### Stage Boundaries Are the Only "One Mistake, Whole Repo Silent" Structural Defect

It differs in nature from the other two defect classes (module boundaries, test wiring):

| Defect                | Typical Manifestation            | Is There a Signal?       |
| --------------------- | -------------------------------- | ------------------------ |
| Lexical/syntax error  | Source code written wrong        | Has diagnostics          |
| Type mismatch         | Type written wrong               | Has diagnostics          |
| Stage missing         | Some stage is not called at all  | **No signal whatsoever** |
| Obligation unconsumed | Field filled but no one reads it | **No signal whatsoever** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
resolved by "writing code more carefully" or "reviewing more strictly" — code review can only see
what was written, not what was not written. This is exactly the root cause RFC-039 diagnoses: "this
project treats 'design' as a documentation convention, not as an executable constraint."

### Existing Mandatory Mechanisms in This Repository

The same repository already has mature mandatory mechanisms — they just have not been extended to
the stage layer:

- **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are enforced by **build-time
  hard gates** through `build.rs:19-55` via `tools/code-tables`, which compares each entry against
  the RFC-013 code table — any inconsistency directly triggers `panic!` to refuse compilation.
- `src/package/` (**76 files / 13,012 lines**, of which the `tests/` directory is 6,227 lines /
  47.9%) has high test density, and each module's test subtree is declared wired — this is the most
  complete test wiring in the repository and can serve as a formal reference for the stage-layer
  gate.

**The design capability is sufficient. What's missing is "a same-level gate in the orchestration
layer as well."**

(Line count metric: `(Get-Content).Count`, see the "Line count metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                                  |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | -------------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                               |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                                   |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking             |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics        |
| 3c  | **LSP within project**     | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)                 |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + standalone IR            |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1                  |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                      |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | Within project goes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                      |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1           |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                        |
| ------------------- | ------------- | --------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                              |
| parsing             | `161`         | `run_parsing`                                                         |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                           |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                 |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization embedded within** (`381-389`) |

### 11 Inconsistencies in Stage Coverage

The table below provides evidence cell by cell. **A blank cell does not mean "this entry does not do
this thing", but rather "no code in this entry does this thing"** — this is the very nature of the
defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                       | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`                  |
| --- | -------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------------ |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **None**                                                | **None**                                                    | **None**                           | **None**                                   |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **None**                                                | Yes `356-377` (role-aware, no config gating)                | **None**                           | **None**                                   |
| 3   | W1006 local module shadowing                             | **None**                                         | **None**                                                | Yes `336-348`                                               | **None**                           | **None**                                   |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                          | Yes `350`                                                   | **None**                           | **None**                                   |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **None**                                                | Yes (same as 2)                                             | **None**                           | **None**                                   |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **None**                                                | N/A                                                         | N/A                                | **None**                                   |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **first-error returns** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)                    |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, drops `used_by`/`shadow_events`)     | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                        |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **None**                                         | **None**                                                | Yes (`321-328`)                                             | **None**                           | **None**                                   |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                     | **None**                                                    | **None**                           | **None**                                   |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                      | Yes (`152-236`)                                         | **None**                                                    | **None**                           | Yes (after standalone ModuleIR then merge) |

> **Review notes (WBS 3.4.3, 2026-10-07 cell-by-cell verification)**: This table is a diagnostic
> snapshot of the `9e02e4db` baseline, preserved unchanged. Status differences after P3 lands:
>
> - **Row 1 has been fixed**: proof_execution shares the same implementation across five entries via
>   `frontend/proof_execution.rs` (pipeline + orchestrator's four entries), the lone-consumer defect
>   no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still does not output W1003, does not run dead
>   code family) → categorized as WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file only) → categorized as WBS 3.4.8 (prerequisite 4.1.3,
>   potential risk unproven defect).
> - **Row 7** check_module / check_module_collect_all dual entry → categorized as 4.2.7 (Aggregation
>   parameter driven).
> - The "#434 ruling" referenced on the code side had zero registration in docs — now registered as
>   RFC-039 **D57**.
> - **Embedded std included asymmetry (new fact outside the table, registered 2026-10-09)**: the
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (required for
>   for-loop desugaring, #117 hard switch); multi-file `discover` previously only recognized
>   explicit `use` — for loops within a project compile fine, but produce E6006 at runtime (probe
>   empirical). Fixed and struck from WBS 4.10.2; the same-family "multi-file missing one
>   single-file step" parse hard-abort quirk filed as WBS 4.10.1 — **also fixed** (landed
>   immediately after 4.2.2, Check path downgraded to per-file collection; this audit's "stage
>   coverage/field consumption" dimension makes this a compilation unit membership difference, a
>   slipped-through dimension).
>
> - **Embedded std registration surface coverage (found during 4.2.5 construction, fixed)**: The
>   Registry arm uses `extract_module_info` to re-harvest registration for embedded std units, which
>   overwrites the native half already registered as "native + yx surface merged" by `with_std()`
>   (loss of `result.is_err` etc → std.test falsely reports E1043). This audit's third
>   slipped-through dimension: "the merge semantics of two registration sources for the same module
>   key" — the single-file path's registry is formed once via `with_std()`, while multi-file/Check's
>   Registry arm inserts per unit to have coverage.

**Two points need precise expression, otherwise they will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` on the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; on the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does. The reason is at
  `compile_project:138` — `type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function **has no `result.warnings` read site whatsoever**; the
  only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's entry `main` criterion is also not from the same source**: `check_project:385-395` uses
  `surfaces.bins` (manifest declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  checks "whether there is a manifest"). The two functions give different answers to "what file must
  define `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. All eight steps below are reproducible.**

**Step 1 — The obligation's single production point.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** position in the entire
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

The semantics are: "The predicate arguments of this refinement constraint are compile-time literals;
this proof function must actually be executed to determine."

**Step 2 — The consumption points are inside checker, but they only emit, they do not consume.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                | Behavior                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **No diagnostic emitted** |
| Call site argument check   | `5306` → hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |
| Return position obligation | `5420` → hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **No diagnostic emitted** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch therefore becomes a real front line rather than forward-looking padding: silent pass must not be resurrected.`

That is: the author is well aware that "not producing diagnostics" is a defect, explicitly
classifying it as a **known, nowhere-to-go intermediate state**, with the design assumption that
"someone will come to read `proof_calls`."

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects it; `checker.rs:1439` writes it to `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registration proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — There is only one read site in the entire repository.** `TypeCheckResult.proof_calls`
(`types.rs:29`) is **read in production code at only one place: `src/frontend/pipeline.rs:187`**
(`189` pass argument):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repo fall into three categories, none of
which are consumers of this field: `checker.rs:1234/4564/5494` are production-side collection;
`verdict.rs:61` is a same-name field of `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — The orchestrator's four entries all bypass that layer.**
`src/frontend/module/orchestrator.rs`'s `compile_project` (`99`), `check_project` (`273`),
`check_source_in_project` (`450`), `compile_embedded_module` (`1374`) **all do not call
`pipeline.rs`**, calling `TypeChecker::check_module` directly (`124` / `493` / `1386`). Therefore
they cannot even reach the only read site at `pipeline.rs:187`.

**Step 6 — Consequence: refinement obligations of the standard library itself also take the discard
path.** `compile_embedded_module` (`1374`, `check_module` call at `1386`) is responsible for
compiling the embedded std. This means **the proof obligations of the embedded std module itself are
likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (with
`Sorted: (x: Int) -> Type = { ... }`) on the multi-file / check / LSP paths **compiles, runs,
produces no diagnostics**. The proof function is never called.

**Step 8 (supplementary verification) — A second silent discard point exists within the layer.**
`checker.rs:1303-1314` when handling the ownership layer's result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` from the ownership check layer is swallowed by an empty match arm, with no
diagnostic, no bookkeeping.** This directly contradicts the `5164` branch's "silent pass must not be
resurrected" promise, and is **independent of the orchestrator issue** — even if the entry layer is
completely fixed, this point will still remain silent. (Handling timing: should be processed in the
same batch as the obligation ledger, because it is the same class of problem as the obligation
ledger.)

### Why Tests Did Not Catch This

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations on the multi-file path.

The three single-tests of RFC-027 (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty — for example `rfc027_return_refinement.rs:350-359` asserts the presence of a `SumUpTo`
call in `result.proof_calls`. But they **all go through `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping right before `pipeline.rs`**.

The test file's own documentation comment has already stated this.
`rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement violation cases live in .yx layer`

**This is exactly the shape of the problem: the tests verify "the obligation was filled in", but the
bug is "the consumer didn't read it".** A test that only tests the producer side, not the consumer
side, is naturally immune to this class of defect.

### A Stronger Finding: The Equivalence Oracle's Main Corpus Is the Single-File Path

The [Equivalence Oracle document](07-equivalence-oracle.md) takes the end-to-end differential of 293
`.yx` corpus files in `tests/yaoxiang/` as the **third-tier criterion**, as the primary acceptance
means for C2 stages. But empirical testing:

- The `tests/` directory has **no `yaoxiang.toml` at all** (zero hits for the whole-directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  for each corpus file (`614-616`) → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  all do not cover the multi-file path**.

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is the
empirical evidence for this. The file is annotated with `// expect: compile-error E4018` (line 15),
and its header comment on line 7 reads "Status: ❌ Should be rejected at compile time" — **it can
pass precisely because it takes the only path that actually executes the proof function**.

**Conclusion: The third-tier criterion needs a supplemental multi-file corpus layer, otherwise it
cannot serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 13 "Produced-Hence-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** whose
documentation comments explicitly state "produced by stage X → consumed by stage Y", i.e. they are
**cross-stage obligations** in nature:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Row conventions**: definition line = the field's **declaration line** in `types.rs` (the `pub` /
> `pub(super)` line), not the first line of the doc comment — a single field's comment block may
> span multiple lines, the declaration line is the only anchorable position. The **struct range** in
> the sentence above the table (`src/frontend/core/typecheck/types.rs:16-76`) floats with the
> struct's line numbers and is mechanically reconciled by gate **F5** (red on mismatch); the
> definition line column is reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**`PlanId` keyed**:
> `ir_gen.rs:360` reads (`type_result.release_plan.clone()`), `:3103` uses (`ReleasePlan::consume`);
> settlement completion marker `:1095`) | | `escaped_refs` | `40` | ownership → IR generation | |
> `instantiation_requests` | `42` | typecheck → monomorphization (`pipeline.rs:381`) | |
> `existential_coercions` | `44` | typecheck → IR generation | | `implementation_proofs` | `46` |
> typecheck → IR generation | | `sum_types` | `54` | typecheck → IR generation | |
> `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches` | `59` |
> typecheck → IR generation (span keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId` keyed** — call site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement completion marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Ruling change record (3.4.9, 2026-10-13 user verdict A)**: The original row 14
`interface_impl_registry` (`types.rs:49`) has been **reclassified as in-tree fact** — by design it
is consumed in-phase by typecheck, with zero cross-stage reads, so it is not an obligation:

- **At in-phase consumption points (live)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), seven constraint-solver queries in `inference/expressions.rs`
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), the read-only accessor at
  `checker.rs:214-217`, injection into body_checker at `checker.rs:974`.
- **Zero cross-stage reads (direct evidence)**:
  `git grep -n 'interface_impl_registry' -- src/middle` → empty; the `src/driver` side only has the
  ledger's own registration and documentation (removed in this round).
- **Disposition**: delete the doc comment **declaration** that says "mirrored for middle-end/LSP
  consumption", keep the field and in-phase consumption; this field is registered in
  `NON_OBLIGATION_FIELDS` in `src/driver/obligations.rs` (with reason). If LSP actually needs this
  table in the future, it should be re-registered under 4.6.x, **not introduce a consumer in this
  round**.

**All these fields cross layers keyed by `Span` or name tables.** The equivalence oracle document
has already pointed out that mismatches in span-keyed contracts "silently fail, with no error
whatsoever" (`ReleasePlan` is typical: any inconsistency in how the two sides compute span → `Drop`
instructions silently disappear).

Why does `release_plan` survive while `proof_calls` dies? The difference is **structural, not
coincidental**:

- `release_plan`'s consumer `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`) is
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it** — the consumer is in the downstream module, and the downstream module
  is on the inevitable path of all entries.
- `proof_calls`'s consumer is at the **top level of `pipeline.rs`**, belonging to **another entry's
  implementation**. The orchestrator's four entries don't go through `pipeline.rs` at all.

**Verified fact**: Currently the entire repository has **no mechanism whatsoever** to guarantee that
these 13 fields are consumed. The only "protection" is that `ReleasePlan` happens to ride the IR
generation's coattails.

### Non-Obligation Field Registration: Channel / In-Phase / In-Tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Fields in `TypeCheckResult` that are not cross-stage
obligations are registered one by one (field name + reason), **complementary** to the "§ Obligation
Fields" table above:

| Field                     | Why Not a Cross-Stage Obligation (Key Point)                                                                                                  |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `module_name`             | Unit identity metadata (0 hits in `src/middle`)                                                                                               |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produced-hence-contract")                                                                    |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but doc comment does not declare cross-stage contract ⇒ not included                           |
| `local_var_types`         | For diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings` — no contract declared                                             |
| `semantic_db`             | Consumer is in LSP (exported via `DriverOutcome.semantic_db` channel from 4.6.1, `src/lsp/server.rs:482`), not a compilation stage obligation |
| `trait_table`             | Cross-stage reads at `ir_gen.rs:4336/:6867`, doc comment does not declare contract                                                            |
| `warnings`                | Warning diagnostic channel (same class as `diagnostics`)                                                                                      |
| `interface_impl_registry` | In-tree fact (3.4.9 user verdict A; in-phase consumption points see "§ Obligation Fields" ruling change record)                               |

**Why not "wire up a consumer"**: Forging a downstream for these fields would only introduce false
obligations out of thin air, disguising "by design no cross-stage consumption is needed" as a
contract defect; the correct closure mechanism is **reverse containment** — `TypeCheckResult` field
set = obligation list ∪ non-obligation registration (hard-enforced by
`scripts/ci/check-obligations.py` criterion 1: add a field and forget to register, delete a field
and forget to sync, non-obligation registration missing reason, two tables overlap — all immediately
red).

### Four Additional Contract Defects in the Proof Layer

The following all come from proof layer analysis, independent of the entry forking problem, but all
belong to the class of "stage contracts not enforced."

**(a) Layer order declaration contradicts actual execution order, and the `equivalence` layer is not
in the pipeline at all.**

The layer order declared in `src/frontend/core/typecheck/layers/README.md:5-11`:

| Layer | File             | Dependencies  |
| ----- | ---------------- | ------------- |
| 0     | `equivalence.rs` | types/eval    |
| 1     | `ownership.rs`   | Layer 0       |
| 2     | `termination.rs` | Layer 0, 1    |
| 3     | `predicate.rs`   | Layer 0, 1, 2 |

README line 3 says "Execute by layer order, lower layer fails, upper layer does not run". Actual
call sites inside `TypeChecker::check_module`:

| Actual Order | Call Site                                         | Corresponding Declared Layer |
| ------------ | ------------------------------------------------- | ---------------------------- |
| 1            | `termination` — `checker.rs:1256-1270`            | Layer 2                      |
| 2            | `ownership` — `checker.rs:1296`                   | Layer 1                      |
| 3            | `predicate` — `checker.rs:5123` / `5276` / `5397` | Layer 3                      |
| —            | `equivalence` — **zero calls in `checker.rs`**    | Layer 0                      |

That is: the order of `termination` and `ownership` is **opposite** to the declaration; the declared
Layer 0 `equivalence` **does not appear at all in `check_module`'s stage sequence** (it is only used
as an `is_subtype` utility function by `inference/assignment.rs:17`, unrelated to `ProofResult`). At
the same time **no short-circuit exists whatsoever** — `checker.rs:1271-1276` after collecting
termination's errors one by one via `add_error` continues forward; the README's promise of "lower
layer fails, upper layer does not run" does not hold.

**(b) The SMT backend has three acquisition strategies, two failure philosophies.**

> **Review note (2026-10-07)**: The hard-failure panic was eliminated by P3's 3.3.1 (SOLVER slot
> Option-ized, missing falls back to `SMTResult::Unknown` conservatively); "silent skip (no
> injection)" was supplemented with the W1081 signal by 3.4.2. Unification of the three forms
> (singletonization) is executed per **RFC-039 D58**; actual fix location `proof/smt/backend.rs`
> (added process-level shared singleton + `with_shared_solver` closure port); the
> `default_solver() → &'static` statement in the 02 change list is governed by D58 (`&'static` raw
> reference is infeasible: `dyn Solver` is not `Sync`).

| Consumer                                               | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                   | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (injected via `checker.rs:1265-1268`) | Construction-time injection `with_solver_owned`          | `None` → **silently skip** (no injection)                 |
| `ownership.rs:627` (back-edge cutoff determination)    | **New on every call** `default_solver()`                 | `None => return false` (`629`) → conservatively not cut   |

The comment at `predicate.rs:31-32` explicitly opts for hard failure: "Initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint exceeds
kernel capability'". Whereas `backend.rs:60`'s contract documentation says "`None`: backend
unavailable… **the caller should degrade conservatively**". **Two philosophies coexist in the same
codebase, and the hard-failure one panics rather than returning an error.**

**(c) Every back-edge determination creates a new Z3 context; the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of back-edge cutoff determination. While
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
new Z3 context. `Z3Backend`'s cache field (`proof/smt/z3_backend.rs:20`
`cache: RefCell<HashMap<u64, SMTResult>>`) is **instance-internal**, while `z3_backend.rs:17`'s doc
comment claims "SMT query results are cached in `cache`". Not shared across calls ⇒ **this cache
never hits under this calling mode**.

(Comparison: the `LazyLock` at `predicate.rs:34-36` is indeed a singleton. Same backend, two
lifetime strategies.)

**(d) Checker silently degrades + comment disagrees with reality.** `checker.rs:1283-1287` and
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
table** and runs normally — ownership analysis degrades to "nothing conflicts", **with no warning
whatsoever**. (Processing suggestion: should be recorded as a warning-level diagnostic, or at least
left as a trace in the obligation ledger.)

The comment at `checker.rs:1244` claims termination check "runs after type check, before constraint
solving". Actual `self.env.solver().solve()` is at `checker.rs:1320` — **termination is at `1256`,
ownership at `1296` afterwards, i.e. what the comment calls "before" is actually "two layers
after"**.

### wasm Current State: Shim Crate Bearer, 27-File Branch Is Live

The wasm target **is built and constructed in CI**, with the `cdylib` not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is only `rlib`                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                                |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                                |
| Shim depends on main crate (rlib) as library      | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                       |
| Main crate has wasm target dependency section     | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                   |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                          |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpacked into `docs/src/.vitepress/public/wasm`, i.e. playground), `nightly.yml:110-114` |
| Z3 wasm static library is prebuilt by Emscripten  | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (fetch `libz3.a` from a fixed URL, downgrades to warning if missing)                                                                                                                                                            |
| **27 files** have the `wasm32` literal            | Of which **25** have actual `#[cfg(...)]` attributes, the other 2 (`frontend/module/roles.rs:9`, `std/fs.rs:10`) only mention it in documentation                                                                                                                                 |
| `orchestrator.rs` 20 occurrences                  | 12 attributes + 8 comments                                                                                                                                                                                                                                                        |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                        |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches in these 27 files are load-bearing.**
They determine which APIs in the main crate `wasm/src/lib.rs` (73 lines) can call under the wasm
target — `lib.rs:139/153/170` gate `run_file` / `run_project` / `build_bytecode` entirely (these
three need `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)` — **the single-file path**.
Therefore:

| Path that consumes `proof_calls`               | Does it execute proof function? |
| ---------------------------------------------- | ------------------------------- |
| Single-file CLI (`lib.rs:140 run_file`)        | **Yes**                         |
| wasm playground (`wasm/src/lib.rs:48`)         | **Yes**                         |
| `build_bytecode` (`lib.rs:171`)                | **Yes**                         |
| Multi-file `run` (`compile_project`)           | **No**                          |
| `check` (`check_project`)                      | **No**                          |
| LSP within project (`check_source_in_project`) | **No**                          |

That is: **the existence of `wasm/` makes "proof_execution has only one consumer" become "has
three"**, but all three are on the single-file path side. `Driver`'s `ProgramKind` must add a new
`WasmPlayground` variant (see "Target Design" 3), otherwise the unified Driver will miss this path.

(Cleanup scope — which of the 12 wasm attributes in `orchestrator.rs` are unreachable in the
playground scenario — goes to `06-cleanup-inventory.md`. The RFC-039 open question "build the wasm
target, or delete these branches" is **already obsolete**: the target is built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Non-Runtime Registration)

**Core constraint: `Stage` is a compile-time exhaustible enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer" — this is exactly the mechanism proven effective for the opcode
table (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check          Project
    Discovery,          // File discovery                    Project
    Parsing,            // Lex + syntax                      PerModule
    Registry,           // Module registry construction      Project
    RoleClassification, // Role classification (Script/Bin/…) Project
    Typecheck,          // Type check (with embedded proof)  PerModule
    DeadCodeAnalysis,   // Dead code family analysis         Project
    ProofExecution,     // Proof function compile-time exec  PerModule
    GlobalSlotAlloc,    // Global slot allocation            Project
    IrGeneration,       // AST → ModuleIR                    PerModule
    Monomorphization,   // Monomorphization                  Project
    Linking,            // Cross-module linking / IR merge   Project
}

impl Stage {
    /// All stages. Adding new variants will cause both this array and
    /// `dispatch`'s exhaustive match to fail compilation.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**Purpose of `StageScope`**: `PerModule` (run once per compilation unit) and `Project` (run once at
project level) make "which stages must run per-module, which must run once at project level" a
type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in `dispatch`
— this eliminates the current need in `orchestrator.rs` to reason by hand about "whether
`allocate_global_slots` should be called once per file".

**The stage table is ordered topologically, not alphabetically**, because failure propagation
depends on the order.

> **Revision notes (P4 implementation, 2026-10-07)**: Two deviations from the original draft,
> corrected per implementation evidence —
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR products
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), so the draft order contradicts the data
>    flow.
> 2. The Check form's stage table is normalized to dead code **before** proof per `Stage::ALL`
>    topological order: `check_project`'s current state has proof before dead code executed, the two
>    have no data dependency, the diagnostic sets are identical (C2 set semantics), only the in-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already conforms to ALL order, byte-level acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user verdict):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products — the
>    draft order would force the Registry arm to "hide parse", making the stage table lie about the
>    data flow; and multi-file parse is thus reduced from 2 times to 1. The current state is
>    honestly registered: multi-file path parse errors used to be a hard abort (`?` propagation in
>    `build_registry_from`, the tension with CollectAll semantics was filed as WBS 4.10.1) —
>    **4.10.1 has been fixed** (2026-10-09 user verdict on rust-style collection semantics + Plan
>    B): the Check path parse failure is downgraded to per-file diagnostic collection, the diseased
>    file exits the compilation unit (does not enter registry, importer reports E5001); the
>    MultiFile path retains hard abort (under FailFast a bad file cannot produce IR, which is the
>    correct semantics, pinned for the long term).
> 4. Check form landing (4.2.2, 2026-10-09 user verdict) supplements two registrations: a. Data
>    dependency edges 14→16: `RoleClassification` consumes Discovery's used_by edge set,
>    `DeadCodeAnalysis` consumes Discovery's W1006 shadowing events — the draft missed these (these
>    two products were produced by `discover_with_used` in the orchestrator era, wrapped and dropped
>    by `discover`, the data flow was lost when not entered into the table); b. The in-file
>    diagnostic order normalization is extended to the full form of note #2 above: the E3020 entry
>    validation moves into the RoleClassification arm (the check has no Linking stage, the arm
>    holding surfaces and AST is responsible for validation), the in-file diagnostic order = stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is
>    unchanged (C2 set semantics), only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: turn "field produced but not consumed" from undetectable into a compile-time or
test-time assertable fact. RFC-039 has already listed this as "the single highest-value item."

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
    /// Settlement: called once at the tail of the stage table.
    /// Each unconsumed field produces a W-level diagnostic (default) or hard error (strict mode).
    pub fn assert_drained(&self) -> Vec<Diagnostic>;
}
```

**Settlement semantics (three tiers)**:

| Situation                               | Criterion                                          | Disposition                                                                    |
| --------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------ |
| Obligation is read by declared consumer | Field is `take()`/marked consumed in `Obligations` | Pass                                                                           |
| Obligation non-empty but no consumer    | Field non-empty and not consumed                   | **Produce diagnostic** (W-level by default; `strict` mode promotes to E-level) |
| Obligation empty                        | Field is empty                                     | Pass (no consumption needed)                                                   |

**Why start with W-level rather than E-level**: Fixing obligation discard will **change the
diagnostic set**. C2 criterion requires "same diagnostic set per entry"; if we jump straight to
E-level, `yaoxiang check` will suddenly output large numbers of `Unproven` diagnostics that were
previously silent. Two steps (W observation first, then promote to E) make the criterion usable at
every step. See "Implementation Points" S4 for details.

**The call site of `assert_drained()` is unique**: `Driver::run` at the tail of the stage table,
before producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third-file read → fail). `assert_drained()` is a **runtime** gate, the script is a
**static** gate; the two are complementary and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landed)**: This note registers the
> **implementation form, measured numbers, and 4.3.2 interface** of the sketch above and the
> three-tier table, corresponding code file:line by line. Landing commit `1941cc47a` (validation
> tree `99079bc52`). It is also the **third** of the three "do not remain silent" places on the red
> list (the other two: audit surface, static gate).
>
> **I. Landed artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-entry obligation list `INVENTORY` (:201, field
>   name/consumer stage/red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (:150, 8 entries with reasons)**, delivery marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (:577), emit `emit_obligation_diagnostics` (:654), non-consumer-side gate
>   `consumer_visible_unit` (:538), skip criterion `consumer_stage_skipped` (:549), IrGeneration
>   delivery set `IR_GENERATION_DELIVERED` (:285), W1090 code string constant
>   `OBLIGATION_UNDRAINED_CODE`.
> - **Unique settlement point**: `Driver::run` at the tail of the stage table, before
>   `into_outcome()` produces `CompilationResult` — `src/driver/mod.rs:110`
>   (`State::assert_drained`, obligations.rs:570). The early-exit path of `Aggregation::FailFast` is
>   also settled (the accounts must balance).
> - **Seven delivery points** (one per arm; removing any one will make the runtime W1090 necessarily
>   fire): ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is `TypeCheckResult`'s only
>   outbound read path for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check) /
>   `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile) — four identical
>   delivery ports: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read of request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by cases.
> - Diagnostic code: W1090 registered in `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, the reason is written above the entry :26-31 — the settlement granularity
>   is "unit × obligation", with no user-pointable source location, so no dummy span is stuffed);
>   six-language copy in `locales/*.json`. **Emission level**: from 4.3.2 onward is **error** (same
>   code promoted to E, see paragraph three) — determined by the explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (obligations.rs:444), the code number remains in
>   the W family.
> - Static gate: `scripts/ci/check-obligations.py`; new CI job `obligations`
>   (`.github/workflows/ci.yml:140-155`, with `--selftest`).
>
> **II. Cell-by-cell landing of the three-tier table + two additional tiers**
>
> | Situation in the three-tier table above | Implementation Criterion                                                                  | Code                   |
> | --------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation is read by declared consumer | **Delivery marker exists** (marker ↔ real read reconciled by static gate, see section V)  | obligations.rs:618-623 |
> | Obligation non-empty but no consumer    | Non-empty + consumer stage ran + no marker → W1090 (**error level**, see paragraph three) | obligations.rs:624-630 |
> | Obligation empty                        | `Obligation::is_produced` per-field emptiness check → pass                                | obligations.rs:587-591 |
>
> Two **additional tiers** (2026-10-13 user verdict, sketch did not include):
>
> 1. **S1**: obligation's consumer stage is not in this `ProgramKind`'s stage table → **not judged
>    as owing** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (program.rs:112-119), `EMBEDDED_STAGES` has no Monomorphization
>    (:121-126).
> 2. **Skip exemption**: the consumer stage is skipped due to `SkipReason::UpstreamFailed` /
>    `ConfigDisabled` → not judged as owing (obligations.rs:598-602). `NoObligations` **does not**
>    exempt — an obligation that is non-empty while that stage declares itself to have no
>    obligations precisely means the account is unbalanced, must be reported.
>
> **III. Emission standard (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: this mechanism only emits **W-level** (W1090); **no strict
> toggle**. **4.3.2 (final state, 2026-10-13 user verdict A)**: **same code promoted to E** —
> `W1090` code number unchanged, emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code** (the same
> fact with two codes is parallel representation). **The promotion only changes the emission level,
> not the emission conditions.** **This is D59's D-class breaking change (changes externally
> observable behavior), honestly registered**: a healthy program emits zero (when all obligations
> have delivery markers), so the existing corpus and user programs are not affected (342/342 "diff:
> empty"); but once a run actually hits an undelivered obligation, that diagnostic **is counted into
> `error_count`, rendered as `error[W1090]`, with a non-zero exit code** — unlike the previous
> "W-level does not block compilation". The decoupling of code family and emission level is
> registered in RFC-013 "Emission Channels" section (the code-number prefix only determines number
> attribution, the level is determined by the explicit severity at the construction point).
> Mechanism fact (measured): severity takes explicit `.severity()`, otherwise "code starts with `W`
> → Warning" (`codes/builder.rs:186-203`); the `ErrorCategory` in the registry **does not
> participate** in severity; exit code is determined by `(error_count, warning_count)`
> (`util/diagnostic/command.rs:87`) and `--deny-warnings` (`src/main.rs:567-568`), **W1090 walks
> `error_count` because of explicit error (from 4.3.2 onward)**.
>
> **Red-list fields do not enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> the current red list has been zeroed, see section IV), for reproducible reasons:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is pre-populated by
>   typecheck's preset table and **is always non-empty**; emitting a per-compile **constant signal**
>   (no discriminative power) would instead break three existing external contracts — "clean program
>   zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing cases turned red**, of which **8 are in
>   out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; another 4 in
>   `src/driver/tests/`), 24/342 fixtures in the corpus baseline change, and the only feasible fix
>   is "filter W1090 out of the zero-diagnostic assertions" = relaxing existing criteria.
> - Ruling B (downgrade to Info level) would still break 9/12 of the list-type assertions, and Info
>   is not "output by ruling level".
> - Ruling C's measurement: existing cases **zero modifications**, corpus baseline **zero changes**
>   (342/342 "diff: empty, behaviorally equivalent"), runtime W1090 becomes a **discriminative
>   regression detector** — healthy tree is silent throughout, removing any delivery marker triggers
>   it (each of the three delivery points has been through a remove-marker mutation, CLI `check`
>   outputs 2 `warning [W1090]` when the Check arm has its marker removed).
>
> The "do not remain silent" of the red list is borne by three places: ①
> `DriverOutcome.obligations.findings` audit surface (each run carries it, with WBS id and evidence
> pointer); ② `check-obligations.py` prints the red list on each run (currently empty) + staleness
> check; ③ this note.
>
> **IV. Current state of the red list (settlement closure) and landing records for
> 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0** — all three settlement tasks landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen generates
>   CreateVariant") is equivalently fulfilled by in-place shape detection on the IR side:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST shape
>   `FieldAccess{Call(Var)/Var}` + takes declaration order from `sum_type_variants`), the call site
>   `:7496` replaces the entire Call with `CreateVariant`; the function's doc explicitly states
>   "span-keyed cross-system is unreliable (the two ends have different semantics for Call node
>   span), so detect in place" — **wiring up a consumer would reintroduce the already-fixed span
>   mismatch defect**. Deletion surface: `types.rs` field, collection in
>   `checker.rs:1489-1493/:1565`, collection chain in `inference/expressions.rs` and
>   `inference/statements.rs`, the `VariantCtorCall` structure in `environment.rs`; type inference
>   and all diagnostics of `infer_variant_ctor_call` **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (keep the in-phase candidate
>   name table and `overload_resolutions`). All read/write points: written in-phase
>   (`environment.rs:205-219` registered in same order as `method_overloads` → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase (resolution
>   point at `expressions.rs:4490-4550`, the unique hit writes to `overload_resolutions` at
>   `:4550`); the **zero-reader mirror** written into `TypeCheckResult` at `checker.rs:1558` is
>   deleted. The naming fact is still expressed at a single point: candidate name table (in-phase) +
>   span-keyed resolution (cross-stage, constructed at `ir_gen.rs:380`, consumed at `:7632/:7705`);
>   the mixed-binding name on the definition side is self-computed by `method_def_ordinals`
>   (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassified as in-tree fact** (user verdict A, see §
>   Obligation Fields table's ruling change record in this document). Delete the **declaration** in
>   the doc comment that says "mirrored for middle-end/LSP consumption" (`types.rs:47-48`), keep the
>   field and its in-phase consumption (operator query at `operator_interfaces.rs:623`, seven
>   constraint-solver queries in `inference/expressions.rs`, read-only accessor at
>   `checker.rs:214-217`); cross-stage zero reads verified by
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire up a consumer**:
>   the content of this table is the environment mirror populated by typecheck's preset table
>   (always non-empty); if LSP really needs it, it should be re-registered per 4.6.x; forging a
>   consumer for a field "by design not needing cross-stage consumption" would only introduce false
>   obligations into the ledger.
>
> After the red list is zeroed, **field enumeration is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reasons)"** — the gate's criterion 1 reverse
> containment guards this end-to-end (add a field and forget to register / delete a field and forget
> to sync both immediately go red). The red list **mechanism** is preserved: gate criterion 4's
> staleness check and `--selftest`'s `stale-red-list` fixture continue to cover its detection
> surface, and re-registration in the future must go through explicit ruling.
>
> Synchronized standard with settlement: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / § Obligation Fields table of this document); the gate's `--selftest` now
> covers seven categories of fixtures (added "unregistered field", "stale anchor", and "empty red
> list" forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Static gate (9.2) actual criteria** (three more hard criteria beyond the sketch's one
> sentence)
>
> 1. **Three-way reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`'s `INVENTORY` ↔ `TypeCheckResult` field definition ↔ this document's "§
>    Obligation Fields" table (including the table header's entry count declaration), any drift is
>    red; and `TypeCheckResult`'s field set must **be exactly equal to** "obligation list ∪
>    non-obligation registration (`NON_OBLIGATION_FIELDS`, with reasons)" — add a field and forget
>    to register, delete a field and forget to sync, non-obligation registration missing reason, two
>    tables overlap all immediately go red;
> 2. **Read-out evidence**: non-red-list obligations must have a real read in the **declared
>    consumer file** (`.field_name` member access; comments/tests/strings don't count) — this turns
>    "delivery marker ↔ downstream real read" into a hard criterion (mechanism A's main risk is
>    exactly the two drifting);
> 3. **Delivery marker**: non-red-list obligations must be marked at delivery points; the red list
>    conversely must not be marked;
> 4. **Red list staleness check**: red-list fields must have zero real reads (as soon as a consumer
>    grows, it's red), and each entry carries a WBS id and evidence pointer; Plus `--selftest`:
>    built-in negative/positive fixtures, negatives must be red, positives must be green (proves the
>    detector doesn't idle).
>
> **VI. Per-place deviations from the sketch above**
>
> 1. The sketch's `pub struct Obligations { …field… }` **does not copy by field**: the obligation
>    data is still held by `TypeCheckResult`, the ledger only records "list + delivery markers +
>    settlement conclusions". Reason: a copy would give two sources of fact (each field is produced
>    separately by typecheck's checker and the ownership layer), which would inevitably drift;
>    cross-layer data surfaces also should not be replicated to the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, replaced by `is_produced` emptiness
>    check + `DeliveryLedger` delivery marker. Reason: obligations and products are the same data,
>    `take()` would take away what the downstream needs; and the 11 IrGeneration obligations are in
>    the form of "the whole `&TypeCheckResult` is handed to ir_gen", which cannot be taken
>    per-field.
> 3. The sketch's signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: conclusion written to `DriverOutcome.obligations`,
>    diagnostics routed by `ProgramKind`: Check/Lsp take the per-file collection channel; other
>    forms route by **level** (W1090 is E-level from 4.3.2) — single-file goes to
>    `PipelineError::TypeCheck` (obligations.rs:654 outbound arm), multi-file/embedded otherwise
>    record `OrchestratorError` failure (otherwise `result.errors` has no consumer). Reason:
>    returning `Vec<Diagnostic>` would lose form routing — check's diagnostics must be attributed
>    per file.
> 4. The first tier of the three-tier table drops from "obligation is read by declared consumer" to
>    "has delivery marker"; "marker ↔ real read" is left to the static gate (section V criterion 2).
> 5. The second tier's "promote to E-level in `strict` mode" **does not go through a strict
>    toggle**, changed to **same-code promote to E in 4.3.2** (promotes even without a strict
>    toggle, because the healthy state emits zero is already safe enough, R4).
> 6. Static gate criteria upgraded from "field appears ≥2 times but no third-file read" to the four
>    in section V.
> 7. The two additional tiers (S1, skip exemption) not in the sketch are in paragraph two.
> 8. The unit test name `test_obligations_drained` mentioned in S1/S4: actually landed as **20
>    cases** in `src/driver/tests/obligations.rs` (skeleton with `#[ignore]` first → turned green;
>    red state preserved: removing the unique settlement point gives `pairs: 0`, 6 failures). S1/S4
>    rows have been synchronized with the actual file.
>
> **VII. 4.3.2 landing record (level + gate reinforcement)**
>
> - **Per-field review input**: `ObligationSettlement.drained_kinds` (the obligation kinds actually
>   drained in this run, available per run) and the five counts (paragraph two's conservation
>   equation); combined with the audit surface `findings`, it can give the fact of "whether this
>   field has been consumed by any entry".
> - **Level: same-code promote to E has landed** (2026-10-13 user verdict A; see paragraph three's
>   final state registration and D59 D-class breaking change declaration). Implementation:
>   `to_diagnostic` explicit `.severity(Severity::Error)`; outbound arm routes error channel by
>   level. Mutation evidence: removing any one delivery marker → that form's compilation fails,
>   `check` exit code is non-zero (`error [W1090]`); restoring makes the healthy state zero-emit
>   again (corpus 342/342 zero changes). locale copy in six languages has written "W family code,
>   emission level is error".
> - **Gate reinforcement items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses the **`file:line` anchors in non-obligation
>     registration reasons and red list evidence**, validating "file is locatable (unique) + line
>     number is in the page + the symbol immediately following the anchor really appears in the
>     anchored file"; an anchor pointing to a non-existent symbol goes red. Comes with negative
>     fixture `tests/baselines/obligations-gate/stale-anchor/` (symbol does not exist / line number
>     out of range, one each). Note: the red list is currently empty, so the actual landing point of
>     this criterion is **non-obligation registration** and future evidence pointers (same code
>     path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = difference", a non-empty difference is red; comes
>     with negative fixture `unregistered-field/` (new field in struct not registered).
> - **Information item F3**: `SkipReason::UpstreamFailed` exemption is unreachable at real entries
>   (units that fail typecheck don't retain `type_result`, obligations don't exist in the first
>   place); this tier's semantics is pinned by synthetic `State` cases, real end-to-end only covers
>   `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 secondary task closure (red list zeroed + obligation diagnostic final state E-level)**
>
> **8.1 Three-round landing timeline** (corresponding to code file:line; this round only supplements
> conclusions, does not modify code)
>
> | Round               | Conclusion                                                                                                                                                                           | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
> | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 ledger online | `State::assert_drained()` unique settlement point; 13-entry obligation list + delivery markers + three-tier settlement; audit surface `DriverOutcome.obligations`                    | call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; outbound `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                                              |
> | 3.4.4 settlement    | `variant_ctor_calls` delete field — declared responsibility equivalently fulfilled by in-place shape detection on the IR side                                                        | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                                          |
> | 3.4.5 settlement    | `method_overload_ir_names` delete cross-stage mirror (keep in-phase candidate name table + `overload_resolutions`)                                                                   | resolution consumer `ir_gen.rs:380/:7632/:7705`; definition side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                                              |
> | 3.4.9 settlement    | `interface_impl_registry` reclassified as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, keep in-phase consumption; obligations 14 → 13, red list 1 → 0 | declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment is at `:47-51` in the same file, rewritten by verdict A); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven places (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 leveling      | **Same-code promote to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; outbound routes error channel by level (first failure priority)                        | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code family and level decoupling registered in `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                                          |
>
> **8.2 Closure conclusion**
>
> - **Red list zeroed**: the number of entries with `red_list: Some` in `INVENTORY`
>   (`src/driver/obligations.rs:201`) = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The **mechanism is
>   preserved**: `RedListEntry` (:120), `Obligation::red_list()`, gate criterion 4's staleness check
>   and `stale-red-list` fixture; the gate outputs "red list 0 entries — settled and zeroed".
> - **Obligation diagnostic final state E-level**: `W1090`'s **code number and W-family attribution
>   are unchanged**, the emission level is `error` (D5 final state, 2026-10-13 user verdict A). This
>   **is a D59 D-class breaking change** (changes externally observable behavior), registered in
>   both paragraph three of this note and the RFC-013 "Emission Channels" section; **the healthy
>   state emits zero ⇒ the existing corpus and user programs are not affected** (corpus 342/342
>   "diff: empty", IR snapshots zero drift; t11 independent verification and t12 review each
>   re-verified).
> - **Field enumeration closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with a measured consumer surface
>   reason); gate F2 explicitly outputs the difference (currently `21 - 13 - 8 = 0`).
> - **No un-landed promises left**: all the reinforcement items registered in this note have
>   **landed** (F1/F2/F4/F5, see sections V, VII, VIII); the only two things not closed are the
>   **known boundary** in 8.4 and t7 F5 which has been turned into an issue, neither is an un-landed
>   criterion of this mechanism.
>
> **8.3 Status of disposition for each round's acceptance/review findings** (item by item)
>
> | Source        | Finding                                                                                                                                                       | Status                                                                                                                                                                                                                                                              |
> | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red list evidence is plain text pointer, goes silently stale when the alternative consumer is removed                                                         | **Landed**: gate F1 anchor validation (file locatable + line in page + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                     |
> | t3 F2 / t4 R3 | Gate criterion 1 lacks reverse containment, silently PASSes when a new field is added without registration                                                    | **Landed**: F2 difference explicit output (:577) + F4 definition line reconciliation (:455) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                       |
> | t4 R4         | W-only and promote-to-E standards not registered                                                                                                              | **Landed**: this note's third/seventh paragraphs + RFC-013 "Emission Channels" + six-language `locales/*.json` W1090 help and error_output examples                                                                                                                 |
> | t7 F5         | Dead code analysis miscounts overloaded method references (legitimate overloads are still falsely reported as `W1005`)                                        | **Turned into issue `#450`**, attributed to WBS **4.9 Reference Pool Correct Implementation** (registered in `docs/src/dev/architecture/09-execution-wbs.md:237`) — not this secondary task                                                                         |
> | t11 F7        | Gate `--root` mode F1 does not cover anchor commitments in module header prose comments                                                                       | **Known boundary** (see 8.4)                                                                                                                                                                                                                                        |
> | t11 F8        | Validator's own tool self-recorded: mutation driver once mis-wrote the main repo (immediate `git restore` cleared, affected round output discarded and rerun) | **Information item** (not a deliverable defect)                                                                                                                                                                                                                     |
> | t11 F9        | Main repo intermediate state (un-staged file makes this document's table header and `INVENTORY` out of sync) triggered gate red                               | **Information item / resolved**: frozen commit self-consistent PASS; "intermediate state committed directly will red the gate" is exactly the design intent of F2/F4                                                                                                |
> | t12 F12-1     | This document's struct range is written as `16-73`, actually `16-76` (no mechanical reconciliation after t9 added 3 comment lines)                            | **Landed + mechanized**: range corrected to `16-76`, and new gate **F5** added (range anchor, red on mismatch); similar numbers grep-aligned at the same time (`17 → 20 cases`, `16 span-keyed fields` → 13)                                                        |
> | t12 F12-2     | Multi-file form's settlement once overwrote the first failure (zzz.yx's type error was covered by W1090)                                                      | **Fixed** (captain has amended commit message to explain): `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pin case** `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registered items, not un-landed promises)**
>
> - **F1's scan surface**: anchor validation only covers the text of **registered entries** —
>   non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red list evidence;
>   **`file:line` commitments in module header prose comments are not in the scan surface**
>   (currently no registered entry is affected). If anchor commitments are written in module headers
>   in the future, **manual review must bear** the responsibility, or extend F1's scan surface —
>   pick one, do not default to no one in charge.
> - **Static boundary of the gate**: set-level validation of delivery markers is on the static side
>   (criterion 3), single-arm removal is covered by **runtime settlement** — static + runtime two
>   layers is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional verification).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization landed)**:
> Two **span-keyed contracts** in the 13 obligations above (`release_plan` / `overload_resolutions`)
> have been changed to **structural identity keys** (`PlanId`) per RFC-039 **D20**. This note
> registers the landing form, criteria, classification basis, and known boundaries; the definition
> line is still reconciled by gate **F4** (the row numbers in the table above are the current code
> fact).
>
> **I. Landing form (carrier takes A: `PlanId`)**
>
> The key is the **structural identity** that travels with the AST node, no longer relying on the
> positional coincidence of "the two sides each compute a span": `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` represents synthetic nodes
> outside the parser) is issued by the **unique allocator** `ParserState::alloc_plan_id`. Carriers
> at two places: `Stmt.plan_id` (statement identity → release plan's attribution and consumption)
> and `Expr::Call.plan_id` (call site identity → overload resolution's production and consumption);
> `Expr::call` is the unique call expression constructor in the whole repo, synthetic calls outside
> the parser are uniformly recorded as `UNPLANNED` (**cannot be consumed**, see section II).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the unique
>   consumer entry is `ReleasePlan::consume(stmt)` — **read and bookkeeping in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (the statement identity is given by `walk_stmt`'s push/pop +
>   CFG node → statement identity table; the Drop attribution for scope exit goes to the last
>   statement in that scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **same entity**),
>   produced at `inference/expressions.rs:4550` (writes by call site `PlanId`, dispatch
>   transparently passes through destructuring from `Expr::Call { plan_id }`), consumed at
>   `ir_gen.rs:7697` (with binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit
>   means bookkeeping).
>
> **II. Consumer-must-reach settlement (reusing 4.3 ledger, not creating a second set)**: the
> consumption ledger is a **single implementation** `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), each of the two tables holds one instance; when `generate_module_ir`
> returns successfully, the two ledgers are **simultaneously** marked as "completed a full round" at
> the same point (`ir_gen.rs:1095` release plan / `:1097` overload resolution); the three-state
> mapping is unique (`consumption_state` in `src/driver/obligations.rs`), and the unique settlement
> point connects one line for each of the two obligations — a non-empty difference is **E-level
> W1090** (same-code promote to E, see section III for the standard). The exemption criterion is
> hooked on the **ledger's own completion marker** (not whether `unit.ir` exists), so the `check`
> form taking away IR products does not constitute an exemption.
>
> **III. D59 classification basis (C class, red first then green)**: The form
> `Type.method(positional args, name = value)` was **silently giving the wrong answer** before the
> change (measured: `r: Int = t.pick(1, a = 2)` compiles cleanly, runtime outputs `1` instead of
> `20` — the resolved candidate was discarded, fell back to the last binding in definition order);
> after the change, the same program has compile-time `error [W1090] ... produced key not consumed`.
> The fix takes **loud** rather than "stuff the resolution name into a call with named arguments":
> the latter would lose named arguments, equivalent to swapping one silence for another (final state
> semantics see [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop` instructions are
> no-ops in the interpreter (`src/backends/interpreter/executor/ops/control.rs` wildcard arm only
> `advance()`), so the release plan's key migration does not change runtime behavior — the corpus
> 342-entry diff being empty is empirical evidence of this.
>
> **IV. Criteria and evidence**: D41 same-type criterion `test_release_plan_spans_consumed` (settled
> by `PlanId`) and the new `test_overload_resolutions_consumed` (per-file lex→parse→typecheck→ir_gen
> for corpus, **difference must be empty + evidence floor**: number of files that produced
> resolution ≥ 1, real consumption entries ≥ 2, no whitelist); the 9 resolution form coverages are
> pinned in `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return /
> positional args / type body binding / `?` operand / pure named / positional + named / ambiguous /
> variant construction / unassigned identity). Mutation driver: consumer key shifted by one ⇒
> criterion red + real program `error [W1090]` + corpus gate diff; restored and re-greens. Gate
> numbers: `check-obligations.py` PASS, corpus 342 entries diff empty, `cargo test --lib` 2712
> passed / 0 failed (IR snapshots zero drift).
>
> **V. Known boundary (registered item, not an un-landed promise)**: The **unbound consumer point**
> at `ir_gen.rs:7766-7790` is **unreachable** in the current corpus and test surface (method
> declarations uniformly go through the bound consumer point via `register_type_bindings`), so the
> key-control correctness of this branch **has no red state to falsify**; the reason for keeping it
> = it is the symmetric branch of the bound path, removing it would rewrite "method calls without
> `type_bindings`" form into silence. Registered by independent verification (t5 F-1) and review (t6
> F2), **no new task opened**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project（lib.rs:154）
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project（LSP within project, results filtered to target file）
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
}

pub enum Aggregation { FailFast, CollectAll }

pub struct Program {
    pub kind: ProgramKind,
    pub units: Vec<Unit>,
    /// Can only come from one of the 6 predefined combinations in `ProgramKind::stages()`
    pub stages: &'static [Stage],
    pub aggregation: Aggregation,
    pub config: CompileConfig,
}
```

**`stages()` can only be one of the 6 predefined combinations above (one for each `ProgramKind`), it
does not accept callers freely passing arrays.** This is a key constraint to prevent the `Program`
abstraction from degrading into "a free parameter that accepts anything", enforced by
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
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // Exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // …12 arms, not one fewer
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // Unique settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landed)**: Four implementation deviations
> from the sketch above, semantically equivalent, registered for the record —
>
> 1. Section 4's `StageOutcome` three states are not returned by the stage arm, but expressed by
>    `State`'s failure marker + `Aggregation` gating (Continue / Warn / Abort semantics unchanged,
>    eliminating per-arm boilerplate returns);
> 2. `Driver` has no `config` field — the configuration's only source is `Program.config` (section 5
>    "Avoid Driver holding mutable global state" verdict covers the sketch's field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded into the `DriverOutcome.skipped` internal
>    ledger, not emitted — S2's "deliberately fix no bug" zero-diff criterion requires this;
>    emission follows the 4.3 obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: the driver arm is the only new
>    caller (L1→L2 is an allowed direction); the orchestrator's existing call sites will be
>    considered after the 4.2 migration is complete.
> 5. Under the multi-file form, ProofExecution is an independent stage, executed **after all**
>    typechecks (A1, 2026-10-09 user verdict): the original `compile_project` interleaved proof
>    within the per-file typecheck loop (file N's proof before file N+1's typecheck). Single failure
>    both are byte-identical; "file 1 proof fails + file 2 typecheck fails" multi-failure scenario,
>    the first report changes from proof error to typecheck error (test pinboard).
> 6. `DriverOutcome` carries products by ProgramKind: `result` (pipeline contract) / `module` +
>    `failure` (orchestrator contract, 4.2.1) — the external error contracts of each entry
>    (PipelineError / OrchestratorError) are not types the Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (every discovered file has an entry, clean files have empty Vec — `check_project`'s current
>    contract); Discovery product extends to "file set + used_by edge set + shadowing events" triple
>    entering State (MultiFile form has no downstream consumer, only recorded not emitted); per-file
>    parse in the check path drops from 2 times to 1 per the C3 topology.
> 8. Lsp form landing (4.2.3, ruled by file source branching, 2026-10-09 user decision): disk file
>    parse failure follows Check's Plan B (collect + exit compilation unit — both sides downgrade
>    converge); edited buffer parse failure retains residual AST to continue typecheck (editor
>    philosophy — intermediate typing state should not make semantic features disappear). The
>    Discovery arm keeps the buffer source for Lsp to overwrite disk stale content; Typecheck under
>    Lsp only checks the target file (other units only for the registry to provide signatures);
>    in-file diagnostic order normalization principle extends to LSP (typecheck diagnostic → W code
>    warning → proof error). The old behavior's quirk of LSP hard-aborting on unrelated disk file
>    parse errors and the handler silently falling back to the single-file path is fixed by this
>    step.
> 9. Embedded form landing (4.2.4): `Program` adds a new `shared_registry` field — the embedded std
>    module is a sub-compilation, the registry is **input** rather than product (EMBEDDED_STAGES has
>    no Registry stage), the #94 SymbolTable sharing contract is thus made explicit as a program
>    declaration from the implicit "caller remembers to pass the same registry". Error path text
>    unified from `<std.test> (embedded std)` to unit virtual path `<std/test>` (only compiler
>    internal error surface, no test pinned). With this, all four orchestrator entries are migrated
>    into the Driver.
> 10. Standalone check unification (4.2.5, ruling A + IR stage ruling, 2026-10-09): the Check form's
>     faithful bearing of single-file semantics for project-rootless programs — the warning surface
>     only covers the entry file (neighboring files only collect errors, better to miss than to
>     misreport); relative `use` is resolved along the importer's directory (rustc single-file mod
>     alignment). CHECK's stage table adds GlobalSlotAlloc + IrGeneration: standalone's old path
>     (pipeline full chain) already runs IR generation, E3019/E1014/E1015 etc. are only produced in
>     ir_gen (runner gate empirical evidence). IrGeneration's Check form is pure check, does not
>     consume IR; Script/Bin split is expressed by module_key's None/Some (existing toggle for
>     E3023, `ir_gen.rs:1660`). Monomorphization is not in CHECK (only produces resource
>     overrun/internal errors, zero corpus dependency). **〔2026-10-10 withdrawn by 3.4.8 empirical
>     evidence〕** — "zero corpus dependency" was actually evidence of the corpus's blind spot for
>     pathological recursive fixtures; E3005 is that class of program's only compile-time defense.
>     See note #14. Incidental fix for latent defect: the Registry arm's repeated registration
>     harvesting for embedded std units would overwrite the native half already merged by with_std()
>     (E1043 false positive) — embedded units now skip repeated registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence deleted, replaced with
>     `Program { kind: SingleFile, aggregation: CollectAll }` — LSP and CLI use the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains residual AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; does not mark stage failure,
>     otherwise topological skip would prevent typecheck from ever being reached); Typecheck
>     dispatches `check_module_collect_all` free function. The LSP single-file path from this point
>     on fills in proof (E4018), W code warnings (W1001–W1005, test pinboard), and IR-level errors;
>     lexical failure changes to real diagnostic (the old synthetic "E0001 lexical error" text
>     disappears with the sequence deletion — real diagnostics carry precise span, no information
>     loss).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 mandatory
>     item): the two entries had already converged each — mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain two modes run together, the difference is carried
>     by whether collected_errors is empty; statements.rs five collection points (function
>     body/use/for/block/while). During verification, a duplicate diagnostic defect was confirmed
>     and fixed: the collection point puts the first error into both collected_errors and the Err
>     return channel, pass-3's add_error on Err causes the same code same span same message ×2
>     (nested can be ×3); plus two independent mechanisms: annotation validation signature formal
>     parameter double-visit with overall annotation (E1003/E1103), and the ownership layer
>     double-emitting at the same point (E2014/E2018). The fix lands at the module result boundary:
>     deduplicate by (code, span, message) — diagnostics are positional facts, duplicates carry no
>     information; 63 corpus entries have duplicate copies removed, run column and exit code zero
>     drift (borrow_conflict_err's "duplicate" is an artifact of the baseline triple format not
>     including column number — the two E2018 are actually legitimate diagnostics for different
>     columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` via `compile_playground` constructs
>     `Program { kind: WasmPlayground }` + Driver — the `Compiler` wrapper layer no longer handles
>     it. Error text byte-aligned with the old `CompileError` Display prefix (Parse error:/Type
>     error:/Internal error:), playground UI zero visible difference. WasmPlayground shares
>     SINGLE_FILE_STAGES + FailFast with SingleFile (4.1 existing declaration, real producers from
>     this step onward).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user verdict,
>     **partially withdraws note #10's mono exclusion**). Empirical evidence overturns "mono only
>     produces resource limit noise": pathological generic recursion (`f(x)=f([x])`) causes the
>     compiler process to stack overflow (0xc00000fd — the interpreter recurses at the Rust layer,
>     no graceful runtime error) on multi-file paths without mono, and the mono depth gate is the
>     only compile-time defense; the "type erasure fallback" assumption was voided by RFC-033
>     reflection ruling — `^^List(Int)` needs the real identity of the instantiated type,
>     monomorphization is the foundation of reflection, not an optional optimization. Four design
>     points: ① typecheck produces instantiation_request for qualified calls (`lib.f(x)`,
>     FieldAccess form), the generic_id uses the qualified name, sharing the namespace with merged
>     IR (currently only recognizes bare Var, expressions.rs:2455); ② `Stage::ALL` topology changes
>     to Linking before Monomorphization, MULTI_FILE stage table tail adds Monomorphization —
>     consuming merged_ir, aggregating all-unit requests (containing_fn qualification in the
>     deferred bucket), mono promotes from single-module pass to whole-program pass; single-file
>     stage table unchanged (no Linking, subsequence property preserved, IR snapshots zero drift); ③
>     CHECK adds Linking (pure merge — E3020 entry validation stays in RoleClassification,
>     preventing double-reporting) and Monomorphization (pure check, does not consume, same as
>     IrGeneration's Check form) two stages; ④ resource protection follows whole-program BFS
>     coverage for cross-unit mutual recursion. Monomorphizer itself unchanged (the contract of
>     consuming ModuleIR + request set happens to be the merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Implement qualified call instantiation collection
>     per #14 design: `ExpressionInferrer` injects the module qualification key table (`use`
>     three-arm registration — normal / alias / item-by-item alias and only SubModule class
>     exports), `callee_generic_name` is concatenated with `SymbolTable::qualify` to the same
>     qualified name as the merged IR source; qualified call first-path arity criterion tightened
>     (when the argument list cannot be resolved, fall back to second path taking actual arguments
>     by signature, avoiding cross-module same-name function mismatches that would falsely report
>     E3018). During implementation, **three asymmetric defects** on the mono side were empirically
>     confirmed and fixed in the same round: (1) delete key (generic name set) and rewrite key
>     (containing_fn + name + span triple) granularity mismatch — call sites that cannot resolve
>     actual arguments degrade to symbol requests entering the deferred bucket, when the container
>     function is non-generic the bucket never drains, the original is removed by name → dangling
>     calls (`list_ops.yx:53` runtime E6006 empirical); fix is the restore gate
>     `restore_generics_with_uncovered_call_sites`, with `build_call_site_map` as the single source
>     of fact to determine coverage, uncovered sites' originals are preserved. (2) The restore
>     segment's `HashSet` iteration order caused function table order to be non-deterministic (t2
>     review F1) → changed to `Vec` and sorted by name, output is byte-level reproducible. (3) The
>     non-rewrite surface forms TailCall / MakeClosure originally looked up map keys in the same way
>     (comment claimed conservative) → `call_form_is_rewritten` single-point judgment, non-rewrite
>     forms are uniformly counted as uncovered. Each of the three comes with a pinned test
>     (red-state empirical discrimination before fix). Corpus diff baseline regenerated with the
>     same commit only one line (remove_at frame adds `(int64)`). **Submission strategy note**: when
>     the typecheck collection and mono fix are split into two commits, the staging tree containing
>     only the former triggers `stdlib_docs`'s red on HEAD's existing defect (`list.is_empty([])`
>     same family E6006) — the two halves are each other's green prerequisite, so they are merged
>     into a single atomic commit.
> 16. 2026-10-10 five rulings registered (after R1 closure): (1) **D2 ruling A** — user module
>     exports carry `Export.type_params` with `generic_fn_type_params_snapshot()`, isomorphic to the
>     std path (`yx_sources.rs:62/108`): the declaration name becomes the authoritative source for
>     qualified call monomorphization, signature reverse-inference downgraded to fallback (merged
>     into R2 scope). (2) **HKT declaration side live, call side broken** (inferring type argument
>     E1002 / explicit type argument E1010) registered as WBS 3.5 (3 three-level tasks). (3)
>     **deferred bucket never drains under non-generic containers** merged into 3.4.8 ②'s acceptance
>     sub-item: the bucket must drain or loudly report an error, not silently not specialize. (4)
>     **Closure variable name call runtime E6006** registered as WBS 3.6; semantics (i) support
>     indirect call / (ii) compile-time rejection, **ruled (i)** — support indirect call: `CallDyn`
>     mechanism is already there (closure expression path at `ir_gen.rs:7945`), what was broken was
>     that `Var` callee was downgraded to by-name `Call`, and typecheck accepts this program in the
>     first place. (5) **Trade-off criteria solidified** — plan rejection reasons can only be based
>     on correctness and readability, "large change surface / introducing new mechanism / high cost"
>     is not a rejection reason, falls into `coding-rules.md` part two and the HOWTO self-check
>     list; trade-offs requiring user decision are given all four items per `AGENTS.md` "When
>     requesting user ruling (mandatory)" (situation+evidence / per-item pros and cons / clear
>     recommendation+cost / destructiveness explicitly named).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② lands per #14 design:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     the Linking arm aggregates the whole-program request **before taking each unit's IR** and
>     qualifies `containing_fn` (otherwise deferred bucket keys and merged IR are from different
>     sources, the bucket never drains). **① closure (D2 ruling A)**: user module exports carry the
>     declaration-phase type parameter snapshot with `Export::type_params`, isomorphic to the std
>     path (`yx_sources.rs:62/108`). **Existing defects empirically confirmed and fixed during the
>     process**: (a) **F1** — `qualify_names` only rewrites function bodies, `ir.init` is never
>     accessed, under multi-file the function definitions are already qualified while module
>     top-level calls are still bare names → runtime E6006 (**reproducible with non-generic,
>     unrelated to generics**); fix is init reusing the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) mono side's "whole-program unique suffix" heuristic with unhit
>     silent `continue` → delete the heuristic, unhit changes to **loud E3018** (silent ambiguity
>     mismatch is worse than not parsing). **Regression caught and fixed by independent
>     verification**: t6 found that after named import `use lib.{pick}`, bare-name calls were
>     falsely rejected by E3018 (IR instruction name is the alias qualified name, request name is
>     the bare name, the aggregation side only qualifies by unit key) → fix is the aggregation side
>     **directly calling ir_gen's** `import_aliases_from` and `resolve_call_target` (extracted as
>     pub single point, one place produces, one place consumes), and covered by matrix pin test
>     covering both named import forms. **On file**: init segment `DefId` blank was empirically
>     confirmed to be "consistency blank" rather than a live defect (codegen name fallback
>     resolvable, four init calls measured pass), registered as WBS 3.7.4 to close the "function
>     body goes def, init relies on name" standard fork.
> 18. init segment DefId standard fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operands + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; after 208 IR
>     snapshots regenerated, only 309 `(def=_)`→`(def=dN)` changes (other-lines 0; init segment of
>     the whole tree has 0 residual `(def=_)`). codegen's DefId main path and name fallback two
>     states' translation results and execution values are identical item by item (four init calls
>     measured), corpus baseline zero drift; probes flipped and strengthened per the new standard
>     (identity + precise index + execution value, fallback init coverage immediately red). **Errata
>     (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase` — the original
>     b554497b/e60dcc87/b7ac7396 correspond to 375789d2/d922a551/38bd4b9e after rebase in sequence;
>     the pushed dev history is based on the new numbers (the old numbers are local temporary
>     numbers, did not enter remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④ design basis see #14; ② landing
>     see #17, this entry is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregating requests + `link_module_irs`), the entry validation block is left to
>     `RoleClassification`; `ir_generation_check` retains `us.ir` (in memory only) after generation;
>     mono failure goes to collection channel in Check, keeps `fail_with` in MultiFile;
>     `into_outcome` first `take_check_diagnostics` then judges failure (structural failure no
>     longer clears already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-form matrix) and one round of review (t6) all pass; corpus
>     diff 340/340 per-line IDENTICAL, IR snapshots zero drift, test count 2643→2656. **Existing
>     gaps found and fixed in the same round**: F1 (rootless multi-file check's request name and IR
>     definition key from different sources → silently not specialize, see the landing note in WBS
>     3.4.8 row) — fix is aggregation side precise reverse lookup by real definition key
>     (non-heuristic), first version of the fix is aggregation side precise reverse lookup by real
>     definition key (`flat_defs`); **same-day errata**: this bridge has been deleted in the
>     regression fix, the root cause is now solved by `module_key`'s one-duty-two-use decoupling
>     (`script_mode`), the three are from the same source `{unit key}.{name}`. The regression's (CI
>     doc example gate 258 cases 10 failures → 0 failures) location and fix see the landing note in
>     WBS 3.4.8 row. **On file (existing, not this round's regression)**: E3005 help text is
>     inconsistent with resource protection semantics; multi-file E3005's span label is misaligned
>     (HEAD binary reproducible); the request collection surface's type parameter position
>     dependency (same surface categorized as 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + subsequent two commits). **3.9.1 (execution layer guard reachable)**: interpreter
>     calls change to **explicit stack-driven** — `CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/
>     `SpawnFromList` push a frame then return control to the outer `step_one`, the frame carries
>     `ReturnTarget{Host|CallerSlot|Discard}`, write back to the caller's slot on return, multi-call
>     instructions continue in the frame's `pending_calls`. Baseline measurement: each layer of
>     interpreter call uses native stack **2544 B**, crash threshold at `count_down(394)` (Windows
>     main thread 1 MiB); after fix, 20,000 layers of recursion can run on a 512 KiB stack thread
>     (about 0 per layer). The guard (`push_frame`) is thus reachable at any entry, any
>     `max_stack_depth` value; `ExecutorError::StackOverflow` is changed to carry the real limit
>     (the original `util/diagnostic/mod.rs` always passes 0). **The original 3.11.2 (deep stack
>     backtrace truncation) is merged into this task per user verdict** (same file, inseparable by
>     file): backtrace changed to first 16 frames + count ellipsis + last 8 frames, shallow stacks
>     unchanged frame by frame. **3.10.1 (no compile-time gate for native recursion, #447)**: locate
>     first, then change — the crash point is `MonoType::type_name` recursion (**328 B/layer**, SP
>     probe + per-operation threshold determination), the same path also has derived `Clone` (~400
>     B/layer), deep copy in `substitute`, `type_name_hash` (164 B/layer),
>     `collect_generic_type_refs`; after the four places change to work list and remove the deep
>     copy of the entire request once per layer, the pathological generic recursion with
>     `mono.max_depth=5000` produces **E3005 (with real limit)**, no longer stack overflows.
>     **3.11.1 (task boundary error code fidelity, #448)**: task result error side changed from
>     value to **structured** (`TaskResult = Result<SyncValue, TaskFailure>`,
>     `TaskOutcome::Err(TaskFailure)`), cross-boundary by-value reduction of `ExecutorError`;
>     task-internal E6001/E6003/E6004/E6008 are identical word-for-word in Standard and Embedded
>     modes. The two **pub API payload form changes** are written in the commits with
>     `BREAKING CHANGE`. **Submission strategy note**: 3.9.1 and 3.11.1 are **inseparable at the
>     file granularity** — `interpreter/executor/executor.rs` simultaneously carries the trampoline
>     and error channel's production/consumption points, `TaskFailure` is defined in
>     `runtime/engine.rs`, staging only one side will not compile (pre-commit **staging subset**
>     compilation, two refusals measured), so per the same standard as #15 they are merged into a
>     **single atomic commit**. **Known boundaries and follow-up**: Drop derive (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate is borne on native
>     recursion (each layer ~3,696 B, default 256 layers ≈ 924 KiB), currently zero call sites,
>     production unreachable, future wiring must close in the same family; Standard mode task
>     interpreter is constructed per default `ExecutorConfig` ⇒ its E6004 limit is always 1024
>     (configuration propagation registered as WBS 3.12, user verdict B: separate team); test
>     runtime budget opens issue #449 (pathological fixture 76s, clippy dual target, commit runs
>     full), not part of this RFC's construction surface. **Corpus directory layering (same round,
>     8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures split
>     into 23 category-based subdirectories, 88 IR snapshots moved in mirror path; corpus baseline
>     regenerated with the same commit (342 lines, independent recount line-by-line comparison: only
>     `file` field and 2 added changes), 41 snapshots regenerated due to file header path length,
>     line-by-line verification shows **only span offset changes** (zero difference after offset
>     normalization).
> 21. LSP semantic data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**; project-internal cross-file references carry
>     `resolves_to` pointing to the real definition — the old form was didOpen/didChange triggering
>     **two passes** of compilation: diagnostics go through the unified Driver pipeline, semantic
>     data goes through server.rs's manual lex→parse→single-file typecheck (no module registry,
>     project-internal `use` resolution fails, reference events not produced, cross-file
>     jump/rename/reference lookup silently falls through — D55's registered silent failure is
>     closed in this task). **Implementation**: (1) `Export` carries `definition_span`
>     (extract_module_info harvests from AST identifier nodes, same source and value as typecheck's
>     definition event `def_span`; native std always None), `ModuleInfo` carries `definition_file`
>     (Registry arm fills from DiscoveredFile); (2) when the checker drains reference events, it
>     uses `import_def_locations` to redirect the cross-file import items' `resolves_to` from the
>     use line to the real position of the definition file (std import has no registration, stays at
>     use line); (3) checker module identity changed from unified `"<module>"` to unit file path
>     (this value is only consumed on the semantic data surface, zero diagnostic consumers —
>     empirical `env.module_name` all 4 consumer points); (4) `DriverOutcome` adds a new
>     `semantic_db` channel — CollectAll form (Lsp target file regardless of diagnostic success +
>     SingleFile fallback) merges in, Lsp arm tail injects non-target files' top-level export
>     definition surface from the registry (`external_definitions`, unopened files can also jump;
>     types/submodule exports are not injected — typecheck side type registration does not produce
>     definition events, both sides maintain consistent gaps); (5) orchestrator adds
>     `analyze_source_in_project` (`check_source_in_project` re-routes and only takes the diagnostic
>     half); (6) LSP side `analysis.rs` single compilation entry replaces the manual pipeline
>     (server.rs's `update_semantic_db` deleted), World key unified to normalized file URI
>     (`uri.rs`: `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX
>     encoding differences normalized to the same key). **Incidental fix (D59 branching B class —
>     old behavior silently did nothing, does not change any originally successful program's
>     external performance)**: project-internal file's cross-file jump/rename/reference lookup
>     changes from silently falling through to hitting the real definition; when lexical error
>     occurs, the old manual pipeline deleting semantic information changes to residual AST tinting
>     preservation (4.2.6 editor philosophy extension); when the target file has type errors,
>     semantic data is no longer discarded along with `type_result` (tinting/jump does not disappear
>     in typing intermediate state). **Pinboard**: driver level 6 (target file path key/cross-file
>     resolves_to+external definition surface round-trip/std import fallback use line/error file
>     preservation/single-file fallback path key+error preservation), LSP e2e 1 (red-then-green
>     empirical — fix before, jump returns None).

**The refactoring of the ten entries (function-by-function specification)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                                               |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to constructing `Program { kind: SingleFile, … }` and hand to Driver                                                                                                                               |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                                         |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                                                   |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                                     |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + result filtering to target file                                                                                                                                                                     |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                                       |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (re-dispatches to `run_project` or `SingleFile`)                                                                                                                                                                      |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (re-dispatches to Driver)                                                                                                                                                                                             |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | Within project changes to `Program { kind: Lsp, aggregation: CollectAll }`; **the manual lex→parse→… sequence in the single-file branch is deleted**, re-dispatching to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                                                   |

**`check_project` and `compile_project` duplication (~40% shared / 60% forked)**. Shared: vendor
consistency check, file discovery, `build_registry_from`, `all_method_bindings`, resolution loop,
per-file checker assembly, result handling. Forked: items 2/3/4/5/7/8/9/10/11 of the 11-item table
above. **This 40% shared skeleton is exactly the value range of `Driver`** — the forked 60% are all
"different stage set" or "different aggregation mode", which is exactly what two fields of `Program`
can fully express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to skip" is not a stage return value.**

Reason: skipping is determined by **topology**, not by the stage itself. If we let each stage return
`Skipped` itself, the information "why didn't I run" would be scattered across 12 stages, impossible
to audit centrally. Change to `dispatch` determining at the start of the loop based on dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly targets the vulnerability this document
aims to fix — the current `pipeline.rs:187` `if !proof_calls.is_empty()` is a silent "skip", it
produces no diagnostic, simply because the obligation happens to be empty. After the rule is
established, any "X was not done so Y did not run" must be explainable.

Diagnostic text needs to distinguish three skip reasons:

| Reason                                                   | Text Direction                                                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Upstream `Abort` causes                                  | "Stage Y did not execute due to upstream stage X failure"                                                                            |
| Pre-condition obligation empty causes                    | "Stage Y did not execute because there are no pending obligations (normal)" — **this severity = Info, not counted in warning count** |
| Conditions not met (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because the configuration is not enabled"                                                                   |

The second category is key: it makes "normal skip" and "abnormal skip" distinguishable in the
diagnostic stream, and does not pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP paths)
}
```

| Mode         | Users                                                                                                                                                                           | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first-error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early-exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global setting of Driver** — LSP serves both
project-internal files and single-files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids Driver holding mutable global state.

**Note the current semantic difference of the two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) are not just "whether to early-exit"
differences — they are two different checker entries. After unification, the same implementation
should be driven by the `Aggregation` parameter, rather than keeping two functions (**this is an
item this document needs additional verification on, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type Layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. No new type representation is introduced, `MonoType` / `PolyType` / `ir::Type` is not touched                                              |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of three parallel
type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type ownership and layering**: what is migrated into the ledger is **settlement
responsibility**, not the type's owning module. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, the other field types stay where
they are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3
consumers (such as `ir_gen` reading `release_plan`) continue to receive specific field types via
parameters, **must not `use crate::driver`** — this is consistent with the driver's no
reverse-dependency red line below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
`Driver` appearing in `TypeChecker`'s import is considered a violation, intercepted by
`scripts/ci/check-module-boundary.py` proposed by RFC-039.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                      |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` no longer takes this pipeline for standalone — unified to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this line now only describes `lib::run_file` / `Compiler::compile` library path)                      |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution was stopped-gap connected in P3; **warning surface remains unchanged** (decision B1, 2026-10-09: dead code family warnings do not enter compile path — pool semantics is being corrected by 4.9 as defective, unified warning surface to be discussed after 4.9; the original "new warning output" in this line is voided) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Stopped-gap connected in P3                                                                                                                                                                                                                                                                                                                |
| LSP (within project)                | No proof_execution                       | Stopped-gap connected in P3                                                                                                                                                                                                                                                                                                                |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Changed to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                 |
| Z3 not installed + multi-file       | Silently skip                            | Same as above, unified                                                                                                                                                                                                                                                                                                                     |

**The only intentional behavior breakage** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This aligns with RFC-027 §8's positioning of "not bound to a specific solver", and is
the contract already declared by `backend.rs:60` ("the caller should degrade conservatively") — the
current implementation is the opposite of its own contract documentation.

### Compiler Change List

**New files (6)**

| File                        | Content                                                               |
| --------------------------- | --------------------------------------------------------------------- |
| `src/driver/mod.rs`         | `Driver`, `run()`, exhaustive `dispatch`                              |
| `src/driver/stage.rs`       | `Stage` (12 variants), `Stage::ALL`, `StageScope`                     |
| `src/driver/program.rs`     | `Program`, `ProgramKind`, `Aggregation`                               |
| `src/driver/obligations.rs` | `Obligations`, `assert_drained()`                                     |
| `src/driver/unit.rs`        | `Unit` (single-file 1 / multi-file N)                                 |
| `src/driver/diagnostics.rs` | Cross-stage diagnostic aggregation, `Skipped` diagnostic construction |

**Modified files**

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                            |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to constructing `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to constructing `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                              |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                 |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                           |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                                |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                             |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                            |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed down to Program constructor                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` reroutes to Driver                                                                                                                                                                                                                                                                                                     |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changes to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to be driven by `Aggregation` parameter                                                                                                                                                                                                            |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, re-dispatching to Driver                                                                                                                                                                                                                                                                      |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                              |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                  |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changes to constructing `Program { kind: WasmPlayground }` (currently implicitly falls to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changes to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                            |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changes to produce diagnostic (**fix the second silent discard point**)                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Correct comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/layers/README.md`     | `1-18`                             | Layer order table changed to **actual** execution order; delete "lower layer fails, upper layer does not run" (no short-circuit exists)                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs`/`checker.rs`/`ownership.rs` three consumer points reroute to shared port. `&'static` raw reference form is infeasible (`dyn Solver` is not `Sync`), the original text is governed by D58 |

**Delete**: `check_single_file` at `src/util/diagnostic/mod.rs:621-661`; manual lex/parse sequence
at `src/lsp/handlers/diagnostics.rs:161-227`.

**Not changed**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct — **the problem is on the consumer side, not the producer side**, changing
the producer would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branch (reachability determination for wasm branches goes to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                   | Compatibility                                                              | Disposition                                                                                                                                          |
| -------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                     | **Breaking**: previously silently passed constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                   | **Breaking**: programs that compiled cleanly now produce warnings          | Warnings are non-blocking (`warning_count` is counted separately, `diagnostic/mod.rs:637-641`), exit code does not change                            |
| `assert_drained()` first run produces W-level diagnostic | **Breaking**: diagnostic set adds                                          | W first, then E, S2/S4 in two steps                                                                                                                  |
| `predicate.rs` panic changes to diagnostic               | **Improvement**: no longer crashes                                         | No breaking                                                                                                                                          |
| `build` / `dump_bytecode` subcommands                    | **No impact**                                                              | These two paths do not go through proof_execution                                                                                                    |
| `TypeCheckResult` field migration to `Obligations`       | **Internal refactoring**                                                   | If `pub` API surface has external dependencies, need to synchronize; in-repo consumers are all listed in the change list                             |

---

## Implementation Points

This document corresponds to RFC-039 global stage sequence **P3 (fix correctness vulnerabilities)**
and **P4 (stage contracts and unified Driver)**. The following S1-S5 are the implementation order
within this document; **each step's acceptance criterion references the C2 category of the
[Equivalence Oracle document](07-equivalence-oracle.md)** (orchestration change: same **diagnostic
set** per entry + same corpus behavior).

### S1: Establish Criteria (Red First)

**Prerequisite, cannot be skipped.** The vulnerability criterion must be written as failing first.

| Deliverable                                                                                                                                                  | Acceptance                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                                | **Must be red**. If accidentally green, it means this document's vulnerability analysis needs to be re-reviewed |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (ruling D48 — not mixed into the single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file two-version corpus diagnostic sets are comparable                                    |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                               | Marked `#[ignore]`, S4 turns green (red state preserved: removing the unique settlement point gives `pairs: 0`) |

**Rollback point**: no code change, pure new tests.

### S2: Introduce `Stage` + `Program`, No Behavior Change

| Deliverable                                              | Acceptance (C2)                                                                               |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equal to expected array |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code byte-identical**                              |
| Whole corpus (293 `.yx`) diff                            | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff**  |

**This stage deliberately does not fix any bug** — it just moves existing behavior into the Driver.
The acceptance criterion is C1/C2 level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` returns to original state.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                   | Acceptance (C2)                                                                   |
| ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all change to Program constructor | Four-entry diagnostic sets are the same after `Aggregation` normalization         |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                     | `yaoxiang check` behaves the same inside and outside the project                  |
| LSP manual stage sequence deleted                                                                                             | LSP and CLI give the same diagnostic set for the same file                        |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                 | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fix                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changes to diagnostic                                                                        | Remove panic in Z3-not-installed environment                                      |

**Rollback point**: vulnerability fix and structural merge are **two commits**. If the structural
merge has issues, only the structural commit can be rolled back, keeping the vulnerability fix
commit — at this point `test_multifile_proof_obligation_not_dropped` remains green; the reverse
(keep the structure, roll back the fix) will turn red, which is an unacceptable intermediate state,
forbidden to merge.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 cases turn green (implementation form see §2 revision note; case count based on that file) |
| `scripts/ci/check-obligations.py`            | Static gate takes effect                                                                                                        |
| Obligation diagnostic W → E upgrade          | After whole corpus diff, manually review each added E                                                                           |

**Rollback point**: the severity of `assert_drained()` can be controlled by a configuration item,
W/E switching does not require code structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                                      | Acceptance                                                                                                                                                           |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changes to actual order                                                                                                                                                                                           | Document corresponds item-by-item with `checker.rs` call sites                                                                                                       |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown does not enter the cache** — otherwise a single timeout is solidified across compilations via the process-level cache, and pollutes the cargo test thread-shared) | Production path `default_solver()` call sites go to zero (tests only retained); cache hit rate observable across three consumer points (counter landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                                  | No `body_checker` has diagnostic                                                                                                                                     |
| Reachability determination of 12 wasm attributes in `orchestrator.rs`                                                                                                                                                                            | Conclusion goes to `06-cleanup-inventory.md`, this document only registers the determination requirement                                                             |

**Note**: correcting the layer order will change the diagnostic set, possibly exposing many
previously silenced `Unproven`. **This item should go independently of S1-S4**, not mixed with entry
merging in the same PR.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                             | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time exhaustible enum, **refuses runtime registration**                                                                                                                                                                                                                                                                                                                                                   | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` does not compile. This is isomorphic to the mechanism proven effective for the opcode table in RFC-039 routing table B. `StageScope` further makes "per-module or project-level" a type fact, eliminating the need for human reasoning about call counts                                                                                              |
| **Obligation mechanism** | `Obligations` + `assert_drained()` for **runtime** settlement, used in parallel with the **static** gate of `scripts/ci/check-obligations.py`; **final state E-level** (2026-10-13 user verdict A: same-code promote to E, code number unchanged; **implementation form see §2 revision note**: ledger does not copy field structure; red list — 0 after settlement — mechanism preserved, does not enter user-visible diagnostic stream) | What it fixes is an entire class of bugs, not one: there are at least 2 verified same-type hidden dangers (current `proof_calls`, `checker.rs:1313`), all 13 obligation fields are in range. W→E in two steps is to make the C2 criterion usable at every step — now the obligation list has been settled to 13 entries, the red list is zero, the healthy state emits zero, so the final state directly takes E (see §2 revision note paragraphs 7, 8) |
| **Fix scope**            | Fix the **consumer side** (orchestration layer), **do not touch** `checker.rs:5164/5179/5306/5318/5420/5448` six `Unproven` branch producer logics                                                                                                                                                                                                                                                                                        | The problem is on the consumer side, not the producer side. Changing the producer would mask the architectural defect as "logic is fixed", while the logic of these three branches itself is correct                                                                                                                                                                                                                                                    |

This design additionally eliminates two types of cracks:

- **The crack between "comment promise" and "code behavior"**. `checker.rs:5165`'s "silent pass must
  not be resurrected" and `layers/README.md:3`'s "lower layer fails, upper layer does not run" are
  both **comment-level contracts** — the former is fulfilled by `assert_drained()`, the latter is
  exposed by topology-driven `Skipped` diagnostic.
- **"Forgot to append to one array"**. Currently five functions each hand-write the call sequence;
  after unification it is a single stage table, and missing a new stage will fail at compile time.

### Rejected Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)** — 7 of the 11 inconsistencies
  do not involve type bridging (dead code analysis two implementations, W1006, W1003,
  monomorphization, role context, IR generation/linking), they are configuration issues of "whether
  to run a certain analysis"; it also cannot express `Aggregation`, only moving the two-choice
  between `check_module` / `check_module_collect_all` from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)** — cancels compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each hand-writing the call" with "forgot to append to one array", which
  is exactly the cause of the current 11 inconsistencies. This repository already has the same-form
  anti-pattern: `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs without touching structure** — fixes the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered in 5 functions, the next fork point will continue to
  grow from here. **It must be done first** (part of S1/S3), because structural refactoring needs a
  known red test to prove the criterion is effective.
- **Change `proof_calls` to `pub` and add `debug_assert`** — `check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **wholly** discarded. This is looking for the bug at the wrong level:
  the bug is in the orchestration layer, detection must be in the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After 139-line
  `compile_project` is split into "Program constructor + several driver arms", the reader needs to
  cross two files to understand the flow. This is the common cost of all "centralize the wiring"
  refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "expose
  previously silenced problems"**. After the fix, a batch of "new error" user reports may appear —
  they are real existing bugs, they just hadn't been reported. Must be clearly explained in
  CHANGELOG.
- **`assert_drained()`'s severity switch requires per-field manual judgment**. For some obligation
  fields (e.g. `module_namespaces`), "no consumer" may be designed to not need consumption, should
  not be reported. S4 needs to go through field by field, cannot be one-size-fits-all.
- **`Program` abstraction may be premature**. If some entry's stage set is unstable in the long
  term, `stages()` will degrade to "each call passes a different array" free parameter, contract
  constraints fall through. **Mitigation is `test_program_stage_coverage` asserting that `stages()`
  can only come from 6 predefined combinations.**
- **Criterion dependencies**: S1/S3's vulnerability fix criteria depend on the
  [Equivalence Oracle document](07-equivalence-oracle.md)'s vulnerability test being red first. **If
  that document does not first establish the multi-file corpus layer, S1 cannot be accepted** —
  because the existing 293 corpus files all go through the single-file path, with zero coverage for
  this class of defect. This document does not involve the IR verifier (`verify_loose`), that part
  of the prerequisite work is not within the scope of this document.
- **Layer order correction will expose previously silenced `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` are in the opposite order to the declaration). This
  is a diagnostic set change risk, should go independently.
- **Thread safety is undetermined after the SMT backend is changed to a singleton**.
  `backend.rs:13-19` has already stated that `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache
  is `RefCell`; after changing to a cross-compilation-unit shared singleton, need to confirm that
  all access paths go through `Mutex`.
- **Stage parallelization is not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). Should be opened after the equivalence criterion stabilizes.

> **The open questions originally listed in this section have all been ruled on.** Per-item
> determination see
> [RFC-039 Decisions Registration](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no open items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — stage table and dependency direction specification,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — convergence of three parallel type
  representations (this document does not introduce a fourth)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactoring (construction steps go to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 grading, three-tier criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — design origin of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  comparison code table for the `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — explicit declaration of "silent pass must not
  be resurrected"
- `src/frontend/core/typecheck/layers/README.md:3` — explicit declaration of "lower layer fails,
  upper layer does not run" (no short-circuit in reality)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — explicit reason for SMT initialization
  hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — contract declaration of "the caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 settlement)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — single test
  self-acknowledges "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  corpus self-acknowledges "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — the fourth compilation caller of the playground (single-file path, so it
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — error code build-time gate, exemplar of this project's mandatory mechanism
