# Compilation Stage Contracts and the Obligation Ledger

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039: Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, equivalence criterion tiers, and execution stage ordering are in the main text
> of RFC-039; the position of each subsidiary document is in the [directory index](index.md).

## Position and Scope

This document is the **L1 orchestration layer** construction drawing for RFC-039. It addresses one
question: **"Which stages ran in this compilation, which didn't, and why" is currently spread across
six paths and ten entry functions, each hand-wired, with no single place that can answer it.**

The current state can be summarized in one sentence:

> **Invariants declared by the code itself are being broken by architecture, not by logic.**

The comment at `src/frontend/core/typecheck/checker.rs:5165` literally says "Unproven → compile
error, no degradation, no silent pass... must not let silent pass return." But this invariant only
holds on the **single-file path**: proof-function constraints of the form `x: Sorted(3)` silently
pass under the three paths of multi-file `yaoxiang run`, `yaoxiang check`, and LSP—the proof
function is never executed, and no diagnostic is produced.

### Coverage

- `Stage` enum (exhaustive, not runtime-registered) and `StageScope`;
- `Obligations` ledger and `assert_drained()` settlement;
- The unified `Driver` (single `dispatch`) and the per-entry refactoring of the ten entry functions;
- Stage failure semantics (`Continue` / `Abort` / `Warn`) and diagnostic aggregation modes
  (`FailFast` / `CollectAll`);
- All files and line numbers touched by the above changes, compatibility impact, and implementation
  order.

### Not Covered

- Four-layer model, dependency direction norms, anti-rebound gates → `01-routing.md`
- Equivalence criteria (C1-C6 tiers, three-layer criteria) → `07-equivalence-oracle.md`
- Convergence of the three parallel type representations → `03-type-unification.md`; SSA conversion
  → `04-ssa.md`; frontend paradigm → `05-frontend-paradigm.md`
- Dead code and wasm branch reachability cleanup → `06-cleanup-inventory.md`
- P1-P10 global execution order and G1-G10 acceptance gates → main text of RFC-039

### Division of Labor with RFC-039

RFC-039 provides the **why** of the refactor and the **order** in which to do it; this document
provides the L1's **concrete form**, **per-file change list**, and **internal implementation
stages** (corresponding to RFC-039's global sequence of P3 "fix correctness vulnerabilities" and P4
"stage contracts and unified Driver"). Where this document conflicts with RFC-039, RFC-039 prevails.

Equivalence criteria are executed per the **C2 (orchestration change)** category of the
[equivalence criteria document](07-equivalence-oracle.md): the diagnostic set for each entry is
identical + the corpus behavior is identical.

## Current State

> This section is entirely **verified fact**, each item carrying a file path + line number. Line
> numbers are based on `9e02e4db`.

### Stage Boundaries Are the Only "Write Once, Silently Break Everywhere" Structural Defect

It differs in nature from the other two defect classes (module boundaries, test wiring):

| Defect                    | Typical Manifestation            | Has Signal?          |
| ------------------------- | -------------------------------- | -------------------- |
| Lexical/syntactic error   | Source code written wrong        | Has diagnostic       |
| Type mismatch             | Type written wrong               | Has diagnostic       |
| Stage missing from wiring | Some stage never called          | **No signal at all** |
| Obligation unconsumed     | Field filled but no one reads it | **No signal at all** |

The common feature of the latter two is: **failure produces no error**. Therefore they cannot be
solved by "writing code more carefully" or "reviewing more strictly"—code review can only see what
was written, not **what was not written**. This is exactly the root cause diagnosed by RFC-039:
"this project treats 'design' as a documentation convention rather than an executable constraint."

### Enforcement Mechanisms Already in This Repository

The same repository already has mature enforcement mechanisms; they just haven't been extended to
the stage layer:

- The **145 error codes** (137 E + 8 W) in `src/util/diagnostic/codes/` are gated at **build time**
  by `build.rs:19-55` via `tools/code-tables`, comparing each one against the RFC-013 code table,
  and any inconsistency directly `panic!`s refusing to compile.
- `src/package/` (**76 files / 13012 lines**, of which the `tests/` directory is 6227 lines, 47.9%)
  has high test density, and each module's test subtree is declared-wired—this is the most
  completely test-wired part of the repository, serving as the formal reference for stage-layer
  gates.

**The design capability is sufficient. What's missing is "putting a gate of the same caliber at the
orchestration layer too."**

(Line count metric: `(Get-Content).Count`, see the "Line count metric" section of
`06-cleanup-inventory.md`.)

### Six Compilation Paths, Ten Entry Functions

| #   | Path                       | Entry Function                                        | Location                                      | Which Compilation                              |
| --- | -------------------------- | ----------------------------------------------------- | --------------------------------------------- | ---------------------------------------------- |
| 1   | **Single-file pipeline**   | `Pipeline::run`                                       | `src/frontend/pipeline.rs:141-227`            | 5 stages direct call                           |
| 2   | **Single-file wrapper**    | `Compiler::compile` / `Compiler::compile_with_source` | `src/frontend/compiler.rs:95` / `106`         | Wraps `Pipeline`                               |
| 3a  | **Multi-file compilation** | `orchestrator::compile_project`                       | `src/frontend/module/orchestrator.rs:99-237`  | Per-file `check_module` + IR + linking         |
| 3b  | **Multi-file check**       | `orchestrator::check_project`                         | `src/frontend/module/orchestrator.rs:273-399` | Per-file typecheck, collect all diagnostics    |
| 3c  | **LSP in-project**         | `orchestrator::check_source_in_project`               | `src/frontend/module/orchestrator.rs:450`     | `check_module_collect_all` (`493`)             |
| 3d  | **Embedded std**           | `orchestrator::compile_embedded_module`               | `src/frontend/module/orchestrator.rs:1374`    | `check_module` (`1386`) + independent IR       |
| 4a  | **CLI single-file**        | `lib::run_file`                                       | `src/lib.rs:140-147`                          | → `run_with_source_name` → path 1              |
| 4b  | **CLI multi-file**         | `lib::run_project`                                    | `src/lib.rs:154-167`                          | → `compile_project` (path 3a)                  |
| 5   | **LSP / `yaoxiang check`** | `lsp::run_diagnostics`                                | `src/lsp/handlers/diagnostics.rs:146`         | In-project goes 3c, otherwise manual lex→parse |
|     |                            | `check_files_with_diagnostics`                        | `src/util/diagnostic/mod.rs:565`              | → `check_project` (`590-591`)                  |
| 6   | **wasm playground**        | `run_code` / `test_compile`                           | `wasm/src/lib.rs:42` / `30`                   | `Compiler::compile_with_source` → path 1       |

The 5 stages of the single-file path (`pipeline.rs`):

| Stage               | Call Site     | Implementation                                                       |
| ------------------- | ------------- | -------------------------------------------------------------------- |
| lexing              | `149`         | `run_lexing` (`230-239`)                                             |
| parsing             | `161`         | `run_parsing`                                                        |
| typecheck           | `173`         | `run_typecheck` (`268-285`)                                          |
| **proof_execution** | **`187-203`** | `run_proof_execution` (`306-311`) — RFC-027 Phase 2.5                |
| ir_generation       | `205`         | `run_ir_generation`, **monomorphization inlined inside** (`381-389`) |

### 11 Stage Coverage Inconsistencies

The following table provides evidence cell by cell. **A blank cell does not mean "this entry doesn't
do this", but rather "no code in this entry does this"**—which is the essence of the defect.

| #   | Stage / Behavior                                         | `pipeline` (single-file)                         | `compile_project`                                         | `check_project`                                             | `check_source_in_project` (LSP)    | `compile_embedded_module`             |
| --- | -------------------------------------------------------- | ------------------------------------------------ | --------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------- | ------------------------------------- |
| 1   | **proof_execution**                                      | **Yes** `187-203`                                | **No**                                                    | **No**                                                      | **No**                             | **No**                                |
| 2   | Dead code analysis                                       | Yes `275-278` (`config.dead_code.enabled` gated) | **No**                                                    | Yes `356-377` (role-aware, no config gate)                  | **No**                             | **No**                                |
| 3   | W1006 local module shadowing                             | **No**                                           | **No**                                                    | Yes `336-348`                                               | **No**                             | **No**                                |
| 4   | W1003 unused imports                                     | Yes (`277` collects `type_result.warnings`)      | **Collected but never output**                            | Yes `350`                                                   | **No**                             | **No**                                |
| 5   | W1001/W1002 dead code family                             | Yes (same as 2)                                  | **No**                                                    | Yes (same as 2)                                             | **No**                             | **No**                                |
| 6   | **Monomorphization**                                     | Yes `381-389` (`config.mono.enabled` gated)      | **No**                                                    | N/A                                                         | N/A                                | **No**                                |
| 7   | typecheck branch                                         | `check_module`                                   | `check_module` (`124`), **return on first error** (`132`) | `check_module_collect_all` (via `315` → `493`), collect all | `check_module_collect_all` (`493`) | `check_module` (`1386`)               |
| 8   | File discovery                                           | N/A                                              | `discover` (`101`, discards `used_by`/`shadow_events`)    | `discover_with_used` (`275`)                                | `discover` (`454`)                 | N/A                                   |
| 9   | Role context (`surfaces`/`test_rules`/`roles::classify`) | **No**                                           | **No**                                                    | Yes (`321-328`)                                             | **No**                             | **No**                                |
| 10  | Global slot allocation                                   | N/A                                              | Yes `allocate_global_slots` (`147`)                       | **No**                                                      | **No**                             | **No**                                |
| 11  | IR generation + qualified name rewrite + linking         | Yes (`205`)                                      | Yes (`152-236`)                                           | **No**                                                      | **No**                             | Yes (independent ModuleIR then merge) |

> **Re-verification note (WBS 3.4.3, 2026-10-07 cell-by-cell review)**: This table is the diagnostic
> snapshot of the `9e02e4db` baseline, retained unchanged. Post-P3 implementation differences:
>
> - **Row 1 fixed**: proof_execution's five entries share the same implementation from
>   `frontend/proof_execution.rs` (pipeline + orchestrator four entries); the only-consumer defect
>   no longer exists.
> - **Rows 2/4/5 status unchanged** (multi-file `run` still doesn't output W1003, doesn't run dead
>   code family) → WBS 3.4.6 (prerequisite 4.2.1).
> - **Row 6** (monomorphization single-file exclusive) → WBS 3.4.8 (prerequisite 4.1.3, potential
>   risk unproven defect).
> - **Row 7** check_module / check_module_collect_all dual entry → 4.2.7 (Aggregation
>   parameter-driven).
> - The "Ruling #434" referenced from the code side had zero prior docs registration—now
>   retroactively registered as RFC-039 **D57**.
> - **Embedded std inclusion asymmetry (new fact outside table, 2026-10-09 retroactive)**: The
>   single-file path unconditionally injects std.list via `merge_embedded_std_ir` (for-loop
>   desugaring requirement, #117 hard switch), while multi-file `discover` previously only
>   recognized explicit `use`—in-project for loops compile through, but at runtime produce E6006
>   (probe-validated). Fixed and ticked off WBS 4.10.2; the same-family "multi-file missing one
>   single-file step" parse-hard-abort quirk is filed as WBS 4.10.1—**also fixed** (4.2.2 landed
>   right after, the Check path degraded to per-file collection; from this audit method's "stage
>   coverage/field consumption" dimension, this is a compilation unit membership difference, a
>   slip-through dimension).
>
> - **Embedded std registry surface coverage (discovered during 4.2.5 construction, fixed)**: The
>   Registry arm re-harvests registration for embedded std units via `extract_module_info`, which
>   overwrites the native half registered via `with_std()`'s "native + yx surface merge"
>   (`result.is_err` etc. lost → std.test false-positive E1043). The third slip-through dimension of
>   this audit: "merge semantics of two registration sources for the same module key"—the
>   single-file path's registry is only formed once via `with_std()`, while multi-file/Check's
>   Registry arm does per-unit insert to gain coverage. **Two points need precise statement, or they
>   will be written wrong:**

- **The accurate conclusion for rows 4/5 is**: `yaoxiang run` under the **multi-file** path
  (`lib.rs:154 run_project` → `compile_project`) never reports W1001/W1002/W1003; under the
  **single-file** path (`lib.rs:140 run_file` → `pipeline.rs:275-278`) it does report. The reason is
  at `compile_project:138`—`type_results.push(result)` stores the complete `TypeCheckResult`
  (including `warnings`), but this function has **no `result.warnings` read site at all**
  throughout, and the only downstream use of `result` is `generate_ir_with_context` (`154-156`).
- **Row 11's `main` entry criterion is also from a different source**: `check_project:385-395` uses
  `surfaces.bins` (manifest-declared surface); `compile_project` uses `is_bin_role` (`250-252`, only
  judging "manifest presence"). The two functions give different answers to "which file must define
  `main`".

### Correctness Vulnerability: Proof Obligations Silently Discarded (Complete Evidence Chain)

**This is the core of this document. The following eight steps are all verifiable.**

**Step 1 — Single production point for obligations.**
`src/frontend/core/typecheck/layers/predicate.rs:232-239` is the **only** position in the entire
repository that constructs non-empty `proof_calls`:

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

The semantics is "the predicate argument of this refinement constraint is a compile-time literal;
the proof function must actually be executed to determine this".

**Step 2 — Consumption point is inside checker, but only emits, never consumes.**
`src/frontend/core/typecheck/checker.rs` has **three isomorphic branches** handling
`ProofResult::Unproven`:

| Branch                     | Location                                                                                                     | Behavior                |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Parameter refinement check | `5164` `if calls.is_empty()` → push hard error (`5170-5177`); `5179` `ctx.proof_calls.extend(calls.clone())` | **Emits no diagnostic** |
| Call-site argument check   | `5306` → push hard error (`5308-5316`); `5318` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |
| Return position obligation | `5420` → push hard error (`5442-5446`); `5448` `ctx.proof_calls.extend(calls)`                               | **Emits no diagnostic** |

The original comment at `checker.rs:5165-5169`:

> `RFC-027 §4/§9: Unproven → compile error, no degradation, no silent pass.` ……
> `This branch thus becomes a true defense line, not a forward-looking cushion: must not let silent pass return.`

That is: the author knew that "emitting no diagnostic" is a defect, and explicitly classified it as
a **known, nowhere-to-go intermediate state**, assuming by design that "someone will come to read
`proof_calls`".

**Step 3 — The field is indeed filled into the result.** `checker.rs:1234-1235` declares a local
`proof_calls` and collects; `checker.rs:1439` writes it into `TypeCheckResult` as
`proof_calls, // Phase 2.5 pre-registered proof function obligation`. The field is defined at
`types.rs:29`.

**Step 4 — The entire repository has only one read site.** `TypeCheckResult.proof_calls`
(`types.rs:29`) has only **one read site in production code: `src/frontend/pipeline.rs:187`** (`189`
passes it as argument):

```rust
// pipeline.rs:187-203
if !typecheck_result.type_result.proof_calls.is_empty() {
    let proof_result = self.run_proof_execution(
        &typecheck_result.type_result.proof_calls, ... );
    ...
}
```

(All other hits of the `proof_calls` identifier in the repository fall into three categories, none
of which are consumers of this field: `checker.rs:1234/4564/5494` are producer-side collection;
`verdict.rs:61` is a same-named field on `ProofResult`; `tests/rfc027_*.rs` reads `ProofResult`.)

**Step 5 — All four entries of orchestrator bypass that layer.** The `compile_project` (`99`),
`check_project` (`273`), `check_source_in_project` (`450`), and `compile_embedded_module` (`1374`)
of `src/frontend/module/orchestrator.rs` **all do not call `pipeline.rs`**, calling
`TypeChecker::check_module` directly (`124` / `493` / `1386`). So they don't even touch the only
read site at `pipeline.rs:187`.

**Step 6 — Consequence: the refinement obligations of the standard library itself also go down the
discard path.** `compile_embedded_module` (`1374`, with the `check_module` call at `1386`) is
responsible for compiling the embedded std. This means that **the proof obligations of the embedded
std module itself are likewise not executed**.

**Step 7 — Consequence: constraints silently pass.** `y: Sorted(3) = 5` (where
`Sorted: (x: Int) -> Type = { ... }`) **compiles through, runs through, and produces no diagnostic**
under the multi-file / check / LSP three paths. The proof function is never called.

**Step 8 (supplementary verification) — The layer has a second silent discard point.**
`checker.rs:1303-1314` when processing the ownership layer result:

```rust
// checker.rs:1313
ProofResult::Unproven { .. } => {}
```

**The `Unproven` of the ownership check layer is swallowed by an empty match arm, with no diagnostic
and no bookkeeping.** This directly contradicts the "must not let silent pass return" promise of the
`5164` branch, and is **independent of the orchestrator issue**—even if the entry layer is
completely fixed, this spot will still stay silent. (Handling timing: should be processed in the
same batch as the obligation ledger, because it is the same class of problem as the obligation
ledger.)

### Why Tests Didn't Catch It

**`tests/integration/multifile.rs` (726 lines / 27 `#[test]`s) has 0 hits for the three keywords
`Sorted` / `proof` / `refin`.** Zero coverage of proof obligations in the multi-file path.

RFC-027's three unit tests (`src/frontend/core/typecheck/tests/rfc027_phase25_proof_fn.rs`,
`rfc027_refined_transparency.rs`, `rfc027_return_refinement.rs`) do indeed assert that `proof_calls`
is non-empty—for example, `rfc027_return_refinement.rs:350-359` asserts the existence of a `SumUpTo`
call in `result.proof_calls`. But they **all go via `check_source` →
`checker.check_module(&module)`** (`rfc027_return_refinement.rs:45/75`,
`rfc027_refined_transparency.rs:27/57`), **stopping exactly before `pipeline.rs`**.

The test file's own documentation comment already says so. `rfc027_refined_transparency.rs:13-15`:

> `This file only asserts what check_module can see. Proof calls (E4018) are executed by pipeline.rs after check_module, so refinement-violation use cases are at the .yx layer`

**This is the shape of the problem: the test verifies "the obligation is filled", while the bug is
"the consumer side doesn't read it".** A test that only tests the producer side and not the consumer
side is naturally immune to this class of defect.

### Stronger Finding: The Subject Corpus of the Equivalence Criterion Is the Single-File Path

The [equivalence criteria document](07-equivalence-oracle.md) takes the end-to-end differential of
the 293 `.yx` corpora in `tests/yaoxiang/` as the **third-layer criterion** and the primary
acceptance means for the C2 stage. But in practice:

- There is **no `yaoxiang.toml` at all under the `tests/` directory** (zero hits by directory glob).
- Therefore `check_files_with_diagnostics` (`diagnostic/mod.rs:565`) hits the `standalone` branch
  (`614-616`) for each corpus file → `check_single_file` (`623-661`) →
  `Compiler::compile_with_source` (`635`) → `Pipeline::run` → **proof_execution executes**.
- In other words, **all 293 corpus files go through the single-file path, all cover proof_execution,
  and all don't cover the multi-file path.**

`tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx` is exactly
the empirical evidence of this point. That file is marked `// expect: compile-error E4018` (line
15), and its header comment on line 7 says "Status: ❌ should be rejected at compile time"—**it can
pass precisely because it goes down the only path that actually executes the proof function**.

**Conclusion: The third-layer criterion needs a multi-file corpus layer added; otherwise it cannot
serve as the acceptance tool for this document.** See "Implementation Points" S1.

### Obligation Fields: 13 "Produce-and-Contract" Fields, Zero Mechanism Guarantee

In `TypeCheckResult` at `src/frontend/core/typecheck/types.rs:16-76`, there are **13 fields** whose
documentation comments explicitly say "produced by stage X → consumed by stage Y", i.e., they are in
essence **cross-stage obligations**:

| Field | Definition Line | Producer → Consumer |
| ----- | --------------- | ------------------- |

> **Row convention**: Definition line = the **declaration line** of the field in `types.rs` (the
> `pub` / `pub(super)` line), not the first line of the doc comment—the comment block of the same
> field may span multiple lines, and the declaration line is the only anchorable position. The
> **struct range** in the sentence above this table (`src/frontend/core/typecheck/types.rs:16-76`)
> floats with the struct's line numbers and is mechanically reconciled by gate **F5** (writes are
> red on wrong); the definition lines are reconciled by **F4**. | `proof_calls` | `36` | typecheck →
> **proof_execution** | | `release_plan` | `38` | ownership → IR generation (**`PlanId`-keyed**:
> read at `ir_gen.rs:360` (`type_result.release_plan.clone()`), used at `:3103`
> (`ReleasePlan::consume`); settlement-complete marker `:1095`) | | `escaped_refs` | `40` |
> ownership → IR generation | | `instantiation_requests` | `42` | typecheck → monomorphization
> (`pipeline.rs:381`) | | `existential_coercions` | `44` | typecheck → IR generation | |
> `implementation_proofs` | `46` | typecheck → IR generation | | `sum_types` | `54` | typecheck → IR
> generation | | `sum_type_param_names` | `57` | typecheck → IR generation | | `operator_dispatches`
> | `59` | typecheck → IR generation (span-keyed) | | `overload_resolutions` | `62` | typecheck → IR
> generation (**`PlanId`-keyed**—call-site identity: produced at `inference/expressions.rs:4550`,
> consumed at `ir_gen.rs:7697` / `:7772` (`OverloadResolutions::get`); settlement-complete marker
> `:1097`) | | `try_expr_impls` | `65` | typecheck → IR generation (span-keyed) | |
> `match_scrutinee_types` | `69` | typecheck → IR generation (span-keyed) | | `module_namespaces` |
> `72` | typecheck → IR generation |

**Re-classification record (3.4.9, 2026-10-13 user ruling A)**: The original row 14
`interface_impl_registry` (`types.rs:49`) has been **reclassified as an in-tree fact**—by design it
is consumed in-phase by typecheck with zero cross-stage reads, and is not an obligation:

- **In-phase consumption points (live)**: `operator_interfaces.rs:623` (operator query reads
  `env.interface_impl_registry`), `inference/expressions.rs` seven constraint-solving queries
  (`:1514` / `:1646` / `:3151` / `:3876` / `:3899` / `:4400` / `:4408`), `checker.rs:214-217`
  read-only accessor, `checker.rs:974` injects body_checker.
- **Cross-stage zero reads (direct proof)**: `git grep -n 'interface_impl_registry' -- src/middle` →
  empty; on the `src/driver` side only the ledger's own registration and documentation (removed in
  this round).
- **Disposition**: Delete the **declaration** of "mirrored for middle-end/LSP consumption" from the
  doc comment, keep the field and its in-phase consumption; the field is registered in
  `NON_OBLIGATION_FIELDS` of `src/driver/obligations.rs` (with reason). If LSP really needs this
  table in the future, follow the 4.6.x new-registration path; **do not create a consumer in this
  round**.

**All these fields are passed across layers keyed by `Span` or name table.** The equivalence
criteria document has already pointed out that mismatches in the span-keyed contract "silently fail,
with no error of any kind" (`ReleasePlan` is typical: any inconsistency in how the two sides compute
the span → `Drop` instruction silently disappears).

Why does `release_plan` survive while `proof_calls` is dead? The difference is **structural, not
accidental**:

- `release_plan`'s consumer `ir_gen.rs:360` (`release_plan: type_result.release_plan.clone()`) sits
  inside `generate_ir_with_context`, and `compile_project:154-156` **happens to pass the complete
  `&TypeCheckResult` to it**—the consumer is in a downstream module, and the downstream module is on
  the must-pass path for all entries.
- `proof_calls`'s consumer is at the **top of `pipeline.rs`**, belonging to **another entry
  implementation**. The orchestrator's four entries don't even go through this layer of
  `pipeline.rs`.

**Verified fact**: currently the entire repository has **no mechanism whatsoever** that guarantees
these 13 fields are consumed. The only "protection" is that `ReleasePlan` happens to piggyback on IR
generation.

### Non-Obligation Field Registration: Channel / In-phase / In-tree Fact Fields in `TypeCheckResult`

**`NON_OBLIGATION_FIELDS` (8 entries)**: Each field in `TypeCheckResult` that is not a cross-stage
obligation is registered one by one (field name + reason), **complementary** to the "§Obligation
Fields" table above:

| Field                     | Why It Isn't a Cross-Stage Obligation (key points)                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `module_name`             | Unit identity meta-info (0 hits in `src/middle`)                                                                                           |
| `diagnostics`             | Error diagnostic channel (existing channel, not "produce-and-contract")                                                                    |
| `bindings`                | Has cross-stage reads (`ir_gen.rs:735/:5034`), but doc comment doesn't declare cross-stage contract ⇒ not incorporated                     |
| `local_var_types`         | Used for diagnostic rendering (`ir_gen.rs:852/:4543/:4794`), same as `bindings`—no declared contract                                       |
| `semantic_db`             | Consumer is in LSP (from 4.6.1 channeled out via `DriverOutcome.semantic_db`, `src/lsp/server.rs:482`), not a compilation-stage obligation |
| `trait_table`             | Cross-stage reads `ir_gen.rs:4336/:6867`, doc comment doesn't declare contract                                                             |
| `warnings`                | Warning diagnostic channel (same class as `diagnostics`)                                                                                   |
| `interface_impl_registry` | In-tree fact (3.4.9 user ruling A; in-phase consumption points see "Re-classification record" in "§Obligation Fields")                     |

**Why not "wire up a consumer"**: Creating a downstream for these fields only introduces fake
obligations, dressing up "no cross-stage consumption needed by design" as a contract defect; the
right closure mechanism is **reverse containment**—`TypeCheckResult` field set = obligation list ∪
non-obligation registration (hard-checked by `scripts/ci/check-obligations.py` criterion 1: added
field without registration, deleted field without synchronization, non-obligation registration
missing reason, two tables overlapping all turn red immediately).

### Four Additional Contract Defects in the Proof Layer

The following all come from proof layer analysis, independent of the entry forking problem, but all
fall under the category of "stage contracts not being enforced".

**(a) Layer order is only declared in docs, with no mechanism to enforce it; the `equivalence` layer
is not in the pipeline.**

`src/frontend/core/typecheck/layers/README.md` describes the actual call order of
`checker.rs::check_module_impl` (positioned by function name / driver point as anchor, line numbers
taken from the current tree):

| Actual Order | Call Point                                                                                                                                                   | Corresponding Layer |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------- |
| 1            | Refinement walkthrough driven by `predicate::check_predicate` (three sites: `revalidate_refined` / `check_call_arg_refinements` / `check_return_refinement`) | `predicate.rs`      |
| 2            | `termination::TerminationChecker::check_module`                                                                                                              | `termination.rs`    |
| 3            | `ownership::check_ownership`                                                                                                                                 | `ownership.rs`      |
| 4            | Type constraint solving `env.solver().solve()` (not in `layers/`)                                                                                            | —                   |
| —            | `equivalence::check_type_equivalence` — **zero calls in `checker.rs`**                                                                                       | `equivalence.rs`    |

Layer number labels (`equivalence.rs` = 0, `ownership.rs` = 1, `predicate.rs` = 3) only exist in
each module's own header comment, and don't correspond to the call order; there is no "number → call
timing" mapping in the README.

`equivalence` is only used by `inference/assignment.rs:17` as a utility function `is_subtype`,
unrelated to `ProofResult`. At the same time **there is no short-circuit
whatsoever**—`check_module_impl` `add_error`s termination's errors one by one and continues going
down, "lower layer fails so upper layer doesn't run" is not a fact.

**(b) The SMT backend has three acquisition strategies and two failure philosophies.**

> **Re-verification note (2026-10-07)**: The hard-failure panic was eliminated by P3's 3.3.1 (SOLVER
> slot Option-ized, missing degrades conservatively as `SMTResult::Unknown`); "silent skip (no
> injection)" gets W1081 signal from 3.4.2. Unification of the three forms (singleton-ization) is
> executed per **RFC-039 D58**, with the actual fix at `proof/smt/backend.rs` (new process-level
> shared singleton + `with_shared_solver` closure access port); the `default_solver() → &'static`
> statement in the 02 change list is governed by D58 (the bare `&'static` reference is not feasible:
> `dyn Solver` is not `Sync`).

| Consumer                                                | Acquisition Strategy                                     | When Solver Unavailable                                   |
| ------------------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------- |
| `predicate.rs:34-36`                                    | Global `static SOLVER: LazyLock<Mutex<Box<dyn Solver>>>` | **`.expect("Z3 solver initialization failed…")` → panic** |
| `termination.rs` (via `checker.rs:1265-1268` injection) | Injected at construction via `with_solver_owned`         | `None` → **silent skip** (no injection)                   |
| `ownership.rs:627` (back-edge cut determination)        | **Each call** `default_solver()`                         | `None => return false` (`629`) → conservative no-cut      |

The comment at `predicate.rs:31-32` explicitly chooses hard failure: "Initialization failure
maintains **hard failure**: softening would misdiagnose 'Z3 not installed' as 'constraint beyond
kernel capability'". Yet the contract at `backend.rs:60` says "`None`: backend unavailable…**caller
should degrade conservatively**". **Two philosophies coexist in the same code, and the hard-failure
one panics rather than returning an error.**

**(c) Each back-edge determination creates a new Z3 context, the cache is effectively useless.**
`ownership.rs:627` calls `default_solver()` on the hot path of the back-edge cut determination. Yet
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
**this cache never hits in this call mode**.

(Comparison: the `LazyLock` at `predicate.rs:34-36` is truly a singleton. The same backend, two
lifetime strategies.)

**(d) Checker's silent degradation + comment doesn't match reality.** `checker.rs:1283-1287` and
`1289-1293`:

```rust
let ledger = self.body_checker.as_ref()
    .map(|bc| bc.var_type_ledger().clone())
    .unwrap_or_default();          // body_checker is None → empty type ledger
let call_ownership = self.body_checker.as_ref()
    .map(|bc| bc.call_ownership.clone())
    .unwrap_or_default();          // body_checker is None → empty call-ownership table
```

When `body_checker` is `None`, the ownership check gets an **empty type ledger and an empty call
table** and runs as usual—ownership analysis degenerates to "nothing conflicts", **with no warning
of any kind**. (Handling suggestion: should be recorded as a warning-level diagnostic, or at least
leave a trace in the obligation ledger.)

The comment at `checker.rs:1244` says termination check "runs after type checking and before
constraint solving". But `self.env.solver().solve()` is at `checker.rs:1320`—**termination is at
`1256`, ownership at `1296` afterwards, i.e., the comment's "before" is actually "two layers
after"**.

### wasm Current State: Shim Crate Carrier, the 29-File Branch Is Live

The wasm target **is already built and built in CI**, with `cdylib` not in the main crate but in a
separate shim crate.

| Fact                                              | Evidence                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main crate is `rlib` only                         | `Cargo.toml:29-31`                                                                                                                                                                                                                                                               |
| **Shim crate provides `cdylib` + `wasm-bindgen`** | `wasm/Cargo.toml:10-11` (`crate-type = ["cdylib"]`), `wasm/Cargo.toml:18` (`wasm-bindgen = "0.2"`)                                                                                                                                                                               |
| Shim depends on main crate (rlib) as a library    | `wasm/Cargo.toml:17` `yaoxiang = { path = "..", default-features = false }`                                                                                                                                                                                                      |
| Main crate has wasm target dependencies section   | `Cargo.toml:133-134` `[target.'cfg(target_arch = "wasm32")'.dependencies]` / `web-time = "1"` (`Cargo.toml:125-131` is the corresponding non-wasm section: tokio/ureq/tempfile)                                                                                                  |
| Shim directory excluded from main workspace       | `Cargo.toml:3` `exclude = ["wasm", ...]`                                                                                                                                                                                                                                         |
| **CI has 4 wasm builds**                          | `_build-wasm.yml` (reusable workflow, `:75` `wasm-pack build --target web --out-name yaoxiang`); callers `dist-release.yml:267-273` (artifact `yaoxiang-wasm`), `docs-deploy.yml:23-56` (unpack into `docs/src/.vitepress/public/wasm`, i.e., playground), `nightly.yml:110-114` |
| Z3 wasm static library pre-built by Emscripten    | `_build-z3-wasm.yml:220`, `_build-wasm.yml:35-59` (pulls `libz3.a` from a fixed URL, degrades to warning if missing)                                                                                                                                                             |
| **29 files** have the `wasm32` literal            | Of which **22** carry actual `#[cfg(...)]` attributes, the other 7 only mention it in docs (list and per-item determination see the "F2 Determination" section of `06-cleanup-inventory.md`)                                                                                     |
| `orchestrator.rs` 16 occurrences                  | 10 attributes (4 positive + 6 negative) + 6 comments                                                                                                                                                                                                                             |
| `lib.rs` 11 occurrences                           | All attributes (`27/30/32/46/133/135/139/153/170/179/238`)                                                                                                                                                                                                                       |

**Conclusion: The `#[cfg(target_arch = "wasm32")]` branches of these 29 files are load-bearing.**
They determine which APIs of the main crate `wasm/src/lib.rs` (152 lines) can call in the wasm
target—`lib.rs:151/170/187` gates `run_file` / `run_project` / `build_bytecode` entirely (these
three all need `std::fs`), and the shim takes a different path.

**But this brings a fact directly relevant to this document**: the playground entry at
`wasm/src/lib.rs:48` calls `compiler.compile_with_source(...)`—**the single-file path**. Therefore:

| Path that consumes `proof_calls`           | Executes proof function? |
| ------------------------------------------ | ------------------------ |
| Single-file CLI (`lib.rs:140 run_file`)    | **Yes**                  |
| wasm playground (`wasm/src/lib.rs:48`)     | **Yes**                  |
| `build_bytecode` (`lib.rs:171`)            | **Yes**                  |
| Multi-file `run` (`compile_project`)       | **No**                   |
| `check` (`check_project`)                  | **No**                   |
| LSP in-project (`check_source_in_project`) | **No**                   |

That is: **the existence of `wasm/` turns "proof_execution has only one consumer" into "three
consumers"**, but all three are on the single-file path side. The `Driver`'s `ProgramKind` must add
a new `WasmPlayground` variant (see "Target Design" §3), otherwise the unified Driver will miss this
path.

(Cleanup scope—among the 10 wasm attributes in `orchestrator.rs` (4 positive + 6 negative), which
are unreachable in playground scenarios—goes to the "F2 Determination" section of
`06-cleanup-inventory.md`: 4 wasm arms are unreachable at playground runtime but load-bearing at
compile time, 6 `not(wasm32)` arms are load-bearing on the native three-stage table, no low-risk
items can be directly deleted. The open issue in RFC-039 "build wasm target, or delete these
branches" is **already outdated**: the target has been built.)

---

## Target Design

### 1. Stage Model: `Stage` Enum (Exhaustive, Not Runtime-Registered)

**Core constraint: `Stage` is a compile-time-exhaustive enum; runtime registration is forbidden.**
The reason is that Rust's exhaustive `match` can enforce at compile time that "new stages must be
handled by the orchestration layer"—exactly the mechanism the opcode table has already proven
effective (RFC-039 routing table B).

```rust
// src/driver/stage.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Stage {
    VendorConsistency,  // Vendor consistency check        Project
    Discovery,          // File discovery                  Project
    Parsing,            // Lexical + syntactic             PerModule
    Registry,           // Module registry construction    Project
    RoleClassification, // Role classification (Script/Bin/...) Project
    Typecheck,          // Type checking (with embedded proof layer) PerModule
    DeadCodeAnalysis,   // Dead code family analysis       Project
    ProofExecution,     // Proof function compile-time execution   PerModule
    GlobalSlotAlloc,    // Global slot allocation          Project
    IrGeneration,       // AST → ModuleIR                  PerModule
    Monomorphization,   // Monomorphization                Project
    Linking,            // Cross-module linking / IR merge Project
}

impl Stage {
    /// All stages. When a new variant is added, both this array and the exhaustive `match` in `dispatch` will fail to compile.
    pub const ALL: &'static [Stage] = &[ /* all 12, in topological order */ ];

    pub fn scope(self) -> StageScope {
        match self { /* PerModule / Project */ }
    }
}

pub enum StageScope { PerModule, Project }
```

**The purpose of `StageScope`**: `PerModule` (runs once per compilation unit) and `Project` (runs
once at project level) turn "which stages must run per module, which must run once at project level"
into a type-level fact. A `Project`-scoped stage is structurally guaranteed to run only once in
`dispatch`—this eliminates questions like "should `allocate_global_slots` be called once per file"
currently in `orchestrator.rs` that require human reasoning.

**The stage table is arranged in topological order rather than alphabetical order**, because failure
propagation depends on order.

> **Revision note (P4 implementation, 2026-10-07)**: Two deviations from the original draft are
> corrected per implementation evidence—
>
> 1. `Monomorphization` moved to **after** `IrGeneration`: monomorphization consumes IR product
>    (`Monomorphizer::monomorphize(&ir, …)`, pipeline.rs), the original draft's order contradicted
>    the data flow.
> 2. The Check form's stage table is normalized to dead code **before** proof per `Stage::ALL`
>    topological order: `check_project`'s current state runs proof before dead code, and the two
>    have no data dependency, the diagnostic set is the same (C2 set semantics), only the in-file
>    diagnostic order is normalized. The single-file path (dead code embedded in typecheck, before
>    proof) already follows ALL order, and byte-by-byte acceptance is unaffected.
> 3. `Parsing` moved to **before** `Registry`/`RoleClassification` (C3, 2026-10-09 user ruling):
>    signature collection (`extract_module_info`) and `ast_has_main` both consume AST products—the
>    original draft's order would force the Registry arm to "hide parse", making the stage table lie
>    about data flow; and multi-file parse therefore drops from 2 times to 1 time. The current state
>    of the attachment is honestly registered: the multi-file path's parse error used to be a hard
>    abort (`build_registry_from`'s `?` propagation, and the tension with CollectAll semantics is
>    registered as WBS 4.10.1)—**4.10.1 is fixed** (2026-10-09 user ruling Rust-style collection
>    semantics + Plan B): the Check path's parse failure is degraded to per-file diagnostic
>    collection, with the failing file exiting the compilation unit (not entering the registry,
>    importers report E5001); the MultiFile path retains hard abort (FailFast with a bad file
>    produces no IR, which is correct semantics, pinned long-term).
> 4. Check form landing (4.2.2, 2026-10-09 user ruling) adds two registrations: a. Data dependency
>    edges 14→16: `RoleClassification` consumes Discovery's `used_by` edge set, `DeadCodeAnalysis`
>    consumes Discovery's W1006 shadowing events—the original draft missed listing these (these two
>    products are produced by `discover_with_used` in the orchestrator era and discarded by the
>    `discover` wrapper, and the data flow is lost when not in the table); b. The normalization of
>    in-file diagnostic order is extended to the complete form of note #2 above: the E3020 entry
>    validation is moved into the RoleClassification arm (Check has no Linking stage, the arm
>    holding surfaces and AST is responsible for validation), the in-file diagnostic order = stage
>    topological order (E3020 → typecheck → W1006/W1003/dead code → proof); the diagnostic set is
>    unchanged (C2 set semantics), only the stderr entry order changes.

### 2. Obligation Ledger: `Obligations` + `assert_drained()`

**Design goal**: Turn "field is produced but no one consumes it" from undetectable into a fact that
can be asserted at compile time or test time. RFC-039 has already listed this as "the most valuable
single item".

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

| Situation                               | Criterion                                                   | Disposition                                                           |
| --------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------- |
| Obligation is read by declared consumer | The field in `Obligations` is `take()`-ed / marked consumed | Pass                                                                  |
| Obligation non-empty but no consumer    | Field non-empty and not consumed                            | **Produce diagnostic** (W-level by default; E-level in `strict` mode) |
| Obligation empty                        | Field is empty                                              | Pass (no consumption needed)                                          |

**Why start with W-level instead of E-level**: Fixing obligation discard will **change the
diagnostic set**. C2 requires "diagnostic set identical for each entry"; if we jump straight to
E-level, `yaoxiang check` will suddenly have a lot of previously silenced `Unproven` diagnostics.
Splitting into two steps (first W observation, then E upgrade) makes the criterion usable at each
step. See "Implementation Points" S4 for details.

**Single call site of `assert_drained()`**: at the end of `Driver::run`'s stage table, before
producing `CompilationResult`.

**Companion static gate**: RFC-039 has proposed `scripts/ci/check-obligations.py` (field appears ≥2
times but no third-file read → fail). `assert_drained()` is a **runtime** gate; this script is a
**static** gate; the two are complementary and both are needed.

> **Revision note (P4 implementation, 2026-10-13, 4.3.1 landing)**: This note registers the
> **implementation form, measured numbers, and 4.3.2 interface** of the above sketch and three-tier
> table, corresponding to code file:line one by one. The landing commit is `1941cc47a` (verification
> tree `99079bc52`). It is also the **third** of the three places that take on "don't be silent"
> from the red list (the other two: audit surface, static gate).
>
> **I. Landing artifacts (file:line)**
>
> - Ledger `src/driver/obligations.rs`: 13-item obligation inventory `INVENTORY` (`:201`, field name
>   / consumer stage / red list), **non-obligation field registration `NON_OBLIGATION_FIELDS`
>   (`:150`, 8 entries with reasons)**, delivery-marker ledger `DeliveryLedger`, pure settlement
>   `settle_obligations` (`:577`), emission `emit_obligation_diagnostics` (`:654`),
>   non-consumer-side gate `consumer_visible_unit` (`:538`), skip criterion `consumer_stage_skipped`
>   (`:549`), IrGeneration delivery set `IR_GENERATION_DELIVERED` (`:285`), W1090 code string
>   constant `OBLIGATION_UNDRAINED_CODE`.
> - **Sole settlement point**: at the end of `Driver::run`'s stage table, before `into_outcome()`
>   produces `CompilationResult`—`src/driver/mod.rs:110` (`State::assert_drained`,
>   `obligations.rs:570`). The early-exit path of `Aggregation::FailFast` also settles (the ledger
>   must balance).
> - **Seven delivery points** (one per arm; remove any one and the runtime W1090 is guaranteed to
>   fire): ProofExecution `src/driver/arms.rs:554` (`proof_calls()` is the sole external read path
>   from `TypeCheckResult` for this obligation, `types.rs:170`); IrGeneration `arms.rs:672` (Check)
>   / `:726` (Embedded) / `:781` (SingleFile·WasmPlayground) / `:842` (MultiFile)—four identical
>   delivery ports: `AstToIrGenerator::new_with_type_result` / `middle::generate_ir_with_context`;
>   Monomorphization `arms.rs:936` (single-file direct read of request table) / `:1221`
>   (MultiFile·Check: Linking arm aggregation, `driver/helpers.rs:295` reads each unit's
>   `instantiation_requests`).
> - Audit surface: `DriverOutcome.obligations` (`src/driver/state.rs:73`) =
>   `ObligationSettlement{ findings, pairs, empty, consumer_stage_absent, consumer_stage_skipped, delivered, drained_kinds, emitted }`;
>   the conservation equation
>   `pairs = empty + consumer_stage_absent + consumer_stage_skipped + delivered + findings.len()` is
>   pinned by use cases.
> - Diagnostic code: W1090 is registered at `src/util/diagnostic/codes/w1xxx.rs:32`
>   (`span_exempt=true`, reason written in the entry above `:26-31`—the settlement granularity is
>   "unit × obligation", there's no user-pointable source location, so don't stuff a dummy span);
>   six-language text in `locales/*.json`. **Emission level**: from 4.3.2 onwards it is **error**
>   (same code promoted to E, see third paragraph)—determined by explicit
>   `.severity(Severity::Error)` in `to_diagnostic` (`obligations.rs:444`), the code number remains
>   in the W family.
> - Static gate: `scripts/ci/check-obligations.py`; CI new job `obligations`
>   (`.github/workflows/ci.yml:140-155`, including `--selftest`).
>
> **II. Per-cell landing of the three-tier table + two added tiers**
>
> | Above three-tier table situation     | Implementation criterion                                                                      | Code                   |
> | ------------------------------------ | --------------------------------------------------------------------------------------------- | ---------------------- |
> | Obligation read by declared consumer | **Delivery marker exists** (marker ↔ real read reconciled by static gate, see section V)      | obligations.rs:618-623 |
> | Obligation non-empty but no consumer | Non-empty + consumer stage has run + no marker → W1090 (**error-level**, see third paragraph) | obligations.rs:624-630 |
> | Obligation empty                     | `Obligation::is_produced` per-field emptiness check → pass                                    | obligations.rs:587-591 |
>
> Two **added tiers** (2026-10-13 user ruling, not in the sketch):
>
> 1. **S1**: The obligation's consumer stage is not in this `ProgramKind`'s stage table → **don't
>    judge as owed** (obligations.rs:593-597). Evidence: `LSP_STAGES` has no
>    IrGeneration/Monomorphization (`program.rs:112-119`), `EMBEDDED_STAGES` has no Monomorphization
>    (`:121-126`).
> 2. **Skip exemption**: Consumer stage skipped for `SkipReason::UpstreamFailed` / `ConfigDisabled`
>    → don't judge as owed (obligations.rs:598-602). `NoObligations` does **not** exempt—when the
>    obligation is non-empty and that stage claims no obligations, that precisely means the ledger
>    is unbalanced, must report.
>
> **III. Emission caliber (ruling "C") and the measured numbers behind it**
>
> **4.3.1 (observation period)**: This mechanism emits only **W-level** (W1090); **no strict
> switch**. **4.3.2 (final state, 2026-10-13 user ruling A)**: **Same-code promote to E**—the W1090
> code number unchanged, the emission level raised to `error` (explicit
> `.severity(Severity::Error)`, `to_diagnostic` obligations.rs:444); **no new E code** (the same
> fact with two codes is parallel representation). **The upgrade only changes the emission level,
> not the emission condition.** **This is a D-class breaking change of D59 (changes externally
> observable behavior), registered truthfully**: healthy programs emit zero (when all obligations
> have delivery markers, no report), so existing corpora and user programs are zero-affected
> (342/342 "diff: empty"); but once a run actually hits an undelivered obligation, that diagnostic
> **counts into `error_count`, renders as `error[W1090]`, exit code non-zero**—different from the
> previous "W-level does not block compilation". The caliber that code family and emission level are
> decoupled is registered in RFC-013 "Emission Channel" section (the code number prefix only
> determines the numbering attribution, the level is determined by the explicit severity at the
> construction point). Mechanism fact (measured): the severity takes the explicit `.severity()`,
> otherwise "code starts with `W` → Warning" (`codes/builder.rs:186-203`), the registry's
> `ErrorCategory` **does not participate** in the severity; the exit code is determined by
> `(error_count, warning_count)` (`util/diagnostic/command.rs:87`) and `--deny-warnings`
> (`src/main.rs:567-568`), **W1090 goes to `error_count` (from 4.3.2) because of explicit error**.
>
> **Red-list fields don't enter the user-visible diagnostic stream** (captain 2026-10-13 ruling C;
> current red list has reached zero, see section IV), the reason is reproducible:
>
> - Ruling A (red list emitted as W) measured cost: `interface_impl_registry` is injected by
>   typecheck from the preloaded table, **constantly non-empty**, the per-compile emission is a
>   **constant signal** (no discriminative power), yet breaks three existing external
>   contracts—"clean program zero diagnostics / LSP zero diagnostics / `warning_count == 0`":
>   `cargo test --lib --all-features` measured **12 existing use cases turned red**, of which **8
>   are on out-of-scope paths** (`orchestrator/tests/namespace_data_access.rs` ×5,
>   `lsp/handlers/tests/diagnostics.rs` ×2, `util/diagnostic/tests/mod_tests.rs` ×1; the other 4 are
>   in `src/driver/tests/`), 24/342 fixtures in the corpus baseline changed, and the only viable fix
>   is "filter W1090 out of the zero-diagnostic assertions" = relaxing the existing criteria.
> - Ruling B (downgrade to Info level) still breaks 9/12 of the list-style assertions, and Info is
>   not "output by ruling level".
> - Ruling C measurement: existing use cases **zero changes**, corpus baseline **zero changes**
>   (342/342 "diff: empty, behavior equivalent"), runtime W1090 becomes a **discriminating
>   regression detector**—healthy tree is silent throughout, only fires when a delivery marker is
>   removed (the three delivery points each had marker-removal mutation done, CLI `check` in the
>   Check arm with marker removed emits 2 `warning [W1090]`).
>
> The red list's "don't be silent" is taken on by three places: ①
> `DriverOutcome.obligations.findings` audit surface (carried out every run, with WBS id and
> evidence pointer); ② `check-obligations.py` prints the red list (currently empty) + staleness
> detection on every run; ③ this note.
>
> **IV. Red list current state (settled) and landing records of 3.4.4/3.4.5/3.4.9**
>
> Red list 3 → **0**—the three settlement tasks all landed on 2026-10-13:
>
> - **3.4.4 `variant_ctor_calls` → delete field**. Its declared responsibility ("ir_gen produces
>   CreateVariant") is equivalently fulfilled by the IR-side in-place form detection:
>   `src/middle/core/ir_gen.rs:7448 fn detect_variant_ctor_call` (same AST form
>   `FieldAccess{Call(Var)/Var}` + takes declaration order from `sum_type_variants`), call site
>   `:7496` swaps the entire Call for `CreateVariant`; the function's doc explicitly writes
>   "span-key cross-system unreliable (two ends have different semantics for Call node span), so
>   detect in-place"—**wiring a consumer would re-introduce the already-fixed span mismatch
>   defect**. Deletion surface: `types.rs` field, `checker.rs:1489-1493/:1565` collection,
>   `inference/expressions.rs` and `inference/statements.rs` collection chain, `environment.rs`'s
>   `VariantCtorCall` struct; `infer_variant_ctor_call`'s type inference and all diagnostics
>   **unchanged**.
> - **3.4.5 `method_overload_ir_names` → delete cross-stage mirror** (kept in-phase candidate name
>   table and `overload_resolutions`). All read-write points: written in-phase
>   (`environment.rs:205-219` and `method_overloads` registered in same order → `checker.rs:972` →
>   `inference/statements.rs:2871-2874` → `inference/expressions.rs:498`), read in-phase
>   (`expressions.rs:4490-4550` decision point, the only hit writes to `overload_resolutions` at
>   `:4550`); what is deleted is the **zero-reader mirror** injected into `TypeCheckResult` at
>   `checker.rs:1558`. Naming facts still singly expressed: candidate name table (in-phase) +
>   span-keyed resolution (cross-stage, constructed at `ir_gen.rs:380`, consumed at `:7632/:7705`);
>   the define-side mixed-ed name is self-computed by `method_def_ordinals` (`:1828`).
>
> - **3.4.9 `interface_impl_registry` → reclassify as in-tree fact** (user ruling A, see the
>   reclassification record in the §Obligation Fields table of this document). The **declaration**
>   of "mirrored for middle-end/LSP consumption" in the doc comment is deleted (types.rs:47-48), the
>   field and its in-phase consumption are kept (`operator_interfaces.rs:623` operator query,
>   `inference/expressions.rs` seven constraint-solving queries, `checker.rs:214-217` read-only
>   accessor); cross-stage zero read direct evidence
>   `git grep -n 'interface_impl_registry' -- src/middle` → empty. **Why not wire a consumer**: the
>   table's content is an environment mirror injected by typecheck from the preloaded table
>   (constantly non-empty); if LSP really needs it, follow the 4.6.x new-registration path; creating
>   a consumer for a field that "by design does not need cross-stage consumption" only introduces
>   fake obligations into the ledger.
>
> After the red list reaches zero, **field coverage is closed by "obligation list ∪ non-obligation
> field registration (`NON_OBLIGATION_FIELDS`, with reasons)"**—the reverse-containment end of gate
> criterion 1 holds this throughout (added field without registration / deleted field without
> synchronization both turn red immediately). The red list **mechanism** is kept: gate criterion 4's
> staleness detection and `--selftest`'s `stale-red-list` fixture continue to cover its detection
> surface; future re-registration must go through explicit ruling.
>
> **Caliber synced with the settlement**: obligation list 16 → **13** (`INVENTORY` /
> `Obligation::ALL` / this document's §Obligation Fields table); gate `--selftest` now covers seven
> categories of fixtures (adding "unregistered field", "stale anchor", and "red list empty" three
> forms), see
> `tests/baselines/obligations-gate/{stale-manifest,stale-struct,stale-red-list,unregistered-field,stale-anchor}/`.
>
> **V. Static gate (9.2) actual criteria** (three more hard criteria beyond the sketch's one
> sentence)
>
> 1. **Three-place reconciliation + reverse containment (F2) + anchor validation (F1)**:
>    `obligations.rs`'s `INVENTORY` ↔ `TypeCheckResult` field definitions ↔ this document's
>    "§Obligation Fields" table (including the table header's entry count declaration), any drift is
>    red; and the field set of `TypeCheckResult` must be **exactly equal to** "obligation list ∪
>    non-obligation registration (`NON_OBLIGATION_FIELDS`, with reasons)"—added field without
>    registration, deleted field without synchronization, non-obligation registration missing
>    reason, two tables overlapping all turn red immediately;
> 2. **Read-out evidence**: non-red-list obligations must have a real read in the **declared
>    consumer file** (`.fieldname` member access; comments / tests / strings don't count)—this turns
>    "delivery marker ↔ downstream real read" into a hard criterion (the main risk of mechanism A is
>    precisely the two drifting apart);
> 3. **Delivery marker**: non-red-list obligations must be marked at the delivery point; red-list
>    ones must not be marked;
> 4. **Red-list staleness detection**: red-list fields must have zero real reads (growing a consumer
>    is red), and each carries a WBS id and evidence pointer; plus `--selftest`: brings its own
>    negative/positive fixtures, negatives must be red, positives must be green (proving the
>    detector isn't spinning idly).
>
> **VI. Per-spot deviations from the above sketch**
>
> 1. The sketch's `pub struct Obligations { …fields… }` **does not copy by field**: obligation data
>    is still held by `TypeCheckResult`, the ledger only records "inventory + delivery marker +
>    settlement conclusion". Reason: copying creates two sources of truth (each field produced
>    separately by typecheck's checker and ownership layer), and they will inevitably drift;
>    cross-layer data surfaces also shouldn't be replicated into the orchestration layer.
> 2. The sketch's `take()` / "mark consumed" **not adopted**, replaced by `is_produced` emptiness
>    check + `DeliveryLedger` delivery marker. Reason: the obligation and the product are the same
>    data; `take()` would take away what the downstream needs; and the 11 IrGeneration obligations
>    are in the form of "the whole `&TypeCheckResult` handed to ir_gen", which can't be taken field
>    by field.
> 3. Sketch signature `pub fn assert_drained(&self) -> Vec<Diagnostic>` → actual
>    `State::assert_drained(&mut self)`: the conclusion is written into `DriverOutcome.obligations`,
>    and diagnostics are channeled by `ProgramKind`: Check/Lsp goes through the per-file collection
>    channel; other forms are channeled by **level** (from 4.3.2 W1090 is E-level)—single-file goes
>    `PipelineError::TypeCheck` (obligations.rs:654 emission arm), multi-file/embedded separately
>    records `OrchestratorError` failure (otherwise `result.errors` has no consumer). Reason:
>    returning `Vec<Diagnostic>` would lose form channeling—check diagnostics must be attributed per
>    file.
> 4. The first tier of the three-tier table's criterion goes from "obligation is read by declared
>    consumer" to "has delivery marker"; "marker ↔ real read" is left to the static gate (section V
>    criterion 2).
> 5. The second tier's "`strict` mode promotes to E-level" **does not go through a strict switch**,
>    replaced by **4.3.2 same-code promote to E** (promotes even without a strict switch, because
>    healthy-state zero emission is already safe enough, R4).
> 6. The static gate criterion is upgraded from "field appears ≥2 times but no third-file read" to
>    the four in section V.
> 7. The two added tiers (S1, skip exemption) not in the sketch see section II.
> 8. The unit test name `test_obligations_drained` mentioned in the S1/S4 table: actually landed as
>    the **20 use cases** of `src/driver/tests/obligations.rs` (skeletons first `#[ignore]` → turn
>    green; red state kept on file: after removing the only settlement point `pairs: 0`, 6
>    failures). The S1/S4 two lines are synced to the actual file.
>
> **VII. 4.3.2 landing record (level determination + gate reinforcement)**
>
> - **Per-field re-review input**: `ObligationSettlement.drained_kinds` (the kinds of obligations
>   actually drained this run, available per run) and five counts (conservation equation in section
>   II); combined with the audit surface `findings` it can give the fact "whether this field has
>   been consumed by any entry".
> - **Level: same-code promote to E landed** (2026-10-13 user ruling A; see the final-state
>   registration and D59 D-class breaking-change declaration in section III). Implementation:
>   `to_diagnostic` explicit `.severity(Severity::Error)`; emission arms channel the error path by
>   level. Mutation evidence: remove any one delivery marker → that form's compilation fails,
>   `check` exit code non-zero (`error [W1090]`); after restore, healthy state zero emission (corpus
>   342/342 zero changes). locale text already written in six languages to make clear "W-family
>   code, emission level is error".
> - **Gate reinforcement items (F1/F2, both landed)**:
>   - **F1/R2 anchor validation**: the gate now parses the `file:line` anchors in **non-obligation
>     registration reasons and red-list evidence**, validating "file is locatable (unique) + line
>     number is registered + the symbol immediately following the anchor actually appears in the
>     anchored file"; an anchor pointing to a non-existent symbol is red. Comes with negative
>     fixtures `tests/baselines/obligations-gate/stale-anchor/` (symbol doesn't exist / line out of
>     range, one each). Note: the red list is currently empty, so this criterion's actual landing
>     point is **non-obligation registration** and future evidence pointers (same code path).
>   - **F2/R3 reverse containment**: the gate outputs "`TypeCheckResult` field count − obligation
>     count − non-obligation registration count = delta", non-zero delta is red; comes with negative
>     fixture `unregistered-field/` (struct added a field without registration).
> - **Information item F3**: `SkipReason::UpstreamFailed` exemption is unreachable in real entries
>   (the unit whose typecheck fails doesn't keep `type_result`, and the obligation doesn't exist in
>   the first place); this tier's semantics is pinned by synthesized `State` use cases, real
>   end-to-end only covers `ConfigDisabled` (t3 T06).
>
> **VIII. 4.3 second-level task closure (red list reaches zero + obligation diagnostic final
> E-level)**
>
> **8.1 Three-round landing timeline** (corresponding to code file:line one by one; this round only
> adds conclusions, doesn't change code)
>
> | Round                     | Conclusion                                                                                                                                                                        | Code Anchor                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
> | ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | 4.3.1 Ledger online       | `State::assert_drained()` sole settlement point; 13-item obligation inventory + delivery marker + three-tier settlement; audit surface `DriverOutcome.obligations`                | Call site `src/driver/mod.rs:110`; settlement `src/driver/obligations.rs:577`; emission `:654`; seven delivery markers `src/driver/arms.rs:554/:672/:726/:781/:842/:936/:1221`                                                                                                                                                                                                                                                                                               |
> | 3.4.4 Settlement          | `variant_ctor_calls` delete field—declared responsibility equivalently fulfilled by IR-side in-place form detection                                                               | `src/middle/core/ir_gen.rs:7448` (`detect_variant_ctor_call`), `:7496` (call site)                                                                                                                                                                                                                                                                                                                                                                                           |
> | 3.4.5 Settlement          | `method_overload_ir_names` delete cross-stage mirror (kept in-phase candidate name table + `overload_resolutions`)                                                                | Decision consumer point `ir_gen.rs:380/:7632/:7705`; define-side `method_def_ordinals` `:1828`                                                                                                                                                                                                                                                                                                                                                                               |
> | 3.4.9 Settlement          | `interface_impl_registry` reclassify as **in-tree fact**: delete "mirrored for middle/LSP consumption" declaration, keep in-phase consumption; obligation 14 → 13, red list 1 → 0 | Declaration carrier `src/frontend/core/typecheck/types.rs:52` (field declaration line; its doc comment see same file `:47-51`, ruling A rewrite); in-phase consumption `src/frontend/core/typecheck/operator_interfaces.rs:623`, `src/frontend/core/typecheck/inference/expressions.rs` seven places (`:1514/:1646/:3151/:3876/:3899/:4400/:4408`), `src/frontend/core/typecheck/checker.rs:214` (read-only accessor :214-217); registration `src/driver/obligations.rs:150` |
> | 4.3.2 Level determination | **Same-code promote to E**: explicit `.severity(Severity::Error)`, code number still `W1090`; emission channeled to error path by level (first failure priority)                  | `src/driver/obligations.rs:450` (severity), `:676-699` (E-level branch); code family and level decoupling registration see `src/util/diagnostic/codes/w1xxx.rs:26-36`                                                                                                                                                                                                                                                                                                        |
>
> **8.2 Closure conclusion**
>
> - **Red list reaches zero**: the count of `red_list: Some` entries in `INVENTORY`
>   (`src/driver/obligations.rs:201`) = **0**
>   (`git grep -c 'red_list: Some' -- src/driver/obligations.rs` → 0). The mechanism is **kept**:
>   `RedListEntry` (`:120`), `Obligation::red_list()`, gate criterion 4's staleness detection and
>   `stale-red-list` fixture; gate output "red list 0 entries—settled to zero".
> - **Obligation diagnostic final E-level**: the **code number and W-family attribution of `W1090`
>   unchanged**, emission level is `error` (D5 final state, 2026-10-13 user ruling A). This
>   **belongs to the D-class breaking change of D59** (changes externally observable behavior),
>   registered both in section III of this note and in the "Emission Channel" section of RFC-013;
>   **healthy-state zero emission ⇒ existing corpora and user programs zero-affected** (corpus
>   342/342 "diff: empty", IR snapshots zero drift; t11 independent verification and t12 re-review
>   each re-reviewed).
> - **Field coverage closed**: `TypeCheckResult` field set = obligation list ∪ non-obligation
>   registration (8 entries, `src/driver/obligations.rs:150`, each with measured consumption-surface
>   reason); gate F2 explicitly outputs the delta (currently `21 - 13 - 8 = 0`).
> - **No unlanded commitments left**: all reinforcement items registered in this note **have
>   landed** (F1/F2/F4/F5, see sections V, VII, VIII); the only two unclosed items are the **known
>   boundaries** in 8.4 and t7 F5 which has been turned into an issue, neither is an unlanded
>   criterion of this mechanism.
>
> **8.3 Disposition status of each round's acceptance / re-review findings** (each marked)
>
> | Source        | Finding                                                                                                                                                                      | Status                                                                                                                                                                                                                                                                     |
> | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | t3 F1 / t4 R2 | Red-list evidence is plain-text pointer, silently stale when alternative consumer is removed                                                                                 | **Landed**: gate F1 anchor validation (file locatable + line registered + symbol anchor, `scripts/ci/check-obligations.py:251`); negative fixture `tests/baselines/obligations-gate/stale-anchor/`                                                                         |
> | t3 F2 / t4 R3 | Gate criterion 1 missing reverse containment, new field added without registration silently PASSes                                                                           | **Landed**: F2 delta explicit output (`:577`) + F4 definition-line reconciliation (`:455`) + F5 range reconciliation; negative fixture `unregistered-field/`                                                                                                               |
> | t4 R4         | W-only and promote-to-E caliber not registered                                                                                                                               | **Landed**: this note's section III / VII + RFC-013 "Emission Channel" + six-language `locales/*.json` W1090 help and error_output examples                                                                                                                                |
> | t7 F5         | Dead code analysis's overloaded method reference count distortion (legal overload still misreported as `W1005`)                                                              | **Turned into issue `#450`**, belongs to WBS **4.9 Reference Pool Correct Implementation** (`docs/src/dev/architecture/09-execution-wbs.md:237` registration)—not this second-level task                                                                                   |
> | t11 F7        | Gate `--root` mode F1 doesn't cover anchor commitments in module header prose comments                                                                                       | **Known boundary** (see 8.4)                                                                                                                                                                                                                                               |
> | t11 F8        | Validator's own self-record: mutation driver once accidentally wrote to the main repository (immediate `git restore` cleared, affected rounds' output invalidated and rerun) | **Information item** (not a delivery defect)                                                                                                                                                                                                                               |
> | t11 F9        | Main repository intermediate state (unstaged files caused this document's table header to desync with `INVENTORY`) triggered gate red                                        | **Information item / resolved**: frozen commit self-consistent PASS; "intermediate state directly committed triggers gate red" is exactly the design intent of F2/F4                                                                                                       |
> | t12 F12-1     | This document wrote struct range as `16-73`, actually `16-76` (no mechanical reconciliation after t9 added 3 comment lines)                                                  | **Landed + mechanized**: range corrected to `16-76`, and added gate **F5** (range anchor, writes are red); same-class numbers grep-aligned in one pass (`17 → 20 use cases`, `16 span-keyed fields` → 13)                                                                  |
> | t12 F12-2     | Multi-file form's settlement once overrode the first failure (`zzz.yx`'s type error was covered by W1090)                                                                    | **Fixed** (captain has amended commit message explaining): `failure.is_none()` guard at `src/driver/obligations.rs:686`; this regression was caught by the **existing pinned** use case `test_driver_run_multi_file_proof_error_yields_to_typecheck_error`, not a new test |
>
> **8.4 Known boundaries (registration items, not unlanded commitments)**
>
> - **F1's scan surface**: anchor validation only covers text in **registered
>   entries**—non-obligation registration reasons (`NON_OBLIGATION_FIELDS`) and red-list evidence;
>   **`file:line` commitments in module header prose comments are not in the scan surface**
>   (currently no registered entry is affected). If anchor commitments are written in module headers
>   in the future, **manual review must take on** this, or extend F1's scan surface—take one or the
>   other, do not allow default to be unattended.
> - **Gate's static boundary**: the set-level validation of delivery markers is on the static side
>   (criterion 3), single-arm removal is supplemented by **runtime settlement**— static + runtime
>   double-layer is the design form, not a gap (t11's M11A/M11B/M11C mutation bidirectional
>   validation).

> **Revision note (P4 implementation, 2026-10-13, 4.5 cross-layer contract PlanId-ization
> landing)**: Among the above 13 obligations, **two span-keyed contracts** (`release_plan` /
> `overload_resolutions`) have been changed to **structural identity keys** (`PlanId`) per RFC-039
> **D20**. This note registers the landing form, criteria, classification basis, and known
> boundaries; the definition lines are still reconciled by gate **F4** (the two rows' line numbers
> in the table above are the current code fact).
>
> **I. Landing form (carrier takes A: `PlanId`)**
>
> The key is a **structural identity** that travels with AST nodes, no longer relying on the
> positional coincidence of "both sides computing span separately": `PlanId` (`src/util/plan_id.rs`,
> `u32` monotonic, unique within compilation unit, `UNPLANNED = u32::MAX` indicates synthesized
> nodes outside the parser) is issued by the **sole allocator** `ParserState::alloc_plan_id`.
> Carriers are in two places: `Stmt.plan_id` (statement identity → ownership and consumption of the
> release plan) and `Expr::Call.plan_id` (call-site identity → production and consumption of
> overload resolution); `Expr::call` is the **only** call expression constructor in the repository,
> and synthesized calls outside the parser are uniformly marked `UNPLANNED` (**cannot be consumed**,
> see section II).
>
> - `release_plan`: `drops` is `HashMap<PlanId, Vec<String>>` (`layers/ownership.rs`), the only
>   consumption entry is `ReleasePlan::consume(stmt)`—**read and bookkeeping in the same call**;
>   `ir_gen.rs:360` reads, `:3103` uses (statement identity given by `walk_stmt`'s set/restore + CFG
>   node→statement identity table; the Drop at scope exit belongs to the last statement in the
>   scope).
> - `overload_resolutions`: `OverloadResolutions` (table + consumption ledger **in one**),
>   production point `inference/expressions.rs:4550` (writes by call-site `PlanId`, dispatch
>   transparently passes through from `Expr::Call { plan_id }`), consumption point `ir_gen.rs:7697`
>   (with binding) / `:7772` (without binding) via `OverloadResolutions::get` (hit immediately
>   books).
>
> **II. Consumption-must-reach settlement (reuse 4.3 ledger, don't create a second one)**: The
> consumption ledger is the **single implementation** `ConsumptionLedger<K>`
> (`src/util/consumption.rs`), and each of the two tables holds one instance; when
> `generate_module_ir` returns successfully, **at the same point** mark "complete one full run" of
> the two ledgers (`ir_gen.rs:1095` release plan / `:1097` overload resolution); the three-state
> mapping is unique (`consumption_state` in `src/driver/obligations.rs`), and the sole settlement
> point hooks one line for each of the two obligations—non-empty difference is **E-level W1090**
> (same-code promote to E, caliber see section III). The exemption criterion hangs on the **ledger's
> own completion marker** (not on whether `unit.ir` exists), so the Check form taking away the IR
> product doesn't constitute an exemption.
>
> **III. D59 classification basis (C-class, red-then-green)**: The form
> `Type.method(positional args, name = value)` was **silently giving the wrong answer** before the
> change (measured: `r: Int = t.pick(1, a = 2)` compiles clean, runs outputting `1` instead of
> `20`—the resolved candidate is discarded, falls back to the last binding in declaration order);
> after the change the same program at compile time `error [W1090] ... produced key not consumed`.
> The fix takes the **loud** approach rather than "stuffing the resolved name into a call with named
> arguments": the latter would lose named arguments, equivalent to another form of silence
> (final-state semantics see [09](./09-execution-wbs.md) §3.14 / issue #452). Also: `Drop`
> instruction is a no-op in the interpreter (the catch-all arm at
> `src/backends/interpreter/executor/ops/control.rs` only does `advance()`), so the release plan's
> key migration doesn't change runtime behavior—the corpus 342 diffs being empty is empirical proof
> of this.
>
> **IV. Criteria and evidence**: D41 same-form criterion `test_release_plan_spans_consumed` (settles
> by `PlanId`) and the newly added `test_overload_resolutions_consumed` (per-file corpus
> lex→parse→typecheck→ir_gen, **difference must be empty + evidence-surface floor**: the count of
> files that produced resolution `≥ 1`, the count of actually consumed entries `≥ 2`, no whitelist);
> 9 decision-form coverage pinned cases see
> `src/frontend/core/typecheck/tests/overload_resolutions.rs` (expected return / positional args /
> type-body binding / `?` operand / pure named / positional+named / ambiguous / variant construction
> / unallocated identity). Mutation driver: consumer key offset by one ⇒ criterion red + real
> program `error [W1090]` + corpus gate diff; after restore back green. Gate numbers:
> `check-obligations.py` PASS, corpus 342 diffs empty, `cargo test --lib` 2712 passed / 0 failed (IR
> snapshots zero drift).
>
> **V. Known boundaries (registration items, not unlanded commitments)**: The **no-binding
> consumption point** at `ir_gen.rs:7766-7790` is **unreachable** in the current corpus and test
> surface (method declarations uniformly go through the with-binding consumption point via
> `register_type_bindings`), so the key correctness of this branch has **no red state that can
> falsify it**; the reason for keeping it = it's the symmetric branch of the with-binding path,
> deleting it would rewrite the "no `type_bindings` method call" form into silence. Registered by
> independent verification (t5 F-1) and re-review (t6 F2), **no new task opened**.

### 3. Unified Driver: Single `dispatch`

```rust
// src/driver/program.rs
pub enum ProgramKind {
    SingleFile,        // pipeline.rs:141 / lib.rs:140 / build_bytecode(lib.rs:171)
    MultiFile,         // compile_project (lib.rs:154)
    Check,             // check_project / check_files_with_diagnostics
    Lsp,               // check_source_in_project (LSP in-project, results filtered to target file)
    Embedded,          // compile_embedded_module (embedded std)
    WasmPlayground,    // wasm/src/lib.rs:48 — the fourth caller of the single-file path
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

**`stages()` can only be one of the 6 predefined combinations above (one per `ProgramKind`); it does
not accept the caller freely passing arrays.** This is the key constraint that prevents the
`Program` abstraction from degrading into "a free parameter that accepts anything", asserted by
`test_program_stage_coverage`.

```rust
// src/driver/mod.rs
pub struct Driver { config: CompileConfig }

impl Driver {
    pub fn run(&mut self, program: Program) -> Result<CompilationResult, DriverError> {
        let mut state = State::new(program);
        for stage in program.stages() {
            // Topology determines whether to skip, not stage return value
            if !state.deps_satisfied(stage) {
                state.record_skipped(stage);   // Must produce diagnostic, see below
                continue;
            }
            match stage {                            // exhaustive match
                Stage::VendorConsistency => { self.vendor_consistency(&mut state)? }
                Stage::Discovery         => { self.discovery(&mut state)? }
                // … 12 arms, not one less
            }
            if state.should_abort() { break }
        }
        state.obligations.assert_drained();           // sole settlement point
        Ok(state.into_result())
    }
}
```

> **Revision note (P4 implementation, 2026-10-09, 4.1.3 landing)**: Four implementation deviations
> from the above sketch, semantically equivalent, registered for future reference—
>
> 1. §4's three-state `StageOutcome` is not returned by stage arms, but expressed by `State`'s
>    failure marker + `Aggregation` gate (Continue / Warn / Abort semantics unchanged, sparing the
>    per-arm boilerplate return);
> 2. `Driver` has no `config` field—configuration's sole source is `Program.config` (§5's "avoid
>    Driver holding mutable global state" ruling covers the sketch's field);
> 3. `Skipped` diagnostic in 4.1.3 is only recorded in `DriverOutcome.skipped` internal ledger, not
>    emitted—S2's "deliberately fix no bug" zero-diff criterion requires it; emission follows 4.3
>    obligation ledger;
> 4. `proof_execution` module visibility relaxed to `pub(crate)`: driver arm is the only new caller
>    (L1→L2 is allowed direction); orchestrator's existing call sites are migrated away with 4.2 and
>    revisited afterwards.
> 5. Under multi-file form, ProofExecution is an independent stage, executed after **all** typecheck
>    (A1, 2026-10-09 user ruling): the original `compile_project` interleaved proof inside the
>    per-file typecheck loop (file N's proof before file N+1's typecheck). Single-failure the two
>    are byte-by-byte identical; in the "file 1 proof failure + file 2 typecheck failure"
>    multi-failure scenario, the first report changes from proof error to typecheck error (test
>    pinned).
> 6. `DriverOutcome` carries products by ProgramKind channel: `result` (pipeline contract) /
>    `module` + `failure` (orchestrator contract, 4.2.1)—the external error contracts of each entry
>    (PipelineError / OrchestratorError) are not the type surface that Driver can unify.
> 7. Check channel landing (4.2.2): `DriverOutcome.check_diagnostics` carries per-file diagnostics
>    (each found file has an entry, clean files have empty Vec—`check_project` current contract);
>    Discovery's product extends to a triplet of "file set + used_by edge set + shadowing events"
>    entering State (MultiFile form has no downstream consumer, only recorded not emitted); Check
>    path's per-file parse drops from 2 times to 1 time with C3 topology.
> 8. Lsp form landing (4.2.3, ruled by file source channelization, 2026-10-09 user decision): disk
>    file parse failure follows Check's Plan B (collect + exit compilation unit—two ends degraded
>    converged); edited buffer parse failure retains incomplete AST to continue typecheck (editor
>    philosophy—typing intermediate state shouldn't make semantic functions disappear). Discovery
>    arm for Lsp retains buffer source overwriting disk stale content; Typecheck under Lsp only
>    checks the target file (other units are only for the registry to provide signatures); the
>    normalization of in-file diagnostic order extends to LSP (typecheck diagnostic → W-code warning
>    → proof error). The old behavior's quirk of LSP hard-aborting on unrelated disk file parse
>    errors, with the handler silently falling back to the single-file path, is fixed along with
>    this step.
> 9. Embedded form landing (4.2.4): `Program` adds new `shared_registry` field—the embedded std
>    module is a sub-compilation, the registry is an **input** rather than a product
>    (EMBEDDED_STAGES has no Registry stage), the #94 SymbolTable sharing contract is thereby
>    externalized from "caller remembers to pass the same registry" to a program declaration. The
>    error path text is normalized from `<std.test> (embedded std)` to the unit virtual path
>    `<std/test>` (only the compiler's internal error surface, no test pinning). At this point all
>    four orchestrator entries are migrated into Driver.
> 10. standalone check unification (4.2.5, ruling A + IR-stage ruling, 2026-10-09): The Check form
>     faithfully carries single-file semantics for no-project-root programs—the warning surface only
>     covers the entry file (neighboring files only get errors, false-negatives over
>     false-positives); relative `use` resolves along the importer's directory (rustc single-file
>     mod alignment). CHECK stage table adds GlobalSlotAlloc + IrGeneration: standalone old path
>     (pipeline full chain) already ran IR generation, E3019/E1014/E1015 etc. only produced in
>     ir_gen (runner gate empirical proof). IrGeneration's Check form is pure check, not consume;
>     Script/Bin split is expressed by module_key's None/Some (E3023's existing switch,
>     `ir_gen.rs:1660`). Monomorphization not in CHECK (only produces resource-exceeded / internal
>     errors, zero corpus dependency). **〔2026-10-10 retracted per 3.4.8 empirical
>     evidence〕**—"zero corpus dependency" turns out to be evidence blind spot that the corpus has
>     no pathological recursive fixture; E3005 is the only compile-time defense for such programs.
>     See note #14. Incidental fix of latent defect: Registry arm's re-harvesting of registration
>     for embedded std units overwrites with_std()'s already-merged native half (E1043 false
>     positive)—embedded units now skip duplicate registration.
> 11. LSP single-file fallback unification (4.2.6): `run_diagnostics`'s manual
>     lex→parse→check_module_collect_all sequence is deleted, going through
>     `Program { kind: SingleFile, aggregation: CollectAll }`—LSP and CLI share the same Driver.
>     SingleFile+CollectAll form: Parsing collects all parse errors, retains incomplete AST to
>     continue typecheck (editor philosophy extended from 4.2.3 ruling; don't mark stage failure,
>     otherwise the topology skip would prevent typecheck from ever running); Typecheck dispatches
>     `check_module_collect_all` free function. The LSP single-file path from now on completes proof
>     (E4018), W-code warnings (W1001–W1005, test pinned), and IR-level errors; lexical failure
>     changes to report real diagnostics (the old synthesized "E0001 lexical error" text disappears
>     with the sequence deletion—no information loss, real diagnostics carry precise span).
> 12. `check_module` / `check_module_collect_all` line-by-line verification (4.2.7, C5 must-do
>     item): the two entries had already converged—the mod.rs layer is
>     `check_module_inner(ast, env, collect_all)` single boolean, the checker layer is
>     `check_module_impl(module, collect_all)`; the only fork is `init_body_checker(collect_all)` →
>     `set_collect_all_errors`, pass-3 and drain both modes run, difference carried by whether
>     collected_errors is empty; statements.rs five collection points (function body / use / for /
>     block / while). During verification confirmed and fixed duplicate diagnostic defect: the
>     collection point puts the first error in both collected_errors and the Err return channel,
>     pass-3's add_error to Err causes same-code same-span same-message ×2 (nested can be ×3); plus
>     two independent mechanisms: annotation validation signature formal parameters and overall
>     annotation double-visit (E1003/E1103), ownership layer same-point double-issue (E2014/E2018).
>     The fix lands at the module result boundary: deduplicate by (code, span, message)—diagnostics
>     are positional facts, duplicates carry no information; 63 corpus entries have duplicate copies
>     removed, run column and exit code zero drift (the "duplicate" of `borrow_conflict_err` is an
>     artifact of the baseline triple format not containing column number—two E2018 are actually
>     legitimate diagnostics for different columns).
> 13. wasm playground landing (4.2.8): `run_code`/`test_compile` go through `compile_playground` to
>     construct `Program { kind: WasmPlayground }` + Driver—the `Compiler` wrapper layer is no
>     longer involved. Error text byte-by-byte aligned with the old `CompileError` Display prefix
>     (Parse error:/Type error:/Internal error:), playground UI zero visible difference.
>     WasmPlayground and SingleFile share SINGLE_FILE_STAGES + FailFast (4.1 existing declaration,
>     this step has a real producer).
> 14. Multi-file monomorphization and CHECK stage table correction (3.4.8, 2026-10-10 user ruling,
>     **partial retraction of note #10's mono exclusion**). Empirical refutation of "mono only
>     produces resource limit noise": pathological generic recursion (`f(x)=f([x])`) under the
>     multi-file path without mono causes the compiler process to stack overflow (0xc00000fd—the
>     interpreter recurses in the Rust layer, no graceful runtime error), mono depth gate is the
>     only compile-time defense; standalone check before 4.2.5 went through pipeline full chain with
>     mono, the exclusion ruling caused silent coverage regression ("zero corpus dependency" only
>     proved the corpus has no pathological fixture). "Type erasure fallback" assumption is
>     invalidated by RFC-033 reflection ruling—`^^List(Int)` needs the real identity of
>     instantiation, monomorphization is the foundation of reflection rather than optional
>     optimization. Design four points: ①typecheck produces instantiation_request for qualified call
>     (`lib.f(x)`, FieldAccess form), generic_id uses qualified name, same namespace as merged IR
>     (currently only recognizes bare Var, `expressions.rs:2455`); ②`Stage::ALL` topology changed to
>     Linking before Monomorphization, MULTI_FILE stage table tail adds Monomorphization—consumes
>     merged_ir, aggregates all-unit requests (deferred bucket's containing_fn qualified), mono
>     promoted from single-module pass to whole-program pass; single-file stage table unchanged (no
>     Linking, subsequence property preserved, IR snapshots zero drift); ③CHECK adds Linking (pure
>     merge—E3020 entry validation stays in RoleClassification, prevent double-report) and
>     Monomorphization (pure check not consume, same as IrGeneration's Check form) two stages;
>     ④resource protection follows whole-program BFS covering cross-unit mutual recursion.
>     Monomorphizer body unchanged (consuming ModuleIR + request set contract happens to be the
>     merged IR form).
> 15. 3.4.8 R1 (①) landing (2026-10-10, a9438007). Per design #14 landing qualified call
>     instantiation collection: `ExpressionInferrer` injects module-qualified key table (`use`
>     three-arm registration—ordinary / alias / per-item alias and only SubModule kind exports),
>     `callee_generic_name` is concatenated via `SymbolTable::qualify` with merged-IR-same-source
>     qualified name; qualified call first-path arity criterion tightened (when the parameter table
>     can't be resolved falls back to second path taking real args by signature, avoiding
>     cross-module same-name function mismatch false-positive E3018). During implementation
>     empirically identified mono-side **three asymmetric defects** and fixed in the same round: (1)
>     Delete key (generic name set) and rewrite key (containing_fn + name + span triple) granularity
>     mismatch—sites that can't resolve real args degenerate to symbol requests entering deferred
>     bucket, when the containing function is non-generic the bucket never drains, yet the original
>     has been deleted by name → dangling call (`list_ops.yx:53` runtime E6006 empirical); fixed by
>     restoring gate `restore_generics_with_uncovered_call_sites`, using `build_call_site_map` as
>     the single source of truth to determine coverage, uncovered sites' originals retained. (2) The
>     recovery segment's `HashSet` iteration order caused function table order non-determinism (t2
>     re-review F1) → changed to `Vec` and sorted by name, product byte-level reproducible. (3)
>     Non-rewrite-side forms TailCall / MakeClosure originally also looked up the mapping key
>     (comment claims conservative) → `call_form_is_rewritten` single-point determination,
>     non-rewrite forms uniformly count as uncovered. Each comes with a pinned test (red-state
>     empirical discriminative power before the fix). Corpus diff baseline regenerates exactly one
>     line with the same commit (remove_at frame adds `(int64)`). **Submission strategy note**: when
>     typecheck collection and mono fix are split into two commits, only the former's staged tree
>     triggers `stdlib_docs`'s red on HEAD's existing defect (`list.is_empty([])` same-family
>     E6006)—the two halves are each other's green prerequisite, so merged into one atomic commit.
> 16. 2026-10-10 five rulings registration (after R1 closure): (1) **D2 ruling A**—user module
>     exports carry `generic_fn_type_params_snapshot()` along with `Export.type_params`, isomorphic
>     with the std path (`yx_sources.rs:62/108`): the declaration name becomes the authoritative
>     source for qualified call monomorphization, signature reverse-inference degraded to fallback
>     (merged into R2 scope). (2) **HKT declaration side live, call side broken** (inferred type
>     argument E1002 / explicit type argument E1010) registered as WBS 3.5 (3 third-level tasks).
>     (3) **deferred bucket never drains under non-generic container** merged into 3.4.8 ②'s
>     acceptance sub-item: the bucket must drain or loudly report error, must not silently not
>     specialize. (4) **closure variable name-based call runtime E6006** registered as WBS 3.6;
>     semantics (i) support indirect call / (ii) compile-time rejection, **ruled (i)**—support
>     indirect call: `CallDyn` mechanism is in place (`ir_gen.rs:7945` closure expression path), the
>     break is `Var` callee being downgraded to name-based `Call`, and typecheck originally accepts
>     the program. (5) **Tradeoff criterion solidification**—plan rejection reasons can only be
>     based on correctness and readability, "large change surface / new mechanism / high cost"
>     doesn't constitute rejection reason, falls into `coding-rules.md` part two and HOWTO
>     self-check list; tradeoffs that need user decision are given the full four items per
>     `AGENTS.md` "When requesting user ruling (mandatory)" (situation+evidence / per-item pros and
>     cons / clear recommendation+cost / breaking item naming).
> 17. 3.4.8 ② landing and F1 root cause (2026-10-11, 375789d2 + d922a551). ② landed per design #14:
>     `Stage::ALL` changed to Linking before Monomorphization, MULTI_FILE table tail appends mono;
>     Linking arm **before taking away each unit's IR** aggregates whole-program requests and
>     qualifies `containing_fn` (otherwise deferred bucket keys aren't same-source as merged IR,
>     bucket never drains). **① closure (D2 ruling A)**: user module exports carry declaration-phase
>     type parameter snapshot along with `Export::type_params`, isomorphic with the std path
>     (`yx_sources.rs:62/108`). **Empirically identified and fixed existing defects in the
>     process**: (a) **F1**—`qualify_names` only rewrites function bodies, `ir.init` is never
>     visited, under multi-file function definitions are already qualified while module top-level
>     calls are still bare-name → runtime E6006 (**reproduces when non-generic**, unrelated to
>     generics); fixed by init reusing the same `rename`/`aliases` and the same
>     `rewrite_call_names`. (b) Mono-side "whole-program unique suffix" heuristic and missed silent
>     `continue` → delete heuristic, missed changes to **loud E3018** (ambiguous silent mismatch is
>     worse than not parsing). **Regression caught and fixed by independent verification**: t6 found
>     that named import `use lib.{pick}` followed by bare-name call is falsely rejected by E3018 (IR
>     instruction name is alias-qualified name, request name is bare name, aggregation side only
>     qualifies by unit key) → fixed by aggregation side **directly calling ir_gen's**
>     `import_aliases_from` and `resolve_call_target` (extracted as pub single-point, one production
>     one consumption), and covered with matrix pinned tests for named import two forms. **Filed**:
>     init segment `DefId` blank empirically verified as "consistency blank" rather than live defect
>     (codegen name fallback resolvable, four kinds of init calls empirically pass), filed under WBS
>     3.7.4 to close "function body uses def, init relies on name" caliber fork.
> 18. init segment DefId caliber fork closure (3.7.4, 2026-10-11, 38bd4b9e): `assign_defs` extracts
>     `resolve_static_targets` single point (Call/TailCall string operand + MakeClosure), function
>     body and `ir.init` share the same implementation and the same `resolve` closure; 208 IR
>     snapshots regenerated with only 309 `(def=_)`→`(def=dN)` changes (other-lines 0; whole-tree
>     init segment 0 `(def=_)` residual). Codegen's DefId main path and name fallback two states'
>     translation results and execution values are item-by-item consistent (four kinds of init calls
>     empirically), corpus baseline zero drift; probe flipped and strengthened per new caliber
>     (identity + exact index + execution value, fallback init covered triggers red). **Errata
>     (2026-10-11)**: the above commit numbers were rewritten by `git pull --rebase`—the original
>     b554497b/e60dcc87/b7ac7396 correspond to the post-rebase 375789d2/d922a551/38bd4b9e in order;
>     the pushed dev history uses the new numbers (the old numbers are local temporary numbers,
>     didn't enter remote history).
> 19. 3.4.8 ③④ landing and verification (2026-10-11, 8ecf56f93). (③④ design basis see #14; ② landing
>     see #17, this item is its closure.) **Implementation**: `CHECK_STAGES` tail =
>     `…IrGeneration → Linking → Monomorphization`; Check's `Linking` only does pure merge
>     (aggregate requests + `link_module_irs`), the entry validation block stays in
>     `RoleClassification`; `ir_generation_check` generated then retained as `us.ir` (memory only);
>     mono failure in Check goes to the collection channel, in MultiFile keeps `fail_with`;
>     `into_outcome` first `take_check_diagnostics` then judges failure (structural failure no
>     longer clears already-collected diagnostics). **Verification**: four rounds of independent
>     verification (t3/t4/t10 + t5 cross-form matrix) and one round of re-review (t6) all pass;
>     corpus diff 340/340 line-by-line IDENTICAL, IR snapshots zero drift, test count 2643→2656.
>     **Existing gaps found and fixed in the same round**: F1 (rootless multi-file check's request
>     name and IR definition key different source → silently not specialize, see WBS 3.4.8 row's
>     landing note)—fix is aggregation side does precise reverse-lookup by real definition key
>     (non-heuristic), first-version fix is aggregation side does precise reverse-lookup by real
>     definition key (`flat_defs`); **same-day errata**: this bridge was already deleted in the
>     regression fix, the root cause is now resolved by `module_key`'s one-job-two-use decoupling
>     (`script_mode`), all three same-source as `{unit key}.{name}`. The positioning and fix of this
>     regression (CI doc example gate 258 blocks 10 failures → 0 failures) see WBS 3.4.8 row's
>     landing note. **Filed (existing, not this round's regression)**: E3005 help text doesn't match
>     resource protection semantics; multi-file E3005's span label misalignment (HEAD binary
>     reproducible); the type argument position dependency of the request collection surface (same
>     surface to 3.5).

> 20. 3.9.1 / 3.10.1 / 3.11.1 landing and diagnostic surface closure (2026-10-12, 809b9b936 +
>     8605a081b + two follow-up commits). **3.9.1 (execution layer guard reachable)**: Interpreted
>     call changes to **explicit
>     stack-driven**—`CallStatic`/`CallVirt`/`CallDyn`/`Spawn`/`SpawnFromList` after pushing frame
>     returns control to outer `step_one`, frame carries `ReturnTarget{Host|CallerSlot|Discard}`,
>     when returning writes back to the caller's slot, multi-call instructions continue running with
>     frame-internal `pending_calls`. Baseline measured per-layer interpreted call consumes native
>     stack **2544 B**, crash critical at `count_down(394)` (Windows main thread 1 MiB); after fix
>     20,000 layers of recursion can run on 512 KiB stack thread (about 0 per layer). The guard
>     (`push_frame`) is thereby reachable at any entry, any `max_stack_depth` value;
>     `ExecutorError::StackOverflow` changed to carry the real upper limit (the original
>     `util/diagnostic/mod.rs` always passed 0). **The original 3.11.2 (deep stack backtrace
>     truncation) was merged into this task per user ruling** (same file, inseparable by file):
>     backtrace changes to first 16 frames + count omission + last 8 frames, shallow stack
>     frame-by-frame unchanged. **3.10.1 (compile-time no-gate native recursion, #447)**: locate
>     first then change—crash point is `MonoType::type_name` recursion (**328 B/layer**, SP probe +
>     per-operation criticality determined), the same path also has derived `Clone` (~400 B/layer),
>     `substitute` internal deep copy, `type_name_hash` (164 B/layer), `collect_generic_type_refs`;
>     after the four work-list changes and removing the per-layer whole-request deep copy,
>     `mono.max_depth=5000` pathological generic recursion produces **E3005 (with real upper
>     limit)**, no more stack overflow. **3.11.1 (task boundary error code fidelity, #448)**: task
>     result error side changes from value to **structured**
>     (`TaskResult = Result<SyncValue, TaskFailure>`, `TaskOutcome::Err(TaskFailure)`),
>     cross-boundary restores `ExecutorError` by value; in-task E6001/E6003/E6004/E6008 in Standard
>     mode and Embedded messages character-for-character consistent. Two **pub API payload form
>     changes** have been written as `BREAKING CHANGE` in the commit. **Submission strategy note**:
>     3.9.1 and 3.11.1 **are inseparable at the file
>     granularity**—`interpreter/executor/executor.rs` simultaneously carries the trampoline and the
>     error channel's production/consumption points, `TaskFailure` is defined in
>     `runtime/engine.rs`, staging only one side fails to compile (pre-commit compiles with **staged
>     subset**, measured twice rejected), so per the same caliber as #15 merged into **one atomic
>     commit**. **Known boundaries and follow-up**: Drop derive (~75 B/layer) ⇒
>     `mono.max_depth ≳ 13000` may still crash; E4012 (constant evaluation) gate load is on native
>     recursion (each layer ~3,696 B, default 256 layers ≈ 924 KiB), currently zero call point,
>     production unreachable, future wiring must be closed in the same family; Standard mode task
>     interpreter is constructed per default `ExecutorConfig` ⇒ its E6004 upper limit is always 1024
>     (config propagation filed as WBS 3.12, user ruling B: separate team); test runtime budget is
>     opened as a separate issue #449 (pathological fixture 76s, clippy double target, full
>     submission run), not in this RFC's construction surface. **Corpus directory layering (same
>     round, 8605a081b)**: `02-type-system` (78) and `06-compile-errors` (134) total 212 fixtures
>     split into 23 sub-directories by category, 88 IR snapshots migrated by mirror path; corpus
>     baseline regenerated with the same commit (342 lines, independent re-calculation line-by-line:
>     only `file` field and 2 new changes), 41 snapshots regenerated due to file header path
>     lengthening, line-by-line check **only span offset change** (after de-offset normalization
>     zero difference).
> 21. LSP semantic data pipeline unification (4.6.1, 2026-10-16). Diagnostics and semantic data are
>     produced by the **same compilation**, in-project cross-file references carry `resolves_to`
>     pointing to the real definition—the old form is didOpen/didChange triggers **two trips** of
>     compilation: diagnostics go through the unified Driver pipeline, semantic data goes through
>     server.rs's manual lex→parse→single-file typecheck (no module registry, in-project `use`
>     resolution fails, reference events not produced, cross-file jump/rename/reference find
>     silently falls through—D55's registered silent failure is closed in this task).
>     **Implementation**: (1) `Export` carries `definition_span` (extract_module_info harvested from
>     AST identifier node, same-source same-value as typecheck definition event's `def_span`; native
>     std always None), `ModuleInfo` carries `definition_file` (Registry arm filled per
>     DiscoveredFile); (2) checker when draining reference events uses `import_def_locations` to
>     redirect cross-file import items' `resolves_to` from use line to the definition file's real
>     position (std import has no registration, maintains falling to use line); (3) checker module
>     identity changed from unified `"<module>"` to unit file path (this value is only consumed in
>     the semantic data surface, zero diagnostic consumption—empirically all 4 consumption points of
>     `env.module_name`); (4) `DriverOutcome` adds new `semantic_db` channel—CollectAll form (Lsp
>     target file regardless of diagnostic success + SingleFile fallback) is aggregated, Lsp arm
>     tail from registry injects non-target-file top-level export definition surface
>     (`external_definitions`, unopened files can also jump; type / submodule exports not
>     injected—typecheck side type registration doesn't produce definition events, both sides keep
>     consistent gaps); (5) orchestrator adds new `analyze_source_in_project`
>     (`check_source_in_project` is transferred and only takes the diagnostic half); (6) LSP side
>     `analysis.rs` single compilation entry replaces manual pipeline (server.rs's
>     `update_semantic_db` deleted), World key unified as normalized file URI (`uri.rs`:
>     `path_to_uri(uri_to_path(u))` idempotent normalization, drive letter case and %XX encoding
>     differences normalized to the same key). **Pass-by fix (D59 branch B-class—old behavior silent
>     inaction, doesn't change any originally successful program's external performance)**:
>     in-project file's cross-file jump/rename/reference find changes from silently falling through
>     to hitting the real definition; on lexical error the old manual pipeline deletes semantic
>     information and changes to incomplete AST tinting retention (4.2.6 editor philosophy
>     extension); target file with type error semantic data no longer discarded along with
>     `type_result` (tint/jump don't disappear in typing intermediate state). **Pinned**:
>     driver-level 6 (target file path key / cross-file resolves_to+external definition surface
>     round-trip / std import fallback use line / error file retention / single-file fallback path
>     key + error retention), LSP e2e 1 (red-then-green empirical proof—jump returned None before
>     the fix), URI normalization 4. **Ten-entry refactoring approach (function-by-function)**:

| Entry                                                      | After Refactoring                                                                                                                                                                                           |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Pipeline::run` (`pipeline.rs:141-227`)                    | Delete function body, change to construct `Program { kind: SingleFile, … }` to hand to Driver                                                                                                               |
| `Compiler::compile` (`compiler.rs:95`)                     | Unchanged (already a stateless wrapper)                                                                                                                                                                     |
| `compile_project` (`orchestrator.rs:99-237`)               | Slim down to `Program { kind: MultiFile, aggregation: FailFast }` constructor                                                                                                                               |
| `check_project` (`orchestrator.rs:273-399`)                | Slim down to `Program { kind: Check, aggregation: CollectAll }` constructor                                                                                                                                 |
| `check_source_in_project` (`orchestrator.rs:450`)          | Change to one Driver call + filter result to target file                                                                                                                                                    |
| `compile_embedded_module` (`orchestrator.rs:1374`)         | Change to `Program { kind: Embedded, units: [embedded] }`                                                                                                                                                   |
| `lib::run_file` (`lib.rs:140`)                             | Unchanged (transfers to `run_project` or `SingleFile`)                                                                                                                                                      |
| `lib::run_project` (`lib.rs:154`)                          | Unchanged (transfers to Driver)                                                                                                                                                                             |
| `lsp::run_diagnostics` (`lsp/handlers/diagnostics.rs:146`) | In-project changes to `Program { kind: Lsp, aggregation: CollectAll }`; **manual lex→parse→… sequence deletion** for single-file branch, changes to `Program { kind: SingleFile, aggregation: CollectAll }` |
| `check_files_with_diagnostics` (`diagnostic/mod.rs:565`)   | `standalone` branch (`614-616` → `check_single_file` `623-661`) deleted, unified to `Program { kind: Check }`                                                                                               |

**Duplication between `check_project` and `compile_project` (about 40% shared / 60% forked)**.
Shared: vendor consistency check, file discovery, `build_registry_from`, `all_method_bindings`,
parsing loop, per-file checker assembly, result processing. Forked: items 2/3/4/5/7/8/9/10/11 of the
11-item table above. **This 40% shared skeleton is exactly where `Driver` creates value**—the forked
60% is all "different stage set" or "different aggregation mode", and these are exactly what the two
fields of `Program` can completely express.

### 4. Failure Semantics: `Continue` / `Abort` / `Warn`

```rust
pub enum StageOutcome {
    Continue,  // Stage succeeded, continue
    Abort,     // Stage failed, terminate this compilation
    Warn,      // Stage has non-blocking issues, continue
}
```

**Key design decision: "upstream failure causes this stage to skip" is not a stage return value.**

Reason: skipping is **topology-determined**, not stage-determined. If we let each stage return
`Skipped` itself, then the information "why didn't I run" is scattered across 12 stages, making
centralized audit impossible. Change to having `dispatch` determine at the loop start based on
dependencies:

```rust
if !state.deps_satisfied(stage) {
    state.record_skipped(stage);
    continue;
}
```

**`Skipped` must produce a diagnostic.** This rule directly addresses the vulnerability this
document is fixing—the current `if !proof_calls.is_empty()` at `pipeline.rs:187` is a silent "skip"
that produces no diagnostic, just because the obligation happens to be empty. Once the rule is
established, any "because X wasn't done, Y didn't run" must be explainable.

The diagnostic text needs to distinguish three skip reasons:

| Reason                                                            | Text Direction                                                                                                                                 |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Caused by upstream `Abort`                                        | "Stage Y did not execute because upstream stage X failed"                                                                                      |
| Caused by prerequisite obligation being empty                     | "Stage Y did not execute because there were no pending obligations (normal)"—**this item's severity = Info, doesn't count into warning count** |
| Prerequisites not satisfied (e.g. `config.mono.enabled == false`) | "Stage Y did not execute because configuration is not enabled"                                                                                 |

The second category is key: it lets "normal skip" and "abnormal skip" be distinguishable in the
diagnostic stream, and doesn't pollute `yaoxiang check`'s `warning_count`
(`diagnostic/mod.rs:637-641` depends on this count's non-blocking contract).

### 5. Diagnostic Aggregation Modes: `FailFast` vs `CollectAll`

```rust
pub enum Aggregation {
    FailFast,    // First error aborts (compile path)
    CollectAll,  // Collect all diagnostics (check / LSP path)
}
```

| Mode         | Users                                                                                                                                                                           | Current State Correspondence |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `FailFast`   | `compile_project` (`orchestrator.rs:125-137` first error returns `OrchestratorError::TypeCheck`), `Pipeline::run` (`pipeline.rs:150/162/174/193` four early-exits)              | Already exists               |
| `CollectAll` | `check_project` (`orchestrator.rs:313-397` no early exit), `check_source_in_project` (`493` `check_module_collect_all`), `lsp::run_diagnostics`, `check_files_with_diagnostics` | Already exists               |

**`Aggregation` must be a field of `Program`, not a global Driver setting**—LSP serves both
in-project files and single-files in the same process, while CLI's `run` and `check` are two
independent calls; putting them on Program avoids the Driver holding mutable global state.

**Note the semantic difference between the current two typecheck branches**: `check_module`
(`orchestrator.rs:124`) and `check_module_collect_all` (`493`) differ not just in "early exit or
not"—they are two different checker entries. After unification, the same implementation should be
driven by the `Aggregation` parameter, rather than keeping two functions (**this is an additional
item this document needs to verify, see "Known Limitations and Risks"**).

---

## Detailed Design

### Type System Impact

| Change                                      | Type-layer Impact                                                                                                                                    |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| New `driver/` module                        | **None**. Doesn't introduce new type representation, doesn't touch `MonoType` / `PolyType` / `ir::Type`                                              |
| `TypeCheckResult` → `Obligations` migration | **Field types unchanged**, only ownership changes. `release_plan` is still `ownership::ReleasePlan`, `proof_calls` is still `Vec<ProofFunctionCall>` |
| `Stage` / `Program` / `Obligations`         | **Brand-new types**, unrelated to the language type system                                                                                           |

**This document does not introduce a fourth type representation.** The convergence of the three
parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) is the scope of
`03-type-unification.md` (RFC-039 routing table C).

**Obligation type ownership and layering**: What migrates into the ledger is **settlement
responsibility**, not the ownership module of the type. `ReleasePlan` is still defined in
`layers/ownership.rs`, `ProofFunctionCall` is still in `proof/`, other field types stay where they
are; `driver/obligations.rs` only holds the aggregate container and `assert_drained()`. L3 consumers
(e.g. `ir_gen` reading `release_plan`) continue to receive concrete field types via parameters,
**must not `use crate::driver`**—this is consistent with the Driver's no-reverse-dependency red line
below.

**Dependency direction** (RFC-039 four-layer model): `driver` (L1) depends on `frontend` (L2) /
`middle` (L3) / `backends` (L4) interfaces; **L2/L3/L4 must not reverse `use crate::driver`**.
Driver appearing in TypeChecker's import is considered a violation, intercepted by RFC-039's
proposed `scripts/ci/check-module-boundary.py`.

### Runtime Behavior

| Scenario                            | Before                                   | After                                                                                                                                                                                                                                                                                                                                                          |
| ----------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `yaoxiang run app.yx` (single-file) | 5 stages                                 | Same 5 stages + `assert_drained()` settlement (**4.10.3 revision, 2026-10-15**: CLI `run` for standalone no longer goes through this pipeline—unified to `compile_project` (MULTI_FILE stage table), Script semantics driven by rootless; this line now only describes the `lib::run_file` / `Compiler::compile` library path)                                 |
| `yaoxiang run` (multi-file)         | No proof_execution, no W1001/W1002/W1003 | proof_execution already connected as a stopgap in P3; **warning surface maintained without new additions** (decision B1, 2026-10-09: dead code family warnings don't enter the compile path—pool semantics being corrected by 4.9 as a defect, normalizing the warning surface waits until after 4.9; this line's original draft "new warning output" is void) |
| `yaoxiang check` (multi-file)       | No proof_execution                       | Already connected as a stopgap in P3                                                                                                                                                                                                                                                                                                                           |
| LSP (in-project)                    | No proof_execution                       | Already connected as a stopgap in P3                                                                                                                                                                                                                                                                                                                           |
| Z3 not installed + single-file      | `predicate.rs:35` **panic**              | Change to `Abort` + E-level diagnostic (**this is a behavior change, see compatibility**)                                                                                                                                                                                                                                                                      |
| Z3 not installed + multi-file       | Silent skip                              | Same as above, unified                                                                                                                                                                                                                                                                                                                                         |

**The only intentional behavior break** is changing `predicate.rs:35`'s `.expect()` to return a
diagnostic. This conforms to RFC-027 §8's "not bound to a specific solver" positioning, and is also
the contract already declared by `backend.rs:60` ("caller should degrade conservatively")—the
current implementation contradicts its own contract document.

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

| File                                               | Line Range                         | Change                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`                                       | `24-36` (module declaration block) | Add `pub mod driver;`                                                                                                                                                                                                                                                                                                                                          |
| `src/lib.rs`                                       | `140-147`                          | `run_file` changes to construct `Program { kind: SingleFile }`                                                                                                                                                                                                                                                                                                 |
| `src/lib.rs`                                       | `154-167`                          | `run_project` changes to construct `Program { kind: MultiFile }`                                                                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                         | `141-227`                          | `Pipeline::run` function body replaced with `Program` construction + Driver call                                                                                                                                                                                                                                                                               |
| `src/frontend/pipeline.rs`                         | `275-278`                          | Dead code analysis moved out, changed to `Stage::DeadCodeAnalysis` arm                                                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `306-311`                          | `run_proof_execution` **kept**, moved into `driver` and used as `Stage::ProofExecution` implementation                                                                                                                                                                                                                                                         |
| `src/frontend/pipeline.rs`                         | `381-389`                          | Monomorphization moved out, changed to `Stage::Monomorphization` arm                                                                                                                                                                                                                                                                                           |
| `src/frontend/module/orchestrator.rs`              | `99-237`                           | `compile_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                               |
| `src/frontend/module/orchestrator.rs`              | `273-399`                          | `check_project` slimmed to Program constructor                                                                                                                                                                                                                                                                                                                 |
| `src/frontend/module/orchestrator.rs`              | `450`                              | `check_source_in_project` goes through Driver                                                                                                                                                                                                                                                                                                                  |
| `src/frontend/module/orchestrator.rs`              | `1374`                             | `compile_embedded_module` changed to `Program { kind: Embedded }`                                                                                                                                                                                                                                                                                              |
| `src/frontend/module/orchestrator.rs`              | `486-494`                          | `typecheck_with_registry_in`'s `check_module` / `check_module_collect_all` two-choice changed to be driven by `Aggregation` parameter                                                                                                                                                                                                                          |
| `src/lsp/handlers/diagnostics.rs`                  | `146-227`                          | `run_diagnostics`'s manual stage sequence deleted, changed to call Driver                                                                                                                                                                                                                                                                                      |
| `src/util/diagnostic/mod.rs`                       | `565-619`                          | `check_files_with_diagnostics` unified to `Program { kind: Check }`                                                                                                                                                                                                                                                                                            |
| `src/util/diagnostic/mod.rs`                       | `621-661`                          | `check_single_file` **deleted**                                                                                                                                                                                                                                                                                                                                |
| `wasm/src/lib.rs`                                  | `30-36`, `42-51`                   | `test_compile` / `run_code` changed to construct `Program { kind: WasmPlayground }` (currently implicitly goes to path 1 via `Compiler::compile_with_source`)                                                                                                                                                                                                  |
| `src/frontend/core/typecheck/layers/predicate.rs`  | `34-36`                            | `.expect()` changed to return `SMTResult::Unknown` + diagnostic (**behavior change**)                                                                                                                                                                                                                                                                          |
| `src/frontend/core/typecheck/checker.rs`           | `1303-1314`                        | `ProofResult::Unproven { .. } => {}` empty arm changed to produce diagnostic (**fix the second silent discard point**)                                                                                                                                                                                                                                         |
| `src/frontend/core/typecheck/checker.rs`           | `1283-1293`                        | `unwrap_or_default()` degradation path adds warning diagnostic                                                                                                                                                                                                                                                                                                 |
| `src/frontend/core/typecheck/checker.rs`           | `1244`                             | Fix comment inconsistent with `1320`                                                                                                                                                                                                                                                                                                                           |
| `src/frontend/core/typecheck/layers/README.md`     | Whole file                         | Layer order table changed to **actual** execution order; delete "lower layer fails so upper layer doesn't run" (no short-circuit exists)                                                                                                                                                                                                                       |
| `src/frontend/core/typecheck/proof/smt/backend.rs` | `67-72`                            | **Per D58**: add process-level shared singleton (`LazyLock<Mutex<Option<Box<dyn Solver>>>>`) + `with_shared_solver` closure access port; `predicate.rs`/`checker.rs`/`ownership.rs` three consumer points changed to go through shared port. `&'static` bare reference form is not feasible (`dyn Solver` is not `Sync`), the original text is governed by D58 |

**Deleted**: `src/util/diagnostic/mod.rs:621-661`'s `check_single_file`;
`src/lsp/handlers/diagnostics.rs:161-227`'s manual lex/parse sequence.

**Unchanged**: `checker.rs:5164/5179/5306/5318/5420/5448` (the six `Unproven` branch logics
themselves are correct—**the problem is at the consumer end, not the producer end**, changing the
producer end would mask the architectural defect); `Cargo.toml`; any
`#[cfg(target_arch = "wasm32")]` branches (wasm branch reachability determination goes to
`06-cleanup-inventory.md`).

### Backward Compatibility

| Change                                                    | Compatibility                                                               | Disposition                                                                                                                                          |
| --------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-file path adds proof_execution                      | **Breaking**: previously silently passing constraints will now report E4018 | `test_multifile_proof_obligation_not_dropped` **write red first** then fix; release in batches, expected diagnostic set changes go through CHANGELOG |
| Multi-file path adds W1001/W1002/W1003                    | **Breaking**: programs that compile cleanly now produce warnings            | Warnings are non-blocking (`warning_count` counted separately, `diagnostic/mod.rs:637-641`), doesn't change exit code                                |
| `assert_drained()` first run produces W-level diagnostics | **Breaking**: diagnostic set adds                                           | W first then E, S2/S4 in two steps                                                                                                                   |
| `predicate.rs` panic changed to diagnostic                | **Improvement**: no longer crashes                                          | No breaking                                                                                                                                          |
| `build` / `dump_bytecode` subcommands                     | **No impact**                                                               | These two paths don't go through proof_execution                                                                                                     |
| `TypeCheckResult` fields migrated to `Obligations`        | **Internal refactor**                                                       | `pub` API surface needs synchronization if there are external dependencies; in-repository consumers are fully listed in the change list              |

---

## Implementation Points

This document corresponds to RFC-039's global stage sequence's **P3 (fix correctness
vulnerabilities)** and **P4 (stage contracts and unified Driver)**. The following S1-S5 are this
document's internal implementation order, **each step's acceptance criterion references the
[equivalence criteria document](07-equivalence-oracle.md)'s C2 category** (orchestration change:
each entry's **diagnostic set** identical + corpus behavior identical).

### S1: Establish Criterion (Red First)

**Prerequisite, cannot be skipped.** The vulnerability criterion must first be written as failing.

| Deliverable                                                                                                                                            | Acceptance                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| `test_multifile_proof_obligation_not_dropped`                                                                                                          | **Must be red**. If accidentally green, this document's vulnerability analysis needs re-review                |
| Multi-file corpus layer: create `tests/yaoxiang-multifile/` (D48 ruling—not mixed into single-file corpus tree; project fixtures with `yaoxiang.toml`) | Single-file and multi-file two versions of the corpus diagnostic set can be compared                          |
| `src/driver/tests/obligations.rs` skeleton (first `#[ignore]`)                                                                                         | Mark `#[ignore]`, S4 turn green (red state kept on file: after removing the only settlement point `pairs: 0`) |

**Rollback point**: no code change, purely added tests.

### S2: Introduce `Stage` + `Program`, Don't Change Behavior

| Deliverable                                              | Acceptance (C2)                                                                              |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `driver/stage.rs`, `driver/program.rs`, `driver/unit.rs` | `test_program_stage_coverage`: `Program::stages()` ⊆ `Stage::ALL` and equals expected array  |
| `pipeline.rs:141-227` changes to Driver call             | **Single-file path diagnostic set and exit code byte-by-byte identical**                     |
| Full corpus (293 `.yx`) diff                             | Diagnostic list (sorted by `(code, file, line)`), exit code, stdout/stderr **all zero-diff** |

**This stage deliberately fixes no bug**—it only moves existing behavior into Driver. The acceptance
criterion is C1/C2-level zero-diff.

**Rollback point**: `git revert` a single commit, `pipeline.rs` reverts to the original state.

### S3: Merge Orchestrator Four Entries + Fix Vulnerabilities

| Deliverable                                                                                                                     | Acceptance (C2)                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `compile_project` / `check_project` / `check_source_in_project` / `compile_embedded_module` all changed to Program constructors | Four-entry diagnostic set normalized by `Aggregation` is identical                |
| `check_single_file` (`diagnostic/mod.rs:621-661`) deleted                                                                       | `yaoxiang check` behaves consistently inside and outside the project              |
| LSP manual stage sequence deleted                                                                                               | LSP and CLI give identical diagnostic set for the same file                       |
| **`test_multifile_proof_obligation_not_dropped` turns green**                                                                   | Vulnerability fixed                                                               |
| `checker.rs:1313` empty match arm fixed                                                                                         | `test_no_silent_pass_on_unproven`: `Unproven` must produce diagnostic in any mode |
| `predicate.rs:34-36` `.expect()` changed to diagnostic                                                                          | No longer panics in Z3 environment                                                |

**Rollback point**: vulnerability fix and structural merge are **two separate commits**. If the
structural merge has issues, you can only roll back the structural commit, keep the vulnerability
fix commit—then `test_multifile_proof_obligation_not_dropped` stays green; the reverse (keep
structure, roll back fix) would turn red, which is an unacceptable intermediate state, forbidden to
merge.

### S4: Enable Obligation Ledger

| Deliverable                                  | Acceptance                                                                                                                             |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `driver/obligations.rs` + `assert_drained()` | `src/driver/tests/obligations.rs` 20 use cases turn green (implementation form see §2 revision note; entry count subject to that file) |
| `scripts/ci/check-obligations.py`            | Static gate effective                                                                                                                  |
| Obligation diagnostic W → E upgrade          | Full corpus diff then manual review each new E                                                                                         |

**Rollback point**: `assert_drained()`'s severity level can be controlled by config item, W/E switch
doesn't require code structure changes.

### S5: Proof Layer and wasm Wrap-up

| Deliverable                                                                                                                                                                                                                        | Acceptance                                                                                                                                                                  |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `layers/README.md` layer order changed to actual order                                                                                                                                                                             | Documentation corresponds one by one to `checker.rs` call points                                                                                                            |
| `backend.rs` process-level shared singleton (D58; **hard prerequisite: Unknown not in cache**—otherwise one timeout gets solidified across compilations through process-level cache, and pollutes cargo test inter-thread sharing) | Production path `default_solver()` call points reach zero (only tests retain); cache hit rate observable across three consumer points (counter already landed in 89576fafd) |
| `checker.rs:1283-1293` degradation adds warning                                                                                                                                                                                    | Diagnostic when no `body_checker`                                                                                                                                           |
| `orchestrator.rs` 10 wasm attributes (4 positive + 6 negative) reachability determination                                                                                                                                          | Conclusion goes to "F2 Determination" section of `06-cleanup-inventory.md`, this document only registers the determination requirement                                      |

**Note**: correcting the layer order will change the diagnostic set, may expose large amounts of
previously silenced `Unproven`. **This item should be independent of S1-S4**, don't mix entry
merging in the same PR.

## Key Decisions and Reasons

| Decision                 | Determination                                                                                                                                                                                                                                                                                                                                                                                                                             | Reason                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stage model**          | `Stage` is a 12-variant compile-time-exhaustive enum, **refuse runtime registration**                                                                                                                                                                                                                                                                                                                                                     | Exhaustive `match` is zero-cost compile-time enforcement: when a new variant is added, `dispatch` fails to compile. This is isomorphic to the mechanism the opcode table has already validated in RFC-039 routing table B. `StageScope` further turns "per-module or project-level" into a type fact, eliminating call-count questions that require human reasoning                                      |
| **Obligation mechanism** | `Obligations` + `assert_drained()` do **runtime** settlement, used in tandem with `scripts/ci/check-obligations.py`'s **static** gate; **severity final state is E-level** (2026-10-13 user ruling A: same-code promote to E, code number unchanged; **implementation form see §2 revision note**: ledger doesn't copy field structure; red list—after settlement 0 entries—mechanism kept, doesn't enter user-visible diagnostic stream) | Fixes a whole class of bug, not one bug: at least 2 same-class latent dangers already verified (proof_calls, `checker.rs:1313`), all 13 obligation fields are in range. W→E in two steps is to make each step's C2 criterion usable—now the obligation list has settled to 13, red list zero, healthy state zero emission, so the final state takes E directly (see §2 revision note sections VII, VIII) |
| **Fix scope**            | Fix the **consumer end** (orchestration layer), **don't touch** the six `Unproven` branch producer-end logic of `checker.rs:5164/5179/5306/5318/5420/5448`                                                                                                                                                                                                                                                                                | The problem is at the consumer end, not the producer end. Changing the producer end would mask the architectural defect as "logic is fixed", while the logic of these three branches is itself correct                                                                                                                                                                                                   |

This design additionally eliminates two kinds of cracks:

- **The crack between "comment commitment" and "code behavior"**. The "must not let silent pass
  return" at `checker.rs:5165` is a **comment-level contract**, fulfilled by `assert_drained()`.
- **"One array forgot to append"**. Currently five functions each hand-write the call sequence;
  after unification there is a single stage table, and missing stages in new additions will cause
  compile-time errors.

### Unadopted Directions

- **Generic stage chain `Stage<A, B>`
  (`pipeline!(Lex -> Parse -> Typecheck<Ast> -> Proof<Ast> -> Ir)`)**—7 of the 11 inconsistencies
  (two implementations of dead code analysis, W1006, W1003, monomorphization, role context, IR
  generation/linking) **don't involve type bridging**, but are configuration questions of "do we
  want to run some analysis"; it also can't express `Aggregation`, and can only move `check_module`
  / `check_module_collect_all`'s two-choice from the function layer to the type layer.
- **`dyn Stage` + runtime registration (`driver.register(Box::new(...))`)**—cancel compile-time
  exhaustiveness: when a new stage is added, `dispatch` no longer fails to compile, equivalent to
  replacing "five functions each hand-write calls" with "one array forgot to append", which is
  exactly the cause of the current 11 inconsistencies. This repository already has the same-form
  anti-example: the `ModuleDependencyGraph` / `affected_modules` / `ModuleCache` / `HotReloader`
  described in `docs/src/dev/design/check/` are all zero-implemented.
- **Only fix bugs, don't move structure**—can fix the known 2 bugs, but only covers 1 of the
  obligation fields, `Stage` is still scattered across 5 functions, the next fork point will
  continue to grow from here. **It must be done first** (part of S1/S3), because the structure
  refactor needs a known-red test to prove the criterion is valid.
- **Change `proof_calls` to `pub` and add `debug_assert`**—`check_module` is a general entry, it
  doesn't know who the caller is, `debug_assert!(<caller will handle>)` cannot hold; `#[must_use]`
  only warns when the field is **entirely** discarded. This is looking for the bug at the wrong
  level: the bug is at the orchestration layer, detection must be at the orchestration layer.

## Known Limitations and Risks

- **`orchestrator.rs`'s slimming will make it harder to read in the short term**. After the 139-line
  `compile_project` is split into "Program constructor + several driver arms", readers need to cross
  two files to understand the flow. This is a common cost of all "centralize the wiring" refactors.
- **S3 will significantly change the diagnostic set, and the direction of change is "exposing
  previously silenced problems"**. After the fix there may be a batch of "new error" user
  reports—they are real bugs, they just hadn't been reported before. Must be explicitly stated in
  the CHANGELOG.
- **`assert_drained()`'s severity-level switch requires per-field manual judgment**. Some obligation
  fields (e.g. `module_namespaces`) having "no consumer" might be designed to not need consumption,
  shouldn't report. S4 needs to go through them one by one, can't be a one-size-fits-all approach.
- **`Program` abstraction may be premature**. If some entries' stage sets are unstable in the long
  term, `stages()` will degrade to "each call passes a different array" free parameter, contract
  constraints fall through. **Mitigation is `test_program_stage_coverage` asserting `stages()` can
  only come from 6 predefined combinations.**
- **Criterion dependency**: S1/S3's vulnerability fix criteria depend on the vulnerability test in
  the [equivalence criteria document](07-equivalence-oracle.md) being red first. **If that document
  doesn't establish the multi-file corpus layer first, S1 cannot be accepted**—because the existing
  293 corpora all go through the single-file path, zero coverage of this class of defect. This
  document doesn't involve the IR validator (`verify_loose`), that prerequisite work is not in this
  document's scope.
- **Layer-order correction will expose previously silenced `Unproven`** (`equivalence` is not in the
  pipeline at all, `termination` and `ownership` order is the opposite of the declaration). This is
  a diagnostic-set-change risk, should be independent.
- **Thread safety after SMT backend changes to singleton is undetermined**. `backend.rs:13-19`
  already explains that `Solver` is only `Send` not `Sync`, `Z3Backend`'s cache is `RefCell`; after
  changing to a cross-compilation-unit-shared singleton, need to confirm all access paths go through
  `Mutex`.
- **Stage parallelization not evaluated**. Multi-file typecheck is naturally parallelizable, but
  parallelization will **mask order-dependency defects** (such as the actual order dependency
  between termination and ownership). Should be opened after the equivalence criteria stabilize.

> **All originally listed open issues in this section have been ruled.** The per-item determination
> see [RFC-039 Ruling Registration](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

- [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md) —
  Four-layer model, routing tables A/B/C, G1-G10 acceptance gates, P1-P10 execution order
- [01-routing.md](01-routing.md) — Stage table and dependency direction norms,
  `scripts/ci/check-module-boundary.py`
- [03-type-unification.md](03-type-unification.md) — Convergence of the three parallel type
  representations (this document does not introduce a fourth set)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — wasm branch reachability cleanup;
  `checker/semantic_tokens.rs`'s `include!` refactor (construction steps belong to
  [09](09-execution-wbs.md) §P5 5.1)
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C1-C6 tiers, three-layer criteria,
  `test_multifile_proof_obligation_not_dropped` vulnerability criterion
- [RFC-027 Compile-time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Design source of Phase 2.5 proof function execution
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Reference
  code table for `build.rs:19-55` gate
- `src/frontend/core/typecheck/checker.rs:5165-5169` — The explicit declaration of "must not let
  silent pass return"
- `src/frontend/core/typecheck/layers/README.md` — Layer order call table and the true identity of
  `equivalence` (no short-circuit)
- `src/frontend/core/typecheck/layers/predicate.rs:31-32` — The explicit reason for SMT
  initialization hard failure
- `src/frontend/core/typecheck/proof/smt/backend.rs:60` — The contract declaration of "caller should
  degrade conservatively" (contradicts `predicate.rs`)
- `src/frontend/core/typecheck/types.rs:16-76` — 13 obligation fields that downstream must consume
  (after 3.4.4/3.4.5/3.4.9 settlement)
- `src/frontend/core/typecheck/tests/rfc027_refined_transparency.rs:13-15` — The unit test's
  self-acknowledgment of "stops before pipeline"
- `tests/yaoxiang/06-compile-errors/refinement/refined_annotation_literal_violates_err.yx:12-13` —
  The corpus's self-acknowledgment of "executed by pipeline.rs after check_module"
- `wasm/src/lib.rs:48` — The fourth compilation caller of the playground (single-file path, so
  executes proof_execution)
- `wasm/Cargo.toml:10-18` — cdylib + wasm-bindgen in the shim crate rather than the main crate
- `build.rs:19-55` — Error code build-time gate, the exemplar of the project's enforcement mechanism
